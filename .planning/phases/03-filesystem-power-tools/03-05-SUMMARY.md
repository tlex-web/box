---
phase: 03-filesystem-power-tools
plan: 05
subsystem: cli-command
tags: [bulk-rename, regex, rename, dry-run, preflight, collision, cycle, swap, path-separator, case-only, destructive, abort-all, windows-safe]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: "RunCommand trait + main() dispatch/exit-code policy; core::fs::is_hidden walk filter; core::output format_row/RowStatus/terminal_width/is_color_on; the cli.rs/main.rs stub-swap precedent; flatten's plan→preview→execute split + case-folded occupied-set model + encode_no_separator separator-refusal invariant"
  - phase: 03-filesystem-power-tools (plans 01-04)
    provides: "the binary-only-crate test-targeting note (cargo test --bin box, NOT --lib); the collect→sort-before-output determinism discipline; the WalkDir follow_links(false) + filter_entry(!is_hidden) + min_depth(1) scope pattern reused across tree/du/dupes; the abort-leaves-tree-unchanged snapshot test discipline from dupes_never_writes"
provides:
  - "Live `box bulk-rename <dir> <pattern> <replacement> [--force] [--recursive]`: regex first-match capture-group rename over the FULL base name; dry-run preview is the DEFAULT (writes nothing); --force executes; an in-memory ABORT-ALL-BEFORE-ANY-RENAME pre-flight detects collisions (target-vs-target + target-vs-existing), cycles/swaps, and path-separator injection and aborts the whole batch (exit 1, nothing written) in BOTH modes"
  - "The first pure, I/O-free safety detector in the repo decoupled from disk: preflight(renames, existing) -> Vec<Conflict>, unit-tested for every D-18 rule without a terminal or filesystem — the model for any future destructive command's pre-flight"
affects: [any-future-destructive-filesystem-command]

# Tech tracking
tech-stack:
  added: []  # no new crate — regex (1.12.4) already in Cargo.toml (T-03-SC, vetted [OK] rust-lang official, no install-time checkpoint); reuses core::output format_row/RowStatus + core::fs::is_hidden
  patterns:
    - "Pure I/O-free pre-flight detector: preflight(&[Rename], &[existing]) -> Vec<Conflict> implementing all four D-18 rules (case-folded per-dir occupied set seeded from on-disk names NOT renamed away, target-vs-target + target-vs-existing collision, cycle/swap = target equals another item's source, path-separator refusal) — disk reads happen in a thin preflight_plan wrapper, the logic itself is unit-tested with no I/O"
    - "Dry-run-DEFAULT / --force-execute inversion of flatten's plan→preview→execute split: identical PlanItem/format_row/arrow_col rendering, but the preview is the default and the mutation is opt-in (the correct UX for a destructive op, D-19)"
    - "Windows-safe rename: std::fs::rename SILENTLY OVERWRITES its destination (no create_new analog for moves) — the in-memory pre-flight is the ENTIRE safety story; every abort path is asserted to leave the directory byte-for-byte unchanged via a before/after snapshot"

key-files:
  created:
    - "src/commands/bulk_rename/mod.rs"
    - "tests/bulk_rename.rs"
  modified:
    - "src/cli.rs"
    - "src/main.rs"
    - "src/commands/mod.rs"

key-decisions:
  - "The collision detector is a PURE function preflight(&[Rename], &[String]) -> Vec<Conflict> with all four D-18 rules and ZERO I/O; a thin preflight_plan wrapper partitions the plan by parent dir (collision scope is per-directory, D-14) and read_dir-seeds each occupied set. This is what makes the load-bearing safety logic unit-testable like flatten::rename is — 9 unit tests cover two-sources-one-target, clobber-of-existing, renamed-away-exclusion, swap-cycle, case-only-is-not-a-conflict, both separator chars, clean-plan, full-Unicode fold, and the abort wording"
  - "Conflict is a 3-variant enum (Collision{target,sources} / Cycle{source,target} / Separator{source,target}) so the abort summary can explain EACH conflict precisely (`x.txt and y.txt both rename to dup.txt.`) and the preview can stamp the right inline reason (`[collision]` / `[cycle]` / `[separator]`) on the offending row. Separator-injecting renames are filtered out of the collision/cycle analysis (they already abort) so one bad target can't mask a second real conflict"
  - "Case-only rename (foo->Foo) is correct by construction: byte-exact no-ops are filtered to `(unchanged)` skips in build_plan BEFORE preflight runs, so any rename reaching the detector whose target folds to its OWN source key is — necessarily — a real case-only change, explicitly skipped in the cycle rule (target==own-source is NOT a cycle). Pitfall 5 closed without a special case"
  - "Rule-1 test-side fix: renm_dryrun_default_no_write and renm_force_capture_group must assert on the case-EXACT directory listing, not Path::exists(). NTFS is case-insensitive/case-PRESERVING, so after IMG_0042.jpg -> img_0042.jpg, Path::new(\"IMG_0042.jpg\").exists() is still TRUE — the implementation was correct; the original assertions were checking case-distinct existence on a case-insensitive FS (the same Pitfall-5 family the detector handles)"

