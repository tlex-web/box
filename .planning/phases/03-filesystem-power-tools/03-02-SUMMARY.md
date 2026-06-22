---
phase: 03-filesystem-power-tools
plan: 02
subsystem: cli-command
tags: [tree, walkdir, box-drawing, human_size, owo-colors, depth, sizes, dir-first]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: "RunCommand trait + main() dispatch/exit-code policy; core::fs::is_hidden walk filter + normalize_path (dunce); core::output color gate (is_color_on); flatten's human_size formatter"
  - phase: 03-filesystem-power-tools (plan 01)
    provides: "phase deps already in Cargo.toml/Cargo.lock; the cli.rs/main.rs stub-swap precedent; binary-only crate test-targeting note (--bin box, not --lib)"
provides:
  - "Live `box tree` command: dir-first Unicode box-drawing render, colored directory names, `--sizes` per-file column, `--depth N` cap, and an `N directories, M files` summary"
  - "core::output::human_size: the promoted shared 1024-based size formatter (B/KB/MB/GB/TB), now consumed by flatten + tree and ready for du (Plan 03-03, D-12)"
  - "WalkDir-depth-1 children pattern that reuses core::fs::is_hidden VERBATIM for a per-level recursive render (D-06)"
affects: [du, dupes, bulk-rename, any-future-size-formatting-or-tree-render-command]

# Tech tracking
tech-stack:
  added: []  # no new crate — only an existing fn was PROMOTED and walkdir/owo-colors reused (D-12)
  patterns:
    - "WalkDir min_depth(1).max_depth(1).follow_links(false).filter_entry(!is_hidden) per directory level — reuses the shared hidden predicate verbatim while giving per-level is-last control for box-drawing prefixes"
    - "Box-drawing prefix accumulation: pass a continuation `prefix` string down the recursion, appending `│   ` (non-last ancestor) or `    ` gap (last ancestor); branch is `└── ` (last) vs `├── ` (non-last)"
    - "Single styled token gated on is_color_on() (dir names .blue().bold()) so piped output is byte-identical minus ANSI (D-10)"

key-files:
  created:
    - "src/commands/tree/mod.rs"
    - "tests/tree.rs"
    - "tests/cmd/tree.trycmd"
    - "tests/cmd/tree.in/ (trycmd input fixture: project/{README.md, src/{lib.rs,main.rs}})"
  modified:
    - "src/core/output.rs"
    - "src/commands/flatten/mod.rs"
    - "src/cli.rs"
    - "src/main.rs"
    - "src/commands/mod.rs"

key-decisions:
  - "human_size moved VERBATIM into core::output (made pub) and its human_size_scales test migrated with it; flatten re-pointed to the import, local copy + test deleted (no dead code) — zero behavior change, D-12"
  - "tree recursion uses WalkDir depth-1 children (not std::fs::read_dir) specifically so core::fs::is_hidden — which takes a walkdir::DirEntry — is reused VERBATIM rather than re-implemented (D-06)"
  - "trycmd transcript backed by a `tree.in/` per-case input fixture (trycmd 1.2 sandbox convention) so `box tree project` has a stable, deterministic input tree across machines; fixture files written with explicit byte counts and no newline so on-disk sizes are CRLF-independent"
  - "Root label printed as the path the user passed (self.path), not the canonical absolute path, so the render reads naturally (GNU tree convention) and the trycmd snapshot is stable"
  - "Summary uses the literal `N directories, M files` wording (GNU tree style, plural-always) — TREE-01's count-summary contract; counts every shown dir/file (depth-capped subtrees are not counted, matching the displayed tree)"

patterns-established:
  - "Per-level recursive walk via WalkDir depth-1 + filter_entry(!is_hidden): the reuse-the-shared-predicate alternative to std::fs::read_dir recursion when you need is-last/depth control AND the D-06 hidden prune"
  - "human_size is now the single shared size formatter in core::output (du will call it next, D-12)"

requirements-completed: [TREE-01]

# Metrics
duration: 4min
completed: 2026-06-22
---

# Phase 3 Plan 02: tree Summary

