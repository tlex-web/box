# Phase 2: Pure Transform Utilities - Pattern Map

**Mapped:** 2026-06-22
**Files analyzed:** 36 (9 CLI variant swaps in 1 file + 9 command modules + 1 shared helper + 1 core registration + 9 integration tests + 2 trycmd snapshots + 1 data dir w/ 3 assets + 1 main.rs dispatch + 1 Cargo.toml + edits to errors.rs)
**Analogs found:** strong analog for all command/test/CLI surfaces; 3 net-new surfaces (`core::input`, `src/data/*` `include_str!`, `BoxError::MissingInput`→exit-2) flagged with closest partial reference.

The repo currently has **exactly one fully-implemented command** (`flatten`, a directory module at `src/commands/flatten/`). It is the canonical analog for every Phase-2 command. The supporting infrastructure (`RunCommand` trait, `BoxError`, `core::output` color gate, `not_implemented` dispatch, the two test files + one trycmd snapshot) is all Phase-1 code and is the source of truth for conventions. Phase 2 is high-repetition: clone the `flatten` shape nine times, adapting the Args struct + pure logic + tests per command.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli.rs` (swap 9 unit variants → Args structs) | CLI registry | request-response | existing `Flatten(FlattenArgs)` variant @ cli.rs:36 | exact |
| `src/main.rs` (remove 9 `not_implemented` arms → `.run()`) | dispatch | request-response | `Commands::Flatten(args) => args.run()` @ main.rs:61 | exact |
| `src/commands/mod.rs` (register 9 `pub mod`) | module registry | n/a | `pub mod flatten;` @ mod.rs:7 | exact |
| `src/core/mod.rs` (register `pub mod input;`) | module registry | n/a | `pub mod output;` @ core/mod.rs:9 | exact |
| `src/core/errors.rs` (add `MissingInput` variant) | typed error | n/a | `BoxError::NotImplemented` @ errors.rs:10 | role-match (NET-NEW variant) |
| `src/core/input.rs` | shared helper | request-response (stdin/arg) | `std::io::IsTerminal` gate @ output.rs:14,43 | partial (NET-NEW surface) |
| `src/commands/uuid/mod.rs` | command logic | transform (random) | `src/commands/flatten/mod.rs` | role-match |
| `src/commands/base64/mod.rs` | command logic | transform (binary I/O) | `src/commands/flatten/mod.rs` | role-match |
| `src/commands/epoch/mod.rs` | command logic | transform (parse/format) | `src/commands/flatten/mod.rs` | role-match |
| `src/commands/color/mod.rs` | command logic | transform + styled output | `flatten/mod.rs` + `output.rs` color gate | role-match |
| `src/commands/passgen/mod.rs` | command logic | transform (CSPRNG) | `flatten/mod.rs` (+ `flatten/rename.rs` for the const-table + pure-fn split) | role-match |
| `src/commands/cowsay/mod.rs` | command logic | transform (text wrap) | `flatten/mod.rs` (+ `output.rs::truncate_middle` for char-count wrap) | role-match |
| `src/commands/fortune/mod.rs` | command logic | transform (random pick) | `flatten/mod.rs` | role-match |
| `src/commands/eight_ball/mod.rs` | command logic | transform (random pick) | `flatten/mod.rs` (+ `rename.rs::RESERVED` for the `const &[&str]` table) | role-match |
| `src/commands/roast/mod.rs` | command logic | transform (random pick) | `flatten/mod.rs` | role-match |
| `src/data/eff_large_wordlist.txt` | embedded data | n/a | none | **NO ANALOG** |
| `src/data/fortunes.txt` | embedded data | n/a | none | **NO ANALOG** |
| `src/data/roasts.txt` | embedded data | n/a | none | **NO ANALOG** |
| `tests/uuid.rs` | test | request-response | `tests/flatten.rs` + `tests/cli.rs` | exact |
| `tests/base64.rs` | test | request-response | `tests/flatten.rs` + `tests/cli.rs` | exact |
| `tests/epoch.rs` | test | request-response | `tests/flatten.rs` + `tests/cli.rs` | exact |
| `tests/passgen.rs` | test | request-response | `tests/flatten.rs` + `tests/cli.rs` | exact |
| `tests/fortune.rs` | test | request-response | `tests/cli.rs` (`eight_ball_varies_across_runs` RESEARCH pattern) | role-match |
| `tests/eight_ball.rs` | test | request-response | `tests/cli.rs` + RESEARCH varies-across-runs pattern | role-match |
| `tests/roast.rs` | test | request-response | `tests/cli.rs` + RESEARCH varies-across-runs pattern | role-match |
| `tests/cmd/cowsay*.trycmd` | snapshot test | n/a | `tests/cmd/help.trycmd` | exact |
| `tests/cmd/color*.trycmd` | snapshot test | n/a | `tests/cmd/help.trycmd` | exact |
| `Cargo.toml` (add 4 deps) | config | n/a | existing `[dependencies]` block @ Cargo.toml:11-27 | exact |

---

## Shared Patterns

These cross-cutting patterns apply to MOST or ALL nine commands. The planner should reference them once and apply per plan rather than re-deriving.

### Pattern A — `RunCommand` trait + Args struct (apply to ALL 9 command modules)
**Source:** `src/commands/mod.rs:10-15` (trait) + `src/commands/flatten/mod.rs:25-46,93-94,202-203`

The trait is one method, static dispatch, returns `anyhow::Result<()>`:
```rust
// src/commands/mod.rs:13-15
pub trait RunCommand {
    fn run(self) -> anyhow::Result<()>;
}
```

Every command module clones this exact shape — derive `Args`, doc-comment each field (becomes `--help` text), implement `RunCommand`:
```rust
// src/commands/flatten/mod.rs:25-46 (imports + Args struct shape to clone)
use anyhow::{bail, Context};
use clap::Args;
use crate::commands::RunCommand;

