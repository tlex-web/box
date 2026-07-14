---
phase: 10-fun-system-depth
plan: 04
subsystem: infra
tags: [config, toml, serde, cache, blake3, weather, hash, tdd]

# Dependency graph
requires:
  - phase: 06-scriptable-core-foundation
    provides: flat config::Config (default_hash_algo) + SPINE-05 tolerance + resolve_algo resolver
  - phase: 07-spine-rollout
    provides: weather --json WeatherOutput + config-precedence chain in hash::run_compute
provides:
  - Nested config schema — Config { hash: HashConfig, weather: WeatherConfig } with [hash] default_algo and [weather] location/units
  - weather::Units made config-deserializable (serde lowercase, PartialEq/Eq) mirroring hash::Algo
  - New core::cache module — best-effort, hashed-key, TTL-bounded, miss-tolerant response store
affects: [10-05, phase-11-config-get-set]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Nested TOML config tables via per-command sub-structs, each #[serde(default, deny_unknown_fields)], all leaf fields Option<T>"
    - "Best-effort file cache: every read failure/staleness is a MISS (None), every write failure a silent no-op — mirrors config tolerance but NEVER errors"
    - "Path-traversal-safe cache filename = blake3-hex(key).json — raw user key never interpolated into the path"
    - "Env-var-FIRST cache/config dir resolution (LOCALAPPDATA / APPDATA) for per-process test isolation"

key-files:
  created:
    - src/core/cache.rs
  modified:
    - src/core/config.rs
    - src/core/mod.rs
    - src/commands/hash/mod.rs
    - src/commands/weather/mod.rs
    - tests/config.rs

key-decisions:
  - "D-13 nested config: migrated flat default_hash_algo -> [hash] default_algo; accepted the one-time break to hand-authored flat config.toml (a stray top-level key is now exit 2)"
  - "Typed Option<Units> in WeatherConfig makes an invalid [weather] units value a loud exit-2 config error, not a silently-ignored string"
  - "Cache is strictly best-effort (never Result) — a broken/absent/stale/malformed entry degrades to a fresh fetch, unlike config's malformed->exit-2"
  - "Cache filename hashes the logical key (blake3 hex) so a hostile location string cannot escape %LOCALAPPDATA%\\box\\cache (T-10-04-TRAVERSAL)"

patterns-established:
  - "Nested config sub-struct table (HashConfig/WeatherConfig) — the growth shape Phase 11 config get/set locks against"
  - "core::cache best-effort store — the analog for any future on-disk cache"

requirements-completed: []  # WTHR-V2-01 is foundation-only here; 10-05 consumes cache+config and completes the requirement

# Metrics
duration: 15min
completed: 2026-07-14
---

# Phase 10 Plan 04: Config Migration + Response Cache Foundation Summary

**D-13 nested TOML config ([hash] default_algo, [weather] location/units) with config-deserializable weather Units, plus a new best-effort blake3-keyed core::cache module — the shared foundation 10-05 wires into weather.**

## Performance

- **Duration:** 15 min (~907 s)
- **Started:** 2026-07-14T14:08:59Z
- **Completed:** 2026-07-14T14:24:06Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 6 (1 created, 5 modified)

## Accomplishments
- Migrated `core::config` from the flat `default_hash_algo` key to nested per-command tables (`[hash] default_algo`, `[weather] location`/`units`), preserving every SPINE-05 tolerance behavior (missing = silent default, malformed/unknown-key/invalid-enum = exit 2).
- Made `weather::Units` config-deserializable (`serde::Deserialize` + `#[serde(rename_all = "lowercase")]` + `PartialEq, Eq`), mirroring `hash::Algo` — the only weather edit in this plan, deliberately scoped so `[weather] units` can win through the config resolver in 10-05.
- Migrated the live hash consumer to `config().hash.default_algo`; verified NO remaining flat-key read.
- Built `src/core/cache.rs`: `get(key) -> Option<String>` / `put(key, payload)` best-effort store with a blake3-hashed filename (path-traversal mitigation), `%LOCALAPPDATA%`-first cache dir, a ~600 s TTL, a pure `is_fresh` boundary helper, and total miss/error tolerance.

## Task Commits

Each task was executed TDD (RED test → GREEN feat):

1. **Task 1: nested Config + hash consumer + config-ready weather Units (D-13)**
   - `47b2412` (test) — RED nested-config unit + integration tests
   - `adfc97e` (feat) — GREEN nested `Config`/`HashConfig`/`WeatherConfig`, hash migration, `Units` serde derive
2. **Task 2: new core::cache module (D-11)**
   - `d440e6b` (test) — RED cache contract (round-trip, stale/malformed/absent MISS, is_fresh boundary, traversal containment)
   - `0f5bd9f` (feat) — GREEN `get`/`put`/`is_fresh`/`cache_dir`/`entry_path` implementation

