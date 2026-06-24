//! The `weather` command: current conditions for a city name or `lat,lon` via
//! the keyless Open-Meteo API (WTHR-01). The only Phase-5 command that touches
//! the network, and the last command in the v1 milestone.
//!
//! Flow (Pattern 1 — thin orchestrator over pure helpers + two blocking GETs):
//! `run()` disambiguates the positional by shape (D-12): if it matches the
//! range-checked `lat,lon` form it is used directly; otherwise it is geocoded as
//! a city name. Each GET goes through the generic [`fetch`] helper, which encodes
//! the load-bearing ureq 3.x error split (Pitfall WTHR-1): non-2xx is an
//! `Err(ureq::Error::StatusCode)` MATCH ARM, never a post-success
//! `resp.status()` check; offline/DNS/connect is a separate arm. The forecast is
//! deserialized via `serde_json::from_reader(resp.into_body().into_reader())`
//! (no ureq `json` feature, D-13).
//!
//! Decisions encoded here:
//! - **D-10** — default units are metric (°C, km/h); `--units imperial` switches
//!   to Fahrenheit via a clap `ValueEnum` (a bad value → clap exit 2).
//! - **D-11** — units are applied SERVER-SIDE (`&temperature_unit=fahrenheit
//!   &wind_speed_unit=mph` on imperial only), and the AUTHORITATIVE unit label is
//!   read from the response `current_units` object, NEVER hardcoded. Open-Meteo
//!   even labels imperial wind `"mp/h"` (not `"mph"`), so the request param ≠ the
//!   label (Pitfall WTHR-3) — reading `current_units` is the only correct source.
//! - **D-12** — location is parse-shape disambiguated (`parse_lat_lon`); a city is
//!   geocoded (URL-encoded name → `results[0]`), the resolved location is echoed
//!   to stderr so a wrong match is visible, and zero results
//!   (the absent `results` key — Pitfall WTHR-2, `#[serde(default)]`) → exit 1
//!   with `no location found for "{name}"`.
//! - **D-13** — `weather_code` maps to text via a WMO `match` returning
//!   `&'static str` (the `_` arm future-proofs unknown codes); errors → stderr +
//!   exit 1 (offline → "could not reach weather service (offline?)"; non-2xx →
//!   "weather service returned {status}"). Exit 2 is reserved for clap's bad
//!   `--units`. Data → stdout, any color gated via `is_color_on()`.
//!
//! ## Test seam
//! `BOX_WEATHER_BASE_URL` overrides the Open-Meteo origin (scheme + host[:port])
//! so the offline integration test can point at a guaranteed-unreachable host
//! (`http://127.0.0.1:1`) and assert the offline-exit-1 shape deterministically,
//! without ever hitting the live network. Unset in normal use → the real
//! Open-Meteo origins.

use clap::{Args, ValueEnum};
use serde::Deserialize;

use crate::commands::RunCommand;

/// Unit system for the forecast (D-10). `Metric` is the default (and Open-Meteo's
/// own API default → the no-flag path sends no unit params). `Imperial` appends
/// the server-side unit params (D-11). A clap `ValueEnum` so an invalid value is
/// a usage error → clap exit 2 (never the command's exit 1).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Units {
    /// °C and km/h (the default; no extra request params).
    Metric,
    /// °F and mph (server-side `temperature_unit`/`wind_speed_unit`).
    Imperial,
}

/// `box weather <LOCATION> [--units metric|imperial]` — current weather (WTHR-01).
///
/// `LOCATION` is either a `lat,lon` pair (e.g. `51.5,-0.13`, used directly) or a
/// city name (e.g. `London`, geocoded via Open-Meteo). `--units imperial`
/// switches to °F/mph; the default is metric °C/km/h. No API key is required.
/// When the service is unreachable a graceful error is printed to stderr (exit 1).
#[derive(Debug, Args)]
pub struct WeatherArgs {
    /// A city name (`London`) or a `lat,lon` pair (`51.5,-0.13`).
    pub location: String,
    /// Unit system for the forecast.
    #[arg(long, value_enum, default_value_t = Units::Metric)]
    pub units: Units,
}

