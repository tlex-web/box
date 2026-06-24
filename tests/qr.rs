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
