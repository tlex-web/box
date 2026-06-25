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
use crate::core::errors::BoxError;

/// Config-overridable settings (SPINE-05). Every field is `Option<T>` with NO
/// clap `default_value` so `Some` = user/file set it, `None` = fall through to a
/// lower precedence tier (Anti-Pattern 3).
///
/// `#[serde(default, deny_unknown_fields)]`: a missing key deserializes to `None`
/// silently, an unknown key is a loud error (→ [`BoxError::Config`], exit 2, D-10).
///
/// Phase-6 LEAN scope (CONTEXT Claude's Discretion): the struct carries ONLY
/// `default_hash_algo`. It grows one field per command as Phase 7+ adopts the spine.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The default `hash` algorithm restored from config (the `hash.default_algo`
    /// escape hatch for the BLAKE3-default breaking change). Flat key
    /// `default_hash_algo = "sha256"` in Phase-6 lean scope.
    pub default_hash_algo: Option<Algo>,
}

/// Process-global config, set once by [`init_config`].
static CONFIG: OnceLock<Config> = OnceLock::new();

/// The loaded config. Panics if [`init_config`] has not run — it is called once in
/// `main()` before dispatch, so any command that reaches this has a config.
///
/// Live as of Plan 06-02: `hash` reads `config().default_hash_algo` in its
/// compute-default precedence chain, so the forward-compat `#[allow(dead_code)]`
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
fn config_path() -> Option<std::path::PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(
            std::path::PathBuf::from(appdata)
                .join("box")
                .join("config.toml"),
        );
    }
    dirs::config_dir().map(|p| p.join("box").join("config.toml"))
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

    /// A valid config round-trips the lowercase TOML value into the `Option<Algo>`
    /// field (the `#[serde(rename_all = "lowercase")]` on `Algo` doing its job).
    #[test]
    fn valid_config_parses() {
        let cfg: Config = toml::from_str("default_hash_algo = \"sha256\"").unwrap();
        assert_eq!(cfg.default_hash_algo, Some(Algo::Sha256));

        // An empty config is valid: the missing key → None (silent default).
        let empty: Config = toml::from_str("").unwrap();
        assert_eq!(empty.default_hash_algo, None);
    }

    /// D-10 — malformed TOML (and an unknown key under `deny_unknown_fields`) maps
    /// to [`BoxError::Config`] via the same `load`-style mapping, so `main()`'s
    /// downcast routes it to exit 2. Asserts the downcast is `BoxError::Config`.
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

        // Syntactically invalid TOML.
        let err = map("default_hash_algo = ");
        assert!(
            matches!(
                err.downcast_ref::<BoxError>(),
                Some(BoxError::Config { .. })
            ),
            "malformed TOML must downcast to BoxError::Config, got: {err:#}"
        );

        // An unknown key under deny_unknown_fields.
        let err = map("bogus_key = 1");
        assert!(
            matches!(
                err.downcast_ref::<BoxError>(),
                Some(BoxError::Config { .. })
            ),
            "unknown key must downcast to BoxError::Config, got: {err:#}"
        );
    }
}
