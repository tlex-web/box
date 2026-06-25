//! Integration tests for `box fortune` (FORT-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. `fortune` prints one
//! random aphorism from the embedded CC0/public-domain list. Randomness comes
//! from `rand::rng()` (OS-seeded ThreadRng, ChaCha12) and the line is picked with
//! unbiased `IndexedRandom::choose` — never `% len`, never a fixed seed (D-08).
//!
//! These assert *properties*, never a seeded value (RESEARCH Pattern F):
//! - output is exactly one non-empty record;
//! - output is a member of the bundled list (read straight from the source asset);
//! - output varies across runs (N=10 fresh processes → ≥2 distinct outputs).
//!
//! All runs set `NO_COLOR=1` so output is deterministic and ANSI-free.

use std::collections::HashSet;

use assert_cmd::Command;

/// The bundled fortune asset, parsed the same way the command does (non-empty
/// trimmed lines). Membership is asserted against THIS so the test and the
/// binary share one source of truth.
const FORTUNES_RAW: &str = include_str!("../src/data/fortunes.txt");

/// Parse the embedded list into trimmed, non-empty entries (mirrors the loader).
fn entries() -> Vec<&'static str> {
    FORTUNES_RAW
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

/// Run `box fortune` with NO_COLOR, assert exit 0 + empty stderr, return the
/// trimmed single-line stdout.
fn fortune_stdout() -> String {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("fortune")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box fortune");
    assert!(
        out.status.success(),
        "box fortune should exit 0, got {:?}; stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "box fortune must write nothing to stderr; got {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// `box fortune` → one non-empty record that is a member of the bundled list.
///
/// A long fortune may be soft-wrapped across several printed lines, so the
/// membership check rejoins the printed lines with a space and asserts the
/// whitespace-collapsed result equals a (whitespace-collapsed) list entry.
#[test]
fn fortune_prints_a_member_of_the_list() {
    let out = fortune_stdout();
    assert!(!out.trim().is_empty(), "fortune output must be non-empty");

    let printed = collapse_ws(&out);
    let members: HashSet<String> = entries().iter().map(|e| collapse_ws(e)).collect();
    assert!(
        members.contains(&printed),
        "fortune output {printed:?} is not a member of the bundled list"
    );
}

/// N=10 fresh processes yield ≥2 distinct outputs — proves the OS-seeded RNG
/// reseeds per process (no fixed seed). P(all 10 identical) is negligible for a
/// list of dozens of entries.
#[test]
fn fortune_varies_across_runs() {
    let mut seen = HashSet::new();
    for _ in 0..10 {
        seen.insert(collapse_ws(&fortune_stdout()));
    }
    assert!(
        seen.len() >= 2,
        "fortune must vary across runs (got {} distinct in 10: {seen:?})",
        seen.len()
    );
}

/// Collapse all runs of whitespace (incl. the soft-wrap newlines) to single
/// spaces and trim, so a wrapped multi-line render compares equal to its
/// single-line source entry.
fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// --- Scriptable spine (SPINE-02) — copied from tests/uuid.rs ---------------------
//
// fortune is a SCALAR command → a flat `{text}` object. The JSON `text` is the
// UNWRAPPED single string (soft-wrapping is a human-render concern, never data —
// D-17 discretion), so `.text` is verbatim-equal to a bundled entry (no
// whitespace-collapse needed). Not in SPINE-04 (no clip).

/// Capture `box fortune --json` raw stdout bytes + exit status.
fn fortune_output() -> std::process::Output {
    Command::cargo_bin("box")
        .unwrap()
        .arg("fortune")
        .arg("--json")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box fortune --json")
}

/// SPINE-02 / D-01 — `box fortune --json` emits EXACTLY one flat `{text}` object
/// whose `text` is the UNWRAPPED aphorism (a verbatim member of the bundled list,
/// no soft-wrap line breaks injected); no ANSI, no UTF-8 BOM.
#[test]
fn json_purity() {
    let out = fortune_output();
    assert!(out.status.success(), "box fortune --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .expect("`.text` must be a string");
    // The UNWRAPPED string is a verbatim member of the bundled list (no wrap-induced
    // newlines): assert exact membership, not a whitespace-collapsed match.
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
        "fortune is scalar — no `results` wrapper"
    );

    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI in --json stdout");
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM"
    );
}
