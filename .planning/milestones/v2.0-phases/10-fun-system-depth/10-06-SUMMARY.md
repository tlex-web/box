---
phase: 10-fun-system-depth
plan: 06
subsystem: system
tags: [weather, open-meteo, cache, forecast, ureq, rust]

# Dependency graph
requires:
  - phase: 10-fun-system-depth (10-05)
    provides: the shipped `weather --forecast` + response-cache feature (WTHR-V2-01) these fixes harden
provides:
  - "WR-01 closed — the bounded 7-day forecast is now ENFORCED: build_forecast_url pins &forecast_days=7 server-side AND build_day_forecasts rejects any daily block whose length is not in 1..=7"
  - "WR-02 closed — the weather location is trimmed once (run) + city-only-lowercased for the cache key (location_key), so whitespace/case variants of the same location share ONE cache entry"
  - "de-overclaimed doc comments: build_forecast_url / build_day_forecasts / Daily now describe the ENFORCED invariant, not an aspiration"
affects: [phase-11 completions, weather]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "location_key(&str) -> String: a units_key-style pure cache-key token helper (trim + gate on parse_lat_lon → lat,lon verbatim | city to_lowercase)"
    - "belt-and-suspenders remote-size bound: server-side forecast_days=7 pin PLUS a defensive n<=7 ensure! gate"

key-files:
  created: []
  modified:
    - src/commands/weather/mod.rs
    - tests/weather.rs

key-decisions:
  - "location_key trims INTERNALLY (idempotent with run's trim) so it is a self-contained pure known-answer helper; run() also trims once so fetch_weather/geocode see the trimmed value"
  - "lat,lon cache-key tokens are trimmed but NEVER lowercased — parse_lat_lon on the fetch path must still see the numeric value verbatim (D-12)"

patterns-established:
  - "Pattern: bound a remote array in BOTH shape and size (n<=7) not just internal consistency (T-10-05-HTTP strengthened)"
  - "Pattern: normalize a user-controlled cache key once at the run() boundary; a pure token helper mirrors units_key for the location component"

requirements-completed: [WTHR-V2-01]

# Metrics
duration: ~13min
completed: 2026-07-14
---

# Phase 10 Plan 06: Weather WR-01/WR-02 Gap-Closure Summary

**The shipped "bounded 7-day forecast" is now enforced (server-side `forecast_days=7` pin + defensive `n<=7` gate), and the response cache actually shares entries across whitespace/case variants of a location (trim + city-only lowercase).**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-07-14T17:59:24+02:00 (first RED commit; initial read ~5 min prior)
- **Completed:** 2026-07-14T18:06:26+02:00
- **Tasks:** 2 (both TDD: RED → GREEN)
- **Files modified:** 2

## Accomplishments
- **WR-01 (bounded 7-day now real):** `build_forecast_url` appends `&forecast_days=7` under `--forecast` so Open-Meteo's runtime default can never drift the span; `build_day_forecasts`'s `ensure!` predicate gained `&& n <= 7` (message now names the "oversized" case), so a >7-day (or anomalously large) well-formed daily block is a clean exit-1 error — never projected, rendered under the `"  7-day forecast:"` header, or serialized into the `--json` `forecast` array.
- **WR-02 (shared cache window):** `run()` rebinds `let location = location.trim().to_string();` once — the single trimmed source for both the cache key AND `fetch_weather` (geocode/parse). A new pure `location_key` helper produces the cache-key location token: trim, then `to_lowercase()` for city names, but a `lat,lon` pair is kept verbatim. `" London "`, `"London"`, and `"london"` now hash to ONE cache key.
- **Docs de-overclaimed:** the three stale "bounded/fixed 7-day" doc comments (`build_forecast_url` inline, `build_day_forecasts`, and the `Daily` struct) now cite the `forecast_days=7` pin plus the `n <= 7` bound — they describe what the code guarantees, not an aspiration.
- **Zero regressions:** full `cargo test` green (235 in-module unit + all integration suites, 0 failures); `cargo clippy --all-targets -D warnings` clean.

## Task Commits

