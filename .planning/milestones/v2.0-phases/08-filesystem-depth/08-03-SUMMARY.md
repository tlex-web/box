---
phase: 08-filesystem-depth
plan: 03
subsystem: filesystem
tags: [dupes, bulk-rename, blake3, hardlink, win32, GetFileInformationByHandle, clap-valueenum, rust]

# Dependency graph
requires:
  - phase: 07-spine-rollout
    provides: "dupes {size,paths} + bulk-rename {results,count,dry_run} --json structs (no-drift fork)"
  - phase: 08-filesystem-depth (08-01)
    provides: "windows 0.61 + indicatif deps in Cargo.toml; per-module localized Win32 FFI pattern"
provides:
  - "dupes size → partial(16 KiB BLAKE3) → full cascade (DUPE-V2-01)"
  - "dupes hardlink-aware wasted space via stable Win32 GetFileInformationByHandle (file_identity)"
  - "bulk-rename --case upper|lower|title (title on the stem only) (RENM-V2-01)"
  - "bulk-rename literal {n} numbering with --number-width/--start/--step over the SORTED plan, before the unchanged pre-flight"
affects: [08-05 dupes --delete (reuses file_identity + cascade), 11-meta-commands (completions must include the new flags)]

# Tech tracking
tech-stack:
  added: []  # all deps were added by 08-01; this plan touched no manifest
  patterns:
    - "Localized per-module Win32 FFI wrapper (file_identity) mirroring du's compressed_size"
    - "Pure post-pass over the built Plan BEFORE pre-flight (safety detector untouched)"
    - "Multi-stage hash cascade where the full hash stays the sole grouping arbiter"

key-files:
  created: []
  modified:
    - src/commands/dupes/mod.rs
    - src/commands/bulk_rename/mod.rs
    - tests/dupes.rs
    - tests/bulk_rename.rs

key-decisions:
  - "Made wasted_space itself hardlink-aware (calls file_identity, errors → own inode) so existing unit tests with synthetic paths keep matching (len-1)*size — no signature change, no dead code"
  - "{n}/--case run over ALL regular files incl. no-op (unchanged) skips, so --case applies even when re.replace was a no-op; directory/symlink skips excluded"
  - "{{n}} literal-brace escape via a NUL sentinel (A4)"
  - "title_case_stem = per-word capitalize on the stem, extension preserved verbatim"

patterns-established:
  - "size → partial → full BLAKE3 cascade: partial is a pure pre-filter; full hash is the only grouping decision"
  - "Win32 file identity (volume_serial, file_index) via GetFileInformationByHandle — stable path, NOT the nightly windows_by_handle std fields"

requirements-completed: [DUPE-V2-01, RENM-V2-01]

# Metrics
duration: ~30min
completed: 2026-06-28
---

# Phase 8 Plan 03: dupes multi-stage + hardlink collapse / bulk-rename --case + {n} Summary

**dupes now runs a size→partial(16 KiB)→full BLAKE3 cascade and collapses hardlink aliases via the stable Win32 GetFileInformationByHandle so a shared inode is never counted as wasted; bulk-rename gains --case upper|lower|title (title on the stem) and a literal {n} counter (--number-width/--start/--step) assigned over the sorted plan, before the unchanged abort-all pre-flight.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-06-28T00:35:00Z
- **Completed:** 2026-06-28T01:05:00Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments
- **DUPE-V2-01:** inserted a partial-hash re-bucketing stage between `collect_by_size` and the rayon full-hash; added a localized `file_identity` Win32 wrapper and made `wasted_space` hardlink-aware (`(distinct_inodes - 1) * size`). The `--json` `{results,count,wasted_bytes}` shape is unchanged; the human render still lists every alias path.
- **RENM-V2-01:** added `Case` (clap `ValueEnum`) + `--number-width`/`--start`/`--step`; a pure `apply_number_and_case` (literal `{n}` expand with `{{n}}` escape, then case fold; `title_case_stem` on the stem) wired as a post-pass over the SORTED plan BETWEEN `build_plan` and the UNCHANGED `preflight_plan`.
- Full suite green (171 bin-unit + all integration tests), `clippy --all-targets -D warnings` clean; default `box dupes`/`box bulk-rename` output preserved (existing trycmd/integration snapshots unchanged).

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold Wave-0 RED tests** - `d62fe65` (test)
2. **Task 2: dupes multi-stage hashing + hardlink-aware collapse (DUPE-V2-01)** - `36c5fba` (feat)
3. **Task 3: bulk-rename --case + {n} numbering over the sorted plan (RENM-V2-01)** - `1727876` (feat)

