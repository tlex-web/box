//! Config-file defaults for `box` (SPINE-05, D-1).
//!
//! `box` reads an optional TOML config from `%APPDATA%\box\config.toml`
//! (resolved by [`config_path`] — `%APPDATA%` env var first, [`dirs::config_dir`]
//! as the fallback; see that fn for the Rule 1 rationale) **once** at startup
//! ([`init_config`]) and installs it as a process-global `OnceLock<Config>`. This mirrors
//! [`crate::core::output`]'s "decide once in `main()`" shape — the `COLOR_ON`
//! atomic + `init_color` writer + `is_color_on` reader — promoted from an
//! `AtomicBool` to an `OnceLock<Config>`.
//!
//! Precedence is **CLI > env > config > builtin**, resolved by construction with
//! `Option::or` chains (`cli.or(env).or(cfg).unwrap_or(builtin)`): every
//! config-overridable flag is an `Option<T>` with NO clap `default_value`, so
//! `Some` means "a tier set this" and `None` means "fall through" (SPINE-05,
//! Anti-Pattern 3). See [`resolve_algo`] for the canonical resolver shape.
//!
//! Two file states, two behaviors (D-10 / SPINE-05):
//! - **Missing** file → silent [`Config::default`]; a normal `box uuid` never
//!   errors just because no config exists. Handled TOCTOU-free by matching
//!   [`std::io::ErrorKind::NotFound`] on the read, never `exists()`-then-read.
//! - **Malformed / unknown-key** file → a clean [`BoxError::Config`] → exit 2,
//!   aborting BEFORE dispatch. `deny_unknown_fields` rejects stray keys.

use std::sync::OnceLock;

use anyhow::Context;

use crate::commands::hash::Algo;
use crate::commands::weather::Units;
use crate::core::errors::BoxError;

/// Config-overridable settings (SPINE-05), grouped into per-command nested TOML
/// tables (D-13). Every leaf field is `Option<T>` with NO clap `default_value` so
/// `Some` = user/file set it, `None` = fall through to a lower precedence tier
/// (Anti-Pattern 3). Each sub-struct also carries [`Default`] so a whole table may
/// be absent (→ all-`None`), while the fields keep their `Some`/`None` semantics.
///
/// `#[serde(default, deny_unknown_fields)]` (top level AND every sub-struct): a
/// missing key/table deserializes to `None`/`Default` silently, an unknown key —
/// top-level OR nested — is a loud error (→ [`BoxError::Config`], exit 2, D-10).
///
/// D-13 migrated the Phase-6 FLAT `default_hash_algo` key into `[hash] default_algo`
/// and added the `[weather]` table (the schema Phase 11's `box config get/set
/// weather.location` locks against). The one-time break to any hand-authored flat
/// `config.toml` is accepted (a stray top-level `default_hash_algo` is now an
/// unknown key → exit 2).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// `[hash]` — the `hash` command's config defaults. Skipped on serialize when
    /// empty (all-`None`) so a single-key `config set` write never emits a bare
    /// `[hash]` header (D-02 minimal-write; the empty table re-parses clean either
    /// way — this just keeps the on-disk file tidy).
    #[serde(skip_serializing_if = "HashConfig::is_empty")]
    pub hash: HashConfig,
    /// `[weather]` — the `weather` command's config defaults (consumed by 10-05).
    /// Skipped on serialize when empty (all-`None`) — see [`Config::hash`].
    #[serde(skip_serializing_if = "WeatherConfig::is_empty")]
    pub weather: WeatherConfig,
}

/// The `[hash]` table (D-13). Carries the BLAKE3-default escape hatch:
/// `[hash] default_algo = "sha256"` restores SHA-256 (config beats the built-in
/// BLAKE3; CLI still beats config).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HashConfig {
    /// The default `hash` algorithm restored from config — the `[hash] default_algo`
    /// escape hatch for the BLAKE3-default breaking change. Skipped on serialize
    /// when unset (D-02) so `config set weather.units …` never writes a stray empty
    /// `default_algo` key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_algo: Option<Algo>,
}

impl HashConfig {
    /// True when every leaf is unset — used by [`Config`]'s `skip_serializing_if`
    /// so an all-`None` `[hash]` table is omitted entirely on write (D-02).
    fn is_empty(&self) -> bool {
        self.default_algo.is_none()
    }
}

