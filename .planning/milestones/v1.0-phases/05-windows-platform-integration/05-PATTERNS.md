# Phase 5: Windows Platform Integration - Pattern Map

**Mapped:** 2026-06-24
**Files analyzed:** 13 (4 new command modules + 3 registry edits + 1 manifest + 4 test files + fixtures/help snapshot)
**Analogs found:** 13 / 13 (every file has a same-repo precedent)

All four commands are independent vertical slices that reuse the **same registration
mechanics** (cli.rs variant swap + main.rs arm removal + commands/mod.rs `pub mod`) and the
same `RunCommand` / `core::{input,output}` seams established in Phases 1-4. The registration
edits are byte-for-byte the same shape Phase 4 used for `lolcat`/`matrix`/`ascii`/`json`
(see `04-PATTERNS.md` lines 114-201). Per-file rows below point at the closest existing file
for each concern; `pomodoro` is the highest-value analog hit — it reuses the matrix raw-mode
loop **wholesale**.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli.rs` | config (command registry) | request-response | `src/cli.rs` (self — swap 4 unit variants `Qr`/`Clip`/`Pomodoro`/`Weather`, lines 85-92) | exact (edit-in-place) |
| `src/main.rs` | config (dispatch) | request-response | `src/main.rs` (self — delete 4 `not_implemented` arms, lines 95-98) | exact (edit-in-place) |
| `src/commands/mod.rs` | config (module registry) | n/a | `src/commands/mod.rs` (self — add 4 `pub mod` lines, alpha order) | exact (edit-in-place) |
| `Cargo.toml` | config (manifest) | n/a | `Cargo.toml` (self — add 5 `[dependencies]`) | exact (edit-in-place) |
| `src/commands/qr/mod.rs` | command | transform (text in → half-block text out) | `src/commands/color/mod.rs` (`read_input` + thin orchestrator) + `src/commands/cowsay/mod.rs` (no-color render) | role-match (monochrome render, no color gate) |
| `src/commands/clip/mod.rs` | command | file-I/O (raw stdin → clipboard / clipboard → stdout) | `src/commands/json/mod.rs` (boolean flag) + `src/core/input.rs` (the precedence it deliberately bypasses) | role-match (raw stdin, NOT `core::input` — D-04) |
| `src/commands/pomodoro/mod.rs` | command | event-driven (raw-mode countdown loop) | `src/commands/matrix/mod.rs` (RawGuard + poll-as-timer + KeyEventKind::Press) | **exact** (same raw-mode family) |
| `src/commands/weather/mod.rs` | command | request-response (2 sequential HTTP GETs → format) | `src/commands/json/mod.rs` (serde_json consumer + thin orchestrator) + `src/commands/color/mod.rs` (gated color, pure parse helpers) | role-match (only Phase-5 network command) |
| `tests/qr.rs` | test (integration) | transform | `tests/json.rs` (`write_stdin` runner + piped-no-ANSI) + `tests/color.rs` (stdout capture) | exact |
| `tests/clip.rs` | test (integration) | file-I/O | `tests/json.rs` (`write_stdin`) + `tests/cli.rs` (round-trip / exit-code shape) | role-match (clipboard round-trip needs live Win32 — partial) |
| `tests/pomodoro.rs` | test (smoke) | event-driven | `tests/matrix.rs` (`write_stdin` + `.timeout()` non-hanging smoke) | exact |
| `tests/weather.rs` | test (integration) | request-response | `tests/color.rs` + `tests/json.rs` (exit-1 error shapes) | role-match (network paths need offline-determinism — partial) |
| `tests/cmd/qr.trycmd`, `tests/cmd/help.trycmd` (update) | test fixtures / snapshot | snapshot | `tests/cmd/color.trycmd` / `tests/cmd/json.trycmd` (plain snapshot) + `tests/cmd/help.trycmd` (must re-snapshot the 23-cmd listing) | exact |

---

## Shared Patterns

These cross-cutting patterns apply to **all four** new command modules. Reference these once;
do not re-describe per plan.

### Module skeleton + `RunCommand` impl
**Source:** `src/commands/color/mod.rs:17-61`, `src/commands/json/mod.rs:24-79`, `src/commands/cowsay/mod.rs:26-63`
**Apply to:** all four `src/commands/<cmd>/mod.rs`
Every command module is: a `//!` doc comment naming the requirement + decisions, a
`#[derive(Debug, Args)] pub struct <Cmd>Args { ... }` with `///` doc-commented fields (clap
turns each into `--help` text), an `impl RunCommand for <Cmd>Args { fn run(self) -> anyhow::Result<()> }`,
and a `#[cfg(test)] mod tests` block for the pure helpers. Imports use absolute `crate::`
paths (no aliases):
```rust
use clap::Args;                          // or clap::{Args, ValueEnum} for an enum flag (weather --units)
use anyhow::Context;                     // .context(...) at every fallible boundary (clip/weather)
use crate::commands::RunCommand;
use crate::core::output::is_color_on;    // only the commands with a gated color path (weather; NOT qr)
```

