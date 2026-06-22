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

/// HASH-01 — `box hash <file>` defaults to SHA-256 and prints
/// `<sha256>  <filename>` (two spaces) and exits 0.
#[test]
fn hash_default_sha256() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    hash_cmd(&[f.path().to_str().unwrap()])
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

/// HASH-01 / D-05 — piping bytes with no path (default sha256) hashes the piped
/// bytes and labels the row `-`.
#[test]
fn hash_stdin_dash_label() {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("hash").env("NO_COLOR", "1").write_stdin("box");
    cmd.assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(format!("{BOX_SHA256}  -")));
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

/// HASH-01 / D-04 — a 40-hex `--verify` (sha1 length, unsupported) exits 2 via
/// the typed `UnsupportedHashLength` variant.
#[test]
fn hash_verify_bad_len_exit2() {
    let dir = assert_fs::TempDir::new().unwrap();
    let f = box_file(&dir);
    let path = f.path().to_str().unwrap();
    // 40 hex chars (a sha1 digest length) — no supported algorithm → exit 2.
    let sha1_len = "a".repeat(40);
    hash_cmd(&["--verify", &sha1_len, path])
        .failure()
        .code(2);
}
