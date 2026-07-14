---
phase: 04-terminal-visuals
plan: 02
subsystem: ui
tags: [lolcat, owo-colors, truecolor, unicode-width, strip-ansi-escapes, rainbow, terminal, clap]

# Dependency graph
requires:
  - phase: 04-terminal-visuals (04-01 json)
    provides: the is_color_on()-gated pure-colorizer template + clap command-registry wiring (cli.rs enum variant → main.rs dispatch → commands/mod.rs)
  - phase: 02 (color)
    provides: src/core/output::is_color_on() the single color gate + the text-in `.truecolor` analog (color/mod.rs)
  - phase: 01 (foundation)
    provides: src/core/input::read_input (arg → piped stdin → exit-2 TTY), the RunCommand trait, the 0/1/2 exit mapping in main.rs
provides:
  - "box lolcat: a live command rendering a smooth diagonal truecolor rainbow over piped/passed text; piped-to-file yields clean plain UTF-8 (all ANSI stripped, byte-identical minus color)"
  - "pure rgb_at(phase) sine-wave gradient fn (freq 0.1, 120°/240° offsets, floor 128) — reusable gradient math"
  - "the per-Unicode-scalar, width-aware gated-emit pattern (UnicodeWidthChar phase advance + is_color_on() gate)"
  - "unicode-width 0.2 + strip-ansi-escapes 0.2 in the manifest (strip-ansi actions the standing STATE.md todo)"
affects: [matrix (04-04), ascii (04-03), any future text-recoloring command]

# Tech tracking
tech-stack:
  added: [unicode-width 0.2.2, strip-ansi-escapes 0.2.1 (transitive vte 0.14.1)]
  patterns:
    - "per-Unicode-scalar gated emit: strip_ansi_escapes::strip_str unconditionally → iterate split_inclusive('\\n') seeding a per-line phase → advance phase by UnicodeWidthChar::width(c) → .truecolor gated SOLELY on is_color_on()"
    - "pure crate-free gradient helper (rgb_at) with an in-module #[cfg(test)] seam, run via cargo test --bin box lolcat (NOT --lib)"

key-files:
  created:
    - src/commands/lolcat/mod.rs
    - tests/lolcat.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/cli.rs
    - src/main.rs
    - src/commands/mod.rs

key-decisions:
  - "rgb_at maps the sine [-1,1] via *127+128 into [1,255] so the `as u8` cast never wraps (no saturating_cast / clamp needed); floor 128 = full-saturation rainbow"
  - "SPREAD=3.0 per-line phase seed (line_idx * SPREAD) gives the diagonal; freq 0.1 and starting phase FIXED — no --freq/--seed/--animate flags (those are VIS-V2-01)"
  - "whitespace advances the phase but is emitted uncolored (a colored space is invisible); newlines emitted raw via println!() (clippy-preferred over print!(\"\\n\"), identical single \\n byte)"
  - "strip_ansi_escapes::strip_str runs UNCONDITIONALLY on both the color and no-color paths (D-13) — the T-04L-01 terminal-escape-injection mitigation, and actions the standing STATE.md strip-ansi todo"

patterns-established:
  - "Per-Unicode-scalar width-aware gated emit (D-12): the gradient phase advances by display width so multi-byte/CJK is never byte-split — carries forward to matrix"
  - "Unconditional incoming-ANSI strip before re-emit (D-13): any command that re-emits untrusted piped text should strip_str first"

requirements-completed: [LOL-01]

# Metrics
duration: 3min
completed: 2026-06-24
---

# Phase 4 Plan 02: lolcat Summary

**`box lolcat` ships a smooth diagonal truecolor rainbow (classic sine-wave RGB, freq 0.1, 120°/240° offsets, floor 128) over piped/passed text, colored per Unicode scalar with width-aware phase advance; incoming ANSI is stripped unconditionally and piped output is clean plain UTF-8 byte-identical minus color.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-06-24T12:36:27Z
- **Completed:** 2026-06-24T12:40:03Z
- **Tasks:** 2 (Task 1 RED, Task 2 TDD GREEN)
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments
- `box lolcat` is live — the `not_implemented("lolcat")` arm is deleted; the command renders a rainbow gradient in a TTY and clean plain UTF-8 when piped.
- Pure `rgb_at(phase)` gradient fn implementing the locked D-11 math, unit-tested for channel range, phase-0 floor, and 120°-spacing.
- Per-Unicode-scalar, width-aware gated emit (D-12): phase advances by `UnicodeWidthChar::width(c)`; multi-byte/CJK never byte-split; `.truecolor` gated solely on `is_color_on()` (D-14).
- Incoming ANSI stripped unconditionally via `strip_ansi_escapes::strip_str` (D-13 / T-04L-01) — actions the standing STATE.md strip-ansi todo.
- `unicode-width` + `strip-ansi-escapes` added to the manifest (verified-legitimate per the threat register).

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1: Wave-0 failing tests for LOL-01** — `6be3e57` (test)
2. **Task 2: Implement lolcat slice — LolcatArgs + rgb_at + strip + gated per-char emit** — `078b636` (feat)

