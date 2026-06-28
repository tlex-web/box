---
phase: 8
slug: filesystem-depth
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-27
validated: 2026-06-28
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Per-requirement test seams are detailed in `08-RESEARCH.md` § Validation Architecture;
> each PLAN.md embeds the concrete `<acceptance_criteria>` per task. This file is the
> sampling contract; the Per-Task Verification Map is reconciled during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) + `assert_fs` / `predicates` / `assert_cmd` (already dev-deps) |
| **Config file** | none — `[dev-dependencies]` in `Cargo.toml` |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30–90 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib` (plus the touched command's integration tests, e.g. `cargo test --test du`)
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green; `cargo build --release` succeeds
- **Max feedback latency:** ~90 seconds

---

## Per-Task Verification Map

> Filled per task during planning/execution. Source of truth for seams: `08-RESEARCH.md` § Validation Architecture.
> Destructive plans (08-04 `flatten --move`, 08-05 `dupes --delete`, 08-06 `bulk-rename --backup`)
> MUST carry a **snapshot-the-tree-unchanged** assertion for every abort path (Code-review gate).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Key Tests | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-----------|-------------|--------|
| 8-01 | 01 | 1 | HASH-V2-02, FLAT-V2-01 | T-8-01 / — | best-effort multi-file hash; `--json` no progress/ANSI leak | integration | `cargo test --test hash --test flatten` | `hash`: `multi_file_two_space`, `json_multifile_purity`, `partial_failure_exit1` · `flatten`: `extensions_filter`, `include_hidden` | ✅ | ✅ green (14+13) |
| 8-02 | 02 | 1 | TREE-V2-01, DU-V2-01, DU-V2-02 | — | gitignore/exclude filter parity human+JSON; on-disk size correct | integration | `cargo test --test tree --test du` | `tree`: `gitignore_nested`, `dirs_only_ignore_sort`, `gitignore_json_nodrift`, `default_output_unchanged` · `du`: `percentage_column`, `exclude_glob`, `json_no_ansi`, `on_disk_size` | ✅ | ✅ green (12+11) |
| 8-03 | 03 | 1 | DUPE-V2-01, RENM-V2-01 | — | hardlink-collapse never counts shared inode; `{n}` over sorted order | integration | `cargo test --test dupes --test bulk_rename` | `dupes`: `multistage_splits`, `hardlink_not_wasted` · `bulk_rename`: `case_transforms`, `numbering_sorted_reproducible` | ✅ | ✅ green (11+17) |
| 8-04 | 04 | 2 | FLAT-V2-02 | T-8-04 | dry-run default; abort leaves source tree byte-identical | integration | `cargo test --test flatten_move` | `move_dryrun_default_no_write`, `move_force_relocates`, `move_abort_containment_snapshot_unchanged`, `move_abort_midbatch_copy_error_snapshot_unchanged`, `move_json_plan_and_executed` | ✅ | ✅ green (5) |
| 8-05 | 05 | 2 | DUPE-V2-02 | T-8-05 | keep-≥1; hardlink-safe; abort-all-before-any | integration | `cargo test --test dupes_delete` | `delete_dryrun_default_no_write`, `delete_keep_first`, `delete_keeps_at_least_one_per_group`, `delete_hardlink_alias_never_deleted`, `delete_abort_preflight_snapshot_unchanged`, `delete_abort_dryrun_snapshot_unchanged`, `delete_json_plan_and_executed` | ✅ | ✅ green (7) |
| 8-06 | 06 | 2 | RENM-V2-02 | T-8-06 | manifest fsync'd before first rename; dir recoverable on abort | integration | `cargo test --test bulk_rename_backup` | `backup_manifest_written`, `backup_dryrun_noop`, `backup_only_with_force_writes_nothing_on_abort`, `backup_partition_recoverable`, `backup_manifest_absolute_paths_for_relative_dir`, `backup_manifest_atomic_no_tmp_leftover` | ✅ | ✅ green (6) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · ❌ W0 = test fixture/seam created in this plan's Wave-0 task*

> **Reconciled 2026-06-28:** all six Wave-0 test seams were implemented and are green. Full run of the nine Phase-8 suites = **96 integration tests, 0 failed** (`cargo test --test hash --test flatten --test flatten_move --test tree --test du --test dupes --test dupes_delete --test bulk_rename --test bulk_rename_backup`). Every destructive abort path (08-04/05/06) carries its `snapshot_tree` before==after assertion as mandated by the code-review gate.

---

## Wave 0 Requirements

- [x] Per-command integration test files (temp-dir fixtures via `assert_fs`) for each new flag's golden behavior — created within each plan's first task
- [x] `--json` parity assertions (piped output byte-identical-minus-ANSI; no progress on stderr under `--json`) reused across commands — `json_purity`/`json_no_ansi`/`json_multifile_purity`/`gitignore_json_nodrift`/`move_json_plan_and_executed`/`delete_json_plan_and_executed`
- [x] Snapshot-the-tree-unchanged helper for the three destructive abort paths (08-04/05/06) — `snapshot_tree`/`snapshot_into` in `flatten_move.rs`; equivalent before==after assertions in `dupes_delete.rs` and `bulk_rename_backup.rs`

*Existing `cargo test` harness + `assert_fs`/`predicates`/`assert_cmd` dev-deps cover the framework; no framework install needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `du --on-disk` compressed-size correctness on a real NTFS-compressed file | DU-V2-02 | `GetCompressedFileSizeW` returns allocation that depends on live NTFS compression state; hard to fixture deterministically in CI | Mark a file/dir compressed (`compact /c`), run `box du --on-disk` and compare to Explorer's "Size on disk" |
| stderr progress bar appears for large inputs only | HASH-V2-02, FLAT-V2-01 | indicatif timing/terminal behavior; assert presence/absence, not pixels | Run on a large input with a TTY; confirm bar on stderr and absent under `--json` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 90s (full nine-suite run completes in ~1s once built)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-06-28

---

## Validation Audit 2026-06-28

State A reconcile (Nyquist auditor not required — zero gaps). All six Wave-0 seams implemented and green; all 10 Phase-8 requirement IDs map to passing integration tests.

| Metric | Count |
|--------|-------|
| Requirements audited | 10 |
| COVERED (test exists, targets behavior, green) | 10 |
| PARTIAL | 0 |
| MISSING | 0 |
| Gaps found | 0 |
| Resolved (tests generated) | 0 |
| Escalated to manual-only | 0 |
| Pre-existing manual-only (unchanged) | 2 |

**Evidence:** `cargo test --test hash --test flatten --test flatten_move --test tree --test du --test dupes --test dupes_delete --test bulk_rename --test bulk_rename_backup` → 96 passed, 0 failed (hash 14, flatten 13, flatten_move 5, tree 12, du 11, dupes 11, dupes_delete 7, bulk_rename 17, bulk_rename_backup 6). Cross-checked against `08-VERIFICATION.md` (5/5 truths verified) and `08-SECURITY.md`.
