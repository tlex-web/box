---
phase: 04-terminal-visuals
reviewed: 2026-06-24T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - src/cli.rs
  - src/main.rs
  - src/commands/mod.rs
  - src/commands/json/mod.rs
  - src/commands/lolcat/mod.rs
  - src/commands/ascii/mod.rs
  - src/commands/matrix/mod.rs
  - tests/json.rs
  - tests/lolcat.rs
  - tests/ascii.rs
  - tests/matrix.rs
  - tests/cmd/json.trycmd
  - Cargo.toml
findings:
  critical: 1
  warning: 6
  info: 4
  total: 11
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-06-24
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Reviewed the four new Phase-4 terminal-visuals commands (`json`, `lolcat`, `ascii`,
`matrix`) plus their clap-registry wiring (`cli.rs`, `main.rs`, `commands/mod.rs`)
and their integration tests, against the established project conventions
(`core::output` color gate, `core::input` precedence, strict 0/1/2 exit codes,
RAII terminal restore, no-panic-on-adversarial-input).

Overall the code is careful and well-documented, and the pure helpers are
well-unit-tested. However there is one terminal-state-restoration **BLOCKER** in
`matrix` (a `?` between `enable_raw_mode()` and the `RawGuard` arm leaves the
terminal in raw mode on early return), plus several correctness WARNINGs:
unchecked `u32` arithmetic on untrusted image dimensions in `ascii` (debug-build
panic / release wrong-output on adversarial input), a trail-erase gap in `matrix`
that contradicts its "clean edge / no artifacts" guarantee at `speed == 2`, a
`json` parse-error path that violates the documented exit-code policy by treating
a parse failure as a runtime error, and a per-`char`/per-`print!` flushing pattern
in `lolcat` that emits unbuffered. Details below.

## Critical Issues

### CR-01: `matrix` leaves the terminal in raw mode if alternate-screen setup fails

**File:** `src/commands/matrix/mod.rs:116-121`
**Issue:** `enable_raw_mode()?` runs FIRST, but the `RawGuard` (whose `Drop` calls
`disable_raw_mode()`) is only armed AFTER the next fallible call:

```rust
enable_raw_mode()?;                                          // raw mode ON
let mut out = std::io::stdout();
crossterm::execute!(out, EnterAlternateScreen, cursor::Hide)?; // <-- can ? here
let _guard = RawGuard;                                       // guard armed too late
```

If `execute!(EnterAlternateScreen, cursor::Hide)` returns `Err` (a real
possibility on a redirected/odd stdout, or a transient console error), the `?`
propagates out of `run()` **before** `RawGuard` is constructed. Its `Drop` never
runs, so `disable_raw_mode()` is never called and the user's PowerShell session is
left in raw mode — no line echo, no cooked input, a broken terminal. This is
precisely the teardown-guarantee the module's own doc-comment (D-10) promises to
hold on "every non-aborting exit path", and it is the single most important
invariant for the only state-mutating command.

The module comment claims the guard is "armed immediately after setup", but
"after setup" includes a fallible step that runs while raw mode is already on.

**Fix:** Arm the guard the instant raw mode is enabled, so it covers the
alternate-screen/cursor setup too. The guard's `Drop` already tolerates being run
when the alternate screen was never entered (`LeaveAlternateScreen` is a no-op /
ignored-error in that case):

```rust
enable_raw_mode()?;
let _guard = RawGuard;            // arm BEFORE any further `?`
let mut out = std::io::stdout();
crossterm::execute!(out, EnterAlternateScreen, cursor::Hide)?;
```

## Warnings

### WR-01: `json` parse error returns exit 1 but is a *usage* (bad-input) error — and the doc-comment contradicts the policy

