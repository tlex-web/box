---
phase: 08-filesystem-depth
plan: 01
subsystem: cli
tags: [hash, flatten, coreutils, indicatif, blake3, walkdir, clap, progress, cargo-deps]

# Dependency graph
requires:
  - phase: 06-scriptable-core-foundation
    provides: "frozen --json/--clip spine (is_json_on/emit_json/out_line), {results,count} struct template, BLAKE3-default hash"
  - phase: 07-spine-rollout
    provides: "flatten FlattenOutput {results,count,dry_run} projection + json_purity test pattern"
provides:
  - "box hash accepts multiple file args → one coreutils two-space digest␣␣filename row per file (HASH-V2-02)"
  - "box hash best-effort partial failure: bad file → stderr error + exit 1, good files still hashed; --json carries successful rows + exit 1 (A1)"
  - "box hash stderr file-count progress bar for large batches, suppressed under --json"
  - "box flatten --extensions/--separator/--include-hidden folded into build_plan + stderr copy progress (FLAT-V2-01)"
  - "rename::encode_relative parameterized by collision-join separator"
  - "all four Phase-8 deps (indicatif/ignore/globset/windows 0.61) in Cargo.toml so 08-02/08-03 never touch the manifest"
affects: [08-02-tree-du, 08-03-dupes-bulk-rename, 08-04-flatten-move, 08-05-dupes-delete, 08-06-bulk-rename-backup]

# Tech tracking
tech-stack:
  added: ["indicatif 0.18.4", "ignore 0.4 (dev→runtime)", "globset 0.4 (dev→runtime)", "windows 0.61 (Win32_Storage_FileSystem + Win32_Foundation)"]
  patterns: ["stderr-only progress via ProgressDrawTarget::stderr() gated on !is_json_on()", "multi-file fan-out into the existing {results,count} doc", "input filter folded into the single source-of-truth walk (no-drift)"]

key-files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/commands/hash/mod.rs
    - src/commands/flatten/mod.rs
    - src/commands/flatten/rename.rs
    - tests/hash.rs
    - tests/flatten.rs

key-decisions:
  - "HASH-V2-02 --json partial failure (A1): emit the document with ONLY the successful rows AND exit 1 — partial-success refinement of D-09 (whose empty-stdout rule is for TOTAL failure)"
  - "Progress cutoffs (Claude's Discretion): hash file-count bar for >8 files; flatten copy bar for >16 plan items; both stderr-only, never constructed under --json"
  - "encode_relative splits on the REAL path separators (not on `sep`) so a multi-char/unusual separator stays correct; dedupe numeric suffix stays `_` regardless of --separator (it is a uniqueness counter, not a segment join)"
  - "--separator validated to reject / and \\ before any I/O (T-8-01); --extensions is a pure lowercased-set compare (no glob/regex, T-8-01-INJ)"
  - "windows 0.61 added as a DIRECT dep with two feature modules; unifies with the transitive windows ^0.61 from tauri-winrt-notification (cargo tree -i windows = single 0.61.3)"

patterns-established:
  - "Stderr progress: ProgressBar::with_draw_target(Some(n), ProgressDrawTarget::stderr()) behind a `!is_json_on() && len > THRESHOLD` guard — the copy-me pattern for du/dupes progress in 08-02/08-03"
  - "Multi-input commands fan out into the frozen {results,count} doc; best-effort partial failure logs per-item `error:` on stderr and exits 1 via std::process::exit after flushing rows"

requirements-completed: [HASH-V2-02, FLAT-V2-01]

# Metrics
duration: 15min
completed: 2026-06-27
---

# Phase 8 Plan 01: hash multi-file + flatten filters Summary

