//! Wave-0 integration tests for the `box` CLI contract: help listing, version,
//! and the strict 0/1/2 exit-code policy. These run against the compiled binary
//! via `assert_cmd` and lock the UX contract every command inherits.

use assert_cmd::Command;
use predicates::prelude::*;

/// FOUND-01 / SC1 — `box --help` exits 0 and lists the command names. We assert
/// a representative set spanning built + stub commands; the full 23-line listing
/// is snapshot-checked by the trycmd transcript (`tests/cmd/help.trycmd`).
#[test]
fn help_lists_23_commands() {
    Command::cargo_bin("box")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("flatten")
                .and(predicate::str::contains("qr"))
                .and(predicate::str::contains("hash"))
                .and(predicate::str::contains("uuid"))
                .and(predicate::str::contains("weather"))
                .and(predicate::str::contains("8ball"))
                .and(predicate::str::contains("bulk-rename")),
        );
}

/// FOUND-02 / SC2 — `box --version` exits 0 and prints a semver.
#[test]
fn version_is_semver() {
    Command::cargo_bin("box")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+\.\d+\.\d+").unwrap());
}

/// D-07 / SC2 — an unknown command is a clap parse error: exit 2, error on
/// stderr (never collapsed to exit 1).
#[test]
fn badcmd_exits_2() {
    Command::cargo_bin("box")
        .unwrap()
        .arg("definitely-not-a-cmd")
        .assert()
        .failure()
        .code(2);
}

/// D-08 / SC2 — bare `box` (no subcommand) exits 2. clap's default for
/// `arg_required_else_help` is exit 0; `main()` must override it (Pitfall 2).
#[test]
fn bare_box_exits_2() {
    Command::cargo_bin("box").unwrap().assert().failure().code(2);
}

/// D-06 / FOUND-03 — invoking an unbuilt command exits 1 with the message on
/// stderr only; stdout stays empty so pipes are never corrupted.
#[test]
fn stub_exits_1_to_stderr() {
    Command::cargo_bin("box")
        .unwrap()
        .arg("qr")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not yet implemented"))
        .stdout(predicate::str::is_empty());
}

/// FOUND-04 / SC3 — when stdout is not a TTY (assert_cmd captures it via a pipe),
/// `box flatten --help` output must contain no ANSI escape sequence (`\x1b[`).
/// Proves the color gate disables ANSI off-TTY (the `piped_help_has_no_ansi`
/// validation test). Uses `flatten` specifically per the plan; `--help` is
/// always available even while the command is a stub.
#[test]
fn piped_help_has_no_ansi() {
    let out = Command::cargo_bin("box")
        .unwrap()
        .args(["flatten", "--help"])
        .output()
        .expect("run box flatten --help");
    assert!(out.status.success(), "flatten --help should exit 0");
    // ESC `[` is the start of every CSI/SGR ANSI sequence.
    assert!(
        !out.stdout.windows(2).any(|w| w == b"\x1b["),
        "piped help must contain no ANSI escape (\\x1b[)"
    );
}

/// FOUND-01 / SC1 — snapshot the full `box --help` listing of all 23 commands
/// via a trycmd transcript (`tests/cmd/*.trycmd`). Locks the doc-comment `about`
/// text and the complete command set against accidental drift.
#[test]
fn trycmd() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd");
}
