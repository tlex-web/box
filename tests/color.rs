//! Integration tests for `box color` (COLR-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. The exact aligned
//! Hex/RGB/Tuple/HSL block (plus the plain `██████████` swatch under NO_COLOR)
//! is locked by the trycmd snapshot (`tests/cmd/color.trycmd`); these tests
//! cover the cross-cutting CLI contract: bidirectional input parses to the same
//! block, and malformed input exits 1 cleanly with no panic (T-02-06).

use assert_cmd::Command;
use predicates::prelude::*;

/// Run `box color <input>` with `NO_COLOR=1`, returning the assert handle.
fn color(input: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("color").arg(input);
    cmd.env("NO_COLOR", "1");
    cmd.assert()
}

/// Capture `box color <input>` stdout (exit 0, stderr empty), NO_COLOR.
fn color_stdout(input: &str) -> String {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("color").arg(input).env("NO_COLOR", "1");
    let out = cmd.output().expect("run box color");
    assert!(out.status.success(), "box color {input:?} should exit 0");
    assert!(
        out.stderr.is_empty(),
        "stderr should be empty, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// Hex input yields the locked block with the uppercase hex, CSS rgb(), tuple,
/// and HSL rows, and a plain `██████████` swatch under NO_COLOR.
#[test]
fn hex_input_prints_full_block() {
    let out = color_stdout("#3b82f6");
    assert!(out.contains("#3B82F6"), "hex row uppercase: {out:?}");
    assert!(out.contains("rgb(59, 130, 246)"), "rgb row: {out:?}");
    assert!(out.contains("59 130 246"), "tuple row: {out:?}");
    assert!(out.contains("hsl(217, 91%, 60%)"), "hsl row: {out:?}");
    assert!(out.contains("██████████"), "plain swatch present: {out:?}");
    // Under NO_COLOR the whole output is byte-identical minus ANSI: no escapes.
    assert!(!out.contains('\u{1b}'), "no ANSI under NO_COLOR: {out:?}");
}

/// RGB input (both comma- and space-separated) yields the identical block.
#[test]
fn rgb_inputs_match_hex_block() {
    let from_hex = color_stdout("#3b82f6");
    let from_comma = color_stdout("59,130,246");
    let from_space = color_stdout("59 130 246");
    assert_eq!(from_comma, from_hex, "comma RGB should match hex block");
    assert_eq!(from_space, from_hex, "space RGB should match hex block");
}

/// Short hex `#abc` expands to `(170,187,204)` → `#AABBCC` (Pitfall 8 / D-13).
#[test]
fn short_hex_expands() {
    let out = color_stdout("#abc");
    assert!(out.contains("#AABBCC"), "short-hex expand: {out:?}");
    assert!(out.contains("rgb(170, 187, 204)"), "short-hex rgb: {out:?}");
}

/// Malformed hex → exit 1, error on stderr, empty stdout, no panic (T-02-06).
#[test]
fn malformed_hex_exits_1_clean() {
    color("#zz")
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}

/// Out-of-range RGB component → exit 1, clean stderr, no panic (T-02-06).
#[test]
fn out_of_range_rgb_exits_1_clean() {
    color("300,0,0")
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().not());
}
