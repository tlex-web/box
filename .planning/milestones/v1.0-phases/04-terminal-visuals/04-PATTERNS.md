# Phase 4: Terminal Visuals - Pattern Map

**Mapped:** 2026-06-24
**Files analyzed:** 13 (4 new command modules + 3 registry edits + 1 manifest + 4 test files + fixtures)
**Analogs found:** 13 / 13 (every file has a same-repo precedent)

All four commands are independent vertical slices that reuse the same registration
mechanics and the same `core::{input,output}` helpers. There is **one analog set** that
covers them all; per-file rows below point at the closest existing file for each concern.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli.rs` | config (command registry) | request-response | `src/cli.rs` (self — swap 4 unit variants, lines 68-74) | exact (edit-in-place) |
| `src/main.rs` | config (dispatch) | request-response | `src/main.rs` (self — delete 4 `not_implemented` arms, lines 76-79) | exact (edit-in-place) |
| `src/commands/mod.rs` | config (module registry) | n/a | `src/commands/mod.rs` (self — add 4 `pub mod` lines) | exact (edit-in-place) |
| `Cargo.toml` | config (manifest) | n/a | `Cargo.toml` (self — add 4 `[dependencies]`) | exact (edit-in-place) |
| `src/commands/lolcat/mod.rs` | command | transform (text in → colored text out) | `src/commands/color/mod.rs` | exact (text-in + `is_color_on()`-gated `.truecolor`) |
| `src/commands/json/mod.rs` | command | transform (text in → pretty/colored text out) | `src/commands/color/mod.rs` | exact (text-in + gated color + parse-error→exit-1) |
| `src/commands/ascii/mod.rs` | command | file-I/O → transform (image file → ASCII out) | `src/commands/cowsay/mod.rs` (render + width) + `src/commands/hash/mod.rs` (`--file`/path arg) | role-match |
| `src/commands/matrix/mod.rs` | command | event-driven (raw-mode animation loop) | `src/core/output.rs::terminal_width` (crossterm usage) + `src/commands/cowsay/mod.rs` (pure-helper shape) | role-match (no existing raw-mode loop — partial) |
| `tests/lolcat.rs` | test (integration) | request-response | `tests/color.rs` | exact |
| `tests/json.rs` | test (integration) | request-response | `tests/color.rs` + `tests/tree.rs` (`piped_no_ansi`) | exact |
| `tests/ascii.rs` | test (integration) | file-I/O | `tests/tree.rs` (temp-dir fixtures) + `tests/hash.rs` (`assert_fs` file fixture) | exact |
| `tests/matrix.rs` | test (smoke) | event-driven | `tests/color.rs` (runner shape) | role-match (smoke only — no snapshot) |
| `tests/cmd/ascii.in/{tiny.png,tiny.jpg}`, optional `tests/cmd/json.trycmd` | test fixtures | file-I/O / snapshot | `tests/cmd/tree.in/` (checked-in input tree) + `tests/cmd/color.trycmd` (plain-text snapshot) | exact |

---

## Shared Patterns

These cross-cutting patterns apply to **all four** new command modules. The planner should
reference these once and not re-describe per plan.

### Module skeleton + `RunCommand` impl
**Source:** `src/commands/color/mod.rs:17-61`, `src/commands/cowsay/mod.rs:26-63`
**Apply to:** all four `src/commands/<cmd>/mod.rs`
Every command module is: a `//!` doc comment naming the requirement + decisions, a
`#[derive(Debug, Args)] pub struct <Cmd>Args { ... }` with `///` doc-commented fields (clap
turns each into `--help` text), an `impl RunCommand for <Cmd>Args { fn run(self) -> anyhow::Result<()> }`,
and a `#[cfg(test)] mod tests` block for the pure helpers. Imports use absolute `crate::`
paths (no path aliases):
```rust
use clap::Args;                 // or clap::{Args, ValueEnum} when an enum flag is needed
use owo_colors::OwoColorize;    // only for the colored commands (lolcat/json/matrix)
use crate::commands::RunCommand;
use crate::core::output::is_color_on;   // colored commands
```

