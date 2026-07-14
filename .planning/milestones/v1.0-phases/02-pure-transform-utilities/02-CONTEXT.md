# Phase 2: Pure Transform Utilities - Context

**Gathered:** 2026-06-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Build **nine zero-external-dependency commands** that prove the shared `RunCommand` pattern on low-risk surfaces (no network, no OS APIs beyond stdin/stdout) before the filesystem (Phase 3), visual (Phase 4), and platform (Phase 5) phases:

`uuid`, `base64`, `epoch`, `color`, `passgen`, `cowsay`, `fortune`, `8ball`, `roast`.

Each command swaps its unit `Commands::*` variant in `src/cli.rs` for a real Args struct implementing `RunCommand`, removes its `not_implemented(...)` arm in `src/main.rs`, and ships with integration tests (`assert_cmd`/`trycmd`). All reuse the Phase 1 core: `core::output` color gating (D-10), strict 0/1/2 exit codes, `data→stdout / messages→stderr`.

**In scope:** UUID-01, B64-01, EPOC-01, COLR-01, PASS-01, COW-01, FORT-01, 8BAL-01, ROST-01 (9 requirements) + one new shared helper `core::input`.

**Not in scope:** the 10 commands in Phases 3–5 (`hash`, `tree`, `du`, `dupes`, `bulk-rename`, `lolcat`, `matrix`, `ascii`, `json`, `qr`, `clip`, `pomodoro`, `weather`); cowsay alternate cow packs / `--think`; color `--float` form; epoch timezone-by-name output; passgen strength-scoring output.

</domain>

<decisions>
## Implementation Decisions

### Dependency strategy — crate vs hand-roll (per command)
- **D-01:** **Lean bundle.** Use crates only where hand-rolling is pure downside; hand-roll where the value is formatted output or bundled content.
  - **Crate** `uuid` 1.23.3 `features=["v4"]` — pulls only `getrandom`; avoids botching RFC-4122 variant/version bit-masking. `Display` already gives lowercase-hyphenated; `--upper` = `.to_string().to_uppercase()`.
  - **Crate** `base64` 0.22.1 — `STANDARD` and `URL_SAFE_NO_PAD` engines; zero transitive deps. Do not hand-roll alphabets/padding.
  - **Crate** `chrono` 0.4.45 — needed for Windows local-timezone conversion (a trap to hand-roll). Trim default features where possible (drop `oldtime`/`wasmbind`).
  - **Hand-roll** `passgen`, `color`, `cowsay`, `fortune`, `8ball`, `roast`.
- **D-02:** **passgen is hand-rolled on `rand` 0.9, NOT the `passwords` crate.** Rationale: `passwords` 3.1.16 pins `rand` 0.8 (duplicating the 0.9 used by the whimsy commands), drags `random-pick`/`random-integer`, ships unused bcrypt/md5, and has no passphrase mode. Both paths are ChaCha12 CSPRNG-grade, so security is a wash — the lean-binary goal is the tiebreaker. ⚠️ **Hand-rolled passgen MUST sample with `IndexedRandom::choose` / `Rng::gen_range`, never `% len`** (modulo bias).
- **D-03:** **color is hand-rolled** (closed-form hex↔RGB↔HSL math, ~50 lines incl. HSL; unit-test the HSL round-trip). Reach for `csscolorparser` only if full CSS-color *input* syntax becomes a future feature. **cowsay is hand-rolled** (greedy word-wrap ~30 lines + embedded cow art); no canonical cowsay crate is worth a dep.

### Input source convention — reusable `core::input` helper (NEW shared surface)
- **D-04:** **Auto-detect, layered.** Add `src/core/input.rs` registered in `src/core/mod.rs`, mirroring the existing `std::io::IsTerminal` gate in `core::output` (output.rs:14, 42-44). Precedence:
  1. positional `arg` is `Some` and not `"-"` → use it;
  2. `arg` is `None` or `"-"` → if `stdin().is_terminal()` is **false** (piped) read stdin to EOF;
  3. `arg` is `None` and stdin **is** an interactive TTY → **do NOT block**; return a usage error to stderr (exit 2): "no input: pass an argument or pipe data".
