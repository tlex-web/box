---
phase: 08-filesystem-depth
plan: 05
subsystem: cli
tags: [dupes, delete, destructive, keep-first, hardlink-safe, file-identity, dry-run, force, abort-all-before-any, snapshot-test, adversarial-review]

# Dependency graph
requires:
  - phase: 06-scriptable-core-foundation
    provides: "frozen --json spine (is_json_on/emit_json), {results,count,…} output template, D-09 abort-empty-stdout"
  - phase: 08-filesystem-depth
    provides: "08-03 DUPE-V2-01 size→partial→full cascade, sorted DupeGroup, file_identity (volume_serial,file_index) Win32 wrapper; 08-04 dry-run-default → --force destructive template + snapshot_tree per-abort-path test discipline"
provides:
  - "box dupes --delete: destructive deduplication, dry-run DEFAULT, --force to execute (DUPE-V2-02)"
  - "keep-first over the already-sorted group order — each group keeps paths[0], so a group can never lose its last copy"
  - "hardlink-safe deletion: a member sharing the kept member's (volume_serial,file_index) is excluded from deletion candidates (never delete an alias of the kept inode)"
  - "abort-all-before-any pre-flight: the WHOLE deletion plan is computed before any remove_file; ANY problem aborts the entire operation with nothing deleted"
  - "--delete --json carries a dry_run marker reflecting the mode; abort keeps stdout empty (D-09)"
  - "tests/dupes_delete.rs with snapshot-the-tree-unchanged assertions for the dry-run and every abort path"
affects: [08-06-bulk-rename-backup]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Plan-then-execute destructive model: build the full deletion plan (one file_identity read per member) BEFORE any mutation — abort-all-before-any"
    - "Keep-first over a deterministic sorted order as the keep-≥1-per-group guarantee (paths[0] is structurally always retained)"
    - "Hardlink-alias collapse by (volume_serial,file_index) reused as a deletion-candidate filter, not just a wasted-space accounting collapse"

key-files:
  created:
    - tests/dupes_delete.rs
  modified:
    - src/commands/dupes/mod.rs

key-decisions:
  - "D-37 (08-05): the abort-all-before-any pre-flight does I/O — one file_identity read per group member during plan construction (build_delete_plan), before any remove_file. This is an honest clarification of the plan's pure-pass wording, NOT a behavioral deviation: the whole plan (every keep/delete/alias decision) is still computed before a single deletion, so the abort-all-before-any guarantee holds byte-for-byte. Reviewed and approved at the adversarial code-review gate."

patterns-established:
  - "build_delete_plan: from the sorted Vec<DupeGroup>, kept = paths[0]; candidates = paths[1..] minus any sharing kept's file_identity; problems collected for the pre-flight"
  - "run_delete: build plan → pre-flight (bail! on ANY problem, plan printed only if !is_json_on()) → dry-run preview by default → remove_file each candidate only under --force with a clean pre-flight"
  - "Per-abort-path snapshot_tree(before)==snapshot_tree(after) test discipline carried from 08-04 into destructive dupes"

requirements-completed: [DUPE-V2-02]

# Metrics
duration: ~25min
completed: 2026-06-28
---

# Phase 8 Plan 05: dupes --delete Summary

**`box dupes --delete` ships safe destructive deduplication — dry-run by DEFAULT, `--force` to execute a keep-first delete over the already-sorted groups, hardlink-safe (an alias of the kept inode is never a deletion candidate), with an abort-all-before-any pre-flight that computes the whole plan before any `remove_file`; `--delete --json` carries the mode in `dry_run` and aborts keep stdout empty (D-09). (DUPE-V2-02)**

## Performance

- **Duration:** ~25 min (implementation; the adversarial review gate is human time on top)
- **Started:** 2026-06-28
- **Completed:** 2026-06-28
- **Tasks:** 3 (2 code + 1 blocking adversarial review gate)
- **Files modified:** 2

