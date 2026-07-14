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
    assert!(
        !text.trim().is_empty(),
        "render must be non-empty: {text:?}"
    );
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

// --- ASCI-V2-01 — truecolor + --braille + --invert (Wave-2 depth) ------------
//
// The rendered art is not byte-snapshotable (it depends on the piped→80 width), so
// these pin the CLI contract for the three additions:
//   - `braille_renders_dot_glyphs`  — --braille emits U+28xx braille glyphs.
//   - `invert_renders_non_empty`    — --invert renders cleanly (exit 0, non-empty).
//   - `piped_render_carries_no_ansi`— SC4: a redirected/NO_COLOR run degrades to the
//                                     mono ramp with no 0x1B (byte-identical-minus-color).
//   - `forced_color_emits_truecolor`— CLICOLOR_FORCE on a pipe DOES emit truecolor
//                                     SGR — proving color is gated ON the single gate.

/// ASCI-V2-01 — `box ascii <img> --braille` renders the 2×4 braille engine: the
/// output contains Unicode braille glyphs (`U+2800..=U+28FF`). NO_COLOR keeps it
/// mono so the glyphs are unwrapped.
#[test]
fn braille_renders_dot_glyphs() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["ascii", PNG_FIXTURE, "--braille"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run box ascii --braille");
    assert!(out.status.success(), "box ascii --braille should exit 0");
    let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(
        text.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)),
        "braille output must contain a U+28xx glyph, got: {text:?}"
    );
    assert!(
        !text.contains('\u{1b}'),
        "mono braille must carry no ANSI escape: {text:?}"
    );
}

/// ASCI-V2-01 — `box ascii <img> --invert` renders cleanly (exit 0, non-empty).
/// The pixel-level correctness of `255 - luma` is unit-locked in-module; this only
/// proves the flag is wired end-to-end. Composes with `--braille`.
#[test]
fn invert_renders_non_empty() {
    Command::cargo_bin("box")
        .unwrap()
        .args(["ascii", PNG_FIXTURE, "--invert"])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::is_empty().not());

    // --invert + --braille compose (exit 0, braille glyphs present).
    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["ascii", PNG_FIXTURE, "--braille", "--invert"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run box ascii --braille --invert");
    assert!(out.status.success(), "--braille --invert should exit 0");
    let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(
        text.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)),
        "inverted braille must still contain U+28xx glyphs"
    );
}

/// SC4 / ASCI-V2-01 — the NEW truecolor path is `is_color_on()`-gated: a piped
/// (`assert_cmd` captures stdout) run WITHOUT `NO_COLOR` still degrades to the mono
/// ramp and carries NO ANSI escape (`0x1B`). Byte-identical-minus-color when
/// redirected. Mirrors `tests/qr.rs::qr_piped_no_ansi`.
#[test]
fn piped_render_carries_no_ansi() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["ascii", PNG_FIXTURE])
        .output()
        .expect("run box ascii (piped)");
    assert!(out.status.success(), "box ascii should exit 0");
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "a piped ascii render must carry no ANSI escape (color must be is_color_on()-gated)"
    );
    // The same holds for the braille engine's gated color path.
    let braille = Command::cargo_bin("box")
        .unwrap()
        .args(["ascii", PNG_FIXTURE, "--braille"])
        .output()
        .expect("run box ascii --braille (piped)");
    assert!(braille.status.success(), "box ascii --braille should exit 0");
    assert!(
        !braille.stdout.contains(&0x1Bu8),
        "a piped braille render must carry no ANSI escape"
    );
}

/// SC4 / ASCI-V2-01 — the color gate has a live ON side: with `CLICOLOR_FORCE=1`
/// forcing color true even on a pipe, the render DOES emit a truecolor SGR escape
/// (`ESC[38;2;`). This is the companion to `piped_render_carries_no_ansi`: together
/// they prove truecolor is emitted iff `is_color_on()` — the single gate, no second
/// color stack.
#[test]
fn forced_color_emits_truecolor() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["ascii", PNG_FIXTURE])
        .env("CLICOLOR_FORCE", "1")
        .env_remove("NO_COLOR")
        .output()
        .expect("run box ascii (forced color)");
    assert!(out.status.success(), "box ascii (forced color) should exit 0");
    const TRUECOLOR_INTRO: &[u8] = b"\x1b[38;2;";
    assert!(
        out.stdout
            .windows(TRUECOLOR_INTRO.len())
            .any(|w| w == TRUECOLOR_INTRO),
        "forced-color ascii must emit a truecolor SGR escape (ESC[38;2;)"
    );
}
