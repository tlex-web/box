//! Wave-0 integration tests for `box hash` (HASH-01).
//!
//! Black-box: each test runs the compiled `box hash` binary and asserts on the
//! `<hash>  <label>` output (two spaces, coreutils style — D-01) and the 0/1/2
//! exit contract of `--verify` (D-04). The hex values embedded below are TRUE
//! known-answer vectors for the fixed input `b"box"` (and the empty input),
//! computed independently of this binary (coreutils `sha256sum`/`sha512sum`/
//! `md5sum` and the blake3 reference), so each test is a known-answer test, not a
//! round-trip. The seven `#[test]`s map 1:1 to the VALIDATION HASH-01 rows.

use std::path::Path;

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

// --- Known-answer vectors for the byte string `box` (3 bytes, no newline) ------
const BOX_SHA256: &str = "26f8567f2569182294c3fa5b9f9cb2270b554eef628b4c149cf82a42888ff4ae";
const BOX_SHA512: &str = "04bbbafb37d4457e27963dbf55c92613ca2ab936ec506c57cd0d4f6504ae8b448191335ad7a9521b9bf2e2af9cee9361ecaab295df0e834ec07fa03b29a4d1ef";
const BOX_MD5: &str = "34be958a921e43d813a2075297d8e862";
const BOX_BLAKE3: &str = "095dfefdedb7f0870e801730da35823caaa8e969078e53b6e262c66f1a5b1c1e";

/// Run `box hash <args...>` with color forced off, returning the assert handle.
fn hash_cmd(args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("hash");
    for a in args {
        cmd.arg(a);
    }
    // Force plain output regardless of the test runner's TTY state (RESEARCH
    // determinism rule — ANSI must never leak into the asserted string).
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Write `bytes` to `<dir>/box.bin` and return the path; the fixture for the
/// known-answer vectors above.
fn box_file(dir: &assert_fs::TempDir) -> assert_fs::fixture::ChildPath {
    let f = dir.child("box.bin");
    f.write_binary(b"box").unwrap();
    f
}

/// The expected `<hash>  <label>` row (two spaces, coreutils style — D-01).
fn row(hash: &str, label: &Path) -> String {
    format!("{hash}  {}", label.display())
}

/// HASH-V2-01 / D-04 — `box hash <file>` now defaults to BLAKE3 (the v2 breaking
/// COMPUTE-default flip; v1 emitted SHA-256). Prints `<blake3>  <filename>` (two
/// spaces) and exits 0. An EXPLICIT `--algo sha256` still emits SHA-256 (the CLI
/// flag wins, the escape hatch). This is the ONE existing default assertion whose
/// expectation legitimately changes. Runnable via
/// `cargo test --test hash default_is_blake3`.
#[test]
fn default_is_blake3() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    let path = f.path().to_str().unwrap();

    // No --algo, no config → the new BLAKE3 default (D-04).
    hash_cmd(&[path])
        .success()
        .code(0)
        .stdout(predicate::str::contains(row(BOX_BLAKE3, f.path())));

    // Explicit --algo sha256 still emits SHA-256 (CLI flag wins — ROADMAP #3).
    hash_cmd(&["--algo", "sha256", path])
        .success()
        .code(0)
        .stdout(predicate::str::contains(row(BOX_SHA256, f.path())));
}

/// HASH-01 / D-02 — `--algo blake3` switches to the BLAKE3 known-answer vector.
#[test]
fn hash_algo_blake3() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    hash_cmd(&["--algo", "blake3", f.path().to_str().unwrap()])
        .success()
        .code(0)
        .stdout(predicate::str::contains(row(BOX_BLAKE3, f.path())));
}

/// HASH-01 / D-02 — `--algo sha512` and `--algo md5` each switch to their
/// matching known-answer vectors.
#[test]
fn hash_algo_sha512_md5() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    let path = f.path().to_str().unwrap();

    hash_cmd(&["--algo", "sha512", path])
        .success()
        .code(0)
        .stdout(predicate::str::contains(row(BOX_SHA512, f.path())));

    hash_cmd(&["--algo", "md5", path])
        .success()
        .code(0)
        .stdout(predicate::str::contains(row(BOX_MD5, f.path())));
}

/// HASH-V2-01 / D-04/D-05 — piping bytes with no path (now the BLAKE3 default)
/// hashes the piped bytes and labels the row `-`. (v1 emitted SHA-256 here; the
/// compute default flipped to BLAKE3 in v2.)
#[test]
fn hash_stdin_dash_label() {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("hash").env("NO_COLOR", "1").write_stdin("box");
    cmd.assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(format!("{BOX_BLAKE3}  -")));
}