### Color gate — the ONE color path (D-14/D-05)
**Source:** `src/commands/color/mod.rs:53-58` (the canonical precedent), reinforced by
`src/core/output.rs:121-129` (`format_row` gates the glyph the same way)
**Apply to:** lolcat, json, matrix
This is the locked pattern. `.truecolor()` is unconditional, so gate on `is_color_on()`;
the `else` branch emits the identical glyph without ANSI → byte-identical-minus-ANSI when piped.
NO `owo_colors::set_override` toggling, NO second color stack, NO raw SGR strings.
```rust
let swatch = "██████████";
if is_color_on() {
    println!("  {}", swatch.truecolor(r, g, b));
} else {
    println!("  {swatch}");
}
```

### Input acquisition (D-04 precedence)
**Source:** `src/commands/color/mod.rs:38` (`read_input`), `src/commands/hash/mod.rs:38-43,
:read_file_or_stdin` usage; resolvers in `src/core/input.rs:47-50` and `:154-158`
**Apply to:** lolcat + json (text via `read_input`), ascii (path arg; may use a plain
positional like color/cowsay, or `read_file_or_stdin` per discretion)
```rust
// lolcat / json — text in (arg → piped stdin → exit-2 on no-arg TTY):
let raw = crate::core::input::read_input(self.input)?;
```
The positional field that feeds it is `pub input: Option<String>` (color.rs:31) or
`pub text: Option<String>` (cowsay.rs:46). For ascii's `--file`/path, the `hash` precedent
shows `pub path: Option<String>` + `read_file_or_stdin` — but ascii needs a real on-disk path
for `image::open`, so a plain positional `PathBuf`/`String` is the closer fit (decode is not a
stream the input layer models).

### Error → stderr → exit code (FOUND-05 / D-06)
**Source:** `src/main.rs:86-108` (the mapping), `src/commands/color/mod.rs:69-91`
(`bail!`/`with_context` usage), `src/commands/hash/mod.rs` (`bail!` for verify-mismatch)
**Apply to:** json (parse error), ascii (decode/open error), all four (no `.unwrap()` on
user input)
`run()` returns `anyhow::Result<()>`; `main()` prefixes `error: ` and prints to stderr.
Plain `anyhow::bail!` / `.with_context(...)` → exit 1. Only `BoxError::{MissingInput,
UnsupportedHashLength}` map to exit 2 — these four introduce **no** new exit-2 variants, so
every error they raise is exit 1.
```rust
// json (D-06): parse error → stderr, exit 1 (main() adds the `error:` prefix)
match serde_json::from_str::<serde_json::Value>(&text) {
    Err(e) => anyhow::bail!("at line {} column {}: {e}", e.line(), e.column()),
    Ok(value) => { /* ... */ }
}
// ascii: decode/open error → exit 1, never a panic
let img = image::open(&self.path)
    .with_context(|| format!("failed to read image {}", self.path))?;
```

### Pure helper + `#[cfg(test)]` seam (the unit-test pattern)
**Source:** `src/commands/color/mod.rs:143-167` (`rgb_to_hsl`) + `:169-226` (tests);
`src/commands/cowsay/mod.rs:70-154` (`wrap`/`bubble`) + `:156-249` (tests)
**Apply to:** all four — extract the per-cell math as a free, crate-free function and
unit-test it without a terminal: `rgb_at(phase)` (lolcat), `luma_to_char(byte, ramp)` +
`rows = cols*sh/sw/2` (ascii), the drop/fade model + katakana table (matrix), `colorize(&Value)`
+ key-order (json). Run via `cargo test --bin box <module>` — **never `--lib`** (binary-only crate).

---

## Pattern Assignments

### `src/cli.rs` (config — variant swap)

**Analog:** itself (lines 68-74) — replace each unit variant with the data-carrying form
already used by lines 36-66.

