---
phase: 03-filesystem-power-tools
verified: 2026-06-23T00:00:00Z
status: passed
human_verified: 2026-06-22T23:47:40Z
human_verified_note: "User confirmed all 3 color-rendering items in PowerShell 7 (tree blue dirs, du cyan sizes, dupes yellow group headers) — approved."
score: 24/24 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run `box tree ./src` in a real PowerShell 7 terminal (not piped) and confirm directory names appear in blue+bold while file names and branch glyphs have no color"
    expected: "Directory names render with ANSI blue+bold; file names are plain; box-drawing glyphs (├── └── │) are visible and correct"
    why_human: "Color output requires a TTY; tests force NO_COLOR=1 so ANSI is always suppressed. This behavior cannot be asserted by the automated suite."
  - test: "Run `box du .` in a terminal and verify the size-value column appears with a cyan accent while directory names are plain"
    expected: "Size values are cyan-colored; the path/name column is uncolored; trailing `/` marks directories; output is biggest-first"
    why_human: "Color requires a TTY. Tests force NO_COLOR=1."
  - test: "Run `box dupes .` in a directory with some duplicates and verify the group header and wasted-space line appear with yellow accent"
    expected: "Group header (`X B each, N copies`) and the final wasted-space line are yellow-colored; file paths inside the group are plain"
    why_human: "Color requires a TTY. Tests force NO_COLOR=1."
---

# Phase 3: Filesystem Power Tools — Verification Report

**Phase Goal:** Users can hash files, explore disk usage visually, find duplicate files, and bulk-rename files with safe dry-run-first workflows.
**Verified:** 2026-06-23
**Status:** human_needed — all automated checks pass; 3 color/TTY behaviors require human confirmation
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

