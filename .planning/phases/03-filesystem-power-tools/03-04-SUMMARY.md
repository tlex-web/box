---
phase: 03-filesystem-power-tools
plan: 04
subsystem: cli-command
tags: [dupes, duplicates, blake3, rayon, parallel, content-hash, size-prefilter, wasted-space, read-only, determinism]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: "RunCommand trait + main() dispatch/exit-code policy; core::fs::is_hidden walk filter + normalize_path (dunce); core::output color gate (is_color_on); the cli.rs/main.rs stub-swap precedent"
  - phase: 03-filesystem-power-tools (plan 01)
    provides: "the BLAKE3 native streaming content-equality path (blake3::Hasher::update_reader) reused for duplicate identity (D-13); the binary-only-crate test-targeting note (--bin box, not --lib)"
  - phase: 03-filesystem-power-tools (plan 02/03)
    provides: "core::output::human_size promoted to pub (D-12) — dupes is its FOURTH consumer (flatten, tree, du, dupes); the collect → sort-before-printing determinism discipline (RESEARCH Pitfall 6) du established for walk/parallel-order-arbitrary output"
provides:
  - "Live `box dupes [PATH]` command: size pre-filter (HashMap<u64, Vec<PathBuf>>, only same-size buckets of >=2 are candidates — most files never hashed) then rayon-parallel BLAKE3 content hash, deterministic sorted duplicate groups + a wasted-space summary, STRICTLY read-only"
  - "The first rayon parallel-content-hash pattern in the repo (par_iter over same-size candidates + collect::<anyhow::Result<Vec<_>>> short-circuit + sort (hash, path) before grouping) — reusable by any future content-equality command"
affects: [bulk-rename, any-future-content-hash-or-parallel-walk-command]

# Tech tracking
tech-stack:
  added: []  # no new crate — rayon (1.12.0) + blake3 (1.8.5) already in Cargo.toml (T-03-SC, no install-time checkpoint); reuses the promoted core::output::human_size (D-12)
  patterns:
    - "Size-pre-filter-then-content-hash duplicate identity: bucket by metadata().len() first, hash ONLY same-size buckets of >=2 (D-13) — unique-size files are never hashed"
    - "rayon parallel content hash: candidates.par_iter().map(|p| Ok((hash(p)?, ...))).collect::<anyhow::Result<Vec<_>>>()? — the first hash error short-circuits to a clean anyhow error (exit 1, no panic, T-03-17)"
    - "Determinism for parallel-order-arbitrary output: collect → sort_by (hash, path) BEFORE grouping; group consecutive-equal-hash runs, emit only groups of >=2 (RESEARCH Pitfall 6 / T-03-16)"

key-files:
  created:
    - "src/commands/dupes/mod.rs"
    - "tests/dupes.rs"
  modified:
    - "src/cli.rs"
    - "src/main.rs"
    - "src/commands/mod.rs"

key-decisions:
  - "Reused the hash slice's BLAKE3 update_reader native streaming path for content equality (D-13), but LIFTED the few-line core (hash_reader_blake3) into dupes rather than widening the hash module's surface — hash::hash_blake3 is private, and the plan's <interfaces> note explicitly sanctioned lifting the snippet over making it pub. Unit-tested against the SAME b\"box\" known vector so the two paths provably agree"
  - "dupes is strictly READ-ONLY (T-03-13): the ONLY filesystem handle is a read-only std::fs::File::open for hashing — no safe_copy/File::create/rename/delete. The dupes_never_writes test snapshots the fixture's file set + contents + mtimes and asserts byte-for-byte unchanged after a run"
  - "Determinism by sort_by((hash, path)) BEFORE grouping (consecutive-equal-hash runs >=2 form a group; RESEARCH Pitfall 6 / T-03-16); the integration tests use distinct-content groups so the order is total and a second run is asserted byte-identical"
  - "Wasted space = sum over groups of (group_len - 1) * file_size, rendered with core::output::human_size (D-12, fourth consumer after flatten/tree/du); the size is captured from the size bucket so no extra metadata call is needed"

patterns-established:
  - "First rayon usage in the repo: par_iter over the same-size candidate set + collect into anyhow::Result (first-error short-circuit) + sort before grouping — the net-new parallel capability with no prior in-repo analog (RESEARCH Pattern 4)"
  - "Read-only walk command: collect_by_size reuses core::fs::is_hidden + follow_links(false) VERBATIM (D-06/D-07, no noise list / no ignore crate); files-only filter; never any write/rename/delete path"

requirements-completed: [DUPE-01]

# Metrics
duration: 4min
completed: 2026-06-22
---

# Phase 3 Plan 04: dupes Summary