/// HASH-01 / D-04 — `--verify <correct>` exits 0; `--verify <wrong>` exits 1
/// (a well-formed-but-mismatched hash is a plain error, never exit 2).
#[test]
fn hash_verify_match_mismatch() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    let path = f.path().to_str().unwrap();

    // Correct sha256 → exit 0.
    hash_cmd(&["--verify", BOX_SHA256, path]).success().code(0);

    // A wrong-but-valid-length sha256 → exit 1 (mismatch, NOT the exit-2 variant).
    let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
    hash_cmd(&["--verify", wrong, path]).failure().code(1);
}

/// HASH-01 / D-04 — `--verify` auto-detects the algorithm by hex length:
/// a 64-hex value with no `--algo` is sha256 (wins the 64-tie over blake3);
/// `--algo blake3` disambiguates the 64-tie to blake3; 32→md5; 128→sha512.
#[test]
fn hash_verify_autodetect() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    let path = f.path().to_str().unwrap();

    // 64-hex, no --algo → sha256 wins the tie → matches → exit 0.
    hash_cmd(&["--verify", BOX_SHA256, path]).success().code(0);
    // 64-hex blake3 with no --algo is treated as sha256, so it MISMATCHES → exit 1.
    hash_cmd(&["--verify", BOX_BLAKE3, path]).failure().code(1);
    // --algo blake3 disambiguates the 64-tie → the blake3 vector now matches → 0.
    hash_cmd(&["--algo", "blake3", "--verify", BOX_BLAKE3, path])
        .success()
        .code(0);
    // 32-hex → md5; 128-hex → sha512.
    hash_cmd(&["--verify", BOX_MD5, path]).success().code(0);
    hash_cmd(&["--verify", BOX_SHA512, path]).success().code(0);
}

/// HASH-01 / WR-01 — an EXPLICIT `--algo` is honored under `--verify` and is
/// NEVER overridden by length auto-detection. The sharpest probe is the 32-hex
/// MD5 of `box`: with the old "explicit-vs-default-by-value" bug, `--algo sha256
/// --verify <md5-of-box>` auto-detected md5 by the 32-length, computed md5, and
/// FALSELY MATCHED (exit 0). With the fix, the explicit sha256 is used: the
/// computed digest is 64-hex, can never equal the 32-hex value, and the command
/// MISMATCHES (exit 1) — it is NOT treated as md5, and NOT the exit-2
/// unsupported-length path either.
#[test]
fn hash_verify_explicit_algo_overrides_length_autodetect() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    let path = f.path().to_str().unwrap();

    // `--algo sha256` + the 32-hex MD5 of `box`: must NOT auto-detect md5. The
    // sha256 digest (64-hex) cannot equal a 32-hex value → mismatch → exit 1.
    hash_cmd(&["--algo", "sha256", "--verify", BOX_MD5, path])
        .failure()
        .code(1);

    // Symmetry check: `--algo md5` + the 64-hex SHA-256 of `box` must NOT
    // auto-detect sha256. md5 (32-hex) cannot equal a 64-hex value → exit 1.
    hash_cmd(&["--algo", "md5", "--verify", BOX_SHA256, path])
        .failure()
        .code(1);

    // And the explicit algo still MATCHES its own correct digest (exit 0): proves
    // the override honors the user's choice rather than just always failing.
    hash_cmd(&["--algo", "sha256", "--verify", BOX_SHA256, path])
        .success()
        .code(0);
    hash_cmd(&["--algo", "md5", "--verify", BOX_MD5, path])
        .success()
        .code(0);
}

/// HASH-01 / D-04 — a 40-hex `--verify` (sha1 length, unsupported) exits 2 via
/// the typed `UnsupportedHashLength` variant.
#[test]
fn hash_verify_bad_len_exit2() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    let path = f.path().to_str().unwrap();
    // 40 hex chars (a sha1 digest length) — no supported algorithm → exit 2.
    let sha1_len = "a".repeat(40);
    hash_cmd(&["--verify", &sha1_len, path]).failure().code(2);
}

// --- Scriptable spine (SPINE-01 / SPINE-03) + D-05 verify probe ----------------
//
// hash is the second --json/--clip spine consumer (06-02). These tests pin the
// D-02 pilot JSON literal `{results:[{path,algo,digest}],count:1}`, the D-09
// empty-stdout-on-error contract, and the D-05 BLAKE3-fallback verify-mismatch
// hint. `hash_verify_autodetect` above MUST keep passing UNCHANGED — it is the
// #1 v2 data-risk backstop (the 64→sha256 length table is intact).