**Current (lines 67-74):**
```rust
/// Rainbow-colorize piped text
Lolcat,
/// Run the Matrix digital-rain screensaver effect
Matrix,
/// Render an image as ASCII art
Ascii,
/// Pretty-print and validate JSON
Json,
```

**Target shape (mirror lines 44-48):**
```rust
/// Rainbow-colorize piped text
Lolcat(crate::commands::lolcat::LolcatArgs),
/// Run the Matrix digital-rain screensaver effect
Matrix(crate::commands::matrix::MatrixArgs),
/// Render an image as ASCII art
Ascii(crate::commands::ascii::AsciiArgs),
/// Pretty-print and validate JSON
Json(crate::commands::json::JsonArgs),
```
Keep the `///` doc comment (it is the `--help` one-liner, cli.rs:3-4).

---

### `src/main.rs` (config — dispatch)

**Analog:** itself (lines 61-75 are the live `=> args.run()` precedent; lines 76-79 are the
arms to delete).

**Current (lines 76-79):**
```rust
Commands::Lolcat => Err(not_implemented("lolcat")),
Commands::Matrix => Err(not_implemented("matrix")),
Commands::Ascii => Err(not_implemented("ascii")),
Commands::Json => Err(not_implemented("json")),
```

**Target (mirror line 65):**
```rust
Commands::Lolcat(args) => args.run(),
Commands::Matrix(args) => args.run(),
Commands::Ascii(args) => args.run(),
Commands::Json(args) => args.run(),
```
Leaves `qr`/`clip`/`pomodoro`/`weather` (lines 80-83) on `not_implemented`, so the
`use crate::commands::stub::not_implemented;` import (main.rs:22) stays.

---

### `src/commands/mod.rs` (config — module registry)

**Analog:** itself (lines 7-22, the alphabetized `pub mod` list).

Add four lines in alpha order: `pub mod ascii;`, `pub mod json;`, `pub mod lolcat;`,
`pub mod matrix;` (between `hash`/`passgen` and `du`/`epoch`/`fortune` as alpha dictates).
The `[01-03] allow-then-remove` dead-code precedent (input.rs:41-46) does not apply — these
modules are live the moment cli.rs/main.rs reference them.

---

### `Cargo.toml` (config — manifest)

**Analog:** the existing `[dependencies]` block (lines 11-47), specifically the
comment-annotated feature-trim style of `chrono` (line 33-34) and `uuid` (line 28-29).

Add under `[dependencies]` (RESEARCH:54-63):
```toml
# ascii (D-01/D-02). Discretion: trim to PNG/JPEG only.
image = "0.25.10"
# json (D-04). preserve_order pulls indexmap→hashbrown; serde comes transitively.
serde_json = { version = "1.0.150", features = ["preserve_order"] }
# lolcat (D-12/D-13)
unicode-width = "0.2"
strip-ansi-escapes = "0.2"
```
`crossterm` (line 21), `owo-colors` (line 19), `enable-ansi-support` (line 20) already present
— matrix and all color paths need no manifest change. `[dev-dependencies]` (lines 49-55)
already has `assert_cmd`, `predicates`, `assert_fs`, `tempfile`, `trycmd` — no test-dep change.

---

### `src/commands/lolcat/mod.rs` (command, transform)

**Analog:** `src/commands/color/mod.rs` (exact — text-in + `is_color_on()`-gated `.truecolor`)

**Imports pattern** (mirror color/mod.rs:17-22, add the two new crates):
```rust
use clap::Args;
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthChar;
use crate::commands::RunCommand;
use crate::core::output::is_color_on;
```

**Args struct** (mirror color/mod.rs:28-32):
```rust
#[derive(Debug, Args)]
pub struct LolcatArgs {
    /// Text to colorize; omit to read from piped stdin.
    pub input: Option<String>,
}
```