**Live `box dupes`: a content-duplicate finder that buckets files by byte size first (so most unique files are never hashed), content-hashes each same-size candidate group in PARALLEL with rayon (reusing the `hash` slice's BLAKE3 `update_reader` streaming path), sorts `(hash, path)` before grouping for deterministic output, emits each group of identical files plus a wasted-space summary via the shared `human_size`, and never mutates the filesystem (D-13, D-06/D-07, T-03-13).**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-22T20:36:55Z
- **Completed:** 2026-06-22 (Wave 4)
- **Tasks:** 2 (Task 1 = Wave-0 RED tests; Task 2 = GREEN implementation + wiring)
- **Files modified:** 5 (2 created — `src/commands/dupes/mod.rs`, `tests/dupes.rs`; 3 modified: cli.rs, main.rs, commands/mod.rs)

## Accomplishments
- `box dupes` is fully live: it walks the target read-only (hidden pruned via `core::fs::is_hidden`, symlinks not followed, NO noise list / NO `ignore` crate, D-06/D-07), buckets every regular file by `metadata().len()`, takes only same-size buckets of `>= 2` as candidates (unique-size files are never hashed), content-hashes those candidates IN PARALLEL with `rayon::par_iter` (reusing the 03-01 BLAKE3 `update_reader` streaming path for content equality, D-13), sorts the `(hash, path)` pairs BEFORE grouping (RESEARCH Pitfall 6 / T-03-16), emits each duplicate group (the identical files) and a `{N} wasted in {M} duplicate group(s).` summary computed as Σ `(group_len - 1) * file_size` via `core::output::human_size` — all 4 DUPE-01 integration tests + 6 co-located unit tests green.
- Strictly READ-ONLY (T-03-13): the only filesystem handle in the whole command is a read-only `std::fs::File::open` for hashing; there is no `safe_copy`/`File::create`/rename/delete anywhere. The `dupes_never_writes` test snapshots the fixture's file set + contents + mtimes and asserts they are byte-for-byte unchanged after a run.
- Introduced the first rayon parallel-content-hash pattern in the repo: `candidates.par_iter().map(|p| Ok((hash(p)?, ...))).collect::<anyhow::Result<Vec<_>>>()?` — the first hash error short-circuits the collect to a clean `anyhow` error (exit 1, never a panic, T-03-17), then a `sort_by((hash, path))` restores determinism before grouping.
- Reused `core::output::human_size` (the D-12 promotion) for the wasted-space figure with ZERO Cargo.toml change — dupes is now its fourth consumer (flatten, tree, du, dupes). `rayon` (1.12.0) and `blake3` (1.8.5) were already present (T-03-SC, no install-time checkpoint).
- Wired the stub: `cli.rs` `Dupes` now carries `DupesArgs`, `main.rs` dispatches `Commands::Dupes(args) => args.run()`, and `pub mod dupes;` is registered — the `not_implemented("dupes")` arm is gone (1 phase-3 stub remains: bulk-rename).

## Task Commits

Each task was committed atomically:

1. **Task 1: Wave-0 dupes tests (incl. never-writes invariant)** — `932ea13` (test) — the RED gate (4 dupes tests fail against the unit `Dupes` stub, which takes no positional path → "unexpected argument", exit 2)
2. **Task 2: dupes command — size pre-filter + rayon BLAKE3 hash + deterministic groups + wiring** — `6d1f92d` (feat) — the GREEN gate (all dupes tests + 6 unit tests + clippy `-D warnings` + fmt clean)

**Plan metadata:** (final docs commit — this SUMMARY + STATE + ROADMAP + REQUIREMENTS)

_TDD-style gate sequence in git log: `test(03-04)` (RED) → `feat(03-04)` (GREEN). No REFACTOR commit needed — the GREEN implementation was already clippy `-D warnings` + `fmt --check` clean._

## Files Created/Modified
- `src/commands/dupes/mod.rs` (created) — `DupesArgs` (`path`, default `.`) + `RunCommand` impl; `collect_by_size` (read-only WalkDir + `is_hidden`, files-only, bucket by `metadata().len()`), `hash_file_blake3` / `hash_reader_blake3` (the lifted BLAKE3 `update_reader` core), `group_duplicates` (consecutive-equal-hash runs >=2 after the sort), `wasted_space` (Σ `(len-1)*size`), `render` (groups + `is_color_on`-gated `.yellow()` accent); co-located unit tests (wasted-space math, group-folding incl. singleton drop, the `b"box"` known vector, same-size/different-content split)
- `tests/dupes.rs` (created) — 4 DUPE-01 integration tests: `dupes_groups_identical`, `dupes_size_then_hash`, `dupes_wasted_space_sorted`, `dupes_never_writes`; `NO_COLOR=1`; the never-writes test snapshots file set + contents + mtimes
- `src/cli.rs` (modified) — `Dupes` variant now carries `DupesArgs` (was a unit stub)
- `src/main.rs` (modified) — `Commands::Dupes(args) => args.run()` (removed the `not_implemented("dupes")` arm)
- `src/commands/mod.rs` (modified) — registered `pub mod dupes;` (alphabetical, after `du`)

## Decisions Made
- **Reused the 03-01 BLAKE3 path, but lifted the snippet (not made it `pub`).** Content equality uses the same native stable `blake3::Hasher::update_reader` streaming path the `hash` command uses (D-13) — BLAKE3 chosen for SPEED, since cryptographic-criticality is irrelevant for equality grouping. Because `hash::hash_blake3` is private and the plan's `<interfaces>` note explicitly sanctioned lifting the few-line snippet rather than widening the `hash` module's surface, `dupes` carries its own `hash_reader_blake3`. It is unit-tested against the SAME `b"box"` known vector (`095dfefd…`) the hash slice locked, so the two paths provably agree.
- **Strictly read-only, asserted by snapshot.** The only filesystem handle is a read-only `std::fs::File::open` for hashing — no `safe_copy`/`File::create`/rename/delete. The `dupes_never_writes` integration test snapshots the fixture's `relative_path -> (bytes, mtime)` map before the run and asserts it is byte-for-byte identical after (T-03-13, locked Out of Scope). A grep of `src/commands/dupes/mod.rs` for any write/mutation API returns only the doc comment that declares the invariant.
- **Determinism by sort-before-group, with distinct-content test fixtures.** rayon completion order is arbitrary, so the `(hash, size, path)` tuples are `sort_by((hash, path))`ed BEFORE folding into groups (consecutive equal hashes form a run; only runs of >=2 are emitted; RESEARCH Pitfall 6 / T-03-16). The `dupes_wasted_space_sorted` test uses distinct-content groups so the order is a total order, and asserts a second run is byte-identical.
- **Wasted space = Σ `(group_len - 1) * file_size`, via the shared `human_size`.** The common file size is carried from the size bucket through the hash phase, so no extra `metadata()` call is needed at render time; the figure is rendered with `core::output::human_size` (D-12, fourth consumer). The single styled token (the size accent) is `.yellow()` gated on `is_color_on()` so `box dupes | cat` is byte-identical minus ANSI (D-10).

## Deviations from Plan

None — plan executed exactly as written.

The only judgement call (lifting the BLAKE3 snippet into `dupes` rather than making `hash::hash_blake3` `pub`) was explicitly anticipated and sanctioned by the plan's `<interfaces>` note ("if it is not `pub`-visible, lift the blake3 streaming snippet … rather than re-deriving the API"), so it is the planned path, not a deviation. The plan's `cargo fmt` only reflowed two long lines after the initial Write (no logic change). Per the carried-over binary-crate note ([03-01]/[03-02]/[03-03]), the co-located unit tests run via `cargo test --bin box dupes::`, NOT `--lib` (process-only, no code change).

## Issues Encountered
- None. All 4 RED tests failed cleanly against the unit `Dupes` stub ("unexpected argument" for the positional path, exit 2 — exactly the du/tree RED-gate shape), then went green immediately once `DupesArgs` + the `RunCommand` impl landed. No auto-fixes were required.

## User Setup Required
None — no external service configuration required. `box dupes` works offline against the local filesystem and is strictly read-only.

## Next Phase Readiness
- The rayon parallel-content-hash pattern (`par_iter` + `collect::<anyhow::Result<Vec<_>>>` short-circuit + sort-before-group) and the read-only size-bucketing walk are now available for any future content-equality command.
- `core::output::human_size` has four live consumers now (flatten, tree, du, dupes); no further promotion needed.
- ROADMAP Phase-3 success criterion #4 (`box dupes ./downloads` shows groups of identical files identified by content hash with a wasted-space summary; no files deleted or modified) is met. No blockers introduced. 1 phase-3 stub remains: bulk-rename (Plan 03-05, the only mutating filesystem command in the phase).

## TDD Gate Compliance
- RED gate: `932ea13` (`test(03-04): ...`) — `tests/dupes.rs` committed failing against the unit `Dupes` stub (4/4 red, exit 2).
- GREEN gate: `6d1f92d` (`feat(03-04): ...`) — implementation lands; all 4 integration + 6 unit tests green.
- REFACTOR: not required (GREEN was already clippy `-D warnings` + `fmt --check` clean).

## Self-Check: PASSED

- FOUND: src/commands/dupes/mod.rs
- FOUND: tests/dupes.rs
- FOUND: .planning/phases/03-filesystem-power-tools/03-04-SUMMARY.md
- FOUND commit: 932ea13 (Task 1, RED)
- FOUND commit: 6d1f92d (Task 2, GREEN)

---
*Phase: 03-filesystem-power-tools*
*Completed: 2026-06-22*