**`box hash a.bin b.bin` now prints coreutils two-space rows with best-effort exit-1 partial failure and stderr progress; `box flatten` gains `--extensions`/`--separator`/`--include-hidden` folded into one no-drift walk; all four Phase-8 deps (indicatif/ignore/globset/windows 0.61) landed once.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-27T21:32:35Z
- **Completed:** 2026-06-27T21:47:48Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments
- **HASH-V2-02:** `HashArgs.path: Option<String>` → `paths: Vec<String>`; new `run_compute` loops each path through `read_file_or_stdin` + `digest_reader`, printing one two-space coreutils `{digest}  {label}` row via `out_line`. Best-effort partial failure (A1): unreadable file → `error:` on stderr + final exit 1, other files still hashed; under `--json` the document carries only the successful rows and still exits 1. stderr file-count progress for batches > 8, never under `--json`. `--verify` preserved single-input (first path), including the D-05 BLAKE3 probe.
- **FLAT-V2-01:** Three new flags folded into the single source-of-truth `build_plan` walk: `--include-hidden` bypasses the D-06 prune; `--extensions` keeps only matching final extensions (case-insensitive) before plan items (no count inflation, not skip rows); `--separator` threads into `rename::encode_relative` (now `encode_relative(rel, sep)`). `--separator` validated to reject `/`/`\` before any I/O. stderr copy progress for plans > 16 items, never under `--json`.
- **Dependency wiring:** `indicatif 0.18.4`, `ignore 0.4` + `globset 0.4` (promoted dev→runtime), and `windows 0.61` (`Win32_Storage_FileSystem` + `Win32_Foundation`) added once so 08-02/08-03 never edit the manifest; `cargo tree -i windows` stays a single 0.61.3.

## Task Commits

Each task was committed atomically:

1. **Task 1: Phase-8 deps + RED test scaffold** - `2568f6c` (chore)
2. **Task 2: hash multi-file + coreutils + stderr progress (HASH-V2-02)** - `d1d5904` (feat)
3. **Task 3: flatten --extensions/--separator/--include-hidden + progress (FLAT-V2-01)** - `5c1ee24` (feat)

_TDD note: the RED seams for both features (multi_file_two_space / json_multifile_purity / partial_failure_exit1 / extensions_filter / include_hidden) landed as the failing `test`-bearing Task 1 commit; Tasks 2 and 3 are the `feat` GREEN implementations._

## Files Created/Modified
- `Cargo.toml` - added indicatif/ignore/globset/windows 0.61 (Phase-8 block before `[dev-dependencies]`)
- `Cargo.lock` - resolved the four new deps (windows unified to 0.61.3)
- `src/commands/hash/mod.rs` - `paths: Vec<String>`, `run_compute` multi-file loop, best-effort exit-1, indicatif stderr bar, `PROGRESS_FILE_THRESHOLD`
- `src/commands/flatten/mod.rs` - `extensions`/`separator`/`include_hidden` args, `parse_extensions`, filters in `build_plan`, separator validation, indicatif copy progress, `PROGRESS_ITEM_THRESHOLD`
- `src/commands/flatten/rename.rs` - `encode_relative(rel, sep)` parameterized join; new `encode_relative_honors_separator` unit test; updated existing tests for the new signature
- `tests/hash.rs` - `multi_file_two_space`, `json_multifile_purity`, `partial_failure_exit1`
- `tests/flatten.rs` - `extensions_filter`, `include_hidden`

## Decisions Made
- **A1 (HASH-V2-02 --json partial failure):** emit the `{results,count}` document with only the successful rows AND exit 1 — the recommended partial-success refinement of D-09 (whose empty-stdout rule targets TOTAL failure). Baked in per the plan's RESOLVED Open Question #1.
- **Progress cutoffs (Claude's Discretion):** hash file-count bar for > 8 files; flatten copy bar for > 16 plan items. Below the cutoff no bar is drawn, so the common single-file/small-tree case (and every existing snapshot) is unaffected; both are stderr-only and never constructed under `--json`.
- **encode_relative rewrite:** split on the REAL path separators (`/` and `\`) rather than first replacing with `_` then splitting on `_`. This makes a multi-char/unusual separator correct and stops splitting a segment that itself contains `_` — a behavior superset of v1 that is byte-identical for the default `_`.
- **dedupe numeric suffix stays `_`** regardless of `--separator` (it is a within-output uniqueness counter, not a path-segment join) — documented in the `dedupe` doc comment.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None. The four deps resolved on the first build (windows unified to a single 0.61.3 with the transitive tauri-winrt-notification copy); all RED seams compiled and failed against the v1 binary as designed, then went green with the implementations.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All four Phase-8 deps are in the manifest, so **08-02 (tree+du)** and **08-03 (dupes+bulk-rename)** can consume `ignore`/`globset`/`windows`/`indicatif` without touching `Cargo.toml`.
- The stderr-progress pattern (`ProgressDrawTarget::stderr()` behind a `!is_json_on() && len > THRESHOLD` guard) is established and copy-me for du/dupes progress.
- The multi-input → `{results,count}` + best-effort-exit-1 pattern is available for any future multi-target command.
- Full `cargo test` green (159 unit + all integration files); `cargo clippy --all-targets -- -D warnings` clean.

## Self-Check: PASSED

All declared files exist; all three task commits (`2568f6c`, `d1d5904`, `5c1ee24`) are present in git history. Full `cargo test` green (159 unit + every integration file); `cargo clippy --all-targets -- -D warnings` clean; `cargo tree -i windows` = single `v0.61.3`.

---
*Phase: 08-filesystem-depth*
*Completed: 2026-06-27*