**Input + gated emit** (input from color/mod.rs:38; gate from color/mod.rs:53-58;
strip+per-char loop from RESEARCH:296-322):
```rust
let raw = crate::core::input::read_input(self.input)?;
let clean = strip_ansi_escapes::strip_str(&raw);   // D-13 unconditional
let spread = 3.0_f64;
for (line_idx, line) in clean.split_inclusive('\n').enumerate() {
    let mut phase = line_idx as f64 * spread;       // per-line offset → diagonal
    for c in line.chars() {
        if c == '\n' { print!("\n"); continue; }
        let w = UnicodeWidthChar::width(c).unwrap_or(0) as f64;
        if c.is_whitespace() {
            print!("{c}");                          // colored space is invisible
        } else if is_color_on() {
            let (r, g, b) = rgb_at(phase);
            print!("{}", c.truecolor(r, g, b));
        } else {
            print!("{c}");                          // plain: byte-identical minus ANSI
        }
        phase += w;
    }
}
```

**Pure helper to unit-test** (the `rgb_to_hsl` seam from color/mod.rs:143-167; math from
RESEARCH:188-194):
```rust
fn rgb_at(phase: f64) -> (u8, u8, u8) {
    let f = 0.1;
    let r = ((f * phase + 0.0).sin() * 127.0 + 128.0) as u8;
    let g = ((f * phase + 2.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    let b = ((f * phase + 4.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    (r, g, b)
}
```

---

### `src/commands/json/mod.rs` (command, transform)

**Analog:** `src/commands/color/mod.rs` (exact — text-in + gated color + parse-error→exit-1)

**Imports pattern** (mirror color/mod.rs:17-22):
```rust
use clap::Args;
use owo_colors::OwoColorize;
use crate::commands::RunCommand;
use crate::core::output::is_color_on;
```

**Args struct** (mirror color/mod.rs:28-32, add the boolean flag in the cowsay `#[arg(long...)]`
style at cowsay/mod.rs:49):
```rust
#[derive(Debug, Args)]
pub struct JsonArgs {
    /// JSON text to format; omit to read from piped stdin.
    pub input: Option<String>,
    /// Minify instead of pretty-printing.
    #[arg(long)]
    pub compact: bool,
}
```

**Validate + colorize-or-delegate** (input from color/mod.rs:38; `bail!` from color/mod.rs:72;
gate from color/mod.rs:53-58; full shape from RESEARCH:325-343):
```rust
let text = crate::core::input::read_input(self.input)?;
match serde_json::from_str::<serde_json::Value>(&text) {
    Err(e) => anyhow::bail!("at line {} column {}: {e}", e.line(), e.column()), // → exit 1
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
`to_string_pretty` is fixed at 2-space (RESEARCH:105) — no `PrettyFormatter` needed.

**Pure helper to unit-test** (color/mod.rs:143-167 seam): a `colorize(&Value, indent) -> String`
walker matching the 6 `Value` variants (RESEARCH:102), plus a key-order test
(`{"b":1,"a":2}` stays `b` before `a` — `preserve_order`, D-04).

---

### `src/commands/ascii/mod.rs` (command, file-I/O → transform)

**Analog:** `src/commands/cowsay/mod.rs` (render + width reproducibility) +
`src/commands/hash/mod.rs` (path-arg field)

**Imports pattern** (cowsay/mod.rs:26-28 base + the image trait, RESEARCH Pitfall 1):
```rust
use clap::Args;
use anyhow::Context;
use image::GenericImageView;          // REQUIRED for .dimensions() — or use .width()/.height()
use crate::commands::RunCommand;
```

**Args struct** (path field mirrors hash/mod.rs's `pub path: Option<String>`, but ascii needs
a concrete path for `image::open`):
```rust
#[derive(Debug, Args)]
pub struct AsciiArgs {
    /// Path to a PNG or JPEG image (extension must match the format).
    pub path: String,
}
```

**Width reproducibility precedent (cowsay/mod.rs:48-51 + doc:6-8):** cowsay deliberately uses a
**fixed** width for pipe-vs-TTY reproducibility. ascii intentionally **diverges** — it uses
`core::output::terminal_width()` (output.rs:215) which already falls back to 80 when piped
(RESEARCH Pitfall 8). Document this divergence; it is the D-02 decision, not a violation.

**Decode → resize → ramp** (RESEARCH:346-361; `with_context` error style from color/hash):
```rust
let img = image::open(&self.path)
    .with_context(|| format!("failed to read image {}", self.path))?;   // → exit 1
