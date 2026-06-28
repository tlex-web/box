//! Integration tests for `box json` (JSON-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. These tests pin the
//! JSON-01 contract (D-04/D-05/D-06):
//!   - invalid JSON → exit 1 with a 1-based line/column error on stderr, empty
//!     stdout (mirrors `tests/color.rs::malformed_hex_exits_1_clean`, color.rs:67-74);
//!   - valid input pretty-prints with a 2-space indent (D-06);
//!   - `--compact` minifies to a single line (D-06);
//!   - input key order is preserved, NOT alphabetized (`preserve_order`, D-04);
//!   - piped output carries no ANSI, byte-identical minus color (D-05) — the
//!     run-WITHOUT-`NO_COLOR` pipe-path proof from `tests/tree.rs::tree_piped_no_ansi`.

use assert_cmd::Command;
use predicates::prelude::*;

/// Run `box json [--compact]` with `NO_COLOR=1`, feeding `input` via stdin,
/// returning the assert handle. Mirrors the `color()` runner (color.rs:13-18).
fn json(input: &str, compact: bool) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("json");
    if compact {
        cmd.arg("--compact");
    }
    cmd.env("NO_COLOR", "1").write_stdin(input);
    cmd.assert()
}

/// Capture `box json` stdout for valid `input` (exit 0, stderr empty), NO_COLOR.
/// Mirrors `color_stdout` (color.rs:21-32).
fn json_stdout(input: &str, compact: bool) -> String {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("json");
    if compact {
        cmd.arg("--compact");
    }
    cmd.env("NO_COLOR", "1").write_stdin(input);
    let out = cmd.output().expect("run box json");
    assert!(out.status.success(), "box json {input:?} should exit 0");
    assert!(
        out.stderr.is_empty(),
        "stderr should be empty, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// (1) D-06 — invalid JSON `{bad` → exit 1, empty stdout, and a 1-based
/// line/column error on stderr. Reuses the color.rs:67-74 malformed-input shape
/// and adds the `line`+`column` predicate (the JSON-01 parse-error contract).
#[test]
fn invalid_json_exits_1_with_line_and_column() {
    json("{bad", false)
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("line").and(predicate::str::contains("column")));
}

/// (2) D-06 — a valid object pretty-prints with a 2-space indent: `{"a":1}`
/// yields a line `  "a": 1` (two leading spaces before the key).
#[test]
fn valid_pretty_uses_two_space_indent() {
    let out = json_stdout("{\"a\":1}", false);
    assert!(
        out.contains("  \"a\": 1"),
        "expected a 2-space-indented `  \"a\": 1` line, got: {out:?}"
    );
    // No ANSI under NO_COLOR — byte-identical minus color (D-05).
    assert!(!out.contains('\u{1b}'), "no ANSI under NO_COLOR: {out:?}");
}

/// (3) D-06 — `--compact` minifies to the single-line `{"a":1}` with no embedded
/// newline before EOF (a trailing `\n` from `println!` is allowed, but no inner
/// break).
#[test]
fn compact_minifies_to_single_line() {
    let out = json_stdout("{ \"a\" : 1 }", true);
    assert_eq!(
        out.trim_end_matches('\n'),
        "{\"a\":1}",
        "compact output should be the minified single line, got: {out:?}"
    );
    // The only newline allowed is the trailing one — no embedded breaks.
    assert!(
        !out.trim_end_matches('\n').contains('\n'),
        "compact output must not contain an embedded newline: {out:?}"
    );
}

/// (4) D-04 — input key order is PRESERVED, not alphabetized: `{"b":1,"a":2}`
/// keeps `b` before `a` in the output (the byte offset of `"b"` is less than
/// the offset of `"a"`). With `preserve_order` ON this holds; without it serde
/// would sort the keys and `a` would come first.
#[test]
fn preserves_input_key_order() {
    let out = json_stdout("{\"b\":1,\"a\":2}", false);
    let b_at = out.find("\"b\"").expect("`b` key present in output");
    let a_at = out.find("\"a\"").expect("`a` key present in output");
    assert!(
        b_at < a_at,
        "input key order must be preserved (b before a), got: {out:?}"
    );
}

/// (5) D-05 — run WITHOUT `NO_COLOR` but with piped stdout (assert_cmd captures
/// stdout via a pipe, so it is not a TTY): the output must contain no ANSI escape
/// (`\x1b[`). Mirrors `tree_piped_no_ansi` (tree.rs:98-116) — proves the pipe
/// path suppresses color so piped output is byte-identical minus ANSI.
#[test]
fn json_piped_no_ansi() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("json")
        .write_stdin("{\"a\":[1,true,null]}")
        .output()
        .expect("run box json");
    assert!(out.status.success(), "box json should exit 0");
    // ESC `[` is the start of every CSI/SGR ANSI sequence.
    assert!(
        !out.stdout.windows(2).any(|w| w == b"\x1b["),
        "piped json output must contain no ANSI escape (\\x1b[)"
    );
}