/// The `[weather]` table (D-13). Holds the stored-default location and unit system
/// the weather depth work (10-05) resolves through the config-precedence chain.
/// `units` is the typed [`Units`] enum (imported exactly like [`Algo`]) so an
/// invalid value is a loud config error rather than a silently-ignored string.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WeatherConfig {
    /// `[weather] location` — the stored default city / `lat,lon` used when the
    /// CLI positional is omitted (10-05 wires the `cli.or(config)` resolution).
    /// Skipped on serialize when unset (D-02).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// `[weather] units` — the stored default unit system (`metric`/`imperial`),
    /// deserialized via the lowercase serde rename on [`Units`]. Skipped on
    /// serialize when unset (D-02).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Units>,
}

impl WeatherConfig {
    /// True when every leaf is unset — used by [`Config`]'s `skip_serializing_if`
    /// so an all-`None` `[weather]` table is omitted entirely on write (D-02).
    fn is_empty(&self) -> bool {
        self.location.is_none() && self.units.is_none()
    }
}

/// Process-global config, set once by [`init_config`].
static CONFIG: OnceLock<Config> = OnceLock::new();

/// The loaded config. Panics if [`init_config`] has not run — it is called once in
/// `main()` before dispatch, so any command that reaches this has a config.
///
/// Live as of Plan 06-02 (nested since D-13): `hash` reads `config().hash.default_algo`
/// in its compute-default precedence chain, so the forward-compat `#[allow(dead_code)]`
/// has been removed (allow-then-remove, mirroring errors.rs's `MissingInput`
/// history), restoring the strict dead-code gate.
pub fn config() -> &'static Config {
    CONFIG
        .get()
        .expect("init_config() must run before config()")
}

/// Load the config once in `main()` BEFORE dispatch (and before `init_color`, per
/// the `main` ordering). A missing file yields a silent [`Config::default`]
/// (SPINE-05); a malformed file yields [`BoxError::Config`] → exit 2 (D-10), routed
/// through the same exit-code downcast `main()` already owns.
pub fn init_config() -> anyhow::Result<()> {
    let _ = CONFIG.set(load()?);
    Ok(())
}

/// Resolve `config_path()`, then read + parse it. Missing file → `Config::default()`
/// (TOCTOU-free: match `NotFound`, never `exists()`-then-read); malformed →
/// [`BoxError::Config`]; any other I/O error → a contextual `anyhow` error (exit 1).
fn load() -> anyhow::Result<Config> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).map_err(|e| {
            BoxError::Config {
                path: path.display().to_string(),
                message: e.to_string(),
            }
            .into()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// `%APPDATA%\box\config.toml` on Windows.
///
/// Resolution order (Rule 1 deviation from the planned `dirs`-first form —
/// documented in 06-01-SUMMARY): `%APPDATA%` env var FIRST, then [`dirs::config_dir`]
/// as the fallback. The plan locked `dirs::config_dir()` with `var_os("APPDATA")`
/// as "the documented fallback only — do not use it unless dirs resists." **dirs
/// resists here:** on Windows `dirs` 6.0 → `dirs-sys` 0.5 resolves `config_dir()`
/// via the `SHGetKnownFolderPath` Known-Folder API, which IGNORES the `APPDATA`
/// environment variable. That makes per-process config isolation impossible — the
/// integration tests (and any user/CI that relies on `APPDATA`) cannot point the
/// lookup at a temp dir. Reading `APPDATA` first restores the standard,
/// per-process-overridable Windows roaming-appdata location (identical
/// `%APPDATA%\box\config.toml` target) while keeping `dirs` for the non-Windows /
/// `APPDATA`-unset case.
pub fn config_path() -> Option<std::path::PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(
            std::path::PathBuf::from(appdata)
                .join("box")
                .join("config.toml"),
        );
    }
    dirs::config_dir().map(|p| p.join("box").join("config.toml"))
}

/// The closed set of keys `box config get`/`set` may touch (D-04/D-13). Exactly the
/// three leaves of the typed [`Config`] schema — `set` rejects any key outside this
/// registry with [`BoxError::ConfigUsage`] (exit 2, NOTHING written) BEFORE
/// touching the disk, and the same list is the "did you mean" surface for both
/// `set` and `get`. A key is only settable if it has a backing `Config` field to
/// round-trip against (D-05: `color` is deliberately absent — no schema field
/// exists for it).
///
/// Forward-compat `#[allow(dead_code)]`: exercised by unit tests now; the `config`
/// command (Task 2) consumes it as the live `get`/`set` registry, at which point the
/// allow is removed (allow-then-remove, mirroring [`resolve_algo`]).
#[allow(dead_code)]
pub const SETTABLE_KEYS: [&str; 3] = ["hash.default_algo", "weather.location", "weather.units"];

