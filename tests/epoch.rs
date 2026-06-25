//! Integration tests for `box epoch` (EPOC-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. Three D-12 modes:
//! no-arg → current Unix timestamp (a single integer near now); integer arg →
//! two labeled human-date lines (local + UTC); a junk string → exit 1 with a
//! format hint on stderr and an empty stdout (no panic). `NO_COLOR=1` keeps
//! output plain regardless of the runner's TTY state.

use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use predicates::prelude::*;

/// Run `box epoch <args>` with `NO_COLOR=1`, returning the assert handle.
fn epoch(args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("epoch");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// `box epoch` (no arg) → a single line matching `^\d+$` whose value is within a
/// sane window of the wall-clock now (proves it prints the current timestamp).
#[test]
fn no_arg_prints_current_timestamp() {
    let out = {
        let mut cmd = Command::cargo_bin("box").unwrap();
        cmd.arg("epoch").env("NO_COLOR", "1");
        cmd.output().expect("run box epoch")
    };
    assert!(out.status.success(), "box epoch should exit 0");
    assert!(out.stderr.is_empty(), "stderr should be empty");
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let line = stdout.trim();
    let re = predicate::str::is_match(r"^\d+$").unwrap();
    assert!(
        re.eval(line),
        "expected a bare integer timestamp, got {line:?}"
    );
    let printed: i64 = line.parse().expect("timestamp parses as i64");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    // Generous ±5 min window covers any reasonable test-runner clock skew.
    assert!(
        (now - printed).abs() < 300,
        "printed timestamp {printed} should be within 5 min of now {now}"
    );
}

/// `box epoch 1700000000` → two lines carrying the `Local` and `UTC` labels for
/// that fixed timestamp (D-12 integer mode).
#[test]
fn integer_prints_local_and_utc() {
    let out = {
        let mut cmd = Command::cargo_bin("box").unwrap();
        cmd.arg("epoch").arg("1700000000").env("NO_COLOR", "1");
        cmd.output().expect("run box epoch 1700000000")
    };
    assert!(out.status.success(), "box epoch 1700000000 should exit 0");
    assert!(out.stderr.is_empty(), "stderr should be empty");
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected two date lines, got {lines:?}");
    assert!(
        stdout.contains("Local"),
        "output should label the local date: {stdout:?}"
    );
    assert!(
        stdout.contains("UTC"),
        "output should label the UTC date: {stdout:?}"
    );
    // 1700000000 == 2023-11-14T22:13:20Z; the UTC line must show that date.
    assert!(
        stdout.contains("2023-11-14"),
        "UTC date for 1700000000 should be 2023-11-14: {stdout:?}"
    );
}

/// `box epoch "not-a-date"` → exit 1, a format hint on stderr, empty stdout, no
/// panic (T-02-05).
#[test]
fn bad_format_exits_1_with_hint() {
    epoch(&["not-a-date"])
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("RFC3339").or(predicate::str::contains("YYYY-MM-DD")));
}

/// A recognized date string round-trips back to a timestamp on stdout (single
/// integer), proving the date-string branch is wired through the CLI.
#[test]
fn date_string_prints_timestamp() {
    let out = {
        let mut cmd = Command::cargo_bin("box").unwrap();
        cmd.arg("epoch")
            .arg("2023-11-14T22:13:20+00:00")
            .env("NO_COLOR", "1");
        cmd.output().expect("run box epoch <rfc3339>")
    };
    assert!(out.status.success(), "box epoch <rfc3339> should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert_eq!(
        stdout.trim(),
        "1700000000",
        "RFC3339 2023-11-14T22:13:20Z should map to 1700000000"
    );
}

// --- Scriptable spine (SPINE-02 / SPINE-04) — copied from tests/uuid.rs ----------
//
// epoch is a SCALAR command with the D-17 LOCKED UNIFIED shape: `{epoch, utc, local}`
// for EVERY input mode (now / int / string) — no branching on input direction. NOT
// in SPINE-04 per the plan's flag list, but the plan adds a clip_roundtrip here.

/// Capture `box epoch <args>` raw stdout bytes + exit status for the purity
/// assertions (which inspect raw bytes for ANSI/BOM).
fn epoch_output(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("epoch");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box epoch")
}

/// Assert the unified `{epoch, utc, local}` shape on a parsed JSON value: `epoch`
/// is an integer, `utc`/`local` are non-empty strings, and there is no `results`
/// wrapper (scalar, flat object).
fn assert_epoch_shape(v: &serde_json::Value, expected_epoch: Option<i64>) {
    let epoch = v
        .get("epoch")
        .and_then(|e| e.as_i64())
        .expect("`.epoch` must be an integer");
    if let Some(want) = expected_epoch {
        assert_eq!(epoch, want, "`.epoch` must equal {want}");
    }
    let utc = v
        .get("utc")
        .and_then(|u| u.as_str())
        .expect("`.utc` must be a string");
    let local = v
        .get("local")
        .and_then(|l| l.as_str())
        .expect("`.local` must be a string");
    assert!(!utc.is_empty(), "`.utc` must be non-empty");
    assert!(!local.is_empty(), "`.local` must be non-empty");
    assert!(
        v.get("results").is_none(),
        "epoch is scalar — no `results` wrapper"
    );
}

/// SPINE-02 / D-17 — `box epoch --json` emits the UNIFIED `{epoch, utc, local}`
/// shape across BOTH now-mode and explicit-int mode (same keys, no input-direction
/// branching); no ANSI, no UTF-8 BOM. Adapted from `tests/uuid.rs::json_purity`.
#[test]
fn json_purity() {
    // now mode (no arg).
    let now_out = epoch_output(&["--json"]);
    assert!(now_out.status.success(), "box epoch --json should exit 0");
    let now_v: serde_json::Value =
        serde_json::from_slice(&now_out.stdout).expect("stdout must be exactly one JSON value");
    assert_epoch_shape(&now_v, None);
    assert!(!now_out.stdout.contains(&0x1Bu8), "no ANSI in --json stdout");
    assert_ne!(
        &now_out.stdout[..3.min(now_out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM"
    );

    // explicit-int mode — `box epoch 0 --json` → the SAME unified shape with
    // `.epoch == 0`.
    let int_out = epoch_output(&["0", "--json"]);
    assert!(int_out.status.success(), "box epoch 0 --json should exit 0");
    let int_v: serde_json::Value =
        serde_json::from_slice(&int_out.stdout).expect("stdout must be exactly one JSON value");
    assert_epoch_shape(&int_v, Some(0));
}

/// SPINE-04 / D-07 — live Windows-clipboard round-trip for `box epoch --clip`.
/// `#[ignore]`d (touches shared OS clipboard). Run locally with:
///   cargo test --test epoch -- --ignored --test-threads=1
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_roundtrip() {
    let printed = {
        let out = epoch_output(&["0", "--clip"]);
        assert!(out.status.success(), "box epoch 0 --clip should exit 0");
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
        "--clip must copy exactly the printed epoch result to the clipboard"
    );
}