// --- JSON-V2-01: opt-in --sort-keys --------------------------------------------

/// `box json --sort-keys` recursively sorts object keys: `{"b":1,"a":2}` →
/// `a` before `b` in the output (the opposite of the preserve-order default).
#[test]
fn sort_keys_sorts_object_keys() {
    let out = {
        let mut cmd = Command::cargo_bin("box").unwrap();
        cmd.arg("json")
            .arg("--sort-keys")
            .env("NO_COLOR", "1")
            .write_stdin("{\"b\":1,\"a\":2}");
        cmd.output().expect("run box json --sort-keys")
    };
    assert!(out.status.success(), "box json --sort-keys should exit 0");
    let s = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let a_at = s.find("\"a\"").expect("`a` present");
    let b_at = s.find("\"b\"").expect("`b` present");
    assert!(a_at < b_at, "--sort-keys must order a before b: {s:?}");
}

/// `box json` (no `--sort-keys`) PRESERVES input order — the default is unchanged.
/// (Mirrors `preserves_input_key_order`, paired here as the --sort-keys
/// counter-control so the contrast is explicit.)
#[test]
fn plain_json_still_preserves_order() {
    let out = json_stdout("{\"b\":1,\"a\":2}", false);
    let b_at = out.find("\"b\"").expect("`b` present");
    let a_at = out.find("\"a\"").expect("`a` present");
    assert!(
        b_at < a_at,
        "without --sort-keys input order is preserved (b before a): {out:?}"
    );
}

/// `box json --sort-keys --json` emits a SORTED machine document (the sort is
/// applied before the `is_json_on()` fork, so it feeds emit_json too).
#[test]
fn sort_keys_json_mode_sorts() {
    let out = {
        let mut cmd = Command::cargo_bin("box").unwrap();
        cmd.arg("json")
            .args(["--sort-keys", "--json"])
            .env("NO_COLOR", "1")
            .write_stdin("{\"b\":1,\"a\":2}");
        cmd.output().expect("run box json --sort-keys --json")
    };
    assert!(out.status.success(), "box json --sort-keys --json should exit 0");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert!(v.as_object().is_some(), "still an object");
    let s = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let a_at = s.find("\"a\"").expect("`a` present");
    let b_at = s.find("\"b\"").expect("`b` present");
    assert!(a_at < b_at, "--sort-keys --json must emit sorted: {s:?}");
}

// --- Scriptable spine (SPINE-02 / SPINE-04) — D-16 identity passthrough --------
//
// json is a Wave-7c odd-fit and the ONE sanctioned direct-serde command: under
// --json it emits the parsed `Value` VERBATIM (D-16 identity passthrough), NOT
// wrapped in {results,count} — a documented root-rule exception alongside tree.
//   - `json_identity_passthrough` — --json emits the document verbatim (top-level
//                                   object with the input keys, no wrapper).
//   - `json_purity`               — one JSON value, no 0x1B, no BOM.
//   - `clip_roundtrip`            — #[ignore]d: --clip copies the pretty form;
//                                   --compact --clip copies the compact form.

/// Capture `box json --json` raw stdout bytes + exit status, feeding `input` via
/// stdin, for the JSON-purity assertions (raw bytes for ANSI/BOM). NO_COLOR=1.
fn json_passthrough_output(input: &str, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("json");
    cmd.args(extra);
    cmd.env("NO_COLOR", "1").write_stdin(input);
    cmd.output().expect("run box json")
}

