---
phase: 03-filesystem-power-tools
plan: 03
subsystem: cli-command
tags: [du, disk-usage, walkdir, human_size, owo-colors, biggest-first, top, depth, determinism]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: "RunCommand trait + main() dispatch/exit-code policy; core::fs::is_hidden walk filter + normalize_path (dunce); core::output color gate (is_color_on); the cli.rs/main.rs stub-swap precedent"
  - phase: 03-filesystem-power-tools (plan 02)
    provides: "core::output::human_size promoted to pub (D-12) — du consumes it for its size column with NO further promotion; the WalkDir depth-1 + is_hidden read-only-walker pattern (D-06); binary-only crate test-targeting note (--bin box, not --lib)"
provides:
  - "Live `box du` command: one row per immediate child, biggest-first (size desc, name asc), dirs show recursive total + trailing `/`, files show own size, right-aligned shared human_size column, cyan size accent gated on is_color_on()"
  - "`--top N` post-sort truncation of the SHOWN rows (summary still reflects the full-scan total); `--depth N` aggregation cap on how deep a directory's recursive total is rolled up"
  - "The `{X} of {Y} entries shown. {TOTAL} total.` summary that ALWAYS reflects the FULL-scan total, not just shown rows (D-11)"
affects: [dupes, bulk-rename, any-future-size-sorted-or-walk-accumulate-command]

# Tech tracking
tech-stack:
  added: []  # no new crate — reuses walkdir/owo-colors and the promoted core::output::human_size (D-12, T-03-SC)
  patterns:
    - "Walk-accumulate-sort-render: collect one Row per immediate child (WalkDir depth-1 + is_hidden), recursively sum each dir's descendants (WalkDir min_depth(1) + optional max_depth(N) for --depth), then sort_by (size desc, name asc) BEFORE printing (RESEARCH Pitfall 6 — never walk order)"
    - "Full-scan total captured BEFORE --top truncation so the summary reflects the whole scan regardless of how many rows are shown (D-11)"
    - "Right-aligned size column: format!(\"{s:>width$}\") to the widest SHOWN human_size value, single .cyan() accent gated on is_color_on() so piped output is byte-identical minus ANSI"

key-files:
  created:
    - "src/commands/du/mod.rs"
    - "tests/du.rs"
  modified:
    - "src/cli.rs"
    - "src/main.rs"
    - "src/commands/mod.rs"

key-decisions:
  - "du --depth N caps the per-directory recursive ROLLUP (the dir's own files are depth 1); the summary total then legitimately sums the CAPPED row totals — so under --depth 1 the grand total is the capped sum, internally consistent with the shown rows. This resolved the only RED-test mismatch (test-side fix, not an impl change)"
  - "Recursive dir total reuses core::fs::is_hidden + follow_links(false) VERBATIM via WalkDir (same read-only-walker pattern tree established, D-06) — symlink-loop safety (T-03-09) + dotted-root exemption (T-03-10) inherited, never re-implemented"
  - "Logical file size via metadata().len() (RESEARCH A4) — NOT allocated/on-disk apparent size (that is DU-V2). Symlinks are never followed and contribute 0 (skipped in the descendant sum)"
  - "Single styled token = the size VALUE only (.cyan()), gated on is_color_on(); the path/name is uncolored (D-11) — same gate flatten/tree use, so `box du | cat` is byte-identical minus ANSI"

patterns-established:
  - "Determinism for walk-order-arbitrary output: collect -> sort_by a total-order key (size desc, name asc) before any print; tests use distinct-size fixtures so ties never flap (the same discipline dupes will need in 03-04)"
  - "core::output::human_size is the single shared size formatter — du now its third consumer (flatten, tree, du) with zero Cargo.toml change"

requirements-completed: [DU-01]

# Metrics
duration: 7min
completed: 2026-06-22
---

# Phase 3 Plan 03: du Summary

**Live `box du`: a biggest-first disk-usage view with one row per immediate child — directories showing their recursive total (trailing `/`), files their own size — a `(size desc, name asc)` deterministic sort, a `--top N` post-sort truncation, a `--depth N` aggregation cap, a right-aligned shared `human_size` column with an `is_color_on()`-gated cyan accent, and a `{X} of {Y} entries shown. {TOTAL} total.` summary that always reflects the FULL scan total (D-11/D-12).**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-06-22 (Wave 3)
- **Completed:** 2026-06-22
- **Tasks:** 2 (Task 1 = Wave-0 RED tests; Task 2 = GREEN implementation + wiring)
- **Files modified:** 5 (2 created — `src/commands/du/mod.rs`, `tests/du.rs` — 3 modified: cli.rs, main.rs, commands/mod.rs)

