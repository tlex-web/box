---
phase: 08-filesystem-depth
verified: 2026-06-28T12:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run `box hash` with a list of 10+ files and confirm the stderr progress bar appears, then run the same command with `--json` and confirm the bar is absent and stdout contains only valid JSON"
    expected: "Progress bar on stderr for large batches; clean JSON document (no 0x1B) under --json"
    why_human: "The automated json_purity test confirms no ANSI on stdout under --json; the visual presence of the bar on stderr with a real large batch requires a live terminal session"
  - test: "On a file compressed with `compact /c`, run `box du --on-disk` and confirm the on-disk size is less than the apparent size and matches Explorer's 'Size on disk' field"
    expected: "On-disk bytes < apparent bytes; value matches what Explorer reports as 'Size on disk'"
    why_human: "The on_disk_size test uses a 5 KB non-resident non-compressed file (allocated >= apparent); confirming that compressed files report smaller-than-apparent values requires NTFS compression setup"
---

# Phase 8: Filesystem Depth Verification Report

**Phase Goal:** Land the deferred filesystem depth flags now that every command already has its `--json` output struct — including the three destructive flags, each isolated to its own plan with mandatory adversarial code review.
**Verified:** 2026-06-28
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `box hash a.bin b.bin` prints one coreutils `digest  filename` line per file + stderr progress for large inputs; `box flatten` honors --extensions/--separator/--include-hidden + progress; `box tree` honors .gitignore/--dirs-only/--ignore/sort-by-size; `box du` shows percentage column + color bands + --exclude + --on-disk NTFS size; `box dupes` collapses hardlinked paths; `box bulk-rename` supports upper/lower/title case + zero-padded {n} numbering. | ✓ VERIFIED | `multi_file_two_space`, `extensions_filter`, `include_hidden`, `gitignore_nested`, `dirs_only_ignore_sort`, `percentage_column`, `exclude_glob`, `on_disk_size`, `hardlink_not_wasted`, `case_transforms`, `numbering_sorted_reproducible` all pass. Source confirms `paths: Vec<String>` in HashArgs; `include_hidden`/`extensions`/`separator` in FlattenArgs; `gitignore`/`ignore`/`dirs_only`/`sort` in TreeArgs; `exclude`/`on_disk` in DuArgs; `GetFileInformationByHandle` in dupes; `apply_number_and_case` in bulk_rename. |
| 2 | DESTRUCTIVE flatten --move: dry-run default writes nothing; --force does copy→verify(dest exists+size matches)→delete-source; every abort path leaves the source tree byte-for-byte unchanged. | ✓ VERIFIED | `move_dryrun_default_no_write`, `move_force_relocates`, `move_abort_containment_snapshot_unchanged`, `move_abort_midbatch_copy_error_snapshot_unchanged` all pass. Source confirms two-phase design: Phase 1 copies+verifies every file with NO deletes; Phase 2 deletes every source (reached only after whole batch copied+verified). D-36 deviation from per-item to two-phase strengthens the invariant. Adversarial code review gate approved. |
| 3 | DESTRUCTIVE dupes --delete: dry-run default; --force removes duplicates keeping ≥1 per group, hardlink-safe, abort-all-before-any pre-flight. | ✓ VERIFIED | `delete_dryrun_default_no_write`, `delete_keep_first`, `delete_keeps_at_least_one_per_group`, `delete_hardlink_alias_never_deleted`, `delete_abort_preflight_snapshot_unchanged`, `delete_abort_dryrun_snapshot_unchanged` all pass. Source confirms paths[0] keep-first (structurally prevents last-copy deletion), `file_identity` alias exclusion, and `build_delete_plan` running to completion before any `remove_file`. Adversarial code review gate approved. |
| 4 | DESTRUCTIVE bulk-rename --backup: writes a recoverable pre-rename backup before the rename plan executes; a failed/aborted run leaves the directory recoverable. | ✓ VERIFIED | `backup_manifest_written`, `backup_dryrun_noop`, `backup_only_with_force_writes_nothing_on_abort`, `backup_partition_recoverable`, `backup_manifest_absolute_paths_for_relative_dir`, `backup_manifest_atomic_no_tmp_leftover` all pass. Source confirms `LOCALAPPDATA` path (outside the tree), `sync_all` via temp-file atomic-rename (WR-03 fix), manifest written+fsync'd after clean `preflight_plan` and before the first `fs::rename`, each entry flipped `applied:true` per rename. Adversarial code review gate approved. |
| 5 | Every depth flag respects the spine: --json still emits one clean JSON document with the new fields and no ANSI/progress contamination. | ✓ VERIFIED | `json_purity` tests pass across all 6 commands (hash, flatten, tree, du, dupes, bulk_rename). Source confirms `is_json_on()` fork is FIRST in every command; `ProgressDrawTarget::stderr()` is never constructed under `--json`; `is_color_on()` (forced false under `--json`) gates all ANSI. `json_multifile_purity`, `json_no_ansi`, `gitignore_json_nodrift`, `move_json_plan_and_executed`, `delete_json_plan_and_executed` confirm new-field JSON docs are parse-clean with no `0x1B`. |

