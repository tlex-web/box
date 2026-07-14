---
phase: 10-fun-system-depth
reviewed: 2026-07-14T00:00:00Z
depth: standard
files_reviewed: 30
files_reviewed_list:
  - .gitattributes
  - Cargo.toml
  - src/cli.rs
  - src/main.rs
  - src/commands/cowsay/mod.rs
  - src/commands/eight_ball/mod.rs
  - src/commands/fortune/mod.rs
  - src/commands/hash/mod.rs
  - src/commands/pomodoro/mod.rs
  - src/commands/roast/mod.rs
  - src/commands/weather/mod.rs
  - src/core/cache.rs
  - src/core/config.rs
  - src/core/errors.rs
  - src/core/mod.rs
  - src/data/fortunes/humor.txt
  - src/data/fortunes/tech.txt
  - src/data/fortunes/wisdom.txt
  - src/data/roasts/general.txt
  - src/data/roasts/javascript.txt
  - src/data/roasts/python.txt
  - src/data/roasts/rust.txt
  - tests/config.rs
  - tests/cowsay.rs
  - tests/eight_ball.rs
  - tests/fixtures/weather/forecast_imperial_7day.json
  - tests/fixtures/weather/forecast_metric_7day.json
  - tests/fortune.rs
  - tests/pomodoro.rs
  - tests/roast.rs
  - tests/weather.rs
findings:
  critical: 0
  critical_resolved: 1
  warning: 0
  warning_resolved: 2
  info: 0
  total: 3
status: resolved
resolution_note: "CR-01 fixed post-review (hash help/doc strings cite [hash] default_algo). WR-01 + WR-02 closed by plan 10-06 gap-closure (2026-07-14): forecast_days=7 server-side pin + n<=7 defensive bound (WR-01); location trim + city-only cache-key normalization (WR-02). All findings resolved."
---

# Phase 10: Code Review Report

**Reviewed:** 2026-07-14T00:00:00Z
**Depth:** standard
**Files Reviewed:** 30
**Status:** issues_found

## Summary

Reviewed the full phase-10 source diff (`d2802e3..HEAD`, plans 10-01..10-05): cowsay
figure roster/`--think`, 8ball ASCII art + sentiment, fortune/roast category/language
buckets, pomodoro auto-cycle + `--sound` (Win32 `MessageBeep` unsafe FFI), the D-13
nested `[hash]`/`[weather]` config migration, the new best-effort `core::cache`
module, and weather's `--forecast` + cache wiring.

The security-sensitive surfaces hold up well under adversarial reading: the cache's
path-traversal mitigation (blake3-hashing the raw key into a fixed hex filename) is
sound and every failure mode (absent/stale/malformed/permission-denied) degrades to a
MISS as claimed, never a panic or an `Err`; the `unsafe { MessageBeep(MB_OK) }` block
takes no untrusted input and is properly `cfg`-gated with a no-op non-Windows stub;
`is_color_on()` gating is applied consistently everywhere ANSI can leak (cowsay stays
pure ASCII by design, 8ball/pomodoro/weather all gate correctly, and the colored
branches that bypass `out_line` are provably unreachable under `--json`/`--clip`
because `init_output` forces `COLOR_ON` off in both cases); JSON purity holds — the
weather `--json` stdout never receives the stderr location echo or any cache
chatter; the fortune/roast/cowsay corpora and default behavior are byte-identical to
the pre-phase content (diffed and confirmed).

Two real defects were found: a config-migration doc/help-text regression that can
lock every `box` subcommand behind an exit-2 config error for a user who follows the
tool's own `--help` text (CR-01), and a "bounded 7-day forecast" claim that the
implementation does not actually enforce (WR-01). A minor cache-key normalization gap
rounds out the findings (WR-02).

## Critical Issues

### CR-01: `box hash --help` still tells users to set the now-rejected top-level `default_hash_algo` config key

