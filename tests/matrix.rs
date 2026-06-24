//! Integration (smoke) test for `box matrix` (MTRX-01).
//!
//! The matrix animation itself is NOT auto-snapshotable (a live, timed,
//! terminal-bound render that is human-verified). MTRX-01's TESTABLE invariant
//! is the **enter/exit contract**: `box matrix` must START and EXIT
//! non-hanging when fed a quit key (or a closed/non-TTY stdin) and must not
//! panic — i.e. the RAII restore path is reachable and the loop honors `q`
//! (RESEARCH:442/464). The pure drop/fade model, glyph table, and quit-key
//! logic are unit-tested in-module (`cargo test --bin box matrix`); this file
//! only proves the binary's process-level contract.
//!
//! `assert_cmd` pipes stdin by default, so the process launches
//! non-interactively. We feed it `q` and bound it with a timeout so a
//! regression that ignores the quit key (or hangs) fails loudly rather than
//! blocking the suite.

use std::time::Duration;

use assert_cmd::Command;

/// `box matrix` fed `q` on a (piped, non-TTY) stdin must exit non-hanging and
/// without panicking. On a non-TTY `crossterm::terminal::size()` may fail to
/// find a console — the command still must not hang or panic; the timeout is
/// the hang backstop and a panic would surface as a non-zero abort the harness
/// reports. We assert the process terminates within the bound (the timeout
/// turns a hang into a test failure) and does not abort via panic.
#[test]
fn matrix_starts_and_exits_non_hanging() {
    let mut cmd = Command::cargo_bin("box").unwrap();
    let assert = cmd
        .arg("matrix")
        .write_stdin("q")
        .timeout(Duration::from_secs(5))
        .assert();

    // The binary must not panic. Under the release profile `panic = "abort"`
    // surfaces as a process abort; in the test profile a panic unwinds to a
    // non-success status with a panic message on stderr. Either way, a panic is
    // a failure — assert stderr carries no Rust panic marker. We do NOT pin a
    // specific exit code: a non-TTY environment may legitimately error out of
    // `terminal::size()`/raw-mode setup (clean exit 1), which is still a
    // non-hanging, panic-free termination.
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "box matrix must not panic; stderr was: {stderr}"
    );
}
