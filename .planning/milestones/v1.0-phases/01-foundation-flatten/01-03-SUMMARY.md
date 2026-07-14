---
phase: 01-foundation-flatten
plan: 03
subsystem: commands
tags: [rust, flatten, walkdir, dunce, filetimes, collision-rename, reserved-names, dry-run, tdd, clap-derive]

# Dependency graph
requires:
  - "01-01: RunCommand trait, Commands enum + Flatten stub arm, main() dispatch + exit-code mapping, tests/cli.rs harness, dev-deps (assert_cmd/predicates/assert_fs/tempfile/trycmd)"
  - "01-02: core::output (color gate, RowStatus/format_row/terminal_width/dry_run_summary/real_run_summary D-09/D-11) + core::fs (normalize_path dunce, is_hidden walkdir predicate, safe_copy FileTimes)"
provides:
  - "box flatten <src> <out> [--dry-run] — the real, safe file-flattening anchor command (FLAT-01..04)"
  - "flatten/rename.rs pure collision encoding: encode_relative (separator-free, traversal-stripped), sanitize_reserved (CON/PRN/AUX/NUL/COM1-9/LPT1-9 + trailing dot/space), dedupe (NTFS case-insensitive numeric fallback)"
  - "flatten planner/executor split: one Plan(items, to_copy, renamed, skipped) feeds both dry-run print and real copy (preview can never diverge from the run)"
  - "tests/flatten.rs — 8 Wave-0 integration tests covering every flatten behavior"
  - "first live caller of the core::output/core::fs surfaces — both module-level #[allow(dead_code)] removed"
