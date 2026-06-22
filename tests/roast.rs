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