impl RunCommand for WeatherArgs {
    fn run(self) -> anyhow::Result<()> {
        // Placeholder until Task 2 wires the geocode + forecast orchestration. The
        // network behavior integration tests (offline → exit 1) are RED until then.
        let _ = &self.location;
        anyhow::bail!("not implemented")
    }
}

/// Map a WMO weather code to a short `&'static str` description (D-13). No alloc;
/// the `_` arm future-proofs any code Open-Meteo adds (T-05-WTHR-DoS). Exact
/// labels are discretion; the `_` fallthrough is mandated.
fn wmo_to_str(code: u32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 | 2 | 3 => "Partly cloudy",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        61 | 63 | 65 => "Rain",
        71 | 73 | 75 => "Snow",
        80 | 81 | 82 => "Rain showers",
        95 => "Thunderstorm",
        _ => "Unknown",
    }
}

/// Parse the positional as `lat,lon` ONLY if it matches the shape AND both
/// components are in range (D-12): `lat ∈ [-90,90]`, `lon ∈ [-180,180]`. Anything
/// else → `None` (geocode it as a city name). Pure → unit-testable.
fn parse_lat_lon(s: &str) -> Option<(f64, f64)> {
    let re = regex::Regex::new(r"^\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*$").ok()?;
    let c = re.captures(s.trim())?;
    let lat: f64 = c[1].parse().ok()?;
    let lon: f64 = c[2].parse().ok()?;
    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
}

/// Build the Open-Meteo forecast URL for `(lat, lon)`. The base `current=...` set
/// is always requested; the imperial server-side unit params
/// (`&temperature_unit=fahrenheit&wind_speed_unit=mph`) are appended ONLY for
/// `Units::Imperial` (D-11/D-13) — the metric path omits them entirely. Pure →
/// the imperial branch is unit-testable without a network call.
///
/// Uses the real Open-Meteo forecast origin; Task 2 routes it through the
/// `BOX_WEATHER_BASE_URL` test seam.
fn build_forecast_url(lat: f64, lon: f64, units: Units) -> String {
    let mut url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m",
    );
    if matches!(units, Units::Imperial) {
        url.push_str("&temperature_unit=fahrenheit&wind_speed_unit=mph");
    }
    url
}

/// Percent-encode a query-string value so reserved characters (`&`/`=`/spaces/
/// non-ASCII) cannot inject extra query params (T-05-WTHR-INJ). A minimal inline
/// encoder over the unreserved set (`A-Za-z0-9-_.~`) — every other byte becomes
/// `%XX`. ureq's transitive `percent-encoding` is NOT re-exported, and adding a
/// crate solely for this would be heavier than this 10-line function (Pitfall
/// WTHR-2 / D-12).
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Format a geocoding hit as `Name, Admin1, Country`, omitting absent parts.
fn format_geo_label(hit: &GeoHit) -> String {
    let mut parts = vec![hit.name.clone()];
    if let Some(admin1) = &hit.admin1 {
        parts.push(admin1.clone());
    }
    if let Some(country) = &hit.country {
        parts.push(country.clone());
    }
    parts.join(", ")
}

// --- Open-Meteo response shapes (match the LIVE JSON, verified 2026-06-24) ---

/// Geocoding search response. `results` is ABSENT on a no-match (NOT an empty
/// array) — `#[serde(default)]` tolerates the missing key (Pitfall WTHR-2).
#[derive(Debug, Deserialize)]
struct GeoResp {
    #[serde(default)]
    results: Vec<GeoHit>,
}

