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

use anyhow::Context;
use clap::{Args, ValueEnum};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::commands::RunCommand;
use crate::core::output::is_color_on;

/// Default (real) Open-Meteo geocoding origin.
const GEOCODE_ORIGIN: &str = "https://geocoding-api.open-meteo.com";
/// Default (real) Open-Meteo forecast origin.
const FORECAST_ORIGIN: &str = "https://api.open-meteo.com";
/// Env var that overrides BOTH origins for deterministic offline testing.
const BASE_URL_ENV: &str = "BOX_WEATHER_BASE_URL";

/// Unit system for the forecast (D-10). `Metric` is the default (and Open-Meteo's
/// own API default → the no-flag path sends no unit params). `Imperial` appends
/// the server-side unit params (D-11). A clap `ValueEnum` so an invalid value is
/// a usage error → clap exit 2 (never the command's exit 1).
///
/// Config-deserializable since D-13 (mirrors `hash::Algo` at `hash/mod.rs:114`):
/// `serde::Deserialize` + `#[serde(rename_all = "lowercase")]` lets `[weather]
/// units = "imperial"` parse into `Some(Units::Imperial)` through the nested
/// `WeatherConfig`; `PartialEq, Eq` make it assertable in the config round-trip
/// tests. The lowercase serde spellings match the `ValueEnum` variant names, so
/// `--units`, config, and any future env tier all share ONE spelling table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
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
    /// A city name (`London`) or a `lat,lon` pair (`51.5,-0.13`). Optional: when
    /// omitted, the stored `[weather] location` config value is used; if neither is
    /// set, the command exits 2 with a hint (D-12).
    pub location: Option<String>,
    /// Unit system for the forecast. NO clap default (Anti-Pattern 3) so the
    /// `[weather] units` config can win when `--units` is absent; the metric builtin
    /// is applied by the resolver as the final fallback.
    #[arg(long, value_enum)]
    pub units: Option<Units>,
    /// Also show a 7-day daily forecast (date, min/max temp, conditions).
    #[arg(long)]
    pub forecast: bool,
}

impl RunCommand for WeatherArgs {
    fn run(self) -> anyhow::Result<()> {
        // Resolve the location: CLI positional > [weather] location config (D-12).
        // Still-`None` (no positional AND no config) is a usage error → exit 2.
        let location = resolve_location(
            self.location.clone(),
            crate::core::config::config().weather.location.clone(),
        )
        .ok_or(crate::core::errors::BoxError::MissingLocation)?;

        // Resolve the unit system: CLI --units > [weather] units config > metric
        // builtin (the settled SPINE-05 precedence, D-12).
        let units = resolve_units(self.units, crate::core::config::config().weather.units);

        // The LOGICAL cache key (location, units, forecast-or-not). It is handed to
        // core::cache, which blake3-hashes it into the filename — the raw location
        // string NEVER becomes a path component (T-10-05-CACHE-KEY, mitigated in
        // 10-04). Distinct forecast/current + metric/imperial keys never collide.
        let cache_key = format!(
            "{location}|{}|{}",
            units_key(units),
            if self.forecast { "forecast" } else { "current" }
        );

        // Try the cache FIRST (D-11, transparent). A hit whose payload also
        // deserializes into the current projection shape skips ALL network (geocode
        // + forecast GET); a miss/stale/malformed entry (cache::get → None) OR a
        // payload that no longer matches this shape (schema drift → the `.ok()`
        // None) transparently falls through to a fresh fetch. The cache NEVER fails
        // the command (T-10-05-CACHE-POISON).
        let weather = match crate::core::cache::get(&cache_key)
            .and_then(|payload| serde_json::from_str::<CachedWeather>(&payload).ok())
        {
            Some(cached) => cached,
            None => {
                let fresh = fetch_weather(&location, units, self.forecast)?;
                // Best-effort write so a repeat within the TTL is a hit. A failed
                // serialize/write is a silent no-op — never fails the command.
                if let Ok(payload) = serde_json::to_string(&fresh) {
                    crate::core::cache::put(&cache_key, &payload);
                }
                fresh
            }
        };

        // Echo the resolved location to stderr so a wrong geocode/config match is
        // visible (D-12). stderr ONLY — stdout stays a clean single document even
        // under --json, and the cache adds NOTHING to stdout (T-10-05-JSON-PURITY).
        eprintln!(
            "Resolved \"{location}\" → {} ({:.2}, {:.2})",
            weather.label, weather.lat, weather.lon
        );

        render(&weather)
    }
}