/// `box flatten <src> <out> [--dry-run]` — flatten a folder tree into one
/// directory (FLAT-01..04).
#[derive(Debug, Args)]
pub struct FlattenArgs {
    /// Source directory tree to flatten.
    pub src: PathBuf,
    /// Output directory to copy every file into (created if missing).
    pub out: PathBuf,
    /// Preview the plan without writing anything.
    #[arg(long)]
    pub dry_run: bool,
}

// src/commands/flatten/mod.rs:93-94 — the impl block opens the same way
impl RunCommand for FlattenArgs {
    fn run(self) -> anyhow::Result<()> {
        // ...orchestrate: acquire input → call pure fn → println! to stdout
        Ok(())
    }
}
```
Note: `flatten` is a **directory module** (`src/commands/flatten/mod.rs` + sibling `rename.rs`). Phase 2 follows the same `src/commands/<cmd>/mod.rs` convention (RESEARCH "Recommended Project Structure"). `8ball` → module dir `eight_ball/` but CLI name stays `8ball` via `#[command(name = "8ball")]` (already in cli.rs:52).

### Pattern B — pure functions + `#[cfg(test)] mod tests` in-module (apply to ALL 9)
**Source:** `src/commands/flatten/mod.rs:331-365` (pure `human_size` + `collision_reason` with unit tests)

`run()` is a thin orchestrator; the real logic is pure, terminal-free functions that get unit-tested in the same file. This is the model for: `color::rgb_to_hsl`/hex parse, `cowsay` wrap+hard-break, `passgen` charset membership, EFF list parse (7776, no digits), `epoch` format parsing.
```rust
// src/commands/flatten/mod.rs:332-344,346-357 — pure fn + its in-module test
fn human_size(bytes: u64) -> String { /* ...no I/O... */ }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1024), "1.0 KB");
        // ...
    }
}
```
The `flatten/rename.rs` sibling shows the same discipline for "dangerous edge cases isolated as pure fns" — relevant to `cowsay` hard-break and `passgen` charset.

### Pattern C — color gating: REUSE `core::output`, never a parallel path (apply to `color`, and `cowsay`/`fortune` if styled)
**Source:** `src/core/output.rs:32-34` (`is_color_on`), `:121-129` (the gate-on-our-flag pattern), `:195-200` (`terminal_width`)