/// One geocoding hit. `admin1`/`country` may be absent for some locations.
#[derive(Debug, Deserialize)]
struct GeoHit {
    latitude: f64,
    longitude: f64,
    name: String,
    #[serde(default)]
    admin1: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

/// Forecast response: the `current` values plus the AUTHORITATIVE `current_units`
/// label object (read the suffix from here, never hardcode — D-11).
#[derive(Debug, Deserialize)]
struct ForecastResp {
    current: Current,
    current_units: CurrentUnits,
}

/// The current-weather values. `relative_humidity_2m` comes back as an integer
/// (e.g. `35`) but `f64` deserializes both integer and fractional JSON numbers
/// safely (Pitfall WTHR-3 / RESEARCH A4).
#[derive(Debug, Deserialize)]
struct Current {
    temperature_2m: f64,
    relative_humidity_2m: f64,
    weather_code: u32,
    wind_speed_10m: f64,
}

/// The unit-label object: e.g. `"°C"`/`"°F"`, `"km/h"`/`"mp/h"` (note the
/// imperial wind label is `"mp/h"`, NOT the `mph` request param — Pitfall WTHR-3).
#[derive(Debug, Deserialize)]
struct CurrentUnits {
    temperature_2m: String,
    wind_speed_10m: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `wmo_to_str` maps every band to its label and the `_` arm catches unknown
    /// codes as "Unknown" (D-13 — the future-proof fallthrough).
    #[test]
    fn wmo_to_str_maps_bands_and_unknown() {
        assert_eq!(wmo_to_str(0), "Clear sky");
        assert_eq!(wmo_to_str(1), "Partly cloudy");
        assert_eq!(wmo_to_str(2), "Partly cloudy");
        assert_eq!(wmo_to_str(3), "Partly cloudy");
        assert_eq!(wmo_to_str(45), "Fog");
        assert_eq!(wmo_to_str(48), "Fog");
        assert_eq!(wmo_to_str(51), "Drizzle");
        assert_eq!(wmo_to_str(61), "Rain");
        assert_eq!(wmo_to_str(71), "Snow");
        assert_eq!(wmo_to_str(80), "Rain showers");
        assert_eq!(wmo_to_str(95), "Thunderstorm");
        // The mandated `_` arm — any unmapped code is "Unknown".
        assert_eq!(wmo_to_str(9999), "Unknown");
        assert_eq!(wmo_to_str(4), "Unknown");
    }

    /// `parse_lat_lon` accepts a well-formed in-range pair, rejects a city name,
    /// an out-of-range component, and a 3-component string (D-12).
    #[test]
    fn parse_lat_lon_shape_and_range() {
        assert_eq!(parse_lat_lon("51.5,-0.13"), Some((51.5, -0.13)));
        // Whitespace around / between the components is tolerated.
        assert_eq!(parse_lat_lon("  51.5 , -0.13 "), Some((51.5, -0.13)));
        // A plain integer pair works.
        assert_eq!(parse_lat_lon("40,-74"), Some((40.0, -74.0)));
        // A city name is NOT a lat,lon → None (geocode it).
        assert_eq!(parse_lat_lon("London"), None);
        // Out of range (lat > 90) → None.
        assert_eq!(parse_lat_lon("91,0"), None);
        // Out of range (lon < -180) → None.
        assert_eq!(parse_lat_lon("0,-181"), None);
        // A 3-component string does NOT match the 2-component shape → None.
        assert_eq!(parse_lat_lon("1,2,3"), None);
    }

    /// `build_forecast_url` omits the unit params for metric and includes BOTH the
    /// temperature_unit AND wind_speed_unit params for imperial (D-11).
    #[test]
    fn build_forecast_url_unit_params() {
        let metric = build_forecast_url(51.5, -0.13, Units::Metric);
        assert!(
            !metric.contains("temperature_unit=fahrenheit"),
            "metric must NOT request fahrenheit: {metric}"
        );
        assert!(
            !metric.contains("wind_speed_unit=mph"),
            "metric must NOT request mph: {metric}"
        );
        assert!(
            metric
                .contains("current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m"),
            "metric must request the current set: {metric}"
        );

        let imperial = build_forecast_url(51.5, -0.13, Units::Imperial);
        assert!(
            imperial.contains("temperature_unit=fahrenheit"),
            "imperial must request fahrenheit: {imperial}"
        );
        assert!(
            imperial.contains("wind_speed_unit=mph"),
            "imperial must request mph: {imperial}"
        );
    }