/// Resolve the location: CLI positional > `[weather] location` config (D-12). Pure so
/// the precedence is unit-testable; `None` means neither tier supplied one (the
/// caller maps that to [`BoxError::MissingLocation`] → exit 2).
fn resolve_location(cli: Option<String>, cfg: Option<String>) -> Option<String> {
    cli.or(cfg)
}

/// Resolve the unit system: CLI `--units` > `[weather] units` config > metric builtin
/// — the settled SPINE-05 `cli.or(cfg).unwrap_or(builtin)` shape (D-12). Pure → the
/// precedence is a known-answer unit test.
fn resolve_units(cli: Option<Units>, cfg: Option<Units>) -> Units {
    cli.or(cfg).unwrap_or(Units::Metric)
}

/// The stable cache-key token for a unit system (`"metric"`/`"imperial"`), matching
/// the lowercase serde spelling so the key stays human-legible and collision-free.
fn units_key(units: Units) -> &'static str {
    match units {
        Units::Metric => "metric",
        Units::Imperial => "imperial",
    }
}

/// Fetch + project a fresh weather response into the [`CachedWeather`] projection (the
/// cache-MISS path). Disambiguates the location by shape (D-12: a range-checked
/// `lat,lon` used directly, else geocoded), runs the current (+ optional daily) GET
/// via the reused [`fetch`], and reads EVERY unit label from the authoritative
/// `current_units`/`daily_units` objects (never hardcoded — D-11). A malformed daily
/// block is a clean error, never a panic (T-10-05-HTTP).
fn fetch_weather(location: &str, units: Units, forecast: bool) -> anyhow::Result<CachedWeather> {
    // Parse-shape disambiguation (D-12): a range-checked `lat,lon` is used directly;
    // anything else is geocoded as a city name.
    let (lat, lon, label) = match parse_lat_lon(location) {
        Some((lat, lon)) => (lat, lon, format!("{lat:.4},{lon:.4}")),
        None => geocode(location)?,
    };

    // Server-side unit params on imperial only (D-11/D-13); the daily block is
    // requested ONLY under `--forecast` (D-10).
    let url = build_forecast_url(lat, lon, units, forecast);
    let resp: ForecastResp = fetch(&url)?;

    // Under `--forecast`, project the parallel daily arrays into the bounded 7-day
    // rows (D-10). An absent/short/mismatched daily block is a plain error → exit 1,
    // NEVER a panic (T-10-05-HTTP). The daily temp label is read from `daily_units`
    // (the SAME authoritative-label rule as `current_units`).
    let (forecast_days, daily_temp_unit) = if forecast {
        let daily = resp
            .daily
            .as_ref()
            .context("forecast response missing the daily block")?;
        let daily_units = resp
            .daily_units
            .as_ref()
            .context("forecast response missing the daily_units block")?;
        (
            Some(build_day_forecasts(daily)?),
            Some(daily_units.temperature_2m_max.clone()),
        )
    } else {
        (None, None)
    };

    Ok(CachedWeather {
        label,
        lat,
        lon,
        temperature: resp.current.temperature_2m,
        // AUTHORITATIVE unit labels from current_units — NEVER hardcoded (D-11). The
        // imperial wind label is "mp/h", not the "mph" request param.
        temp_unit: resp.current_units.temperature_2m.clone(),
        conditions: wmo_to_str(resp.current.weather_code).to_string(),
        wind_speed: resp.current.wind_speed_10m,
        wind_unit: resp.current_units.wind_speed_10m.clone(),
        humidity: resp.current.relative_humidity_2m,
        daily_temp_unit,
        forecast: forecast_days,
    })
}

