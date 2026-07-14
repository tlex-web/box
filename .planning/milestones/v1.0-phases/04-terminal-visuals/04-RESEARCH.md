# Phase 4: Terminal Visuals - Research

**Researched:** 2026-06-24
**Domain:** Terminal-visual Rust CLI commands (gradient text, animation, image→ASCII, JSON formatting) on the existing `box` core
**Confidence:** HIGH

## Summary

This phase is **API de-risking, not design**. `04-CONTEXT.md` already locked 14 decisions (D-00..D-14) via dedicated advisor research — exact crate versions, gradient math, glyph ranges, the rendering pipeline, the animation-loop shape, and the JSON colorizer approach. The job here was to (1) confirm every locked crate API is real and current for the pinned version, (2) surface implementation landmines beyond what STATE.md records, and (3) map each requirement to the real reusable symbols already in the tree. **Every locked API verified against official docs.rs and crates.io for the exact pinned version — zero drift, zero deprecation, zero signature surprise.** All four new crates pass slopcheck `OK` against crates.io and are canonical (image-rs / dtolnay / unicode-rs / luser).

The four commands are independent vertical slices that all reuse Phase-1/2/3 infrastructure unchanged: `core::output::{is_color_on, init_color, terminal_width}` for the locked byte-identical-minus-ANSI color gate, `core::input::{read_input, read_file_or_stdin}` for json/lolcat input, the `RunCommand` trait + `not_implemented` arms in `src/main.rs` for registration, and the `owo_colors::OwoColorize::truecolor` (verified unconditional → must gate on `is_color_on()`) for all color. No `core::*` surface change is required.

**Primary recommendation:** Implement each command as one thin slice exactly per its locked decision block. The only API nuances the planner must encode as task notes are: (a) `.dimensions()` requires `use image::GenericImageView`; (b) `GrayImage::as_raw()` is the clean row-major luma source for the ASCII ramp; (c) `image::open` detects format from **file extension**, not content; (d) `serde_json::to_string_pretty` is fixed at **2-space** indent (matches D-06 — no `PrettyFormatter` needed); (e) `KeyEventKind` has Press/Repeat/Release → filter `Press` only (Windows double-fire); (f) `.truecolor` is unconditional so every cell/glyph color path must consult `is_color_on()`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| lolcat gradient colorize | CLI command logic (`src/commands/lolcat`) | `core::output` (color gate) + `core::input` (text in) | Pure per-char RGB math over stripped text; color is a decoration gated by the shared startup decision |
| matrix digital-rain animation | CLI command logic (`src/commands/matrix`) | `crossterm` (raw mode / alt screen / events) | Stateful per-column drop model + single-flush render loop; the only command that owns terminal raw-mode state and an RAII teardown guard |
| ascii image→art | CLI command logic (`src/commands/ascii`) | `image` (decode/resize/luma) + `core::output::terminal_width` | Decode is delegated to `image`; the ramp mapping + width math is local pure logic |
| json pretty/validate/minify | CLI command logic (`src/commands/json`) | `serde_json` (parse/serialize) + `core::input` + `core::output` (color gate) | Parsing/serialization delegated to serde_json; the colorizer walks `Value` and gates per token |

All four are **CLI-command-tier** capabilities. None touches a network, filesystem-walk, or persistence tier. `matrix` is the only one that mutates terminal state and therefore the only one needing a restore guard.

## Standard Stack

> Locked by `04-CONTEXT.md` D-01..D-14 and `CLAUDE.md`. NOT re-derived. Versions below were re-verified against crates.io on 2026-06-24 — all are the current latest stable.

### Core (new this phase)
| Library | Version (locked) | Latest verified | Purpose | Why Standard |
|---------|------------------|-----------------|---------|--------------|
| `image` | 0.25.10 | 0.25.10 (2026-03-10) [VERIFIED: crates.io] | ascii: decode PNG/JPEG → resize → luma (D-01/D-02) | de-facto Rust image library; the legitimate "image decoding" hand-roll exception (D-01). `artem` REJECTED (drags clap/colored/ureq, bypasses `terminal_width()`) |
| `serde_json` | 1.0.150 + `preserve_order` | 1.0.150 (2026-05-21) [VERIFIED: crates.io] | json: parse / pretty / minify / line+col errors (D-04/D-06) | dtolnay canonical; `preserve_order` backs `Map` with `IndexMap` to keep input key order (D-04) |
| `unicode-width` | 0.2 (latest 0.2.2) | 0.2.2 (2025-10-06) [VERIFIED: crates.io] | lolcat: advance gradient phase by char display width (D-12) | unicode-rs canonical UAX#11 table; tiny, table-only |
| `strip-ansi-escapes` | 0.2.1 | 0.2.1 (2025-01-14) [VERIFIED: crates.io] | lolcat: strip incoming ANSI unconditionally (D-13) | luser canonical; one transitive dep (`vte`). Actions the standing STATE.md todo |

### Supporting (already present — no manifest change)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `crossterm` | 0.29 | matrix: raw mode, alt screen, `event::poll/read`, `queue!`/`execute!`, `terminal::size`, cursor hide/show | matrix only (already a dep; `terminal_width()` already uses `terminal::size`) |
| `owo-colors` | 4.3 (`supports-colors`) | `.truecolor(r,g,b)` for lolcat / json token color / matrix head+trail | all three colored commands; **unconditional** so always gate on `is_color_on()` |
| `enable-ansi-support` | 0.3 | already called first in `main()` | nothing new — ANSI already bootstrapped for PS7 |

