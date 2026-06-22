//! Integration tests for `box uuid` (UUID-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. uuid is a
//! non-deterministic command (each run reseeds from the OS), so the discipline
//! is **properties, not values** (RESEARCH:492-512): every line must match the
//! v4 regex, `-n N` yields N lines, and large batches are all distinct.

use std::collections::HashSet;

use assert_cmd::Command;
use predicates::prelude::*;

/// RFC-4122 v4 lowercase regex: 8-4-4-4-12 hex, version nibble `4`, variant
/// nibble `[89ab]` (RESEARCH:512).
const V4_RE: &str = r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";
/// Same shape, uppercase, for `--upper`.
const V4_RE_UPPER: &str = r"^[0-9A-F]{8}-[0-9A-F]{4}-4[0-9A-F]{3}-[89AB][0-9A-F]{3}-[0-9A-F]{12}$";

/// Run `box uuid` with the given extra args and return the assert handle.
/// Forces `NO_COLOR=1` so output is plain regardless of the runner's TTY state.
fn uuid(args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("uuid");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Capture `box uuid <args>` stdout as trimmed lines (exit 0, stderr empty).
fn uuid_lines(args: &[&str]) -> Vec<String> {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("uuid");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    let out = cmd.output().expect("run box uuid");
    assert!(out.status.success(), "box uuid {args:?} should exit 0");
    assert!(
        out.stderr.is_empty(),
        "box uuid {args:?} stderr should be empty, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("stdout is UTF-8")
        .lines()
        .map(str::to_string)
        .collect()
}

/// `box uuid` → exactly one line matching the v4 regex; exit 0; stderr empty.
#[test]
fn single_uuid_is_v4() {
    // Assert exit 0 + empty stderr via the assert handle; match the v4 regex on
    // the single trimmed line (the raw stdout carries a trailing `\n`, so an
    // anchored `^…$` against the whole captured buffer would not match).
    uuid(&[]).success().stderr(predicate::str::is_empty());
    let re = regex_lite_match(V4_RE);
    let lines = uuid_lines(&[]);
    assert_eq!(lines.len(), 1, "expected one line, got {lines:?}");
    assert!(re(&lines[0]), "line is not a v4 UUID: {}", lines[0]);
}

/// `box uuid -n 5` → 5 lines, all distinct, all matching the v4 regex.
#[test]
fn count_five_distinct_v4() {
    let re = regex_lite_match(V4_RE);
    let lines = uuid_lines(&["-n", "5"]);
    assert_eq!(lines.len(), 5, "expected 5 lines, got {lines:?}");
    for line in &lines {
        assert!(re(line), "line is not a v4 UUID: {line}");
    }
    let distinct: HashSet<&String> = lines.iter().collect();
    assert_eq!(distinct.len(), 5, "5 UUIDs must be distinct: {lines:?}");
}

/// `box uuid --upper` → the uppercase form of a valid v4 UUID.
#[test]
fn upper_is_uppercase_v4() {
    uuid(&["--upper"]).success().stderr(predicate::str::is_empty());
    let re = regex_lite_match(V4_RE_UPPER);
    let lines = uuid_lines(&["--upper"]);
    assert_eq!(lines.len(), 1, "expected one line, got {lines:?}");
    assert!(re(&lines[0]), "line is not an uppercase v4 UUID: {}", lines[0]);
}

/// `box uuid -n 100` → 100 distinct lines (uniqueness via HashSet).
#[test]
fn count_hundred_all_distinct() {
    let lines = uuid_lines(&["-n", "100"]);
    assert_eq!(lines.len(), 100, "expected 100 lines, got {}", lines.len());
    let distinct: HashSet<&String> = lines.iter().collect();
    assert_eq!(distinct.len(), 100, "100 UUIDs must all be distinct");
}

/// Tiny anchored-regex matcher built on `predicates`' regex backend, returned as
/// a closure so per-line assertions in loops stay terse. (`predicates` re-exports
/// the same `regex` engine used by `predicate::str::is_match`.)
fn regex_lite_match(pattern: &str) -> impl Fn(&str) -> bool {
    let p = predicate::str::is_match(pattern).unwrap();
    move |s: &str| p.eval(s)
}
