# Phase 4: Terminal Visuals - Context

**Gathered:** 2026-06-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Build **four terminal-visual commands** on the existing `core` infrastructure (and, for `matrix`, the already-present `crossterm`):

- `lolcat` — rainbow-colorize piped/arg/`--file` text with a smooth truecolor gradient (LOL-01)
- `matrix` — full-terminal green digital-rain animation with clean Ctrl+C teardown (MTRX-01)
- `ascii` — render a PNG/JPEG image as ASCII art fitted to the terminal width (ASCI-01)
- `json` — pretty-print + validate + `--compact` minify JSON (JSON-01)

Each command swaps its unit `Commands::*` variant in `src/cli.rs` for a real Args struct implementing `RunCommand`, removes its `not_implemented(...)` arm in `src/main.rs`, and ships with `assert_cmd` / `trycmd` tests. All reuse the Phase-1 core: `core::output` color gating (`is_color_on`/`init_color`, byte-identical-minus-ANSI when piped, Phase-1 D-09/D-10), `terminal_width()`, strict 0/1/2 exit codes, `data→stdout / messages→stderr`, and `core::input` (`read_input` / `read_file_or_stdin` + `-` sentinel + `--file`) for the text/file consumers (`json`, `lolcat`). `enable_ansi_support` is already called first in `main()`.

**In scope:** LOL-01, MTRX-01, ASCI-01, JSON-01 (4 requirements). New Cargo deps: `image`, `serde_json` (`preserve_order`), `unicode-width`, `strip-ansi-escapes` (`crossterm` + `owo-colors` already present).

**Not in scope (v2 / Out of Scope — see REQUIREMENTS.md):** colored ASCII / braille / invert (VIS-V2-01); lolcat `--animate`/`--freq`/`--seed` (VIS-V2-01); matrix configurable color/speed/charset incl. any Latin fallback flag (VIS-V2-01); ascii video/GIF/URL input (Out of Scope); json full jq query language (Out of Scope); json `arbitrary_precision` exact-number fidelity (deferred behind a flag). The Phase-5 commands (`qr`, `clip`, `pomodoro`, `weather`) and their standing todos (winrt spike, pomodoro blocking, weather units) are NOT this phase.

</domain>

<decisions>
## Implementation Decisions

> Each area was researched by a dedicated advisor agent (calibration: full_maturity) and chosen by the user from a scored comparison table. All recommendations converge on the project's established **lean-binary** value and the **locked color-gating contract** (byte-identical-minus-ANSI when piped, via `is_color_on()`).

### Shared / carry-forward (locked by Phases 1–3 — do NOT re-derive)
- **D-00:** Every command follows the established slice: swap the unit variant in `src/cli.rs` for a real `Args` struct + `RunCommand` impl, delete its `not_implemented(...)` arm in `src/main.rs`, route all input through `core::input`, all styled output through `core::output::is_color_on()` (no parallel color path / no `owo_colors::set_override`), `data→stdout` / `messages→stderr`, exit `0/1/2`, and ship `assert_cmd` + `trycmd` tests. `box` is binary-only → unit tests run via `cargo test --bin box`, NOT `--lib`.

### ascii — rendering engine (ASCI-01)
- **D-01:** **Engine = hand-roll on the `image` crate 0.25.10**, NOT the `artem` 3.0.0 crate. ⚠️ Correction to the advisor note: `image` is **not yet a dependency** — hand-rolling adds **exactly one** new crate (`image`), which is the legitimate "trap to hand-roll" exception to the lean-binary policy (image decoding). `artem` was rejected because, even consumed as a library, its `[dependencies]` are **unconditional**: it drags `clap`, `colored`, `terminal_size`, `log`, `env_logger`, `anstyle-svg`, `once_cell` (+ `ureq`/TLS via the default `web_image` feature unless disabled) — every one duplicating infra `box` already owns (its own pinned clap 4.6, owo-colors, `core::output::terminal_width()`), and its internal width logic **bypasses** `terminal_width()`. This is the same "convenience crate whose cost outweighs its value" rejected for `humansize`/`ignore`/`passwords`.
- **D-02:** **Pipeline:** `image::open(path)` (auto-decodes PNG + JPEG via default features) → `resize_exact(cols, rows, FilterType::Triangle)` → `to_luma8()` → map each luma byte through a **dark→light ASCII ramp**. **`cols = core::output::terminal_width()`** (falls back to 80 when piped); **`rows = cols * src_h / src_w / 2`** — the `/2` corrects the ~2:1 terminal character-cell aspect so the image isn't vertically stretched. Source dims via `image.dimensions()`.
- **D-03:** **Monochrome v1.** Plain ASCII to stdout — no color path now (colored ASCII = VIS-V2-01). The ramp emit is the clean v2-color seam: when color is added later it gates truecolor per-cell through `is_color_on()`. Input path (positional or `--file`) via `core::input`.

