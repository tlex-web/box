---
phase: 08-filesystem-depth
plan: 04
subsystem: cli
tags: [flatten, move, destructive, safe-copy, dry-run, force, two-phase, snapshot-test, adversarial-review]

# Dependency graph
requires:
  - phase: 06-scriptable-core-foundation
    provides: "frozen --json spine (is_json_on/emit_json), {results,count} FlattenOutput template"
  - phase: 08-filesystem-depth
    provides: "08-01 filtered build_plan + FlattenOutput/flatten_rows + safe_copy (create-new) + containment guard + collision-rename (encode_relative)"
provides:
  - "box flatten --move: destructive relocation, dry-run DEFAULT, --force to execute (FLAT-V2-02)"
  - "two-phase copy→verify ALL then delete ALL — the only ordering that keeps the source byte-for-byte unchanged on a mid-batch copy error"
  - "every abort path (containment refusal, dry-run, mid-batch copy error) provably leaves the source tree unchanged (snapshot_tree before==after)"
  - "empty source directories preserved (only files relocate)"
  - "--move --json reuses FlattenOutput with dry_run reflecting the mode"
  - "tests/flatten_move.rs with snapshot-the-tree-unchanged assertions per abort path (copy-me destructive test pattern for 08-05/08-06)"
affects: [08-05-dupes-delete, 08-06-bulk-rename-backup]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Destructive template: dry-run is the DEFAULT, --force opt-in to execute (inverse of copy mode's opt-in --dry-run)"
    - "Two-phase batch mutation: copy+verify EVERY item BEFORE deleting ANY — abort-leaves-source-unchanged invariant for batch relocations"
    - "snapshot_tree(dir) before==after assertion as the per-abort-path data-loss regression backstop"

key-files:
  created:
    - tests/flatten_move.rs
  modified:
    - src/commands/flatten/mod.rs

key-decisions:
  - "D-36 (08-04): --move executes in TWO phases (copy+verify ALL, then delete ALL) rather than the plan's per-item copy→verify→delete loop — the only ordering satisfying the 'mid-batch copy error leaves the source byte-for-byte unchanged' invariant. Reviewed and approved at the adversarial code-review gate."

patterns-established:
  - "Dry-run-default + --force destructive gate (run_move): preview writes nothing unless --force && !dry_run"
  - "Two-phase relocation: Phase 1 safe_copy(create-new)+size-verify every file; Phase 2 remove_file every source — reached only after the whole batch verified"
  - "Per-abort-path snapshot_tree(before)==snapshot_tree(after) test discipline for destructive commands"

requirements-completed: [FLAT-V2-02]

# Metrics
duration: ~10min
completed: 2026-06-28
---

# Phase 8 Plan 04: flatten --move Summary

**`box flatten --move` ships destructive relocation — dry-run by default, `--force` to execute a two-phase copy→verify-ALL then delete-ALL sequence that keeps the source tree byte-for-byte unchanged on every abort path; empty source dirs preserved; `--json` carries the mode in `dry_run`. (FLAT-V2-02)**

## Performance

- **Duration:** ~10 min (implementation; the adversarial review gate is human time on top)
- **Started:** 2026-06-28T01:27:38+02:00
- **Completed:** 2026-06-28T01:31:38+02:00
- **Tasks:** 3 (2 code + 1 blocking adversarial review gate)
- **Files modified:** 2

## Accomplishments
- **FLAT-V2-02:** `flatten --move` relocates files instead of copying. `--move` (field `move_`, A6 — `move` is a Rust keyword) selects relocation; `--force` gates execution. The destructive template inverts copy mode: dry-run is the DEFAULT, so `box flatten <src> <out> --move` previews the plan and writes nothing unless `--force` is given (an explicit `--dry-run` also forces a preview even alongside `--force`).
- **Shared pipeline reuse (no-drift):** `run_move` runs after the existing separator validation, containment guard, `occupied` seed, and `build_plan`, so every FLAT-V2-01 filter (`--extensions`/`--separator`/`--include-hidden`) applies identically — only the execution differs. Collision-renamed destinations and the create-new `safe_copy` backstop are reused verbatim.
- **Two-phase `--force`:** Phase 1 copies (`safe_copy`, create-new) and verifies (destination exists with a byte length equal to the source) EVERY file; Phase 2 deletes EVERY source, reached only after the whole batch copied+verified. A failed/short copy `?`-propagates in Phase 1 with zero sources deleted. Empty source directories are deliberately left in place (only files relocate).
- **`--json`:** reuses `FlattenOutput`/`flatten_rows` (no-drift) — dry-run emits the plan projection with `dry_run:true` and zero `copied`/`total_bytes`; `--force` emits the executed result with `dry_run:false` and the real relocation counts. `is_json_on()` fork is FIRST, stdout stays pure (no `0x1B`).
- **tests/flatten_move.rs:** a `snapshot_tree(dir)` helper (copied from `tests/dupes.rs`) plus 5 tests — `move_dryrun_default_no_write`, `move_force_relocates`, `move_abort_containment_snapshot_unchanged`, `move_abort_midbatch_copy_error_snapshot_unchanged`, `move_json_plan_and_executed`. Every abort-path test asserts `snapshot_tree` before == after.

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold tests/flatten_move.rs (RED)** - `f6703d4` (test)
2. **Task 2: flatten --move = dry-run default → --force copy→verify→delete (FLAT-V2-02)** - `9e557c7` (feat)
3. **Task 3: Adversarial code-review gate** - no commit (blocking human-verify checkpoint; reviewed and approved)

**Plan metadata:** this finalization commit (docs: complete plan)

_TDD note: Task 1 landed the failing relocation + per-abort-path snapshot tests; Task 2 is the GREEN implementation. No refactor commit was needed._

## Files Created/Modified
- `tests/flatten_move.rs` (created, 338 lines) - `snapshot_tree` helper + 5 relocation/abort-path tests, each abort test asserting the source tree is unchanged
- `src/commands/flatten/mod.rs` (modified, +164 lines → 726) - `--move`/`--force` args on `FlattenArgs`, the `run_move` two-phase executor, module doc documenting the destructive template

## Decisions Made

- **D-36 — Two-phase relocation (copy+verify ALL, then delete ALL):** The plan's Task 2 `<action>` described a *per-item* `copy → verify → delete` loop. Implemented instead as two distinct phases over the whole batch. See Deviations below — reviewed and explicitly approved at the adversarial code-review gate as the only ordering that satisfies the plan's own stated invariant.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Two-phase copy-all-then-delete-all instead of the planned per-item copy→verify→delete loop**
- **Found during:** Task 2 (implementation), confirmed and approved at Task 3 (adversarial review gate)
- **Issue:** The plan's `<action>` prose specified a per-item loop (`safe_copy` → verify → `remove_file` for each file in turn). That ordering is self-contradictory with the plan's own `must_haves` truth and threat T-8-04 ("every abort path — including a mid-batch copy error — leaves the source tree byte-for-byte unchanged"): with a per-item loop, a copy failure on item N would leave items 1..N-1 *already deleted*, so the source tree would NOT be unchanged on that abort path.
- **Fix:** `run_move` executes in two phases. Phase 1 copies (`safe_copy`, create-new) and size-verifies EVERY planned file with NO deletes; any error `?`-propagates here with zero sources deleted. Phase 2 deletes EVERY source, and is reached only once the whole batch has copied+verified. This is the only ordering under which a mid-batch copy error leaves the source byte-for-byte unchanged.
- **Trade-offs (documented, accepted at review):** peak disk usage doubles for the batch (all destinations exist before any source is freed); a rare Phase-2 `remove_file` error leaves a fully-copied destination tree with a partially-deleted source tree — recoverable, no data loss (every file still exists in at least one location). The Phase-1/Phase-2 TOCTOU window is the accepted T-8-04-TOCTOU disposition (single-process local CLI).
- **Files modified:** src/commands/flatten/mod.rs
- **Verification:** `move_abort_midbatch_copy_error_snapshot_unchanged` proves no source is deleted before a copy error; all 5 `flatten_move` tests green; copy-mode `tests/flatten` (13) stays green (byte-identical to 08-01).
- **Committed in:** `9e557c7` (Task 2 commit)

---

**Total deviations:** 1 (Rule 1 — corrected an ordering flaw in the planned algorithm against the plan's own abort-leaves-source-unchanged invariant).
**Impact on plan:** The deviation strengthens the data-loss guarantee the plan required; it does not expand scope. Explicitly reviewed and approved at the mandatory adversarial code-review gate.

## Issues Encountered
None beyond the ordering correction documented above. The RED tests compiled and failed as designed; the two-phase GREEN implementation passed all five plus the full suite on the first build.

## Adversarial Code-Review Gate (Task 3)

The mandatory blocking human-verify gate (the v1 Phase-3 destructive-command review, applied to `flatten`) was satisfied and **approved**. Checklist confirmed:
1. `--move` defaults to a dry-run that writes nothing (output dir stays empty, source unchanged).
2. Every delete is strictly AFTER a successful `safe_copy` + size-verify; no `remove_file` can run on a failed/short copy (the two-phase ordering makes this structural — Phase 2 is unreachable until Phase 1 finished for the whole batch).
3. All snapshot-the-tree-unchanged tests GREEN for every abort path (containment refusal, dry-run, mid-batch copy error).
4. Empty source directories left in place (only files relocate).
5. `--move --json` stdout pure (one parseable value, no `0x1B`); `dry_run` reflects the mode.

The adversarial look (cross-volume delete-before-confirm, verify→delete TOCTOU, containment-guard bypass under `--move`) raised the two-phase ordering decision (D-36), which was accepted as documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- **08-05 (`dupes --delete`)** and **08-06 (`bulk-rename --backup`)** can reuse the destructive template established here: dry-run-default → `--force` gate, two-phase abort-all-before-any mutation, and the `snapshot_tree(before)==snapshot_tree(after)` per-abort-path test discipline.
- All `flatten_move` abort-path snapshot tests green; copy mode (`box flatten` without `--move`) byte-identical to 08-01.
- Full `cargo test` green; `cargo clippy --all-targets -- -D warnings` clean; working tree clean.
- Phase 8 Wave 2 progress: 1 of 3 destructive plans done (08-04 ✅; 08-05 / 08-06 remain).

## Self-Check: PASSED

Both declared files exist (`tests/flatten_move.rs` 338 lines, `src/commands/flatten/mod.rs` 726 lines — both above the plan's `min_lines`). Both task commits present in git history (`f6703d4` test, `9e557c7` feat). All 5 `flatten_move` tests + 13 `flatten` copy-mode tests reported green; full suite + clippy clean per the verified pre-finalization state.

---
*Phase: 08-filesystem-depth*
*Completed: 2026-06-28*
