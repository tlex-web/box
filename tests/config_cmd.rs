//! Integration proofs for the `box config` meta-command (CFG-01), Phase 11 Plan 01.
//!
//! Black-box against the compiled binary via `assert_cmd`. Locks the CFG-01
//! behavior contract: `show`/`get`/`set`/`path`, the 0/1/2 exit codes, the SC2
//! write→resolver→consume round-trip, the D-06 human/JSON parity, and JSON purity.
//!
//! **APPDATA isolation (load-bearing):** `core::config::config_path` resolves
//! `%APPDATA%\box\config.toml` (`%APPDATA%` env var first). Every test points
//! `APPDATA` at its own `assert_fs::TempDir` + `NO_COLOR=1`, so a test NEVER touches
//! (or depends on) the developer's real config — the same discipline as
//! `tests/config.rs`.

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Known-answer SHA-256 digest of `b"box"` (mirrors `tests/config.rs`), used by the
/// SC2 `set hash.default_algo` → `box hash` resolver round-trip.
const BOX_SHA256: &str = "26f8567f2569182294c3fa5b9f9cb2270b554eef628b4c149cf82a42888ff4ae";

/// Build a `box <args...>` command with an isolated, empty config dir (`APPDATA` →
/// `appdata`, which has no `box/` subdir unless a test writes one) and plain output.
/// Takes the FULL arg list so both `config …` and `hash …` (the round-trip) share it.
fn box_cmd(appdata: &assert_fs::TempDir, args: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.env("APPDATA", appdata.path());
    cmd
}

/// The `<APPDATA>/box/config.toml` path `config_path()` resolves under the test's
/// isolated APPDATA — for on-disk existence/content assertions.
fn config_file(appdata: &assert_fs::TempDir) -> std::path::PathBuf {
    appdata.child("box").child("config.toml").path().to_path_buf()
}