### Alternatives Considered (already rejected in CONTEXT — do not relitigate)
| Instead of | Could Use | Tradeoff / why rejected |
|------------|-----------|--------------------------|
| hand-roll on `image` | `artem` 3.0.0 | D-01: unconditional deps (clap/colored/terminal_size/log/env_logger/once_cell + ureq/TLS via default `web_image`); bypasses `terminal_width()` |
| hand-rolled owo-colors json colorizer | `colored_json` | D-05: ships a second `yansi` color stack that bypasses `is_color_on()` → breaks byte-identical-minus-ANSI |
| sine-wave RGB gradient | HSV hue-rotation | D-11: not the lolcat look; needs HSV→RGB code or a `palette` dep |
| RAII Drop guard restore | `ctrlc`/signal crate | D-10: redundant — in raw mode crossterm delivers Ctrl+C as a KeyEvent, not SIGINT |

**Installation (Cargo.toml `[dependencies]` additions):**
```toml
# ascii (D-01/D-02). Discretion: trim defaults to PNG/JPEG only (see Pitfall 6).
image = "0.25.10"
# or, default-trimmed:  image = { version = "0.25.10", default-features = false, features = ["png", "jpeg"] }
# json (D-04). preserve_order pulls indexmap→hashbrown; serde comes transitively (Value needs no derive).
serde_json = { version = "1.0.150", features = ["preserve_order"] }
# lolcat (D-12/D-13)
unicode-width = "0.2"
strip-ansi-escapes = "0.2"
```
`crossterm`/`owo-colors`/`enable-ansi-support` are already in the manifest — matrix and all color paths need no manifest change.

**Version verification (run 2026-06-24):** crates.io API confirmed `image` max_stable = 0.25.10, `serde_json` = 1.0.150, `unicode-width` = 0.2.2, `strip-ansi-escapes` = 0.2.1. Every locked version IS the current latest stable — no upgrade and no stale-pin risk.

## Package Legitimacy Audit

> slopcheck WAS available and ran successfully against the `crates.io` ecosystem.

| Package | Registry | Age | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-------------|-----------|-------------|
| `image` | crates.io | mature (0.25.x line since 2024) | github.com/image-rs/image | OK | Approved [VERIFIED: crates.io] |
| `serde_json` | crates.io | mature (>1B downloads) | github.com/serde-rs/json | OK | Approved [VERIFIED: crates.io] |
| `unicode-width` | crates.io | mature (101.8M downloads) | github.com/unicode-rs/unicode-width | OK | Approved [VERIFIED: crates.io] |
| `strip-ansi-escapes` | crates.io | mature (20.6M downloads on 0.2.1) | github.com/luser/strip-ansi-escapes | OK | Approved [VERIFIED: crates.io] |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

All four are canonical crates from well-known ecosystem maintainers, verified BOTH by slopcheck (`status: OK`, no flags) AND by official docs.rs documentation, so they qualify for `[VERIFIED: crates.io]`. No `checkpoint:human-verify` gate is required.

## API Verification (the de-risking deliverable)

Every locked-decision API was checked against docs.rs **for the exact pinned version**. Results:

### `image` 0.25.10 (ascii — D-01/D-02)
| Symbol | Status | Note for planner |
|--------|--------|------------------|
| `image::open(path) -> ImageResult<DynamicImage>` | [VERIFIED: docs.rs/image/0.25.10] | `P: AsRef<Path>`. **Detects format from file EXTENSION, not content** — a JPEG renamed `.png` errors/mis-decodes. For content sniffing use `ImageReader::open(p)?.with_guessed_format()?.decode()` (NOT needed for v1; document the extension behavior in `--help`). |
| `DynamicImage::resize_exact(nw: u32, nh: u32, FilterType) -> DynamicImage` | [VERIFIED] | Exact-size, does NOT preserve aspect — correct, since D-02 computes `rows` itself. |
| `FilterType::Triangle` | [VERIFIED] | One of {Nearest, Triangle, CatmullRom, Gaussian, Lanczos3}. Triangle (linear) is a good cost/quality default (D-02 / discretion). |
| `DynamicImage::to_luma8() -> GrayImage` | [VERIFIED] | `GrayImage = ImageBuffer<Luma<u8>, Vec<u8>>`. |
| `.dimensions() -> (u32, u32)` | [VERIFIED — TRAIT METHOD] | ⚠️ On the `GenericImageView` trait, NOT inherent. **Must `use image::GenericImageView`** to call `img.dimensions()`. Alternatively use the inherent `img.width()` / `img.height()` (no import). |
| luma byte access | [VERIFIED] | `GrayImage::as_raw() -> &Vec<u8>` is already row-major (x 0..w then y 0..h) — the cleanest source for the ramp loop. `pixels()` / `get_pixel(x,y)` also work; `pixel[0]` is the luma byte. |
| `Limits` / `set_limits` | [CITED: docs.rs/image] | Exists for decompression-bomb protection but `image::open` uses **default (effectively unlimited)** limits — see Pitfall 6. |

