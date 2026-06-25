//! Integration tests for `box qr` (QR-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. These tests pin the
//! QR-01 byte-level contract (D-01/D-02/D-03) — the camera-readability of the
//! rendered QR is a physical/perceptual property covered by the human-verify
//! checkpoint (phone scan), not here. What IS auto-testable:
//!   - input as a positional arg renders a half-block QR (exit 0, non-empty);
//!   - the SAME input via piped stdin renders identically (the `-`/no-arg path);
//!   - the output carries NO ANSI even WITHOUT `NO_COLOR` — `qr` must NEVER emit
//!     ANSI (the whole reason qr2term was rejected, D-01). Mirrors the run-
//!     WITHOUT-`NO_COLOR` pipe-path proof from `tests/json.rs::json_piped_no_ansi`.

use assert_cmd::Command;

/// True if `s` contains at least one Unicode half-block glyph (`▀`/`▄`/`█`) —
/// the proof that the Dense1x2 renderer produced real QR output, not an empty or
/// ANSI-only stream.
fn has_half_block(s: &str) -> bool {
    s.contains('▀') || s.contains('▄') || s.contains('█')
}

/// (1) D-01/D-02 — input as a positional arg renders a half-block QR: `box qr
/// "https://example.com"` exits 0 with non-empty stdout containing a half-block
/// glyph and no ANSI.
#[test]
fn arg_renders_half_block_qr() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .arg("https://example.com")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box qr");
    assert!(out.status.success(), "box qr <arg> should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(!stdout.is_empty(), "rendered QR must be non-empty");
    assert!(
        has_half_block(&stdout),
        "output must contain a half-block glyph (▀/▄/█), got: {stdout:?}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "output must contain no ANSI escape: {stdout:?}"
    );
}

/// (2) D-03 — the SAME input via piped stdin (no arg) also renders a half-block
/// QR. Proves the `core::input` arg → stdin precedence is wired for `qr`.
#[test]
fn piped_stdin_renders_half_block_qr() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .env("NO_COLOR", "1")
        .write_stdin("https://example.com")
        .output()
        .expect("run box qr");
    assert!(out.status.success(), "box qr (piped) should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(
        has_half_block(&stdout),
        "piped output must contain a half-block glyph (▀/▄/█), got: {stdout:?}"
    );
}

/// (3) D-01 — run WITHOUT `NO_COLOR` but with piped stdout (assert_cmd captures
/// stdout via a pipe, so it is not a TTY): the output must contain no ANSI escape
/// (`\x1b[`). `qr` must NEVER emit ANSI — the exact reason qr2term was rejected.
/// Mirrors `json_piped_no_ansi` (json.rs:109-123).
#[test]
fn qr_piped_no_ansi() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .write_stdin("https://example.com")
        .output()
        .expect("run box qr");
    assert!(out.status.success(), "box qr should exit 0");
    // ESC `[` is the start of every CSI/SGR ANSI sequence.
    assert!(
        !out.stdout.windows(2).any(|w| w == b"\x1b["),
        "piped qr output must contain no ANSI escape (\\x1b[)"
    );
}

// --- Scriptable spine (SPINE-02 / SPINE-04) — D-14 metadata + D-15 clip-text ---
//
// qr is a Wave-7c odd-fit: under --json it emits ENCODE METADATA, not the glyph
// block (D-14: `{text, error_correction}`); under --clip it copies the SOURCE
// TEXT, not the rendered half-blocks (D-15 — the one deliberate break from the
// copy-all rule, since pasting ▀▄ glyphs as clipboard text is garbage).
//   - `json_metadata_not_glyphs` — --json yields {text, error_correction:"M"},
//                                  NO glyphs in the document (D-14).
//   - `json_purity`              — one JSON value, the QrOutput shape, no 0x1B/BOM.
//   - `clip_copies_source_text`  — #[ignore]d: pasted == INPUT text, not glyphs (D-15).

/// Capture `box qr <args>` raw stdout bytes + exit status for the JSON-purity
/// assertions (which inspect raw bytes for ANSI/BOM). Forces `NO_COLOR=1`.
fn qr_output(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("qr");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box qr")
}

/// D-14 — `box qr "https://example.com" --json` emits encode METADATA, NOT the
/// glyph block: `{text, error_correction:"M"}`. The half-block glyphs (▀/▄/█) are
/// a visual, never a scriptable value, so the JSON document must carry NONE of
/// them. Runnable via `cargo test --test qr json_metadata_not_glyphs`.
#[test]
fn json_metadata_not_glyphs() {
    let out = qr_output(&["https://example.com", "--json"]);
    assert!(out.status.success(), "box qr --json should exit 0");

    // stdout is exactly one JSON value.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // D-14 metadata: text == the input, error_correction == "M".
    assert_eq!(
        v.get("text"),
        Some(&serde_json::json!("https://example.com")),
        "`.text` must echo the encoded input"
    );
    assert_eq!(
        v.get("error_correction"),
        Some(&serde_json::json!("M")),
        "`.error_correction` must be the fixed v1 level \"M\""
    );

    // The document carries NO half-block glyph anywhere (metadata, not the render).
    let stdout = String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8");
    assert!(
        !has_half_block(&stdout),
        "the --json document must NOT contain glyphs (▀/▄/█): {stdout:?}"
    );
}

/// SPINE-02 — `box qr <text> --json` emits EXACTLY one well-formed JSON document
/// `{"text":…,"error_correction":"M"}`: no UTF-8 BOM, no ANSI escape, no human
/// chrome. The QrOutput-shaped variant of the frozen `json_purity` template.
/// Runnable via `cargo test --test qr json_purity`.
#[test]
fn json_purity() {
    let out = qr_output(&["https://example.com", "--json"]);
    assert!(out.status.success(), "box qr --json should exit 0");

    // 1. stdout parses as EXACTLY one JSON value.
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    // 2. The flat QrOutput shape (scalar → flat object, D-01): a `text` string +
    //    an `error_correction` string. No `results`/`count` wrapper (qr is scalar).
    assert!(
        v.get("text").and_then(|t| t.as_str()).is_some(),
        "`.text` must be a string"
    );
    assert!(
        v.get("error_correction")
            .and_then(|e| e.as_str())
            .is_some(),
        "`.error_correction` must be a string"
    );

    // 3. PURITY — no ANSI escape (0x1B) anywhere.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout"
    );
    // 4. PURITY — no UTF-8 BOM (EF BB BF) at the front.
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix --json stdout"
    );
}

/// SPINE-04 / D-15 — live Windows-clipboard round-trip with the deliberate qr
/// exception: `box qr <text> --clip` prints the glyph block AND copies the SOURCE
/// TEXT (not the glyphs); reading the clipboard back equals the INPUT string.
/// `#[ignore]`d (touches shared OS clipboard). Run locally with:
///   cargo test --test qr -- --ignored --test-threads=1
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_copies_source_text() {
    const INPUT: &str = "https://example.com";

    // `box qr <text> --clip` prints the glyphs AND tees the SOURCE TEXT to clip.
    let out = qr_output(&[INPUT, "--clip"]);
    assert!(out.status.success(), "box qr --clip should exit 0");
    let printed = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    // The human stdout is still the glyph block (display unchanged, D-15).
    assert!(
        has_half_block(&printed),
        "the human --clip path still prints the glyph block"
    );

    // Read the clipboard back — it must be the INPUT text, NOT the glyph block.
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
        pasted, INPUT,
        "--clip must copy the SOURCE TEXT (D-15), not the rendered glyph block"
    );
    assert!(
        !has_half_block(&pasted),
        "the clipboard must NOT contain glyphs (D-15)"
    );
}