/// SC1 / D-06 — `box config show --json` exits 0 and emits EXACTLY one JSON object:
/// `.hash.default_algo == "blake3"`, `.weather.units == "metric"`,
/// `.weather.location == null` on an empty config. NO ANSI, NO UTF-8 BOM (the uuid
/// `json_purity` assertions). This is the machine-readable effective config.
#[test]
fn show_json_is_one_clean_object() {
    let appdata = assert_fs::TempDir::new().unwrap();
    let out = box_cmd(&appdata, &["config", "show", "--json"])
        .output()
        .expect("run box config show --json");

    assert_eq!(out.status.code(), Some(0), "show --json must exit 0");

    // stdout parses as EXACTLY one JSON value (from_slice over the whole buffer
    // succeeds only when there is one document + optional trailing whitespace).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert!(v.is_object(), "the effective config must be a JSON object");
    assert_eq!(
        v.pointer("/hash/default_algo").and_then(|x| x.as_str()),
        Some("blake3"),
        "empty config → resolved hash.default_algo == blake3"
    );
    assert_eq!(
        v.pointer("/weather/units").and_then(|x| x.as_str()),
        Some("metric"),
        "empty config → resolved weather.units == metric"
    );
    assert_eq!(
        v.pointer("/weather/location"),
        Some(&serde_json::Value::Null),
        "weather.location keeps its key and is null when unset"
    );

    // PURITY — no ANSI escape (0x1B) anywhere; no UTF-8 BOM (EF BB BF) at the front.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

/// D-06 human/JSON parity — on the SAME empty-config fixture, plain-text
/// `box config show` (no `--json`) exits 0 and its stdout carries the IDENTICAL
/// resolved values `show_json_is_one_clean_object` asserts for the `--json` branch
/// (`hash.default_algo = blake3` AND `weather.units = metric`). Proves `show` and
/// `show --json` render the same picture, so `config show` can never lie about what
/// `box hash`/`box weather` will consume.
#[test]
fn show_human_matches_json_parity() {
    let appdata = assert_fs::TempDir::new().unwrap();
    let out = box_cmd(&appdata, &["config", "show"])
        .output()
        .expect("run box config show");

    assert_eq!(out.status.code(), Some(0), "show must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("hash.default_algo = blake3"),
        "human show must carry the resolved hash.default_algo = blake3, got: {stdout}"
    );
    assert!(
        stdout.contains("weather.units = metric"),
        "human show must carry the resolved weather.units = metric, got: {stdout}"
    );
}

/// D-07 case 1 — `box config get hash.default_algo` on an empty config prints the
/// resolved builtin default `blake3` on stdout, exit 0.
#[test]
fn get_hash_default_on_empty_is_blake3() {
    let appdata = assert_fs::TempDir::new().unwrap();
    box_cmd(&appdata, &["config", "get", "hash.default_algo"])
        .assert()
        .success()
        .code(0)
        .stdout("blake3\n");
}

/// D-07 case 2 — `box config get weather.location` unset → EMPTY stdout, exit 1,
/// EMPTY stderr (the git-style silent "not set" signal — NOT a usage error, so no
/// `error:` line).
#[test]
fn get_unset_location_is_empty_exit1() {
    let appdata = assert_fs::TempDir::new().unwrap();
    let out = box_cmd(&appdata, &["config", "get", "weather.location"])
        .output()
        .expect("run box config get weather.location");

    assert_eq!(
        out.status.code(),
        Some(1),
        "an unset no-default key must exit 1 (not set), got {:?}",
        out.status.code()
    );
    assert!(
        out.stdout.is_empty(),
        "the not-set signal must print nothing on stdout, got: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        out.stderr.is_empty(),
        "the not-set signal must be silent on stderr, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// D-07 case 3 — `box config get nope.key` → exit 2 (usage error), stderr names the
/// three known keys.
#[test]
fn get_unknown_key_exit2() {
    let appdata = assert_fs::TempDir::new().unwrap();
    let out = box_cmd(&appdata, &["config", "get", "nope.key"])
        .output()
        .expect("run box config get nope.key");

    assert_eq!(out.status.code(), Some(2), "an unknown key must exit 2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    for key in ["hash.default_algo", "weather.location", "weather.units"] {
        assert!(
            stderr.contains(key),
            "the unknown-key error must list {key}, got: {stderr}"
        );
    }
}

/// SC2 (file level) — `box config set weather.units imperial` → exit 0, keeps stdout
/// empty; the on-disk `config.toml` re-parses (contains `units = "imperial"`); a
/// second `box config get weather.units` reads back `imperial`.
#[test]
fn set_units_writes_valid_toml() {
    let appdata = assert_fs::TempDir::new().unwrap();
    box_cmd(&appdata, &["config", "set", "weather.units", "imperial"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::is_empty());

    let contents = std::fs::read_to_string(config_file(&appdata))
        .expect("config.toml must exist after a successful set");
    assert!(
        contents.contains("units = \"imperial\""),
        "the written TOML must carry units = \"imperial\", got: {contents}"
    );
    // Re-parses cleanly (proven by a subsequent get reading it back).
    box_cmd(&appdata, &["config", "get", "weather.units"])
        .assert()
        .success()
        .code(0)
        .stdout("imperial\n");
}

/// D-03 / T-11-02 — `box config set weather.units kelvin` → exit 2 and NO config.toml
/// is created (the invalid enum is caught by the validate-before-write round-trip,
/// so nothing is written — a self-inflicted exit-2 lockout is impossible).
#[test]
fn set_bad_enum_exit2_nothing_written() {
    let appdata = assert_fs::TempDir::new().unwrap();
    box_cmd(&appdata, &["config", "set", "weather.units", "kelvin"])
        .assert()
        .failure()
        .code(2);
    assert!(
        !config_file(&appdata).exists(),
        "a bad-value set must write NOTHING (no config.toml created)"
    );
}

/// D-04 — `box config set nope.key 1` → exit 2, stderr lists the known keys, and NO
/// config.toml is created (unknown keys rejected before any write).
#[test]
fn set_unknown_key_exit2_nothing_written() {
    let appdata = assert_fs::TempDir::new().unwrap();
    let out = box_cmd(&appdata, &["config", "set", "nope.key", "1"])
        .output()
        .expect("run box config set nope.key 1");

    assert_eq!(out.status.code(), Some(2), "an unknown-key set must exit 2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("hash.default_algo"),
        "the unknown-key error must list the settable keys, got: {stderr}"
    );
    assert!(
        !config_file(&appdata).exists(),
        "an unknown-key set must write NOTHING (no config.toml created)"
    );
}

/// SC2 (resolver level, offline) — in a fresh APPDATA,
/// `box config set hash.default_algo sha256` → exit 0, then a bare `box hash <file>`
/// (a `b"box"` fixture in a SEPARATE temp dir) emits the `BOX_SHA256␣␣<path>` row —
/// proving `config set` feeds the SAME CLI>env>config>builtin resolver `box hash`
/// consumes. Fully offline/deterministic (unlike a networked `box weather`).
#[test]
fn set_then_hash_roundtrips_sha256() {
    let appdata = assert_fs::TempDir::new().unwrap();
    box_cmd(&appdata, &["config", "set", "hash.default_algo", "sha256"])
        .assert()
        .success()
        .code(0);

    // A `b"box"` fixture in its OWN temp dir (kept separate from the APPDATA dir).
    let work = assert_fs::TempDir::new().unwrap();
    let f = work.child("box.bin");
    f.write_binary(b"box").unwrap();
    let path = f.path().to_str().unwrap();
    let row_sha256 = format!("{BOX_SHA256}  {path}");

    box_cmd(&appdata, &["hash", path])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(row_sha256));
}

/// D-08 — `box config path` prints the resolved `%APPDATA%\box\config.toml` to
/// stdout, exit 0, whether or not the file exists (here it does NOT). Locate, never
/// read: the command must not create the file.
#[test]
fn path_prints_config_path() {
    let appdata = assert_fs::TempDir::new().unwrap();
    let out = box_cmd(&appdata, &["config", "path"])
        .output()
        .expect("run box config path");

    assert_eq!(out.status.code(), Some(0), "config path must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let trimmed = stdout.trim_end();
    assert!(
        trimmed.ends_with("config.toml"),
        "config path must end with config.toml, got: {stdout}"
    );
    assert!(
        trimmed.contains("box"),
        "config path must include the box/ dir, got: {stdout}"
    );
    // Locate, never read — the file must NOT have been created.
    assert!(
        !config_file(&appdata).exists(),
        "config path must not create the config file"
    );
}
