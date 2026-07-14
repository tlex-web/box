---
phase: 08-filesystem-depth
plan: 06
subsystem: cli
tags: [bulk-rename, backup, undo-manifest, destructive, fsync, sync_all, localappdata, applied-partition, recoverable, dry-run, force, abort-all-before-any, serde, snapshot-test, adversarial-review]

# Dependency graph
requires:
  - phase: 03-filesystem-power-tools
    provides: "v1 bulk-rename abort-all-before-any pre-flight (collision/cycle/separator detector) + std::fs::rename executor loop — the backstop --backup is additive to"
  - phase: 08-filesystem-depth
    provides: "08-03 RENM-V2-01 build_plan → {n}/--case transform → UNCHANGED preflight_plan pipeline (the manifest is a serde projection of that same build-once Plan); 08-04/08-05 dry-run-default → --force destructive template + snapshot_tree per-abort-path test discipline"
provides:
  - "box bulk-rename --backup: a JSON undo manifest written + fsync'd BEFORE the first rename, recoverable on a mid-batch error (RENM-V2-02)"
  - "build_manifest(plan): a pure, zero-drift serde projection of the pre-flight-cleared Plan — one BackupEntry {old, new, applied} per ItemKind::Rename file with ABSOLUTE paths (parent.join, to_string_lossy per D-4)"
  - "manifest written to %LOCALAPPDATA%\\box\\undo\\box-undo-<unix_millis>.json (OUTSIDE the renamed tree so --recursive never re-walks it and it survives renaming the target dir); falls back to the target dir only if LOCALAPPDATA unset"
  - "write_manifest: serde_json::to_writer_pretty + File::sync_all() — the FULL all-applied:false manifest is durable on disk before any std::fs::rename, then each entry flips applied:true (rewrite+fsync) as its rename returns"
  - "applied-flag partition reconcilability: a mid-batch I/O error leaves a manifest whose applied flags exactly partition done (new exists/old gone) vs pending (old exists)"
  - "--backup is a no-op on dry-run (only meaningful with --force) and never changes the rename plan or the pre-flight; manifest path echoed to stderr"
  - "tests/bulk_rename_backup.rs: manifest-written + dry-run-noop + abort-writes-nothing + partition-recoverable (real mid-batch lock) tests"
affects: [09-dev-transform-visual-depth]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Durable-undo-manifest destructive model: write+fsync the FULL recovery record BEFORE the first mutation, flip per-item applied flags (rewrite+fsync) as each mutation returns — a mid-batch failure leaves an applied-partitioned, reconcilable manifest"
    - "Manifest-outside-the-tree: persistent recovery state under %LOCALAPPDATA% (LOCALAPPDATA, not APPDATA) so --recursive never re-walks it and renaming the target dir never orphans it (Pitfall 8)"
    - "Zero-drift serde projection: the manifest is built from the SAME pre-flight-cleared Plan the executor consumes (build_manifest is pure), so the recovery record can never disagree with what the rename loop does"

key-files:
  created:
    - tests/bulk_rename_backup.rs
  modified:
    - src/commands/bulk_rename/mod.rs

key-decisions:
  - "D-38 (08-06): RENM-V2-02 ships a JSON undo MANIFEST, not byte copies — a pure std::fs::rename (MoveFileExW) changes only the NAME, so the entire reversible state is the {old → new} map (D-22); copying file bytes would protect data that was never at risk. The manifest is the recovery record; --undo replay is explicitly Deferred (manual reverse documented in the module doc)."

patterns-established:
  - "build_manifest(plan: &Plan) -> Vec<BackupEntry>: maps every ItemKind::Rename item to {old: parent.join(old_name), new: parent.join(new_name), applied:false} with to_string_lossy() absolute paths — a pure projection consumed by the --force executor"
  - "write_manifest(path, manifest): serde_json::to_writer_pretty into a File then File::sync_all() — reused for the initial all-false write AND each per-rename applied:true rewrite, so the on-disk flags always partition done-vs-pending"
  - "Manifest write is wired strictly AFTER preflight_plan returns clean and BEFORE the first std::fs::rename, gated on (backup && force); the abort-all-before-any bail! path (unchanged) writes neither manifest nor renames"
  - "tests/bulk_rename_backup.rs points the command's LOCALAPPDATA env at a SECOND temp dir so the manifest location is observable + isolated; backup_partition_recoverable induces a REAL mid-batch failure (a locked target) and asserts the applied-flag partition against on-disk reality"