**Plan metadata:** see the final `docs(08-03)` commit.

_TDD note: Task 1 is the plan-level RED commit (`test`); Tasks 2 and 3 are the GREEN commits (`feat`)._

## Files Created/Modified
- `src/commands/dupes/mod.rs` - 3-stage cascade (`PARTIAL_BYTES`, `partial_hash`), `file_identity` (Win32/Unix/other), `distinct_inodes`, hardlink-aware `wasted_space`; module doc updated.
- `src/commands/bulk_rename/mod.rs` - `Case` enum + 4 args; `apply_number_and_case`, `expand_number`, `title_case_stem`/`title_case_words`, `digit_count`, `apply_number_and_case_to_plan`; unit tests.
- `tests/dupes.rs` - `multistage_splits`, `hardlink_not_wasted`.
- `tests/bulk_rename.rs` - `case_transforms`, `numbering_sorted_reproducible`.

## Decisions Made
- **`wasted_space` made hardlink-aware in place** rather than adding a new function: it calls `file_identity` per path and treats an identity error as the path's own distinct inode. This keeps the existing unit tests (whose synthetic `f0`/`a1` paths do not exist on disk) matching the old `(len-1)*size` arithmetic, avoids a signature change, and avoids a dead-code (`-D warnings`) failure.
- **`{n}`/`--case` apply to no-op (unchanged) files too**: the post-pass operates on every regular file (renames + `(unchanged)` skips), so `box bulk-rename . "(.*)" "$1" --case upper` still uppercases everything; the byte-exact no-op check is re-run after the transform and tallies are kept in sync.
- **`{{n}}` escape** implemented with a NUL sentinel (file names can never contain NUL).
- **`Case` enum is `pub`** to match the established codebase pattern (`hash::Algo`, `tree::SortMode`) and satisfy the `private_interfaces` lint on the public `BulkRenameArgs` field.

## Deviations from Plan

None - plan executed exactly as written (all auto-fixes below were within the planned scope and the deviation rules).

### Minor in-scope fixes

**1. [Rule 3 - Blocking] clippy `unnecessary_cast` on the Win32 handle**
- **Found during:** Task 2
- **Issue:** `HANDLE(file.as_raw_handle() as *mut core::ffi::c_void)` tripped `clippy::unnecessary_cast` (`RawHandle` is already `*mut c_void`).
- **Fix:** dropped the cast → `HANDLE(file.as_raw_handle())`.
- **Committed in:** `36c5fba`.

**2. [Rule 3 - Blocking] `private_interfaces` lint on `pub case: Option<Case>`**
- **Found during:** Task 3
- **Issue:** a public `BulkRenameArgs` field exposed the private `Case` enum.
- **Fix:** made `Case` `pub` (matching `hash::Algo`/`tree::SortMode`).
- **Committed in:** `1727876`.

---

**Total deviations:** 2 in-scope blocking fixes (both clippy `-D warnings` gates).
**Impact on plan:** none — both were required to keep the build clean; no scope change.

## Issues Encountered
- **`multistage_splits` is green-from-start, not RED.** Black-box grouping cannot distinguish a 2-stage (size→full) from a 3-stage (size→partial→full) pipeline: the full hash is the sole grouping arbiter, so the partial stage is a pure pre-filter optimization that provably never changes the output. The test therefore passes on the pre-Task-2 code and stands as a regression guard for the preserved behavior. The genuinely-new dupes behavior (`hardlink_not_wasted`) was correctly RED before Task 2 (counted 5000 B wasted) and GREEN after (0 B). `case_transforms` and `numbering_sorted_reproducible` were RED (unknown args) before Task 3 and GREEN after.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Wave 1 of Phase 8 is complete (08-01, 08-02, 08-03). Wave 2 destructive plans follow: 08-04 `flatten --move`, 08-05 `dupes --delete`, 08-06 `bulk-rename --backup`.
- **08-05 reuse:** `file_identity` + the sorted-group cascade are ready for `dupes --delete` keep-first + hardlink-safe deletion. The `(distinct_inodes - 1) * size` wasted figure is the deletion accounting basis.
- No blockers.

## Self-Check: PASSED

All 4 modified files exist; all 3 task commits (`d62fe65`, `36c5fba`, `1727876`) are present in history. Full `cargo test` green (171 unit + all integration); `cargo clippy --all-targets -- -D warnings` clean.

---
*Phase: 08-filesystem-depth*
*Completed: 2026-06-28*