All 24 must-have truths from the 5 plan frontmasters verified against the codebase and confirmed by the live test suite (98 unit + integration tests, 0 failures).

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `box hash <file>` outputs SHA-256 in `<hash>  <filename>` format (two spaces) | VERIFIED | `src/commands/hash/mod.rs:164` `println!("{computed}  {label}")`. Integration test `hash_default_sha256` asserts format. |
| 2 | `--algo blake3/sha512/md5` switches algorithm to known-answer vector | VERIFIED | `digest_reader` enum-dispatch at `hash/mod.rs:119-126`. Tests `hash_algo_blake3`, `hash_algo_sha512_md5` pass known-answer vectors. |
| 3 | Piping bytes to `box hash` (no arg or `-`) hashes stdin, labeled `-` | VERIFIED | `read_file_or_stdin` branch 2 in `core/input.rs:187-190`. Test `hash_stdin_dash_label` passes. |
| 4 | `--verify` exits 0 on match, 1 on mismatch, 2 on unsupported length | VERIFIED | `hash/mod.rs:138-167`; `UnsupportedHashLength` downcast in `main.rs:100-105`. Tests `hash_verify_match_mismatch`, `hash_verify_bad_len_exit2` pass. |
| 5 | Explicit `--algo sha256 --verify <32-hex>` verifies as SHA-256, not MD5 (WR-01 fix) | VERIFIED | `algo` is `Option<Algo>`; `match self.algo { Some(a) => a, None => algo_from_len(...)? }` at `hash/mod.rs:140-148`. Test `hash_verify_explicit_algo_overrides_length_autodetect` passes. |
| 6 | `box tree ./src` renders box-drawing tree (├── └── │) with dirs-first, case-insensitive alpha sort | VERIFIED | Constants `TEE/ELL/PIPE/GAP` at `tree/mod.rs:42-45`. `sort_children` comparator at `tree/mod.rs:219-223`. Unit test `dirs_sort_before_files_then_alpha` passes. |
| 7 | `--sizes` shows per-file size column; directories show blank size | VERIFIED | `tree/mod.rs:148-156`: `Some(bytes) => format!("  {}", human_size(bytes))` for files, empty string for dirs. Test `tree_sizes_and_depth` passes. |
| 8 | `--depth N` caps displayed tree depth | VERIFIED | `render_dir` checks `depth > max` at `tree/mod.rs:128-131`. Test `tree_sizes_and_depth` passes. |
| 9 | Trailing `N directories, M files` summary appears after tree | VERIFIED | `tree/mod.rs:108-109` `println!("{} directories, {} files", ...)`. Test `tree_count_summary` passes. |
| 10 | Piping `box tree` yields plain text with no ANSI escapes | VERIFIED | `color_dir` gated on `is_color_on()` at `tree/mod.rs:229-234`. Test `tree_piped_no_ansi` asserts no `\x1b[`. |
| 11 | `box du` shows one row per immediate child, biggest-first, human-readable | VERIFIED | `collect_rows` + `sort_rows` at `du/mod.rs:145-203`. Test `du_biggest_first` passes. |
| 12 | Directories show recursive total + trailing `/`; files show own size | VERIFIED | `du/mod.rs:160-165`: dir uses `dir_total`, file uses `metadata().len()`. Trailing `/` at `du/mod.rs:126`. Test `du_biggest_first` passes. |
| 13 | `--top N` truncates shown rows; summary reflects FULL scan total | VERIFIED | `rows.truncate(top)` after capturing `total` at `du/mod.rs:110-112`. Test `du_top_and_total_summary` passes. |
| 14 | `--depth N` caps recursive total rollup | VERIFIED | `dir_total(entry.path(), depth)?` at `du/mod.rs:161`. Unit test `dir_total_sums_recursive_descendants` verifies depth-1 cap. |
| 15 | Summary `{X} of {Y} entries shown. {TOTAL} total.` always reflects full scan | VERIFIED | `du/mod.rs:133-136`. Test `du_top_and_total_summary` asserts full total. |
| 16 | `box dupes` groups identical files by content hash | VERIFIED | `group_duplicates` at `dupes/mod.rs:166-195`. Test `dupes_groups_identical` passes. |
| 17 | Same-size but different-content files are NOT grouped (size pre-filter then hash) | VERIFIED | `filter(|(_, paths)| paths.len() >= 2)` at `dupes/mod.rs:84-87`. Test `dupes_size_then_hash` passes. |
| 18 | Hidden entries skipped; no noise list applied | VERIFIED | `filter_entry(|e| !is_hidden(e))` at `dupes/mod.rs:122`. No noise-directory skip list anywhere in `dupes/mod.rs`. |
| 19 | Wasted-space summary reports redundant bytes | VERIFIED | `wasted_space` formula `(group_len - 1) * file_size` at `dupes/mod.rs:199-204`. Test `dupes_wasted_space_sorted` passes. |
| 20 | `dupes` is strictly read-only — no file deleted or modified | VERIFIED | No `std::fs::rename`/`remove`/`write` anywhere in `dupes/mod.rs`. Test `dupes_never_writes` snapshots mtimes before/after and asserts unchanged. |
| 21 | `box bulk-rename` is dry-run by DEFAULT; `--force` required to execute | VERIFIED | `if !self.force { print_plan(...); return Ok(()) }` at `bulk_rename/mod.rs:327-335`. Test `renm_dryrun_default_no_write` asserts no writes without `--force`. |
| 22 | Collision detection ABORTS the whole batch before any rename (in BOTH modes) | VERIFIED | `preflight_plan` called before any `rename` at `bulk_rename/mod.rs:312-323`; `bail!` on conflict. Tests `renm_collision_aborts`, `renm_cycle_aborts` assert `before == after`. |
| 23 | `..`/`.`/dots-only targets refused in pre-flight (CR-01 fix) | VERIFIED | `injects()` helper at `bulk_rename/mod.rs:285-291` checks `name == ".."`, `name == "."`, and `name.trim_matches(['.', ' ']).is_empty()`. Unit test `refuses_dot_and_dotdot_targets` covers `["..",".", "...", "  ", " . "]`. Integration test `renm_dotdot_target_aborts` passes in both dry-run and `--force`. |
| 24 | Case-only rename (`foo→Foo`) succeeds; path separators in target abort | VERIFIED | Exact-name compare for case-only at `bulk_rename/mod.rs:260-265`. `refuses_path_separators` unit test. Integration tests `renm_case_only_ok`, `renm_separator_refused` pass. |

