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
use predicates::prelude::*;

/// Known-answer digests of `b"box"` (mirrors `tests/hash.rs`), used by the
/// `hash_default_override` precedence round-trip.
const BOX_SHA256: &str = "26f8567f2569182294c3fa5b9f9cb2270b554eef628b4c149cf82a42888ff4ae";
const BOX_BLAKE3: &str = "095dfefdedb7f0870e801730da35823caaa8e969078e53b6e262c66f1a5b1c1e";

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

/// SPINE-05 / HASH-V2-01 / ROADMAP success #4 — the FULL config-precedence
/// round-trip now that `hash` adopts the config tier (06-02):
///   - config `default_hash_algo = "sha256"` → `box hash <file>` restores SHA-256
///     (the escape hatch for the BLAKE3-default breaking change: config beats the
///     built-in BLAKE3);
///   - `box hash --algo blake3 <file>` under the SAME config still emits BLAKE3
///     (CLI > config — the precedence by construction, T-06-05).
///
/// Runnable via `cargo test --test config hash_default_override`.
#[test]
fn hash_default_override() {
    let appdata = assert_fs::TempDir::new().unwrap();
    // D-13: the escape hatch now lives under the nested `[hash]` table
    // (`default_hash_algo` flat key → `[hash] default_algo`).
    write_config(&appdata, "[hash]\ndefault_algo = \"sha256\"\n");

    // A `b"box"` fixture in its own temp dir (kept separate from the APPDATA dir).
    let work = assert_fs::TempDir::new().unwrap();
    let f = work.child("box.bin");
    f.write_binary(b"box").unwrap();
    let path = f.path().to_str().unwrap();
    let row_sha256 = format!("{BOX_SHA256}  {path}");
    let row_blake3 = format!("{BOX_BLAKE3}  {path}");

    // config restores SHA-256 (config beats the built-in BLAKE3 default).
    box_cmd(&appdata, "hash", &[path])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(row_sha256));

    // CLI --algo blake3 STILL wins over the config sha256 (CLI > config).
    box_cmd(&appdata, "hash", &["--algo", "blake3", path])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(row_blake3));
}

/// D-13 / SPINE-05 tolerance — a VALID nested `[weather]` table (the schema
/// Phase 11's `box config get/set weather.location` locks against) parses cleanly:
/// a normal `box uuid` still exits 0 with NO stderr, exactly like the
/// missing-config case. Proves the migration widened the schema without
/// regressing the silent-default tolerance.
/// Runnable via `cargo test --test config valid_weather_config_is_silent`.
#[test]
fn valid_weather_config_is_silent() {
    let appdata = assert_fs::TempDir::new().unwrap();
    write_config(&appdata, "[weather]\nlocation = \"London\"\nunits = \"imperial\"\n");

    let out = box_cmd(&appdata, "uuid", &[])
        .output()
        .expect("run box uuid");

    assert_eq!(
        out.status.code(),
        Some(0),
        "a valid nested config must not error `box uuid`, got code {:?} / stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "box uuid must still print a UUID on stdout"
    );
    assert!(
        out.stderr.is_empty(),
        "a valid config must produce no stderr, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// D-13 — an INVALID nested enum value (`[weather] units = "kelvin"`, which is
/// neither `metric` nor `imperial`) is a malformed config: it aborts BEFORE the
/// op with exit 2 and an `error:` line on stderr, exactly like an unknown key.
/// The typed `Option<Units>` field is what makes a bad value loud instead of
/// silently ignored.
/// Runnable via `cargo test --test config unknown_units_exit2`.
#[test]
fn unknown_units_exit2() {
    let appdata = assert_fs::TempDir::new().unwrap();
    write_config(&appdata, "[weather]\nunits = \"kelvin\"\n");

    let out = box_cmd(&appdata, "uuid", &[])
        .output()
        .expect("run box uuid");

    assert_eq!(
        out.status.code(),
        Some(2),
        "an invalid units value must exit 2, got {:?}",
        out.status.code()
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("error:"),
        "an invalid units value must print an `error:` line on stderr, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stdout.is_empty(),
        "an invalid config must abort before the op (empty stdout), got: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