/// D-16 — `box json '{"b":1,"a":2}' --json` emits the parsed document VERBATIM:
/// a top-level object whose members are `a` and `b` (NOT a `{results,count}`
/// wrapper), with input key order preserved (`b` before `a`). This is the
/// identity-passthrough root-rule exception. Runnable via
/// `cargo test --test json json_identity_passthrough`.
#[test]
fn json_identity_passthrough() {
    let out = json_passthrough_output("{\"b\":1,\"a\":2}", &["--json"]);
    assert!(out.status.success(), "box json --json should exit 0");

    // stdout is exactly one JSON value — the parsed document, emitted verbatim.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // Identity passthrough: the top level is the INPUT object (keys a, b), NOT a
    // {results,count} wrapper.
    let obj = v.as_object().expect("the document is a top-level object");
    assert!(obj.contains_key("a"), "passthrough keeps the input key `a`");
    assert!(obj.contains_key("b"), "passthrough keeps the input key `b`");
    assert!(
        !obj.contains_key("results"),
        "identity passthrough must NOT wrap in `results` (D-16)"
    );
    assert!(
        !obj.contains_key("count"),
        "identity passthrough must NOT wrap in `count` (D-16)"
    );
    assert_eq!(obj.len(), 2, "exactly the two input members, nothing added");

    // preserve_order keeps the input key order (`b` before `a`) in the raw bytes.
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let b_at = stdout.find("\"b\"").expect("`b` present");
    let a_at = stdout.find("\"a\"").expect("`a` present");
    assert!(b_at < a_at, "input key order preserved (b before a): {stdout:?}");

    // D-16 — the `--json` fork wins over `--compact`: `emit_json` is ALWAYS the
    // pretty `to_writer_pretty` document (not the minified compact form). This is
    // the decisive proof the command routes through `emit_json` under `--json`
    // (the pre-spine human path would have honored `--compact` and minified).
    let pretty = json_passthrough_output("{\"b\":1,\"a\":2}", &["--json", "--compact"]);
    assert!(pretty.status.success(), "box json --json --compact should exit 0");
    let pretty_str = String::from_utf8(pretty.stdout).expect("stdout is UTF-8");
    assert!(
        pretty_str.contains('\n') && pretty_str.contains("  \"b\""),
        "--json forces the pretty emit_json document even with --compact (D-16): {pretty_str:?}"
    );
}

/// SPINE-02 — `box json <doc> --json` emits EXACTLY one well-formed JSON document
/// (the passthrough): no UTF-8 BOM, no ANSI escape, parses as a single value.
/// Runnable via `cargo test --test json json_purity`.
#[test]
fn json_purity() {
    let out = json_passthrough_output("{\"a\":[1,true,null]}", &["--json"]);
    assert!(out.status.success(), "box json --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    // The document is the parsed input (an object with key `a`), emitted verbatim.
    assert!(
        v.get("a").and_then(|a| a.as_array()).is_some(),
        "`.a` is the input array, passed through verbatim"
    );

    // 2. PURITY — no ANSI escape (0x1B) anywhere.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // 3. PURITY — no UTF-8 BOM (EF BB BF) at the front.
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

/// SPINE-04 — live Windows-clipboard round-trip: `box json <doc> --clip` copies
/// the printed pretty form AND prints it; reading the clipboard back equals the
/// printed value. A `--compact --clip` variant proves the compact branch tees
/// too. `#[ignore]`d (touches shared OS clipboard). Run locally with:
///   cargo test --test json -- --ignored --test-threads=1
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_roundtrip() {
    const DOC: &str = "{\"b\":1,\"a\":2}";

    // Helper: run `box json <doc> --clip [extra]`, return trimmed printed stdout.
    let printed_with = |extra: &[&str]| -> String {
        let mut args = vec!["--clip"];
        args.extend_from_slice(extra);
        let out = json_passthrough_output(DOC, &args);
        assert!(out.status.success(), "box json --clip {extra:?} should exit 0");
        String::from_utf8(out.stdout)
            .expect("stdout is UTF-8")
            .trim()
            .to_string()
    };
    // Helper: read the clipboard back via `box clip --paste`, trimmed.
    let paste = || -> String {
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

    // Pretty (default) --clip: clipboard == the printed pretty form.
    let printed_pretty = printed_with(&[]);
    assert_eq!(paste(), printed_pretty, "--clip copies the pretty form");

    // --compact --clip: clipboard == the printed compact form.
    let printed_compact = printed_with(&["--compact"]);
    assert_eq!(
        paste(),
        printed_compact,
        "--compact --clip copies the compact form"
    );
}