let (sw, sh) = img.dimensions();
let cols = crate::core::output::terminal_width() as u32;                 // 80 when piped
let rows = (cols * sh / sw / 2).max(1);                                  // /2 = cell aspect
let gray = img.resize_exact(cols, rows, image::imageops::FilterType::Triangle).to_luma8();
const RAMP: &[u8] = b" .:-=+*#%@";
let buf = gray.as_raw();
for y in 0..rows {
    let mut line = String::with_capacity(cols as usize);
    for x in 0..cols { line.push(luma_to_char(buf[(y*cols+x) as usize], RAMP) as char); }
    println!("{line}");
}
```

**Pure helper to unit-test** (cowsay/mod.rs:70-154 seam): `luma_to_char(luma, ramp)` (monotonic
0→darkest, 255→lightest) and the `rows = cols*sh/sw/2` formula. Monochrome v1 — no color path
(D-03), so this module does NOT import `owo_colors`/`is_color_on`.

---

### `src/commands/matrix/mod.rs` (command, event-driven)

**Analog:** partial — no existing raw-mode/animation loop in the repo. Closest precedents:
`src/core/output.rs:215-220` (the only existing `crossterm::terminal::size()` caller) for the
crossterm-import style, and `src/commands/cowsay/mod.rs:70-154` for the pure-helper extraction
shape. The loop skeleton itself comes from RESEARCH:364-385 (verified APIs), not a repo analog.

**Args struct** (cowsay-style, likely empty or just doc):
```rust
#[derive(Debug, Args)]
pub struct MatrixArgs {}   // no flags in v1 (charset/speed/color = VIS-V2-01)
```

**Loop skeleton + RAII guard** (RESEARCH:202-215, :364-385 — verified crossterm 0.29 APIs):
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
// in run():
crossterm::terminal::enable_raw_mode()?;
let mut out = std::io::stdout();
crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen, crossterm::cursor::Hide)?;
let _guard = RawGuard;                                  // armed right after setup (D-10)
let (cols, rows) = crossterm::terminal::size()?;
loop {
    // 1. update per-column drop state
    // 2. queue! the WHOLE frame (head bright, trail green→dark, erase past tail)
    out.flush()?;                                        // ONCE per frame (D-08)
    if crossterm::event::poll(std::time::Duration::from_millis(50))? {   // = 20 FPS timer
        if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
            if k.kind == crossterm::event::KeyEventKind::Press {         // Windows double-fire
                use crossterm::event::{KeyCode, KeyModifiers};
                let quit = matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
                    || (k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL));
                if quit { break; }
            }
        }
    }
}
```

**Pure helpers to unit-test** (cowsay seam): the per-column drop/fade model (head advance,
trail fade green→dark by distance behind head, reset on clear-bottom) and the katakana glyph
table (all in U+FF66–U+FF9D and all East-Asian-Width Narrow / width-1). Keep the loop
panic-free (no `.unwrap()` on terminal ops) so the Drop guard is the real restore path under
`panic = "abort"` (RESEARCH A1 / Cargo.toml:61). Document the CJK-font requirement in `--help`
(D-07 tofu note).

---

### `tests/lolcat.rs` (integration test)

**Analog:** `tests/color.rs` (exact)

Mirror the `color()` / `color_stdout()` runner shape (color.rs:13-32): `Command::cargo_bin("box")`,
`.arg("lolcat")`, `.env("NO_COLOR", "1")`. Assert the no-ANSI invariant the same way
(color.rs:45: `assert!(!out.contains('\u{1b}'))`). LOL-01 specifics (RESEARCH:440-441):
piped → no `\x1b`, multi-byte UTF-8 intact; incoming ANSI stripped (`"\x1b[31mx\x1b[0m"` →
plain `x`). Pass input as a positional arg (color uses `.arg(input)`) or via stdin.

