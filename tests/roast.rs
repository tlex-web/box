//! Integration tests for `box roast` (ROST-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. `roast` prints one
//! random programmer one-liner from the embedded CC0/self-authored list, picked
//! with `rand::rng()` + unbiased `IndexedRandom::choose` — never `% len`, never a
//! fixed seed (D-08).
//!
//! These assert *properties*, never a seeded value (RESEARCH Pattern F):
//! - output is exactly one non-empty record;
//! - output is a member of the bundled list (read straight from the source asset);
//! - output varies across runs (N=10 fresh processes → ≥2 distinct outputs).
//!
//! All runs set `NO_COLOR=1` so output is deterministic and ANSI-free.

use std::collections::HashSet;

use assert_cmd::Command;

/// The bundled roast asset, parsed the same way the command does. Membership is
/// asserted against THIS so the test and the binary share one source of truth.
const ROASTS_RAW: &str = include_str!("../src/data/roasts.txt");

/// Parse the embedded list into trimmed, non-empty entries (mirrors the loader).
fn entries() -> Vec<&'static str> {
    ROASTS_RAW
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

/// Run `box roast` with NO_COLOR, assert exit 0 + empty stderr, return stdout.
fn roast_stdout() -> String {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("roast")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box roast");
    assert!(
        out.status.success(),
        "box roast should exit 0, got {:?}; stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "box roast must write nothing to stderr; got {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// `box roast` → one non-empty record that is a member of the bundled list.
/// A long roast may soft-wrap, so compare whitespace-collapsed forms.
#[test]
fn roast_prints_a_member_of_the_list() {
    let out = roast_stdout();
    assert!(!out.trim().is_empty(), "roast output must be non-empty");

    let printed = collapse_ws(&out);
    let members: HashSet<String> = entries().iter().map(|e| collapse_ws(e)).collect();
    assert!(
        members.contains(&printed),
        "roast output {printed:?} is not a member of the bundled list"
    );
}

/// N=10 fresh processes yield ≥2 distinct outputs — proves the OS-seeded RNG
/// reseeds per process (no fixed seed).
#[test]
fn roast_varies_across_runs() {
    let mut seen = HashSet::new();
    for _ in 0..10 {
        seen.insert(collapse_ws(&roast_stdout()));
    }
    assert!(
        seen.len() >= 2,
        "roast must vary across runs (got {} distinct in 10: {seen:?})",
        seen.len()
    );
}

/// Collapse runs of whitespace (incl. soft-wrap newlines) to single spaces.
fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// --- Scriptable spine (SPINE-02) — copied from tests/uuid.rs ---------------------
//
// roast is a SCALAR command → a flat `{text}` object (identical shape to fortune).
// The JSON `text` is the UNWRAPPED single string (wrapping is human-only), so
// `.text` is verbatim-equal to a bundled entry. Not in SPINE-04 (no clip).

/// Capture `box roast --json` raw stdout bytes + exit status.
fn roast_output() -> std::process::Output {
    Command::cargo_bin("box")
        .unwrap()
        .arg("roast")
        .arg("--json")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box roast --json")
}

/// SPINE-02 / D-01 — `box roast --json` emits EXACTLY one flat `{text}` object
/// whose `text` is the UNWRAPPED one-liner (a verbatim member of the bundled list,
/// no soft-wrap line breaks); no ANSI, no UTF-8 BOM.
#[test]
fn json_purity() {
    let out = roast_output();
    assert!(out.status.success(), "box roast --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .expect("`.text` must be a string");
    assert!(
        !text.contains('\n'),
        "`.text` must be the UNWRAPPED single line (no soft-wrap newlines): {text:?}"
    );
    let members: HashSet<&str> = entries().into_iter().collect();
    assert!(
        members.contains(text),
        "`.text` {text:?} must be a verbatim member of the bundled list"
    );
    assert!(
        v.get("results").is_none(),
        "roast is scalar — no `results` wrapper"
    );

    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI in --json stdout");
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM"
    );
}

// --- Programming-language buckets (ROST-V2-01) -----------------------------------
//
// `box roast` gains a `--language <general|python|javascript|rust>` filter selecting
// a DEV ecosystem (roasts stay English — D-01), and a `language` scalar in the
// `--json` document. Bare `box roast` resolves to the `general` default bucket
// (today's behavior). An unknown `--language` is a clap usage error (exit 2) whose
// message lists the valid languages. Behavioral (binary-boundary) assertions.

/// The canonical language taxonomy (D-01), asserted against the CLI surface.
const LANGUAGES: [&str; 4] = ["general", "python", "javascript", "rust"];

/// Capture raw `Output` of `box roast` with the given extra args + NO_COLOR.
fn roast_with(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("roast");
    for a in args {
        cmd.arg(a);
    }
    cmd.env("NO_COLOR", "1").output().expect("run box roast")
}

/// `box roast --language python --json` reports `.language == "python"` and exits 0.
#[test]
fn language_filter_reports_that_language_in_json() {
    let out = roast_with(&["--language", "python", "--json"]);
    assert!(
        out.status.success(),
        "--language python --json should exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be one JSON value");
    assert_eq!(
        v.get("language").and_then(|l| l.as_str()),
        Some("python"),
        "`.language` must be \"python\" when filtered to python"
    );
}

/// Bare `box roast --json` resolves to the `general` default bucket (D-01) — the
/// `language` field is present and concrete, never null / absent.
#[test]
fn bare_json_reports_general_language() {
    let out = roast_with(&["--json"]);
    assert!(out.status.success(), "roast --json should exit 0");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be one JSON value");
    assert_eq!(
        v.get("language").and_then(|l| l.as_str()),
        Some("general"),
        "bare `.language` must resolve to \"general\""
    );
}

/// An unknown `--language` value is a clap usage error: exit 2, and stderr lists the
/// available languages.
#[test]
fn unknown_language_exits_2_and_lists_valid_values() {
    let out = roast_with(&["--language", "klingon"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "unknown --language must exit 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    for l in LANGUAGES {
        assert!(
            stderr.contains(l),
            "unknown --language stderr must list {l:?}; got: {stderr}"
        );
    }
}