patterns-established:
  - "First destructive command with a pure pre-flight safety gate: the preflight()/Conflict model (abort-all-before-any-mutation, every abort path snapshot-asserts the tree unchanged) is the template for any future mutating command"
  - "Dry-run-default / --force-execute inversion reusing flatten's render helpers VERBATIM (format_row, arrow_col, RowStatus::{Rename,Skip}, is_color_on-gated glyph) — no rendering code was re-derived"

requirements-completed: [RENM-01]

# Metrics
duration: 6min
completed: 2026-06-22
---

# Phase 3 Plan 05: bulk-rename Summary

**Live `box bulk-rename`: a regex bulk-rename whose preview is the DEFAULT (writes nothing) and whose mutation is opt-in via `--force`, guarded by an in-memory ABORT-ALL-BEFORE-ANY-RENAME pre-flight that detects collisions, cycles/swaps, and path-separator injection — the ONLY Windows-safe path because `std::fs::rename` silently overwrites its destination with no `create_new` backstop (D-14..D-19, RESEARCH Pitfall 4). The pre-flight is a pure, I/O-free `preflight() -> Vec<Conflict>` unit-tested for every rule, and every abort path is asserted to leave the directory byte-for-byte unchanged. This is the last Phase-3 slice — all 5 `not_implemented` arms are now gone and the phase is feature-complete.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-06-22
- **Completed:** 2026-06-22 (Wave 5)
- **Tasks:** 2 (Task 1 = Wave-0 RED tests + the pure pre-flight detector; Task 2 = command wiring / GREEN gate)
- **Files modified:** 5 (2 created — `src/commands/bulk_rename/mod.rs`, `tests/bulk_rename.rs`; 3 modified: `src/cli.rs`, `src/main.rs`, `src/commands/mod.rs`)

## Accomplishments
- `box bulk-rename` is fully live and Windows-safe. It compiles the user regex (`Regex::new`, a bad pattern is a clean `anyhow` error → exit 1, never a panic, FOUND-05), scopes candidates to the target dir's top-level files by default (`--recursive` opts into the shared `WalkDir::follow_links(false).min_depth(1).filter_entry(!is_hidden)` walk, D-14), skips directories and symlinks as `-` rows (D-15), and computes each new name with `regex.replace(full_base_name, replacement)` — FIRST match only, over the WHOLE base name incl. extension (D-16/D-17). A byte-exact no-op is an `(unchanged)` skip; a case-only change is a real rename.
- **The pre-flight check is the ENTIRE safety story.** A pure, I/O-free `preflight(&[Rename], &[existing]) -> Vec<Conflict>` implements all four D-18 rules: (1) a full-Unicode case-folded (`to_lowercase`, WR-01) occupied set per directory seeded from on-disk names NOT renamed away; (2) every target checked vs other planned targets AND that occupied set → any clobber is a `Collision`; (3) cycles/swaps (a target equal to another item's source) → `Cycle` (detect-and-abort, no two-phase temp pass in v1); (4) any target containing `/` or `\` → `Separator` (refused, mirrors flatten's `encode_no_separator`). ANY conflict aborts the whole batch (exit 1, nothing written) in BOTH dry-run and `--force`, printing `[collision]`/`[cycle]`/`[separator]` inline reasons on the offending rows plus the locked abort summary.
- **Dry-run is the DEFAULT, mutation is opt-in.** The default run prints the `~ old -> new` preview via the reused `format_row`/`arrow_col` helpers plus `Dry run: N to rename, M unchanged, K skipped. Re-run with --force to apply.` and writes nothing. `--force` executes `std::fs::rename(src, dst).with_context(...)` per file only AFTER a clean pre-flight, stopping on the first UNEXPECTED I/O error (a predictable collision already aborted). No `+`/Copy glyph — only `~` and `-`.
- Verified manually (PS-equivalent): `bulk-rename ./photos "IMG_(\d+)" "img_$1"` gives the exact CONTEXT.md preview (rename rows, `(unchanged)`, `(skipped: directory)`, the dry-run summary); a collision case prints `~ x.txt -> dup.txt [collision]` rows and aborts with `Aborted: 1 conflict detected. x.txt and y.txt both rename to dup.txt. No files were renamed.` to stderr, exit 1.
- Wired the LAST Phase-3 stub: `cli.rs` `BulkRename` now carries `BulkRenameArgs` (keeping `#[command(name = "bulk-rename")]`), `main.rs` dispatches `Commands::BulkRename(args) => args.run()`, and `pub mod bulk_rename;` is registered — the `not_implemented("bulk-rename")` arm is gone. All 5 Phase-3 `not_implemented` arms are now removed; the `not_implemented` import remains for the 8 Phase-4/5 stubs.