/// Render a [`CachedWeather`] to stdout — the SHARED path for both a cache hit and a
/// fresh fetch, so a hit is byte-identical to a miss. Forks on `--json` FIRST
/// (Pitfall 1): under it, emit the [`WeatherOutput`] document (the current-only shape
/// stays byte-unchanged because `forecast` is `None` off `--forecast`,
/// `skip_serializing_if`); otherwise print the aligned human block. Colour is gated
/// SOLELY on `is_color_on()` (piped/`--json` output is byte-identical minus ANSI) and
/// plain lines route through `out_line` so `--clip` tees them. Cache/echo add NOTHING
/// to stdout (T-10-05-JSON-PURITY).
fn render(w: &CachedWeather) -> anyhow::Result<()> {
    if crate::core::output::is_json_on() {
        let doc = WeatherOutput {
            location: w.label.clone(),
            temperature: w.temperature,
            unit: w.temp_unit.clone(),
            conditions: w.conditions.clone(),
            wind_speed: w.wind_speed,
            wind_unit: w.wind_unit.clone(),
            humidity: w.humidity,
            forecast: w.forecast.clone(),
        };
        return crate::core::output::emit_json(&doc);
    }

    // Aligned labeled current block → stdout. The colored `Conditions` branch stays a
    // direct `println!` because `is_color_on()` is forced false under `--clip`
    // (init_output), so that branch is unreachable when clip is active (mirrors the
    // `color` swatch pattern); the plain lines route through `out_line` to tee.
    if is_color_on() {
        println!("  Conditions  : {}", w.conditions.cyan());
    } else {
        crate::core::output::out_line(&format!("  Conditions  : {}", w.conditions));
    }
    crate::core::output::out_line(&format!("  Temperature : {}{}", w.temperature, w.temp_unit));
    crate::core::output::out_line(&format!("  Wind        : {} {}", w.wind_speed, w.wind_unit));
    crate::core::output::out_line(&format!("  Humidity    : {}%", w.humidity));

    // The daily section (D-10) — additive AFTER the current block, only under
    // `--forecast`. The temp label is the authoritative `daily_units` value carried
    // in the projection; the conditions colour is gated exactly like the current
    // block (plain lines through `out_line` so `--clip` tees them).
    if let Some(days) = &w.forecast {
        let unit = w.daily_temp_unit.as_deref().unwrap_or("");
        crate::core::output::out_line("");
        crate::core::output::out_line("  7-day forecast:");
        for d in days {
            if is_color_on() {
                println!(
                    "    {}  {}{unit} / {}{unit}  {}",
                    d.date,
                    d.temp_min,
                    d.temp_max,
                    d.conditions.cyan()
                );
            } else {
                crate::core::output::out_line(&format!(
                    "    {}  {}{unit} / {}{unit}  {}",
                    d.date, d.temp_min, d.temp_max, d.conditions
                ));
            }
        }
    }
    Ok(())
}

/// The `--json` document for `weather` (D-17): the CURRENT-ONLY conditions, a
/// flat scalar object (scalar → flat, D-01). The `unit`/`wind_unit` labels are
/// taken from the response `current_units` object, NEVER hardcoded — the imperial
/// wind label is the API's `"mp/h"`, not the `mph` request param (Pitfall WTHR-3).
/// The `f64` fields come straight from the parsed `current` block (finite real
/// API data — never a hand-computed NaN/Inf, Pitfall 2). No forecast/multi-day
/// fields here — those are Phase 10 (out of v1 scope).
#[derive(Serialize)]
struct WeatherOutput {
    /// The resolved location label (the `lat,lon` echo or the geocoded name).
    location: String,
    /// Current temperature, in the unit named by `unit`.
    temperature: f64,
    /// The authoritative temperature unit label from `current_units` (`°C`/`°F`).
    unit: String,
    /// The WMO-mapped conditions text (e.g. `"Clear sky"`).
    conditions: String,
    /// Current wind speed, in the unit named by `wind_unit`.
    wind_speed: f64,
    /// The authoritative wind unit label from `current_units` (`km/h`/`mp/h`).
    wind_unit: String,
    /// Relative humidity as a percentage value.
    humidity: f64,
    /// The 7-day daily outlook (D-10), present ONLY under `--forecast`.
    /// `skip_serializing_if` omits the key entirely on the current-only path, so
    /// the pre-Phase-10 `--json` document is byte-unchanged. Each temp is in the
    /// SAME unit as the current `temperature` (the server-side unit param applies to
    /// current AND daily), so no per-day unit label is needed in the machine shape.
    #[serde(skip_serializing_if = "Option::is_none")]
    forecast: Option<Vec<DayForecast>>,
}