### The `RunCommand` trait (the dispatch contract)
**Source:** `src/commands/mod.rs:31-33`
```rust
pub trait RunCommand {
    fn run(self) -> anyhow::Result<()>;
}
```
`run` takes `self` by value (clap-parsed Args struct), returns `anyhow::Result<()>`. `main()`
maps the `Err` to an exit code. There is no `Box<dyn>` — static dispatch only.

### Error → stderr → exit code (FOUND-05 / D-00)
**Source:** `src/main.rs:101-123` (the mapping), `src/commands/color/mod.rs:69-91` (`bail!`/`with_context`), `src/commands/json/mod.rs:60` (`bail!` for a data error → exit 1)
**Apply to:** all four — no `.unwrap()`/`.expect()` on user/network/clipboard input
`run()` returns `anyhow::Result<()>`; `main()` prefixes `error: ` and prints to stderr
(`eprintln!("error: {e:#}")`, main.rs:106). Plain `anyhow::bail!` / `.with_context(...)` → **exit 1**.
Only `BoxError::{MissingInput, UnsupportedHashLength}` downcast to exit 2 (main.rs:115-119).
**These four introduce NO new exit-2 variants** — every error they raise is exit 1, EXCEPT:
- `qr` uses `core::input::read_input` → inherits `BoxError::MissingInput` (exit 2) on a no-arg interactive TTY.
- `weather` reserves exit 2 ONLY for clap's bad `--units` value (clap raises it, not the command — D-13).
- `clip`, `pomodoro` have no exit-2 path at all.
The full downcast mapping to copy verbatim:
```rust
// src/main.rs:115-121 — the single owner of the exit-2 mapping. Do NOT add to it.
match e.downcast_ref::<crate::core::errors::BoxError>() {
    Some(
        crate::core::errors::BoxError::MissingInput
        | crate::core::errors::BoxError::UnsupportedHashLength { .. },
    ) => ExitCode::from(2),
    _ => ExitCode::from(1),
}
```

### Color gate — the ONE color path (D-00, locked Phases 1-4)
**Source:** `src/commands/color/mod.rs:53-58` (canonical), `src/core/output.rs:32-34,121-129`, `src/commands/json/mod.rs:65-74`
**Apply to:** weather (gated conditions/temperature, discretion); pomodoro (gated countdown digits, discretion). **NOT qr** (D-03 monochrome — no color path, no `owo_colors`/`is_color_on` import).
`is_color_on()` is the single gate. `.truecolor()`/`.green()` are unconditional, so wrap in
the `if is_color_on()` / `else` so the plain branch emits the identical glyph without ANSI →
byte-identical-minus-ANSI when piped. NO `owo_colors::set_override` toggling, NO second color stack.
```rust
// src/commands/color/mod.rs:53-58 — the locked precedent
let swatch = "██████████";
if is_color_on() {
    println!("  {}", swatch.truecolor(r, g, b));
} else {
    println!("  {swatch}");
}
```
Note `init_color(cli.no_color)` already ran once in `main()` (main.rs:73) before dispatch — commands only *read* `is_color_on()`, never set it.

### Input acquisition — the seam, and clip's deliberate bypass (D-03/D-04)
**Source:** `src/core/input.rs:47-50` (`read_input`), `:35-46` (precedence doc), `src/commands/color/mod.rs:38` (live caller)
**Apply to:** qr (uses `read_input`); clip (deliberately does NOT — D-04)
`read_input(arg: Option<String>) -> anyhow::Result<String>` resolves arg → piped stdin →
exit-2 on a no-arg interactive TTY (the three branches at input.rs:73-89). qr uses it verbatim:
```rust
// qr (D-03) — arg / "-" stdin / no-arg-TTY→exit-2, same as color.rs:38
let input = crate::core::input::read_input(self.input)?;
```
**clip MUST NOT call `read_input`** (D-04): that path raises `BoxError::MissingInput`→exit-2 on
an empty interactive TTY, which is wrong for a clipboard copy. clip reads raw bytes itself:
```rust
// clip copy (D-04) — raw stdin bytes, owns its own UTF-8 + newline policy
let mut buf = Vec::new();
std::io::stdin().read_to_end(&mut buf)?;
```