## Task Commits

Each task was committed atomically:

1. **Task 1: Wave-0 bulk-rename tests + the pure pre-flight detector** — `08c29db` (test) — the RED gate (7 integration tests fail against the unit `BulkRename` stub which takes no positional args → "unexpected argument", exit 2; 9 detector unit tests green via `cargo test --bin box bulk_rename`)
2. **Task 2: bulk-rename command wiring + Windows case-insensitive test fix** — `64e6236` (feat) — the GREEN gate (all 7 integration tests + 96 unit tests + clippy `-D warnings` + fmt `--check` clean)

**Plan metadata:** (final docs commit — this SUMMARY + STATE + ROADMAP + REQUIREMENTS)

_TDD-style gate sequence in git log: `test(03-05)` (RED) → `feat(03-05)` (GREEN). No REFACTOR commit needed — the GREEN implementation was already clippy `-D warnings` + `fmt --check` clean; the only `cargo fmt` change reflowed long lines with no logic change._

## Files Created/Modified
- `src/commands/bulk_rename/mod.rs` (created, ~600 lines) — `BulkRenameArgs` (positional `dir`/`pattern`/`replacement` + `--force`/`--recursive`, with `--help` documenting the full-base-name match and the `${1}abc` vs `$1abc` foot-gun) + `RunCommand` impl; the pure `preflight()` detector + `Conflict` enum (Collision/Cycle/Separator) + `Rename` struct; `build_plan` (scope walk, files-only, no-op/`(unchanged)` + dir/symlink `-` skips, first-match regex replace), `preflight_plan` (per-dir partition + read_dir occupied seed), `abort_summary`/`conflict_reason`/`print_plan*`/`arrow_col`/`tally` render helpers; 9 co-located unit tests for every D-18 rule + the abort wording
- `tests/bulk_rename.rs` (created) — 7 RENM-01 integration tests: `renm_dryrun_default_no_write`, `renm_force_capture_group`, `renm_collision_aborts`, `renm_cycle_aborts`, `renm_case_only_ok`, `renm_separator_refused`, `renm_skips_and_first_match`; `NO_COLOR=1`; every abort path snapshots the directory before/after and asserts byte-for-byte unchanged; the swap-cycle fixture uses `(.)(.)(\.txt)` -> `$2$1$3` for a genuine `ab.txt`<->`ba.txt` swap from one pattern
- `src/cli.rs` (modified) — `BulkRename` variant now carries `BulkRenameArgs` (was a unit stub), keeps `#[command(name = "bulk-rename")]`
- `src/main.rs` (modified) — `Commands::BulkRename(args) => args.run()` (removed the `not_implemented("bulk-rename")` arm — the last Phase-3 stub; the import stays for Phase-4/5 stubs)
- `src/commands/mod.rs` (modified) — registered `pub mod bulk_rename;` (alphabetical, after `base64`)

## Decisions Made
- **The detector is a pure function, disk I/O lives in a thin wrapper.** `preflight(&[Rename], &[String]) -> Vec<Conflict>` has zero I/O and implements all four D-18 rules; `preflight_plan` partitions the plan by parent directory (collision scope is per-directory, D-14, so two files in different dirs may both legitimately become `img_1.jpg`) and `read_dir`-seeds each directory's occupied set. This is what makes the load-bearing safety logic unit-testable exactly like `flatten::rename` is tested — 9 unit tests, no terminal, no filesystem.
- **`Conflict` is a 3-variant enum so every conflict is explained precisely.** Each variant carries the names needed for the locked abort summary and the inline preview reason. Separator-injecting renames are filtered out of the collision/cycle analysis (they already abort on their own) so one bad target can never mask a second real conflict, and a deterministic sort on contested keys keeps the abort message stable.
- **Case-only rename is correct by construction, not by special case.** Byte-exact no-ops are filtered to `(unchanged)` skips in `build_plan` BEFORE the detector runs, so any rename reaching `preflight` whose target folds to its own source key is necessarily a real case-only change — the cycle rule explicitly skips `target == own-source`, and the renamed-away exclusion means the source no longer occupies its slot. Pitfall 5 (false self-collision) is closed without a bespoke branch.
- **Rule-1 test-side fix for Windows case-insensitivity.** `renm_dryrun_default_no_write` and `renm_force_capture_group` originally asserted `!Path::exists()` on the case-distinct name; NTFS is case-insensitive/case-PRESERVING, so after `IMG_0042.jpg` -> `img_0042.jpg` the path `IMG_0042.jpg` still "exists". The fix asserts on the case-EXACT directory listing (`fs::read_dir` + byte-exact name compare) — the implementation was already correct.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Windows case-insensitive `Path::exists()` in two integration tests**
- **Found during:** Task 2 (GREEN gate — `renm_dryrun_default_no_write` and `renm_force_capture_group` failed)
- **Issue:** Both tests asserted file presence/absence with `Path::exists()` on case-distinct names (e.g. `!IMG_0042.jpg.exists()` after a rename to `img_0042.jpg`). NTFS is case-insensitive but case-preserving, so `Path::exists()` matched the lowercased file against the uppercased query and returned `true` — a false negative on the assertion. The command implementation was correct (the on-disk casing actually changed); the test was checking the wrong invariant on a case-insensitive filesystem.
- **Fix:** Added a `listed_names(dir)` helper that reads the directory and compares the LISTED names byte-exactly, then re-pointed both assertions to it (observing the stored casing). This is the same Pitfall-5 case-insensitivity family the detector itself handles.
- **Files modified:** `tests/bulk_rename.rs`
- **Commit:** `64e6236`