## Accomplishments
- **DUPE-V2-02:** `dupes --delete` is the destructive companion to 08-03's read-only `dupes`. New `delete: bool` and `force: bool` flags on `DupesArgs`. The destructive template (carried from 08-04) inverts the read-only posture: dry-run is the DEFAULT, so `box dupes <dir> --delete` previews which copies it would remove and writes NOTHING unless `--force` is given.
- **Keep-first over the sorted group order:** `build_delete_plan` takes each group's `paths[0]` as the kept member (the existing deterministic 08-03 `(hash,path)` sort gives a stable keep-first), and the deletion candidates are `paths[1..]`. Because `paths[0]` is structurally always retained, a group can never lose its last real copy — the keep-≥1-per-group guarantee is built into the selection, not enforced after the fact.
- **Hardlink-safe deletion:** the candidate set is filtered to EXCLUDE any path whose `file_identity` (08-03's `(volume_serial, file_index)` via `GetFileInformationByHandle`) equals the kept member's identity. An alias sharing the kept inode is never a deletion candidate — deleting it would free nothing and destroy a name (Pitfall 6, threat T-8-05-HL). Verified with a real `std::fs::hard_link` fixture.
- **Abort-all-before-any pre-flight:** `run_delete` builds the ENTIRE plan first, then runs a pre-flight that collects problems (defensively, a group with no member to keep). If ANY problem exists it `bail!`s (exit 1) with NOTHING deleted; the human plan is printed only `if !is_json_on()` so under `--json` the abort leaves stdout empty (D-09, threat T-8-05-ANSI). `remove_file` is reached only under `--force` AND a clean pre-flight, and `?`-propagates on the first error (threat T-8-05-PARTIAL).
- **`--json`:** a `DeleteOutput` carrying the per-group kept/deleted projection plus a `dry_run` marker, staying within the frozen `{results, count, …}` family. Dry-run emits the plan with `dry_run:true`; `--force` emits the executed result with `dry_run:false`. The `is_json_on()` fork is FIRST; stdout stays pure (no `0x1B`). The read-only `dupes --json` shape (without `--delete`) is byte-identical to 08-03.
- **tests/dupes_delete.rs:** a `snapshot_tree(dir)` helper (copied from `tests/dupes.rs`) plus 7 tests — `delete_dryrun_default_no_write`, `delete_keep_first`, `delete_keeps_at_least_one_per_group`, `delete_hardlink_alias_never_deleted`, `delete_abort_dryrun_snapshot_unchanged`, `delete_abort_preflight_snapshot_unchanged`, `delete_json_plan_and_executed`. Every abort/dry-run test asserts `snapshot_tree` before == after.

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold tests/dupes_delete.rs (RED)** - `967734e` (test)
2. **Task 2: dupes --delete = dry-run default → --force keep-first, hardlink-safe, abort-all-before-any (DUPE-V2-02)** - `2b12bf4` (feat)
3. **Task 3: Adversarial code-review gate** - no commit (blocking human-verify checkpoint; reviewed and **approved**)

**Plan metadata:** this finalization commit (docs: complete plan)

_TDD note: Task 1 landed the failing keep-first / hardlink-safety / per-abort-path snapshot tests; Task 2 is the GREEN implementation. No refactor commit was needed._

## Files Created/Modified
- `tests/dupes_delete.rs` (created, 404 lines) - `snapshot_tree` helper + 7 keep-first / hardlink-safety / per-abort-path snapshot-unchanged / JSON tests
- `src/commands/dupes/mod.rs` (modified, +461 lines → 985) - `--delete`/`--force` args on `DupesArgs`; `build_delete_plan` (keep-first + `file_identity` alias-skip); `run_delete` (pre-flight → dry-run-default → `--force` `remove_file`); `DeletePlan`/`DeleteProblem`/`DeleteOutput`; `emit_delete_json`; module doc updated (dupes is no longer strictly read-only when `--delete --force` is given; default and dry-run remain read-only)

## Decisions Made

- **D-37 — the pre-flight does I/O (one `file_identity` read per group member):** the plan's Task 2 `<action>` described the pre-flight as "a pure pass." In practice the hardlink-safe candidate filter requires reading each member's `(volume_serial, file_index)`, so plan construction (`build_delete_plan`) performs one identity read per member. This is an honest clarification, NOT a behavioral deviation — the WHOLE plan (every keep/delete/alias decision) is still computed before any `remove_file`, so the abort-all-before-any guarantee holds exactly. See Deviations below; reviewed and explicitly approved at the adversarial code-review gate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Clarification, not behavior change] Pre-flight performs I/O (file_identity read per member) during plan construction**
- **Found during:** Task 2 (implementation), confirmed and approved at Task 3 (adversarial review gate)
- **Issue:** The plan's `<action>` called the pre-flight "a pure pass that validates every group." Hardlink-safe candidate selection cannot be pure — it must read each member's `(volume_serial, file_index)` to know whether a candidate is an alias of the kept inode.
- **Fix:** `build_delete_plan` does the one `file_identity` read per member while assembling the plan, BEFORE the pre-flight check and BEFORE any deletion. The abort-all-before-any guarantee is preserved verbatim: the entire deletion plan is computed before a single `remove_file`, and ANY problem aborts the whole operation with nothing deleted.
- **Files modified:** src/commands/dupes/mod.rs
- **Verification:** `delete_abort_preflight_snapshot_unchanged` and `delete_abort_dryrun_snapshot_unchanged` prove the tree is byte-for-byte unchanged on every abort path; `delete_hardlink_alias_never_deleted` proves the alias of the kept inode is never removed; all 7 `dupes_delete` tests green; read-only `tests/dupes` (11) stays green (byte-identical to 08-03).
- **Committed in:** `2b12bf4` (Task 2 commit)