---

### `tests/json.rs` (integration test)

**Analog:** `tests/color.rs` (runner + exit-1 shape) + `tests/tree.rs:98-116`
(`tree_piped_no_ansi` — the run-WITHOUT-NO_COLOR pipe-path proof)

Reuse color.rs's malformed-input pattern verbatim for the JSON-01 invalid case
(color.rs:67-74): `.failure().code(1).stdout(is_empty()).stderr(is_empty().not())`, then add
`predicate::str::contains("line")` + `contains("column")` (RESEARCH:447). Valid pretty/compact:
assert 2-space output / single-line minify. Key-order: pipe `{"b":1,"a":2}`, assert `b` appears
before `a`. Use the `tree_piped_no_ansi` byte-scan (`!out.stdout.windows(2).any(|w| w==b"\x1b[")`)
for the piped-no-ANSI assertion. Optional `tests/cmd/json.trycmd` for the locked 2-space layout
(snapshot the color.trycmd plain shape — RESEARCH:462).

---

### `tests/ascii.rs` + fixtures (integration test)

**Analog:** `tests/tree.rs` (temp-dir / checked-in fixtures) + `tests/hash.rs:36-42`
(`assert_fs` child-file fixture) + `tests/cmd/tree.in/` (checked-in input tree)

Two fixture options, both precedented:
- **Checked-in tiny images** (RESEARCH:446/459): `tests/cmd/ascii.in/{tiny.png,tiny.jpg}` —
  mirrors `tests/cmd/tree.in/project/` (checked-in input the test reads). Assert exit 0 +
  non-empty ASCII for each format.
- Bad/missing file → exit 1, no panic: reuse tree.rs's friendly-error assertion shape
  (tree.rs:121-142, `.failure().stderr(contains(...))`).

Runner mirrors `tree()` (tree.rs:29-38) / `hash_cmd()` (hash.rs:24-34): `cargo_bin("box")`,
`.arg("ascii")`, `.env("NO_COLOR","1")`. Pipe-assert an 80-col render is acceptable (RESEARCH:285).

---

### `tests/matrix.rs` (smoke test)

**Analog:** `tests/color.rs` (runner shape only — no snapshot)

MTRX-01's animation is not auto-snapshotable (RESEARCH:442, :464). Smoke-test the enter/exit
contract only: `Command::cargo_bin("box").arg("matrix")` with **non-TTY stdin / a fed `q`** →
the binary starts and exits non-hanging without leaving artifacts. Use `assert_cmd`'s
`.write_stdin(...)` + a timeout posture; the visual rain is human-verify. The drop/fade model
and glyph-table correctness are covered by the in-module `#[cfg(test)]` unit tests, not here.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none — full file-level coverage) | — | — | Every file has a same-repo precedent. The **matrix raw-mode/animation loop body** is the only logic with NO existing analog (no command yet owns terminal raw-mode state); its loop skeleton comes from RESEARCH:364-385 (verified crossterm 0.29 APIs), while its module skeleton, pure-helper seam, and test posture still follow the repo patterns above. |

---

## Metadata

**Analog search scope:** `src/cli.rs`, `src/main.rs`, `src/commands/mod.rs`,
`src/commands/{color,cowsay,hash}/mod.rs`, `src/core/{output,input}.rs`, `src/commands/stub.rs`,
`tests/{color,tree,hash}.rs`, `tests/cmd/{color.trycmd,tree.trycmd,tree.in/}`, `Cargo.toml`.
**Files scanned:** 16
**Module layout confirmed:** one directory per command (`src/commands/<cmd>/mod.rs`) —
all 4 new commands follow this (matches `color/`, `cowsay/`, `hash/` etc.).
**Pattern extraction date:** 2026-06-24