### Pure helper + `#[cfg(test)]` seam (the unit-test pattern)
**Source:** `src/commands/color/mod.rs:69-167` (helpers) + `:169-226` (tests); `src/commands/matrix/mod.rs:262-313` (helpers) + `:315-519` (tests); `src/commands/json/mod.rs:96-179` + `:181-304`
**Apply to:** all four — extract the testable math as free, crate-light functions and unit-test
without a terminal/network: `trim_one_trailing_newline` (clip), `wmo_to_str` + `parse_lat_lon` +
`build_forecast_url` (weather), `fmt_mmss` + `resolve_duration` + `is_quit`/`is_cancel` (pomodoro),
the qr build-to-String round-trip (qr). Run via **`cargo test --bin box`** — NEVER `--lib`
(binary-only crate; STATE.md [03-01], reinforced 04-PATTERNS:108).

---

## Pattern Assignments

### `src/cli.rs` (config — variant swap)

**Analog:** itself (lines 85-92 are the 4 unit variants; lines 36-66 are the data-carrying form to mirror).

**Current (lines 85-92):**
```rust
/// Render a QR code for text or a URL in the terminal
Qr,
/// Read from or write to the system clipboard
Clip,
/// Run a focus timer with Windows toast notifications
Pomodoro,
/// Fetch the weather for a location
Weather,
```

