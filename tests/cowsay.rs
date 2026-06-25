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
