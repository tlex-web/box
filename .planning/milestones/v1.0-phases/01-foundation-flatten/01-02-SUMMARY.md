---
phase: 01-foundation-flatten
plan: 02
subsystem: core-infra
tags: [rust, owo-colors, dunce, walkdir, filetimes, is-terminal, color-gating, path-normalization, tdd]

# Dependency graph
requires:
  - "01-01: crate scaffold, src/core/mod.rs, src/main.rs parse+dispatch, tests/cli.rs harness, locked dep set"
provides:
  - "core::output::color_enabled + init_color — single color gate (NO_COLOR + --no-color + TTY) installed once at startup"
  - "core::output reusable flatten-row helpers (D-09): RowStatus glyphs (+ ~ -), format_row with aligned -> arrow + middle-ellipsis truncation + width cap"
  - "core::output locked D-11 summary helpers (dry_run_summary, real_run_summary)"
  - "core::fs::normalize_path — dunce-based UNC-safe canonicalization (no \\?\ leak)"
  - "core::fs::is_hidden — walkdir filter_entry predicate (root gotcha + dotfile + FILE_ATTRIBUTE_HIDDEN)"
  - "core::fs::safe_copy — fs::copy + FileTimes mtime/atime preserve, .context() on every I/O"
  - "piped_help_has_no_ansi integration test proving off-TTY output is ANSI-free (FOUND-04)"
affects: [01-03 flatten, phase-3 hash/tree/du/dupes/bulk-rename, phase-4 visuals]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Color decided once at startup, stored in a process-global AtomicBool (COLOR_ON); row helpers gate on it because owo-colors .green() is unconditional and set_override only affects if_supports_color"
    - "Reusable, mostly-pure string-returning output helpers so layout is unit-testable without a TTY (glyph is source of truth, color is decoration)"
    - "dunce::canonicalize wrapped once in core::fs::normalize_path; std::fs::canonicalize banned"
    - "std::fs::FileTimes (stdlib, no filetime crate) for timestamp preservation; atime best-effort"
    - "TDD RED (unimplemented!() bodies + failing tests) -> GREEN (real impl) for the fs behavioral task"

key-files:
  created:
    - "src/core/output.rs"
    - "src/core/fs.rs"
  modified:
    - "src/core/mod.rs"
    - "src/main.rs"
    - "tests/cli.rs"
    - "Cargo.toml"
    - "Cargo.lock"

key-decisions:
  - "Gate row coloring on our own COLOR_ON AtomicBool, NOT owo-colors set_override — the plain .green() trait method is unconditional and ignores the override; only if_supports_color honors it"
  - "Enabled owo-colors `supports-colors` feature so set_override/with_override compile (the locked default-feature set excludes it)"
  - "safe_copy preserves atime best-effort (only when meta.accessed() succeeds) but always preserves mtime — Assumption A3; FLAT-04 only mandates mtime"
  - "fs unit tests live inline in src/core/fs.rs (run via cargo test --bin box) because this is a binary crate with no lib target; integration tests can only reach the CLI"

requirements-completed: [FOUND-04, FOUND-06]

# Metrics
duration: 6min
completed: 2026-06-22
---

# Phase 1 Plan 02: Core Infrastructure (output + fs) Summary

