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
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        // Serve a handful of requests so a client retry/extra GET cannot hang the
        // test; each connection gets the same fixture body and is then closed.
        for conn in listener.incoming().take(4) {
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