/// SPINE-01 / D-02 — `box hash <file> --json` emits exactly ONE well-formed JSON
/// document `{"results":[{"path":…,"algo":"blake3","digest":<64hex>}],"count":1}`:
/// a one-element results array inside an object (NOT a bare array — D-02), `algo`
/// serializes lowercase, no ANSI, no UTF-8 BOM. `algo` is "blake3" because no
/// `--algo` + no config → the new default (D-04).
/// Runnable via `cargo test --test hash json_shape`.
#[test]
fn json_shape() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);

    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["hash", "--json", f.path().to_str().unwrap()])
        .env("NO_COLOR", "1")
        .output()
        .expect("run box hash --json");
    assert!(out.status.success(), "box hash --json should exit 0");

    // Exactly one JSON value.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // `{results:[{path,algo,digest}],count:1}` shape.
    assert_eq!(v.get("count"), Some(&serde_json::json!(1)), "`.count` == 1");
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 1, "one file → one result element");
    let elem = &results[0];
    // `algo` serializes lowercase via `#[serde(rename_all = "lowercase")]`; it is
    // "blake3" because no --algo + no config → the new default.
    assert_eq!(
        elem.get("algo"),
        Some(&serde_json::json!("blake3")),
        "`.results[0].algo` must be the lowercase \"blake3\" default"
    );
    let digest = elem
        .get("digest")
        .and_then(|d| d.as_str())
        .expect("`.results[0].digest` must be a string");
    assert_eq!(digest, BOX_BLAKE3, "the digest is the BLAKE3 of b\"box\"");
    // `path` is the file path (the row label), serde-escaped, never raw-printed.
    assert_eq!(
        elem.get("path").and_then(|p| p.as_str()),
        Some(f.path().to_str().unwrap()),
        "`.results[0].path` is the input path"
    );

    // PURITY — no ANSI escape, no UTF-8 BOM (Pitfall 1/2).
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

/// D-09 — under `--json`, a verify MISMATCH leaves stdout EMPTY, prints an
/// `error:` line on stderr, and exits 1. Uses a 64-hex value that is neither the
/// file's sha256 NOR its blake3, so it mismatches under both algorithms.
/// Runnable via `cargo test --test hash json_error_empty_stdout`.
#[test]
fn json_error_empty_stdout() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    // All-zeros 64-hex: a valid sha256 LENGTH (→ verify autodetects sha256), but
    // neither the sha256 nor the blake3 of b"box" → mismatch under both.
    let wrong = "0".repeat(64);

    let out = Command::cargo_bin("box")
        .unwrap()
        .args([
            "hash",
            "--verify",
            &wrong,
            "--json",
            f.path().to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .expect("run box hash --verify --json");

    assert_eq!(
        out.status.code(),
        Some(1),
        "a --json verify mismatch must exit 1, got {:?}",
        out.status.code()
    );
    // D-09: stdout stays EMPTY (no partial JSON, no chrome) under --json on error.
    assert!(
        out.stdout.is_empty(),
        "under --json a failure must leave stdout EMPTY, got: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error:"),
        "a --json failure must print an `error:` line on stderr, got: {stderr}"
    );
}

/// D-05 — `box hash --verify <BOX_BLAKE3> <file>` (no `--algo`, file is b"box"):
/// the 64-hex value autodetects as sha256, MISMATCHES the file's sha256 → exit 1,
/// AND because blake3 of the file DOES match the value, a DECISIVE stderr hint
/// mentioning `--algo blake3` is printed. The SAME invocation WITH `--json`
/// suppresses the probe hint (D-05/D-09) while still failing exit 1.
/// Runnable via `cargo test --test hash verify_blake3_probe_hint`.
#[test]
fn verify_blake3_probe_hint() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    let path = f.path().to_str().unwrap();

    // Human path: 64-hex blake3 value, no --algo → sha256 mismatch (exit 1) + the
    // decisive `--algo blake3` hint on stderr (blake3 matches the file).
    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["hash", "--verify", BOX_BLAKE3, path])
        .env("NO_COLOR", "1")
        .output()
        .expect("run box hash --verify <blake3>");
    assert_eq!(
        out.status.code(),
        Some(1),
        "a 64-hex blake3 value verified as sha256 must mismatch → exit 1, got {:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--algo blake3"),
        "the D-05 probe must hint `--algo blake3` on a 64-hex mismatch, got: {stderr}"
    );

    // --json path: the SAME mismatch suppresses the probe hint (D-05/D-09) — still
    // exit 1, but stderr carries the plain `error:` line, NOT the probe phrasing.
    let out_json = Command::cargo_bin("box")
        .unwrap()
        .args(["hash", "--verify", BOX_BLAKE3, "--json", path])
        .env("NO_COLOR", "1")
        .output()
        .expect("run box hash --verify <blake3> --json");
    assert_eq!(
        out_json.status.code(),
        Some(1),
        "the --json mismatch still exits 1, got {:?}",
        out_json.status.code()
    );
    let stderr_json = String::from_utf8_lossy(&out_json.stderr);
    assert!(
        !stderr_json.contains("--algo blake3"),
        "under --json the D-05 probe hint must be SUPPRESSED, got: {stderr_json}"
    );
}
