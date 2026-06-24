---
phase: 05-windows-platform-integration
plan: 04
subsystem: cli
tags: [weather, open-meteo, ureq, serde, geocode, forecast, wmo, value-enum, milestone-complete]

# Dependency graph
requires:
  - phase: 01-foundation-flatten
    provides: "RunCommand trait, 0/1/2 exit policy, anyhow error→exit-1 mapping in main(), is_color_on() gate, core::input precedent"
  - phase: 05-windows-platform-integration
    plan: 03
    provides: "the registry slice (cli.rs unit-variant→Args swap, main.rs not_implemented arm removal, commands/mod.rs pub mod) that this plan mirrors + finally retires for weather — the LAST stub"
provides:
  - "box weather <city|lat,lon> [--units metric|imperial]: keyless Open-Meteo current weather (temperature/conditions/wind/humidity); parse-shape disambiguation (lat,lon used directly, else geocoded with the resolved match echoed to stderr); server-side units with the AUTHORITATIVE label read from current_units; graceful offline (exit 1) / non-2xx (exit 1) / no-location (exit 1) / bad --units (clap exit 2) (WTHR-01)"
  - "ureq 3.3.0 (blocking HTTPS, rustls default, no json feature) + serde derive dependencies"
  - "the ureq 3.x error-split pattern (Ok=2xx / Err(StatusCode)=non-2xx / Err(_)=offline) and a BOX_WEATHER_BASE_URL test seam for deterministic offline integration testing"
  - "MILESTONE: all 23 commands real — the final not_implemented arm + the dead stub import + src/commands/stub.rs + pub mod stub + BoxError::NotImplemented variant are removed; v1 feature-complete"
affects: []

# Tech tracking
tech-stack:
  added:
    - "ureq 3.3.0 (blocking HTTPS GET to Open-Meteo; default [rustls,gzip] = no OpenSSL/DLL, +crt-static-compatible; json feature deliberately OFF — deserialize via serde_json::from_reader; non-2xx is Err(StatusCode) by default)"
    - "serde 1 with derive (only the derive macro is new; serde_json 1.0.150 already a dep) for the Open-Meteo response structs"
  patterns:
    - "ureq 3.x error-split (Pitfall WTHR-1): one generic fetch<T>() matches ureq::get(url).call() — Ok(resp) ALWAYS 2xx → serde_json::from_reader(resp.into_body().into_reader()); Err(Error::StatusCode(code)) → bail!(\"weather service returned {code}\") (NOT a post-success resp.status() check); Err(_) → bail!(\"could not reach weather service (offline?)\"). Both error arms are plain anyhow::bail! → exit 1"
    - "AUTHORITATIVE unit label from current_units (D-11): the temperature/wind suffix is read straight from the response current_units object, never hardcoded — Open-Meteo labels imperial wind \"mp/h\" (≠ the mph request param, Pitfall WTHR-3), so the request param is NOT a safe label source"
    - "absent-key tolerance (Pitfall WTHR-2): GeoResp.results is #[serde(default)] so a no-match response that OMITS the results key entirely deserializes to an empty Vec (never a \"missing field\" error) → no-location bail (exit 1)"
    - "BOX_WEATHER_BASE_URL env seam: geocode/forecast origins read from the env var when set so the offline integration test points at an unbound loopback port (http://127.0.0.1:1) — deterministic, offline, never hits the live API"
    - "inline url_encode over the unreserved set (T-05-WTHR-INJ / D-12): the city name is percent-encoded with a 10-line encoder rather than adding a crate (ureq's transitive percent-encoding is not re-exported)"

key-files:
  created:
    - "src/commands/weather/mod.rs"
    - "tests/weather.rs"
    - "tests/fixtures/weather/forecast_metric.json"
    - "tests/fixtures/weather/forecast_imperial.json"
    - "tests/fixtures/weather/geocode_hit.json"
    - "tests/fixtures/weather/geocode_no_match.json"
  modified:
    - "Cargo.toml (ureq + serde derive deps)"
    - "Cargo.lock (ureq 3.3.0 + rustls/ring/webpki-roots/http/percent-encoding transitives)"
    - "src/cli.rs (Weather unit variant → Weather(WeatherArgs); extended --help body; module/enum doc updated — all 23 now real)"
    - "src/main.rs (not_implemented(weather) arm → args.run(); DELETED the now-dead `use crate::commands::stub::not_implemented;` import; updated 2 doc comments)"
    - "src/commands/mod.rs (added `pub mod weather;` in alpha order; removed the now-dead `pub mod stub;`)"
    - "src/core/errors.rs (removed the now-dead BoxError::NotImplemented variant)"
    - "tests/cli.rs (removed the obsolete stub_exits_1_to_stderr test — no stubs remain; left an explanatory comment)"

