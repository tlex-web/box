//! Config-file defaults for `box` (SPINE-05, D-1).
//!
//! `box` reads an optional TOML config from `%APPDATA%\box\config.toml`
//! (resolved via [`dirs::config_dir`]) **once** at startup ([`init_config`]) and
//! installs it as a process-global `OnceLock<Config>`. This mirrors
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
pub fn config() -> &'static Config {
    CONFIG.get().expect("init_config() must run before config()")
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

/// `%APPDATA%\box\config.toml` on Windows, via [`dirs::config_dir`] (the locked
/// resolver form). `std::env::var_os("APPDATA")` is the documented fallback only —
/// not used unless `dirs` ever resists.
fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("box").join("config.toml"))
}

/// The canonical config-precedence resolver (SPINE-05): **CLI > env > config >
/// builtin**, by construction. Pure and terminal-free so the precedence matrix is
/// deterministic. Every Phase-7 config-overridable flag resolves through this exact
/// `cli.or(env).or(cfg).unwrap_or(builtin)` shape — the builtin here is the
/// BLAKE3-default (D-04).
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
            toml::from_str::<Config>(toml_src)
                .map_err(|e| {
                    BoxError::Config {
                        path: "test/config.toml".to_string(),
                        message: e.to_string(),
                    }
                    .into()
                })
                .err()
                .expect("expected a parse error")
        };

        // Syntactically invalid TOML.
        let err = map("default_hash_algo = ");
        assert!(
            matches!(err.downcast_ref::<BoxError>(), Some(BoxError::Config { .. })),
            "malformed TOML must downcast to BoxError::Config, got: {err:#}"
        );

        // An unknown key under deny_unknown_fields.
        let err = map("bogus_key = 1");
        assert!(
            matches!(err.downcast_ref::<BoxError>(), Some(BoxError::Config { .. })),
            "unknown key must downcast to BoxError::Config, got: {err:#}"
        );
    }
}