> **✅ RESOLVED (2026-07-14):** The three stale strings in `src/commands/hash/mod.rs` (module doc line 12, the `--algo` field help at line 81, and the `Algo` enum doc at line 108) now cite `[hash] default_algo`. Verified against live `box hash --help` output; full `cargo test` (29 suites) + `cargo clippy -D warnings` green. The two `default_hash_algo` mentions remaining in `config.rs` are intentional — they document the migration itself.

**File:** `src/commands/hash/mod.rs:80-82` (also stale at lines 12 and 108)
**Issue:**
D-13 (this phase, `src/core/config.rs`) migrated the flat `default_hash_algo` config
key into the nested `[hash] default_algo` table, and the top-level `Config` struct
carries `#[serde(deny_unknown_fields)]`. A stray top-level `default_hash_algo` key is
now an **unknown field** that makes `core::config::load()` return
`BoxError::Config` → **exit 2 for every `box` subcommand**, not just `hash`, because
config is loaded once in `main()` before dispatch (`src/main.rs:81`).

The `run_compute` resolver itself was correctly updated to read
`config().hash.default_algo` (`src/commands/hash/mod.rs:299-307`), but the
**user-facing `--algo` flag help text** (a `///` doc comment on the `algo` field,
which clap renders verbatim in `box hash --help`) was not updated:

```rust
    /// Hash algorithm. Unset means BLAKE3 when computing (the v2 default — D-04;
    /// pass `--algo sha256` or set `default_hash_algo` in the config to restore
    /// SHA-256), or (under `--verify`) auto-detect by the digest's hex length. An
```

A user who runs `box hash --help`, reads this text, and follows it literally will
write `default_hash_algo = "sha256"` at the top level of `%APPDATA%\box\config.toml`
(the help text gives no indication it must live inside a `[hash]` table). The very
next invocation of **any** `box` command — not just `hash` — exits 2 with a config
error, until the user manually discovers and fixes the file. The tool's own shipped
documentation actively walks a user into breaking the tool. The same stale key name
also appears in the module-level doc comment (line 12: `config (default_hash_algo)`)
and the `Algo` enum's doc comment (line 108: `lets the config value
default_hash_algo = "sha256" parse`), both of which reinforce the wrong key name for
anyone reading `cargo doc` or the source.

**Fix:**
```rust
    /// Hash algorithm. Unset means BLAKE3 when computing (the v2 default — D-04;
    /// pass `--algo sha256` or set `[hash] default_algo = "sha256"` in the config
    /// to restore SHA-256), or (under `--verify`) auto-detect by the digest's hex
    /// length. An EXPLICIT `--algo` ALWAYS wins — it is never overridden by length
    /// auto-detection (WR-01).
    #[arg(long, value_enum)]
    pub algo: Option<Algo>,
