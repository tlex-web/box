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
//! LOL-V2-01 (`--animate`) automatable subset (the on-screen smoothness + clean
//! terminal restore are the PS7 human-verify gate — 09-03 Task 3, NOT here):
//!   4. piped/non-TTY `--animate` degrades to the static one-pass render — exits
//!      cleanly, emits no `\x1b`, and is byte-identical to the non-animate render
//!      (SC3/SC4): the loop NEVER runs off-TTY.
//!   5. the piped `--animate` invocation does NOT hang on `event::poll` without a
//!      TTY (guarded by an `assert_cmd` timeout that kills + fails on a hang).
//!   6. `--freq`/`--seed` change the COLORED static render: with `CLICOLOR_FORCE`
//!      forcing color on a pipe, two different `--seed` values differ, AND even
//!      then `--animate` STILL degrades to static (no alt-screen escape) — the
//!      real T-09-03-PIPE proof that `is_terminal()` (not `is_color_on()`) gates
//!      the raw-mode loop.
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

// ---------------------------------------------------------------------------
// LOL-V2-01 — `--animate` automatable subset (the PS7 smoothness/restore gate is
// Task 3, not here). assert_cmd captures stdout via a pipe, so every run below is
// non-TTY by construction — exactly the degrade path these tests pin.
// ---------------------------------------------------------------------------

use std::time::Duration;

/// The crossterm "enter alternate screen" escape (`ESC [ ? 1049 h`). Its presence
/// in piped output would mean the raw-mode loop ran off-TTY — the SC3-forbidden
/// hazard. The degrade path must NEVER emit it.
const ALT_SCREEN_ENTER: &[u8] = b"\x1b[?1049h";

/// Run `box lolcat <args...>` against the compiled binary with a hard 10s timeout
/// (so a hang on `event::poll` FAILS the test instead of blocking CI), explicit
/// color env, and optional piped stdin. Returns the raw `Output` for byte asserts.
///
/// * `no_color`    — set `NO_COLOR=1` (force color OFF) when true.
/// * `force_color` — set `CLICOLOR_FORCE=1` (force color ON even on a pipe) when
///   true; mutually exclusive with `no_color` (NO_COLOR wins in the gate anyway).
fn run_lolcat(
    args: &[&str],
    stdin: Option<&str>,
    no_color: bool,
    force_color: bool,
) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("lolcat").args(args);
    cmd.timeout(Duration::from_secs(10));
    // Pin the color decision deterministically regardless of the host env.
    cmd.env_remove("NO_COLOR").env_remove("CLICOLOR_FORCE");
    if no_color {
        cmd.env("NO_COLOR", "1");
    }
    if force_color {
        cmd.env("CLICOLOR_FORCE", "1");
    }
    if let Some(s) = stdin {
        cmd.write_stdin(s);
    }
    cmd.output().expect("run box lolcat")
}

/// (4) Piped/non-TTY `--animate` under NO_COLOR DEGRADES to the static render:
/// exits 0, emits NO `\x1b` byte (and no alt-screen escape), and is byte-identical
/// to the non-animate render. Proves the loop is unreachable off-TTY (SC3/SC4).
/// Covers both the arg form (`box lolcat "x" --animate`) and the stdin-piped form.
#[test]
fn piped_animate_degrades_to_static_byte_identical() {
    // Arg form.
    let animate = run_lolcat(&["Hello World", "--animate"], None, true, false);
    let static_ = run_lolcat(&["Hello World"], None, true, false);
    assert!(
        animate.status.success(),
        "piped `lolcat <text> --animate` must exit 0 (stderr: {})",
        String::from_utf8_lossy(&animate.stderr)
    );
    assert!(
        !animate.stdout.contains(&0x1b),
        "piped --animate (NO_COLOR) must emit no \\x1b byte: {:?}",
        String::from_utf8_lossy(&animate.stdout)
    );
    assert!(
        !animate.stdout.windows(ALT_SCREEN_ENTER.len()).any(|w| w == ALT_SCREEN_ENTER),
        "piped --animate must NEVER emit the alternate-screen escape (SC3)"
    );
    assert_eq!(
        animate.stdout, static_.stdout,
        "piped --animate must be byte-identical to the static one-pass render"
    );

    // Stdin-piped form: `echo hi | box lolcat --animate`.
    let animate_in = run_lolcat(&["--animate"], Some("line one\nline two\n"), true, false);
    let static_in = run_lolcat(&[], Some("line one\nline two\n"), true, false);
    assert!(animate_in.status.success(), "piped-stdin --animate must exit 0");
    assert!(
        !animate_in.stdout.contains(&0x1b),
        "piped-stdin --animate must emit no \\x1b byte"
    );
    assert_eq!(
        animate_in.stdout, static_in.stdout,
        "piped-stdin --animate must equal the static render byte-for-byte"
    );
}