requirements-completed: [RENM-V2-02]

# Metrics
duration: ~20min
completed: 2026-06-28
---

# Phase 8 Plan 06: bulk-rename --backup Summary

**`box bulk-rename --backup` ships a recoverable destructive rename — a JSON undo manifest (a zero-drift serde projection of the pre-flight-cleared `Plan`) is written + `File::sync_all()`'d to `%LOCALAPPDATA%\box\undo\box-undo-<unix_millis>.json` (OUTSIDE the renamed tree) BEFORE the first `std::fs::rename`, then each entry flips `applied:true` (rewrite+fsync) as its rename returns — so a mid-batch I/O error leaves a manifest whose `applied` flags exactly partition done-vs-pending and the directory is reconcilable; `--backup` is a no-op on dry-run, `--force`-only, the path is echoed to stderr, and `--undo` replay is Deferred. (RENM-V2-02)**

## Performance

- **Duration:** ~20 min (implementation; the adversarial review gate is human time on top)
- **Started:** 2026-06-28
- **Completed:** 2026-06-28
- **Tasks:** 3 (2 code + 1 blocking adversarial review gate)
- **Files modified:** 2

## Accomplishments
- **RENM-V2-02:** `bulk-rename --backup` is the recoverability companion to 08-03's `--case`/`{n}` depth. New `backup: bool` (`#[arg(long)]`) flag on `BulkRenameArgs`. `--backup` is orthogonal to and only meaningful with `--force`: on a dry-run it is a clean no-op (nothing to back up), and it never alters the rename plan or the unchanged pre-flight.
- **Manifest = a serde projection, not byte copies (D-38 / D-22):** a pure `build_manifest(plan: &Plan) -> Vec<BackupEntry>` maps every `ItemKind::Rename` item to a `BackupEntry { old, new, applied:false }` with ABSOLUTE paths (`parent.join(old_name)` / `parent.join(new_name)`, `to_string_lossy()` per D-4). A pure `std::fs::rename` (`MoveFileExW`) changes only the NAME, so the `{old → new}` map IS the entire reversible state — copying file bytes would protect data that was never at risk. The entries are wrapped in a `BackupManifest { id, dir, entries }`.
- **Durable BEFORE the first rename:** the manifest is written via `write_manifest` (`serde_json::to_writer_pretty` + `File::sync_all()`) strictly AFTER `preflight_plan` returns clean and BEFORE the first `std::fs::rename`, with the FULL set of entries all `applied:false`. `sync_all()` forces the bytes to disk so the recovery record survives a crash mid-batch. The manifest path is echoed to stderr.
- **applied-flag partition reconcilability:** inside the `--force` rename loop, after each `std::fs::rename(...)` returns Ok, that entry's `applied` flips to `true` and the manifest is rewritten + `sync_all()`'d. So at any instant a mid-batch I/O error leaves a manifest whose `applied` flags EXACTLY partition done-vs-pending: every `applied:true` entry's `new` name exists and its `old` is gone; every `applied:false` entry's `old` still exists. The directory is reconcilable from the manifest alone.
- **Manifest lives OUTSIDE the renamed tree (Pitfall 8):** the path is `%LOCALAPPDATA%\box\undo\box-undo-<unix_millis>.json` via `std::env::var_os("LOCALAPPDATA")` (`LOCALAPPDATA`, NOT `APPDATA`) — `create_dir_all`'d, with a sortable `<id>` (Assumption A5). Outside the tree so `--recursive` never re-walks it and renaming the target dir never orphans it; falls back to the target dir only if `LOCALAPPDATA` is unset.
- **Abort-all-before-any unchanged:** the load-bearing collision/cycle/separator pre-flight `bail!` path is byte-for-byte untouched — an aborted run writes NEITHER the manifest NOR any rename (the manifest write is strictly after a clean pre-flight). Non-`--backup` `bulk-rename` (incl. 08-03's `--case`/`{n}` and the pre-flight) output is byte-identical to 08-03.
- **tests/bulk_rename_backup.rs:** 4 tests that point the command's `LOCALAPPDATA` at a SECOND temp dir so the manifest location is observable + isolated — `backup_manifest_written` (manifest under `box/undo/`, one `{old,new,applied}` per rename, absolute paths, all `applied:true` after success, path echoed to stderr), `backup_dryrun_noop` (no manifest, no rename without `--force`), `backup_only_with_force_writes_nothing_on_abort` (pre-flight collision → no manifest, no rename, snapshot unchanged), and `backup_partition_recoverable` (a REAL mid-batch failure via a locked target → the executor exits 1 and the surviving manifest's `applied` flags partition done/pending against on-disk reality).

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold tests/bulk_rename_backup.rs — manifest + dry-run-noop + partition-recoverable (RED)** - `b9d3693` (test)
2. **Task 2: bulk-rename --backup undo manifest (RENM-V2-02)** - `cf7ccf9` (feat)
3. **Task 3: Adversarial code-review gate** - no commit (blocking human-verify checkpoint; reviewed and **approved**)

