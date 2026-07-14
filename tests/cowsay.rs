//! Integration tests for `box cowsay` (COW-01) — the `--json` spine slice.
//!
//! cowsay's byte-exact bubble is locked by `src/commands/cowsay/mod.rs`'s unit
//! tests + `tests/cmd/cowsay-*.trycmd`; this NEW file is the home for cowsay's
//! `--json` purity test (no `tests/cowsay.rs` existed before Phase 7). cowsay is
//! `--json`-only (it is NOT in SPINE-04), so there is no `clip_roundtrip` here.
//!
//! Schema (A6): under `--json`, cowsay emits a flat `{text}` object carrying the
//! raw spoken message — the ASCII bubble/cow is a *visual* (like qr glyphs) and is
//! NOT serialized.
//!
//! All runs set `NO_COLOR=1` so output is deterministic and ANSI-free.

use assert_cmd::Command;

/// Capture `box cowsay <args>` raw stdout bytes + exit status for the purity
/// assertions (which inspect raw bytes for ANSI/BOM, not a trimmed String).
fn cowsay_output(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("cowsay");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box cowsay")
}

/// SPINE-02 / D-01 — `box cowsay moo --json` emits EXACTLY one well-formed JSON
/// document carrying a `text` string field; no human bubble chrome, no ANSI, no
/// UTF-8 BOM. Copied from the frozen `tests/uuid.rs::json_purity` template,
/// adapted to cowsay's flat `{text}` schema (A6).
#[test]
fn json_purity() {
    let out = cowsay_output(&["moo", "--json"]);
    assert!(out.status.success(), "box cowsay moo --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value (whole-buffer from_slice).
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. The flat schema (A6): a `text` string carrying the raw message — NOT the
    //    rendered bubble (no `<`, no cow art in the value).
    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .expect("`.text` must be a string");
    assert_eq!(text, "moo", "`.text` is the raw spoken message");
    assert!(
        v.get("results").is_none(),
        "cowsay is scalar — no `results` wrapper"
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

// --- COW-V2-01: figure roster + --figure / --list-figures / --think ---------------
//
// The curated built-in figure roster (D-02, no external `.cow` I/O), the
// thought-bubble `--think` mode (D-03), and the roster enumerator. The default
// no-flag output stays byte-identical (locked by `tests/cmd/cowsay-*.trycmd`); these
// tests cover the NEW additive surface. All runs set NO_COLOR=1 (via
// `cowsay_output`) so output is deterministic and ANSI-free.

/// COW-V2-01 / D-02 — `box cowsay --figure dragon "hi"` renders an ALTERNATE figure
/// under the speech bubble, not the classic cow. The message is still bubbled
/// (`< hi >`), but the cow's signature `^__^` face must be gone.
#[test]
fn figure_selects_alternate_art() {
    let out = cowsay_output(&["--figure", "dragon", "hi"]);
    assert!(
        out.status.success(),
        "box cowsay --figure dragon should exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("< hi >"),
        "the message is still bubbled: {stdout:?}"
    );
    assert!(
        !stdout.contains("^__^"),
        "an alternate figure must NOT render the classic cow face `^__^`: {stdout:?}"
    );
}

/// COW-V2-01 / D-02 — `box cowsay --list-figures` enumerates the roster (>= 5
/// names, one per line), exits 0, and renders NO bubble. The default `cow` plus at
/// least `dragon` must appear.
#[test]
fn list_figures_enumerates_roster() {
    let out = cowsay_output(&["--list-figures"]);
    assert!(
        out.status.success(),
        "box cowsay --list-figures should exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    let names: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        names.len() >= 5,
        "roster lists >= 5 figures, got {}: {names:?}",
        names.len()
    );
    assert!(names.iter().any(|n| n.trim() == "cow"), "roster includes cow");
    assert!(
        names.iter().any(|n| n.trim() == "dragon"),
        "roster includes dragon"
    );
    assert!(
        !stdout.contains('<'),
        "--list-figures renders no speech bubble: {stdout:?}"
    );
}

/// COW-V2-01 / D-03 — `box cowsay --think "hi"` renders the THOUGHT bubble: rounded
/// `( )` delimiters instead of the `< >` speech delimiters. The content line must
/// use `( hi )`, and no `<` speech delimiter may appear anywhere.
#[test]
fn think_renders_thought_bubble() {
    let out = cowsay_output(&["--think", "hi"]);
    assert!(
        out.status.success(),
        "box cowsay --think should exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("( hi )"),
        "think mode uses rounded `( )` content delimiters: {stdout:?}"
    );
    assert!(
        !stdout.contains('<'),
        "think mode must NOT use the `<` speech delimiter: {stdout:?}"
    );
}

/// COW-V2-01 / D-02 / T-10-01-FIG — `box cowsay --figure __nope__ "x"` is a USAGE
/// error: exit 2, and the stderr message NAMES the available figures so the user
/// can correct the call (a fixed-roster lookup, never a file read).
#[test]
fn unknown_figure_exits_2_and_lists_available() {
    let out = cowsay_output(&["--figure", "__nope__", "x"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "an unknown figure is a usage error → exit 2"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cow") && stderr.contains("dragon"),
        "the error must list the available figures, got: {stderr:?}"
    );
}
