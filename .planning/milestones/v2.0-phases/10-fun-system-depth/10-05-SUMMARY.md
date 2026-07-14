---
phase: 10-fun-system-depth
plan: 05
subsystem: api
tags: [weather, open-meteo, ureq, serde, cache, config, forecast, tdd]

# Dependency graph
requires:
  - phase: 10-fun-system-depth (plan 04)
    provides: nested [weather] location/units config + config-deserializable Units + core::cache get/put
  - phase: 07-spine-rollout
    provides: weather --json WeatherOutput + current_units authoritative-label rule + fetch()/build_forecast_url seams
provides:
  - weather --forecast — a 7-day daily outlook (date/min-max temp/conditions) additive to the current block, with a forecast:[…] --json array
  - Optional location resolution — CLI positional > [weather] location config > exit-2 MissingLocation
  - Units resolution — CLI --units > [weather] units config > metric builtin (SPINE-05 shape)
  - Transparent response cache — CachedWeather projection stored via core::cache, keyed by (location, units, forecast); a hit skips ALL network
  - BoxError::MissingLocation (exit 2) + its main() downcast
affects: [phase-11-config-get-set]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cache the RENDERED projection (CachedWeather), not the raw API JSON — a hit skips geocode+fetch and shares one render path with a miss"
    - "Optional CLI arg + config fallback + typed exit-2 error for a missing required value (Option<String> positional → resolve_location → MissingLocation)"
    - "Pure resolver helpers (resolve_location/resolve_units) unit-tested as a known-answer precedence matrix, mirroring config::resolve_algo"

key-files:
  created:
    - tests/fixtures/weather/forecast_metric_7day.json
    - tests/fixtures/weather/forecast_imperial_7day.json
  modified:
    - src/commands/weather/mod.rs
    - src/core/errors.rs
    - src/main.rs
    - src/core/cache.rs
    - src/cli.rs
    - tests/weather.rs

key-decisions:
  - "Cache a small CachedWeather projection (label/coords + resolved values) rather than the raw Open-Meteo JSON, so a cache hit needs NO geocode and the render path is identical for hit and miss"
  - "One URL builder with a forecast bool (not a second function) — the daily block rides alongside the existing current block; the imperial temperature_unit param covers both current and daily temps"
  - "DailyUnits captures only temperature_2m_max (the daily min shares its unit); no deny_unknown_fields means the extra JSON label is harmlessly ignored — no field-level allow(dead_code)"
  - "New BoxError::MissingLocation (exit 2) rather than reusing MissingInput — a distinct, self-describing usage error pointing at weather.location"

patterns-established:
  - "Response-cache projection: serialize the resolved render values so a hit is byte-identical to a miss and skips upstream resolution (geocode)"
  - "Cache-hit integration probe: a one-shot fixture server (spawn_fixture_server_n(body,1)) proves the second identical call took no network GET"

requirements-completed: [WTHR-V2-01]

# Metrics
duration: 17min
completed: 2026-07-14
---

# Phase 10 Plan 05: Weather Depth (--forecast + response cache + config-default location) Summary

**`box weather --forecast` renders a 7-day daily outlook (additive to current conditions, with a `forecast:[{date,temp_min,temp_max,conditions}]` --json array), an optional location resolves CLI > `[weather] location` config > exit-2, and a transparent `core::cache` projection serves repeat calls with no network — completing WTHR-V2-01.**

## Performance

- **Duration:** ~17 min
- **Started:** 2026-07-14T14:41:00Z
- **Completed:** 2026-07-14T14:58:00Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 8 (2 created, 6 modified)

## Accomplishments
- **`--forecast` 7-day daily outlook (D-10):** a new daily struct pair (`Daily`/`DailyUnits`) requested via `&daily=temperature_2m_max,temperature_2m_min,weather_code`, projected by `build_day_forecasts` into a bounded 7-day span (a short/mismatched daily array is a clean exit-1 error, never a panic — T-10-05-HTTP). Daily temp labels are read from `daily_units` (authoritative, never hardcoded). The `--json` `forecast` array is `Option` + `skip_serializing_if`, so the current-only shape is byte-unchanged.
- **Optional location + config default (D-12):** `WeatherArgs.location` is now `Option<String>` and `units` is `Option<Units>` (clap default dropped). `resolve_location` (CLI > config) and `resolve_units` (CLI > config > metric) are pure, unit-tested resolvers. Bare `box weather` with no positional and no config location exits 2 with a `weather.location` hint via the new `BoxError::MissingLocation`.
- **Transparent response cache (D-11):** `run()` builds a logical key `(location|units|forecast)`, calls `cache::get` FIRST, and on a hit deserializes the stored `CachedWeather` projection and renders WITHOUT any network call; a miss/stale/malformed entry transparently fetches fresh and `cache::put`s the projection. The cache never fails the command.
- **JSON purity preserved:** the resolved-location echo stays stderr-only and the cache adds nothing to stdout; the existing `json_purity` harness (no `forecast`/`daily`/`hourly` key on the non-forecast path) still passes.
- Removed `core::cache`'s module-scoped `#![allow(dead_code)]` — `weather::run` is now its live caller (the "allow-then-remove" hand-off 10-04 flagged).