### `serde_json` 1.0.150 + `preserve_order` (json — D-04/D-05/D-06)
| Symbol | Status | Note for planner |
|--------|--------|------------------|
| `serde_json::Value` {Null, Bool(bool), Number(Number), String(String), Array(Vec<Value>), Object(Map<String,Value>)} | [VERIFIED: docs.rs/serde_json/1.0.150] | These 6 variants are the colorizer's match arms (D-05). `Map` = `IndexMap` under `preserve_order` (insertion order preserved). |
| `serde_json::from_str::<Value>(s)` | [VERIFIED] | Parse + validate in one call; on `Err` use `.line()`/`.column()`. |
| `Error::line() -> usize`, `Error::column() -> usize` | [VERIFIED] | **Both 1-based** (line 1 / column 1 = first char) — matches D-06 "1-based". Also `classify()`, `is_syntax()`, `is_eof()` available. |
| `serde_json::to_string_pretty(&Value) -> Result<String>` | [VERIFIED] | **Default indent is exactly 2 spaces** — matches D-06's locked 2-space requirement with NO `PrettyFormatter` customization needed. |
| `serde_json::to_string(&Value) -> Result<String>` | [VERIFIED] | Compact/minified — the `--compact` path (D-06). |
| `preserve_order` feature | [VERIFIED] | Switches `Map` from `BTreeMap` (alpha-sorted) to `IndexMap` (input order). Confirms D-04. Do NOT also enable `arbitrary_precision` (D-04 landmine). |

### `crossterm` 0.29 (matrix — D-08/D-09/D-10)
| Symbol | Status | Note for planner |
|--------|--------|------------------|
| `event::poll(Duration) -> Result<bool>` | [VERIFIED: docs.rs/crossterm/0.29.0] | The 50ms poll IS the frame timer (D-09). |
| `event::read() -> Result<Event>` | [VERIFIED] | Call only when `poll` returns true. |
| `KeyEvent` / `KeyCode` / `KeyModifiers` | [VERIFIED] | `KeyCode::Char('c')` + `KeyModifiers::CONTROL` for Ctrl+C; `KeyCode::Char('q')`, `KeyCode::Esc` for the other exits (D-10). |
| `KeyEventKind` {Press, Repeat, Release} | [VERIFIED] | ⚠️ Three variants. **Filter `KeyEventKind::Press` only** — Windows fires Press AND Release → otherwise the exit key double-counts / a hold repeats (D-10). `is_press()` helper exists. |
| `terminal::size() -> Result<(u16,u16)>` | [VERIFIED] | `(columns, rows)`. Same call `terminal_width()` already wraps. |
| `enable_raw_mode()` / `disable_raw_mode()` | [VERIFIED] | Setup / teardown (D-08/D-10). |
| `EnterAlternateScreen` / `LeaveAlternateScreen` | [VERIFIED] | Command structs usable with `execute!`/`queue!`. Alt screen means nothing persists in the user's real terminal after exit (D-10 / MTRX-01 "no artifacts"). |
| `cursor::{Hide, Show, MoveTo}` | [VERIFIED] | `queue!(stdout, MoveTo(x,y), ...)` then ONE `stdout.flush()` per frame. |
| `queue!` + `stdout.flush()` | [VERIFIED] | Queued commands execute only on flush — exactly the single-flush-per-frame model (D-08, STATE.md hard pitfall). |

### `unicode-width` 0.2 / `strip-ansi-escapes` 0.2.1 / `owo-colors` 4.3 (lolcat — D-11..D-14)
| Symbol | Status | Note for planner |
|--------|--------|------------------|
| `UnicodeWidthChar::width(c) -> Option<usize>` | [VERIFIED: docs.rs/unicode-width] | `use unicode_width::UnicodeWidthChar`. `width(c).unwrap_or(0)` per D-12 (control/combining → 0, wide/CJK → 2). |
| `strip_ansi_escapes::strip_str(s) -> String` | [VERIFIED: docs.rs/strip-ansi-escapes] | Public fn; takes string-like, returns `String`. Also `strip(bytes) -> Vec<u8>` and a `Writer`. Use `strip_str` (D-13) unconditionally before recolor AND on the no-color path. |
| `OwoColorize::truecolor(r,g,b)` | [VERIFIED: docs.rs/owo-colors/4.3.0] | Signature `fn truecolor(&self, r: u8, g: u8, b: u8) -> FgDynColorDisplay<...>`. **Unconditional — always emits ANSI regardless of TTY.** This is precisely why the codebase gates on its OWN `is_color_on()` flag (see `core::output` doc + `color/mod.rs`); the global `set_override` only affects `if_supports_color`, not `.truecolor`. |

## Architecture Patterns

### System Architecture Diagram

```
                       box <cmd>  (clap parse in src/cli.rs → src/main.rs dispatch)
                              │  init_color(no_color)  ── sets COLOR_ON once (TTY ∧ ¬NO_COLOR ∧ ¬--no-color)
                              ▼
        ┌──────────────┬───────────────┬────────────────┬───────────────────┐
        │   lolcat     │     json      │     ascii      │      matrix        │
        ▼              ▼               ▼                ▼                    ▼
  core::input      core::input    positional/--file   terminal::size()
  read_input       read_input     path arg            ↓
        │              │               │            enable_raw_mode +
  strip_str(in)   from_str::<Value>  image::open    EnterAlternateScreen + Hide
  (D-13)              │ Ok        Err  (ext-detect)   ↓  [RAII Drop guard armed]
        │              ▼          │    │              loop @20FPS (poll(50ms)=timer):
  per-char:       walk Value  line/col │            ┌──────────────────────────┐
  width→phase     colorize    →stderr  resize_exact │ update per-column drops    │
  sin RGB         (owo)       exit 1   (Triangle)   │ queue! whole frame buffer  │
        │ gate is_color_on()  │            │        │ stdout.flush() ── ONCE     │
        │              │  to_string_pretty │        │ poll Press? c/q/Esc → break│
        ▼              ▼  / to_string      to_luma8 └──────────────────────────┘
   stdout (colored TTY / plain piped)      as_raw()      ↓ (normal/?/panic)
   byte-identical-minus-ANSI               ramp→stdout   Drop: Show+Leave+disable_raw
        └──────────────────── stdout (data) ;  errors/messages ──────────────► stderr
                              exit 0 / 1 / 2  (main() maps BoxError → exit 2)
```