**Score: 24/24 truths verified**

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/commands/hash/mod.rs` | HashArgs + Algo ValueEnum + streaming Hasher + verify logic + RunCommand impl | VERIFIED | 231 lines. Contains `impl RunCommand for HashArgs`, `Algo` ValueEnum (sha256/blake3/sha512/md5), `hash_rustcrypto` generic helper, `hash_blake3` native arm, `algo_from_len`. `Option<Algo>` for WR-01 fix. |
| `src/core/input.rs` | Streaming `read_file_or_stdin` (deferred --file branch) + unit tests | VERIFIED | `ResolvedInput`, `read_file_or_stdin`, `resolve_reader` at lines 121-201. Unit tests for all 3 branches at lines 253-306. |
| `src/core/errors.rs` | `UnsupportedHashLength` variant + `MissingInput` | VERIFIED | Both variants present at lines 25-36. Both downcast-mapped to exit 2 in `main.rs:100-105`. |
| `tests/hash.rs` | 8 named integration tests incl. `hash_verify_explicit_algo_overrides_length_autodetect` | VERIFIED | 8 tests pass. The WR-01 fix added 1 test beyond the original 7. |
| `src/commands/tree/mod.rs` | TreeArgs + box-drawing render + --sizes/--depth + count summary + RunCommand impl | VERIFIED | 289 lines. Contains `TEE/ELL/PIPE/GAP` constants, `sort_children`, `render_dir`, `impl RunCommand for TreeArgs`. |
| `src/core/output.rs` | `pub fn human_size` (promoted from flatten) | VERIFIED | `pub fn human_size` at line 198. |
| `tests/tree.rs` | 6 integration tests incl. WR-02/03/04 fix tests | VERIFIED | 6 tests pass. |
| `src/commands/du/mod.rs` | DuArgs + per-child accumulation + biggest-first sort + --top/--depth + summary + RunCommand impl | VERIFIED | 281 lines. `RangedU64ValueParser` on `--top`/`--depth` (WR-04). `sort_rows`, `dir_total`, `collect_rows`. |
| `tests/du.rs` | 5 integration tests | VERIFIED | 5 tests pass (`du_biggest_first`, `du_top_and_total_summary`, `du_depth_cap`, `du_file_argument_errors`, `du_missing_path_friendly_error`, `du_zero_depth_and_top_rejected`). |
| `src/commands/dupes/mod.rs` | DupesArgs + size pre-filter + rayon BLAKE3 + deterministic groups + wasted-space + RunCommand impl | VERIFIED | 312 lines. `collect_by_size`, `hash_file_blake3`, `group_duplicates`, `wasted_space`, `render`. `par_iter` at line 93. |
| `tests/dupes.rs` | 6 integration tests | VERIFIED | 6 tests pass. |
| `src/commands/bulk_rename/mod.rs` | BulkRenameArgs + pre-flight detector (all D-18 rules + CR-01 dots/dotdot) + dry-run-default + RunCommand impl | VERIFIED | 865 lines. `injects()` helper handles CR-01. `preflight` implements all 4 D-18 rules. `build_plan`, `preflight_plan`, `print_plan_with_conflicts`. |
| `tests/bulk_rename.rs` | 9 integration tests incl. CR-01 `renm_dotdot_target_aborts` and WR-05 `renm_recursive_cross_directory_independent` | VERIFIED | 9 tests pass. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `commands::hash::HashArgs::run` | `Commands::Hash(args) => args.run()` | VERIFIED | Line 71. No `not_implemented("hash")` arm remains. |
| `src/main.rs` | `BoxError::UnsupportedHashLength` | downcast arm → exit 2 | VERIFIED | `main.rs:100-105` matches both `MissingInput` and `UnsupportedHashLength { .. }`. |
| `src/commands/hash/mod.rs` | `blake3::Hasher::update_reader` | native stable hasher arm | VERIFIED | `hash_blake3` at line 109-115 uses `update_reader`. No `traits-preview`. |
| `src/main.rs` | `commands::tree::TreeArgs::run` | `Commands::Tree(args) => args.run()` | VERIFIED | Line 72. No `not_implemented("tree")` arm remains. |
| `src/commands/tree/mod.rs` | `core::output::human_size` | `--sizes` column render | VERIFIED | `use crate::core::output::{human_size, is_color_on}` at line 38. Called at `tree/mod.rs:151`. |
| `src/commands/tree/mod.rs` | `core::fs::is_hidden` | shared walk filter | VERIFIED | `filter_entry(|e| !is_hidden(e))` at `tree/mod.rs:194`. |
| `src/commands/flatten/mod.rs` | `core::output::human_size` | call site updated to promoted fn | VERIFIED | Confirmed via grep: no local `fn human_size` in flatten/mod.rs. |
| `src/main.rs` | `commands::du::DuArgs::run` | `Commands::Du(args) => args.run()` | VERIFIED | Line 73. No `not_implemented("du")` arm remains. |
| `src/commands/du/mod.rs` | `core::output::human_size` | shared size formatter | VERIFIED | `use crate::core::output::{human_size, is_color_on}` at line 44. |
| `src/main.rs` | `commands::dupes::DupesArgs::run` | `Commands::Dupes(args) => args.run()` | VERIFIED | Line 74. No `not_implemented("dupes")` arm remains. |
| `src/commands/dupes/mod.rs` | `blake3::Hasher::update_reader` | content-equality hash via `par_iter` | VERIFIED | `hash_file_blake3` → `hash_reader_blake3` → `update_reader` at `dupes/mod.rs:155-159`. `par_iter` at line 93. |
| `src/commands/dupes/mod.rs` | `core::fs::is_hidden` | shared walk filter, no noise list | VERIFIED | `filter_entry(|e| !is_hidden(e))` at `dupes/mod.rs:122`. No noise-directory list anywhere. |
| `src/main.rs` | `commands::bulk_rename::BulkRenameArgs::run` | `Commands::BulkRename(args) => args.run()` | VERIFIED | Line 75. No `not_implemented("bulk-rename")` arm remains. |
| `src/commands/bulk_rename/mod.rs` | pre-flight ABORT-ALL collision set | `preflight_plan` → `preflight` per directory | VERIFIED | `preflight_plan` at lines 499-528. Called before any `rename` in `RunCommand::run`. |
| `src/commands/bulk_rename/mod.rs` | `regex::Regex::replace` (first-match) | pattern applied to full base name | VERIFIED | `re.replace(&old_name, replacement).into_owned()` at `bulk_rename/mod.rs:447`. `.replace` is first-match-only. |

### Data-Flow Trace (Level 4)

These commands are pure I/O tools (compute and display from real filesystem/stdin data). No hardcoded empty returns or static mock data found.

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `hash/mod.rs` | `computed` (hex string) | `digest_reader(algo, input.reader)` streaming from file/stdin | Yes — streaming hash from real bytes | FLOWING |
| `tree/mod.rs` | `children` | `read_children(dir)` via `WalkDir` | Yes — real filesystem walk | FLOWING |
| `du/mod.rs` | `rows` | `collect_rows(&root, self.depth)` via `WalkDir` + `metadata().len()` | Yes — real file sizes | FLOWING |
| `dupes/mod.rs` | `hashed` / `groups` | `collect_by_size` walk → `par_iter` BLAKE3 hash | Yes — real content hashes | FLOWING |
| `bulk_rename/mod.rs` | `plan` | `build_plan` via `WalkDir` + `regex.replace` | Yes — real directory entries | FLOWING |

### Behavioral Spot-Checks

The full test suite provides behavioral validation. The binary is built and all commands run real logic (no `not_implemented` stubs for Phase 3 commands).

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full test suite (98 unit + integration) | `cargo test` | 98 passed, 0 failed | PASS |
| No `not_implemented` arms for Phase 3 commands | grep `not_implemented("hash\|tree\|du\|dupes\|bulk-rename")` | 0 matches | PASS |
| Clippy -D warnings | `cargo clippy --all-targets -- -D warnings` | 0 warnings | PASS |
| `cargo fmt --check` | `cargo fmt --check` | clean | PASS |

### Probe Execution

No phase-declared probe scripts found. Step 7c: SKIPPED (no probe-*.sh files declared or conventional).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| HASH-01 | 03-01-PLAN.md | Hash files (SHA-256 default), choose algorithm, hash stdin, verify against known hash (exit 0/1/2) | SATISFIED | `hash/mod.rs` implements all behaviors. 8 integration tests green. `REQUIREMENTS.md` row marked Complete. |
| TREE-01 | 03-02-PLAN.md | Directory tree with box-drawing chars, optional sizes, depth limit, colored dirs, file/dir count summary | SATISFIED | `tree/mod.rs` implements all behaviors. 6 integration tests green. Requires TTY for color (human check). |
| DU-01 | 03-03-PLAN.md | Disk usage size-sorted view (biggest first), human-readable sizes, depth limit, top-N truncation | SATISFIED | `du/mod.rs` implements all behaviors. Integration tests green. Requires TTY for color accent (human check). |
| DUPE-01 | 03-04-PLAN.md | Find duplicates by content (size pre-filter then hash), display groups + wasted-space summary, read-only | SATISFIED | `dupes/mod.rs` implements all behaviors including read-only invariant. 6 integration tests green. |
| RENM-01 | 03-05-PLAN.md | Bulk-rename with regex, dry-run-first default, --force to execute, collision detection aborts before any rename | SATISFIED | `bulk_rename/mod.rs` implements all behaviors including CR-01 dots-only fix. 9 integration tests green. |

No orphaned requirements — all 5 Phase 3 requirements appear in the REQUIREMENTS.md traceability table as Complete and are covered by their respective plans.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/commands/bulk_rename/mod.rs` | 382 | `let _ = is_color_on();` (dead call, IN-02) | Info | No behavioral impact. The `is_color_on()` call is dead — color is applied inside `format_row`. Explicitly deferred in 03-REVIEW.md as an Info finding. Clippy accepts it because the return value is explicitly discarded via `let _`. |