key-decisions:
  - "Dead-stub cleanup taken FULLY (planner discretion within D-00 / PATTERNS:184): with weather real, not_implemented + stub.rs + pub mod stub + BoxError::NotImplemented were all dead → deleted ALL of them (not just the mandated import) so `clippy -D warnings` stays clean with no leftover unused-code. The exit-1 RUNTIME-error contract the stub test covered is now exercised by real command error paths (json invalid-JSON, color malformed-input, weather offline — all exit 1)"
  - "BOX_WEATHER_BASE_URL test seam (discretion within the 'make the offline test deterministic' plan clause): both Open-Meteo origins read the env var when set; the offline integration test points it at http://127.0.0.1:1 (unbound loopback port → connection refused) so the path is deterministic AND never touches the live network — chosen over an #[ignore]'d live test"
  - "Conditions colored .cyan() gated on is_color_on() (D-13 discretion / D-00): the only color path; piped/NO_COLOR output is byte-identical minus ANSI. Temperature/wind/humidity are plain (the unit glyphs °C/°F carry the meaning)"
  - "WMO consecutive bands written as ranges 1..=3 / 80..=82 (clippy manual_range_patterns on this toolchain): identical semantics to the research's `1|2|3`/`80|81|82` OR form; the non-consecutive bands (45|48, 51|53|55, etc.) stay as OR patterns"
  - "humidity deserialized as f64 (RESEARCH A4 / Pitfall WTHR-3): the live sample returns an integer (35) but f64 parses both integer and fractional JSON numbers, future-proofing a fractional value; printed as `{humidity}%`"
  - "imperial fixture built per the research note (no live imperial capture): temperature_2m \"°F\", wind_speed_10m \"mp/h\" (the ≠-request-param label), proving the current_units label-sourcing invariant in a unit test"

patterns-established:
  - "The ONLY network command in the toolbox: two sequential blocking HTTPS GETs (geocode → forecast) over ureq with rustls, deserialized via serde_json::from_reader over into_body().into_reader() (no ureq json feature) — the lean-HTTP template for any future networked command"
  - "ureq 3.x status-as-error split as the canonical fetch shape (non-2xx is a match arm, never a resp.status() check) — the de-risked correctness invariant for this and any future ureq caller"

requirements-completed: [WTHR-01]

# Metrics
duration: 10min
completed: 2026-06-24
---

# Phase 5 Plan 04: box weather (WTHR-01) Summary

**`box weather <city|lat,lon> [--units metric|imperial]` ships the toolbox's only network command: keyless Open-Meteo current weather (temperature / conditions / wind / humidity) where a range-checked `lat,lon` is used directly and any other string is geocoded (the resolved match echoed to stderr so a wrong hit is visible), units are applied server-side with the AUTHORITATIVE label read from `current_units` (never hardcoded — Open-Meteo labels imperial wind `mp/h`, not the `mph` request param), and the load-bearing ureq 3.x error split classifies non-2xx as `Err(StatusCode)` and offline as `Err(_)` — both clean exit-1 — in a single generic `fetch<T>()`. This is the LAST Phase-5 stub: removing its arm retired the final `not_implemented` caller, so the dead stub import, `src/commands/stub.rs`, `pub mod stub`, and `BoxError::NotImplemented` were all deleted — all 23 commands are now real and the v1 milestone is feature-complete.**

## Performance
- **Duration:** ~10 min (RED scaffold + GREEN implementation + stub cleanup + full verification)
- **Started:** 2026-06-24T18:28:41Z
- **Completed:** 2026-06-24T18:38:25Z
- **Tasks:** 2 of 2 (both fully automated — no human-verify gate; the live fetch is an OPTIONAL phase-UAT convenience)
- **Files:** 6 created, 7 modified (incl. Cargo.lock), 1 deleted (stub.rs)