- **D-05:** Two helper shapes: `read_input(arg: Option<String>) -> anyhow::Result<String>` (UTF-8 text: cowsay, epoch, color) and `read_input_bytes(arg) -> anyhow::Result<Vec<u8>>` (binary-exact: base64). This satisfies both Phase-2 success criteria unchanged (`box cowsay "hi"` → branch 1; `… | box base64` → branch 2) and kills the interactive-hang footgun.
- **D-06:** **PowerShell 7 piping caveat:** PS7's pipeline re-encodes native-command output through .NET strings (UTF-16). Read text as UTF-8 `String`; read base64 bytes via `read_to_end` into `Vec<u8>`. A future `--file PATH` layer (Phase 3 `hash`/`json`) sidesteps PS pipe re-encoding for byte-exact commands — defer it, but design `core::input` so it can be added without reshaping.

### Bundled data & randomness
- **D-07:** **Embedding = `include_str!` text files** for the large/edited lists (smallest binary, fastest compile, auditable, easy to re-source/attribute): EFF wordlist, fortune, roast under e.g. `src/data/`. **`const &[&str]`** for the canonical 20 `8ball` answers (small, readable in-source, lets you tag tone). No build-script/`phf` — these commands need indexed random pick, not keyed lookup.
- **D-08:** **RNG = split.** `OsRng` (Windows BCryptGenRandom via getrandom) for `passgen` — non-negotiable CSPRNG. `rand::rng()` (ThreadRng, ChaCha12, OS-seeded) for `fortune`/`8ball`/`roast`. Each `box` run is a fresh process that reseeds from the OS, so repeated calls differ as required — **no manual/fixed seed anywhere**. Unbiased selection via `choose` / `random_range` only.
- **D-09:** **Content sources/counts:**
  - passphrase: **EFF Large (Diceware) 7776-word list** (strip leading dice codes; keep words only). ~12.9 bits/word → `--words 4` ≈ 51.6 bits. **CC-BY 3.0 US — attribution required** (see canonical refs). Do not use the short EFF lists (weaker entropy/word). ~77 KB blob accepted against the lean-binary goal.
  - 8ball: **canonical 20 answers** (10 affirmative / 5 non-committal / 5 negative) — do not pad.
  - fortune: ~50–150 curated **original / public-domain / CC0** aphorisms (avoid bundling BSD `fortune` datfiles wholesale — mixed licensing).
  - roast: ~30–80 **self-authored / CC0** programmer one-liners.

### color & cowsay output design
- **D-10:** **`color` output** = aligned `label : value` block with rows **Hex**, **RGB (CSS `rgb(r, g, b)`)**, **Tuple (`r g b`)**, **HSL**, followed by a **foreground `██…` truecolor swatch** via `owo_colors`'s `.truecolor(r,g,b)`. The foreground-block swatch is the ONLY option that degrades correctly under the locked `COLOR_ON` gating — when piped/`NO_COLOR`, the `██` glyphs survive as a still-meaningful line, byte-identical minus ANSI, with zero special-casing. A background-ANSI swatch (`\x1b[48;2…m` + spaces) is **rejected** — it becomes a blank line when stripped and needs a banned parallel color path. Reuse `core::output::{is_color_on, terminal_width}`; the only new pure helper is an `hsl(r,g,b)` conversion. Target layout:
  ```
    Hex   : #3B82F6
    RGB   : rgb(59, 130, 246)
    Tuple : 59 130 246
    HSL   : hsl(217, 91%, 60%)

    ██████████
  ```