## Task Commits

Each task was executed TDD (RED test → GREEN feat), plus one refactor:

1. **Task 1: --forecast 7-day daily outlook + forecast JSON array (D-10)**
   - `e3f42b0` (test) — RED `--forecast --json` 7-day integration test + offline metric/imperial 7-day fixtures
   - `f221caa` (feat) — GREEN `--forecast` flag, `Daily`/`DailyUnits`, `build_day_forecasts`, `WeatherOutput.forecast`, daily human section
2. **Task 2: optional location + config default + units resolver + response cache (D-11/D-12)**
   - `443ed63` (test) — RED config-location, missing-location→exit2, cache-hit integration tests + one-shot server helper
   - `b177e81` (feat) — GREEN `Option` location/units, `resolve_location`/`resolve_units`, `BoxError::MissingLocation`, `CachedWeather` cache wiring, cache allow removal, cli help
   - `42664cd` (refactor) — drop the unused `DailyUnits.temperature_2m_min` field (serde ignores the extra JSON label)

_Plan metadata (this SUMMARY) committed separately per worktree protocol._

## Files Created/Modified
- `src/commands/weather/mod.rs` — `--forecast` flag; `Daily`/`DailyUnits`/`DayForecast`/`CachedWeather` structs; `build_day_forecasts`; `resolve_location`/`resolve_units`/`units_key`/`fetch_weather`/`render` helpers; cache-first `run()`; `WeatherOutput.forecast`.
- `src/core/errors.rs` — new `BoxError::MissingLocation` variant (exit-2 usage error).
- `src/main.rs` — added `MissingLocation` to the exit-2 downcast arm.
- `src/core/cache.rs` — removed the module-scoped `#![allow(dead_code)]` (weather is now the live caller).
- `src/cli.rs` — weather help reflects optional `[LOCATION]`, `--forecast`, config default, and ~10 min caching.
- `tests/weather.rs` — `forecast_json_has_7_day_array`, `bare_weather_uses_config_location`, `bare_weather_no_location_exits_2`, `second_identical_call_is_a_cache_hit`; `spawn_fixture_server_n` + `appdata_with_config` helpers.
- `tests/fixtures/weather/forecast_metric_7day.json`, `forecast_imperial_7day.json` (created) — offline current + 7-entry daily/daily_units fixtures.

## Decisions Made
- **Projection cache over raw JSON:** caching `CachedWeather` (resolved label/coords + render values) means a hit skips geocoding and reuses one `render()` for both hit and miss. A stored payload that no longer matches the shape (schema drift) is treated as a MISS via `serde_json::from_str(...).ok()`.
- **Single URL builder with a `forecast: bool`** rather than a second function — keeps the current+daily request in one place; the imperial `temperature_unit` param already applies to both blocks.
- **New `MissingLocation` variant** (not reusing `MissingInput`) for a self-describing exit-2 message pointing at `weather.location`.
- **`DailyUnits` keeps only `temperature_2m_max`** — the daily min shares the unit and the struct has no `deny_unknown_fields`, so the extra label is ignored without an `allow(dead_code)`.

## Deviations from Plan

None - plan executed as written. The plan explicitly allowed "raw JSON or a small serialized projection" for the cache payload; the projection was chosen (documented above). The `42664cd` refactor is the TDD refactor phase, not an unplanned deviation.

## Issues Encountered
None. The stale-worktree base was aligned to current `main` via the sanctioned `git merge --ff-only main` before any code was written (10-04's foundation — `core::cache`, nested `[weather]` config — verified present).

## User Setup Required
None - no external service configuration required. The response cache lives at `%LOCALAPPDATA%\box\cache\` and is created on demand; `[weather] location`/`units` config is optional.

## Next Phase Readiness
- **WTHR-V2-01 complete:** `--forecast`, response cache, and config-default location are all live and integration-tested offline.
- **Phase 11 (`box config get/set`)** locks against the now-consumed `[weather] location`/`units` schema; nothing here blocks it.
- Verification: `cargo test --bin box weather` (17), `cargo test --test weather` (7), `cargo test --test config` (5), full `cargo test` suite (0 failures), and `cargo clippy --all-targets -- -D warnings` all green.

## Self-Check: PASSED

All 7 claimed files exist on disk; all 5 task commits (`e3f42b0`, `f221caa`, `443ed63`, `b177e81`, `42664cd`) are present in git history.

---
*Phase: 10-fun-system-depth*
*Completed: 2026-07-14*