**Target shape (mirror lines 44-48, e.g. `Color(crate::commands::color::ColorArgs)`):**
```rust
/// Render a QR code for text or a URL in the terminal
Qr(crate::commands::qr::QrArgs),
/// Read from or write to the system clipboard
Clip(crate::commands::clip::ClipArgs),
/// Run a focus timer with Windows toast notifications
Pomodoro(crate::commands::pomodoro::PomodoroArgs),
/// Fetch the weather for a location
Weather(crate::commands::weather::WeatherArgs),
```
Keep each `///` doc comment (it is the `box --help` one-liner + `box <cmd> --help` long help, cli.rs:3-4). The `matrix` variant (lines 70-80) shows the multi-paragraph `///` form if a command wants extended `--help` body text (e.g. clip's trailing-newline-trim note, D-05).

---

### `src/main.rs` (config — dispatch)

**Analog:** itself (lines 76-94 are the live `=> args.run()` precedent; lines 95-98 are the arms to delete).

**Current (lines 95-98):**
```rust
Commands::Qr => Err(not_implemented("qr")),
Commands::Clip => Err(not_implemented("clip")),
Commands::Pomodoro => Err(not_implemented("pomodoro")),
Commands::Weather => Err(not_implemented("weather")),
```

**Target (mirror line 80, `Commands::Color(args) => args.run()`):**
```rust
Commands::Qr(args) => args.run(),
Commands::Clip(args) => args.run(),
Commands::Pomodoro(args) => args.run(),
Commands::Weather(args) => args.run(),
```
**Consequence:** after removing ALL FOUR arms, the `not_implemented` import becomes dead — unlike Phase 4 (which left 4 stubs), this is the **last** phase, so delete the import too:
`use crate::commands::stub::not_implemented;` (main.rs:37). Whether to also delete `src/commands/stub.rs` + its `pub mod stub;` (commands/mod.rs:24) is planner discretion — but the dead-code gate will flag the unused import if left.

---

### `src/commands/mod.rs` (config — module registry)

**Analog:** itself (lines 7-26, the alphabetized `pub mod` list).

Add four lines in alpha order: `pub mod clip;`, `pub mod pomodoro;`, `pub mod qr;`, `pub mod weather;` (clip after `bulk_rename`/before `color`; pomodoro after `passgen`/before `roast`; qr after `passgen`/`pomodoro`; weather after `uuid`, at the end). Modules are live the moment cli.rs/main.rs reference them — the `[01-03]` allow-then-remove dead-code dance does NOT apply.

---

### `Cargo.toml` (config — manifest)

**Analog:** the existing `[dependencies]` block (lines 11-67), specifically the comment-annotated feature-trim style of `chrono` (lines 32-34), `image` (lines 57-63), and `serde_json` (lines 44-49).

Add under `[dependencies]` (RESEARCH:121-140 — note the feature trims):
```toml
# qr (QR-01 / D-01/D-02). default-features=false drops image/svg/pic; the
# render::unicode::Dense1x2 path is always available without any feature.
qrcode = { version = "0.14.1", default-features = false }
# clip (CLIP-01 / D-06). default-features=false drops image-data (binary/image
# clipboard is Out of Scope); set_text/get_text remain.
arboard = { version = "3.6.1", default-features = false }
# pomodoro toast (POMO-01 / D-09). Maintained fork of winrt-notification
# (OVERRIDES CLAUDE.md). Cargo builds it only on the msvc target.
tauri-winrt-notification = "0.7.2"
# weather (WTHR-01 / D-13). Blocking HTTPS; default ["rustls","gzip"] (no OpenSSL/DLL).
# Do NOT enable the `json` feature — deserialize via serde_json::from_reader.
ureq = "3.3.0"
# weather Deserialize derive (D-13). serde_json (1.0.150, line 49) is ALREADY a dep;
# only the derive macro is new.
serde = { version = "1", features = ["derive"] }
```
`crossterm` (line 21), `owo-colors` (line 19), `rand` (line 36), `serde_json` (line 49),
`regex` (line 41), `enable-ansi-support` (line 20) are all present — pomodoro and weather's
lat/lon parse + JSON path need no further manifest change. `[dev-dependencies]` (lines 69-75)
already has `assert_cmd`, `predicates`, `assert_fs`, `tempfile`, `trycmd`, `insta` — no test-dep change.
⚠️ Release profile is `panic = "abort"` (line 81) — the pomodoro `RawGuard` Drop is the real restore path; keep the loop panic-free (no `.unwrap()` on terminal ops), exactly as matrix does (matrix doc:44-47).

---

### `src/commands/qr/mod.rs` (command, transform — monochrome)

**Analog:** `src/commands/color/mod.rs` (thin orchestrator + `read_input`) + `src/commands/cowsay/mod.rs` (no-color render → `println!`)

**Imports** (color.rs:17-22 minus the color gate — D-03 has NO color path):
```rust
use clap::Args;
use qrcode::render::unicode::Dense1x2;
use qrcode::{EcLevel, QrCode};
use crate::commands::RunCommand;
// NO owo_colors / is_color_on — qr is monochrome (D-03)
```

**Args struct** (mirror color.rs:28-32 — a single optional positional fed to `read_input`):
```rust
#[derive(Debug, Args)]
pub struct QrArgs {
    /// Text or URL to encode; omit to read from piped stdin.
    pub input: Option<String>,
}
```

**`run()` body** (input from color.rs:38; render chain from RESEARCH:405-418, API-verified RESEARCH:170-181):
```rust
let input = crate::core::input::read_input(self.input)?;   // arg / - / no-arg-TTY→exit-2 (D-03)
let code = QrCode::with_error_correction_level(input.as_bytes(), EcLevel::M)?; // M = qrcode default (D-02)
let rendered: String = code
    .render::<Dense1x2>()
    .quiet_zone(true)        // full ISO-18004 4-module border (D-02)
    // .dark_color(Dense1x2::Dark).light_color(Dense1x2::Light) // ONLY if phone-scan fails (Pitfall QR-1)
    .build();                // owned String, pure half-blocks, NO ANSI (D-01)
println!("{rendered}");      // data → stdout (D-03)
```
⚠️ `Renderer` builder methods take `&mut self` and return `&mut Self` (RESEARCH:178) — the full chain compiles as one expression because `.build()` is `&self`. `Dense1x2::Pixel::Image = String` (RESEARCH:177) → `.build()` is an owned `String`.

**Pure helper to unit-test:** a `render_qr(input: &str) -> anyhow::Result<String>` wrapper around the chain, asserted to (a) be non-empty, (b) contain NO ANSI (`!s.contains('\u{1b}')` — the D-01 invariant, same assertion as color.rs:45/json.rs:69), (c) be deterministic for a fixed input. Verify dark-on-light polarity scans on a dark PS background during impl (Pitfall QR-1 — a human-verify step, not an automated test).

---

### `src/commands/clip/mod.rs` (command, file-I/O — raw stdin, NOT core::input)

**Analog:** `src/commands/json/mod.rs:37-44` (boolean `#[arg(long)]` flag) + `src/core/input.rs` (the precedence it deliberately bypasses, D-04)

**Imports** (note `std::io` traits + `anyhow::Context` for clean clipboard errors):
```rust
use std::io::{Read, Write};
use anyhow::Context;
use clap::Args;
use crate::commands::RunCommand;
```

**Args struct** (boolean flag — mirror json.rs:42-43 `#[arg(long)] pub compact: bool`):
```rust
#[derive(Debug, Args)]
pub struct ClipArgs {
    /// Read the clipboard to stdout instead of writing stdin to it.
    /// On copy, at most one trailing newline (\r\n or \n) is stripped (PowerShell
    /// appends an implicit CRLF when piping). --paste emits byte-exact. (D-05)
    #[arg(long)]
    pub paste: bool,
}
```
The `--help` trailing-newline-trim wording is Claude's discretion (D-05) — the multi-line `///` doc-comment form (cli.rs:70-79 matrix shows the style) is the place for it.

**`run()` body** (RESEARCH:420-437; arboard API-verified RESEARCH:183-190 — both ops need `&mut self`):
```rust
if self.paste {
    let mut cb = arboard::Clipboard::new().context("open clipboard")?;
    let text = cb.get_text().context("read clipboard")?;       // owned String, &mut self
    std::io::stdout().write_all(text.as_bytes())?;             // byte-exact, no newline policy (D-05)
} else {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;                   // raw bytes, NOT core::input (D-04)
    let buf = trim_one_trailing_newline(buf);                 // ≤1 trailing \r\n / \n (D-05)
    let text = String::from_utf8(buf).context("clipboard input must be UTF-8")?;
    let mut cb = arboard::Clipboard::new().context("open clipboard")?;
    cb.set_text(text).context("write clipboard")?;            // persists via OS handle (D-06); &mut self
}
Ok(())
```
⚠️ `let mut cb` (both ops take `&mut self` — Pitfall CLIP-2). Create→op→return on the **main thread**; never spawn onto a worker (STATE.md hard pitfall, satisfied by this single-shot flow — D-06).

**Pure helper to unit-test** (the matrix/cowsay free-function seam; exact body from RESEARCH:278-285):
```rust
fn trim_one_trailing_newline(mut s: Vec<u8>) -> Vec<u8> {
    if s.last() == Some(&b'\n') {
        s.pop();
        if s.last() == Some(&b'\r') { s.pop(); }
    }
    s
}
```
Test: `"x\r\n"`→`"x"`, `"x\n"`→`"x"`, `"x"`→`"x"`, `"x\n\n"`→`"x\n"` (only ONE stripped), `""`→`""`, `"a\nb"`→`"a\nb"` (no interior strip).

---

### `src/commands/pomodoro/mod.rs` (command, event-driven — EXACT matrix analog)

**Analog:** `src/commands/matrix/mod.rs` — **the single most direct analog in the phase.** Reuse the `RawGuard` struct + Drop impl (matrix.rs:101-116), the guard-armed-pre-execute! ordering (matrix.rs:118-133), the `event::poll`-as-timer loop (matrix.rs:149-209), the `KeyEventKind::Press` filter + `is_quit` predicate (matrix.rs:200-208, 307-313), and the `#[cfg(test)]` key-event test harness (matrix.rs:463-518).

**Imports** (matrix.rs:49-64 subset — pomodoro likely skips the alternate screen, see note):
```rust
use std::io::Write;
use std::time::{Duration, Instant};
use clap::Args;
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crate::commands::RunCommand;
use tauri_winrt_notification::Toast;
```

**Args struct** (positional + two bool flags — the cowsay/json flag style; D-08):
```rust
#[derive(Debug, Args)]
pub struct PomodoroArgs {
    /// Minutes to run; defaults to 25 (work), 5 with --break, 15 with --long-break.
    pub minutes: Option<u64>,
    /// Run a 5-minute short break instead of a 25-minute work session.
    #[arg(long = "break")]
    pub break_: bool,
    /// Run a 15-minute long break.
    #[arg(long = "long-break")]
    pub long_break: bool,
}
```
(`break` is a Rust keyword → `#[arg(long = "break")] pub break_: bool`.)

**RAII guard** (matrix.rs:101-116 verbatim — but pomodoro likely keeps the NORMAL screen, so the Drop restores `Show` + `disable_raw_mode` only, NO `LeaveAlternateScreen` — RESEARCH:313-326):
```rust
// Source: src/commands/matrix/mod.rs:101-116 (adapt: drop LeaveAlternateScreen if no alt screen)
struct RawGuard;
impl Drop for RawGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, cursor::Show);
        let _ = disable_raw_mode();
    }
}
```

**Loop skeleton** (matrix.rs:118-133 setup + :149-209 loop; with the D-07/Pitfall-POMO-2 final-second-precise `poll(min(1s, remaining))`):
```rust
enable_raw_mode()?;
let _guard = RawGuard;                       // armed INSTANT raw mode is on, before any ? (matrix.rs:131 / CR-01)
let mut out = std::io::stdout();
crossterm::execute!(out, cursor::Hide)?;     // no EnterAlternateScreen — in-place countdown (discretion)

let total = resolve_duration(self.minutes, self.break_, self.long_break); // Duration
let end = Instant::now() + total;
let cancelled = loop {
    let remaining = end.saturating_duration_since(Instant::now());
    if remaining.is_zero() { break false; }                    // COMPLETION (loop fell through)
    // render MM:SS in place: single clean write per tick (matrix flush-once discipline, D-07)
    let secs = remaining.as_secs();
    // ... queue!/write the line, then ONE flush (out.flush()?) — never per char ...
    out.flush()?;
    let tick = remaining.min(Duration::from_secs(1));          // exact final partial second (Pitfall POMO-2)
    if event::poll(tick)? {                                    // poll = timer AND keypress gate (D-07)
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && is_cancel(&key) { break true; } // CANCEL → exit 1, NO toast
        }
    }
};
drop(_guard);                                // restore terminal BEFORE firing the toast (D-07)
if cancelled {
    eprintln!("cancelled");                  // message → stderr
    std::process::exit(1);                   // exit 1, NO toast (D-07)
}
// COMPLETION → toast + exit 0:
let _ = Toast::new(Toast::POWERSHELL_APP_ID) // owned-self builder (RESEARCH:197); no AUMID registration (D-09)
    .title("Pomodoro")
    .text1("Time's up!")
    .show();                                 // fire-and-return; a failed toast should not fail the completed timer (RESEARCH:198)
Ok(())
```

**Cancel predicate** (the matrix `is_quit` shape verbatim — matrix.rs:307-313; Press-only, Ctrl+C-as-KeyEvent, q, Esc):
```rust
// Source: src/commands/matrix/mod.rs:307-313 (rename is_quit → is_cancel; identical body)
fn is_cancel(key: &KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press { return false; }       // Windows Press+Release double-fire (Pitfall POMO-1)
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}
```

**Pure helpers to unit-test** (matrix.rs:262-313 free-fn seam + :463-518 KeyEvent test harness):
- `fmt_mmss(secs: u64) -> String` → `format!("{:02}:{:02}", secs/60, secs%60)` (RESEARCH:309). Test `0`→`"00:00"`, `65`→`"01:05"`, `1500`→`"25:00"`.
- `resolve_duration(minutes, break_, long_break) -> Duration` → positional override wins, else 5 (break), 15 (long-break), 25 (default) (D-08). Test all four paths + override-beats-flag.
- `is_cancel` — copy the matrix key-event test harness verbatim (matrix.rs:463-518): the `key(code, modifiers, kind)` builder, Press-quits, plain-`c`-is-not, Release/Repeat-don't-cancel (the Windows double-fire filter).

---

### `src/commands/weather/mod.rs` (command, request-response — the only network command)

**Analog:** `src/commands/json/mod.rs` (serde_json consumer + thin orchestrator + `bail!`→exit-1) + `src/commands/color/mod.rs` (pure parse helpers + gated color + `ValueEnum`-adjacent flag)

**Imports** (json.rs:24-29 base + serde derive + the `ValueEnum` for `--units`):
```rust
use anyhow::Context;
use clap::{Args, ValueEnum};
use serde::Deserialize;
use crate::commands::RunCommand;
use crate::core::output::is_color_on;        // any conditions/temperature color is gated (discretion)
```

**Args struct + the `--units` ValueEnum** (clap `ValueEnum` — a bad value is clap's exit 2, D-13):
```rust
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Units { Metric, Imperial }

#[derive(Debug, Args)]
pub struct WeatherArgs {
    /// City name (geocoded) or `lat,lon` (e.g. "London" or "51.5,-0.13").
    pub location: String,
    /// Unit system; defaults to metric (°C, km/h). (D-10)
    #[arg(long, value_enum, default_value_t = Units::Metric)]
    pub units: Units,
}
```

**`run()` orchestration** (thin-orchestrator pattern; the HTTP + Deserialize from RESEARCH; error split is the load-bearing part — Pitfall WTHR-1):
```rust
// 1. parse-shape disambiguation (D-12): lat,lon vs city name
let (lat, lon, label) = match parse_lat_lon(&self.location) {
    Some((lat, lon)) => (lat, lon, format!("{lat:.4},{lon:.4}")),
    None => geocode(&self.location)?,        // GET .../v1/search; absent results → exit 1 (D-12)
};
eprintln!("Resolved \"{}\" → {label} ({lat:.2}, {lon:.2})", self.location); // echo → stderr (D-12)
// 2. forecast GET (+ unit params on imperial — D-11)
let forecast = fetch_forecast(lat, lon, self.units)?;
// 3. format: read the AUTHORITATIVE unit label from current_units (D-11 — never hardcode °C/mph)
//    conditions via wmo_to_str(forecast.current.weather_code) (D-13)
//    data → stdout; any color gated via is_color_on() (discretion)
```

**The error split — the single most important weather pattern (Pitfall WTHR-1, RESEARCH:204-205):**
ureq 3.x returns `Err(Error::StatusCode(code))` on non-2xx by DEFAULT — so the non-2xx message is a `match` arm on the error, NEVER a post-success `resp.status()` check:
```rust
match ureq::get(&url).call() {
    Ok(resp) => serde_json::from_reader(resp.into_body().into_reader())  // D-13: into_reader, no `json` feature
        .context("parse weather response")?,
    Err(ureq::Error::StatusCode(code)) => anyhow::bail!("weather service returned {code}"), // exit 1
    Err(_) => anyhow::bail!("could not reach weather service (offline?)"),                   // exit 1 (offline/DNS/connect)
}
```
`bail!` → exit 1 (json.rs:60 precedent). Exit 2 is ONLY clap's bad `--units` (the `ValueEnum` raises it). `main()` prepends `error: ` (main.rs:106), so the bail message is the locked-shape suffix.

**Deserialize structs** (RESEARCH Pitfalls WTHR-2/WTHR-3 — the three landmines the planner MUST encode):
```rust
// (a) no-results geocode OMITS the `results` key entirely → #[serde(default)] (Pitfall WTHR-2)
#[derive(Deserialize)]
struct GeoResp { #[serde(default)] results: Vec<GeoHit> }
#[derive(Deserialize)]
struct GeoHit { latitude: f64, longitude: f64, name: String,
                #[serde(default)] admin1: Option<String>, #[serde(default)] country: Option<String> }
// (c) current_units carries the AUTHORITATIVE label ("mp/h" ≠ request param "mph") (Pitfall WTHR-3)
#[derive(Deserialize)]
struct ForecastResp { current: Current, current_units: CurrentUnits }
#[derive(Deserialize)]
struct Current { temperature_2m: f64, relative_humidity_2m: f64, weather_code: u32, wind_speed_10m: f64 }
#[derive(Deserialize)]
struct CurrentUnits { temperature_2m: String, wind_speed_10m: String }
```

**Pure helpers to unit-test** (the color.rs:69-167 parse-helper seam + the matrix free-fn discipline):
```rust
// WMO code → &'static str (D-13; no alloc, _ arm mandated). Body from RESEARCH:287-299.
fn wmo_to_str(code: u32) -> &'static str { match code { 0 => "Clear sky", /* ... */ _ => "Unknown" } }
// lat,lon parse-shape + range (D-12). regex 1.12.4 already a dep (Cargo.toml:41). Body RESEARCH:301-307.
fn parse_lat_lon(s: &str) -> Option<(f64, f64)> { /* ^...$ regex + lat∈[-90,90] lon∈[-180,180] */ }
// URL builder (D-11/D-13) — pure, so the imperial unit-param branch is unit-testable without a network call.
fn build_forecast_url(lat: f64, lon: f64, units: Units) -> String { /* + &temperature_unit=fahrenheit&wind_speed_unit=mph on Imperial */ }
```
Test `wmo_to_str` for each mapped band + the `_`→`"Unknown"` arm; `parse_lat_lon` for `"51.5,-0.13"`→`Some`, `"London"`→`None`, `"91,0"`→`None` (out of range), `"1,2,3"`→`None`; `build_forecast_url` for metric (no unit params) vs imperial (both params present).

---

### `tests/qr.rs` (integration test)

**Analog:** `tests/json.rs` (`write_stdin` runner + `json_piped_no_ansi` byte-scan, json.rs:18-26,109-123) + `tests/color.rs` (`color_stdout` capture, color.rs:21-32)

Runner mirrors `json()`/`color()`: `Command::cargo_bin("box").arg("qr")`, `.env("NO_COLOR","1")`, input via `.arg(...)` or `.write_stdin(...)`. Assert: exit 0 + non-empty half-block stdout for `box qr "https://example.com"`; the D-01 no-ANSI invariant via the json.rs:120 byte-scan (`!out.stdout.windows(2).any(|w| w == b"\x1b[")`) — qr must NEVER emit ANSI even WITHOUT NO_COLOR (the whole reason for rejecting qr2term, D-01). Stable render → an optional `tests/cmd/qr.trycmd` snapshot (color.trycmd/json.trycmd plain shape).

---

### `tests/clip.rs` (integration test — partial: live clipboard)

**Analog:** `tests/json.rs:18-26` (`write_stdin` runner) + `tests/cli.rs:42-75` (exit-code/round-trip shape)

A full copy→paste round-trip needs a live Win32 clipboard (CI-fragile, single-threaded). Testable shape: `echo "hi" | box clip` then `box clip --paste` round-trips `hi` with NO trailing newline (proves the D-05 single-trim) — run as one test that writes then reads, gated/`#[ignore]`d if the runner has no clipboard. The deterministic coverage is the **`trim_one_trailing_newline` unit test** (in-module). Also assert `--paste` and copy both exit 0 cleanly and `clip` never blocks on empty piped stdin (it reads raw stdin, NOT `core::input`, so no exit-2-on-TTY path — D-04). Carry a **human-verify checkpoint** (real copy→paste in PowerShell), like matrix/pomodoro.

---

### `tests/pomodoro.rs` (smoke test)

**Analog:** `tests/matrix.rs` (exact — `write_stdin` + `.timeout()` non-hanging, panic-free smoke, matrix.rs:27-49)

The live countdown + real toast + Ctrl+C cancel are NOT auto-snapshotable → **human-verify checkpoint** (like matrix, RESEARCH/CONTEXT code_context:112). The automated smoke test is the matrix shape verbatim: `box pomodoro 1` (or a tiny duration) fed a cancel key on piped stdin, bounded by `.timeout(Duration::from_secs(5))`, asserting the process terminates non-hanging and stderr carries no `"panicked"` marker (matrix.rs:43-48). Do NOT pin an exit code (a non-TTY may legitimately error out of raw-mode setup — matrix.rs:39-42). The `fmt_mmss` / `resolve_duration` / `is_cancel` correctness lives in the in-module `#[cfg(test)]` tests, not here.

---

### `tests/weather.rs` (integration test — partial: network)

**Analog:** `tests/color.rs:67-84` (exit-1 clean-error shape) + `tests/json.rs:50-57` (`bail!`→exit-1 with stderr predicate)

Network paths are non-deterministic → the **deterministic coverage is the pure-helper unit tests** (`wmo_to_str`, `parse_lat_lon`, `build_forecast_url`) in-module. The integration tests cover the offline-determinable shapes: a bad `--units value` → exit 2 (clap, `tests/cli.rs:42-50` badcmd shape); a malformed/unreachable host or `--location ""` → exit 1 with the locked-shape stderr message + empty stdout (color.rs:67-74 shape). A live "London"→temperature happy-path test is network-dependent → `#[ignore]` or gate it, with a human-verify checkpoint for the real fetch + `--units imperial`.

---

### `tests/cmd/help.trycmd` (snapshot — MUST re-snapshot) + `tests/cmd/qr.trycmd` (new, optional)

**Analog:** `tests/cmd/help.trycmd` (self — the 23-command `box --help` listing) + `tests/cmd/json.trycmd`/`color.trycmd` (plain command snapshots)

⚠️ **Regression risk:** `tests/cli.rs::trycmd` (cli.rs:100-103) snapshots the full `box --help` listing. The current help.trycmd shows the OLD one-liners for `qr`/`clip`/`pomodoro`/`weather`. If the cli.rs `///` doc comments change (e.g. clip gains a longer `--help`), the **one-line `about` text in `box --help` stays the same** unless the FIRST `///` line changes — but verify after the cli.rs edit and re-snapshot help.trycmd if the listing drifts (run `TRYCMD=overwrite cargo test --test cli` per trycmd convention, then review the diff). A new `tests/cmd/qr.trycmd` (qr render snapshot) is optional and follows the json.trycmd plain shape.

---

## No Analog Found

| File / Logic | Role | Data Flow | Reason |
|--------------|------|-----------|--------|
| weather HTTP + Deserialize body (`ureq::get(...).call()` error-split, `from_reader`, Open-Meteo structs) | command (network tier) | request-response | No existing command touches the network — `ureq`/`serde::Deserialize` are NEW. The module SKELETON, `serde_json` consumption (json.rs), gated color (color.rs), pure-helper seam, and error→exit-1 mapping all follow repo patterns; only the HTTP-call + Deserialize-struct bodies come from RESEARCH:200-208 (API-verified) — Pitfalls WTHR-1/2/3 are the must-encode landmines. |
| clip copy→paste round-trip test | test | file-I/O | No existing test exercises a live OS clipboard; the round-trip is human-verify + an `#[ignore]`d/gated integration test. The deterministic `trim_one_trailing_newline` unit test fully covers the D-05 logic. |
| pomodoro live countdown + toast | command/test | event-driven | The LOOP has an exact analog (matrix), but the live render + WinRT toast + cancel are human-verify (matrix-precedented, RESEARCH no automated path). |

The `pomodoro` raw-mode loop, by contrast, has a **full** same-repo analog (matrix) — it is the
strongest analog match in the phase.

---

## Metadata

**Analog search scope:** `src/cli.rs`, `src/main.rs`, `src/commands/mod.rs`, `src/commands/stub.rs`,
`src/commands/{matrix,color,json,cowsay}/mod.rs`, `src/core/{input,output,errors}.rs`,
`tests/{color,json,matrix,hash,cli}.rs`, `tests/cmd/{json.trycmd,help.trycmd,color.trycmd,ascii.in/}`, `Cargo.toml`,
plus `04-PATTERNS.md` for the registration-edit shape.
**Files scanned:** 18
**Module layout confirmed:** one directory per command (`src/commands/<cmd>/mod.rs`) — all 4 new
commands follow this (matches `matrix/`, `color/`, `json/`, `cowsay/`, etc.).
**Key cross-cutting facts for the planner:**
- `pomodoro` reuses `src/commands/matrix/mod.rs` wholesale (RawGuard + poll-as-timer + Press-filter + KeyEvent test harness) — the highest-leverage analog.
- `clip` is the ONE command that deliberately bypasses `core::input` (raw stdin, D-04) — do not "fix" it to use `read_input`.
- `qr` has NO color path (D-03) — do not import `owo_colors`/`is_color_on`.
- `weather` is the ONLY network command; the ureq-non-2xx-is-Err split (Pitfall WTHR-1) and the `#[serde(default)] results` absence (Pitfall WTHR-2) are the must-encode landmines.
- This is the LAST phase → removing all 4 `not_implemented` arms makes the `stub::not_implemented` import dead (main.rs:37); delete it.
- `cargo test --bin box` (NEVER `--lib`) for unit tests; re-snapshot `tests/cmd/help.trycmd` if the `box --help` listing drifts after the cli.rs edit.
**Pattern extraction date:** 2026-06-24