The locked rule (D-10): consult `is_color_on()` and only emit ANSI when true, so plain output is byte-identical minus ANSI. owo-colors plain methods (`.green()`, `.truecolor()`) emit ANSI unconditionally, so you MUST gate on the flag, not on the owo-colors override alone:
```rust
// src/core/output.rs:121-129 — the EXACT gate the color swatch must mirror
let glyph_str = if is_color_on() {
    match status {
        RowStatus::Copy => glyph.green().to_string(),
        RowStatus::Rename => glyph.yellow().to_string(),
        RowStatus::Skip => glyph.red().to_string(),
    }
} else {
    glyph.to_string()
};
```
For the `color` swatch this becomes (RESEARCH Code Examples):
```rust
use owo_colors::OwoColorize;
use crate::core::output::is_color_on;
let swatch = "██████████";
if is_color_on() { println!("{}", swatch.truecolor(r, g, b)); }
else { println!("{swatch}"); }     // byte-identical minus ANSI (D-10)
```
`init_color()` is already called once in `main()` (main.rs:58) before dispatch — commands never call it. `terminal_width()` (output.rs:195) is available for `fortune` fit-to-terminal but cowsay deliberately ignores it (fixed 40, D-11).

### Pattern D — stdout=data / stderr=messages + exit-code mapping (apply to ALL 9)
**Source:** `src/main.rs:60-95`

Data goes to stdout via `println!`; errors propagate as `anyhow::Result` and `main()` owns the exit mapping (`Ok`→0, `Err`→1, clap parse error→2). Commands NEVER call `std::process::exit`:
```rust
// src/main.rs:86-94 — the single exit-code owner
match result {
    Ok(()) => ExitCode::SUCCESS,
    Err(e) => {
        eprintln!("error: {e:#}");   // stderr only; stdout stays clean for pipes
        ExitCode::from(1)
    }
}
```

### Pattern E — integration test harness via `assert_cmd` (apply to ALL 9 test files)
**Source:** `tests/flatten.rs:14-24` (helper + `NO_COLOR` env) and `tests/cli.rs:42-75` (exit-code assertions)

Black-box test against the compiled binary; force `NO_COLOR=1` so output is deterministic regardless of the runner's TTY:
```rust
// tests/flatten.rs:15-24 — per-command runner helper + NO_COLOR
fn flatten(src: &Path, out: &Path, dry_run: bool) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("flatten").arg(src).arg(out);
    if dry_run { cmd.arg("--dry-run"); }
    cmd.env("NO_COLOR", "1");   // force plain output regardless of TTY
    cmd.assert()
}
```
```rust
// tests/cli.rs:66-75 — the exit-code + stream-discipline assertion template
Command::cargo_bin("box").unwrap()
    .arg("qr").assert()
    .failure().code(1)
    .stderr(predicate::str::contains("not yet implemented"))
    .stdout(predicate::str::is_empty());
```

### Pattern F — non-determinism testing: membership + N-runs-differ (apply to `uuid`, `passgen`, `fortune`, `eight_ball`, `roast`)
**Source:** RESEARCH.md:497-511 (the canonical anti-flake pattern; no existing repo file — RESEARCH is the analog).

Run N times, collect into a `HashSet`, assert ≥2 distinct (or, for uuid/passgen `--count`, assert exactly N distinct). NEVER assert exact random values:
```rust
// RESEARCH.md:501-511 — clone this for fortune/8ball/roast varies-across-runs
use std::collections::HashSet;
use assert_cmd::Command;
let mut seen = HashSet::new();
for _ in 0..10 {
    let out = Command::cargo_bin("box").unwrap()
        .args(["8ball", "will it work?"]).output().unwrap();
    seen.insert(String::from_utf8_lossy(&out.stdout).trim().to_string());
}
assert!(seen.len() >= 2, "8ball must vary across runs (got {seen:?})");
```

### Pattern G — trycmd snapshot (apply to `cowsay`, `color` ONLY — deterministic output)
**Source:** `tests/cli.rs:100-103` (registration) + `tests/cmd/help.trycmd` (transcript format)