## Accomplishments
- `box weather "London"` geocodes the city (URL-encoded name → `results[0]`), echoes `Resolved "London" → London, England, United Kingdom (51.51, -0.13)` to stderr, then fetches and prints the current temperature / conditions / wind / humidity to stdout — all keyless.
- `box weather "51.5,-0.13"` skips geocoding entirely (the range-checked `parse_lat_lon` shape match) and fetches the forecast for those coordinates directly (D-12).
- `--units imperial` appends `&temperature_unit=fahrenheit&wind_speed_unit=mph` server-side (D-11); the metric default omits them. The displayed unit suffix is read from the response `current_units` object, NEVER hardcoded — so imperial wind correctly shows `mp/h` (the live label) rather than the `mph` request param (Pitfall WTHR-3).
- The generic `fetch<T>()` encodes the ureq 3.x error split (Pitfall WTHR-1): `Ok(resp)` is ALWAYS 2xx → `serde_json::from_reader(resp.into_body().into_reader())` (D-13, no ureq `json` feature); `Err(ureq::Error::StatusCode(code))` → `weather service returned {code}` (exit 1); `Err(_)` → `could not reach weather service (offline?)` (exit 1). There is NO post-success `resp.status()` check — the non-2xx branch is a match arm.
- A no-match geocode (the response OMITS the `results` key entirely) deserializes to an empty `results` via `#[serde(default)]` (Pitfall WTHR-2), not a "missing field" error → `no location found for "{name}"` (exit 1). A bad `--units` value is rejected by clap's `ValueEnum` → exit 2 (usage error), never the command's exit 1.
- **MILESTONE:** weather was the LAST `not_implemented` stub. Removing its arm made `not_implemented`, `src/commands/stub.rs`, `pub mod stub`, and `BoxError::NotImplemented` all dead — every one was deleted, the stale module/enum doc comments were updated, and the obsolete `stub_exits_1_to_stderr` test was removed. All 23 commands are now real; `cargo clippy -- -D warnings` is clean with no leftover dead code.

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1: Wave-0 scaffold (pure green, behavior red)** — `4375d13` (test) — `pub mod weather` (alpha order, end of list), `WeatherArgs { location, units }` + `Units` `ValueEnum`, the three pure helpers (`wmo_to_str`, `parse_lat_lon`, `build_forecast_url`) + `url_encode` + `format_geo_label`, the five `#[derive(Deserialize)]` structs (incl. `#[serde(default)] results`), four checked-in Open-Meteo fixtures, 8 in-module unit tests (WMO bands + `_`→Unknown, lat/lon shape+range+3-component, url-param metric-vs-imperial, url_encode reserved-char escape, Deserialize of all four fixtures incl. the no-match-empty-results proof and the `mp/h` label), and `tests/weather.rs` (bad-`--units`→exit-2 GREEN, offline→exit-1 RED). The placeholder `run()` bails so the offline behavior test is RED; the 8 pure/Deserialize tests are GREEN immediately. `serde` derive dep added.
2. **Task 2: Implement weather orchestration (GREEN)** — `01b6347` (feat) — `ureq` dep, the real `run()` / `geocode()` / `fetch()` / origin helpers (with the `BOX_WEATHER_BASE_URL` seam), the cli.rs/main.rs registry wiring, and the full dead-stub cleanup. All 8 pure tests + both integration tests green; clippy `-D warnings` + fmt clean.

**Plan metadata:** committed separately with this SUMMARY.