/// One day of the 7-day daily outlook (D-10) in the `--json` `forecast` array:
/// a flat `{date, temp_min, temp_max, conditions}` object (snake_case house style).
/// `temp_min`/`temp_max` are the raw daily extremes from the API (finite real data);
/// `conditions` is the WMO-mapped text for that day's `weather_code`. `Deserialize`
/// + `Clone` let the response cache (10-05 Task 2) round-trip this projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DayForecast {
    /// The ISO-8601 calendar date for this daily entry (e.g. `"2026-06-24"`).
    date: String,
    /// The day's minimum temperature, in the top-level `unit`.
    temp_min: f64,
    /// The day's maximum temperature, in the top-level `unit`.
    temp_max: f64,
    /// The WMO-mapped conditions text for the day (e.g. `"Clear sky"`).
    conditions: String,
}

/// The response-cache projection (D-11): the fully-resolved values a render needs,
/// serialized as the cache payload. Caching the PROJECTION (not the raw Open-Meteo
/// JSON) means a cache hit skips geocoding too — the resolved `label`/`lat`/`lon` are
/// stored — and keeps the render path identical for a hit and a fresh fetch. Every
/// unit label was already read from the authoritative `current_units`/`daily_units`
/// (D-11) at fetch time. Round-trips via serde (`Serialize` on the cache write,
/// `Deserialize` on the read); a stored payload that no longer matches this shape is
/// treated as a MISS (a fresh fetch), never a trust of an unvalidated shape.
#[derive(Serialize, Deserialize)]
struct CachedWeather {
    /// The resolved location label (the `lat,lon` echo or the geocoded name).
    label: String,
    /// The resolved latitude (cached so a hit needs no geocode for the echo).
    lat: f64,
    /// The resolved longitude (cached so a hit needs no geocode for the echo).
    lon: f64,
    /// Current temperature, in the unit named by `temp_unit`.
    temperature: f64,
    /// The authoritative current temperature unit label (`°C`/`°F`).
    temp_unit: String,
    /// The WMO-mapped current conditions text.
    conditions: String,
    /// Current wind speed, in the unit named by `wind_unit`.
    wind_speed: f64,
    /// The authoritative current wind unit label (`km/h`/`mp/h`).
    wind_unit: String,
    /// Relative humidity as a percentage value.
    humidity: f64,
    /// The authoritative daily temp unit label for the human forecast render —
    /// `Some` only under `--forecast`.
    daily_temp_unit: Option<String>,
    /// The 7-day daily outlook — `Some` only under `--forecast`.
    forecast: Option<Vec<DayForecast>>,
}

/// Geocode a city `name` to `(lat, lon, "City, Region, Country")` via the
/// Open-Meteo geocoding API (D-12). The name is URL-encoded so reserved
/// characters (`&`/`=`/spaces) cannot inject extra query params
/// (T-05-WTHR-INJ). A no-match response OMITS the `results` key entirely
/// (Pitfall WTHR-2 — `#[serde(default)]` deserializes that to an empty Vec), so
/// an empty/absent `results` → `no location found` (exit 1).
fn geocode(name: &str) -> anyhow::Result<(f64, f64, String)> {
    let url = format!(
        "{}/v1/search?name={}&count=1&language=en&format=json",
        geocode_origin(),
        url_encode(name),
    );
    let resp: GeoResp = fetch(&url)?;
    let hit = resp
        .results
        .into_iter()
        .next()
        .with_context(|| format!("no location found for \"{name}\""))?;
    let label = format_geo_label(&hit);
    Ok((hit.latitude, hit.longitude, label))
}