- **D-11:** **`cowsay` output** = classic cow + classic speech bubble, **fixed 40-column word-wrap** with a `--width N` override. Fixed beats wrapping to `terminal_width()` because `terminal_width()` falls back to 80 when piped → non-reproducible pipe-vs-TTY output. Standard border switch: single-line `< text >`; multi-line `/ \`, `| |`, `\ /` with shorter lines space-padded to the longest. Hard-break any single word longer than the width. Pure ASCII (Phase 1 "glyph is the source of truth" rule).

### Per-command behavioral details (resolved follow-ups)
- **D-12:** **`epoch` date-string formats** = ISO 8601 / RFC 3339 (`2026-06-22T14:30:00Z`) **plus** `YYYY-MM-DD` and `YYYY-MM-DD HH:MM:SS` (the latter two assumed **local** time). Anything else → clear error with a hint. No `MM/DD/YYYY`-style ambiguous formats. (No-arg → current Unix timestamp; integer arg → local + UTC human date.)
- **D-13:** **`color` input is bidirectional, auto-detected** — accept hex (`#3b82f6`, `3b82f6`, `#abc` short form) **and** RGB (`"59,130,246"` or space-separated `59 130 246`); detect which was given and print all representations + swatch either way.
- **D-14:** **`passgen` default charset** = lower + upper + digits + symbols, but a **curated symbol subset that avoids shell/quoting-hostile characters** (so passwords paste cleanly into PowerShell). Provide flags to restrict (e.g. `--no-symbols`); default length 16; `--words N` passphrase; `--count N` bulk; **all output to stdout only**.

### Claude's Discretion
- Exact `core::input` error wording and whether the bytes/string helpers share an inner reader.
- Module layout under `src/commands/<cmd>/mod.rs` (note: `8ball` → module `eight_ball` per STATE.md naming pitfall); whether `color` HSL helper lives in the color module or `core`.
- The specific curated symbol set for passgen and the exact `--charset`/restriction flag surface (as long as the default is the four-class curated-symbol set and CSPRNG sampling is unbiased).
- cowsay's exact cow art bytes and `--width` default/clamp behavior beyond the locked fixed-40 default.
- Exact fortune/roast wording and final counts within the ranges above; the data-file directory name.
- `uuid` flag spelling beyond the locked `-n N` (count) and `--upper`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements & success criteria
- `.planning/ROADMAP.md` § "Phase 2: Pure Transform Utilities" — goal + the 5 success criteria (the exact CLI behaviors: `uuid -n 5`/`--upper`, base64 `--decode`/`--url-safe`, epoch 3 modes, passgen 16-char/`--words 4`/`--count 10`, the four whimsy commands).
- `.planning/REQUIREMENTS.md` § acceptance criteria for UUID-01, B64-01, EPOC-01, COLR-01, PASS-01, COW-01, FORT-01, 8BAL-01, ROST-01.

### Architecture & locked patterns to reuse (do NOT re-derive)
- `.planning/STATE.md` § "Key Decisions", § "Critical Pitfalls to Remember" — RunCommand trait, `8ball`→`eight_ball` module name, `dunce::canonicalize`, ANSI bootstrap order, MSVC + crt-static.
- `.planning/phases/01-foundation-flatten/01-CONTEXT.md` — D-05 (clap-derive variants + dispatch), D-10 (color gating: `COLOR_ON` from `--no-color ∧ NO_COLOR ∧ TTY`), data→stdout/messages→stderr, exit-code policy.
- `src/cli.rs` — the `Commands` enum; each Phase-2 command currently a unit variant to be swapped for a real Args struct (note `#[command(name = "8ball")]`).
- `src/main.rs` — dispatch + 0/1/2 exit mapping; remove each command's `not_implemented(...)` arm as it's built.
- `src/core/output.rs` — `is_color_on()` (output.rs:32), `terminal_width()` (output.rs:195), `truncate_middle()`, `init_color()`, the owo-colors global-override gating pattern (output.rs:121-129) that the color swatch MUST follow.
- `src/core/mod.rs` — register the new `input` module here.

