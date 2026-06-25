//! Integration tests for the config-file spine (SPINE-05 / D-10), Phase 6 Plan 01.
//!
//! Black-box against the compiled binary via `assert_cmd`. The reusable
//! Phase-7 template: a MISSING config never errors a normal command (silent
//! built-in defaults), a MALFORMED / unknown-key config aborts BEFORE dispatch
//! with exit 2 and an `error:` line on stderr, and a VALID config parses cleanly.
//!
//! **APPDATA isolation (load-bearing):** `core::config::config_path` resolves
//! `%APPDATA%\box\config.toml` via `dirs::config_dir()`, which reads the `APPDATA`
//! env var on Windows. Every test sets `APPDATA` to its own `TempDir`, so a test
//! NEVER touches (or depends on) the developer's real `%APPDATA%\box\config.toml`.
//! This is the same env-override discipline `tests/hash.rs` uses for `NO_COLOR`.

use assert_cmd::Command;
use assert_fs::prelude::*;

/// Build a `box <subcommand> <args...>` command with deterministic output and an
/// isolated, empty config dir (`APPDATA` → `appdata`, which has no `box/` subdir
/// unless a test writes one). Forces `NO_COLOR=1` so stdout/stderr are plain.
fn box_cmd(appdata: &assert_fs::TempDir, subcommand: &str, args: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg(subcommand);
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    // Isolate the config lookup: dirs::config_dir() reads %APPDATA% on Windows.
    cmd.env("APPDATA", appdata.path());
    cmd
}

/// Write `contents` to `<APPDATA>/box/config.toml`, creating the `box/` dir. This
/// is exactly the path `config_path()` resolves under the test's isolated APPDATA.
fn write_config(appdata: &assert_fs::TempDir, contents: &str) {
    let cfg = appdata.child("box").child("config.toml");
    cfg.write_str(contents).unwrap();
}

/// SPINE-05 / ROADMAP success #4 (first half) — a MISSING config never errors a
/// normal `box uuid`: it exits 0, prints a UUID, and writes NOTHING to stderr.
/// Runnable via `cargo test --test config missing_is_silent`.
#[test]
fn missing_is_silent() {
    // Fresh TempDir with NO box/config.toml present → silent built-in defaults.
    let appdata = assert_fs::TempDir::new().unwrap();
    let out = box_cmd(&appdata, "uuid", &[])
        .output()
        .expect("run box uuid");

    assert_eq!(
        out.status.code(),
        Some(0),
        "a missing config must not error `box uuid`, got code {:?} / stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    // It printed a UUID-shaped line on stdout.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "box uuid must print a UUID on stdout"
    );
    // SPINE-05: the missing-config path is silent — no stderr at all.
    assert!(
        out.stderr.is_empty(),
        "a missing config must produce no stderr, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// D-10 / ROADMAP success #4 — a MALFORMED (or unknown-key) config aborts BEFORE
/// the op with exit 2 and an `error:` line on stderr.
/// Runnable via `cargo test --test config malformed_exit2`.
#[test]
fn malformed_exit2() {
    let appdata = assert_fs::TempDir::new().unwrap();
    // An unknown key under deny_unknown_fields — malformed for our schema.
    write_config(&appdata, "bogus_key = 1\n");

    let out = box_cmd(&appdata, "uuid", &[])
        .output()
        .expect("run box uuid");

    assert_eq!(
        out.status.code(),
        Some(2),
        "a malformed config must exit 2, got {:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error:"),
        "a malformed config must print an `error:` line on stderr, got: {stderr}"
    );
    // Abort-before-dispatch: no UUID was printed to stdout.
    assert!(
        out.stdout.is_empty(),
        "a malformed config must abort before the op (empty stdout), got: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// SPINE-05 — a VALID config containing `default_hash_algo = "sha256"` (the
/// Phase-6 lean flat key) parses WITHOUT erroring `box uuid` (exit 0, no stderr).
/// The full "config restores SHA-256 default + CLI --algo blake3 still wins"
/// round-trip assertion lives in 06-02 (hash adopts the config tier there).
/// Runnable via `cargo test --test config hash_default_override`.
#[test]
fn hash_default_override() {
    let appdata = assert_fs::TempDir::new().unwrap();
    write_config(&appdata, "default_hash_algo = \"sha256\"\n");

    let out = box_cmd(&appdata, "uuid", &[])
        .output()
        .expect("run box uuid");

    assert_eq!(
        out.status.code(),
        Some(0),
        "a valid `default_hash_algo` config must parse without erroring, got {:?} / stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "a valid config must produce no stderr, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.stdout.trim_ascii().is_empty(),
        "box uuid must still print a UUID with a valid config present"
    );
}
