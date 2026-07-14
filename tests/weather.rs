//! Integration tests for `box weather` (WTHR-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. These cover the
//! cross-cutting CLI contract that does NOT require live network access:
//!   - a bad `--units` value is a clap usage error → exit 2 (`ValueEnum` rejects
//!     it; the command never runs);
//!   - an unreachable weather service → a graceful exit-1 error on stderr with an
//!     empty stdout (the offline path, D-13 / Pitfall WTHR-1).
//!
//! ## Determinism — never hit the live network
//! The offline test sets `BOX_WEATHER_BASE_URL=http://127.0.0.1:1`. Port 1 on
//! loopback is not bound, so the very first GET (the geocode call) fails with a
//! connection error → the `Err(_)` offline arm in `fetch` → "could not reach
//! weather service (offline?)" + exit 1. This is deterministic and offline: it
//! never resolves DNS and never contacts the real Open-Meteo API. The
//! authoritative happy-path coverage (geocode hit/no-match, metric/imperial unit
//! labels, the WMO map) lives in the in-module `#[cfg(test)]` fixture tests
//! (`cargo test --bin box weather`); a live fetch is an OPTIONAL phase-UAT
//! convenience, not part of the automated suite.

use std::io::{Read, Write};
use std::net::TcpListener;

use assert_cmd::Command;
use predicates::prelude::*;

/// A bad `--units` value is a clap usage error: exit 2, message on stderr. clap's
/// `ValueEnum` rejects the value before the command runs (mirrors the bad-flag
/// exit-2 shape in `tests/cli.rs`). Exit 2 is reserved for usage errors; the
/// command's own runtime errors (offline / non-2xx / no-match) are exit 1.
#[test]
fn bad_units_exits_2() {
    Command::cargo_bin("box")
        .unwrap()
        .args(["weather", "London", "--units", "bogus"])
        .assert()
        .failure()
        .code(2);
}

/// An unreachable weather service → exit 1 with a graceful error on stderr and an
/// empty stdout (D-13). Deterministic + offline via the `BOX_WEATHER_BASE_URL`
/// seam pointing at an unbound loopback port (never touches the live API).
#[test]
fn offline_exits_1_clean() {
    Command::cargo_bin("box")
        .unwrap()
        .args(["weather", "London"])
        .env("BOX_WEATHER_BASE_URL", "http://127.0.0.1:1")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("offline").or(predicate::str::contains("service")));
}

// --- Scriptable spine (SPINE-02) — D-17 current-only, unit from current_units ---
//
// weather is a Wave-7c odd-fit: under --json it emits the CURRENT-ONLY document
// {location, temperature, unit, conditions, …} (D-17). The unit labels MUST be
// read from the API response `current_units` object, NEVER hardcoded — the
// imperial wind label is "mp/h", not the "mph" request param (Pitfall WTHR-3).
//
// ## Determinism — a local fixture server, never the live network
// `json_purity` serves the imperial forecast fixture from a one-shot loopback
// TcpListener and points `BOX_WEATHER_BASE_URL` at it. A `lat,lon` location skips
// geocoding entirely (parse_lat_lon succeeds), so ONLY the forecast GET runs —
// the server answers exactly one request with the fixture body and exits. No DNS,
// no live Open-Meteo call.

/// The imperial forecast fixture (wind label `"mp/h"`, temp label `"°F"`) served
/// by the local fixture server in `json_purity`.
const FORECAST_IMPERIAL: &str = include_str!("fixtures/weather/forecast_imperial.json");

/// Spawn a one-shot HTTP server on a free loopback port that answers the first
/// request with `body` (HTTP/1.1 200 + JSON), then closes. Returns the bound
/// `http://127.0.0.1:<port>` base URL for `BOX_WEATHER_BASE_URL`. The accept loop
/// runs on a detached thread; the test outlives it (one request → forecast GET).
fn spawn_fixture_server(body: &'static str) -> String {
    spawn_fixture_server_n(body, 4)
}