_Plan metadata (this SUMMARY) committed separately per worktree protocol._

## Files Created/Modified
- `src/core/cache.rs` (created) - Best-effort weather response cache: hashed-key filename, TTL, miss-tolerant `get`/`put`, pure `is_fresh`.
- `src/core/config.rs` - Flat `Config` → nested `Config { hash: HashConfig, weather: WeatherConfig }`; imports `weather::Units`; `deny_unknown_fields` + `Option<T>` leaves + tolerance preserved.
- `src/core/mod.rs` - Registered `pub mod cache;`.
- `src/commands/hash/mod.rs` - Compute-default chain reads `config().hash.default_algo` (was `config().default_hash_algo`).
- `src/commands/weather/mod.rs` - `Units` gains `PartialEq, Eq, serde::Deserialize` + `#[serde(rename_all = "lowercase")]` (ONLY edit; forecast/cache-wiring is 10-05).
- `tests/config.rs` - Migrated `hash_default_override` to `[hash]` form; added `valid_weather_config_is_silent` + `unknown_units_exit2`.

## Decisions Made
- Kept `WeatherConfig.units` a typed `Option<Units>` (not a raw string) so a bad value (`kelvin`) is a loud exit-2 config error via `deny_unknown_fields` + the enum's serde rejection.
- Nested sub-structs each carry `#[derive(Default)]` + `#[serde(default, deny_unknown_fields)]` so a whole absent table → all-`None` (silent), while an unknown key nested inside a known table is still rejected.
- Cache `get`/`put` return `Option`/`()` (never `Result`) — the contract 10-05 consumes; a cache problem must never fail a working command.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Module-scoped `#![allow(dead_code)]` on core::cache**
- **Found during:** Task 2 (GREEN)
- **Issue:** `core::cache` is foundation with NO live caller until 10-05 wires it into weather, so the bin build emitted 8 dead-code warnings (`get`/`put`/helpers/`Envelope`/`TTL_SECS` unused), which fail the plan's `cargo clippy --all-targets -- -D warnings` gate.
- **Fix:** Added a module-level `#![allow(dead_code)]` with an explanatory comment, mirroring the existing forward-compat precedent in `config.rs` (`#[allow(dead_code)]` on `resolve_algo` before hash adopted it — "allow-then-remove"). 10-05 removes it when the last item becomes reachable.
- **Files modified:** src/core/cache.rs
- **Verification:** `cargo build` and `cargo clippy --all-targets -- -D warnings` both clean; 6 cache tests still pass.
- **Committed in:** `0f5bd9f` (Task 2 GREEN commit)

---

**Total deviations:** 1 auto-fixed (1 blocking).
**Impact on plan:** Necessary to satisfy the plan's own clippy gate for a foundation module with no live caller yet; follows an established in-repo pattern. No scope creep.

## Issues Encountered
- **Stale worktree baseline (setup-level, resolved).** The worktree branch `worktree-agent-a0f6e3ca71bcb0f52` was created from commit `986c841` (end of Phase 06) rather than main's tip `d2802e3`, so it lacked all Phase 07–09 source AND the Phase 10 plan files (the plan file did not exist in the worktree). HEAD was a strict ancestor of main with zero unique commits and a clean working tree, so I fast-forwarded the worktree branch to main (`git merge --ff-only main`) — a content-lossless pure fast-forward, not a reset/clean — to obtain the correct baseline. No prior-wave work was touched. Flagging for orchestrator awareness in case other Wave-1 worktrees were spawned from the same stale point.

## Verification Evidence
- `cargo test --bin box config` → 4 passed; `cargo test --bin box cache` → 6 passed; `cargo test --test config` → 5 passed.
- `cargo clippy --all-targets -- -D warnings` → clean.
- `grep "config().default_hash_algo" src/` → none (flat read fully migrated).
- Full `cargo test` suite → 0 failures across all bin + integration targets.
- Tolerance preserved: `missing_is_silent` (exit 0, no stderr), `malformed_exit2` + `unknown_units_exit2` (exit 2, `error:` on stderr).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- **10-05 (Wave 2) unblocked:** nested `[weather] location`/`units` config schema and config-deserializable `Units` are ready to resolve through `cli.or(config)`; `core::cache::{get,put}` is ready to wire around weather's `fetch`. Remove `core::cache`'s module-level `#![allow(dead_code)]` once the first live caller lands.
- No blockers. Contract surface (`get`/`put` signatures, envelope shape, TTL) is exactly as the plan's interfaces block published.

## Self-Check: PASSED

All 7 claimed files exist on disk; all 4 task commits (`47b2412`, `adfc97e`, `d440e6b`, `0f5bd9f`) present in git history.

---
*Phase: 10-fun-system-depth*
*Completed: 2026-07-14*