### Component Responsibilities
| File (new) | Responsibility |
|------------|----------------|
| `src/cli.rs` | Swap 4 unit variants (`Lolcat`/`Matrix`/`Ascii`/`Json`, lines 68–74) for real `Args` structs |
| `src/main.rs` | Delete the 4 `not_implemented(...)` arms (lines 76–79); add `=> args.run()` |
| `src/commands/mod.rs` | Register 4 `pub mod` lines |
| `src/commands/lolcat/mod.rs` | `LolcatArgs` + gradient (pure `rgb_at(phase)` fn) + strip + per-char emit, gated |
| `src/commands/json/mod.rs` | `JsonArgs` + parse + colorize-over-`Value` (pure) + pretty/compact delegation + line/col error |
| `src/commands/ascii/mod.rs` | `AsciiArgs` + decode/resize/luma + pure `luma_to_char(byte)` ramp + width math |
| `src/commands/matrix/mod.rs` | `MatrixArgs` + drop model (pure) + glyph table + RAII guard + render loop |
| `tests/{lolcat,json,ascii,matrix}.rs` | `assert_cmd` integration; `tests/cmd/*.trycmd` where snapshot-able |

### Pattern 1: Thin orchestrator over pure helpers (established, all commands)
**What:** `run()` acquires input via `core::input`, delegates math to pure crate-free functions (unit-tested), prints, returns `Result`. Color is gated once via `is_color_on()`.
**When to use:** Every command.
**Example (the exact precedent for lolcat/json color):**
```rust
// Source: src/commands/color/mod.rs:53-58 (the ONE color path, gated)
let swatch = "██████████";
if is_color_on() {
    println!("  {}", swatch.truecolor(r, g, b));
} else {
    println!("  {swatch}");
}
```

### Pattern 2: Pure gradient/ramp/colorize functions (the unit-test seam)
**What:** Extract the per-cell math as a free function that takes a number and returns RGB/char/styled-string. This is what makes LOL/ASCI/JSON/MTRX unit-testable without a terminal.
```rust
// lolcat (D-11) — pure, crate-free, unit-testable
fn rgb_at(phase: f64) -> (u8, u8, u8) {
    let f = 0.1;
    let r = ((f * phase + 0.0).sin() * 127.0 + 128.0) as u8;
    let g = ((f * phase + 2.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    let b = ((f * phase + 4.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    (r, g, b)
}
// ascii (D-02) — pure: map a luma byte through a dark→light ramp
fn luma_to_char(luma: u8, ramp: &[u8]) -> u8 {
    let idx = (luma as usize * (ramp.len() - 1)) / 255;
    ramp[idx]
}
```