## Files Created/Modified
- `src/commands/weather/mod.rs` — `WeatherArgs { location, units }` + `Units` `ValueEnum`; `run()` (parse-shape disambiguation → geocode-or-direct, resolved-location echo to stderr, forecast fetch, `current_units`-sourced labels, gated `.cyan()` conditions); `geocode()` (URL-encoded name → `results[0]`, no-match bail); the generic `fetch<T>()` (ureq 3.x error split); `geocode_origin`/`forecast_origin` (the `BOX_WEATHER_BASE_URL` seam); the three pure helpers + `url_encode` + `format_geo_label`; the five Deserialize structs; `#[cfg(test)] mod tests` (8 cases). Module doc records D-10..D-13 + the test seam.
- `tests/weather.rs` — `bad_units_exits_2` (clap `ValueEnum` rejects `--units bogus` → exit 2) and `offline_exits_1_clean` (the `BOX_WEATHER_BASE_URL=http://127.0.0.1:1` seam → connection refused → "offline" exit 1, empty stdout); header documents the never-hit-the-live-network determinism.
- `tests/fixtures/weather/{forecast_metric,forecast_imperial,geocode_hit,geocode_no_match}.json` — captured/constructed Open-Meteo shapes encoding the three landmines (metric `km/h` label, imperial `°F`/`mp/h` label, London hit, no-`results`-key no-match).
- `Cargo.toml` / `Cargo.lock` — `ureq = "3.3.0"` (rustls, no `json` feature) + `serde = { version = "1", features = ["derive"] }` with the D-13 feature-trim rationale; lockfile pins ureq + rustls/ring/webpki-roots/http/percent-encoding transitives.
- `src/cli.rs` — `Weather` unit variant → `Weather(crate::commands::weather::WeatherArgs)`; the first `///` line (`Fetch the weather for a location`) preserved verbatim so the `box --help` trycmd snapshot is UNCHANGED, with an added `--help` body; the module/enum doc updated (all 23 commands now real, no stub handler).
- `src/main.rs` — `Commands::Weather => Err(not_implemented("weather"))` → `Commands::Weather(args) => args.run()`; **DELETED** `use crate::commands::stub::not_implemented;` (the last caller); two doc comments updated to drop the `NotImplemented` mention.
- `src/commands/mod.rs` — `pub mod weather;` added (alpha order); `pub mod stub;` removed.
- `src/core/errors.rs` — `BoxError::NotImplemented` variant removed (now dead).
- `src/commands/stub.rs` — **deleted** (the whole module is now dead — intentional, the milestone cleanup).
- `tests/cli.rs` — removed the obsolete `stub_exits_1_to_stderr` test (it used `weather` as the last stub) + updated the help-test comment; left an explanatory note pointing at the real exit-1 paths that now cover the contract.

## Decisions Made
Beyond the locked CONTEXT decisions (D-10/D-11/D-12/D-13), the Claude's-discretion items were resolved as:
- **Stdout layout:** a 4-line aligned labeled block (`Conditions` / `Temperature` / `Wind` / `Humidity`), mirroring the `color` block style; the geocoded `Name, Admin1, Country` surfaces in the stderr resolved-location echo rather than the stdout header.
- **Color:** only the conditions line is colored, `.cyan()`, gated on `is_color_on()` (the one color path); everything else is plain. Piped/NO_COLOR is byte-identical minus ANSI.
- **Dead-stub cleanup:** taken fully (delete `stub.rs` + `pub mod stub` + `BoxError::NotImplemented` + the import + the obsolete test), not just the mandated import — the cleanest way to keep `clippy -D warnings` green now that no stub exists.
- **Offline-test determinism:** the `BOX_WEATHER_BASE_URL` env seam → an unbound loopback port, over an `#[ignore]`'d live test, so the offline path is fully automated and never network-dependent.

## Deviations from Plan

None — plan executed exactly as written. The two TDD tasks followed the locked interfaces, the RESEARCH code example, and the Pitfall guidance verbatim. `ureq` 3.3.0 + `serde` derive resolved on the first build; the ureq error-split, the `current_units` label-sourcing, the `#[serde(default)]` absent-key tolerance, and the URL-encode were implemented as specified. Two minor in-task corrections (NOT plan deviations): (1) `clippy manual_range_patterns` flagged the consecutive WMO bands `1|2|3` and `80|81|82` → rewritten as `1..=3` / `80..=82` (identical semantics); (2) two `cargo fmt` reflows (a multi-line `.stderr(...)` and a `.contains(...)` assertion) applied before the Task 2 commit. No bugs, no missing critical functionality, no architectural changes.

