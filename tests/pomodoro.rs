//! Integration (smoke) test for `box pomodoro` (POMO-01).
//!
//! The live countdown, the real Windows toast, and the interactive Ctrl+C/q/Esc
//! cancel are NOT auto-snapshotable (a live, timed, terminal-bound render + a
//! WinRT toast that is human-verified — Task 3, deferred to phase UAT). POMO-01's
//! TESTABLE process-level invariant is the **enter/exit contract**: `box pomodoro`
//! must START and EXIT non-hanging when fed a cancel key (or a closed/non-TTY
//! stdin) and must not panic — i.e. the RAII restore path is reachable and the
//! loop honors a cancel key. The pure countdown/format/cancel logic
//! (`fmt_mmss` / `resolve_duration` / `is_cancel`) is unit-tested in-module
//! (`cargo test --bin box pomodoro`); this file only proves the binary's
//! process-level contract.
//!
//! `assert_cmd` pipes stdin by default, so the process launches
//! non-interactively. We feed it `q` and bound it with a timeout so a regression
//! that ignores the cancel key (or hangs) fails loudly rather than blocking the
//! suite.

use std::time::Duration;

use assert_cmd::Command;

/// `box pomodoro 1` fed `q` on a (piped, non-TTY) stdin must exit non-hanging and
/// without panicking. On a non-TTY `enable_raw_mode()` may fail to find a console —
/// the command still must not hang or panic; the timeout is the hang backstop and a
/// panic would surface as a process abort the harness reports. We assert the process
/// terminates within the bound (the timeout turns a hang into a test failure) and
/// does not abort via panic. We do NOT pin an exit code: a non-TTY environment may
/// legitimately error out of raw-mode setup (clean exit 1), which is still a
/// non-hanging, panic-free termination (mirrors `tests/matrix.rs`).
#[test]
fn pomodoro_starts_and_exits_non_hanging() {
    let mut cmd = Command::cargo_bin("box").unwrap();
    let assert = cmd
        .arg("pomodoro")
        .arg("1")
        .write_stdin("q")
        .timeout(Duration::from_secs(5))
        .assert();

    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "box pomodoro must not panic; stderr was: {stderr}"
    );
}

/// WR-01 / FOUND-05 (T-05 no-panic) — an out-of-range `[MINUTES]` (here the maximum
/// `u64`, which would otherwise wrap `mins * 60` and/or overflow-panic
/// `Instant::now() + total`) is rejected by clap's `RangedU64ValueParser` as a
/// USAGE error: exit code 2, a clap error on stderr, NO `panicked`, and NO hang.
/// This is the deterministic, clipboard/terminal-independent proof that an absurd
/// numeric argument can no longer abort the process. The fixed `1` timeout doubles
/// as the hang backstop: a parse rejection returns immediately, well inside it.
#[test]
fn pomodoro_absurd_minutes_exits_2_no_panic() {
    let assert = Command::cargo_bin("box")
        .unwrap()
        .args(["pomodoro", "18446744073709551615"])
        .timeout(Duration::from_secs(5))
        .assert();

    let output = assert.get_output();
    assert_eq!(
        output.status.code(),
        Some(2),
        "an out-of-range minutes arg must be a clap usage error (exit 2), got: {:?}",
        output.status.code()
    );
    assert!(
        output.stdout.is_empty(),
        "stdout must stay clean on a usage error, got: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "an out-of-range minutes arg must NOT panic/abort; stderr was: {stderr}"
    );
    assert!(
        !stderr.is_empty(),
        "clap must print a usage error to stderr for the out-of-range value"
    );
}

/// WR-01 — the lower bound holds too: `0` minutes is rejected (exit 2) rather than
/// silently running a zero-length timer. Bounds the accepted range at `1..` on the
/// low end, mirroring the `du --depth`/`--top` "must be >= 1" parser contract.
#[test]
fn pomodoro_zero_minutes_exits_2() {
    let assert = Command::cargo_bin("box")
        .unwrap()
        .args(["pomodoro", "0"])
        .timeout(Duration::from_secs(5))
        .assert();

    let code = assert.get_output().status.code();
    assert_eq!(
        code,
        Some(2),
        "a zero minutes arg must be a clap usage error (exit 2), got: {code:?}"
    );
}

/// POMO-V2-01 — `--cycles` is bounded (`1..=1000`) by the same `RangedU64ValueParser`
/// pattern as `[MINUTES]`: an out-of-range cycle count is a clap USAGE error (exit 2)
/// BEFORE `run()`, so an absurd value can never spin the outer segment loop. Returns
/// immediately (well inside the timeout) and must not panic.
#[test]
fn pomodoro_cycles_over_max_exits_2() {
    let assert = Command::cargo_bin("box")
        .unwrap()
        .args(["pomodoro", "--cycles", "100000"])
        .timeout(Duration::from_secs(5))
        .assert();

    let output = assert.get_output();
    assert_eq!(
        output.status.code(),
        Some(2),
        "an out-of-range --cycles must be a clap usage error (exit 2), got: {:?}",
        output.status.code()
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "an out-of-range --cycles must NOT panic"
    );
}

/// POMO-V2-01 — `--cycles` and `--loop` are mutually exclusive: supplying both is a
/// clap conflict (exit 2), so the "bounded N cycles" and "unbounded loop" modes can
/// never be requested together. Returns immediately; never hangs.
#[test]
fn pomodoro_cycles_and_loop_conflict_exits_2() {
    let assert = Command::cargo_bin("box")
        .unwrap()
        .args(["pomodoro", "--cycles", "4", "--loop"])
        .timeout(Duration::from_secs(5))
        .assert();

    let code = assert.get_output().status.code();
    assert_eq!(
        code,
        Some(2),
        "--cycles with --loop must be a clap conflict (exit 2), got: {code:?}"
    );
}

/// POMO-V2-01 — the auto-cycle depth flags land on `box pomodoro --help` (exit 0).
/// This is the fast, non-blocking proof that `--cycles`, `--loop`, and `--label` are
/// wired onto the arg surface (the live countdown itself cannot be run headless —
/// see the module note). `--help` returns immediately, well inside the timeout.
/// (The `--sound` flag is asserted by `pomodoro_sound_flag_parses`, POMO-V2-02.)
#[test]
fn pomodoro_help_lists_new_flags() {
    let assert = Command::cargo_bin("box")
        .unwrap()
        .args(["pomodoro", "--help"])
        .timeout(Duration::from_secs(5))
        .assert();

    let output = assert.get_output();
    assert!(
        output.status.success(),
        "box pomodoro --help must exit 0 (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout);
    for flag in ["--cycles", "--loop", "--label"] {
        assert!(
            help.contains(flag),
            "box pomodoro --help must list `{flag}`; help was:\n{help}"
        );
    }
}
