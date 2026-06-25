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