    /// `url_encode` percent-encodes reserved chars (space/`&`/`=`) so a hostile
    /// city name cannot inject extra query params (T-05-WTHR-INJ), while leaving
    /// the unreserved set untouched.
    #[test]
    fn url_encode_escapes_reserved() {
        assert_eq!(url_encode("New York"), "New%20York");
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
        // Unreserved characters pass through verbatim.
        assert_eq!(url_encode("London-1_2.3~"), "London-1_2.3~");
    }

    /// The metric forecast fixture deserializes into `ForecastResp`: the
    /// `weather_code` maps via `wmo_to_str`, and the AUTHORITATIVE wind label is
    /// `"km/h"` read straight from `current_units` (D-11 — proves we never
    /// hardcode the suffix).
    #[test]
    fn forecast_metric_fixture_deserializes() {
        let raw = include_str!("../../../tests/fixtures/weather/forecast_metric.json");
        let f: ForecastResp = serde_json::from_str(raw).expect("metric forecast parses");
        assert_eq!(wmo_to_str(f.current.weather_code), "Clear sky");
        assert_eq!(f.current_units.temperature_2m, "°C");
        assert_eq!(f.current_units.wind_speed_10m, "km/h");
        // Humidity (a JSON integer) deserializes into f64 fine (Pitfall WTHR-3).
        assert_eq!(f.current.relative_humidity_2m, 35.0);
    }

    /// The imperial forecast fixture's wind label is `"mp/h"` — NOT the `mph`
    /// request param (Pitfall WTHR-3). Reading it from `current_units` is the only
    /// way to get the right suffix.
    #[test]
    fn forecast_imperial_fixture_label_is_mp_slash_h() {
        let raw = include_str!("../../../tests/fixtures/weather/forecast_imperial.json");
        let f: ForecastResp = serde_json::from_str(raw).expect("imperial forecast parses");
        assert_eq!(f.current_units.temperature_2m, "°F");
        assert_eq!(
            f.current_units.wind_speed_10m, "mp/h",
            "the imperial wind LABEL is mp/h, not the mph request param (Pitfall WTHR-3)"
        );
    }

    /// The geocoding-hit fixture deserializes with a non-empty `results` and the
    /// expected London coordinates + label parts.
    #[test]
    fn geocode_hit_fixture_deserializes() {
        let raw = include_str!("../../../tests/fixtures/weather/geocode_hit.json");
        let g: GeoResp = serde_json::from_str(raw).expect("geocode hit parses");
        assert_eq!(g.results.len(), 1);
        let hit = &g.results[0];
        assert_eq!(hit.name, "London");
        assert_eq!(hit.admin1.as_deref(), Some("England"));
        assert_eq!(hit.country.as_deref(), Some("United Kingdom"));
        let label = format_geo_label(hit);
        assert_eq!(label, "London, England, United Kingdom");
    }

    /// The no-match geocoding fixture OMITS the `results` key entirely; with
    /// `#[serde(default)]` it deserializes into an EMPTY `results` rather than a
    /// "missing field" error (Pitfall WTHR-2 — the proof).
    #[test]
    fn geocode_no_match_fixture_empty_results() {
        let raw = include_str!("../../../tests/fixtures/weather/geocode_no_match.json");
        let g: GeoResp =
            serde_json::from_str(raw).expect("no-match geocode parses (absent results key)");
        assert!(
            g.results.is_empty(),
            "absent results key must deserialize to empty, not error"
        );
    }
}
