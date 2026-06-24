//! Integration tests for `box clip` (CLIP-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. The clipboard is
//! SHARED OS state, so the tests that actually touch the Win32 clipboard
//! (round-trip copy→paste, paste-exits-cleanly) are `#[ignore]`d by default:
//! they are order-dependent (one test's copy would clobber another's), and a CI
//! runner may have no interactive clipboard at all (headless / session-0 service
//! → arboard `Clipboard::new()` fails). Run them locally, serialized, with:
//!
//!   cargo test --test clip -- --ignored --test-threads=1
//!
//! The DETERMINISTIC, environment-independent contract IS auto-tested without a
//! clipboard:
//!   - the newline-trim policy (D-05) is unit-tested in `src/commands/clip/mod.rs`;
//!   - non-UTF-8 stdin on copy → clean exit 1 with a stderr message and NO panic
//!     (the FOUND-05 no-panic invariant) — mirrors the `tests/color.rs`
//!     malformed-input exit-1 shape (color.rs:67-74). This test does NOT depend on
//!     a clipboard: UTF-8 validation happens BEFORE `arboard::Clipboard::new()`.

use assert_cmd::Command;
use predicates::prelude::*;

/// CLIP-01 / D-05 — round-trip Unicode through the live Win32 clipboard: copy
/// `héllo 世界` (with the PowerShell-style trailing CRLF the pipe would add), then
/// `--paste` it back and assert stdout equals the input MINUS the one trailing
/// terminator (the D-05 single-trim), exit 0, Unicode byte-exact.
///
/// `#[ignore]`d: touches shared OS clipboard state (order-dependent across tests)
/// and requires an interactive clipboard the CI runner may lack. Run locally with
/// `--ignored --test-threads=1`. The human-verify checkpoint (Task 3) is the
/// canonical real-clipboard proof; this is the local automated mirror.
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn round_trip_unicode_single_threaded() {
    // Copy with a trailing CRLF (what `"héllo 世界" | box clip` sends in PS7).
    Command::cargo_bin("box")
        .unwrap()
        .arg("clip")
        .write_stdin("héllo 世界\r\n")
        .assert()
        .success();

    // Paste it back — byte-exact, and the one trailing CRLF was trimmed on copy.
    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["clip", "--paste"])
        .output()
        .expect("run box clip --paste");
    assert!(out.status.success(), "box clip --paste should exit 0");
    let pasted = String::from_utf8(out.stdout).expect("clipboard text is UTF-8");
    assert_eq!(
        pasted, "héllo 世界",
        "round-trip must preserve Unicode and strip exactly the one trailing CRLF"
    );
}

/// CLIP-01 / D-05 — `box clip --paste` exits 0 cleanly (reads the live clipboard
/// to stdout). `#[ignore]`d: requires an interactive clipboard the CI runner may
/// lack. Run locally with `--ignored`.
#[test]
#[ignore = "requires an interactive OS clipboard; run locally with --ignored"]
fn paste_exits_zero() {
    // Seed the clipboard first so paste has deterministic content.
    Command::cargo_bin("box")
        .unwrap()
        .arg("clip")
        .write_stdin("seed")
        .assert()
        .success();

    Command::cargo_bin("box")
        .unwrap()
        .args(["clip", "--paste"])
        .assert()
        .success()
        .stdout(predicate::str::contains("seed"));
}

/// WR-02 — when the clipboard holds no text (empty, or non-text content like an
/// image or a file list), `box clip --paste` must surface the distinct, clearer
/// "clipboard is empty or contains no text" message (exit 1) instead of the generic
/// "read clipboard: …". `#[ignore]`d and NOT auto-runnable: the `box` binary has no
/// way to deterministically PUT the OS clipboard into the `ContentNotAvailable`
/// state (there is no clear-clipboard or copy-image subcommand — `clip` only ever
/// sets text), and the clipboard is shared OS state a CI runner may lack entirely.
/// To exercise locally: empty the clipboard manually (e.g. copy an image, or run
/// PowerShell `Set-Clipboard -Value $null`) and then
/// `cargo test --test clip clip_paste_empty -- --ignored`. The deterministic half
/// of the diagnosability fix is verified by code review of the match arm; this test
/// is the local, manually-seeded confirmation of the message text.
#[test]
#[ignore = "requires the OS clipboard to be manually emptied first; run locally with --ignored"]
fn clip_paste_empty_reports_no_text() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["clip", "--paste"])
        .output()
        .expect("run box clip --paste");
    // Exit code is unchanged by WR-02 — still a plain runtime error (exit 1).
    assert_eq!(
        out.status.code(),
        Some(1),
        "an empty/non-text clipboard must stay exit 1, got: {:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("clipboard is empty or contains no text"),
        "the empty-clipboard message must be the clearer WR-02 text, got: {stderr}"
    );
    assert!(
        !stderr.contains("read clipboard"),
        "the empty-clipboard case must NOT use the generic read-failure context, got: {stderr}"
    );
}

/// CLIP-01 / D-04 / FOUND-05 (T-05-CLIP-DoS) — non-UTF-8 stdin on copy → clean
/// exit 1 with a stderr message and NO panic. This is DETERMINISTIC and needs no
/// clipboard: `String::from_utf8(buf)` rejects the bytes BEFORE `Clipboard::new()`
/// is ever called, so the failure is the UTF-8 error, not a clipboard error.
/// Mirrors `tests/color.rs::malformed_hex_exits_1_clean` (color.rs:67-74) and adds
/// the no-`panicked` assertion (the no-panic invariant).
#[test]
fn non_utf8_copy_exits_1_no_panic() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("clip")
        .write_stdin([0xff_u8, 0xfe_u8].as_slice())
        .output()
        .expect("run box clip");
    assert_eq!(
        out.status.code(),
        Some(1),
        "non-UTF-8 copy must exit 1, got: {:?}",
        out.status.code()
    );
    assert!(
        out.stdout.is_empty(),
        "stdout must stay empty on a copy error, got: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.is_empty(), "a stderr error message is required");
    assert!(
        !stderr.contains("panicked"),
        "non-UTF-8 copy must NOT panic, stderr was: {stderr}"
    );
}