```rust
// tests/cli.rs:101-103 — glob-registers every tests/cmd/*.trycmd
fn trycmd() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd");
}
```
The transcript format (`tests/cmd/help.trycmd:1-5`): fenced block, `$ box <cmd> ...` line, then expected stdout. Note `box[EXE]` placeholder for the Windows `.exe` suffix. NEVER snapshot random commands (they flake); force `NO_COLOR` so no ANSI leaks into the snapshot.

---

## Pattern Assignments (per file)

### `src/cli.rs` — swap 9 unit variants for Args structs
**Analog:** the existing `Flatten(FlattenArgs)` variant.
- **Read first:** `src/cli.rs:33-83`
- Each Phase-2 variant is currently a **unit** variant with a `///` doc comment (cli.rs:37-55). Swap to carry an Args struct, keeping the doc comment verbatim (it is snapshot-locked by help.trycmd — RESEARCH A5):
```rust
// FROM (cli.rs:37-38):
/// Generate a random UUID
Uuid,
// TO:
/// Generate a random UUID
Uuid(crate::commands::uuid::UuidArgs),
```
- `EightBall` keeps `#[command(name = "8ball")]` (cli.rs:52) untouched.
- ⚠️ help.trycmd (tests/cmd/help.trycmd:9-17) locks the exact `about` one-liners. Keep doc comments byte-identical or regenerate with `TRYCMD=overwrite cargo test`.

### `src/main.rs` — remove 9 `not_implemented` arms
**Analog:** `Commands::Flatten(args) => args.run()` @ main.rs:61.
- **Read first:** `src/main.rs:60-84`
- Replace each `Commands::Uuid => Err(not_implemented("uuid"))` (main.rs:62-70) with `Commands::Uuid(args) => args.run()`. The `not_implemented` import (main.rs:22) stays while ANY stub remains (Phases 3-5).

### `src/commands/mod.rs` — register 9 modules
**Analog:** `pub mod flatten;` @ mod.rs:7.
- **Read first:** `src/commands/mod.rs:7-8`
- Add `pub mod uuid; pub mod base64; pub mod epoch; pub mod color; pub mod passgen; pub mod cowsay; pub mod fortune; pub mod eight_ball; pub mod roast;`

### `src/core/mod.rs` — register the input module
**Analog:** `pub mod output;` @ core/mod.rs:9.
- **Read first:** `src/core/mod.rs:7-9`
- Add `pub mod input;` and a one-line module-doc mention (core/mod.rs:1-5 is the doc-comment style to extend).

