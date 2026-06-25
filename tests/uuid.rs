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
    uuid(&["--upper"])
        .success()
        .stderr(predicate::str::is_empty());
    let re = regex_lite_match(V4_RE_UPPER);
    let lines = uuid_lines(&["--upper"]);
    assert_eq!(lines.len(), 1, "expected one line, got {lines:?}");
    assert!(
        re(&lines[0]),
        "line is not an uppercase v4 UUID: {}",
        lines[0]
    );
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

// --- Scriptable spine (SPINE-01 / SPINE-03) — the copy-me Phase-7 template -----
//
// uuid is the first --json/--clip consumer of the wave-1 spine. These tests are
// THE frozen template Phase 7 copies across the other 21 commands, so they are
// kept clean and self-documenting:
//   - `json_purity`        — one well-formed JSON value, {results,count} shape,
//                            version "v4", NO ANSI (0x1B), NO UTF-8 BOM (D-01/D-02/D-03).
//   - `json_count_multi`   — `-n 5 --json` → count 5, results length 5.
//   - `human_output_unchanged` — the non-JSON path is byte-stable (regression).
//   - `clip_roundtrip`     — #[ignore]d live-clipboard round-trip (SPINE-03 / D-07).

/// Capture `box uuid <args>` raw stdout bytes plus the exit status, for the
/// JSON-purity assertions (which must inspect raw bytes for ANSI/BOM, not a
/// trimmed String). Forces `NO_COLOR=1` for determinism (mirrors `tests/hash.rs`).
fn uuid_output(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("uuid");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box uuid")
}

/// SPINE-01 / D-01/D-02/D-03 — `box uuid --json` emits EXACTLY one well-formed
/// JSON document `{"results":[{"uuid":…,"version":"v4"}],"count":1}` on stdout:
/// snake_case fields, no UTF-8 BOM, no ANSI escape, no human chrome. This is the
/// copy-me JSON-purity template for all 23 Phase-7 commands.
/// Runnable via `cargo test --test uuid json_purity`.
#[test]
fn json_purity() {
    let out = uuid_output(&["--json"]);
    assert!(out.status.success(), "box uuid --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value (from_slice over the whole
    //    buffer succeeds only when there is one document + optional trailing ws).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. The D-01/D-02 shape: an object wrapping a `results` array + a `count`,
    //    always-wrapped even for N=1 (NOT a bare array — Phase-8-compatible).
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 1, "single uuid → one result element");
    assert_eq!(v.get("count"), Some(&serde_json::json!(1)), "`.count` == 1");

    // 3. The element struct (D-03 locked field names): `uuid` (36-char canonical)
    //    + `version` == "v4".
    let elem = &results[0];
    assert_eq!(
        elem.get("version"),
        Some(&serde_json::json!("v4")),
        "`.results[0].version` must be the literal \"v4\""
    );
    let uuid_str = elem
        .get("uuid")
        .and_then(|u| u.as_str())
        .expect("`.results[0].uuid` must be a string");
    assert_eq!(uuid_str.len(), 36, "canonical hyphenated UUID is 36 chars");

    // 4. PURITY — no ANSI escape (0x1B) anywhere (Pitfall 1).
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // 5. PURITY — no UTF-8 BOM (EF BB BF) at the front (Pitfall 2).
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

/// SPINE-01 — `box uuid -n 5 --json` → `.count == 5` and a 5-element `results`
/// array of the same `{uuid,version}` struct. Runnable via
/// `cargo test --test uuid json_count_multi`.
#[test]
fn json_count_multi() {
    let out = uuid_output(&["-n", "5", "--json"]);
    assert!(out.status.success(), "box uuid -n 5 --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(v.get("count"), Some(&serde_json::json!(5)), "`.count` == 5");
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 5, "`-n 5` → 5 result elements");
    for elem in results {
        assert_eq!(
            elem.get("version"),
            Some(&serde_json::json!("v4")),
            "every element is version v4"
        );
        assert_eq!(
            elem.get("uuid").and_then(|u| u.as_str()).map(str::len),
            Some(36),
            "every element carries a 36-char uuid"
        );
    }
}

/// Regression backstop — `box uuid` (no flags) STILL prints exactly one 36-char
/// UUID line on stdout (the human path is byte-stable after the spine adoption).
/// Runnable via `cargo test --test uuid human_output_unchanged`.
#[test]
fn human_output_unchanged() {
    let lines = uuid_lines(&[]);
    assert_eq!(lines.len(), 1, "expected one line, got {lines:?}");
    assert_eq!(
        lines[0].len(),
        36,
        "the human path still prints one 36-char canonical UUID, got: {}",
        lines[0]
    );
    let re = regex_lite_match(V4_RE);
    assert!(
        re(&lines[0]),
        "the human line is still a v4 UUID: {}",
        lines[0]
    );
}

/// SPINE-03 / D-07 — live Windows-clipboard round-trip: `box uuid --clip` copies
/// the printed UUID AND prints it; reading the clipboard back equals the printed
/// value. `#[ignore]`d (touches shared OS clipboard state; a headless CI runner
/// may have no interactive clipboard — mirrors `tests/clip.rs`). Run locally with:
///   cargo test --test uuid -- --ignored --test-threads=1
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_roundtrip() {
    // `box uuid --clip` prints the UUID to stdout AND tees it to the clipboard.
    let printed = {
        let out = uuid_output(&["--clip"]);
        assert!(out.status.success(), "box uuid --clip should exit 0");
        String::from_utf8(out.stdout)
            .expect("stdout is UTF-8")
            .trim()
            .to_string()
    };

    // Read it back via `box clip --paste` (the v1 paste path) — it must equal the
    // single printed UUID (flush_clip trims the trailing newline exactly once).
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
        "--clip must copy exactly the printed UUID to the clipboard"
    );
}