No TBD/FIXME/XXX markers found in any Phase 3 source files. No stub implementations found. The IN-02 dead call is intentionally deferred per the code review.

### Human Verification Required

The automated suite forces `NO_COLOR=1` on every integration test, so terminal color rendering cannot be machine-verified. Three color behaviors need human confirmation in a real PowerShell 7 session:

#### 1. Tree Directory Color

**Test:** Run `box tree ./src` in PowerShell 7 (no piping)
**Expected:** Directory names appear in blue+bold; file names and branch glyphs (├── └── │) are uncolored/plain; the box-drawing glyphs render visibly (no font rendering issues)
**Why human:** Color requires a TTY; all automated tests suppress color via `NO_COLOR=1`

#### 2. Du Size Accent Color

**Test:** Run `box du .` in PowerShell 7 (no piping)
**Expected:** The size-value column (left side of each row) is cyan-colored; file/directory names are plain; trailing `/` marks directory rows; rows appear biggest-first
**Why human:** Color requires a TTY; automated tests force `NO_COLOR=1`

#### 3. Dupes Group Header Accent Color

**Test:** Run `box dupes .` in a directory with some duplicate files (e.g. copy a file twice)
**Expected:** Group header lines (`X B each, N copies`) and the final wasted-space summary are yellow-accented; file paths inside groups are plain
**Why human:** Color requires a TTY; automated tests force `NO_COLOR=1`

### Gaps Summary

No gaps. All 24 must-have truths are VERIFIED. All 5 requirement IDs (HASH-01, TREE-01, DU-01, DUPE-01, RENM-01) are SATISFIED. The code review BLOCKER (CR-01: `..`/`.`/dots-only bulk-rename target escape) and all 5 Warnings (WR-01 through WR-05) have been fixed and are confirmed verified by the test suite. The 4 Info items (IN-01 through IN-04) are intentionally deferred and do not affect goal achievement.

The only open items are 3 human verification checks for terminal color rendering — a standard TTY-dependent behavior that is architecturally sound (gated uniformly on `is_color_on()`) but cannot be asserted programmatically.

---

_Verified: 2026-06-23_
_Verifier: Claude (gsd-verifier)_
