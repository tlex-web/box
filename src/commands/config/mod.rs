//! The `config` command: read, edit, and locate `%APPDATA%\box\config.toml`
//! (CFG-01). The CLI's FIRST nested subcommand (D-01) — `show | get <key> |
//! set <key> <value> | path`.
//!
//! Flow (Pattern 1 — thin orchestrator over the settled `core::config` subsystem):
//! `show`/`get` read the EFFECTIVE resolved config through the SAME
//! `config().<table>.<field>.unwrap_or(builtin)` resolution every command uses, so
//! `config show` can never lie about what `box hash`/`box weather` will consume
//! (D-06 human/JSON parity). `set` delegates to
//! [`crate::core::config::set_value`], which validates-before-writes (D-03) so a
//! self-inflicted exit-2 lockout is structurally impossible. `path` prints the
//! resolved location whether or not the file exists (D-08 — locate, never read).
//!
//! Exit contract (0/1/2):
//! - usage/config errors (unknown key, bad value) → [`BoxError::ConfigUsage`] /
//!   [`BoxError::Config`] → exit 2 (via `main`'s downcast).
//! - `get` on an unset key with NO builtin default (`weather.location`) → print
//!   NOTHING and `std::process::exit(1)` (the git-style "not set" data signal — it
//!   MUST be silent on stderr, so it is NOT a `BoxError` that would trip `main`'s
//!   always-on `error:` line). Under `--json` it emits `null` first.
//! - success → exit 0.
//!
//! `--json` (dogfoods the spine): `show --json` emits one nested effective doc via
//! [`emit_json`]; `get --json` emits the BARE resolved value (a JSON string, or
//! `null` for the unset-no-default case). The fork is on [`is_json_on`] FIRST
//! (Pitfall 1) so the ONLY stdout write reachable under `--json` is `emit_json`.

use std::io::IsTerminal;

use anyhow::Context;
use clap::{Args, Subcommand};

use crate::commands::hash::Algo;
use crate::commands::weather::Units;
use crate::commands::RunCommand;
use crate::core::config::{config, config_path, set_value, SETTABLE_KEYS};
use crate::core::errors::BoxError;
use crate::core::output::{emit_json, is_json_on, out_line};

/// `box config <show|get|set|path>` — manage the config-file defaults every command
/// reads through the SPINE-05 resolver (CFG-01). Carries the nested
/// [`ConfigCommand`] subcommand (D-01, the CLI's first nested subcommand).
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// The four `config` operations (D-01). `--json` is the GLOBAL flag on `Cli` (read
/// via [`is_json_on`]), NOT a per-subcommand flag.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the effective resolved config (built-in defaults filled in)
    Show,
    /// Print the effective value of a key (exit 1 if unset with no default)
    Get {
        /// The config key to read (e.g. `hash.default_algo`)
        key: String,
    },
    /// Set a config key, validating before writing (nothing written on error)
    Set {
        /// The config key to write (one of the settable keys)
        key: String,
        /// The new value
        value: String,
    },
    /// Print the path to the config file (whether or not it exists)
    Path,
}

impl ConfigArgs {
    /// True for the config-INDEPENDENT subcommands that must stay usable even when
    /// `%APPDATA%\box\config.toml` is malformed (WR-02): `path` (locate the broken
    /// file) and `set` (the intended repair — it re-round-trips through the D-03
    /// validate-before-write, replacing the bad file). `Show`/`Get` deliberately
    /// return `false`: they report the EFFECTIVE value, which requires a parseable
    /// file, so they must keep exiting 2 on a malformed config (no tolerance hole).
    pub fn tolerates_malformed_config(&self) -> bool {
        matches!(
            self.command,
            ConfigCommand::Path | ConfigCommand::Set { .. }
        )
    }
}

impl RunCommand for ConfigArgs {
    fn run(self) -> anyhow::Result<()> {
        match self.command {
            ConfigCommand::Show => show(),
            ConfigCommand::Get { key } => get(&key),
            ConfigCommand::Set { key, value } => set(&key, &value),
            ConfigCommand::Path => path(),
        }
    }
}

/// The EFFECTIVE resolved config doc (D-06) — resolved to exactly what `box hash`/
/// `box weather` will consume. snake_case per house style; `location` keeps its key
/// and serializes to `null` when unset (a stable doc shape), while `default_algo`/
/// `units` are always resolved to a concrete value. `default_algo` resolves through
/// the FULL env > config > builtin chain via
/// [`crate::commands::hash::effective_default_algo`] (WR-01), so it never drifts from
/// `run_compute`; `weather.units` has no env tier (config > builtin). `Algo`/`Units`
/// serialize to their lowercase spellings, so the `--json` doc and the human rows
/// (which read the SAME serde spelling via [`serde_str`]) can never drift.
#[derive(serde::Serialize)]
struct EffectiveConfig {
    hash: EffectiveHash,
    weather: EffectiveWeather,
}

#[derive(serde::Serialize)]
struct EffectiveHash {
    default_algo: Algo,
}

#[derive(serde::Serialize)]
struct EffectiveWeather {
    location: Option<String>,
    units: Units,
}

