---
phase: 08-filesystem-depth
plan: 02
subsystem: filesystem
tags: [tree, du, gitignore, globset, ignore-crate, windows-ffi, GetCompressedFileSizeW, percentage, band-color]

# Dependency graph
requires:
  - phase: 07-spine-rollout
    provides: "du {results,count,total_bytes,total_children} + tree recursive {name,type,size?,children} --json output structs (the no-drift fork these new fields ride)"
  - phase: 08-filesystem-depth (08-01)
    provides: "ignore 0.4 / globset 0.4 / windows 0.61 runtime deps already in Cargo.toml (this plan does NOT touch the manifest)"
provides:
  - "tree --gitignore (root + nested ancestor-stack, deeper-wins), --ignore <glob>, --dirs-only, --sort size — all opt-in, default render byte-identical to v1"
  - "du in-line percentage column (full-scan basis) + percentage-band color (>50% red, 10-50% yellow), --exclude <glob>, --on-disk (Win32 GetCompressedFileSizeW) + on_disk:bool JSON marker"
affects: [08-03 (dupes Win32 GetFileInformationByHandle reuses the per-module FFI pattern), 09-dev-visual-depth, 11-meta-commands (completions reflect these flags)]

# Tech tracking
tech-stack:
  added: [ignore 0.4 (promoted dev->runtime, gitignore matcher), globset 0.4 (promoted dev->runtime, --exclude), windows 0.61 (direct GetCompressedFileSizeW FFI)]
  patterns:
    - "Matcher-as-filter ancestor-stack (Vec<Gitignore> push/pop) threaded via a WalkCtx through both render_dir + build_node (the shared read_children chokepoint) — no WalkBuilder, no-drift by construction"
    - "Per-module localized Win32 unsafe wrapper (compressed_size in du/mod.rs) — the v1 arboard/winrt pattern; INVALID_FILE_SIZE disambiguated via GetLastError"
    - "Percentage is a render-only value (A2): guarded percent_str (total==0 -> 0.0%, never NaN); never an f64 in the JSON struct"

key-files:
  created: []
  modified:
    - "src/commands/tree/mod.rs (gitignore/ignore/dirs-only/sort matcher-as-filter inside read_children; WalkCtx; SortMode)"
    - "src/commands/du/mod.rs (percent column + band_color replacing color_size; --exclude globset; --on-disk Win32; on_disk marker)"
    - "tests/tree.rs (gitignore_nested 3-level fixture, dirs_only_ignore_sort, gitignore_json_nodrift, default_output_unchanged guard)"
    - "tests/du.rs (percentage_column + NaN guard, exclude_glob child+descendant, json_no_ansi, on_disk_size)"

key-decisions:
  - "D-32: tree gitignore = ancestor-stack Vec<Gitignore> checked deepest-first (deeper rule wins, eza #1086); --ignore globs as the shallowest matcher via add_line; --dirs-only after the ignore filter; --sort size = files biggest-first with dirs to the end; default byte-identical to v1"
  - "D-33: du percent is render-only (no f64 in JSON, A2); basis = full-scan total (on-disk total under --on-disk); --exclude globset matched relative to the target root (children + descendants); compressed_size Win32 wrapper localized per-module"

patterns-established:
  - "Matcher-as-filter at the single shared walk chokepoint (read_children) preserves human+JSON no-drift for filter flags"
  - "WalkCtx bundles immutable walk config to keep recursion arg counts under clippy too_many_arguments"
  - "Render-only derived columns (percentage) never enter the JSON channel — divide-by-zero guarded to keep NaN out of every output"

requirements-completed: [TREE-V2-01, DU-V2-01, DU-V2-02]

# Metrics
duration: ~30min
completed: 2026-06-28
---

# Phase 8 Plan 02: tree + du Depth Flags Summary