**Deferred-items log** (pre-existing repo-wide `cargo fmt` drift, NOT introduced by this plan) - `3136ff1` (docs)

**Plan metadata:** this finalization commit (docs: complete plan)

_TDD note: Task 1 landed the failing manifest-written / dry-run-noop / abort-writes-nothing / partition-recoverable tests (RED); Task 2 is the GREEN implementation. No refactor commit was needed._

## Files Created/Modified
- `tests/bulk_rename_backup.rs` (created, 324 lines) - `bulk_rename` runner + `LOCALAPPDATA`-isolating helpers + 4 tests (manifest-written, dry-run-noop, abort-writes-nothing, partition-recoverable via a real locked-target mid-batch failure)
- `src/commands/bulk_rename/mod.rs` (modified, +145 lines → 1389) - `backup: bool` arg on `BulkRenameArgs`; `BackupEntry { old, new, applied }` + `BackupManifest { id, dir, entries }` serde structs; pure `build_manifest(plan)` projection; `write_manifest` (`to_writer_pretty` + `sync_all`); `--backup` wired into the `--force` executor (manifest written+fsync'd after a clean `preflight_plan` and before the first `std::fs::rename`, each entry flipped `applied:true`+rewrite+fsync per rename, path echoed to stderr); module doc updated with the D-22/D-38 manifest model, the LOCALAPPDATA location, the fsync-before-first-rename ordering, the applied-partition reconcilability, and the Deferred `--undo`

## Decisions Made

- **D-38 — JSON undo MANIFEST, not byte copies (D-22 applied to RENM-V2-02):** a pure `std::fs::rename` changes only the NAME, so the `{old → new}` map is the complete reversible state. `--backup` writes that map (a serde projection of the same `Plan` the executor consumes) rather than copying file bytes — copying would protect data that was never at risk and double disk for no recoverability gain. An automated `--undo` replay subcommand is explicitly Deferred (RENM-V2-02 needs only the backup written + the dir recoverable); the manual reverse is documented in the module doc comment. Reviewed and approved at the adversarial code-review gate.

## Deviations from Plan

None - plan executed exactly as written. (The plan's Task 1 `<action>` allowed either a real mid-batch I/O error OR a structural write-order assertion for `backup_partition_recoverable`; a REAL mid-batch failure via a locked target was used, the stronger of the two sanctioned options — no behavioral deviation.)

## Issues Encountered

None beyond the pre-existing, out-of-scope repo-wide `cargo fmt --check` drift discovered during this plan (see below). The RED tests compiled and failed as designed; the GREEN implementation passed all 4 plus the read-only regression and the full suite.

### Pre-existing repo-wide `cargo fmt` drift (deferred, out of scope)

`cargo fmt --check` reports formatting diffs across many committed files that plan 08-06 never touched (e.g. `src/commands/{du,dupes,flatten,tree}/mod.rs` and most `tests/*.rs`) — pre-existing in committed code (clean `git status` at 08-06 start, unmodified by this plan), most likely a rustfmt-version difference vs. when 08-01..08-05 were committed. This was logged to `.planning/phases/08-filesystem-depth/deferred-items.md` (commit `3136ff1`) per the executor scope-boundary rule and NOT fixed in-plan. The 08-06 verification gate is `cargo test` + `cargo clippy --all-targets -- -D warnings` (both clean); `cargo fmt --check` is not part of the plan's gate. The two files 08-06 authored ARE fmt-clean. **Recommended follow-up:** a dedicated `style: cargo fmt` sweep commit at the repo root outside any feature plan.

## Adversarial Code-Review Gate (Task 3)

The mandatory blocking human-verify gate (the v1 Phase-3 destructive-command review, applied to `bulk-rename --backup`) was satisfied and **approved**. Checklist confirmed:
1. `--backup` is a no-op without `--force` (no manifest, nothing renamed) and never alters the rename plan or the pre-flight.
2. The FULL manifest (all `applied:false`) is written and `sync_all()`'d BEFORE the first `std::fs::rename` — the manifest write is strictly after a clean `preflight_plan`.
3. The manifest path is `%LOCALAPPDATA%\box\undo\<id>.json` (OUTSIDE the renamed tree) so `--recursive` never re-walks it and renaming the target dir does not orphan it; the stderr echo is present.
4. Each entry flips `applied:true` (rewrite+fsync) as its rename returns, so a mid-batch error yields an `applied`-partitioned, reconcilable manifest.
5. The pre-flight abort path writes NEITHER the manifest NOR any rename (abort-all-before-any unchanged); the snapshot-the-tree-unchanged property holds on the abort path.
6. All `tests/bulk_rename_backup` tests are GREEN.

The adversarial look (a manifest written inside the renamed tree; a rename running before the manifest is fsync'd; an `applied` flag not reflecting on-disk reality after an error) found no data-loss surface — the implementation matches the plan with no behavioral deviation.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- **Phase 8 (Filesystem Depth) implementation is complete** — all 10 requirements done (HASH-V2-02, FLAT-V2-01/02, DUPE-V2-01/02, RENM-V2-01/02, TREE-V2-01, DU-V2-01/02). All three destructive Wave-2 plans (08-04 flatten --move, 08-05 dupes --delete, 08-06 bulk-rename --backup) shipped with their mandatory adversarial code-review gates approved. **The phase itself is NOT marked complete here — the orchestrator owns phase verification + phase.complete.**
- Verification state at finalization: `cargo test --test bulk_rename_backup` 4/4 + `cargo test --test bulk_rename` 15/15 green; full `cargo test` green; `cargo clippy --all-targets -- -D warnings` clean; working tree clean.
- One follow-up carried forward (out of scope): a `style: cargo fmt` sweep to clear the pre-existing repo-wide formatting drift logged in `deferred-items.md`.
- Next milestone work: **Phase 9 (Dev-Transform & Visual Depth)** — UUID/EPOC/COLR/JSON/PASS + visual (LOL/MTRX/QR/ASCI).

## Self-Check: PASSED

Both declared files exist (`tests/bulk_rename_backup.rs` 324 lines, `src/commands/bulk_rename/mod.rs` 1389 lines — both above the plan's `min_lines` of 110 / 1080). Both task commits present in git history (`b9d3693` test, `cf7ccf9` feat) plus the deferred-items log (`3136ff1`). Source contains `LOCALAPPDATA`, `sync_all`, `BackupEntry { ... applied: bool }`, `build_manifest`, and the manifest write occurs after `preflight_plan` and before the first `std::fs::rename`. All 4 `bulk_rename_backup` tests + 15 `bulk_rename` tests reported green; full suite + clippy clean per the verified pre-finalization state.

---
*Phase: 08-filesystem-depth*
*Completed: 2026-06-28*
