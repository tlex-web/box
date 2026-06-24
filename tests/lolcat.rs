//! Integration tests for `box lolcat` (LOL-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. lolcat's truecolor
//! rainbow is NOT snapshotable (per-char ANSI is perceptual — RESEARCH:462, no
//! `.trycmd`), so these pin the byte-level CLI contract instead:
//!   1. piped → clean plain UTF-8: under NO_COLOR the stripped text round-trips
//!      byte-for-byte with NO `\x1b` escape (D-13/D-14).
//!   2. multi-byte UTF-8 intact: a string with wide/accented scalars survives
//!      with no mojibake — proves coloring is per Unicode scalar, not per byte
//!      (D-12).
//!   3. incoming ANSI stripped: a pre-existing `\x1b[31m…\x1b[0m` escape is
//!      removed before re-emit, neutralizing terminal-escape injection
//!      (T-04L-01 / D-13).
//!
//! Mirrors the `tests/color.rs` runner shape (`cargo_bin("box")`, `NO_COLOR=1`,
//! `assert!(!out.contains('\u{1b}'))`). Input is fed via `.write_stdin(...)` so
//! the piped (non-TTY) branch of `read_input` is exercised.

use assert_cmd::Command;

/// Run `box lolcat` with `stdin` piped and `NO_COLOR=1`, returning the raw
/// `Output` (exit status + stdout BYTES) for byte-level assertions.
fn lolcat_piped(stdin: &str) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("lolcat").env("NO_COLOR", "1").write_stdin(stdin);
    cmd.output().expect("run box lolcat")
}

/// (1) Piped under NO_COLOR → exit 0, NO `\x1b` byte anywhere, and the visible
/// text round-trips unchanged (the stripped plain text equals the input).
/// Byte-identical minus ANSI is the D-14 contract for piping to a file.
#[test]
fn piped_output_is_plain_text_no_ansi() {
    let input = "line one\nline two\n";
    let out = lolcat_piped(input);
    assert!(out.status.success(), "box lolcat should exit 0");
    assert!(
        !out.stdout.contains(&0x1b),
        "piped lolcat output must contain no \\x1b byte: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
    let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert_eq!(
        text, input,
        "no-color lolcat must round-trip the input byte-for-byte (minus ANSI)"
    );
}

/// (2) A string with multi-byte scalars (accented + wide CJK + an arrow) passes
/// through with those EXACT scalars intact — no byte-split corruption, no
/// mojibake. Proves the per-Unicode-scalar emit (D-12), not a per-byte loop.
#[test]
fn multibyte_utf8_passes_through_intact() {
    let input = "héllo→世界\n";
    let out = lolcat_piped(input);
    assert!(out.status.success(), "box lolcat should exit 0");
    let text = String::from_utf8(out.stdout).expect("stdout must be valid UTF-8");
    assert!(text.contains("héllo"), "accented scalars intact: {text:?}");
    assert!(text.contains('→'), "arrow scalar intact: {text:?}");
    assert!(text.contains("世界"), "wide CJK scalars intact: {text:?}");
    assert!(
        !text.contains('\u{fffd}'),
        "no U+FFFD replacement char (no byte-split corruption): {text:?}"
    );
}

/// (3) Pre-existing ANSI in the input is stripped unconditionally before
/// re-emit (D-13 / T-04L-01): `\x1b[31mx\x1b[0m` → the visible `x` survives but
/// NO `\x1b` byte remains. This is the terminal-escape-injection mitigation.
#[test]
fn incoming_ansi_is_stripped() {
    let input = "\x1b[31mx\x1b[0m\n";
    let out = lolcat_piped(input);
    assert!(out.status.success(), "box lolcat should exit 0");
    // Scan the bytes for an escape BEFORE consuming `stdout` into a String.
    let has_esc = out.stdout.contains(&0x1b);
    let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(text.contains('x'), "visible char survives strip: {text:?}");
    assert!(
        !has_esc,
        "incoming ANSI must be stripped — no \\x1b byte: {text:?}"
    );
}

/// (4) WR-06 — a lone/embedded carriage return is stripped end-to-end, so no
/// stray `\r` reaches the terminal to move the cursor to column 0 and overwrite
/// the already-printed colored prefix. Feeding `"a\rb\n"` yields `"ab\n"` with
/// NO `\r` (0x0d) byte anywhere. Only `\n` line breaks are supported (D-11/D-12);
/// the `\r` is removed by the unconditional D-13 strip before any emit.
#[test]
fn lone_carriage_return_is_stripped() {
    let out = lolcat_piped("a\rb\n");
    assert!(out.status.success(), "box lolcat should exit 0");
    assert!(
        !out.stdout.contains(&0x0d),
        "no carriage-return (\\r) byte must survive: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
    let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert_eq!(
        text, "ab\n",
        "lone CR is dropped, leaving the visible chars and the \\n: {text:?}"
    );
}