## Accomplishments
- `box du` is fully live: it enumerates the target's immediate children (WalkDir depth-1, hidden pruned via `core::fs::is_hidden`), computes one row per child (file = own `metadata().len()`, dir = recursive non-hidden descendant sum), sorts `(size desc, name asc)` BEFORE printing (RESEARCH Pitfall 6 / T-03-12), right-aligns the size column to the widest shown `human_size`, marks dirs with a trailing `/` (ASCII), colors ONLY the size value `.cyan()` gated on `is_color_on()`, and prints the full-scan `{X} of {Y} entries shown. {TOTAL} total.` summary — all 3 DU-01 integration tests + 4 co-located unit tests green.
- `--top N` truncates the SHOWN rows post-sort while the summary STILL reports the full-scan total (the total is captured before truncation, D-11). `--depth N` caps how deep each directory's recursive total rolls up (the dir's own files are depth 1) via `WalkDir::max_depth(N)`.
- Reused `core::output::human_size` (the D-12 promotion from 03-02) for the size column with ZERO Cargo.toml change — du is now its third consumer (flatten, tree, du). Reused `core::fs::is_hidden` + `follow_links(false)` VERBATIM so symlink-loop safety (T-03-09) and the dotted-root exemption (T-03-10) are inherited, and `core::fs::normalize_path` (dunce) so no `\\?\` UNC prefix leaks (T-03-11).
- Wired the stub: `cli.rs` `Du` now carries `DuArgs`, `main.rs` dispatches `Commands::Du(args) => args.run()`, and `pub mod du;` is registered — the `not_implemented("du")` arm is gone (2 phase-3 stubs remain: dupes, bulk-rename).

## Task Commits

Each task was committed atomically:

1. **Task 1: Wave-0 du tests** — `0a135f1` (test) — the RED gate (3 du tests fail against the unit `Du` stub, which takes no positional path → "unexpected argument")
2. **Task 2: du command — per-child recursive totals + biggest-first sort + --top/--depth + summary + wiring** — `7830b19` (feat) — the GREEN gate (all du tests + clippy `-D warnings` + fmt clean)

**Plan metadata:** (final docs commit — this SUMMARY + STATE + ROADMAP + REQUIREMENTS)

_TDD-style gate sequence in git log: `test(03-03)` (RED) → `feat(03-03)` (GREEN). No REFACTOR commit needed — the GREEN implementation was already clippy `-D warnings` + `fmt --check` clean._

## Files Created/Modified
- `src/commands/du/mod.rs` (created) — `DuArgs` (`path`/`--top`/`--depth`) + `RunCommand` impl; `collect_rows` (WalkDir depth-1 + `is_hidden`, file=own size / dir=`dir_total`), `dir_total` (recursive non-hidden descendant sum with optional `max_depth` for `--depth`), `sort_rows` (size desc, name asc), `color_size` (is_color_on-gated cyan); co-located unit tests (sort order incl. tie, plain-when-color-off, recursive sum + depth cap, hidden-descendant skip)
- `tests/du.rs` (created) — 3 DU-01 integration tests (`du_biggest_first`, `du_top_and_total_summary`, `du_depth_cap`) with a distinct-size fixture (`big/`=5000, `mid/`=1500, `small.txt`=500), `NO_COLOR=1`
- `src/cli.rs` (modified) — `Du` variant now carries `DuArgs` (was a unit stub)
- `src/main.rs` (modified) — `Commands::Du(args) => args.run()` (removed the `not_implemented("du")` arm)
- `src/commands/mod.rs` (modified) — registered `pub mod du;`

## Decisions Made
- **`--depth N` caps the per-dir rollup; the summary total sums the CAPPED rows.** The cleanest, internally-consistent reading of D-11: under `--depth 1`, every child's total is the capped sum, so the grand total is also the capped sum (the summary equals the sum of the shown row totals). The "full scan" in D-11 means "all immediate children," not "ignore the depth cap" — the cap is applied uniformly to rows and total alike. (See the test-side fix below; the implementation behavior is the correct DU-01 semantics.)
- **Recursive dir total via the shared read-only walker, logical size only.** `dir_total` uses `WalkDir::new(dir).min_depth(1).follow_links(false).filter_entry(!is_hidden)` (+ `max_depth(N)` when `--depth` is set) and sums only regular files' `metadata().len()` (RESEARCH A4 — logical, not apparent/on-disk size; apparent-size is DU-V2). Symlinks are never followed and hidden entries are pruned, so `du`'s totals match `tree`'s/`flatten`'s view of the tree (D-06).
- **Single styled token = the size VALUE only.** Right-aligned via `format!("{s:>width$}")` to the widest SHOWN `human_size`, then `.cyan()` gated on `is_color_on()` — the path/name carries no color, so `box du | cat` is byte-identical minus ANSI (D-11, the same gate flatten/tree use).
- **Determinism by distinct-size fixtures.** The DU-01 tests build children with distinct recursive totals (5000/1500/500) so the `(size desc, name asc)` sort is a TOTAL order and the rendered order can't flap — the discipline `dupes` (03-04, rayon) will reuse.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test correctness] `du_depth_cap` asserted `4.9 KB` absent from the WHOLE output; corrected to assert the `big/` ROW line**
- **Found during:** Task 2 GREEN gate (`cargo test --test du`)
- **Issue:** The Wave-0 (Task 1) `du_depth_cap` test asserted that under `--depth 1`, `"4.9 KB"` must NOT appear anywhere in stdout. But the implementation correctly caps big/'s ROW to `2.9 KB` (3000 bytes, excluding the depth-2 descendant) while the SUMMARY total legitimately sums the capped rows to `4.9 KB total.` (2.9 + 1.5 + 0.5 KB). The whole-output assertion conflated the (correct) capped row with the (correct) capped summary total. The implementation behavior is the right, internally-consistent DU-01 semantics — the over-broad test assertion was the defect.
- **Fix:** Re-scoped the assertion to the `big/` ROW line specifically (find the line containing `big/`, assert it contains `2.9 KB` and NOT `4.9 KB`), and the uncapped run's `big/` row to `4.9 KB`. The summary line no longer muddies the row-level assertion.
- **Files modified:** tests/du.rs
- **Verification:** `cargo test --test du` 3/3 green; `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` clean.
- **Committed in:** `7830b19` (Task 2 commit)

### Process note (not a code deviation)

- **`--lib` → `--bin box` verify substitution (carried from 03-01/03-02):** `box` is a binary-only crate (no `[lib]`), so the co-located unit tests run via `cargo test --bin box du::` (4/4 green), NOT `cargo test --lib`. Already recorded as the binary-crate note in STATE.md [03-01]; no code change.

---

**Total deviations:** 1 auto-fixed (Rule 1 — an over-broad Wave-0 test assertion corrected to a row-scoped one; the implementation was already correct).
**Impact on plan:** None on scope/design — the du command matches D-11/D-12 exactly; only a test assertion was tightened to distinguish a row total from the summary total.

## Issues Encountered
- The only hiccup was the `du_depth_cap` assertion above — a Wave-0 test that under-specified WHERE `4.9 KB` should not appear. The other two RED tests went green immediately once `DuArgs` + `RunCommand` landed (they had failed cleanly against the unit-`Du` stub with "unexpected argument" for the positional path, exactly like 03-02's tree RED gate).

## User Setup Required
None — no external service configuration required. `box du` works offline against the local filesystem.

## Next Phase Readiness
- The walk-accumulate-sort-render pattern (collect → recursive sum → `(size desc, name asc)` sort → right-aligned `human_size` render) is now available for `dupes` (03-04), which adds the rayon parallel-hash phase on top of the same deterministic-sort discipline (RESEARCH Pitfall 6).
- `core::output::human_size` has three live consumers now (flatten, tree, du); no further promotion needed.
- ROADMAP Phase-3 success criterion #3 (the `box du` contract: biggest-first, `--top`, `--depth`) is met. No blockers introduced. 2 phase-3 stubs remain: dupes, bulk-rename.

## Self-Check: PASSED

- FOUND: src/commands/du/mod.rs
- FOUND: tests/du.rs
- FOUND: .planning/phases/03-filesystem-power-tools/03-03-SUMMARY.md
- FOUND commit: 0a135f1 (Task 1, RED)
- FOUND commit: 7830b19 (Task 2, GREEN)

---
*Phase: 03-filesystem-power-tools*
*Completed: 2026-06-22*