**Shared `core::output` (single color gate + reusable D-09 glyph rows and D-11 summary helpers) and `core::fs` (dunce UNC-safe normalize_path, the hidden-entry predicate with the walkdir#142 root gotcha, and timestamp-preserving safe_copy), with `init_color` wired into main() and a passing `piped_help_has_no_ansi` test — the color (FOUND-04) and path-safety (FOUND-06) contracts now hold globally.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-06-22T13:35:05Z
- **Completed:** 2026-06-22T13:41:38Z
- **Tasks:** 2 (one TDD)
- **Files modified:** 2 created, 5 modified

## Accomplishments

- **`core::output` color gate (FOUND-04, D-10).** `color_enabled(no_color_flag)` = `!flag && NO_COLOR unset && std::io::stdout().is_terminal()` (stdlib `IsTerminal`, no `is-terminal` crate). `init_color` decides once at startup and is wired into `main()` before dispatch. Color is gated on a process-global `COLOR_ON` flag rather than `owo_colors::set_override`, because the plain `.green()` trait method is unconditional — gating on our own flag is what guarantees the plain layout is byte-identical minus ANSI.
- **Reusable flatten-row helpers (D-09/D-10).** `RowStatus` (`+`/`~`/`-` ASCII glyphs as the source of truth), `format_row` (color-only-on-glyph, aligned `->` arrow column, terminal-width cap via `crossterm::terminal::size()` with an 80-col fallback, middle-ellipsis truncation keeping the filename visible), and `truncate_middle`/`terminal_width`. These are the contract plan 03 (flatten) consumes.
- **Locked D-11 summary helpers.** `dry_run_summary` and `real_run_summary` emit the exact locked wording, unit-tested against the literal strings.
- **`core::fs` path/hidden/copy helpers (FOUND-06, D-12).** `normalize_path` wraps `dunce::canonicalize` (no `\\?\` leak); `is_hidden` returns false at depth 0 (walkdir#142) and otherwise treats a dotfile or `FILE_ATTRIBUTE_HIDDEN` (0x2, `#[cfg(windows)]`) as hidden; `safe_copy` does `fs::copy` + `std::fs::FileTimes` (mtime always, atime best-effort) with `.context(...)` on every fallible call so deep-path failures surface loudly.
- **Proof of the color contract.** `piped_help_has_no_ansi` runs `box flatten --help` captured (not a TTY) and asserts no `\x1b[` byte sequence appears (FOUND-04, T-02-01).
- **All gates green:** `cargo build` 0, `cargo test` (12 inline + 7 integration) 0, `cargo clippy --all-targets -- -D warnings` clean.

## Task Commits

1. **Task 1: core/output.rs — color gating + reusable flatten-row helpers** — `da29710` (feat)
2. **Task 2: core/fs.rs — normalize_path + is_hidden + safe_copy; piped-no-ANSI test** (TDD)
   - `54287ae` (test) — RED: 5 failing fs unit tests + the `piped_help_has_no_ansi` integration test, fs bodies `unimplemented!()`
   - `f1baa03` (feat) — GREEN: real `normalize_path`/`is_hidden`/`safe_copy`, all tests pass

No separate refactor commit was needed (the GREEN implementation was already clean).

## Files Created/Modified

- `src/core/output.rs` — **created.** Color gate (`color_enabled`, `init_color`, `is_color_on`), `RowStatus` + `format_row` + `truncate_middle` + `terminal_width` (D-09/D-10), `dry_run_summary` + `real_run_summary` (D-11). 7 inline unit tests.
- `src/core/fs.rs` — **created.** `normalize_path` (dunce), `is_hidden` (walkdir predicate), `safe_copy` (FileTimes). 5 inline unit tests (incl. a Windows `attrib +h` hidden-attribute test).
- `src/core/mod.rs` — declared `pub mod output;` and `pub mod fs;`.
- `src/main.rs` — replaced the plan-01 `no_color` TODO with the real `crate::core::output::init_color(cli.no_color)` call before dispatch.
- `tests/cli.rs` — added the `piped_help_has_no_ansi` integration test.
- `Cargo.toml` — enabled the `supports-colors` feature on `owo-colors` (required for `set_override`).
- `Cargo.lock` — updated for the feature-driven transitive deps (`supports-color`, `is_ci`, `hermit-abi`, transitive `is-terminal`).

## Decisions Made

- **Color gated on our own `COLOR_ON` flag, not `owo_colors::set_override`.** The plain `OwoColorize` methods (`.green()` etc.) always emit ANSI regardless of the global override — `set_override` only governs the `if_supports_color` API. The first version relied on `set_override` and the inline tests caught that piped rows still carried ANSI (would have broken D-10 in flatten). Fixed by storing the color decision in a `static AtomicBool` that `format_row` consults; `init_color` still also calls `set_override` so any future `if_supports_color` call agrees.
- **`safe_copy` preserves atime best-effort, mtime always.** Some filesystems don't report `accessed()`; we add it only when available so the copy never fails for a missing atime while still preserving mtime (Assumption A3; FLAT-04 only mandates mtime).
- **fs unit tests live inline (`#[cfg(test)]` in `fs.rs`).** This is a binary crate with no `[lib]` target, so `tests/` integration tests can only exercise the compiled CLI. Internal helpers (`normalize_path`/`is_hidden`/`safe_copy`) are tested inline and run via `cargo test --bin box`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Enabled the `owo-colors` `supports-colors` feature**
- **Found during:** Task 1 (`cargo build`)
- **Issue:** `owo_colors::set_override` — the API the plan/RESEARCH Pattern 5 specifies for `init_color` — is gated behind the `supports-colors` feature, which the locked `Cargo.toml` did not enable. `cargo build` failed with `cannot find function set_override in crate owo_colors` ("item configured out").
- **Fix:** Added `features = ["supports-colors"]` to the existing `owo-colors = "4.3"` dependency. Same crate, same locked version — only enabling the feature the planned API requires (not a package substitution).
- **Files modified:** Cargo.toml, Cargo.lock
- **Verification:** `cargo build` exited 0 afterward.
- **Committed in:** da29710 (Task 1 commit)

**2. [Rule 1 - Bug] Row coloring leaked ANSI even when color was disabled**
- **Found during:** Task 1 (inline unit tests)
- **Issue:** `format_row` originally relied on `owo_colors::set_override(false)` to suppress color, calling `glyph.green().to_string()` unconditionally. The inline tests (`row_has_glyph_and_arrow`, `row_carries_reason`) failed: the plain `.green()` method always emits ANSI; `set_override` only affects the `if_supports_color` path. This would have produced `\x1b[`-laden rows in piped flatten output, violating D-10/FOUND-04.
- **Fix:** Introduced a process-global `COLOR_ON: AtomicBool` set by `init_color`; `format_row` colors the glyph only when `is_color_on()` is true, otherwise emits the bare glyph char. Added a `row_colors_glyph_when_enabled` test (serialized with the others via a `Mutex` because they mutate the global) and a no-ANSI assertion to the plain-row tests.
- **Files modified:** src/core/output.rs
- **Verification:** All 7 inline output tests pass; `piped_help_has_no_ansi` integration test passes.
- **Committed in:** da29710 (Task 1 commit)

**3. [Rule 3 - Blocking] `#[allow(dead_code)]` on the forward-compat helper surfaces**
- **Found during:** Task 1 (clippy gate) and Task 2 (build)
- **Issue:** The flatten-row/summary helpers (`output.rs`) and the fs helpers (`fs.rs`) have no caller yet (plan 03 is the first), so the binary build reports 10+ `never used` warnings and the wave-merge `cargo clippy -- -D warnings` gate would fail.
- **Fix:** Added a module-level `#![allow(dead_code)]` to each of `output.rs` and `fs.rs` with a comment noting plan 03 wires the first call site (mirrors plan 01-01's `RunCommand` `#[allow(dead_code)]`). The helpers are exercised by the inline unit tests regardless.
- **Files modified:** src/core/output.rs, src/core/fs.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` clean.
- **Committed in:** da29710 (output.rs), f1baa03 (fs.rs)

**4. [Rule 1 - Bug] clippy `manual_repeat_n` lint**
- **Found during:** Task 1 (clippy gate)
- **Issue:** `std::iter::repeat(' ').take(n)` in `format_row` tripped `clippy::manual_repeat_n` under `-D warnings`.
- **Fix:** Switched to `std::iter::repeat_n(' ', n)` as clippy suggested.
- **Files modified:** src/core/output.rs
- **Verification:** clippy clean.
- **Committed in:** da29710 (Task 1 commit)

---

**Total deviations:** 4 auto-fixed (2 blocking config/build, 2 bugs). All were necessary to satisfy the plan's own acceptance criteria (`cargo build` 0, the wave-merge clippy gate, and the D-10 byte-identical-minus-ANSI contract). No scope creep — no functionality beyond the plan's interfaces was added. **Deviation 2 is the most consequential:** without it, flatten output (plan 03) would have leaked ANSI when piped, silently breaking FOUND-04.

## Threat Surface

All four threat-register dispositions for this plan are satisfied:
- **T-02-01** (info disclosure / color off-TTY): `color_enabled` requires `is_terminal()` AND `NO_COLOR` unset; `piped_help_has_no_ansi` asserts no `\x1b[` when piped. Deviation 2 closed a real gap here.
- **T-02-02** (tampering / path normalization): `normalize_path` uses `dunce::canonicalize` only; `fs.rs` contains no `std::fs::canonicalize`.
- **T-02-03** (info disclosure-integrity / deep-path copy): every `safe_copy` I/O call (copy, metadata, modified, open, set_times) is wrapped in `.context(...)`; failures surface per-file, never silently dropped.
- **T-02-04** (DoS / is_hidden root gotcha): `is_hidden` returns false at depth 0, unit-tested by `is_hidden_false_for_root_even_if_dotted`, so a dotted source root is not pruned to zero files.

No new security surface beyond the plan's threat model was introduced.

## Known Stubs

The `core::output` row/summary helpers and `core::fs` helpers have no production call site yet — they are intentional reusable surfaces (`#[allow(dead_code)]`) consumed by **plan 01-03 (flatten)**, exactly as defined in this plan's `<interfaces>` block. They are fully implemented and unit-tested; not placeholders. The `#[allow(dead_code)]` should be removed in plan 03 once flatten wires the first call.

## Next Phase Readiness

- **Ready for plan 01-03 (flatten):** `core::output::{RowStatus, format_row, truncate_middle, terminal_width, dry_run_summary, real_run_summary}` and `core::fs::{normalize_path, is_hidden, safe_copy}` are the documented contracts flatten consumes. Flatten should: canonicalize src + out via `normalize_path`, run `WalkDir::new(src).filter_entry(|e| !is_hidden(e))`, build a plan, render rows with `format_row` (compute the max source-label width for `arrow_col`), copy via `safe_copy`, and print the locked summaries. Remove the two `#[allow(dead_code)]` once the first caller exists.
- **Color contract holds globally now:** any command can call `.green()`-style decoration and trust that piped output stays plain, *provided it gates on `core::output::is_color_on()` or routes glyph decoration through `format_row`* (the plain `.green()` method alone does NOT honor the gate — documented in `output.rs`).
- No blockers.

## Self-Check: PASSED

All created files verified on disk (`src/core/output.rs`, `src/core/fs.rs`, `01-02-SUMMARY.md`); all 3 task commits (da29710, 54287ae, f1baa03) verified in git history.

---
*Phase: 01-foundation-flatten*
*Completed: 2026-06-22*