affects: [phase-3 hash/tree/du/dupes/bulk-rename (reuse walkdir+rename+dry-run patterns), 01-04 install (flatten is the smoke-test command)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure collision-encoding in flatten/rename.rs (no I/O) so Windows reserved-name / path-injection / NTFS-case edge cases are unit-tested in isolation"
    - "Planner/executor split (Pattern 6): build one Plan first; dry-run prints it and stops; execute consumes the same plan — dry-run names match the real run exactly"
    - "Canonicalize-before-guard: create_dir_all(out) then dunce-normalize BOTH roots, then a case-insensitive containment guard BEFORE any copy (no output-inside-source loop)"
    - "occupied-name set seeded from read_dir(out) lowercased + inserted as the plan is built, so pre-existing AND within-run collisions both dedupe (no silent overwrite)"
    - "TDD RED (unimplemented!() bodies + failing tests) -> GREEN for both the pure rename task and the orchestration task"

key-files:
  created:
    - "src/commands/flatten/rename.rs"
    - "src/commands/flatten/mod.rs"
    - "tests/flatten.rs"
  modified:
    - "src/commands/mod.rs"
    - "src/cli.rs"
    - "src/main.rs"
    - "src/core/output.rs"
    - "src/core/fs.rs"

key-decisions:
  - "encode_relative neutralizes `..`/`.` traversal segments (not just leading separators) so no literal `..` survives the encoded name — closes T-03-pathinject beyond the RESEARCH sample, which left `.._escape.txt`"
  - "sanitize_reserved trims trailing dots/spaces FIRST, then tests the trimmed stem for a reserved match, so `con .txt` is recognised as reserved (the RESEARCH order tested the untrimmed stem and missed it)"
  - "Walk order is deterministic by walkdir: the FIRST file to claim a base name keeps it (Copy); later same-named files are collision-renamed — both files always land with distinct names, never lost"
  - "[collision] vs [collision xN] reason derived from whether dedupe appended a numeric suffix (N = suffix+1, the instance count)"
  - "human_size helper formats the {size} written byte count (B/KB/MB/GB/TB, 1-decimal) — Claude's-discretion accumulation during copy"
  - "Removed BOTH forward-compat #[allow(dead_code)] (core::output, core::fs) now that flatten is a live caller; clippy -D warnings stays clean, proving every helper is reached"

requirements-completed: [FLAT-01, FLAT-02, FLAT-03, FLAT-04]

# Metrics
duration: 6min
completed: 2026-06-22
---

# Phase 1 Plan 03: Flatten Anchor Command Summary

**`box flatten <src> <out> [--dry-run]` — a safe, no-silent-data-loss file-flattening tool: dunce-canonicalized roots with a case-insensitive output-inside-source guard, hidden-pruned/symlink-skipping walk, source-relative collision encoding with Windows-reserved-name sanitization and numeric dedupe, a one-Plan dry-run/execute split that preserves timestamps, and the locked D-09/D-11 output — backed by 8 integration tests and 10 pure unit tests.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-06-22T13:46:09Z
- **Completed:** 2026-06-22T13:52:35Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 3 created, 5 modified

## Accomplishments

- **Pure collision rename (`flatten/rename.rs`, FLAT-02).** `encode_relative` turns a source-relative path into a flat, separator-free filename and strips `..`/`.` traversal segments; `sanitize_reserved` neutralizes every Windows reserved device stem (CON/PRN/AUX/NUL/COM1-9/LPT1-9, case-insensitive, with/without extension) and trailing dots/spaces; `dedupe` appends `_N` before the extension with NTFS-case-insensitive keying. 10 unit tests, including a property assertion that no encoded name contains `\`, `/`, or `..` (T-03-pathinject).
- **Flatten orchestration (`flatten/mod.rs`, FLAT-01/03/04).** `FlattenArgs::run()` = create output dir (D-13) -> dunce-normalize both roots -> case-insensitive containment guard before any I/O (Pitfall 4) -> seed `occupied` from `read_dir(out)` (D-14) -> `WalkDir::follow_links(false).filter_entry(!is_hidden)` (D-12) skipping symlinks (Pitfall 8) -> build one `Plan` -> dry-run prints + writes nothing, OR execute copies via `core::fs::safe_copy` (timestamps preserved) and accumulates bytes. Copy I/O is `.context(...)`-wrapped so deep-path (>260) failures surface loudly (FOUND-06).
- **Locked output produced exactly.** Manual smoke test of `--dry-run` shows `+`/`~` glyphs, aligned `->` arrows, inline `[collision x2]`, and the verbatim two-line D-11 dry-run summary; piped output is plain (no ANSI), confirming the FOUND-04 color gate holds for the first real command.
- **Stub Flatten arm replaced.** `cli.rs` `Flatten` variant now carries `FlattenArgs`; `main.rs` arm is `Commands::Flatten(args) => args.run()`; `RunCommand`'s `#[allow(dead_code)]` and both `core::output`/`core::fs` module allows removed (flatten is the first live caller).
- **All gates green:** `cargo test` 37 (22 unit + 7 cli + 8 flatten), `cargo clippy --all-targets -- -D warnings` clean.

## Task Commits

Each task was committed atomically (both TDD: RED test -> GREEN feat):

1. **Task 1: rename.rs — pure collision encoding + reserved-name sanitization + dedupe** (TDD)
   - `82b1947` (test) — RED: 8 failing rename unit tests, function bodies `unimplemented!()`; flatten module wired; `RunCommand` allow removed
   - `2ab913c` (feat) — GREEN: real `encode_relative`/`sanitize_reserved`/`dedupe`, all 8 unit tests pass
2. **Task 2: flatten orchestration — canonicalize, guard, plan, dry-run/execute + tests/flatten.rs** (TDD)
   - `aa0b6b5` (test) — RED: 8 failing integration tests in `tests/flatten.rs`; `Flatten(FlattenArgs)` wired into `cli.rs`/`main.rs`; `run()` `unimplemented!()`
   - `5bd7605` (feat) — GREEN: full orchestration; both `core::output`/`core::fs` dead-code allows removed; full suite + clippy green

No separate refactor commits were needed (both GREEN implementations were already clean under `-D warnings`).

## Files Created/Modified

- `src/commands/flatten/rename.rs` — **created.** `RESERVED` slice + `encode_relative` / `sanitize_reserved` / `dedupe` pure functions; 10 inline unit tests.
- `src/commands/flatten/mod.rs` — **created.** `FlattenArgs` (clap derive) + `impl RunCommand`; `ItemKind`/`PlanItem`/`Plan`; `build_plan`, `collision_reason`, `arrow_col`, `print_plan`, `human_size`; 2 inline unit tests.
- `tests/flatten.rs` — **created.** 8 integration tests (assert_cmd + assert_fs): `dry_run_plans_collisions_writes_nothing`, `flatten_copies_all_files_flat`, `preserves_mtime`, `originals_untouched`, `no_silent_overwrite`, `out_inside_src_aborts`, `skips_symlinks`, `deep_path_no_silent_loss`.
- `src/commands/mod.rs` — declared `pub mod flatten;`; removed `RunCommand`'s `#[allow(dead_code)]`.
- `src/cli.rs` — `Flatten` variant now carries `FlattenArgs`.
- `src/main.rs` — `Flatten` arm dispatches `args.run()` (no longer the stub); imports `RunCommand`.
- `src/core/output.rs` — removed the module-level `#![allow(dead_code)]` (flatten now calls the row/summary helpers).
- `src/core/fs.rs` — removed the module-level `#![allow(dead_code)]` (flatten now calls `normalize_path`/`is_hidden`/`safe_copy`).

## Decisions Made

- **`encode_relative` strips `..`/`.` traversal segments, not just leading separators.** The RESEARCH sample produced `.._escape.txt` for `../escape.txt`. That is harmless as a filename (no separator can traverse), but the threat register (T-03-pathinject) requires that no literal `..` survives. The implementation splits the `_`-joined segments and drops any segment that is exactly `..` or `.`, so the encoded name is both separator-free and traversal-token-free.
- **`sanitize_reserved` trims trailing dots/spaces before the reserved match.** The RESEARCH code matched the *untrimmed* stem, so `con .txt` was not recognised as `CON`. Reordering (trim, then match the trimmed stem) makes `con .txt` -> `con_.txt`, closing a hidden-collision gap.
- **First-claim-wins, deterministic by walkdir order.** The first file to take a base name keeps it (Copy); later same-named files are collision-renamed (`encode_relative` + `dedupe`). Both files always land with distinct names — nothing is silently lost or overwritten.
- **`human_size` for the `{size} written` summary** (B/KB/MB/GB/TB, 1 decimal) — the byte count is accumulated cheaply during the copy loop (Claude's discretion in the plan).
- **Removed both forward-compat `#[allow(dead_code)]`.** Now that flatten reaches every `core::output`/`core::fs` helper, the allows are gone and `clippy -D warnings` still passes — a positive proof that the reusable surface has no orphan functions.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `encode_relative` left a literal `..` in the encoded name**
- **Found during:** Task 1 (GREEN — my own `encode_no_separator` property test failed)
- **Issue:** The RESEARCH implementation (`replace(['\\','/'],"_")` + `trim_start_matches('_')`) turns `../escape.txt` into `.._escape.txt`. The threat register (T-03-pathinject) and the test require that no `..` survives the encoded name.
- **Fix:** After separator replacement, split on `_` and drop any segment that is exactly `..` or `.`, then rejoin. The encoded name is now both separator-free and traversal-token-free for all inputs (verified across nested paths and `..`-laden inputs).
- **Files modified:** src/commands/flatten/rename.rs
- **Verification:** `encode_no_separator` (8 inputs incl. `../escape.txt`, `a/../../b/c.txt`, `..\\..\\windows\\system32\\evil.dll`) passes.
- **Committed in:** 2ab913c (Task 1 GREEN commit)

**2. [Rule 1 - Bug] `sanitize_reserved` missed reserved stems with trailing dots/spaces**
- **Found during:** Task 1 (GREEN — `sanitize_trims_trailing_dots_and_spaces` failed)
- **Issue:** The reserved-name match ran against the untrimmed stem, so `con .txt` (stem `"con "`) did not match `CON` and was returned as `con.txt` — Windows would then trim the trailing space, re-creating the reserved device name.
- **Fix:** Trim trailing `.`/` ` from the stem first, then run the case-insensitive reserved match against the trimmed stem. `con .txt` -> `con_.txt`.
- **Files modified:** src/commands/flatten/rename.rs
- **Verification:** `sanitize_trims_trailing_dots_and_spaces` + `sanitize_reserved_covers_every_class` pass.
- **Committed in:** 2ab913c (Task 1 GREEN commit)

**3. [critical_reuse_note] Removed the two forward-compat `#[allow(dead_code)]` (planned cleanup)**
- **Found during:** Task 2 (after GREEN, wave clippy gate)
- **Issue:** Plan 01-02 left module-level `#![allow(dead_code)]` on `core::output` and `core::fs` because no caller existed. This plan's `critical_reuse_note` item 2 mandates removing them now.
- **Fix:** Deleted both allows; `cargo clippy --all-targets -- -D warnings` confirms every helper is now reached by flatten (or by `init_color`/`format_row` internally).
- **Files modified:** src/core/output.rs, src/core/fs.rs
- **Verification:** clippy clean with `-D warnings`.
- **Committed in:** 5bd7605 (Task 2 GREEN commit)

---

**Total deviations:** 3 (2 Rule-1 bugs surfaced by my own RED/GREEN tests, 1 planned cleanup mandated by `critical_reuse_note`).
**Impact on plan:** The two bugs are direct improvements over the RESEARCH sample code and were caught by tests the plan asked for (`encode_no_separator`, `sanitize_reserved`) — they harden the documented threat mitigations rather than expand scope. No functionality beyond the plan's interfaces was added. No architectural change.

## Issues Encountered

- **`skips_symlinks` is privilege-dependent on Windows.** Creating a file symlink requires Developer Mode or elevation. The test creates the symlink best-effort and self-skips (with an `eprintln!`) if creation fails, so it never produces a false failure on an unprivileged runner. On this host symlink creation succeeded, so the skip-count + `(skipped: symlink)` assertions ran and passed.
- **`deep_path_no_silent_loss` adapts to host long-path support.** It builds a >300-char fixture; if the host's long-path support copies it, the test asserts the buried file is present (no silent loss); if a copy fails, it asserts a LOUD error on stderr (FOUND-06). If the OS refuses to even create the >260-char fixture, the test self-skips. It passed on this host.

## Honored Reuse Constraints (from plan 01-02 findings)

1. **Color routed through `core::output`.** All glyph/colored rows go through `format_row` (which gates on `is_color_on()`); no bare `.green()` calls in flatten. Piped output is plain (verified by smoke test + the existing `piped_help_has_no_ansi`).
2. **`#[allow(dead_code)]` removed** on both `core::output` and `core::fs` (deviation 3).
3. **`core::fs` helpers reused, not reimplemented:** `normalize_path` (both roots), `is_hidden` (`filter_entry`), `safe_copy` (copy + timestamps). No path/copy logic was hand-rolled in flatten.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready for plan 01-04 (install):** `box flatten` is now the working command the installer's `box --help` smoke test (and a real `box flatten` invocation) can exercise. The MSVC + crt-static release link is still unverified — confirm during 01-04 (carried over from 01-01).
- **Ready for Phase 3 (hash/tree/du/dupes/bulk-rename):** the walkdir + `filter_entry(!is_hidden)` traversal, the planner/executor `Plan` split, the dry-run/summary pattern, and `flatten/rename.rs` (reusable by `bulk-rename`) are all established and tested here.
- No blockers.

## Self-Check: PASSED

All 3 created files verified on disk (`src/commands/flatten/rename.rs`, `src/commands/flatten/mod.rs`, `tests/flatten.rs`) plus this SUMMARY; all 4 task commits (82b1947, 2ab913c, aa0b6b5, 5bd7605) verified in git history.

---
*Phase: 01-foundation-flatten*
*Completed: 2026-06-22*