### Other notes (planned paths, NOT deviations)
- **The full `RunCommand` impl landed in the Task-1 commit, not Task-2.** The module file holds both the pure detector and the command; rather than stub the `run()` body in Task 1 and complete it in Task 2 (which would have meant editing the same function twice), the whole module was authored in Task 1 and the GREEN gate in Task 2 is purely the `cli.rs`/`main.rs` wiring that makes the integration tests reach the live command. Task 1's done-criterion ("the pure pre-flight detector is implemented and green at the unit level") is fully met; the integration tests are RED at Task 1 (against the unit stub) and GREEN at Task 2 (after wiring) exactly as planned. This keeps each commit a coherent, building unit.
- **Verify command `cargo test --lib bulk_rename` does not work on this binary-only crate** (carried-over note from 03-01..03-04): the detector unit tests run via `cargo test --bin box bulk_rename` (9/9 green). `cargo build --tests` reports 0 `error[` lines as the Task-1 gate requires. Process-only substitution, no code change.
- **The cycle test drives a genuine swap from ONE pattern.** Files `ab.txt`/`ba.txt` with `^(.)(.)(\.txt)$` -> `$2$1$3` produce `ab.txt`->`ba.txt` AND `ba.txt`->`ab.txt` (each target equals the other's source), so the detector's cycle rule fires without also tripping the plain-collision rule — exercising the two abort paths independently.

## Issues Encountered
- The two case-insensitivity test failures above (Rule-1 test-side fix). No implementation bug; resolved within the fix-attempt budget on the first attempt. No other auto-fixes were required.

## User Setup Required
None — no external service configuration required. `box bulk-rename` works offline against the local filesystem and is dry-run-by-default (writes nothing without `--force`).

## Next Phase Readiness
- ROADMAP Phase-3 success criterion #5 is met: `box bulk-rename ./photos "(\d+)" "img_$1"` gives a dry-run preview by default; `--force` executes; collision detection aborts before any rename if a conflict is found.
- **Phase 3 is feature-complete.** All 5 requirements shipped (HASH-01, TREE-01, DU-01, DUPE-01, RENM-01); all 5 Phase-3 `not_implemented` arms are gone. The remaining 8 stubs are Phase-4 (lolcat/matrix/ascii/json) and Phase-5 (qr/clip/pomodoro/weather).
- The pure `preflight()`/`Conflict` pre-flight model (abort-all-before-any-mutation, every abort path snapshot-asserted) is available as the template for any future destructive command. No new dependency was added (`regex` 1.12.4 was already present, T-03-SC). No blockers introduced.

## TDD Gate Compliance
- RED gate: `08c29db` (`test(03-05): ...`) — `tests/bulk_rename.rs` committed failing against the unit `BulkRename` stub (7/7 red, exit 2); the pure detector's 9 unit tests are green at the unit level (the safety logic is locked before any disk-mutating wiring).
- GREEN gate: `64e6236` (`feat(03-05): ...`) — the `cli.rs`/`main.rs` wiring lands; all 7 integration + 9 unit tests green, full suite (96 unit + all integration) + clippy `-D warnings` + fmt `--check` clean.
- REFACTOR: not required (GREEN was already clippy `-D warnings` + `fmt --check` clean; the only `cargo fmt` change reflowed long lines with no logic change).

## Self-Check: PASSED

- FOUND: src/commands/bulk_rename/mod.rs
- FOUND: tests/bulk_rename.rs
- FOUND: .planning/phases/03-filesystem-power-tools/03-05-SUMMARY.md
- FOUND commit: 08c29db (Task 1, RED)
- FOUND commit: 64e6236 (Task 2, GREEN)

---
*Phase: 03-filesystem-power-tools*
*Completed: 2026-06-22*