/// The single most important weather pattern: one blocking GET whose result is
/// classified by ureq 3.x's DEFAULT status-as-error behavior (Pitfall WTHR-1).
/// `Ok` is ALWAYS 2xx → deserialize the body via `serde_json::from_reader` over
/// `into_body().into_reader()` (D-13; no ureq `json` feature). Non-2xx arrives as
/// `Err(Error::StatusCode(code))` → a clean exit-1 status message (NOT a
/// post-success `resp.status()` check). Everything else (Io / ConnectionFailed /
/// HostNotFound / catch-all) is the offline/DNS family → the graceful offline
/// error. Both error arms are plain `anyhow::bail!` → exit 1.
fn fetch<T: serde::de::DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    match ureq::get(url).call() {
        Ok(resp) => serde_json::from_reader(resp.into_body().into_reader())
            .context("parse weather response"),
        Err(ureq::Error::StatusCode(code)) => {
            anyhow::bail!("weather service returned {code}")
        }
        Err(_) => anyhow::bail!("could not reach weather service (offline?)"),
    }
}

/// The geocoding origin: `BOX_WEATHER_BASE_URL` if set (offline test seam), else
/// the real Open-Meteo geocoding host.
fn geocode_origin() -> String {
    std::env::var(BASE_URL_ENV).unwrap_or_else(|_| GEOCODE_ORIGIN.to_string())
}

/// The forecast origin: `BOX_WEATHER_BASE_URL` if set (offline test seam), else
/// the real Open-Meteo forecast host.
fn forecast_origin() -> String {
    std::env::var(BASE_URL_ENV).unwrap_or_else(|_| FORECAST_ORIGIN.to_string())
}

