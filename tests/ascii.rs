//! Integration tests for `box ascii` (ASCI-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. The rendered ASCII is
//! NOT snapshotable byte-for-byte (it depends on the detected terminal width, and
//! under a pipe `terminal_width()` falls back to 80 — RESEARCH:285), so these pin
//! the CLI contract instead:
//!   1. PNG render: the checked-in `tiny.png` renders → exit 0, non-empty stdout,
//!      valid UTF-8 (ASCII ramp glyphs).
//!   2. JPEG render: the checked-in `tiny.jpg` renders → exit 0, non-empty stdout
//!      (proves BOTH formats work — ROADMAP Phase 4 success criterion 3).
//!   3. Missing file → exit 1, NO panic: a non-existent path errors cleanly via
//!      anyhow's `.with_context(...)?` (FOUND-05 / T-04A-02) — stderr is non-empty
//!      and contains no `panicked` backtrace text.
//!
//! Fixtures (`tests/cmd/ascii.in/{tiny.png,tiny.jpg}`) are tiny 8x8 grayscale
//! diagonal gradients — small enough to commit, varied enough that the dark→light
//! ramp produces several distinct glyphs (a non-empty, non-uniform render). The
//! extension MUST match the format because `image::open` detects by EXTENSION,
//! not content (Pitfall 2).
//!
//! Mirrors the `tree()` runner shape (`cargo_bin("box")`, `NO_COLOR=1`). ASCII is
//! monochrome v1 (D-03) so `NO_COLOR` is belt-and-suspenders; the render carries
//! no ANSI regardless.

use assert_cmd::Command;
use predicates::prelude::*;

/// Path to a checked-in fixture under `tests/cmd/ascii.in/`.
const PNG_FIXTURE: &str = "tests/cmd/ascii.in/tiny.png";
const JPG_FIXTURE: &str = "tests/cmd/ascii.in/tiny.jpg";
const MISSING_FIXTURE: &str = "tests/cmd/ascii.in/does-not-exist.png";

/// Run `box ascii <path>` with plain output forced and return the assert handle.
fn ascii(path: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("ascii").arg(path).env("NO_COLOR", "1");
    cmd.assert()
}

/// (1) The PNG fixture renders: exit 0 with non-empty stdout. The output is the
/// dark→light ramp glyphs fitted to the (piped → 80) column width.
#[test]
fn renders_png_fixture() {
    ascii(PNG_FIXTURE)
        .success()
        .code(0)
        .stdout(predicate::str::is_empty().not());
}

/// (1b) The rendered PNG output is valid UTF-8 (ASCII ramp characters only) — a
/// String round-trip proves no byte corruption in the render path.
#[test]
fn png_render_is_valid_utf8() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("ascii")
        .arg(PNG_FIXTURE)
        .env("NO_COLOR", "1")
        .output()
        .expect("run box ascii");
    assert!(out.status.success(), "box ascii <png> should exit 0");
    let text = String::from_utf8(out.stdout).expect("stdout must be valid UTF-8");
    assert!(!text.trim().is_empty(), "render must be non-empty: {text:?}");
    assert!(
        !text.contains('\u{1b}'),
        "monochrome v1 render must carry no ANSI escape: {text:?}"
    );
}

/// (2) The JPEG fixture renders: exit 0 with non-empty stdout. Together with (1)
/// this proves both PNG and JPEG inputs work (ROADMAP Phase 4 criterion 3).
#[test]
fn renders_jpeg_fixture() {
    ascii(JPG_FIXTURE)
        .success()
        .code(0)
        .stdout(predicate::str::is_empty().not());
}

/// (3) A missing file errors cleanly: exit 1, non-empty stderr, and NO `panicked`
/// text (the decode error is mapped via `.with_context(...)?`, never unwrapped —
/// FOUND-05 / T-04A-02).
#[test]
fn missing_file_exits_1_no_panic() {
    ascii(MISSING_FIXTURE)
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty().not())
        .stderr(predicate::str::contains("panicked").not());
}