### Pattern 3: RAII restore guard for matrix (D-10)
**What:** Construct a guard right after raw-mode setup; its `Drop` runs `execute!(stdout, cursor::Show, LeaveAlternateScreen)` then `disable_raw_mode()` (errors ignored). Fires on normal return, `?` early-return, AND panic.
**Why:** `panic = "abort"` is set in the release profile (`Cargo.toml:61`) — note that `Drop` guards DO still run on a normal `?`/return and on unwinding in test/debug builds, but with `panic = "abort"` a genuine panic in a release binary will NOT unwind and the guard will NOT fire. The mitigation is the alternate screen itself: even an aborted process leaves the real terminal buffer untouched, so nothing persists (D-10's stated reasoning). Keep the loop panic-free (no `unwrap` on terminal ops) so the guard is the real path.
```rust
struct RawGuard;
impl Drop for RawGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, crossterm::cursor::Show,
                                    crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}
```

### Anti-Patterns to Avoid
- **A parallel color path** (raw SGR strings, `owo_colors::set_override` toggling, a second color stack). All color goes through `.truecolor()` gated on `is_color_on()` — anything else breaks byte-identical-minus-ANSI (D-05/D-14, the `colored_json` rejection).
- **Per-character `flush()` in matrix.** STATE.md hard pitfall (~5 FPS). Buffer the whole frame with `queue!`, `flush()` exactly once (D-08).
- **Coloring per byte in lolcat.** Corrupts multi-byte UTF-8 — color per `char` (D-12).
- **`cargo test --lib`.** `box` is binary-only; the in-module unit tests run via `cargo test --bin box` (STATE.md [03-01]).
- **Calling `.dimensions()` without importing `GenericImageView`** — compile error; or use `.width()`/`.height()`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| PNG/JPEG decode | a decoder | `image::open` (D-01) | image decoding is the canonical "trap to hand-roll" — the one sanctioned new-crate exception |
| JSON parse + line/col errors | a parser | `serde_json::from_str` + `Error::{line,column}` | hand-rolling a tolerant JSON parser with accurate positions is the classic rewrite trap |
| Char display width | a width table | `unicode-width` (D-12) | UAX#11 is a large, versioned table; the crate is `no_std` table-only |
| Strip incoming ANSI | a regex/state machine | `strip-ansi-escapes::strip_str` (D-13) | ANSI/VTE parsing has many edge cases (the crate wraps `vte`) |
| Raw mode / alt screen / events | Win32 console calls | `crossterm` (D-08) | already a dep; cross-console correctness incl. PS7 |
| Truecolor escapes | manual `\x1b[38;2;…m` | `owo_colors::truecolor` | already wired; correct reset handling |

**Key insight:** Three of the four new crates are *decode/parse* libraries (image, serde_json, unicode-width) where the lean-binary policy explicitly bends, plus one tiny ANSI-strip helper that actions a standing todo. The actual command logic — gradient math, ramp mapping, drop model, JSON colorizer — IS hand-rolled (pure functions), consistent with the project's "hand-roll the logic, lean on crates for the hard substrate" pattern.

## Common Pitfalls

### Pitfall 1: `.dimensions()` is a trait method (ascii)
**What goes wrong:** `img.dimensions()` fails to compile.
**Why:** `dimensions()` lives on `image::GenericImageView`, not on `DynamicImage` inherently.
**How to avoid:** `use image::GenericImageView;` OR use inherent `img.width()` / `img.height()` (D-02 needs `src_w`/`src_h` for the `rows = cols * src_h / src_w / 2` formula).
**Warning signs:** "method not found in `DynamicImage`".

### Pitfall 2: `image::open` is extension-based (ascii)
**What goes wrong:** A correctly-formatted JPEG named `photo.png` errors or mis-decodes; an extensionless file fails.
**Why:** `open` infers format from the path extension, not content.
**How to avoid:** Document in `--help` that the extension must match (PNG/JPEG only — ASCI-01 scope). A `.context("...")`-wrapped error → exit 1 (FOUND-05); never panic. If content-sniffing is ever wanted (v2), `ImageReader::open(p)?.with_guessed_format()?.decode()`.
**Warning signs:** "format could not be determined" / decode error on a valid-looking image.

### Pitfall 3: Windows key double-fire (matrix)
**What goes wrong:** Pressing `q`/Esc/Ctrl+C registers twice, or a held key spams.
**Why:** On Windows, crossterm emits both `KeyEventKind::Press` AND `Release` (and `Repeat`).
**How to avoid:** `if key.kind == KeyEventKind::Press { ... }` before matching the code (D-10). Verified: `KeyEventKind` has all three variants.
**Warning signs:** exit feels "sticky"; release events mistaken for a second keypress.

### Pitfall 4: Ctrl+C is a KeyEvent, not a signal, in raw mode (matrix)
**What goes wrong:** Waiting for SIGINT never fires; or a `ctrlc` crate is added needlessly.
**Why:** Raw mode disables terminal line discipline — Ctrl+C arrives as `KeyCode::Char('c')` + `KeyModifiers::CONTROL`.
**How to avoid:** Match it as a key (D-10). No signal crate. The RAII guard handles restore on every path.
**Warning signs:** Ctrl+C does nothing / kills the process without restoring the cursor.

### Pitfall 5: Per-character flush tanks FPS (matrix)
**What goes wrong:** Animation runs at ~5 FPS and tears.
**Why:** Each `execute!`/flush is a syscall + a Windows console round-trip.
**How to avoid:** `queue!` the entire frame into a buffered `stdout`, then `flush()` exactly once per frame (D-08). The STATE.md hard pitfall.
**Warning signs:** visible stutter, high CPU, choppy rain.

### Pitfall 6: Untrusted-image resource use (ascii — LOW severity)
**What goes wrong:** A maliciously crafted/huge image (decompression bomb) makes `image::open` + `resize_exact` allocate a lot / spend seconds.
**Why:** `image::open` uses default (effectively unlimited) `Limits`; `resize_exact` cost scales with the SOURCE pixel count before downscale (a 20000×20000 PNG is decoded fully first).
**How to avoid:** This is a local dev tool reading the user's own files (D-01/ASCI-01 scope), so a hard limit is not required for v1. If a guard is cheap to add, `ImageReader` with `Limits::default()`+`max_alloc` is the seam; otherwise document it as accepted. Decode/resize errors must surface as exit-1 anyhow errors, never a panic (FOUND-05).
**Warning signs:** a brief hang on a very large input (functionally fine, just slow).

### Pitfall 7: Halfwidth-katakana tofu on Cascadia Mono (matrix — cosmetic, by design)
**What goes wrong:** Glyphs render as `□` boxes on the bare default PS7 font.
**Why:** U+FF66–U+FF9D need a CJK-capable font; Windows conhost does no glyph fallback.
**How to avoid:** This is a **documented limitation, not a bug** (D-07). Note the font requirement in `--help`/README. It does NOT affect MTRX-01's "no visual artifacts" (that is about clean teardown, D-10, which works on any font). No fallback flag (VIS-V2-01). Width math stays correct because halfwidth katakana are East-Asian-Width *Narrow* (single cell).
**Warning signs:** N/A — expected on Cascadia Mono.

### Pitfall 8: `terminal_width()` fallback when piped (ascii/lolcat)
**What goes wrong:** ascii art is 80 cols wide when piped to a file (no console to size).
**Why:** `core::output::terminal_width()` returns 80 when `crossterm::terminal::size()` has no console (already handled, output.rs:215).
**How to avoid:** This is correct/intended (D-02 explicitly says "falls back to 80 when piped"). Just reuse `terminal_width()` — do not call `terminal::size()` directly. matrix only runs interactively so piping it is a non-case.
**Warning signs:** none — desired behavior; tests should pipe-assert an 80-col render is acceptable.

### Pitfall 9: `arbitrary_precision` × `preserve_order` (json — avoided by design)
**What goes wrong:** Enabling both features breaks round-tripping / serialization (serde-rs/json #505/#721/#845).
**Why:** A documented feature-interaction landmine.
**How to avoid:** D-04 enables ONLY `preserve_order`. Do not add `arbitrary_precision` (deferred behind a future flag). Default number handling loses exact precision only for exotic large/high-precision literals — out of scope.
**Warning signs:** N/A — the avoidance is locked.

## Code Examples

### lolcat: per-char gradient over stripped input (LOL-01)
```rust
// strip incoming ANSI unconditionally (D-13), color per Unicode scalar (D-12),
// advance phase by display width, per-line vertical offset for the diagonal (D-11).
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthChar;
use crate::core::output::is_color_on;

let raw = crate::core::input::read_input(self.input)?;
let clean = strip_ansi_escapes::strip_str(&raw);
let spread = 3.0_f64;
for (line_idx, line) in clean.split_inclusive('\n').enumerate() {
    let mut phase = line_idx as f64 * spread;
    for c in line.chars() {
        if c == '\n' { print!("\n"); continue; }
        let w = UnicodeWidthChar::width(c).unwrap_or(0) as f64;
        if c.is_whitespace() {
            print!("{c}");              // colored space is invisible — emit uncolored
        } else if is_color_on() {
            let (r, g, b) = rgb_at(phase);
            print!("{}", c.truecolor(r, g, b));
        } else {
            print!("{c}");              // plain path: byte-identical minus ANSI
        }
        phase += w;                      // width-0 combining marks inherit base color
    }
}
```

### json: validate + colorize-or-delegate (JSON-01)
```rust
// Source pattern: serde_json::from_str + Error::{line,column}; to_string_pretty = 2-space.
let resolved = /* core::input read of the text */;
match serde_json::from_str::<serde_json::Value>(&text) {
    Err(e) => {
        // -> stderr, exit 1 (D-06); main() wraps with `error:` prefix.
        anyhow::bail!("at line {} column {}: {e}", e.line(), e.column());
    }
    Ok(value) => {
        if self.compact {
            println!("{}", serde_json::to_string(&value)?);          // minify
        } else if is_color_on() {
            print!("{}", colorize(&value, 0));                       // hand-rolled walker
        } else {
            println!("{}", serde_json::to_string_pretty(&value)?);   // 2-space, plain
        }
    }
}
```

### ascii: decode → resize → ramp (ASCI-01)
```rust
use image::GenericImageView;                 // for .dimensions()
let img = image::open(&self.path)
    .with_context(|| format!("failed to read image {}", self.path))?;
let (sw, sh) = img.dimensions();
let cols = crate::core::output::terminal_width() as u32;            // 80 when piped (D-02)
let rows = (cols * sh / sw / 2).max(1);                            // /2 = cell aspect
let gray = img.resize_exact(cols, rows, image::imageops::FilterType::Triangle).to_luma8();
const RAMP: &[u8] = b" .:-=+*#%@";                                  // dark->light (discretion)
let buf = gray.as_raw();                                            // row-major luma bytes
for y in 0..rows {
    let mut line = String::with_capacity(cols as usize);
    for x in 0..cols { line.push(luma_to_char(buf[(y*cols+x) as usize], RAMP) as char); }
    println!("{line}");
}
```

### matrix: loop skeleton (MTRX-01)
```rust
crossterm::terminal::enable_raw_mode()?;
let mut out = std::io::stdout();
crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen, crossterm::cursor::Hide)?;
let _guard = RawGuard;                                              // RAII restore (D-10)
let (cols, rows) = crossterm::terminal::size()?;
// ... init per-column drop state (negative-staggered heads, random trail/speed) ...
loop {
    // 1. update drop state
    // 2. queue! the whole frame (head bright, trail green->dark, erase past tail)
    out.flush()?;                                                  // ONCE per frame (D-08)
    if crossterm::event::poll(std::time::Duration::from_millis(50))? {   // = 20 FPS timer
        if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
            if k.kind == crossterm::event::KeyEventKind::Press {   // Windows double-fire (D-10)
                use crossterm::event::{KeyCode, KeyModifiers};
                let quit = matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
                    || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL));
                if quit { break; }
            }
        }
    }
}   // _guard drops here -> Show + LeaveAlternateScreen + disable_raw_mode
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `image` JPEG via `jpeg-decoder` | `zune-jpeg` (faster) behind the `jpeg` feature | image 0.25.x | trimming to `features=["png","jpeg"]` still gets JPEG via zune; no API change |
| `serde_json` Object = `BTreeMap` (alpha sort) | `IndexMap` via `preserve_order` | feature, stable | formatter keeps input key order (D-04) |
| `unicode-width` 0.1.x | 0.2.x (UAX#11 updated tables) | 0.2.0 (2024) | same `UnicodeWidthChar::width` API; newer Unicode tables |

**Deprecated/outdated:** none affecting this phase. `artem` and `colored_json` were evaluated and rejected (CONTEXT D-01/D-05); `ctrlc` is unnecessary (D-10). No locked crate is deprecated.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `panic="abort"` (release profile) means a true panic in the matrix loop will NOT unwind, so the `Drop` guard fires only on normal/`?`/return paths in release — alt-screen is the real safety net for an aborting panic | Pattern 3 | LOW — matched D-10's own reasoning (alt screen means nothing persists); keep the loop panic-free so the point is moot |
| A2 | Trimming `image` to `default-features=false, features=["png","jpeg"]` is safe and meaningfully shrinks the binary | Standard Stack / discretion | LOW — `jpeg` pulls `zune-jpeg`, `png` pulls `png`; if a needed default (e.g. color conversion) is missing the build errors loudly. Discretion area (D-02) — planner may keep full defaults |
| A3 | An untrusted/huge image is an accepted (documented) slowness, not a v1 blocker | Pitfall 6 | LOW — ASCI-01 scope is the user's own local files; no compliance/security requirement attached |

**All other claims are [VERIFIED] against docs.rs/crates.io or [CITED] to official docs — these three are the only assumptions, all LOW risk and all already aligned with locked decisions.**

## Open Questions

1. **Exact dark→light ramp string for ascii** — Claude's discretion (D-02 note / CONTEXT). `b" .:-=+*#%@"` (10-level) is a standard choice; a longer 70-char ramp gives finer gradients. Recommendation: start with the 10-level ramp (clean output, easy to eyeball-test); it's a one-line change to lengthen. Not blocking.
2. **matrix trail-length range / per-column speed model / head+trail RGB shades / exact FPS (15–25) / katakana sub-range density** — all Claude's discretion (D-08 note). Recommendation: trail 6–20 (D-08), 20 FPS (50ms poll, D-09), head `(180,255,180)`, trail G interpolated 255→40. Not blocking; tune during implementation.
3. **json token color shades** — discretion (D-05 note). Recommendation: key=blue/cyan, string=green, number=yellow, bool/null=magenta, punctuation=plain/dim. Not blocking.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build | ✓ | rustc 1.90.0 / cargo 1.90.0 | — |
| crates.io network (first build) | fetch 4 new crates | assumed ✓ | — | `cargo` offline cache if pre-fetched |
| PowerShell 7 + Cascadia font | matrix authentic glyphs | ✓ (PS7) | — | tofu on Cascadia Mono is a documented cosmetic limitation (D-07), not a blocker |

No external services, no toolchain gaps. All four commands are pure compute over local input.

## Validation Architecture

> `workflow.nyquist_validation: true` (config.json) → this section is required.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (unit, in-module `#[cfg(test)]`) + `assert_cmd` 2.2 / `predicates` 3.1 (integration) + `trycmd` 1.2 (CLI snapshots) |
| Config file | none — Cargo convention (`tests/<cmd>.rs`, `tests/cmd/*.trycmd`) |
| Quick run command | `cargo test --bin box <module>` (unit) — **`--bin box`, NEVER `--lib`** (binary-only crate, STATE.md [03-01]) |
| Full suite command | `cargo test` (all unit + integration) then `cargo clippy -- -D warnings` + `cargo fmt --check` |

### Phase Requirements → Test Map
| Req | Behavior | Test Type | Automated Command | File |
|-----|----------|-----------|-------------------|------|
| LOL-01 | smooth truecolor rainbow in TTY | unit (pure `rgb_at` math: known phase→RGB, 120° spacing, floor 128) | `cargo test --bin box lolcat` | `src/commands/lolcat/mod.rs` #[cfg(test)] ❌ Wave 0 |
| LOL-01 | piped → clean plain text, byte-identical minus ANSI; incoming ANSI stripped | integration (`assert_cmd` with `NO_COLOR=1` / piped: assert no `\x1b`, multi-byte UTF-8 intact) | `cargo test --test lolcat` | `tests/lolcat.rs` ❌ Wave 0 |
| LOL-01 | strip pre-existing ANSI before recolor | unit (`strip_str` on `"\x1b[31mx\x1b[0m"` → `"x"`) | `cargo test --bin box lolcat` | same ❌ Wave 0 |
| MTRX-01 | full-terminal green rain | (not auto-snapshotable — animation) | manual / human-verify | — |
| MTRX-01 | drop/fade model + glyph table correctness | unit (pure: head advance, trail fade green→dark by distance, reset on clear-bottom, katakana table in U+FF66–U+FF9D and all width-1) | `cargo test --bin box matrix` | `src/commands/matrix/mod.rs` #[cfg(test)] ❌ Wave 0 |
| MTRX-01 | exits cleanly + restores (no artifacts) | smoke (`assert_cmd` with piped/non-TTY stdin → binary starts and exits non-hanging; OR a feed of `q`) | `cargo test --test matrix` | `tests/matrix.rs` ❌ Wave 0 |
| ASCI-01 | PNG + JPEG fitted to terminal width | unit (pure `luma_to_char`: 0→darkest, 255→lightest, monotonic; `rows = cols*sh/sw/2` formula) | `cargo test --bin box ascii` | `src/commands/ascii/mod.rs` #[cfg(test)] ❌ Wave 0 |
| ASCI-01 | renders a real PNG + a real JPEG | integration (`assert_cmd` over tiny checked-in fixtures `tests/cmd/ascii.in/{tiny.png,tiny.jpg}`: exit 0, non-empty ASCII; missing/bad file → exit 1, no panic) | `cargo test --test ascii` | `tests/ascii.rs` + fixtures ❌ Wave 0 |
| JSON-01 | invalid → exit 1 + line/col on stderr | integration (`assert_cmd`: pipe `{bad` → `.code(1)`, stderr contains `line`+`column`, stdout empty) | `cargo test --test json` | `tests/json.rs` ❌ Wave 0 |
| JSON-01 | valid pretty (2-space) + `--compact` minify | integration + unit (colorizer over a `Value` is pure; compact/pretty delegate; piped→no ANSI byte-identical minus color) | `cargo test --test json` + `cargo test --bin box json` | `tests/json.rs` + module ❌ Wave 0 |
| JSON-01 | preserve key order | unit (parse `{"b":1,"a":2}` → pretty keeps `b` before `a`) | `cargo test --bin box json` | module ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --bin box <module>` (the touched command's unit tests) — sub-second.
- **Per wave merge:** `cargo test` (full unit + integration).
- **Phase gate:** `cargo test` green + `cargo clippy -- -D warnings` + `cargo fmt --check` before `/gsd:verify-work` (the established Phase-3 gate).

### Wave 0 Gaps
- [ ] `tests/lolcat.rs` — LOL-01 piped-plain + strip + UTF-8 integration
- [ ] `tests/json.rs` — JSON-01 invalid/valid/compact/order
- [ ] `tests/ascii.rs` + `tests/cmd/ascii.in/{tiny.png,tiny.jpg}` — ASCI-01 PNG+JPEG fixtures (tiny, checked-in)
- [ ] `tests/matrix.rs` — MTRX-01 enters/exits-cleanly smoke (feed `q` or non-TTY)
- [ ] In-module `#[cfg(test)]` blocks for the four pure helpers (`rgb_at`, `luma_to_char`, drop/fade + glyph table, json colorize/order)
- [ ] Optional `tests/cmd/json.trycmd` for the locked 2-space pretty layout (trycmd is snapshot-friendly for json; NOT for lolcat ANSI or matrix)

*The four pure-logic helpers (gradient RGB, luma→ramp, drop/fade+glyph, json colorizer/order) unit-test cleanly without a terminal. The matrix raw-mode/animation loop is the only hard-to-snapshot piece → smoke-test the enter/exit contract only (MTRX-01's "exits cleanly" IS the testable invariant; the visual rain is human-verify).*

## Security Domain

> `security_enforcement` key is absent from config.json → treat as enabled. This phase has a narrow, mostly-N/A surface: four local-compute commands, no network, no persistence, no auth, no secrets.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no auth surface) |
| V3 Session Management | no | — |
| V4 Access Control | no | — (reads only user-supplied paths/stdin) |
| V5 Input Validation | yes | `serde_json::from_str` validates JSON (exit 1, no panic); `image::open` validates image format (exit 1, no panic); all input errors are `anyhow` → exit 1, never a panic (FOUND-05, established) |
| V6 Cryptography | no | — (no crypto in this phase) |
| V12 Files/Resources | yes (partial) | ascii reads a user file via `image::open`; untrusted-image resource limits are LOW-severity local-tool concern (Pitfall 6) — `Limits` API is the seam if ever needed |

### Known Threat Patterns for {Rust local CLI, image+json input}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Decompression bomb / huge image | Denial of Service | `image` default Limits are permissive; accepted for a local dev tool (Pitfall 6) — document; `ImageReader::set_limits` available if escalated |
| Malformed JSON / image → panic | Denial of Service | `from_str`/`open` return `Result`; map to exit-1 anyhow error, never `.unwrap()` (FOUND-05, T-02-06 precedent) |
| ANSI injection via piped input (lolcat) | Tampering (output spoofing) | `strip_ansi_escapes::strip_str` removes ALL incoming escapes unconditionally before re-emit (D-13) — neutralizes terminal-escape injection |
| Path traversal / arbitrary read (ascii `--file`) | Information Disclosure | Reads exactly the path the user passes (no traversal logic); same posture as hash/json `--file` — out of threat scope for a single-user local tool |

No new secrets, network calls, or persistence are introduced. The phase's only genuine security-relevant controls (input validation → no-panic, ANSI strip) are already part of the locked decisions and the established codebase pattern.

## Sources

### Primary (HIGH confidence)
- docs.rs/image/0.25.10 — `fn open`, `DynamicImage::{resize_exact, to_luma8, width, height}`, `GenericImageView::dimensions`, `ImageBuffer::{as_raw, pixels, get_pixel}`, `FilterType` variants
- docs.rs/serde_json/1.0.150 — `Value` variants, `from_str`, `to_string_pretty` (2-space default), `to_string`, `Error::{line, column}` (1-based), `preserve_order` semantics
- docs.rs/crossterm/0.29.0 — `event::{poll, read}`, `KeyEvent`, `KeyEventKind` (Press/Repeat/Release), `KeyModifiers`, `KeyCode`, `terminal::{size, enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}`, `cursor::{Hide, Show, MoveTo}`, `queue!`
- docs.rs/unicode-width — `UnicodeWidthChar::width`
- docs.rs/strip-ansi-escapes — `strip_str`, `strip`, `Writer`
- docs.rs/owo-colors/4.3.0 — `OwoColorize::truecolor` (unconditional)
- crates.io API — current-version + slopcheck `OK` for image/serde_json/unicode-width/strip-ansi-escapes
- Local codebase (read in full) — `src/main.rs`, `src/cli.rs`, `src/core/output.rs`, `src/core/input.rs`, `src/commands/{color,cowsay}/mod.rs`, `tests/{color,cli}.rs`, `Cargo.toml`, `.planning/{STATE.md,REQUIREMENTS.md}`, `04-CONTEXT.md`, `.planning/config.json`

### Secondary (MEDIUM confidence)
- serde_json `PrettyFormatter` default 2-space indent — confirmed via docs.rs/serde_json/ser/PrettyFormatter + multiple ecosystem sources
- `image` `Limits`/decompression-bomb posture — image-rs DeepWiki + docs.rs ImageReader

### Tertiary (LOW confidence)
- none — all findings cross-verified against an authoritative source

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all 4 new versions are the current crates.io latest; all pass slopcheck OK; all canonical
- API surface: HIGH — every locked symbol verified against docs.rs for the exact pinned version, zero drift
- Architecture/reuse mapping: HIGH — read the actual `core::{output,input}` + precedent command source; symbols cited by line
- Pitfalls: HIGH — each tied to a verified API fact (trait import, KeyEventKind variants, extension detection) or a recorded STATE.md/CONTEXT decision

**Research date:** 2026-06-24
**Valid until:** ~2026-07-24 (30 days; all crates stable, pinned versions are current latest)