### json — formatter internals (JSON-01)
- **D-04:** **serde_json config = `preserve_order` ON, default numbers.** Enable the `preserve_order` feature (backs `Map` with `IndexMap` → keeps the user's input key order; a formatter must NOT silently reorder keys, which the default `BTreeMap` does alphabetically). Cost is minimal (`indexmap` → `hashbrown`). **Do NOT enable `arbitrary_precision`** in v1 — it is a documented feature-interaction landmine **combined with `preserve_order`** (serde-rs/json #505/#721/#845) and only matters for exotic large-number/high-precision literals. Cargo: `serde_json = { version = "1.0.150", features = ["preserve_order"] }` (serde pulled transitively; no `serde` derive / no direct `serde` dep needed for `Value`).
- **D-05:** **Syntax coloring = hand-rolled colorizer over `serde_json::Value` using owo-colors, gated through `is_color_on()`.** REJECTED `colored_json` — it ships a second `yansi` color stack that **bypasses** `is_color_on()` and the startup global override, which would break the locked byte-identical-minus-ANSI-when-piped rule. The non-colored path and `--compact` delegate to `serde_json::to_string_pretty` / `to_string` so piped output is byte-identical minus ANSI.
- **D-06:** **Validation / errors / format:** parse via serde_json; on parse error print `error at line {L} column {C}: {msg}` to **stderr** using `serde_json::Error::{line(), column()}` (1-based) and exit **1**; valid input exits **0**. Pretty default = **2-space indent**; **`--compact`** minifies (`to_string`). (`box`'s `error: …` stderr prefix from `main()` already wraps the message — wording within that is discretion.)

### matrix — glyph set, loop, teardown (MTRX-01)
- **D-07:** **Glyph set = pure halfwidth katakana** (U+FF66–U+FF9D) — authentic Matrix look; **halfwidth** (East-Asian-Width *Narrow*) keeps every glyph single-cell so the column/drop grid math stays clean (fullwidth would be double-width and break it). **Font requirement documented** in `--help` / README: the authentic look needs a CJK-capable font (e.g. Cascadia Next JP, or any font containing U+FF66–U+FF9D); on the bare default **Cascadia Mono** these render as **tofu** (Windows conhost does no glyph fallback) — a **documented cosmetic limitation, not a bug**. NO fallback flag / charset option (those are VIS-V2-01). ⚠️ Note: MTRX-01's "no visual artifacts" is about **clean teardown** (no leftover rain/cursor after exit), which D-10 guarantees on any font — it is independent of the glyph/tofu question.
- **D-08:** **Animation loop (crossterm 0.29, already a dep).** Setup: `enable_raw_mode()` → `execute!(EnterAlternateScreen, cursor::Hide)` → `terminal::size()`. Per-column drop state: head row position (an `i32`/`f32`, may start at a random **negative** row to stagger columns in), randomized trail length (~6–20), per-column speed; reset a column to a fresh random negative start when its head clears bottom+trail. **Render the WHOLE frame into one buffer with `queue!`, then `stdout.flush()` EXACTLY ONCE per frame** (STATE.md hard pitfall: per-character flush ≈ 5 FPS). Head cell bright (truecolor ~`(180,255,180)`/white-green), trail fades green→dark (interpolate G ~255→~40 by distance behind head), erase the cell just past the tail; fresh random glyph for the head each frame.
- **D-09:** **Frame rate ≈ 20 FPS, and the frame timer IS the input poll:** `if event::poll(Duration::from_millis(50))? { let ev = event::read()?; … }` — single-threaded, no background thread, no separate sleep.
- **D-10:** **Exit + restore.** Exit on **Ctrl+C** (`KeyCode::Char('c')` + `KeyModifiers::CONTROL` — in raw mode crossterm delivers Ctrl+C as a **KeyEvent**, NOT SIGINT), **`q`**, and **Esc**. **Filter `KeyEventKind::Press` only** (Windows fires both press AND release → otherwise double-counts). Restoration via an **RAII `Drop` guard** constructed right after setup: its `drop` runs `execute!(cursor::Show, LeaveAlternateScreen)` then `disable_raw_mode()` (errors ignored) — fires on normal return, `?` early-return, AND panic. **No `ctrlc`/signal crate** (redundant; the window-close `CTRL_CLOSE_EVENT` edge case is harmless because `LeaveAlternateScreen`'s alternate buffer means nothing persists in the user's real terminal). Satisfies MTRX-01's "exits cleanly and restores … no visual artifacts" on any font.

### lolcat — gradient & input handling (LOL-01)
- **D-11:** **Gradient = classic lolcat sine-wave RGB.** Per cell, with a running float phase `i`: `r = (sin(0.1·i + 0.0)·127 + 128) as u8`, `g = (sin(0.1·i + 2π/3)·127 + 128) as u8`, `b = (sin(0.1·i + 4π/3)·127 + 128) as u8` (120° phase offsets; full saturation/brightness floor of 128 keeps colors vivid). Advance `i` per **visible char** by its display width; seed each new line's starting phase with a **per-line vertical offset** (`line_index · spread`, spread ≈ 3.0) → the signature **diagonal** rainbow. **Fixed** frequency (≈0.1) and fixed starting phase — NO flags (`--freq`/`--seed`/`--animate` = VIS-V2-01). REJECTED horizontal-only (identical lines → vertical stripes → fails the smooth multi-line success criterion) and HSV hue-rotation (not the lolcat look; needs HSV→RGB code or a `palette` dep).
- **D-12:** **Coloring granularity = per Unicode scalar (`char`), NEVER per byte** (byte-level coloring corrupts multi-byte UTF-8). Advance the gradient phase by each char's display width via **`unicode-width`** (`UnicodeWidthChar::width(c).unwrap_or(0)`): wide/CJK glyphs (width 2) advance correctly; combining marks (width 0) inherit the base color. Whitespace advances the phase but is emitted **uncolored** (a colored space is invisible); newlines emitted raw. NEW dep `unicode-width` 0.2 (tiny, `no_std`, table-only, canonical unicode-rs).
- **D-13:** **Strip pre-existing ANSI from input UNCONDITIONALLY** (before recoloring, and even on the no-color path) via **`strip-ansi-escapes`** (`strip_str`). Prevents garbled output when input already carries ANSI (e.g. `ls --color | box lolcat`) and stops stray escape bytes from skewing the gradient. NEW dep `strip-ansi-escapes` 0.2.1 (one transitive dep, `vte` 0.14, well-maintained) — this **actions the standing STATE.md todo**.
- **D-14:** **Gating:** apply `.truecolor(r,g,b)` (owo-colors, already wired) only when `is_color_on()` is true; when off, write the stripped plain text through unchanged → piping to a file yields clean plain text (LOL-01 criterion). Input via `core::input::read_input` (stdin / positional / `--file` / `-`).

### Claude's Discretion
- Module layout under `src/commands/<cmd>/mod.rs` per command; whether `json`/`lolcat` use `read_input` vs the streaming `read_file_or_stdin` (as long as it routes through `core::input`).
- ascii: the exact dark→light ramp string and `FilterType` (Triangle suggested), exact luma weighting, and whether to trim `image` default features to just PNG/JPEG for binary size.
- json: exact color shades per token type (key / string / number / bool / null / punctuation) and exact error-string wording within the locked `error at line L column C: …` shape.
- matrix: exact trail-length range, per-column speed model, head/trail RGB shades, exact FPS within ~15–25, and which katakana sub-range/density to draw from.
- lolcat: exact `freq`/`spread` constants within the smooth-gradient target (`freq≈0.1`, `spread≈3.0`) and the starting phase.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements & success criteria (the binding contract)
- `.planning/ROADMAP.md` § "Phase 4: Terminal Visuals" — goal + the 4 success criteria (exact CLI behaviors: `lolcat` truecolor-in-TTY/plain-when-piped, `matrix` full-terminal rain + clean Ctrl+C restore, `ascii ./photo.jpg` width-fitted PNG+JPEG, `json` invalid→exit 1 + line/col, valid→colored pretty, `--compact` minify).
- `.planning/REQUIREMENTS.md` § "Terminal Visuals" (LOL-01, MTRX-01, ASCI-01) + § "Dev Utilities" (JSON-01) acceptance criteria; § "v2 Requirements" (VIS-V2-01 — what's deferred); § "Out of Scope" (json no jq, ascii no video/GIF/URL).

### Architecture & locked patterns to reuse (do NOT re-derive)
- `.planning/STATE.md` § "Architecture Established", § "Critical Pitfalls to Remember" (⚠️ **matrix: buffer full frame, flush once per frame**; `enable_ansi_support` first in `main()`), § "Key Decisions", § "Todos" (the **`strip-ansi-escapes` for lolcat** todo — actioned by D-13). `box` is binary-only → `cargo test --bin box`, not `--lib`.
- `.planning/phases/01-foundation-flatten/01-CONTEXT.md` — D-09/D-10 (glyph-is-truth output + `is_color_on()` gating, byte-identical-minus-ANSI when piped), the strict 0/1/2 exit policy.
- `.planning/phases/02-pure-transform-utilities/02-CONTEXT.md` — D-04/D-05 (`core::input` precedence + `read_input` shape, the `-` sentinel + `--file` extension point) used by `json`/`lolcat`.
- `.planning/phases/03-filesystem-power-tools/03-CONTEXT.md` — D-05 (the `read_file_or_stdin` streaming `--file` layer) and the `core::output` promotion convention (pattern reference).

### Source files to read before implementing
- `src/cli.rs` — the 4 unit variants `Lolcat` / `Matrix` / `Ascii` / `Json` to swap for real Args structs.
- `src/main.rs` — the 4 `not_implemented(...)` arms to remove (lines ~76-79); the exit-code policy + `BoxError` downcast mapping (exit 2 only for `MissingInput`/`UnsupportedHashLength`; everything else err→exit 1).
- `src/core/output.rs` — `is_color_on()` (`:32`), `init_color()` (`:52`), `truncate_middle()` (`:174`), `human_size()` (`:198`), `terminal_width()` (`:215`).
- `src/core/input.rs` — `read_input` / `read_file_or_stdin` + `-` sentinel + `--file` (json/lolcat input).
- `src/commands/color/mod.rs` — owo-colors `.truecolor(r,g,b)` usage precedent + `is_color_on()`-gated swatch (the exact pattern `lolcat`/`json`/`matrix` color follows).
- `src/commands/cowsay/mod.rs` — hand-rolled render + the fixed-width-vs-`terminal_width()` reproducibility precedent (informs ascii width handling under piping).

### Tech stack (locked crate versions — use as-is, do not re-research)
- `CLAUDE.md` (project root) — `image` 0.25.10 (ascii, NEW dep), **`artem` 3.0.0 = REJECTED** (D-01), `serde_json` 1.0.150 (json, NEW — add `preserve_order`), `crossterm` 0.29.0 (matrix, already present), `owo-colors` 4.3.0 (already present), `enable-ansi-support` 0.3.1 (already wired). NEW small deps not in CLAUDE.md: `unicode-width` 0.2 + `strip-ansi-escapes` 0.2 (lolcat, D-12/D-13). The "What NOT to Use" table still applies.

**No external ADRs/specs exist** — all decisions are captured above and in the files listed.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `core::output::{is_color_on, init_color}` + the startup owo-colors global override — every styled output (lolcat truecolor, json token colors, matrix head/trail) routes through this so piped output is byte-identical minus ANSI; no parallel color path.
- `core::output::terminal_width()` — ascii `cols` source (falls back to 80 when piped); also available to any wrapped output.
- `core::input::{read_input, read_file_or_stdin}` + `-` sentinel + `--file` — `json` and `lolcat` input; design proven by Phase 2/3 consumers.
- `owo_colors::OwoColorize::truecolor(r,g,b)` — already in the stack and gated; lolcat/json/matrix all use it. `enable_ansi_support` already called first in `main()` so truecolor renders in PS7.
- `crossterm` 0.29 — already a dep; matrix uses `terminal::{enable/disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, size}`, `cursor::{Hide, Show, MoveTo}`, `event::{poll, read, KeyEvent, KeyCode, KeyModifiers, KeyEventKind}`, `queue!`/`execute!`.

### Established Patterns
- `RunCommand` trait + static dispatch (`src/commands/mod.rs`, `src/main.rs`): swap the `src/cli.rs` unit variant, remove the `not_implemented(...)` arm.
- Glyph-is-source-of-truth output, color as decoration, byte-identical-minus-ANSI when piped (Phase 1 D-09/D-10); `data→stdout`, `messages→stderr`, exit 0/1/2.
- Hand-roll over convenience crates when a dep's cost outweighs its value (color, cowsay; rejected humansize/ignore/passwords) — D-01 (artem) and D-05 (colored_json) continue this.
- Tests: integration via `assert_cmd` in `tests/<cmd>.rs`; CLI snapshots via `trycmd` (normalizes `\`→`/` in Windows path snapshots). matrix's animation/raw-mode loop is hard to snapshot — prefer unit-testing the pure pieces (drop/fade model, glyph table) + a smoke test that the binary enters/exits cleanly.

### Integration Points
- New `Cargo.toml` deps: `image = "0.25.10"` (ascii), `serde_json = { version = "1.0.150", features = ["preserve_order"] }` (json), `unicode-width = "0.2"` + `strip-ansi-escapes = "0.2"` (lolcat). `crossterm`/`owo-colors` already present (matrix/lolcat/json color).
- Each command removes one `not_implemented(...)` arm in `src/main.rs` and swaps one unit variant in `src/cli.rs`.
- No `core::*` surface changes are required by these four (unlike Phase 3's `human_size` promotion) — they consume the existing helpers as-is.

</code_context>

<specifics>
## Specific Ideas

- **lolcat** gradient: `r/g/b = sin(0.1·i + {0, 2π/3, 4π/3})·127 + 128`; advance `i` per visible char by `unicode-width`; per-line offset `line_index·~3.0` for the diagonal; strip incoming ANSI first; gate via `is_color_on()` so `box lolcat … > file` is clean plain text.
- **matrix**: pure halfwidth katakana U+FF66–U+FF9D; alt-screen + hide cursor + raw mode; full-frame buffer flushed once per frame at ~20 FPS via `event::poll(50ms)` as the timer; bright head + green→dark trail; RAII Drop guard restores cursor + leaves alt-screen + disables raw mode on every exit incl. panic; exit on Ctrl+C / q / Esc; `KeyEventKind::Press` filtered; font-requirement note in `--help`.
- **ascii**: `image::open` → `resize_exact(cols, rows, Triangle)` → `to_luma8()` → dark→light ramp; `cols = terminal_width()`, `rows = cols·src_h/src_w/2`; PNG + JPEG; monochrome v1.
- **json**: `preserve_order` (keep input key order); default numbers (no `arbitrary_precision`); hand-rolled owo-colors colorizer gated via `is_color_on()`; non-color + `--compact` delegate to `to_string_pretty`/`to_string`; error `error at line {L} column {C}: {msg}` → stderr, exit 1; 2-space indent.

</specifics>

<deferred>
## Deferred Ideas

- **VIS-V2-01** (REQUIREMENTS.md): lolcat `--animate`/`--freq`/`--seed`; matrix configurable color/speed/charset; ascii color/braille/invert modes. The fixed constants in D-09/D-11 are chosen so these flags graft on cleanly later.
- **matrix Latin/ASCII fallback charset flag** (e.g. `--latin`) — explicitly considered as the katakana font-risk mitigation and **deferred** to VIS-V2-01 charset options; v1 ships pure katakana + a documented font requirement (D-07) instead.
- **json `arbitrary_precision`** (exact large-number / high-precision fidelity) — deferred behind a future flag; revisit only if a concrete fidelity need appears (documented landmine when combined with `preserve_order`).
- **ascii colored output** — VIS-V2-01; the D-02 ramp emit is the seam where v2 gates truecolor per-cell through `is_color_on()`.
- **ascii video / GIF / URL input** and **json full jq query language** — Out of Scope (REQUIREMENTS.md), not v2.

### Reviewed Todos (not folded)
None — `gsd-sdk query todo.match-phase 4` returned 0 matches.

</deferred>

---

*Phase: 4-terminal-visuals*
*Context gathered: 2026-06-23*