/// (5) Non-hanging smoke: a piped `--animate` invocation TERMINATES promptly
/// without a TTY. If the `is_terminal()` gate were missing the process would call
/// `enable_raw_mode()` + block on `event::poll` forever; the 10s `run_lolcat`
/// timeout would then kill it and `status.success()` would be false. A clean exit
/// proves the loop was never entered.
#[test]
fn piped_animate_does_not_hang() {
    let out = run_lolcat(&["x", "--animate", "--duration", "0"], None, true, false);
    assert!(
        out.status.success(),
        "piped --animate --duration 0 must return immediately (degrade path), not hang"
    );
}

/// (6a) `--seed` changes the COLORED static render. Color cannot be observed when
/// piped UNLESS forced, so set `CLICOLOR_FORCE=1`: now the piped static render is
/// colored, two different `--seed` values produce different ANSI bytes, and both
/// carry an `\x1b` (proving color was actually on — otherwise the diff would be
/// vacuous). The *visible* on-screen effect remains the PS7 human gate (Task 3).
#[test]
fn seed_changes_colored_static_output() {
    let seed0 = run_lolcat(&["Hello World", "--seed", "0"], None, false, true);
    let seed50 = run_lolcat(&["Hello World", "--seed", "50"], None, false, true);
    assert!(seed0.status.success() && seed50.status.success(), "both runs exit 0");
    assert!(
        seed0.stdout.contains(&0x1b) && seed50.stdout.contains(&0x1b),
        "CLICOLOR_FORCE must make the piped static render colored (\\x1b present)"
    );
    assert_ne!(
        seed0.stdout, seed50.stdout,
        "two different --seed values must produce a different colored gradient"
    );
}

/// (6b) The real T-09-03-PIPE proof: even with `CLICOLOR_FORCE` forcing color ON,
/// a piped `--animate` STILL degrades to the static render — it does NOT enter the
/// raw-mode loop, because the gate keys on `is_terminal()`, not `is_color_on()`.
/// The output is colored (forced) yet byte-identical to the non-animate colored
/// render and carries NO alternate-screen escape.
#[test]
fn forced_color_piped_animate_still_degrades_to_static() {
    let animate = run_lolcat(&["Hello World", "--animate"], None, false, true);
    let static_ = run_lolcat(&["Hello World"], None, false, true);
    assert!(
        animate.status.success(),
        "forced-color piped --animate must exit 0 (degrade), not hang or error"
    );
    assert!(
        animate.stdout.contains(&0x1b),
        "with CLICOLOR_FORCE the degraded render is colored (\\x1b present)"
    );
    assert!(
        !animate.stdout.windows(ALT_SCREEN_ENTER.len()).any(|w| w == ALT_SCREEN_ENTER),
        "even under CLICOLOR_FORCE, --animate must NOT enter the alternate screen on a pipe (T-09-03-PIPE)"
    );
    assert_eq!(
        animate.stdout, static_.stdout,
        "forced-color --animate must equal the colored static render byte-for-byte"
    );
}