Each task was TDD (RED → GREEN), committed atomically:

1. **Task 1 (WR-01): enforce bounded 7-day forecast**
   - RED — `83615ec` (test): failing `forecast_days=7` URL assertions + an 8-day (matched-length) oversized-daily rejection test
   - GREEN — `69be222` (feat): `&forecast_days=7` pin, `n <= 7` bound + reworded message, de-overclaimed docs
2. **Task 2 (WR-02): normalize the weather location for a shared cache key**
   - RED — `58e2047` (test): failing `location_key_tokens` matrix (identity stub) + `whitespace_variants_share_cache_key` integration test
   - GREEN — `f638159` (feat): `location.trim()` rebind in `run()`, real `location_key` helper (trim + city-only lowercase), wired into `cache_key`

_Plan metadata (this SUMMARY) committed separately after the task commits._

## Files Created/Modified
- `src/commands/weather/mod.rs` — `build_forecast_url` (`forecast_days=7` pin), `build_day_forecasts` (`n <= 7` bound + message), new `location_key` helper, `run()` trim rebind + `cache_key` wiring, three de-overclaimed doc comments, two new in-module tests (`build_day_forecasts_rejects_oversized_arrays`, `location_key_tokens`) + extended `build_forecast_url_daily_param`
- `tests/weather.rs` — new `whitespace_variants_share_cache_key` integration test (mirrors `second_identical_call_is_a_cache_hit`, `spawn_fixture_server_n(FORECAST_METRIC, 1)`)

## Decisions Made
- **`location_key` trims internally** (not just relying on `run()`'s trim) so it is a self-contained, pure, known-answer-testable helper (mirrors `units_key`); `run()`'s own trim is idempotent with it and additionally normalizes the value handed to `fetch_weather`/geocode.
- **`lat,lon` tokens are trimmed but never lowercased** — digits are case-free anyway, but keeping the value verbatim documents that `parse_lat_lon` on the fetch path still sees the exact numeric value (D-12).
- **Belt-and-suspenders for WR-01** — both the server-side `forecast_days=7` pin and the client-side `n <= 7` gate were applied (the reviewer offered either/or); the defensive gate also covers the `BOX_WEATHER_BASE_URL` test-seam / untrusted-origin case where the pin is not honored.

## Deviations from Plan

None - plan executed exactly as written.

The plan's `<verify>` blocks reference the main-repo path `C:/Users/aster/projects-source/cli-tools/box`; because this ran as a worktree executor, all `cargo test`/`clippy` runs were executed from the worktree checkout instead so they exercised the actual edits (an execution-environment adaptation, not a behavioral deviation).

## Issues Encountered
- **cwd-drift false-green (self-caught):** the first Task-1 GREEN test run was accidentally executed from the main-repo path (the plan's literal `<verify>` cwd), which lacked the worktree edits and silently ran stale code (the new oversized test was absent). Detected immediately (test count / missing test), re-run from the worktree path, and confirmed truly green. No bad commit resulted.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Both deferred `10-REVIEW.md` weather warnings (WR-01, WR-02) are resolved; the remaining Phase-10 code-review item CR-01 (hash `default_algo` doc naming) was already closed on `main`.
- No new dependencies, no schema/API-surface changes; the `--json` current-only shape is byte-unchanged.
- Ready for orchestrator phase verification + close-out.

## Self-Check: PASSED

- Files verified present: `src/commands/weather/mod.rs`, `tests/weather.rs`, `.planning/phases/10-fun-system-depth/10-06-SUMMARY.md`
- Commits verified in git log: `83615ec`, `69be222`, `58e2047`, `f638159`, `1bc9550`
- Full `cargo test` green (235 unit + all integration suites, 0 failures); `cargo clippy --all-targets -D warnings` clean
- Source assertions confirmed: `forecast_days=7` (mod.rs:513), `n <= 7` (mod.rs:534), `location.trim()` rebind (mod.rs:114), `location_key(&location)` (mod.rs:126)

---
*Phase: 10-fun-system-depth*
*Completed: 2026-07-14*