---

**Total deviations:** 1 (Rule 1 — a clarification of the pre-flight's I/O reality; the abort-all-before-any behavior is unchanged).
**Impact on plan:** No behavioral change and no scope expansion — the deviation only documents that the plan-build step reads identity. Explicitly reviewed and approved at the mandatory adversarial code-review gate.

## Issues Encountered
None beyond the pre-flight-I/O clarification documented above. The RED tests compiled and failed as designed; the GREEN implementation passed all 7 plus the read-only regression and the full suite.

## Adversarial Code-Review Gate (Task 3)

The mandatory blocking human-verify gate (the v1 Phase-3 destructive-command review, applied to `dupes --delete`) was satisfied and **approved**. Checklist confirmed:
1. `--delete` defaults to a dry-run that deletes NOTHING (preview only; tree unchanged).
2. Every group keeps ≥1 real copy — keep-first selects `paths[0]`, so a group can never have all members deleted.
3. A hardlink alias of the kept member is NEVER a deletion candidate (the `file_identity` collapse excludes it).
4. The pre-flight aborts the ENTIRE plan before any `remove_file` — no partial deletion is possible; the whole plan is computed first.
5. All snapshot-the-tree-unchanged tests GREEN for the dry-run and every abort path (`cargo test --test dupes_delete`).
6. `--delete --json` keeps stdout pure (one parseable value, no `0x1B`); `dry_run` reflects the mode; abort keeps stdout empty (D-09).

The adversarial look (a group whose last copy could be deleted; an alias counted as a deletable duplicate; a deletion running before a clean pre-flight) surfaced the pre-flight-I/O clarification (D-37), which was accepted as documented above. No data-loss surface was found.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- **08-06 (`bulk-rename --backup`)** is the last Wave-2 destructive plan. It can reuse the same patterns proven here and in 08-04: dry-run-default → `--force` gate, abort-all-before-any plan construction, and the `snapshot_tree(before)==snapshot_tree(after)` per-abort-path test discipline; plus the existing bulk-rename abort-all pre-flight as its backstop.
- All `dupes_delete` abort-path snapshot tests green; read-only `dupes` (without `--delete`) byte-identical to 08-03.
- Full `cargo test` green; `cargo clippy --all-targets -- -D warnings` clean; working tree clean.
- Phase 8 Wave 2 progress: 2 of 3 destructive plans done (08-04 ✅, 08-05 ✅; 08-06 remains).

## Self-Check: PASSED

Both declared files exist (`tests/dupes_delete.rs` 404 lines, `src/commands/dupes/mod.rs` 985 lines — both above the plan's `min_lines` of 120 / 520). Both task commits present in git history (`967734e` test, `2b12bf4` feat). All 7 `dupes_delete` tests + 11 read-only `dupes` tests + `build_delete_plan` unit tests reported green; full suite + clippy clean per the verified pre-finalization state.

---
*Phase: 08-filesystem-depth*
*Completed: 2026-06-28*