**File:** `src/commands/json/mod.rs:55`; policy in `src/main.rs:6-8,92-106`
**Issue:** A JSON parse failure is surfaced with `anyhow::bail!`, which `main()`
maps to exit **1** (runtime error). The project's exit-code policy is
"2 = usage error (bad user input), 1 = runtime error, never panic on bad user
input". Malformed JSON supplied by the user is the canonical *bad-input/usage*
case and arguably belongs at exit 2, consistent with `MissingInput` and
`UnsupportedHashLength` which are downcast-mapped to exit 2.

This is a deliberate, *documented* decision (D-06 says invalid JSON → exit 1), and
the integration test `invalid_json_exits_1_with_line_and_column` pins exit 1 — so
this is a convention-consistency call, not an outright bug. But it is an
inconsistency worth flagging: `box hash --verify` with a wrong-length hash is exit
2, while `box json` with syntactically broken input is exit 1. If exit 1 is
intended, the divergence from the "2 = bad user input" rule should be called out
explicitly in `main()`'s policy comment so a future reader does not "fix" it.

**Fix:** Either (a) keep exit 1 but document the carve-out in `main.rs` ("parse
errors from `json` are runtime errors, not usage errors, by D-06"), or (b) if
alignment with the usage-error convention is desired, introduce a typed
`BoxError::InvalidInput`-style variant and downcast-map it to exit 2 like the
other two usage variants. Do not leave the rule and the behavior silently at odds.

### WR-02: `ascii` does unchecked `u32` arithmetic on attacker-controlled image dimensions

**File:** `src/commands/ascii/mod.rs:65-66,78,108-109`
**Issue:** `src_w`/`src_h` come from an untrusted, user-supplied image. The row
computation and the buffer index are both plain `u32` arithmetic:

```rust
fn compute_rows(cols: u32, src_w: u32, src_h: u32) -> u32 {
    ((cols * src_h / src_w) / 2).max(1)   // cols * src_h can overflow u32
}
...
let luma = buf[(y * cols + x) as usize];  // y * cols + x can overflow u32
```

`cols` is the terminal width (crossterm caps at `u16::MAX = 65535`); `src_h` for a
decodable image can also approach the tens of thousands. `cols * src_h` and
`y * cols + x` are computed in `u32`. In a **debug/test build** (overflow checks
on — this is exactly how `cargo test` runs) a crafted-but-decodable image can make
these products exceed `u32::MAX` and **panic**, violating the "malformed/large
image must exit 1 cleanly with no panic" contract (FOUND-05 / T-04A-02). In a
release build the overflow wraps silently, producing a wrong `rows` value and a
garbled render rather than a panic — still incorrect. The existing tests only
exercise tiny 8×8 fixtures, so this path is unguarded by the suite.

**Fix:** Do the size math in a wider type and reject implausibly large images up
front. Widen to `u64`/`usize` for the intermediate products and clamp:

```rust
fn compute_rows(cols: u32, src_w: u32, src_h: u32) -> u32 {
    let rows = ((cols as u64 * src_h as u64) / src_w as u64 / 2).max(1);
    rows.min(u16::MAX as u64) as u32
}
...
let idx = y as usize * cols as usize + x as usize;
let luma = buf[idx];
```

and/or add an explicit upper-bound check on `src_w`/`src_h` after decode (alongside
the existing zero-dimension guard) that `bail!`s cleanly for absurd dimensions.

### WR-03: `matrix` trail-erase clears only one cell per frame but the head advances up to two — leaves artifacts

**File:** `src/commands/matrix/mod.rs:74-76,172-175,231-232`
**Issue:** Per-column speed is `SPEED_MIN..=SPEED_MAX` = `1..=2`, so a head can
advance **2** rows in a single `step`. But the per-frame cleanup erases exactly
**one** cell, the single cell just past the tail:

```rust
let erase_y = d.head - d.trail_len - 1;   // clears ONE row only
if erase_y >= 0 && erase_y < rows { queue!(out, MoveTo(x, erase_y as u16), Print(' '))?; }
```

When `speed == 2`, the tail moves down 2 rows per frame, so the row at
`old_head - trail_len - 1` (now two rows above the new erase point) is never
cleared and a stale dim-green glyph is left behind. Over time fast columns smear a
faint trail of un-erased glyphs down the screen. This directly contradicts the
module's stated "the cell just past the tail is erased … the trail has a clean
edge" / "no visual artifacts" guarantee. The unit tests cover `fade`/`step` math
but not the on-screen erase coverage, so this is invisible to the suite.

**Fix:** Erase the full band the tail swept this frame, not a single cell — clear
every row in `(head - trail_len - speed)..(head - trail_len)`:

```rust
for k in 1..=d.speed {
    let erase_y = d.head - d.trail_len - k;
    if erase_y >= 0 && erase_y < rows {
        queue!(out, MoveTo(x, erase_y as u16), Print(' '))?;
    }
}
```

(Or cap `SPEED_MAX = 1` if multi-row jumps are not actually desired.)

### WR-04: `lolcat` emits one unbuffered `print!`/`println!` per character — stdout lock + flush per scalar

**File:** `src/commands/lolcat/mod.rs:63-89`
**Issue:** The emit loop calls `print!`/`println!` once per Unicode scalar. Each
macro re-acquires the stdout lock and `print!` line-buffering means frequent
syscalls; for a large piped input this is many thousands of tiny writes. This is
the same class of issue the `matrix` module explicitly calls out as a pitfall
("never per character") — `lolcat` re-introduces it on the hot path. It is not a
correctness bug (output is byte-correct), but it is a robustness/throughput defect
and a deviation from the buffered-write pattern used elsewhere.

**Fix:** Build the colored line into a `String` (or write into a
`BufWriter<Stdout>` / locked `StdoutLock`) and emit once per line, mirroring the
`ascii` command which already accumulates into `let mut line = String::…` and
prints once per row:

```rust
use std::io::Write;
let stdout = std::io::stdout();
let mut w = std::io::BufWriter::new(stdout.lock());
// ... write!(w, "{}", c.truecolor(r,g,b))? etc., one flush at the end
```

### WR-05: `json` colorizes `bool` via `b.magenta()` on the `&&bool` — relies on Display passthrough, brittle vs. documented "delegate to serde"

**File:** `src/commands/json/mod.rs:101-104`
**Issue:** Two of the leaf renderers format the value through owo-colors on a
non-string type:

```rust
Value::Bool(b) => out.push_str(&b.magenta().to_string()),
Value::Number(n) => out.push_str(&n.to_string().yellow().to_string()),
```

`b` is `&bool`; `b.magenta()` wraps it and relies on `bool`'s `Display` to print
`true`/`false`. That happens to match JSON, but it bypasses the module's own
stated contract that the colored path is "byte-for-byte the same shape
`serde_json::to_string_pretty` produces … so piped (plain) and TTY (colored)
output differ only by the ANSI escapes". For `Number` this is the sharper risk:
`n.to_string()` is `serde_json::Number`'s Display, which is generally faithful, but
the module comment in `Cargo.toml:46-48` warns that number rendering is a known
D-04 landmine — hand-formatting numbers here instead of round-tripping through the
same serializer the plain path uses is the kind of divergence that comment warns
against. The `colorize_*` tests assert presence of substrings (`"1"`, `"true"`),
not byte-equality of the colored-stripped output vs. `to_string_pretty`, so a
real divergence would not be caught.

**Fix:** Add a test that strips ANSI from `colorize(&v, 0)` and asserts it equals
`serde_json::to_string_pretty(&v)` for a battery of values (floats, large ints,
nested), which is the actual D-05 invariant. If any divergence is found, render
scalars by formatting the `Value` through serde rather than per-type Display.

### WR-06: `lolcat` per-line phase seeding ignores `\r` and treats `split_inclusive` segments as visual lines — diagonal can desync on `\r\n` / no-final-newline input

**File:** `src/commands/lolcat/mod.rs:63-64`
**Issue:** `for (line_idx, line) in clean.split_inclusive('\n').enumerate()` seeds
`phase = line_idx * SPREAD`. On Windows-origin text a line is `"...\r\n"`; the `\r`
is iterated as a whitespace char and advances `phase` by its width (1) before the
`\n` resets to the next line — harmless for color but means the within-line phase
includes a phantom `\r` advance. More notably, if the final line has no trailing
`\n`, `split_inclusive` still yields it as its own segment, so the per-line diagonal
seeding is correct there; but a stray embedded `\r` not followed by `\n` (old-Mac
or mid-line carriage returns, which `strip_ansi_escapes` does NOT remove) is
emitted verbatim and can move the cursor to column 0, visually overwriting the
already-printed colored prefix of that line. This is cosmetic but is an input-driven
rendering glitch on adversarial/mixed-newline input.

**Fix:** This is low-severity; if tightening is desired, normalize or strip lone
`\r` after the ANSI strip, or explicitly document that only `\n` line breaks are
supported. At minimum add a test feeding `"a\rb\n"` to confirm the intended
behavior is locked.

## Info

### IN-01: `matrix` head/trail re-pick a random glyph every frame for every cell — flicker is intentional but undocumented per-cell, and `unwrap_or('ﾝ')` dead fallback

**File:** `src/commands/matrix/mod.rs:156,163`
**Issue:** `glyphs.choose(&mut rng).copied().unwrap_or('ﾝ')` — `glyphs` is the
non-empty katakana table (`katakana_glyphs()` always yields 56 entries), so
`choose` never returns `None` and the `unwrap_or('ﾝ')` branch is dead code. Not a
bug, but the fallback can never execute; a `.expect("katakana table is non-empty")`
would document the invariant more honestly than a silent fallback that hides a
would-be-empty-table regression.
**Fix:** Replace `unwrap_or('ﾝ')` with `.expect("katakana table is non-empty")`,
or hoist the glyph pick so the comment's "fresh random glyph per cell" intent is
explicit.

### IN-02: `ascii` `RAMP.len() - 1` and `cols as usize` casts assume non-empty ramp / fit — fine today, fragile to edits

**File:** `src/commands/ascii/mod.rs:94`
**Issue:** `luma_to_char` computes `(luma as usize * (ramp.len() - 1)) / 255`. If a
future edit made `RAMP` empty, `ramp.len() - 1` underflows (`usize` wrap →
out-of-bounds index → panic). `RAMP` is a const so this is safe now, but the
function takes `ramp: &[u8]` as if general-purpose while silently assuming
non-empty.
**Fix:** Add a debug assertion or early return for an empty ramp, or document the
non-empty precondition on the parameter.

### IN-03: `colorize` builds the whole output in one `String` then `print!`s it — fine, but no streaming for very large JSON

**File:** `src/commands/json/mod.rs:64,91-95`
**Issue:** The colored path materializes the entire pretty-printed document into a
single `String` before printing. For very large JSON this doubles peak memory
(parsed `Value` + the rendered string). Out of v1 perf scope and not a
correctness issue; noting for completeness since the plain path also buffers via
`to_string_pretty`.
**Fix:** None required for v1; if large-document support becomes a goal, stream the
walker into a `BufWriter`.

### IN-04: `cli.rs` doc-comment is stale — says "only `flatten` becomes functional, in plan 03" while json/lolcat/ascii/matrix are now wired

**File:** `src/cli.rs:8-10`
**Issue:** The module doc still reads "This phase registers all 23 commands; only
`flatten` becomes functional, in plan 03 … Until then every variant is routed to
the stub handler." That is no longer true — this phase wires `Json`, `Lolcat`,
`Ascii`, and `Matrix` to real `run()` impls (see `main.rs:76-79`). Stale comment,
no behavioral impact.
**Fix:** Update the doc-comment to reflect the commands now implemented, or drop
the phase-specific narrative that goes stale every phase.

---

_Reviewed: 2026-06-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