## Issues Encountered
None of substance. The RED→GREEN cycle ran cleanly: in RED the 8 pure/Deserialize tests passed at once and `offline_exits_1_clean` failed (the placeholder `run()` bailed "not implemented" with no offline message); the RED-phase dead-code warnings (the network-only structs/helpers consumed by the real `run()`) cleared in GREEN. The only GREEN corrections were the two clippy range-pattern nits and the fmt reflows, both fixed before the commit; clippy `-D warnings` and `cargo fmt --check` are clean post-GREEN.

## Verification Status

**Automated (all GREEN):**
- `cargo test --bin box weather` — the 8 in-module tests pass: `wmo_to_str` (every band + `_`→"Unknown"), `parse_lat_lon` (in-range Some / city None / out-of-range None / 3-component None), `build_forecast_url` (metric omits both unit params, imperial includes both), `url_encode` (space/`&`/`=` escaped, unreserved untouched), and Deserialize of all four fixtures (metric `°C`/`km/h` + WMO map, imperial `°F`/`mp/h` label, London hit + label, no-match empty-`results` proof).
- `cargo test --test weather` — `bad_units_exits_2` (clap `ValueEnum` → exit 2) and `offline_exits_1_clean` (the `BOX_WEATHER_BASE_URL` unreachable-host seam → exit 1, "offline"/"service" on stderr, empty stdout) both pass.
- `cargo test` — full suite green: **149 bin-unit tests** (was 141 + 8 new weather) + all integration suites + the `trycmd` `box --help` snapshot (UNCHANGED — the `Fetch the weather for a location` one-liner was preserved). `tests/cli.rs` is now 6 tests (the obsolete `stub_exits_1_to_stderr` removed). **No regression.**
- `cargo clippy --all-targets -- -D warnings` — clean (no dead `not_implemented` import / dead stub module / dead `NotImplemented` variant; the WMO ranges satisfy `manual_range_patterns`).
- `cargo fmt --check` — clean.

**Optional (phase-UAT convenience — NOT a blocking gate):**
- A live fetch in PowerShell 7: `box weather "London"` and `box weather "London" --units imperial` showing real temperature/conditions/wind/humidity with the authoritative unit label (`mp/h` on imperial) and the resolved-location echo on stderr. This is offered as a convenience check during phase UAT; the plan is COMPLETE on the green fixture-based suite (the live network is deliberately never hit by the automated tests). Per WTHR-01 / D-13 the offline, non-2xx, no-location, and bad-units paths are all automated above.

## Known Stubs
None. `box weather` is a real network command — no hardcoded empty values flowing to UI, no placeholder text, no unwired data sources. This plan REMOVED the last stub infrastructure in the codebase (`src/commands/stub.rs` + `BoxError::NotImplemented`).

## User Setup Required
None — Open-Meteo is keyless (no API key, no env var, no account, no persistence). The only runtime dependency is internet egress to Open-Meteo over HTTPS 443; its absence is an explicitly-specified graceful exit-1 error (D-13), not a failure.

## Next Phase Readiness
- `box weather` (city + `lat,lon`, metric/imperial, graceful offline) is live and fully automated-tested. `ureq` (rustls blocking HTTPS) + the ureq 3.x error-split fetch pattern are established as the lean-HTTP template for any future networked command.
- **MILESTONE: all 23 commands are real.** Every `not_implemented` stub is gone; the stub module, the `NotImplemented` error variant, and the dead import were deleted. `box` is v1 feature-complete.
- Phase-5 UAT convenience checks remaining (deferred to phase-end UAT, mirroring 05-01 QR phone-scan / 05-02 clip round-trip / 05-03 pomodoro live countdown+toast+cancel): the OPTIONAL `box weather "London"` live fetch above. None block plan completion.

## Self-Check: PASSED
- `src/commands/weather/mod.rs` — FOUND
- `tests/weather.rs` — FOUND
- `tests/fixtures/weather/forecast_metric.json` — FOUND
- `tests/fixtures/weather/forecast_imperial.json` — FOUND
- `tests/fixtures/weather/geocode_hit.json` — FOUND
- `tests/fixtures/weather/geocode_no_match.json` — FOUND
- `src/commands/stub.rs` — ABSENT (intentionally deleted — the milestone cleanup)
- Commit `4375d13` (Task 1, test) — FOUND
- Commit `01b6347` (Task 2, feat) — FOUND

---
*Phase: 05-windows-platform-integration*
*Completed: 2026-06-24*