**Live `box tree`: a depth-first, directories-first Unicode box-drawing tree render with `is_color_on()`-gated blue directory names, an optional per-file `--sizes` column, a `--depth N` cap, and an `N directories, M files` summary — plus the D-12 promotion of flatten's `human_size` into `core::output` as the shared size formatter for tree and du.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-22T20:19:13Z
- **Completed:** 2026-06-22T20:23:29Z
- **Tasks:** 2 (Task 1 = shared-core promotion + Wave-0 RED tests; Task 2 = GREEN implementation)
- **Files modified:** 9 (4 created — counting the trycmd `tree.in/` fixture as one — 5 modified)

## Accomplishments
- `box tree` is fully live: it renders a deterministic, dir-first (D-08) Unicode box-drawing tree (`├── └── │   ` + gap), colors only directory names `.blue().bold()` gated on `is_color_on()` (D-10), shows per-file `human_size` under `--sizes` (blank for dirs), caps the displayed depth with `--depth N`, and prints the `N directories, M files` summary — all 3 TREE-01 integration tests + the byte-exact `tree.trycmd` transcript green.
- Promoted flatten's tested `human_size` into `core::output` (made `pub`, migrated its unit test), re-pointed flatten's one call site, and deleted the local copy — so tree (now) and du (Plan 03-03) share one 1024-based formatter with zero Cargo.toml change (D-12). Flatten's 8 integration tests stayed green, confirming the promotion was behavior-preserving.
- Reused `core::fs::is_hidden` VERBATIM (via a `WalkDir` depth-1 per-level walk) and `follow_links(false)` so the dotted-root exemption (walkdir#142) and symlink-loop safety (T-03-05) come for free — never re-implemented (D-06).
- Wired the stub: `cli.rs` `Tree` now carries `TreeArgs`, `main.rs` dispatches `Commands::Tree(args) => args.run()`, and `pub mod tree;` is registered — the `not_implemented("tree")` arm is gone (3 phase-3 stubs remain: du/dupes/bulk-rename).

## Task Commits

Each task was committed atomically:

1. **Task 1: Promote human_size to core::output + Wave-0 tree tests** - `5c14e5e` (test) — the RED gate (3 tree tests fail against the stub; the promotion itself is behavior-preserving)
2. **Task 2: tree command — dir-first box-drawing render + --sizes/--depth + summary + wiring** - `5fb88a1` (feat) — the GREEN gate (all tree tests + trycmd green)

**Plan metadata:** (final docs commit — this SUMMARY + STATE + ROADMAP + REQUIREMENTS)

_TDD-style gate sequence in git log: `test(03-02)` (RED) → `feat(03-02)` (GREEN). No REFACTOR commit needed — the GREEN implementation was already clippy `-D warnings` + `fmt --check` clean._

## Files Created/Modified
- `src/commands/tree/mod.rs` (created) - `TreeArgs` (`path`/`--sizes`/`--depth`) + `RunCommand` impl; depth-first `render_dir` with the four box-drawing constants + prefix accumulation; `read_children` (WalkDir depth-1 + `is_hidden` + size) + `sort_children` (D-08 dir-first) + `color_dir` (is_color_on gate); co-located unit tests
- `tests/tree.rs` (created) - 3 TREE-01 integration tests (`tree_sizes_and_depth`, `tree_count_summary`, `tree_piped_no_ansi`) with a fixed known-byte-size fixture, `NO_COLOR=1`
- `tests/cmd/tree.trycmd` (created) - byte-exact `box tree project` box-drawing transcript (forward-slash paths)
- `tests/cmd/tree.in/` (created) - trycmd per-case input fixture (`project/{README.md, src/{lib.rs, main.rs}}`) giving the transcript a stable input tree
- `src/core/output.rs` (modified) - added `pub fn human_size` + migrated `human_size_scales` test (D-12)
- `src/commands/flatten/mod.rs` (modified) - import + call site re-pointed to `core::output::human_size`; local `human_size` fn and its in-module test removed (no dead code)
- `src/cli.rs` (modified) - `Tree` variant now carries `TreeArgs` (was a unit stub)
- `src/main.rs` (modified) - `Commands::Tree(args) => args.run()` (removed the `not_implemented("tree")` arm)
- `src/commands/mod.rs` (modified) - registered `pub mod tree;`

## Decisions Made
- **`human_size` promotion is verbatim + behavior-preserving:** moved the exact 1024-based fn (B/KB/MB/GB/TB) into `core::output`, made it `pub`, migrated its test, and re-pointed flatten — no `humansize` crate, no Cargo.toml change (D-12). Verified flatten's 8 integration tests stay green.
- **WalkDir-depth-1 for children, not `std::fs::read_dir`:** `core::fs::is_hidden` takes a `walkdir::DirEntry`, so enumerating each directory's immediate children via `WalkDir::new(dir).min_depth(1).max_depth(1).follow_links(false).filter_entry(!is_hidden)` reuses the shared predicate VERBATIM (root exemption + Windows hidden-attr + symlink no-follow all inherited) instead of re-deriving it (D-06, RESEARCH Pitfall: never re-implement is_hidden).
- **trycmd input fixture (`tree.in/`):** trycmd 1.2 copies a per-case `<name>.in/` dir into a sandbox and runs the case there, so `box tree project` gets a stable, checked-in input tree. Fixture files are written with explicit byte content and no trailing newline, so their sizes are independent of git's CRLF normalization.
- **Root label = passed path:** `box tree src/core` prints the header `src/core` (the argument), not the dunce-canonicalized absolute path — natural GNU-tree-style output and a stable snapshot. `normalize_path` is still applied for the actual walk (T-03-07, dunce — no `\\?\` leak).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `cargo fmt` reflowed `tests/tree.rs` (committed in Task 1) — applied in Task 2**
- **Found during:** Task 2 verification (`cargo fmt --check`)
- **Issue:** The Wave-0 `tests/tree.rs` authored in Task 1 had `.success().stdout(...)` chains that rustfmt wanted to reflow; `cargo fmt --check` (a Task 2 acceptance gate) failed against the already-committed file.
- **Fix:** Ran `cargo fmt` (idempotent reformat, no logic change) and included the reformatted `tests/tree.rs` in the Task 2 commit. The tests assert identically before and after.
- **Files modified:** tests/tree.rs
- **Verification:** `cargo fmt --check` exits 0; `cargo test --test tree` still 3/3 green.
- **Committed in:** `5fb88a1` (Task 2 commit)

### Process note (not a code deviation)

- **`--lib` verify substitution (carried from 03-01):** the plan's Task-1 `<automated>` verify uses `cargo test --lib core::output`, but `box` is a binary-only crate (no `[lib]`), so `--lib` errors. Ran the equivalent `cargo test --bin box core::output` (8/8 green) — same in-module tests, only the invocation differs. This was already recorded as the binary-crate note in STATE.md [03-01]; no code change.

---

**Total deviations:** 1 auto-fixed (Rule 3 - blocking; a mechanical `cargo fmt` reflow of a Task-1 test file to satisfy the Task-2 fmt gate).
**Impact on plan:** Cosmetic only — no logic, scope, or design change. The promotion and the render match the plan exactly.

## Issues Encountered
- None. The RED tree tests failed cleanly against the unit-`Tree` stub ("unexpected argument" for the path, since the stub variant took no positional), then went green once `TreeArgs` + `RunCommand` landed. The trycmd transcript passed byte-exact on the first run because the render was authored to the fixture's known dir-first order.

## User Setup Required
None - no external service configuration required. `box tree` works offline against the local filesystem.

## Next Phase Readiness
- `core::output::human_size` is now the shared size formatter ready for `du` (Plan 03-03, D-12) — du will call it for its size column with no further promotion.
- The WalkDir-depth-1 + `is_hidden` per-level walk and the deterministic dir-first sort are available as the read-only-walker pattern for du/dupes.
- ROADMAP Phase-3 success criterion #2 (the `box tree` contract) is met. No blockers introduced. 3 phase-3 stubs remain: du, dupes, bulk-rename.

## Self-Check: PASSED

- FOUND: src/commands/tree/mod.rs
- FOUND: tests/tree.rs
- FOUND: tests/cmd/tree.trycmd
- FOUND: .planning/phases/03-filesystem-power-tools/03-02-SUMMARY.md
- FOUND commit: 5c14e5e (Task 1, RED)
- FOUND commit: 5fb88a1 (Task 2, GREEN)

---
*Phase: 03-filesystem-power-tools*
*Completed: 2026-06-22*