/// Resolve the effective config to exactly what each command consumes. The
/// `hash.default_algo` leaf routes through
/// [`crate::commands::hash::effective_default_algo`] — the SAME env > config >
/// builtin (`Algo::Blake3`) chain `run_compute` uses (the one place the hash env
/// tier is looked up) — so `config show`/`get` can never lie about what `box hash`
/// actually consumes (WR-01).
/// Weather has no env tier: `location`/`units` stay `config().weather.<field>`
/// (config > builtin `Units::Metric`; `weather.location` has no builtin).
fn effective() -> EffectiveConfig {
    let cfg = config();
    EffectiveConfig {
        hash: EffectiveHash {
            default_algo: crate::commands::hash::effective_default_algo(),
        },
        weather: EffectiveWeather {
            location: cfg.weather.location.clone(),
            units: cfg.weather.units.unwrap_or(Units::Metric),
        },
    }
}

/// The lowercase serde spelling of a value (`Algo`/`Units` → `"blake3"`/`"metric"`),
/// so the human `key = value` rows carry the IDENTICAL string the `--json` doc
/// reports (D-06 parity — human and JSON can never disagree about a resolved value).
fn serde_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// `config show` — the effective resolved config. `--json` FIRST (Pitfall 1): the
/// one nested doc via `emit_json`; otherwise three `key = value` human rows carrying
/// the SAME resolved values (D-06).
fn show() -> anyhow::Result<()> {
    let eff = effective();
    if is_json_on() {
        return emit_json(&eff);
    }
    out_line(&format!(
        "hash.default_algo = {}",
        serde_str(&eff.hash.default_algo)
    ));
    let location = eff.weather.location.as_deref().unwrap_or("(not set)");
    out_line(&format!("weather.location = {location}"));
    out_line(&format!("weather.units = {}", serde_str(&eff.weather.units)));
    Ok(())
}

/// `config get <key>` — the three D-07 cases:
/// 1. unset WITH a builtin default (`hash.default_algo`/`weather.units`) → the
///    resolved default, exit 0.
/// 2. `weather.location` unset (no builtin) → print NOTHING, exit 1 (silent on
///    stderr — the git-style "not set" data signal); under `--json` emit `null`
///    first.
/// 3. unknown key → [`BoxError::ConfigUsage`] → exit 2.
fn get(key: &str) -> anyhow::Result<()> {
    match key {
        "hash.default_algo" => {
            // WR-01: resolve through the SAME shared env > config > builtin resolver
            // `box hash` consumes, so `get` reports the env-tier value too.
            let v = crate::commands::hash::effective_default_algo();
            emit_value(&serde_str(&v))
        }
        "weather.units" => {
            let v = config().weather.units.unwrap_or(Units::Metric);
            emit_value(&serde_str(&v))
        }
        "weather.location" => match &config().weather.location {
            Some(loc) => emit_value(loc),
            None => {
                // D-07 case 2: unset, no builtin → the "not set" signal. Under
                // `--json` emit `null` (flushes the trailing `\n`), then exit 1
                // WITHOUT an Err — a `BoxError` would trip `main`'s always-on
                // `error:` line, breaking the "silent on stderr" contract.
                if is_json_on() {
                    emit_json(&serde_json::Value::Null)?;
                }
                std::process::exit(1);
            }
        },
        _ => Err(unknown_key(key)),
    }
}

/// `config set <key> <value>` — delegates to the validate-before-write
/// [`set_value`] (D-03: `ConfigUsage`/`Config` → exit 2, NOTHING written). On
/// success prints a concise TTY-gated stderr confirmation (so `2>log` stays clean)
/// and keeps stdout empty; exit 0.
fn set(key: &str, value: &str) -> anyhow::Result<()> {
    set_value(key, value)?;
    if std::io::stderr().is_terminal() {
        eprintln!("Set {key} = {value}");
    }
    Ok(())
}

/// `config path` — the resolved `config_path()` to stdout, exit 0, whether or not
/// the file exists (D-08 — locate, never read). A `None` path (no `%APPDATA%`, no
/// `dirs` fallback) is a runtime error → exit 1.
fn path() -> anyhow::Result<()> {
    let p = config_path().context("could not resolve the config file path")?;
    out_line(&p.display().to_string());
    Ok(())
}

/// Print one resolved value: `emit_json` the BARE value (a JSON string) under
/// `--json`, else `out_line` it plain. The single fork point for `get`'s scalar
/// output (Pitfall 1 — the only stdout write under `--json` is `emit_json`).
fn emit_value(s: &str) -> anyhow::Result<()> {
    if is_json_on() {
        emit_json(&s)
    } else {
        out_line(s);
        Ok(())
    }
}

/// The shared unknown-key usage error (D-04): names the key and lists the settable
/// keys — the same "did you mean" surface `set_value` uses, so `get` and `set`
/// report identical text for an unknown key. Routes to exit 2 via `main`'s downcast.
fn unknown_key(key: &str) -> anyhow::Error {
    BoxError::ConfigUsage {
        message: format!(
            "unknown config key '{key}'; settable keys: {}",
            SETTABLE_KEYS.join(", ")
        ),
    }
    .into()
}