/// Validate-before-write a single config key (D-03/D-04): the CFG-01 `config set`
/// write path.
///
/// Steps, in order, so a self-inflicted exit-2 lockout is **structurally
/// impossible** (T-11-02):
/// 1. Reject any `key` not in [`SETTABLE_KEYS`] with [`BoxError::ConfigUsage`] whose
///    message names the unknown key and lists the settable keys (→ exit 2, nothing
///    written).
/// 2. Reconstruct the target document from the startup-loaded [`config`] snapshot,
///    insert `field = value` under its sub-table, and re-serialize.
/// 3. Re-parse the reconstructed text through the SAME `toml::from_str::<Config>`
///    the startup `load` uses. Any parse / invalid-enum / unknown-key failure maps
///    to [`BoxError::Config`] (→ exit 2) and writes NOTHING — the typed `Algo`/
///    `Units` enums + `deny_unknown_fields` do the value/key validation for free.
/// 4. ONLY on a clean re-parse, [`crate::core::fs::atomic_write`] the validated text
///    to [`config_path`] (temp-write + rename; parent dir created). A `None` path
///    is an `anyhow` error (exit 1).
///
/// Forward-compat `#[allow(dead_code)]`: the `config set` command (Task 2) is the
/// live consumer; the allow is removed there (allow-then-remove).
#[allow(dead_code)]
pub fn set_value(key: &str, value: &str) -> anyhow::Result<()> {
    // Validate (unknown key OR bad value) and reconstruct the full document from the
    // startup-loaded snapshot BEFORE any disk touch — the returned text is already
    // startup-safe (D-03).
    let text = build_config_toml(config(), key, value)?;
    // Only a validated document reaches the disk. A `None` path (no %APPDATA%, no
    // dirs fallback) is a runtime error → exit 1, not a usage error.
    let path = config_path().context("could not resolve the config file path")?;
    crate::core::fs::atomic_write(&path, &text)
}

/// The pure core of [`set_value`] (D-03): validate `key`, splice `value` into a
/// reconstruction of `base`, re-parse it through the SAME `toml::from_str::<Config>`
/// startup uses, and return the validated TOML text — WITHOUT touching the disk.
/// Returns [`BoxError::ConfigUsage`] for an unknown key and [`BoxError::Config`] for
/// a bad value / bad TOML, so `set_value` can `atomic_write` the returned text
/// knowing it is already startup-safe. Pure + `config()`-free so the whole
/// validation contract is unit-testable without `init_config` or a real file.
///
/// Forward-compat `#[allow(dead_code)]`: reachable via [`set_value`] once the
/// `config` command wires it (Task 2); exercised by unit tests now.
#[allow(dead_code)]
fn build_config_toml(base: &Config, key: &str, value: &str) -> anyhow::Result<String> {
    // 1. Closed-registry gate (D-04): reject an unknown key BEFORE any work, naming
    //    it and listing the settable keys — exit 2 via ConfigUsage, nothing built.
    if !SETTABLE_KEYS.contains(&key) {
        return Err(BoxError::ConfigUsage {
            message: format!(
                "unknown config key '{key}'; settable keys: {}",
                SETTABLE_KEYS.join(", ")
            ),
        }
        .into());
    }
    // A SETTABLE_KEYS entry is always a `table.field` pair (guarded by a unit test).
    let (table_name, field) = key
        .split_once('.')
        .expect("SETTABLE_KEYS entries are table.field pairs");

    // 2. Reconstruct the current document as a TOML table (so existing keys in other
    //    tables/fields are preserved), then splice in the new leaf as a string. The
    //    typed enums validate the VALUE in step 3's re-parse, not here.
    let base_text = toml::to_string(base).context("serializing the current config")?;
    let mut doc: toml::Table =
        toml::from_str(&base_text).context("re-parsing the current config")?;
    let sub = doc
        .entry(table_name)
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let sub_table = sub
        .as_table_mut()
        .with_context(|| format!("config section [{table_name}] is not a table"))?;
    sub_table.insert(field.to_string(), toml::Value::String(value.to_string()));

    let text = toml::to_string(&doc).context("serializing the updated config")?;

    // 3. Validate-before-write (D-03): re-parse through the SAME
    //    `toml::from_str::<Config>` the startup `load` uses. Any parse / invalid-enum
    //    / unknown-key failure maps to BoxError::Config (exit 2) and returns Err, so
    //    the caller NEVER writes — a self-inflicted exit-2 lockout is structurally
    //    impossible (T-11-02).
    toml::from_str::<Config>(&text).map_err(|e| BoxError::Config {
        path: config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "config.toml".to_string()),
        message: e.to_string(),
    })?;

    Ok(text)
}