```
Also update line 12 (`config (default_hash_algo)` → `config ([hash] default_algo)`)
and line 108 (`default_hash_algo = "sha256"` → `[hash]\ndefault_algo = "sha256"`) so
every reference to the escape hatch names the correct nested key. Consider adding a
regression test asserting `box hash --help` does not contain the bare string
`default_hash_algo` (only `[hash]` / `default_algo`).

## Warnings

### WR-01: `--forecast` daily array is not actually bounded to 7 days despite the code/docs repeatedly claiming it is

> **✅ RESOLVED (2026-07-14, plan 10-06):** `build_forecast_url` now pins `&forecast_days=7` server-side (`src/commands/weather/mod.rs:513`, current-only omits it) and `build_day_forecasts`'s `ensure!` gained `&& n <= 7` (mod.rs:534) — an over-7-day (or anomalously large) well-formed daily block is now a clean exit-1 error, never rendered under the `"  7-day forecast:"` header nor serialized into the `--json` `forecast` array. Three stale "bounded/fixed 7-day" doc comments de-overclaimed. Covered by `forecast_json_has_7_day_array` + the `forecast_days=7` URL-pin assertions; full `cargo test` + `cargo clippy --all-targets -D warnings` green.

**File:** `src/commands/weather/mod.rs:473-489, 497-514`
**Issue:**
The module doc, the `render` doc comment, and multiple test names assert the daily
forecast is "the bounded 7-day rows" / "a fixed 7-day span" / "the fixed 7-day
daily block" (e.g. `build_day_forecasts`'s doc comment, `forecast_metric_7day_fixture_projects_7_rows`).
In practice:

1. `build_forecast_url` never sends Open-Meteo's `forecast_days` parameter, so the
   day count is whatever the API's undocumented-in-code default happens to be at
   request time — there is no explicit `&forecast_days=7` pinning the span the code
   assumes everywhere else.
2. `build_day_forecasts`'s validation (`anyhow::ensure!`) only checks that the four
   parallel arrays are non-empty and equal-length to each other — it never checks
   `n <= 7` (or any upper bound). A response with, say, 16 days (Open-Meteo supports
   up to 16 for `daily`) or an anomalously large array would be accepted, projected,
   rendered under a hard-coded `"  7-day forecast:"` header (`render`,
   `src/commands/weather/mod.rs:278`), and serialized verbatim into the `--json`
   `forecast` array — silently contradicting both the UI label and the documented/
   tested "bounded 7-day" invariant.

This is reachable from a real, non-malicious drift (Open-Meteo changing its default
`forecast_days`) as well as from the `BOX_WEATHER_BASE_URL` test seam pointed at an
untrusted origin, and it means the "T-10-05-HTTP: a malformed remote block is an
error, not a crash" mitigation only bounds *mismatched* lengths, not the *size* of an
otherwise well-formed response.

**Fix:** Either request the span explicitly so the server-side default can never
drift:
```rust
url.push_str("&daily=temperature_2m_max,temperature_2m_min,weather_code&forecast_days=7");
```
and/or enforce the upper bound defensively in `build_day_forecasts`, matching the
"bounded" claim the surrounding docs and tests already make:
```rust
anyhow::ensure!(
    n > 0 && n <= 7
        && daily.temperature_2m_max.len() == n
        && daily.temperature_2m_min.len() == n
        && daily.weather_code.len() == n,
    "malformed daily forecast: mismatched, empty, or oversized daily arrays"
);
```

### WR-02: Weather cache key and geocode lookup use the raw, un-normalized location string

> **✅ RESOLVED (2026-07-14, plan 10-06):** `run()` trims the location once (`src/commands/weather/mod.rs:114`) as the single source for both the cache key and `fetch_weather`; a new pure `location_key` helper (mod.rs:196) trims + lowercases CITY names while leaving `lat,lon` pairs verbatim (so `parse_lat_lon` still sees the numeric value), making `"London"` / `" London "` / `"london"` share one cache key and the ~10-min window. Covered by `whitespace_variants_share_cache_key`, `second_identical_call_is_a_cache_hit`, and `location_key_tokens`.

**File:** `src/commands/weather/mod.rs:102-120, 389-403`
**Issue:** `resolve_location` returns the CLI/config location string unchanged (no
`.trim()`), and it flows straight into both `cache_key` (`format!("{location}|...")`)
and `geocode(name)` / `parse_lat_lon(location)` without normalization beyond what
`parse_lat_lon`'s own regex trims internally for the `lat,lon` shape. Two
functionally-identical invocations that differ only in incidental whitespace or case
(`box weather "London"` vs `box weather " London "` vs `box weather "london"`) hash
to different cache keys and are treated as unrelated entries — each pays a fresh
network round-trip and geocode instead of sharing the ~10-minute cache window the
feature is meant to provide. This is not a crash or data-integrity issue (the cache
module itself is sound), just a quiet miss-rate gap in the new caching feature.
**Fix:** Normalize the location once before it is used for both the cache key and the
geocode/parse paths, e.g. `let location = location.trim().to_string();` right after
`resolve_location`, and consider lower-casing city names (not `lat,lon` pairs) before
hashing into the cache key.

---

_Reviewed: 2026-07-14T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
