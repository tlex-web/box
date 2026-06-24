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
        .stderr(
            predicate::str::contains("line").and(predicate::str::contains("column")),
        );
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