/// Map a WMO weather code to a short `&'static str` description (D-13). No alloc;
/// the `_` arm future-proofs any code Open-Meteo adds (T-05-WTHR-DoS). Exact
/// labels are discretion; the `_` fallthrough is mandated.
fn wmo_to_str(code: u32) -> &'static str {
    match code {
        0 => "Clear sky",
        1..=3 => "Partly cloudy",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        61 | 63 | 65 => "Rain",
        71 | 73 | 75 => "Snow",
        80..=82 => "Rain showers",
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
/// `Units::Imperial` (D-11/D-13) — the metric path omits them entirely. Pure
/// (aside from reading the origin env seam) → the imperial branch is unit-testable
/// without a network call.
///
/// The forecast origin comes from [`forecast_origin`] (the real Open-Meteo host,
/// or `BOX_WEATHER_BASE_URL` when the offline test seam is set).
fn build_forecast_url(lat: f64, lon: f64, units: Units, forecast: bool) -> String {
    let mut url = format!(
        "{}/v1/forecast?latitude={lat}&longitude={lon}\
         &current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m",
        forecast_origin(),
    );
    // `--forecast` (D-10): request the fixed 7-day daily block ALONGSIDE the current
    // block. The imperial `temperature_unit` param below applies to BOTH the current
    // and daily temps, so no separate daily-unit param is needed.
    if forecast {
        url.push_str("&daily=temperature_2m_max,temperature_2m_min,weather_code");
    }
    if matches!(units, Units::Imperial) {
        url.push_str("&temperature_unit=fahrenheit&wind_speed_unit=mph");
    }
    url
}

/// Project the parallel `daily` arrays into the bounded 7-day rows (D-10). Zips the
/// `time`/`temperature_2m_max`/`temperature_2m_min`/`weather_code` vectors by index,
/// mapping each `weather_code` through [`wmo_to_str`]. A short/mismatched daily block
/// (unequal array lengths, or an empty span) is a plain `Err` → exit 1, NEVER a panic
/// or an out-of-bounds index (T-10-05-HTTP: a malformed remote block is an error, not
/// a crash). Pure over its input → unit-testable from the offline fixtures.
fn build_day_forecasts(daily: &Daily) -> anyhow::Result<Vec<DayForecast>> {
    let n = daily.time.len();
    anyhow::ensure!(
        n > 0
            && daily.temperature_2m_max.len() == n
            && daily.temperature_2m_min.len() == n
            && daily.weather_code.len() == n,
        "malformed daily forecast: mismatched or empty daily arrays"
    );
    Ok((0..n)
        .map(|i| DayForecast {
            date: daily.time[i].clone(),
            temp_min: daily.temperature_2m_min[i],
            temp_max: daily.temperature_2m_max[i],
            conditions: wmo_to_str(daily.weather_code[i]).to_string(),
        })
        .collect())
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
/// label object (read the suffix from here, never hardcode — D-11). The `daily` +
/// `daily_units` blocks are present ONLY when the request asked for them (`--forecast`,
/// D-10); `#[serde(default)]` keeps the current-only response (no daily key)
/// deserializing into `None` rather than a missing-field error.
#[derive(Debug, Deserialize)]
struct ForecastResp {
    current: Current,
    current_units: CurrentUnits,
    /// The 7-day daily block — `Some` only under `--forecast`.
    #[serde(default)]
    daily: Option<Daily>,
    /// The daily unit-label object — `Some` only under `--forecast`.
    #[serde(default)]
    daily_units: Option<DailyUnits>,
}

/// The 7-day daily forecast arrays (D-10). Each field is a parallel `Vec` indexed by
/// day; the typed shape bounds the deserialize to these four arrays (never an
/// open-ended stream — T-10-05-HTTP). `build_day_forecasts` validates the lengths
/// match before zipping them, so a short/mismatched block is an error not a panic.
#[derive(Debug, Deserialize)]
struct Daily {
    /// ISO-8601 dates, one per forecast day.
    time: Vec<String>,
    /// Per-day maximum temperature (in the requested unit).
    temperature_2m_max: Vec<f64>,
    /// Per-day minimum temperature (in the requested unit).
    temperature_2m_min: Vec<f64>,
    /// Per-day WMO weather code (mapped to text via `wmo_to_str`).
    weather_code: Vec<u32>,
}

/// The daily unit-label object: e.g. `"°C"`/`"°F"` for the temps. Read the
/// authoritative temp label from here, NEVER hardcode (the SAME rule as
/// [`CurrentUnits`], D-11). Only `temperature_2m_max` is captured — the daily min
/// shares its unit, and this struct has no `deny_unknown_fields` so the extra
/// `temperature_2m_min` label in the JSON is harmlessly ignored.
#[derive(Debug, Deserialize)]
struct DailyUnits {
    /// The authoritative daily temp unit label (`"°C"`/`"°F"`), used for both extremes.
    temperature_2m_max: String,
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
        let metric = build_forecast_url(51.5, -0.13, Units::Metric, false);
        assert!(
            !metric.contains("temperature_unit=fahrenheit"),
            "metric must NOT request fahrenheit: {metric}"
        );
        assert!(
            !metric.contains("wind_speed_unit=mph"),
            "metric must NOT request mph: {metric}"
        );
        assert!(
            metric.contains(
                "current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m"
            ),
            "metric must request the current set: {metric}"
        );

        let imperial = build_forecast_url(51.5, -0.13, Units::Imperial, false);
        assert!(
            imperial.contains("temperature_unit=fahrenheit"),
            "imperial must request fahrenheit: {imperial}"
        );
        assert!(
            imperial.contains("wind_speed_unit=mph"),
            "imperial must request mph: {imperial}"
        );
    }

    /// `build_forecast_url` requests the 7-day `daily=` block ONLY under `--forecast`
    /// (D-10); the current-only path omits it entirely (so the pre-Phase-10 response
    /// shape is unchanged). The imperial unit param still rides alongside the daily
    /// block (one `temperature_unit` covers current AND daily).
    #[test]
    fn build_forecast_url_daily_param() {
        let daily_param = "daily=temperature_2m_max,temperature_2m_min,weather_code";

        // No --forecast → NO daily block (current-only shape unchanged).
        let current = build_forecast_url(51.5, -0.13, Units::Metric, false);
        assert!(
            !current.contains(daily_param),
            "current-only must NOT request the daily block: {current}"
        );

        // --forecast → the daily block is appended.
        let forecast = build_forecast_url(51.5, -0.13, Units::Metric, true);
        assert!(
            forecast.contains(daily_param),
            "--forecast must request the daily block: {forecast}"
        );

        // WR-01: the span is PINNED server-side — the --forecast URL carries
        // `forecast_days=7` so Open-Meteo's runtime default can never drift.
        assert!(
            forecast.contains("forecast_days=7"),
            "--forecast must pin the span with forecast_days=7: {forecast}"
        );
        // The current-only path must NOT pin a daily span.
        assert!(
            !current.contains("forecast_days=7"),
            "current-only must NOT request forecast_days=7: {current}"
        );

        // --forecast + imperial → both the daily block AND the imperial unit param.
        let imperial = build_forecast_url(51.5, -0.13, Units::Imperial, true);
        assert!(
            imperial.contains(daily_param) && imperial.contains("temperature_unit=fahrenheit"),
            "--forecast imperial must request daily + fahrenheit: {imperial}"
        );
        assert!(
            imperial.contains("forecast_days=7"),
            "--forecast imperial must still pin forecast_days=7: {imperial}"
        );
    }

    /// The metric 7-day fixture deserializes into `ForecastResp` with a `daily` +
    /// `daily_units` block; `build_day_forecasts` projects exactly 7 rows, mapping
    /// each `weather_code` through `wmo_to_str`, and the AUTHORITATIVE daily temp
    /// label is `"°C"` read from `daily_units` (D-10/D-11 — never hardcoded).
    #[test]
    fn forecast_metric_7day_fixture_projects_7_rows() {
        let raw = include_str!("../../../tests/fixtures/weather/forecast_metric_7day.json");
        let f: ForecastResp = serde_json::from_str(raw).expect("metric 7-day forecast parses");

        let daily = f.daily.as_ref().expect("daily block present");
        let daily_units = f.daily_units.as_ref().expect("daily_units block present");
        assert_eq!(
            daily_units.temperature_2m_max, "°C",
            "the daily temp label is read from daily_units (authoritative, never hardcoded)"
        );

        let days = build_day_forecasts(daily).expect("daily arrays project");
        assert_eq!(days.len(), 7, "a fixed 7-day span");
        assert_eq!(days[0].date, "2026-06-24");
        assert_eq!(days[0].temp_min, 12.0);
        assert_eq!(days[0].temp_max, 22.0);
        // weather_code 0 → "Clear sky"; code 95 (day 6) → "Thunderstorm".
        assert_eq!(days[0].conditions, "Clear sky");
        assert_eq!(days[5].conditions, "Thunderstorm");
    }

    /// The imperial 7-day fixture's daily temp label is `"°F"` — proving the daily
    /// render reads `daily_units`, not a hardcoded suffix (Pitfall WTHR-3 applied to
    /// the daily block).
    #[test]
    fn forecast_imperial_7day_fixture_label_is_fahrenheit() {
        let raw = include_str!("../../../tests/fixtures/weather/forecast_imperial_7day.json");
        let f: ForecastResp = serde_json::from_str(raw).expect("imperial 7-day forecast parses");
        let daily_units = f.daily_units.as_ref().expect("daily_units block present");
        assert_eq!(
            daily_units.temperature_2m_max, "°F",
            "the imperial daily temp label is °F, read from daily_units"
        );
    }

    /// A malformed daily block (mismatched array lengths) is an `Err`, NEVER a panic
    /// or an out-of-bounds index (T-10-05-HTTP — a bad remote block degrades to a
    /// clean exit-1 error).
    #[test]
    fn build_day_forecasts_rejects_mismatched_arrays() {
        let bad = Daily {
            time: vec!["2026-06-24".into(), "2026-06-25".into()],
            temperature_2m_max: vec![22.0], // shorter than `time`
            temperature_2m_min: vec![12.0, 11.0],
            weather_code: vec![0, 1],
        };
        assert!(
            build_day_forecasts(&bad).is_err(),
            "a mismatched daily block must be an error, not a panic"
        );
    }

    /// WR-01 — an OVERSIZED daily block (all four arrays a matched length 8) is an
    /// `Err`, not projected: the "bounded 7-day" invariant is now defensively
    /// enforced (`n <= 7`), so an over-7-day (or anomalously large) well-formed
    /// response is a clean exit-1 error, never rendered under the "7-day forecast:"
    /// header or serialized into the `--json` `forecast` array (T-10-05-HTTP: bound
    /// the SIZE, not just the shape).
    #[test]
    fn build_day_forecasts_rejects_oversized_arrays() {
        let dates: Vec<String> = (24..32).map(|d| format!("2026-06-{d}")).collect();
        let oversized = Daily {
            time: dates,                              // 8 days
            temperature_2m_max: vec![22.0; 8],        // matched length 8
            temperature_2m_min: vec![12.0; 8],        // matched length 8
            weather_code: vec![0; 8],                 // matched length 8
        };
        assert_eq!(oversized.time.len(), 8, "the fixture is an 8-day block");
        assert!(
            build_day_forecasts(&oversized).is_err(),
            "an 8-day (oversized) daily block must be an error, not a 7-day-bound violation"
        );
    }

    /// A current-only forecast fixture (no `daily` key) still deserializes: the
    /// `#[serde(default)]` on `daily`/`daily_units` yields `None` rather than a
    /// missing-field error, so the non-`--forecast` path is unaffected.
    #[test]
    fn current_only_fixture_has_no_daily() {
        let raw = include_str!("../../../tests/fixtures/weather/forecast_metric.json");
        let f: ForecastResp = serde_json::from_str(raw).expect("current-only forecast parses");
        assert!(f.daily.is_none(), "current-only response has no daily block");
        assert!(f.daily_units.is_none(), "current-only response has no daily_units");
    }

    /// D-12 — `resolve_units` proves CLI > config > metric builtin as a known-answer
    /// matrix (the SPINE-05 `cli.or(cfg).unwrap_or(builtin)` shape), terminal-free.
    #[test]
    fn resolve_units_precedence() {
        // CLI wins over config and the builtin.
        assert_eq!(
            resolve_units(Some(Units::Imperial), Some(Units::Metric)),
            Units::Imperial
        );
        assert_eq!(resolve_units(Some(Units::Metric), None), Units::Metric);
        // With no CLI, the config value wins over the builtin.
        assert_eq!(resolve_units(None, Some(Units::Imperial)), Units::Imperial);
        // Neither → the metric builtin.
        assert_eq!(resolve_units(None, None), Units::Metric);
    }

    /// D-12 — `resolve_location` proves the CLI positional beats the config location,
    /// config is used when the positional is absent, and neither → `None` (which the
    /// caller maps to the exit-2 `MissingLocation`).
    #[test]
    fn resolve_location_precedence() {
        assert_eq!(
            resolve_location(Some("London".into()), Some("Paris".into())).as_deref(),
            Some("London"),
            "the CLI positional beats the config location"
        );
        assert_eq!(
            resolve_location(None, Some("Paris".into())).as_deref(),
            Some("Paris"),
            "with no positional, the config location is used"
        );
        assert_eq!(
            resolve_location(None, None),
            None,
            "neither → None (the caller maps this to exit 2)"
        );
    }

    /// The cache-key unit tokens are the stable lowercase spellings, so a
    /// metric key and an imperial key for the same location never collide.
    #[test]
    fn units_key_tokens() {
        assert_eq!(units_key(Units::Metric), "metric");
        assert_eq!(units_key(Units::Imperial), "imperial");
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