/// The canonical config-precedence resolver (SPINE-05): **CLI > env > config >
/// builtin**, by construction. Pure and terminal-free so the precedence matrix is
/// deterministic. Every Phase-7 config-overridable flag resolves through this exact
/// `cli.or(env).or(cfg).unwrap_or(builtin)` shape — the builtin here is the
/// BLAKE3-default (D-04).
///
/// Forward-compat `#[allow(dead_code)]`: exercised by the `precedence_matrix` unit
/// test now; `hash` adopts it as the live resolver in Plan 06-02 (allow removed there).
#[allow(dead_code)]
pub fn resolve_algo(cli: Option<Algo>, env: Option<Algo>, cfg: Option<Algo>) -> Algo {
    cli.or(env).or(cfg).unwrap_or(Algo::Blake3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::weather::Units;

    /// SPINE-05 / Pitfall 3 — prove CLI > env > config > builtin as a known-answer
    /// matrix, terminal-free and without touching a real config file. Runnable via
    /// `cargo test --bin box precedence_matrix`.
    #[test]
    fn precedence_matrix() {
        let c = Some(Algo::Md5);
        let e = Some(Algo::Sha512);
        let f = Some(Algo::Sha256);

        // CLI Some wins over everything below it.
        assert_eq!(resolve_algo(c, e, f), Algo::Md5);
        assert_eq!(resolve_algo(c, None, None), Algo::Md5);

        // With CLI None, env wins over config + builtin.
        assert_eq!(resolve_algo(None, e, f), Algo::Sha512);
        assert_eq!(resolve_algo(None, e, None), Algo::Sha512);

        // With CLI + env None, config wins over builtin.
        assert_eq!(resolve_algo(None, None, f), Algo::Sha256);

        // All None → builtin (Algo::Blake3, the D-04 default).
        assert_eq!(resolve_algo(None, None, None), Algo::Blake3);
    }

    /// A valid NESTED config round-trips the lowercase TOML values into the typed
    /// `Option` fields (D-13): `[hash] default_algo` parses into `Option<Algo>`
    /// (the `#[serde(rename_all = "lowercase")]` on `Algo` doing its job), and an
    /// empty config leaves every nested field `None` (silent default).
    #[test]
    fn valid_config_parses() {
        let cfg: Config = toml::from_str("[hash]\ndefault_algo = \"sha256\"").unwrap();
        assert_eq!(cfg.hash.default_algo, Some(Algo::Sha256));

        // An empty config is valid: every nested key → None (silent default).
        let empty: Config = toml::from_str("").unwrap();
        assert_eq!(empty.hash.default_algo, None);
        assert_eq!(empty.weather.location, None);
        assert_eq!(empty.weather.units, None);
    }

    /// D-13 — `[weather] location`/`units` parse into the nested `WeatherConfig`;
    /// the units value round-trips through the lowercase serde rename on
    /// `weather::Units` (mirroring `hash::Algo`), so `[weather] units = "imperial"`
    /// deserializes to `Some(Units::Imperial)`.
    #[test]
    fn weather_nested_parses() {
        let cfg: Config =
            toml::from_str("[weather]\nlocation = \"London\"\nunits = \"imperial\"").unwrap();
        assert_eq!(cfg.weather.location.as_deref(), Some("London"));
        assert_eq!(cfg.weather.units, Some(Units::Imperial));

        // The other spelling round-trips too.
        let metric: Config = toml::from_str("[weather]\nunits = \"metric\"").unwrap();
        assert_eq!(metric.weather.units, Some(Units::Metric));
    }

    /// D-10 / D-13 — malformed TOML, an unknown key (top-level OR nested under
    /// `deny_unknown_fields`), and an invalid enum value (`units = "kelvin"`) all
    /// map to [`BoxError::Config`] via the same `load`-style mapping, so `main()`'s
    /// downcast routes each to exit 2. Asserts the downcast is `BoxError::Config`.
    #[test]
    fn malformed_maps_to_config_error() {
        // Mirror `load`'s mapping: toml::from_str error → BoxError::Config.
        let map = |toml_src: &str| -> anyhow::Error {
            let parsed: Result<Config, _> = toml::from_str(toml_src);
            let err = parsed.expect_err("expected a parse error");
            anyhow::Error::from(BoxError::Config {
                path: "test/config.toml".to_string(),
                message: err.to_string(),
            })
        };
        let is_config_err = |err: anyhow::Error, what: &str| {
            assert!(
                matches!(err.downcast_ref::<BoxError>(), Some(BoxError::Config { .. })),
                "{what} must downcast to BoxError::Config, got: {err:#}"
            );
        };

        // Syntactically invalid TOML (a value is missing).
        is_config_err(map("[hash]\ndefault_algo = "), "malformed TOML");
        // An unknown TOP-LEVEL key under deny_unknown_fields.
        is_config_err(map("bogus_key = 1"), "unknown top-level key");
        // An unknown key NESTED inside a known table (HashConfig also denies).
        is_config_err(map("[hash]\nbogus = 1"), "unknown nested key");
        // An invalid units enum value — kelvin is not metric|imperial.
        is_config_err(map("[weather]\nunits = \"kelvin\""), "invalid units value");
    }

    // --- CFG-01 (11-01): set_value validate-before-write (D-03/D-04) -----------
    //
    // These drive the PURE `build_config_toml` core with `&Config::default()` as the
    // base, so the full validation contract is exercised without `init_config` /
    // `config()` / a real file. `set_value` itself (which calls `config()` +
    // `atomic_write`) is covered black-box in `tests/config_cmd.rs` (Task 3).

    /// D-03 — a valid `weather.units` set produces TOML that re-parses (via the SAME
    /// `toml::from_str::<Config>` startup uses) to the typed `Some(Units::Imperial)`.
    #[test]
    fn build_config_toml_valid_units_roundtrips() {
        let text = build_config_toml(&Config::default(), "weather.units", "imperial").unwrap();
        let cfg: Config = toml::from_str(&text).unwrap();
        assert_eq!(cfg.weather.units, Some(Units::Imperial));
    }

    /// D-03 — a valid `hash.default_algo` set re-parses to `Some(Algo::Sha256)`.
    #[test]
    fn build_config_toml_valid_algo_roundtrips() {
        let text = build_config_toml(&Config::default(), "hash.default_algo", "sha256").unwrap();
        let cfg: Config = toml::from_str(&text).unwrap();
        assert_eq!(cfg.hash.default_algo, Some(Algo::Sha256));
    }

    /// D-03 — a valid `weather.location` set re-parses to the stored string.
    #[test]
    fn build_config_toml_valid_location_roundtrips() {
        let text = build_config_toml(&Config::default(), "weather.location", "London").unwrap();
        let cfg: Config = toml::from_str(&text).unwrap();
        assert_eq!(cfg.weather.location.as_deref(), Some("London"));
    }

    /// D-03 / T-11-02 — an invalid enum value (`weather.units = kelvin`) is caught by
    /// the re-parse round-trip: `build_config_toml` returns a `BoxError::Config`
    /// error and produces NO text to write, so a self-inflicted exit-2 lockout is
    /// structurally impossible.
    #[test]
    fn build_config_toml_invalid_enum_errs() {
        let err = build_config_toml(&Config::default(), "weather.units", "kelvin").unwrap_err();
        assert!(
            matches!(err.downcast_ref::<BoxError>(), Some(BoxError::Config { .. })),
            "invalid enum must map to BoxError::Config, got: {err:#}"
        );
    }

    /// D-04 — an unknown key is rejected BEFORE any reconstruction with
    /// `BoxError::ConfigUsage` whose message lists the settable keys; nothing is
    /// built (so `set_value` never reaches `atomic_write`).
    #[test]
    fn build_config_toml_unknown_key_errs_config_usage() {
        let err = build_config_toml(&Config::default(), "nope.key", "1").unwrap_err();
        assert!(
            matches!(err.downcast_ref::<BoxError>(), Some(BoxError::ConfigUsage { .. })),
            "unknown key must map to BoxError::ConfigUsage, got: {err:#}"
        );
        let msg = format!("{err}");
        for key in SETTABLE_KEYS {
            assert!(
                msg.contains(key),
                "the unknown-key message must list the settable key {key}, got: {msg}"
            );
        }
    }

    /// D-04 — every key in [`SETTABLE_KEYS`] is a real `table.field` pair (a sanity
    /// guard so the registry can never drift out of `table.field` shape).
    #[test]
    fn settable_keys_are_dotted_pairs() {
        assert_eq!(
            SETTABLE_KEYS,
            ["hash.default_algo", "weather.location", "weather.units"]
        );
        for key in SETTABLE_KEYS {
            assert!(key.split_once('.').is_some(), "{key} must be table.field");
        }
    }
}