**Plan metadata:** (this commit) (docs: complete plan)

_No REFACTOR commit needed — the GREEN implementation was clean (clippy -D warnings + fmt passed)._

## Files Created/Modified
- `src/commands/lolcat/mod.rs` (created) — `LolcatArgs` + `RunCommand` impl (strip → per-line/per-char gated emit) + pure `rgb_at(phase)` + 4 `#[cfg(test)]` unit tests.
- `tests/lolcat.rs` (created) — 3 LOL-01 integration tests: piped→plain-no-ANSI byte round-trip, multi-byte UTF-8 intact, incoming-ANSI stripped.
- `Cargo.toml` (modified) — added `unicode-width = "0.2"` + `strip-ansi-escapes = "0.2"` (annotated).
- `Cargo.lock` (modified) — locked unicode-width 0.2.2, strip-ansi-escapes 0.2.1, transitive vte 0.14.1.
- `src/cli.rs` (modified) — `Lolcat` unit variant → `Lolcat(crate::commands::lolcat::LolcatArgs)`.
- `src/main.rs` (modified) — replaced `Commands::Lolcat => Err(not_implemented("lolcat"))` with `=> args.run()`.
- `src/commands/mod.rs` (modified) — added `pub mod lolcat;` in alpha order.

## Decisions Made
- **`*127+128` mapping is overflow-safe by construction:** the sine is bounded to `[-1,1]`, so each channel is bounded to `[1,255]` — the `as u8` cast can never wrap, so no clamp/saturating cast is needed.
- **Diagonal via per-line phase seed (`SPREAD = 3.0`):** discretion within D-11 (which fixes freq 0.1). No animation/seed/freq flags — deferred to VIS-V2-01.
- **Newlines emitted via `println!()` not `print!("\n")`:** identical single `\n` byte; clippy `print_with_newline` prefers `println!()`. Whitespace is emitted uncolored but still advances the phase (a colored space is invisible, but the gradient must stay continuous).
- **Cargo.lock committed with the manifest:** binary crate — the lockfile is part of the reproducible-build contract ([01-01]).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `print!("\n")` → `println!()` for clippy `-D warnings`**
- **Found during:** Task 2 (verification gate)
- **Issue:** The plan's pseudocode emitted newlines via `print!("\n")`, which trips the clippy `print_with_newline` lint; `cargo clippy -- -D warnings` (a hard gate) failed to compile.
- **Fix:** Replaced `print!("\n")` with `println!()` — emits the identical single `\n` byte, so the per-char output and all three integration tests are unchanged (behavior-preserving).
- **Files modified:** src/commands/lolcat/mod.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` clean; `cargo test --test lolcat` still 3/3 green; piped `printf 'a\nb\n'` round-trips byte-identical (`od -c` shows `a \n b \n`).
- **Committed in:** `078b636` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking).
**Impact on plan:** The single fix is a behavior-preserving lint compliance with no scope change. No architectural deviation.

## Known Stubs
None. `box lolcat` is a complete vertical slice wired to real input/output; no placeholder data or unwired components. (3 Phase-4/5 stubs remain elsewhere: matrix, ascii, plus the Phase-5 commands — out of scope for this plan.)

## Issues Encountered
- A first draft of `tests/lolcat.rs` had a borrow-after-move on `out.stdout` (moved into `String::from_utf8` then borrowed for the byte scan). Fixed by capturing `out.stdout.contains(&0x1b)` into a `has_esc` bool before consuming `stdout` into a `String`. This was caught at the Task-1 RED compile (the test must compile against the current binary), corrected, and the three tests then failed for the right reason (binary stubs lolcat → exit 1).

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- Phase 4 is 2/4 plans complete (04-01 json ✓, 04-02 lolcat ✓). Next: 04-03 ascii (`image` 0.25.10 to add) and 04-04 matrix (crossterm raw-mode loop; crossterm/owo-colors already present).
- The per-Unicode-scalar width-aware gated-emit pattern and the unconditional incoming-ANSI strip are now established and reusable for matrix.
- No blockers.

## Self-Check: PASSED

- FOUND: `src/commands/lolcat/mod.rs`
- FOUND: `tests/lolcat.rs`
- FOUND: `.planning/phases/04-terminal-visuals/04-02-SUMMARY.md`
- FOUND commit: `6be3e57` (test — Task 1 RED)
- FOUND commit: `078b636` (feat — Task 2 GREEN)

---
*Phase: 04-terminal-visuals*
*Completed: 2026-06-24*