/// Like [`spawn_fixture_server`] but serves AT MOST `max` requests before the
/// listener is dropped (the socket closes). A `max` of 1 is the cache-hit probe: the
/// server answers the FIRST fetch, and any later fetch (a cache miss) gets a
/// connection error → the command's offline exit-1 path — so a second identical call
/// succeeding PROVES it was served from the cache with no network GET.
fn spawn_fixture_server_n(body: &'static str, max: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for conn in listener.incoming().take(max) {
            let Ok(mut stream) = conn else { break };
            // Drain the request headers (read until we have seen them) so the
            // client's write side does not block on a full socket buffer.
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
    });
    base
}

/// Write `%APPDATA%\box\config.toml` (the `config_path` reads `APPDATA` first) inside
/// a fresh temp dir and return the temp dir (kept alive by the caller — dropping it
/// deletes the config). Used to drive the `[weather] location`/`units` config
/// resolution tests offline and per-process-isolated.
fn appdata_with_config(toml: &str) -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().join("box");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.toml"), toml).unwrap();
    tmp
}

/// SPINE-02 / D-17 — `box weather "51.5,-0.25" --units imperial --json` emits the
/// CURRENT-ONLY document, served offline from the imperial fixture. Asserts:
///
///   - it parses as exactly one JSON value with `temperature` / `unit` /
///     `conditions` (current-only — NO forecast/multi-day fields, Phase 10);
///   - `.unit` is the AUTHORITATIVE label from `current_units` (the imperial wind
///     unit is the API's `"mp/h"`, NOT a hardcoded "mph");
///   - no UTF-8 BOM, no ANSI escape.
///
/// Network-free + deterministic. Runnable via
/// `cargo test --test weather json_purity`.
#[test]
fn json_purity() {
    let base = spawn_fixture_server(FORECAST_IMPERIAL);

    let mut cmd = Command::cargo_bin("box").unwrap();
    let out = cmd
        .args(["weather", "51.5,-0.25", "--units", "imperial", "--json"])
        .env("NO_COLOR", "1")
        .env("BOX_WEATHER_BASE_URL", &base)
        .output()
        .expect("run box weather --json");
    assert!(
        out.status.success(),
        "box weather --json should exit 0 (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );

    // 1. stdout is exactly one JSON value (the resolved-location echo is on stderr,
    //    so stdout stays a clean single document).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. Current-only shape (D-17): temperature + unit + conditions present.
    let obj = v.as_object().expect("weather --json is a flat object");
    assert!(obj.contains_key("temperature"), "`.temperature` present");
    assert!(obj.contains_key("unit"), "`.unit` present");
    assert!(obj.contains_key("conditions"), "`.conditions` present");
    assert!(obj.contains_key("location"), "`.location` present");
    // NO forecast / multi-day fields — those are Phase 10, out of v1 scope.
    assert!(
        !obj.contains_key("forecast") && !obj.contains_key("daily") && !obj.contains_key("hourly"),
        "weather --json must be current-only (no forecast/daily/hourly): {obj:?}"
    );

    // 3. The unit label comes from current_units — the imperial wind unit is the
    //    API's authoritative "mp/h", proving it is NOT hardcoded (Pitfall WTHR-3).
    //    The fixture's current_units.wind_speed_10m is "mp/h" and temperature_2m
    //    is "°F".
    assert_eq!(
        v.get("unit").and_then(|u| u.as_str()),
        Some("°F"),
        "`.unit` must be the authoritative temperature label from current_units"
    );
    assert_eq!(
        v.get("wind_unit").and_then(|u| u.as_str()),
        Some("mp/h"),
        "`.wind_unit` must be the API's \"mp/h\" label, NOT a hardcoded \"mph\""
    );
    // The conditions text comes from the WMO map (weather_code 0 → "Clear sky").
    assert_eq!(
        v.get("conditions").and_then(|c| c.as_str()),
        Some("Clear sky"),
        "`.conditions` is the WMO-mapped text for the fixture's weather_code"
    );

    // 4. PURITY — no ANSI escape (0x1B) anywhere.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // 5. PURITY — no UTF-8 BOM (EF BB BF) at the front.
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

// --- WTHR-V2-01 (10-05) — `--forecast` 7-day daily outlook + JSON array (D-10) ---

/// The metric 7-day fixture (current block + a 7-entry `daily`/`daily_units`
/// block) served by the fixture server for the `--forecast` JSON test.
const FORECAST_METRIC_7DAY: &str = include_str!("fixtures/weather/forecast_metric_7day.json");

/// D-10 — `box weather 51.5,-0.13 --forecast --json` extends `WeatherOutput` with a
/// 7-element `forecast` array of `{date, temp_min, temp_max, conditions}` objects,
/// served offline from the 7-day fixture. A `lat,lon` location skips geocoding, so
/// only the single forecast GET runs. Asserts:
///   - stdout is exactly one JSON value carrying a `forecast` array of length 7;
///   - each entry has `date` / `temp_min` / `temp_max` / `conditions` keys;
///   - the current-only fields (`temperature`/`unit`/`conditions`) are still present.
///
/// `LOCALAPPDATA` is pointed at a fresh temp dir so the response cache (wired in
/// Task 2) is a guaranteed miss → a real fetch, keeping this test deterministic
/// across both tasks. Network-free.
#[test]
fn forecast_json_has_7_day_array() {
    let base = spawn_fixture_server(FORECAST_METRIC_7DAY);
    let cache = tempfile::TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("box").unwrap();
    let out = cmd
        .args(["weather", "51.5,-0.13", "--forecast", "--json"])
        .env("NO_COLOR", "1")
        .env("LOCALAPPDATA", cache.path())
        .env("BOX_WEATHER_BASE_URL", &base)
        .output()
        .expect("run box weather --forecast --json");
    assert!(
        out.status.success(),
        "box weather --forecast --json should exit 0 (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    let obj = v.as_object().expect("weather --json is an object");

    // Current-only fields remain present alongside the new forecast array.
    assert!(obj.contains_key("temperature"), "`.temperature` present");
    assert!(obj.contains_key("unit"), "`.unit` present");
    assert!(obj.contains_key("conditions"), "`.conditions` present");

    // The new forecast array: exactly 7 entries, each a {date, temp_min, temp_max,
    // conditions} object.
    let forecast = obj
        .get("forecast")
        .and_then(|f| f.as_array())
        .expect("`.forecast` must be an array under --forecast");
    assert_eq!(forecast.len(), 7, "the daily forecast spans 7 days: {forecast:?}");
    for (i, day) in forecast.iter().enumerate() {
        let d = day.as_object().unwrap_or_else(|| panic!("forecast[{i}] is an object"));
        assert!(d.contains_key("date"), "forecast[{i}].date present: {d:?}");
        assert!(d.contains_key("temp_min"), "forecast[{i}].temp_min present: {d:?}");
        assert!(d.contains_key("temp_max"), "forecast[{i}].temp_max present: {d:?}");
        assert!(d.contains_key("conditions"), "forecast[{i}].conditions present: {d:?}");
    }
    // The first day's conditions is the WMO-mapped text for weather_code 0.
    assert_eq!(
        forecast[0].get("conditions").and_then(|c| c.as_str()),
        Some("Clear sky"),
        "forecast[0].conditions is the WMO-mapped text"
    );

    // PURITY — no ANSI escape in the --forecast JSON stdout either.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
}

// --- WTHR-V2-01 (10-05) — optional location + config default + response cache (D-11/D-12) ---

/// The metric current-only fixture served by the config/cache tests.
const FORECAST_METRIC: &str = include_str!("fixtures/weather/forecast_metric.json");

/// D-12 — a bare `box weather` (NO positional) with `[weather] location` set in
/// config resolves the stored location and runs, served offline. `location` is a
/// `lat,lon` so geocoding is skipped (one forecast GET). Proves CLI-optional +
/// config-default location resolution.
#[test]
fn bare_weather_uses_config_location() {
    let base = spawn_fixture_server(FORECAST_METRIC);
    let appdata = appdata_with_config("[weather]\nlocation = \"51.5,-0.13\"\n");
    let cache = tempfile::TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("box").unwrap();
    let out = cmd
        .args(["weather", "--json"]) // NO positional location
        .env("NO_COLOR", "1")
        .env("APPDATA", appdata.path())
        .env("LOCALAPPDATA", cache.path())
        .env("BOX_WEATHER_BASE_URL", &base)
        .output()
        .expect("run bare box weather --json");
    assert!(
        out.status.success(),
        "bare weather with a config location must resolve + run (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout is one JSON value");
    let loc = v.get("location").and_then(|l| l.as_str()).unwrap_or("");
    assert!(
        loc.contains("51.5"),
        "the resolved location comes from [weather] location config: {loc:?}"
    );
}

/// D-12 — a bare `box weather` with NO positional AND no `[weather] location` config
/// is a usage error → exit 2 with a message pointing the user at a location or
/// `weather.location`. APPDATA points at an empty temp dir (no config.toml → default
/// config → no stored location), so the only reason to run is a missing location.
#[test]
fn bare_weather_no_location_exits_2() {
    let appdata = tempfile::TempDir::new().unwrap(); // empty → no config.toml
    let cache = tempfile::TempDir::new().unwrap();

    let assert = Command::cargo_bin("box")
        .unwrap()
        .args(["weather"]) // no positional, no config location
        .env("APPDATA", appdata.path())
        .env("LOCALAPPDATA", cache.path())
        .assert()
        .failure()
        .code(2);
    assert.stderr(predicate::str::contains("weather.location").or(
        predicate::str::contains("location"),
    ));
}

/// D-11 — a response cache serves a second identical call with no network GET. The
/// fixture server answers AT MOST ONE request; two sequential identical `box weather`
/// calls (same location/units/forecast → same cache key) both succeed, which is only
/// possible if the second was served from the cache. `LOCALAPPDATA` is a shared temp
/// dir so the second call sees the first call's cached entry; `APPDATA` is empty so
/// config never interferes.
#[test]
fn second_identical_call_is_a_cache_hit() {
    let base = spawn_fixture_server_n(FORECAST_METRIC, 1); // ONE request only
    let appdata = tempfile::TempDir::new().unwrap(); // no config
    let cache = tempfile::TempDir::new().unwrap(); // shared across both calls

    let run = || {
        Command::cargo_bin("box")
            .unwrap()
            .args(["weather", "51.5,-0.13", "--units", "metric", "--json"])
            .env("NO_COLOR", "1")
            .env("APPDATA", appdata.path())
            .env("LOCALAPPDATA", cache.path())
            .env("BOX_WEATHER_BASE_URL", &base)
            .output()
            .expect("run box weather")
    };

    // First call: a cache miss → the single served fetch → caches the projection.
    let first = run();
    assert!(
        first.status.success(),
        "first call (cache miss → fetch) must succeed (stderr: {})",
        String::from_utf8_lossy(&first.stderr)
    );

    // Second call: the server is exhausted (answered its one request). Success is
    // ONLY possible via a cache hit (no network GET).
    let second = run();
    assert!(
        second.status.success(),
        "second identical call must be served from the cache with no network (stderr: {})",
        String::from_utf8_lossy(&second.stderr)
    );

    // Both produced the same clean JSON document.
    let a: serde_json::Value = serde_json::from_slice(&first.stdout).expect("first is JSON");
    let b: serde_json::Value = serde_json::from_slice(&second.stdout).expect("second is JSON");
    assert_eq!(a, b, "the cache hit renders the same document as the fetch");
}
