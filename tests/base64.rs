//! Integration tests for `box base64` (B64-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. Covers the four
//! locked behaviors: encode/decode round-trip (via the Rust-side stdin reader),
//! `--url-safe` alphabet, clean exit-1 on malformed input, and the arg path.
//!
//! Scope note (RESEARCH Pitfall 4, :536-538): `write_stdin` writes raw bytes
//! straight to the child's stdin pipe — it proves the *Rust* `read_input_bytes`
//! path is byte-exact, NOT that a real PowerShell 7 `… | box base64` pipeline is
//! byte-exact (PS7 re-encodes through UTF-16). That shell property is a
//! manual/human-verify item in 02-VALIDATION.md, not an automated gate here.
//! The interactive-TTY no-arg → exit-2 branch is unit-proven in `core::input`
//! (it cannot be faked in CI, where stdin is always a pipe — RESEARCH:540-543).

use assert_cmd::Command;
use predicates::prelude::*;

/// `box base64 <args>` with `NO_COLOR=1`; returns the assert handle.
fn base64_cmd() -> Command {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("base64");
    cmd.env("NO_COLOR", "1");
    cmd
}

/// Encode `input` (raw bytes via stdin), returning the trimmed base64 stdout.
fn encode_via_stdin(input: &[u8], url_safe: bool) -> String {
    let mut cmd = base64_cmd();
    if url_safe {
        cmd.arg("--url-safe");
    }
    let out = cmd
        .write_stdin(input.to_vec())
        .output()
        .expect("run encode");
    assert!(out.status.success(), "encode should exit 0");
    String::from_utf8(out.stdout)
        .expect("encoded output is UTF-8")
        .trim_end()
        .to_string()
}

/// Decode `b64` (passed as a positional arg), returning the raw stdout bytes.
fn decode_arg(b64: &str, url_safe: bool) -> Vec<u8> {
    let mut cmd = base64_cmd();
    cmd.arg("--decode");
    if url_safe {
        cmd.arg("--url-safe");
    }
    cmd.arg(b64);
    let out = cmd.output().expect("run decode");
    assert!(
        out.status.success(),
        "decode should exit 0 (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// `echo -n hello | box base64 | box base64 --decode` → `hello` (round-trip via
/// the real stdin reader, raw bytes, bypassing any shell — RESEARCH Pitfall 4).
#[test]
fn round_trip_standard_via_stdin() {
    let encoded = encode_via_stdin(b"hello", false);
    // Pipe the encoded text back through `--decode` via stdin.
    let out = base64_cmd()
        .arg("--decode")
        .write_stdin(encoded.into_bytes())
        .output()
        .expect("run decode");
    assert!(out.status.success(), "decode should exit 0");
    assert_eq!(out.stdout, b"hello", "round-trip must recover 'hello'");
}

/// Round-trip is byte-exact through the arg path for bytes including `0x00` and
/// `0xFF`, for BOTH the standard and url-safe alphabets.
#[test]
fn round_trip_binary_both_engines() {
    let payload: &[u8] = &[0x00, 0xFF, b'a', 0x80, 0x7F, 0x00];
    for url_safe in [false, true] {
        let encoded = encode_via_stdin(payload, url_safe);
        let decoded = decode_arg(&encoded, url_safe);
        assert_eq!(decoded, payload, "binary round-trip (url_safe={url_safe})");
    }
}

/// `box base64 "hello"` (arg path) → standard base64 of `hello`, single line,
/// no trailing line-wrap (B64-01).
#[test]
fn encode_arg_no_wrap() {
    base64_cmd()
        .arg("hello")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        // base64("hello") == "aGVsbG8=" ; trimmed, on one line.
        .stdout(predicate::str::contains("aGVsbG8="))
        .stdout(predicate::str::contains("\naGVsbG8").not());
}

/// `--url-safe` of bytes that would contain `+`/`/` under STANDARD uses `-`/`_`
/// and has NO `=` padding (URL_SAFE_NO_PAD).
#[test]
fn url_safe_alphabet_no_padding() {
    // 0xFB,0xFF,0xFE → STANDARD "+//+" family bytes; pick a payload that forces
    // `+` and `/` in STANDARD so the url-safe substitution is observable.
    let payload: &[u8] = &[0xFB, 0xFF, 0xBF];
    let std_enc = encode_via_stdin(payload, false);
    let url_enc = encode_via_stdin(payload, true);
    // Standard form contains at least one of `+`/`/`; url-safe must contain none.
    assert!(
        std_enc.contains('+') || std_enc.contains('/'),
        "test payload should force +// in STANDARD (got {std_enc})"
    );
    assert!(
        !url_enc.contains('+') && !url_enc.contains('/'),
        "url-safe must not contain +/ (got {url_enc})"
    );
    assert!(
        !url_enc.contains('='),
        "url-safe (NO_PAD) must have no '=' (got {url_enc})"
    );
}

/// Decoding non-alphabet input → exit 1, clean error on stderr, stdout empty
/// (no panic). Trust-boundary mitigation T-02-03 / V5.
#[test]
fn decode_garbage_exits_1_clean() {
    base64_cmd()
        .arg("--decode")
        .arg("!!!not-base64!!!")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}

/// `box base64 ""` (explicit empty arg) is the arg branch with empty input:
/// encoding empty bytes yields empty base64 → exit 0, empty stdout. This is the
/// deterministic CI-reachable cousin of the interactive-TTY no-arg case (which
/// is unit-proven in core::input, RESEARCH:540-543, since CI stdin is a pipe).
#[test]
fn empty_arg_encodes_to_empty() {
    base64_cmd()
        .arg("")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().or(predicate::str::is_match(r"^\s*$").unwrap()));
}