**Score:** 5/5 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/commands/hash/mod.rs` | Multi-file hashing into {results,count} HashOutput | ✓ VERIFIED | 492 lines (min 380). Contains `paths: Vec<String>`, `indicatif`/`ProgressDrawTarget::stderr` guarded by `!is_json_on()`. `PROGRESS_FILE_THRESHOLD = 8`. |
| `src/commands/flatten/mod.rs` | --extensions/--separator/--include-hidden filters + --move destructive | ✓ VERIFIED | 731 lines (min 420/480). Contains `include_hidden`, extension filter inside `build_plan`, `move_` + `force` args, `run_move` two-phase executor. |
| `src/commands/flatten/rename.rs` | `encode_relative` parameterized by separator | ✓ VERIFIED | 374 lines. `encode_relative(rel: &Path, sep: &str)` confirmed. `encode_relative_honors_separator` unit test passes. `is_reserved_device_name` exported for bulk_rename. |
| `src/commands/tree/mod.rs` | gitignore matcher-as-filter + --ignore + --dirs-only + --sort size | ✓ VERIFIED | 654 lines (min 430). Contains `Gitignore`, `matched(`, `WalkOpts`/`WalkCtx`, `SortMode`. No `WalkBuilder` token. |
| `src/commands/du/mod.rs` | percentage column + band_color + --exclude globset + --on-disk Win32 | ✓ VERIFIED | 549 lines (min 380). Contains `GetCompressedFileSizeW`, `GlobSet`, `band_color`, `percent_str`. |
| `src/commands/dupes/mod.rs` | partial-hash stage + hardlink-index collapse + --delete | ✓ VERIFIED | 985 lines (min 430/520). Contains `GetFileInformationByHandle`, `PARTIAL_BYTES`, `file_identity`, `build_delete_plan`, `run_delete`. No `MetadataExt::file_index`. |
| `src/commands/bulk_rename/mod.rs` | --case + {n} numbering + --backup manifest | ✓ VERIFIED | 1521 lines (min 1020/1080). Contains `apply_number_and_case`, `BackupEntry { ... applied: bool }`, `LOCALAPPDATA`, `sync_all`, `write_manifest` (atomic temp-file), `is_reserved_device_name` import, `fold` with `trim_end_matches(['.', ' '])`. |
| `tests/flatten_move.rs` | Move relocation + snapshot-unchanged tests | ✓ VERIFIED | 338 lines (min 120). 5 tests, each abort test asserts `snapshot_tree` before == after. |
| `tests/dupes_delete.rs` | Keep-first + hardlink-safe + snapshot-unchanged tests | ✓ VERIFIED | 404 lines (min 120). 7 tests with real `hard_link` fixture. |
| `tests/bulk_rename_backup.rs` | Manifest-written + dry-run-noop + partition-recoverable tests | ✓ VERIFIED | 422 lines (min 110). 6 tests; partition test uses real locked-target mid-batch failure. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `hash/mod.rs` | `core::input::read_file_or_stdin` | per-path streaming read in multi-file loop | ✓ WIRED | Source: `read_file_or_stdin(t)` call in the compute loop |
| `hash/mod.rs` | `indicatif stderr progress` | `ProgressDrawTarget::stderr()` guarded by `!is_json_on()` | ✓ WIRED | Source: `PROGRESS_FILE_THRESHOLD = 8`; bar not constructed under `--json` |
| `flatten/mod.rs` | extension/hidden filter | filter inside `build_plan` (single source of truth) | ✓ WIRED | Source: `parse_extensions()` + `include_hidden` in `build_plan`; both human and JSON ride the same plan |
| `flatten/rename.rs` | separator parameter | `encode_relative(rel, sep)` call from `build_plan` | ✓ WIRED | Source confirms separator threaded into `encode_relative` |
| `flatten/mod.rs` | `core::fs::safe_copy` + verify + delete | two-phase `run_move`: Phase 1 copy+verify, Phase 2 delete | ✓ WIRED | Source: Phase 1 `safe_copy` + size-verify loop, Phase 2 `remove_file` loop after Phase 1 completion |
| `tree/mod.rs` | `ignore::gitignore::Gitignore` matcher | `matched(rel_path, is_dir)` in `read_children` (the shared chokepoint) | ✓ WIRED | Source: `is_ignored(entry, stack)` called inside `read_children`; both `render_dir` and `build_node` use the same path |
| `du/mod.rs` | `globset::GlobSet` exclude | `is_excluded(path, root, exclude)` in `collect_rows`/`dir_total` | ✓ WIRED | Source: `build_exclude()` + `is_excluded()` threaded through both functions |
| `du/mod.rs` | Win32 `GetCompressedFileSizeW` | `compressed_size(path)` wrapper, `#[cfg(windows)]` | ✓ WIRED | Source: `compressed_size` called in file-size path when `on_disk` is set |
| `dupes/mod.rs` | partial-hash re-bucketing | size→(size,partial)→full cascade between `collect_by_size` and rayon | ✓ WIRED | Source: `partial_hash()` stage, `PARTIAL_BYTES = 16 * 1024`; only surviving partial-buckets enter full hash |
| `dupes/mod.rs` | Win32 `GetFileInformationByHandle` | `file_identity(path)` → `(volume_serial, file_index)` | ✓ WIRED | Source: `file_identity` called in `wasted_space` and `build_delete_plan` |
| `dupes/mod.rs` | `preflight_plan` pattern / `build_delete_plan` | keep-first over sorted groups, file_identity alias skip, problems collected before any `remove_file` | ✓ WIRED | Source: `build_delete_plan` → pre-flight check → dry-run-default → `--force` `remove_file` |
| `bulk_rename/mod.rs` | `apply_number_and_case_to_plan` | post-pass over sorted plan BETWEEN `build_plan` and `preflight_plan` | ✓ WIRED | Source: `apply_number_and_case_to_plan(&mut plan, ...)` called before `preflight_plan` |
| `bulk_rename/mod.rs` | `LOCALAPPDATA` + `write_manifest` | manifest written+fsync'd AFTER clean `preflight_plan`, BEFORE first `fs::rename` | ✓ WIRED | Source: backup block positioned after `preflight_plan` returns and before the rename loop |
| `bulk_rename/mod.rs` | `flatten::rename::is_reserved_device_name` | shared reserved-name predicate in `injects` | ✓ WIRED | Source: `use crate::commands::flatten::rename::is_reserved_device_name;` at top of file; called in `injects` |
| `bulk_rename/mod.rs` | `fold()` trailing-dot normalization | CR-01 fix: `name.trim_end_matches(['.', ' ']).to_lowercase()` | ✓ WIRED | Source: `fn fold(name: &str) -> String { name.trim_end_matches(['.', ' ']).to_lowercase() }` |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `hash/mod.rs` | `rows: Vec<HashRow>` | `digest_reader()` over `read_file_or_stdin()` for each path | Yes — streams real file bytes through BLAKE3/SHA/MD5 | ✓ FLOWING |
| `flatten/mod.rs` | `plan: Plan` | `build_plan()` walks real src tree with WalkDir, applies extension/hidden filters | Yes — enumerates actual files from src_root | ✓ FLOWING |
| `tree/mod.rs` | `children: Vec<Child>` | `read_children()` reads real directory entries, applies gitignore/ignore/dirs-only/sort | Yes — real WalkDir over the target path | ✓ FLOWING |
| `du/mod.rs` | `rows: Vec<Row>` | `collect_rows()` + `dir_total()` with on-disk/exclude threading | Yes — real metadata + optional `GetCompressedFileSizeW` | ✓ FLOWING |
| `dupes/mod.rs` | `groups: Vec<DupeGroup>` | size→partial→full cascade + `file_identity` hardlink collapse | Yes — real file reads, Win32 handle queries | ✓ FLOWING |
| `bulk_rename/mod.rs` | `plan: Plan` | `build_plan()` WalkDir + regex replace + `{n}`/case transform | Yes — real directory walk + per-file name transformation | ✓ FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo test --test hash multi_file_two_space` | Checks two-space coreutils rows | 14/14 hash tests pass, 0 failed | ✓ PASS |
| `cargo test --test flatten extensions_filter include_hidden` | Extension + hidden filters work | 13/13 flatten tests pass, 0 failed | ✓ PASS |
| `cargo test --test flatten_move` | All 5 move/abort tests pass | 5/5 pass, 0 failed | ✓ PASS |
| `cargo test --test tree gitignore_nested dirs_only_ignore_sort` | Tree depth flags work | 12/12 tree tests pass, 0 failed | ✓ PASS |
| `cargo test --test du percentage_column exclude_glob on_disk_size` | Du depth flags work | 11/11 du tests pass, 0 failed | ✓ PASS |
| `cargo test --test dupes_delete` | All 7 delete tests pass | 7/7 pass, 0 failed | ✓ PASS |
| `cargo test --test bulk_rename_backup` | All 6 backup tests pass | 6/6 pass, 0 failed | ✓ PASS |
| `cargo test --test bulk_rename renm_trailing_dot_clobber_of_existing_aborts` | CR-01 regression test | PASS (trailing-dot clobber detected and aborted) | ✓ PASS |
| `cargo clippy --all-targets -- -D warnings` | Zero warnings/errors | Finished cleanly | ✓ PASS |
| `cargo test` (full suite) | No failures | 178 bin-unit + all integration suites pass; 0 failed | ✓ PASS |

---

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` probes exist for this phase. `cargo test` + `cargo clippy --all-targets -- -D warnings` are the declared phase gates, both confirmed clean above.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| HASH-V2-02 | 08-01 | `hash` accepts multiple file arguments + coreutils two-space rows + stderr progress | ✓ SATISFIED | `multi_file_two_space`, `json_multifile_purity`, `partial_failure_exit1` pass; `paths: Vec<String>` in source; REQUIREMENTS.md marked Complete |
| FLAT-V2-01 | 08-01 | `flatten` gains --extensions/--separator/--include-hidden + progress | ✓ SATISFIED | `extensions_filter`, `include_hidden` pass; args in FlattenArgs; `encode_relative(rel, sep)` confirmed; REQUIREMENTS.md marked Complete |
| FLAT-V2-02 | 08-04 | `flatten --move` relocates files: dry-run default, --force copy→verify→delete | ✓ SATISFIED | All 5 `flatten_move` tests pass; two-phase design confirmed in source; adversarial review approved; REQUIREMENTS.md marked Complete |
| DUPE-V2-01 | 08-03 | `dupes` multi-stage hashing (size→partial→full BLAKE3) + hardlink-aware | ✓ SATISFIED | `multistage_splits`, `hardlink_not_wasted` pass; `PARTIAL_BYTES`, `file_identity`, `GetFileInformationByHandle` in source; REQUIREMENTS.md marked Complete |
| DUPE-V2-02 | 08-05 | `dupes --delete`: dry-run default, keep-first, hardlink-safe, abort-all-before-any | ✓ SATISFIED | All 7 `dupes_delete` tests pass; adversarial review approved; REQUIREMENTS.md marked Complete |
| RENM-V2-01 | 08-03 | `bulk-rename` gains case transforms (upper/lower/title) + {n} sequential numbering | ✓ SATISFIED | `case_transforms`, `numbering_sorted_reproducible` pass; `apply_number_and_case` in source; REQUIREMENTS.md marked Complete |
| RENM-V2-02 | 08-06 | `bulk-rename --backup` writes recoverable JSON undo manifest + fsync'd before first rename | ✓ SATISFIED | All 6 `bulk_rename_backup` tests pass; adversarial review approved; REQUIREMENTS.md marked Complete |
| TREE-V2-01 | 08-02 | `tree` gains --gitignore, --dirs-only, --ignore, sort-by-size | ✓ SATISFIED | `gitignore_nested`, `dirs_only_ignore_sort`, `default_output_unchanged`, `gitignore_json_nodrift` pass; REQUIREMENTS.md marked Complete |
| DU-V2-01 | 08-02 | `du` gains in-line percentage column + color-coded size ranges + --exclude | ✓ SATISFIED | `percentage_column`, `exclude_glob`, `json_no_ansi` pass; REQUIREMENTS.md marked Complete |
| DU-V2-02 | 08-02 | `du --on-disk` reports allocated/compressed NTFS size via Win32 GetCompressedFileSizeW | ✓ SATISFIED | `on_disk_size` passes; `GetCompressedFileSizeW` in source; `on_disk: bool` in DuOutput; REQUIREMENTS.md marked Complete |

All 10 Phase 8 requirement IDs are marked Complete in REQUIREMENTS.md traceability table.

---

### Code-Review Findings Verification

All 6 findings from the mandatory adversarial code review (`08-REVIEW.md`) are confirmed fixed:

| Finding | Severity | Fix Commit | Verification |
|---------|----------|-----------|--------------|
| CR-01: trailing-dot/space targets bypass pre-flight → silent clobber | Critical | `10c6b94` | `fold()` confirmed: `name.trim_end_matches(['.', ' ']).to_lowercase()`; `trailing_dot_clobbers_existing` + `renm_trailing_dot_clobber_of_existing_aborts` tests pass |
| WR-01: flatten containment guard uses ASCII-only fold | Warning | `1e334cc` | `src_low.to_lowercase()` (not `to_ascii_lowercase()`) confirmed in source |
| WR-02: backup manifest records relative paths from relative dir arg | Warning | `807e5b3` | `normalize_path(&self.dir)` before planning confirmed; `backup_manifest_absolute_paths_for_relative_dir` test passes |
| WR-03: write_manifest truncates in place — a failed flip-write corrupts manifest | Warning | `32210e8` | Atomic temp-file-then-rename confirmed (`<id>.json.tmp`); `backup_manifest_atomic_no_tmp_leftover` test passes |
| WR-04: bulk-rename does not neutralize Windows reserved device-name targets | Warning | `10c6b94` | `is_reserved_device_name` imported from `flatten::rename`; `renm_reserved_device_name_target_aborts` test passes |
| IN-01: dead `let _ = is_color_on();` no-op | Info | `753530b` | Line removed; clippy clean confirms no dead code |

---

### Anti-Patterns Found

No blockers found.

| Pattern | Result |
|---------|--------|
| TBD/FIXME/XXX in modified source files | 0 matches in `src/commands/**/*.rs` |
| Stub returns (empty array/null/not-implemented) | 0 production stubs; "placeholder" appears only in a test comment |
| Hardcoded empty data flowing to user-visible output | Not found; all data sources confirmed real (WalkDir, file reads, Win32 queries) |

---

### Human Verification Required

### 1. Large-batch stderr progress bar (hash + flatten)

**Test:** Run `box hash` with 9+ files and observe stderr in a real PowerShell 7 terminal; then run the same command with `--json` and confirm stderr has no progress bar and stdout is a valid JSON document.

**Expected:** A file-count progress bar appears on stderr for batches above 8 files (hash) / 16 items (flatten), and is completely absent when `--json` is set. The JSON stdout has no `0x1B` escape byte.

**Why human:** The `json_purity` test confirms no ANSI on stdout under `--json`, and the source confirms `ProgressDrawTarget::stderr()` is only constructed when `!is_json_on() && len > THRESHOLD`. Visual confirmation of the bar appearing (not just its absence) requires a live terminal session. Low risk — the structural guarantee is already verified.

---

### 2. `du --on-disk` vs Explorer "Size on disk" for a NTFS-compressed file

**Test:** Run `compact /c somefile.txt` in PowerShell 7, then run `box du --on-disk` on the containing directory and compare the reported on-disk size for that file against what Explorer shows in Properties > "Size on disk".

**Expected:** `box du --on-disk` reports a value equal to or matching Explorer's "Size on disk" (the allocated/compressed NTFS cluster count). The `--on-disk` value should be less than the apparent size for a compressible file.

**Why human:** The `on_disk_size` integration test uses a 5 KB non-resident non-compressed file (allocated ≥ apparent due to cluster rounding). Confirming that `GetCompressedFileSizeW` returns the correct smaller value for a genuinely NTFS-compressed file requires a real `compact /c` setup and comparison with Explorer. The source and the Win32 API documentation both confirm the correct implementation path, so this is an empirical confirmation, not a correctness risk.

---

### Gaps Summary

No gaps. All 5 roadmap success criteria are verified in the codebase with passing automated tests. All 10 requirement IDs are marked Complete in REQUIREMENTS.md. All 6 code-review findings are confirmed fixed with committed regressions tests. `cargo test` (0 failures, all integration suites green) and `cargo clippy --all-targets -- -D warnings` (clean) are both confirmed passing. The two human verification items above are low-stakes empirical confirmations of behavior that is already structurally guaranteed by the code.

---

_Verified: 2026-06-28_
_Verifier: Claude (gsd-verifier)_