### `src/core/errors.rs` — add `MissingInput`→exit-2 variant  ⚠️ NET-NEW
**Analog:** `BoxError::NotImplemented` @ errors.rs:6-12 (closest partial — same enum, same thiserror derive, but `NotImplemented` maps to exit 1; `MissingInput` must map to exit **2**).
- **Read first:** `src/core/errors.rs:1-12` AND `src/main.rs:86-94` (the mapping site).
- Add a variant following the existing `#[error("...")]` style:
```rust
// extends src/core/errors.rs:7-12 pattern
#[error("no input: pass an argument or pipe data")]
MissingInput,
```
- ⚠️ **Exit-2 wiring is the open landmine (RESEARCH Pitfall 2 / Open Question 1).** main.rs:88-93 currently maps EVERY `Err`→1. To get exit 2 for the TTY-no-input case, `main()` must downcast the error and special-case `BoxError::MissingInput`. Closest existing pattern is how `NotImplemented` is constructed (`stub::not_implemented` @ stub.rs:8-10) and printed — but NO existing code maps a variant to exit 2 from the `Err` arm (clap's exit-2 at main.rs:42-51 is a separate path). This is genuinely net-new dispatch logic; the foundation slice must add a downcast in main.rs's `Err(e)` arm.

### `src/core/input.rs` — shared stdin/arg reader  ⚠️ NET-NEW SURFACE
**Analog:** the `std::io::IsTerminal` gate in `src/core/output.rs:14,42-44` (closest partial — same "consult `is_terminal()` once, branch on it" shape; `core::output` uses `stdout().is_terminal()`, `core::input` mirrors it with `stdin().is_terminal()`).
- **Read first:** `src/core/output.rs:14,42-44` (the IsTerminal gate to mirror) AND `src/core/errors.rs:1-12` (for the `MissingInput` return).
```rust
// src/core/output.rs:42-44 — the gate shape core::input mirrors for stdin
pub fn color_enabled(no_color_flag: bool) -> bool {
    !no_color_flag && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}
```
- Implement `read_input(arg: Option<String>) -> Result<String>` (text) and `read_input_bytes(arg) -> Result<Vec<u8>>` (binary, `read_to_end`) per D-04/D-05. Branch 3 (no arg + interactive TTY) returns `BoxError::MissingInput` (NOT `bail!`, so it can map to exit 2).
- RESEARCH.md:540-543 recommends factoring an inner `read_input_inner(arg, is_tty: bool, reader)` so the TTY decision is unit-testable without a real TTY. No repo analog for that injection pattern — it is net-new but low-risk.

### `src/commands/uuid/mod.rs` (command, random)
**Analog:** `src/commands/flatten/mod.rs` (Pattern A + B + F).
- **Read first:** `src/commands/flatten/mod.rs:25-46` (Args + imports), `:93-94` (impl), `:346-357` (cfg(test)).
- Args: `{ count: u8 (#[arg(short='n', long="count", default_value_t=1)]), upper: bool }` (RESEARCH.md:213-220). `run()`: loop `count`, `uuid::Uuid::new_v4().to_string()`, uppercase if flag, `println!`. No `core::input`. Test in `tests/uuid.rs` (Pattern E + F + v4 regex).

### `src/commands/base64/mod.rs` (command, binary I/O)
**Analog:** `src/commands/flatten/mod.rs` + `core::input::read_input_bytes`.
- **Read first:** `src/commands/flatten/mod.rs:25-46,93-94` AND `src/core/input.rs` (once written).
- Args: `{ decode: bool, url_safe: bool, input: Option<String> }`. `run()`: `read_input_bytes(self.input)?` → `STANDARD`/`URL_SAFE_NO_PAD` engine encode/decode (RESEARCH Pattern 5; `.trim()` before decode — Pitfall 3). Unit-test round-trip on raw `Vec<u8>` incl. non-UTF-8 bytes.

### `src/commands/epoch/mod.rs` (command, parse/format)
**Analog:** `src/commands/flatten/mod.rs` + `core::input::read_input` (optional; epoch mostly arg-driven).
- **Read first:** `flatten/mod.rs:25-46,93-94,346-357`.
- Args: `{ value: Option<String> }`. Pure `parse_date(s) -> Result<i64>` trying RFC3339 → `%Y-%m-%d %H:%M:%S` (local) → `%Y-%m-%d` (local) per RESEARCH Pattern 6 (D-12), unit-tested as a round-trip. No-arg → `Utc::now().timestamp()`. Bad format → clear error (→ exit 1 via Pattern D).

### `src/commands/color/mod.rs` (command, transform + styled output)
**Analog:** `flatten/mod.rs` + `core::output` color gate (Pattern C) + `core::input::read_input` (optional).
- **Read first:** `flatten/mod.rs:25-46,93-94` AND `src/core/output.rs:32-34,121-129` (is_color_on + the gate).
- Args: `{ input: Option<String> }`. Pure fns: hex parse incl. `#abc` short-expand (Pitfall 8), RGB parse (`"r,g,b"` and `"r g b"`, D-13), `rgb_to_hsl` (RESEARCH Code Examples lines 437-453) — all unit-tested (HSL round-trip ±1). Output the aligned `label : value` block + gated `██` swatch (Pattern C). Snapshot the block (NO_COLOR) via `tests/cmd/color*.trycmd` (Pattern G).

### `src/commands/passgen/mod.rs` (command, CSPRNG)
**Analog:** `flatten/mod.rs` (orchestration) + `flatten/rename.rs:23-26` (the `const &[&str] RESERVED` table is the model for the curated symbol-set constant + the pure-fn-with-unit-tests split).
- **Read first:** `flatten/mod.rs:25-46,93-94` AND `flatten/rename.rs:18-26` (const table style).
- Args: `{ length: usize (default 16), count: usize (default 1), words: Option<usize>, no_symbols: bool, ... }`. CSPRNG via `OsRng.unwrap_err()` (RESEARCH Pattern 4); unbiased `IndexedRandom::choose`/`random_range` — NEVER `% len` (D-02). EFF wordlist via `include_str!` (see data files below). Curated paste-safe symbol const. stdout-only (D-14). Tests: length, charset membership, `--no-symbols`, `--count N` distinct, wordlist==7776 (Pitfall 7).

### `src/commands/cowsay/mod.rs` (command, text wrap)
**Analog:** `flatten/mod.rs` + `core::input::read_input` + `output.rs::truncate_middle:174-190` (the closest existing char-count string manipulation — cowsay's greedy wrap is a sibling discipline).
- **Read first:** `flatten/mod.rs:25-46,93-94` AND `src/core/output.rs:174-190` (char-count string handling) AND `src/core/input.rs`.
- Args: `{ text: Option<String>, width: usize (default 40) }`. Pure greedy-wrap + hard-break fn (Pitfall 5: split words > width into width chunks; unit-test no line exceeds width). Fixed 40 cols, NOT `terminal_width()` (D-11). Single-line `< … >` vs multi-line `/ \`/`| |`/`\ /` border. Snapshot single + multi-line via `tests/cmd/cowsay*.trycmd` (Pattern G).

### `src/commands/fortune/mod.rs` (command, random pick)
**Analog:** `flatten/mod.rs` (Pattern A + F).
- **Read first:** `flatten/mod.rs:25-46,93-94` AND RESEARCH.md:497-511.
- Args: `{}` (no flags required). `include_str!("../../data/fortunes.txt")` (Pitfall 6: path relative to THIS file), parse to `&[&str]`, `rand::rng()` + `IndexedRandom::choose`. Expose `pub(crate) fn entries() -> &'static [&str]` so the test can assert membership (RESEARCH.md:495). Test: membership + N-runs-differ (Pattern F).

### `src/commands/eight_ball/mod.rs` (command, random pick)
**Analog:** `flatten/mod.rs` + `flatten/rename.rs:23-26` (the canonical `const &[&str]` table — the 20 8ball answers live in-source the same way `RESERVED` does).
- **Read first:** `flatten/mod.rs:25-46,93-94` AND `flatten/rename.rs:21-26` (const table) AND RESEARCH.md:497-511.
- Module dir is `eight_ball/`; CLI name `8ball` (cli.rs:52, already set). Args: `{ question: Option<String> }` (optional, ignored for the draw). `const EIGHT_BALL_ANSWERS: &[&str] = &[ ...20... ]`. `rand::rng()` + `choose`. Test: output ∈ the 20 + N-runs-differ.

### `src/commands/roast/mod.rs` (command, random pick)
**Analog:** `flatten/mod.rs` (identical shape to `fortune`).
- **Read first:** `flatten/mod.rs:25-46,93-94` AND `src/commands/fortune/mod.rs` (once written — they are near-identical). RESEARCH.md:497-511.
- Args: `{}`. `include_str!("../../data/roasts.txt")`, parse, `rand::rng()` + `choose`. Test: membership + N-runs-differ.

### `src/data/eff_large_wordlist.txt`, `fortunes.txt`, `roasts.txt`  ⚠️ NO ANALOG
**Net-new — `src/data/` does not exist (verified: Glob `src/data/**` → no files).** No embedded-asset precedent in the repo.
- **Closest partial reference:** `flatten/rename.rs:23-26` `const RESEARCH &[&str]` is the in-source analog for *small* lists (used for the 20 8ball answers). The text files are the *large-list* path with no precedent.
- EFF wordlist: 7776 words, strip leading dice-code+TAB at load (Pitfall 7), CC-BY 3.0 US attribution REQUIRED (CONTEXT.md:98). fortunes ~50-150 CC0; roasts ~30-80 CC0/self-authored.
- `include_str!` path is relative to the including `.rs` file (Pitfall 6): from `src/commands/<cmd>/mod.rs` → `include_str!("../../data/<file>.txt")`. Verify with a trivial build.

### `tests/uuid.rs`, `tests/base64.rs`, `tests/epoch.rs`, `tests/passgen.rs`
**Analog:** `tests/flatten.rs` (harness + helper) + `tests/cli.rs` (exit codes).
- **Read first:** `tests/flatten.rs:1-24` (helper + NO_COLOR) AND `tests/cli.rs:42-75` (exit/stream assertions). For uuid/passgen also RESEARCH.md:512-513 (uniqueness via HashSet + v4 regex).

### `tests/fortune.rs`, `tests/eight_ball.rs`, `tests/roast.rs`
**Analog:** `tests/cli.rs` + RESEARCH.md:497-511 (Pattern F — the only model for varies-across-runs).
- **Read first:** `tests/cli.rs:1-50` AND RESEARCH.md:492-513. Assert membership + ≥2 distinct over N runs; never exact value.

### `tests/cmd/cowsay*.trycmd`, `tests/cmd/color*.trycmd`
**Analog:** `tests/cmd/help.trycmd` (the only existing transcript).
- **Read first:** `tests/cmd/help.trycmd:1-38` (fenced `$ box ...` + expected stdout, `box[EXE]` placeholder). Registered automatically by the existing glob in `tests/cli.rs:101-103` — no test-fn change needed. Generate with `TRYCMD=overwrite cargo test` then review (RESEARCH A4: verify CRLF on Windows).

### `Cargo.toml` — add 4 deps
**Analog:** existing `[dependencies]` block @ Cargo.toml:11-27 (commented-by-section style).
- **Read first:** `Cargo.toml:11-35`.
- Add (RESEARCH "Installation"): `uuid = { version = "1.23.3", features = ["v4"] }`, `base64 = "0.22.1"`, `chrono = { version = "0.4.45", default-features = false, features = ["clock", "std"] }`, `rand = "0.9"`. Conditionally `rand_core = "0.9"` only if `OsRng.unwrap_err()` import doesn't resolve through `rand` (RESEARCH A1). Dev-deps (`assert_cmd`/`predicates`/`assert_fs`/`tempfile`/`trycmd`/`insta`) are ALL already present (Cargo.toml:29-35) — do not re-add.

---

## No Analog Found

| File / Mechanism | Role | Why net-new | Closest partial reference |
|------------------|------|-------------|---------------------------|
| `src/data/eff_large_wordlist.txt` + `fortunes.txt` + `roasts.txt` | embedded data | `src/data/` does not exist; no `include_str!` asset precedent in the repo | `flatten/rename.rs:23-26` (`const &[&str]` — the small-list in-source analog only) |
| `src/core/input.rs` (the module) | shared helper | first shared stdin/arg reader; first consumer of a TTY gate on *stdin* | `src/core/output.rs:14,42-44` (the `stdout().is_terminal()` gate — same shape, different stream) |
| `BoxError::MissingInput`→exit-2 mapping in `main.rs` | error→exit dispatch | NO existing `Err`-arm path maps a variant to exit 2 (clap's exit-2 @ main.rs:42-51 is a separate, pre-dispatch path; the `Err(e)` arm @ main.rs:88-93 flattens everything to 1) | `stub::not_implemented` @ stub.rs:8-10 + `BoxError::NotImplemented` @ errors.rs:7-11 (the typed-variant *construction* pattern) — but the exit-2 downcast in main.rs is genuinely new |

---

## Metadata

**Analog search scope:** `src/cli.rs`, `src/main.rs`, `src/commands/{mod.rs,stub.rs,flatten/mod.rs,flatten/rename.rs}`, `src/core/{mod.rs,output.rs,errors.rs}`, `tests/{cli.rs,flatten.rs}`, `tests/cmd/help.trycmd`, `Cargo.toml`. Confirmed absent: `src/data/**`, `src/core/input.rs`, any second built command.
**Files scanned:** 13 source/test/config files read in full or targeted ranges.
**Pattern extraction date:** 2026-06-22
