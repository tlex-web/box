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

// --- Scriptable spine (SPINE-02 / SPINE-04) — copied from tests/uuid.rs ---------
//
// base64 is a SCALAR command → a flat `{output, mode}` object (NOT {results,count}).
// `mode` is "encode"|"decode". The A1 surprise: under --json, decode emits the
// decoded bytes as a base64 string (binary-safe, lossless, round-trippable) — it
// NEVER `String::from_utf8(...).unwrap()`s arbitrary bytes (T-07a-01).

/// Capture `box base64 <args>` raw stdout bytes + exit status for the purity
/// assertions (which inspect raw bytes for ANSI/BOM, not a trimmed String).
fn base64_output(args: &[&str]) -> std::process::Output {
    let mut cmd = base64_cmd();
    cmd.args(args);
    cmd.output().expect("run box base64")
}

/// SPINE-02 / D-01 — `box base64 hello --json` emits EXACTLY one well-formed JSON
/// document `{"output":…,"mode":"encode"}` on stdout: flat scalar object, no human
/// chrome, no ANSI, no UTF-8 BOM. Copied from the frozen `tests/uuid.rs::json_purity`
/// template, adapted to base64's flat `{output, mode}` schema.
#[test]
fn json_purity() {
    let out = base64_output(&["hello", "--json"]);
    assert!(out.status.success(), "box base64 hello --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value (whole-buffer from_slice).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. The flat scalar schema: `mode` == "encode" and a non-empty `output`
    //    (base64("hello") == "aGVsbG8="). NOT wrapped in {results,count}.
    assert_eq!(
        v.get("mode"),
        Some(&serde_json::json!("encode")),
        "`.mode` must be \"encode\""
    );
    assert_eq!(
        v.get("output").and_then(|o| o.as_str()),
        Some("aGVsbG8="),
        "`.output` must be base64(\"hello\")"
    );
    assert!(
        v.get("results").is_none(),
        "base64 is scalar — no `results` wrapper"
    );

    // 3. PURITY — no ANSI escape (0x1B) anywhere (Pitfall 1).
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // 4. PURITY — no UTF-8 BOM (EF BB BF) at the front (Pitfall 2).
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

/// SPINE-02 / A1 / T-07a-01 — `box base64 --decode <non-utf8-b64> --json` does NOT
/// panic and emits valid JSON. The decoded bytes (0xFF 0xFE 0x80 — invalid UTF-8)
/// are carried as a base64 string in `.output` (binary-safe), and `.mode` is
/// "decode". The whole point: a JSON string can't hold raw non-UTF-8 bytes, so the
/// decode path re-encodes to base64 rather than `from_utf8(...).unwrap()`.
#[test]
fn json_decode_non_utf8() {
    // base64 of the non-UTF-8 byte sequence [0xFF, 0xFE, 0x80] (STANDARD): "//6A".
    let non_utf8_b64 = "//6A";
    let out = base64_output(&["--decode", non_utf8_b64, "--json"]);
    assert!(
        out.status.success(),
        "decode of non-UTF-8 bytes under --json must exit 0 (no panic); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // stdout parses as exactly one JSON value (no panic, no corruption).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(
        v.get("mode"),
        Some(&serde_json::json!("decode")),
        "`.mode` must be \"decode\""
    );
    // `.output` is the bytes re-encoded as base64 — for input "//6A" decoding to
    // [0xFF,0xFE,0x80], re-encoding yields the same canonical "//6A".
    assert_eq!(
        v.get("output").and_then(|o| o.as_str()),
        Some("//6A"),
        "`.output` carries the decoded bytes re-encoded as base64 (binary-safe)"
    );

    // PURITY — no ANSI, no BOM.
    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI in --json stdout");
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM"
    );
}

/// SPINE-04 / D-07 — live Windows-clipboard round-trip for `box base64 <text> --clip`
/// (the encode path; binary decode is not clip-supported). `#[ignore]`d (touches
/// shared OS clipboard). Run locally with:
///   cargo test --test base64 -- --ignored clip_roundtrip
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_roundtrip() {
    let printed = {
        let out = base64_output(&["hello", "--clip"]);
        assert!(out.status.success(), "box base64 hello --clip should exit 0");
        String::from_utf8(out.stdout)
            .expect("stdout is UTF-8")
            .trim()
            .to_string()
    };
    let pasted = {
        let out = Command::cargo_bin("box")
            .unwrap()
            .args(["clip", "--paste"])
            .output()
            .expect("run box clip --paste");
        assert!(out.status.success(), "box clip --paste should exit 0");
        String::from_utf8(out.stdout)
            .expect("clipboard text is UTF-8")
            .trim()
            .to_string()
    };
    assert_eq!(
        pasted, printed,
        "--clip must copy exactly the printed base64 encoding to the clipboard"
    );
}