### Tech stack (locked crate versions — use as-is, do not re-research)
- `CLAUDE.md` (project root) — confirmed versions: `uuid` 1.23.3 (`v4`), `base64` 0.22.1, `chrono` 0.4.45, `passwords` 3.1.16 (rejected for passgen per D-02), `rand` 0.9.x (OsRng for passgen), plus the "What NOT to Use" table.

### Bundled-content licensing (MANDATORY attribution)
- **EFF Long/Large Wordlist** — © Electronic Frontier Foundation, **CC-BY 3.0 US**. Source: https://www.eff.org/dice (file `eff_large_wordlist.txt`). Add an attribution line to README / `--help` / a `LICENSE-THIRD-PARTY` note: *"Passphrase wordlist: EFF Long Wordlist, © Electronic Frontier Foundation, CC-BY 3.0 US."*

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `core::output::{is_color_on, terminal_width, truncate_middle, init_color}` — color/width helpers already gate once at startup; `color` swatch and any styled output reuse these (no parallel color path).
- `owo_colors::OwoColorize::truecolor(r,g,b)` — already available (CLAUDE.md stack); used for the color swatch. `enable_ansi_support` is already called first in `main()`, so truecolor renders in PS7.
- The `RunCommand` trait + static dispatch in `src/commands/mod.rs` and `src/main.rs` — pattern each new command follows.

### Established Patterns
- `std::io::IsTerminal` gate (output.rs) is the exact shape `core::input` mirrors for the stdin auto-detect (D-04).
- Phase 1 output philosophy: ASCII glyphs are machine-readable source of truth, color is decoration only, output is byte-identical minus ANSI when piped — `color`/`cowsay` honor this.
- Tests: integration via `assert_cmd` in `tests/<cmd>.rs`; CLI snapshots via `trycmd` (`tests/cmd/*.trycmd`).

### Integration Points
- **NEW** `src/core/input.rs` (registered in `src/core/mod.rs`) — the shared stdin/arg reader; first consumed by base64/cowsay/epoch, later by hash/json/clip/lolcat. Design the `--file` extension point now (D-06), implement it in Phase 3.
- **NEW** `src/data/` (or similar) — embedded `include_str!` text assets (EFF wordlist, fortune, roast).
- Each command removes one `not_implemented(...)` arm in `src/main.rs` and swaps one unit variant in `src/cli.rs`.

</code_context>

<specifics>
## Specific Ideas

- color command target output (color ON; byte-identical minus ANSI when piped):
  ```
    Hex   : #3B82F6
    RGB   : rgb(59, 130, 246)
    Tuple : 59 130 246
    HSL   : hsl(217, 91%, 60%)

    ██████████        (renders in the actual color via .truecolor(59,130,246))
  ```
- cowsay single-line target:
  ```
   _____________
  < Hello, box! >
   -------------
          \   ^__^
           \  (oo)\_______
              (__)\       )\/\
                  ||----w |
                  ||     ||
  ```
- 8ball answer set = the authentic Magic 8-Ball 20 (10 affirmative / 5 non-committal / 5 negative).
- passgen passwords must be paste-safe in PowerShell — curated symbol set, no shell-hostile chars.

</specifics>

<deferred>
## Deferred Ideas

- `--file PATH` input flag for byte-exact commands (sidesteps the PS7 UTF-16 pipe re-encoding) — design the `core::input` extension point now, implement in Phase 3 (`hash`/`json`).
- cowsay alternate cow packs + `--think` thought-bubble variant — follow-up, not Phase 2.
- color `--float` (normalized 0-1) representation row — niche (shader/GL); future flag.
- passgen strength-scoring output (the feature `passwords` crate offers) — not required; revisit only if asked.
- epoch timezone-by-name output (`chrono-tz`) — out of scope; current scope is local + UTC only.

</deferred>

---

*Phase: 2-pure-transform-utilities*
*Context gathered: 2026-06-22*