**Opt-in `tree --gitignore` (root + nested ancestor-stack, deeper-wins whitelist) / `--ignore` / `--dirs-only` / `--sort size` via the shared `read_children` chokepoint, plus a `du` full-scan percentage column + percentage-band color, `--exclude` globset, and `--on-disk` Win32 `GetCompressedFileSizeW` with an `on_disk` JSON marker.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-06-28T00:00:00Z (approx)
- **Completed:** 2026-06-28T00:25:29Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- **TREE-V2-01:** `tree` gains four opt-in depth flags, all layered inside the single `read_children` chokepoint that feeds BOTH `render_dir` (human) and `build_node` (`--json`), so the two recursions cannot drift:
  - `--gitignore` honors the root `.gitignore` AND every nested one, via an ancestor-stack `Vec<Gitignore>` checked deepest-first so a deeper rule wins (the eza #1086 class — a nested `!keep.log` whitelist re-shows a file the root `*.log` hid).
  - `--ignore '<glob>'` (repeatable) folds into the same matcher as the shallowest entry via `add_line(None, glob)`.
  - `--dirs-only` drops file children after the ignore filter.
  - `--sort size` orders files biggest-first (ties alpha), directories (no intrinsic size) to the end; `--sort name`/default keeps the v1 D-08 order.
  - With no flags the render is byte-identical to v1 (the `tests/cmd/tree.trycmd` pin stays green).
- **DU-V2-01:** `du` now prints a right-aligned `NN.N%` column before the size, with the basis = the full-scan total (so percentages stay honest after `--top`, and never become `100.0%`). `band_color` replaces the lone `.cyan()` accent: `>50%` red, `10–50%` yellow, else plain — gated on `is_color_on()`. `--exclude '<glob>'` (globset) drops matching immediate children AND keeps matching descendant files out of every directory total, matched on the path relative to the target root.
- **DU-V2-02:** `du --on-disk` reports each file's allocated/compressed NTFS size via a localized Win32 `GetCompressedFileSizeW` wrapper; a directory row sums its descendants' on-disk sizes and the percentage basis switches to the on-disk total. The JSON document gains a top-level `on_disk: bool` marker (no raw `f64` percent field — A2 — so JSON never carries `NaN`).
- Full suite green: 167 unit tests + all integration tests; `cargo clippy --all-targets -- -D warnings` clean.

## Task Commits

1. **Task 1: Scaffold Wave-0 tree/du tests (RED)** — `0a36f69` (test)
2. **Task 2: tree --gitignore/--ignore/--dirs-only/--sort size** — `1fe1d1e` (feat) + `4578b5f` (docs — reword to drop the literal `WalkBuilder` token so the source satisfies the "no WalkBuilder" acceptance)
3. **Task 3: du percentage + band color + --exclude + --on-disk** — `8e2b3ae` (feat)

**Plan metadata:** this commit (docs: complete plan)

_Note: Tasks 2 and 3 were TDD `feat` greens against the Task-1 RED integration tests, each adding its own pure-function unit tests (sort comparators; `percent_str`/`band_color`/`build_exclude`/`dir_total` exclude+depth)._

## Files Created/Modified

- `src/commands/tree/mod.rs` — `TreeArgs` + `SortMode` ValueEnum; `WalkOpts`/`WalkCtx`; matcher-as-filter inside `read_children`; `is_ignored` (deepest-first), `build_ignore_matcher`, `load_dir_gitignore`, `push_dir_gitignore`; size-aware `sort_children`; opt-in module doc.
- `src/commands/du/mod.rs` — `DuArgs` + `--exclude`/`--on-disk`; `on_disk` on `DuOutput`; percentage column + `band_color` (replacing `color_size`); `percent_str`; `build_exclude`/`is_excluded`; `compressed_size` (Win32 `#[cfg(windows)]` + portable fallback); `collect_rows`/`dir_total` thread root+exclude+on_disk.
- `tests/tree.rs` — `gitignore_nested` (3-level fixture w/ deeper-wins whitelist), `gitignore_json_nodrift` (recursive name set, no-drift), `dirs_only_ignore_sort`, `default_output_unchanged` (opt-in guard).
- `tests/du.rs` — `percentage_column` (full-scan basis + NaN guard), `exclude_glob` (child dropped + descendant out of total), `json_no_ansi` (`on_disk` marker false/true), `on_disk_size` (Win32 allocated size reported).

## Decisions Made

- **D-32** (logged to STATE.md): tree gitignore ancestor-stack design (deepest-first, `--ignore` shallowest, opt-in default-unchanged, `WalkCtx` for clippy arg-count).
- **D-33** (logged to STATE.md): du percent is render-only (A2), full-scan/on-disk basis, root-relative `--exclude`, per-module Win32 FFI (the wave-isolation choice resolved in RESEARCH Open Question 3 — keeps 08-02 and 08-03 from both editing `core/fs.rs`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `clippy::too_many_arguments` on `render_dir`/`build_node`**
- **Found during:** Task 2 (tree)
- **Issue:** Threading `opts` + `stack` through both recursions pushed each function to 8 arguments, which `cargo clippy --all-targets -- -D warnings` (a plan verification gate) rejects.
- **Fix:** Introduced a `WalkCtx` struct bundling the immutable walk config (`max_depth`, `sizes`, `opts`), passed by `&`; the mutable threading state (`Counts`, the gitignore `stack`) stays separate. Both functions drop to ≤7 args.
- **Files modified:** src/commands/tree/mod.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` clean; all tree tests still pass.
- **Committed in:** 1fe1d1e (Task 2 commit)

**2. [Rule 3 - Blocking] Literal `WalkBuilder` token in a doc comment tripped the acceptance grep**
- **Found during:** Task 2 (tree) post-implementation check
- **Issue:** The TREE-V2-01 acceptance asserts the source "contains NO `WalkBuilder`"; the matcher-as-filter rationale comment named `ignore::WalkBuilder` to explain what was deliberately avoided, which a literal grep would flag.
- **Fix:** Reworded the comment to "the recursive walker from the `ignore` crate" — no behavior change, the source no longer contains the token.
- **Files modified:** src/commands/tree/mod.rs
- **Verification:** `grep WalkBuilder src/commands/tree/mod.rs` → 0 matches; build clean.
- **Committed in:** 4578b5f

---

**Total deviations:** 2 auto-fixed (both Rule 3 - blocking, both to satisfy the plan's own verification gates).
**Impact on plan:** No scope change. Both fixes were required to pass the explicit clippy-clean and "no WalkBuilder" acceptance criteria; the feature behavior matches the plan exactly.

## Issues Encountered

- The `gsd-sdk query` state handlers (`record-metric`, `record-session`) and `roadmap.update-plan-progress` use named-flag / SUMMARY-on-disk inputs rather than the positional form in the executor template, and the percentage recalculation regressed the frontmatter to a different counting model. Resolved by completing the position/decision/metric/session/progress edits manually so STATE.md, ROADMAP.md, and REQUIREMENTS.md are internally coherent (requirements were marked complete cleanly by the SDK).
- `du --on-disk` test fixtures: a small (<~700 byte) file is MFT-resident and `GetCompressedFileSizeW` returns 0 for it, so the `on_disk_size` test uses a 5000-byte (non-resident) file to assert a positive allocated size. Directional `on_disk` vs apparent comparisons were avoided (uncompressed files round UP to a cluster, so on-disk ≥ apparent — the opposite of the compressed case).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 08-03 (dupes multi-stage + hardlink collapse; bulk-rename `--case` + `{n}`) is unblocked: the `windows 0.61` direct dep + `Win32_Storage_FileSystem` feature are live, and the per-module localized Win32 FFI pattern (D-33, `compressed_size`) is the copy-me for dupes' `GetFileInformationByHandle` hardlink identity.
- Wave 2 destructive plans (08-04/05/06) remain; each carries the mandatory adversarial code-review gate.

## Self-Check: PASSED

- Files verified present: `src/commands/tree/mod.rs`, `src/commands/du/mod.rs`, `tests/tree.rs`, `tests/du.rs`.
- Commits verified in git history: `0a36f69` (RED), `1fe1d1e` (tree feat), `4578b5f` (tree doc), `8e2b3ae` (du feat).
- Artifacts: `tree/mod.rs` 654 lines (≥430, contains `Gitignore`, `matched`, no `WalkBuilder`); `du/mod.rs` 549 lines (≥380, contains `GetCompressedFileSizeW`, `GlobSet`).

---
*Phase: 08-filesystem-depth*
*Completed: 2026-06-28*
