//! Wave-0 integration tests for the `box` CLI contract: help listing, version,
//! and the strict 0/1/2 exit-code policy. These run against the compiled binary
//! via `assert_cmd` and lock the UX contract every command inherits.

use assert_cmd::Command;
use predicates::prelude::*;

/// FOUND-01 / SC1 — `box --help` exits 0 and lists the command names. We assert
/// a representative set spanning the 23 commands; the full 23-line listing is
/// snapshot-checked by the trycmd transcript (`tests/cmd/help.trycmd`).
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
    Command::cargo_bin("box")
        .unwrap()
        .assert()
        .failure()
        .code(2);
}

// NOTE: the `stub_exits_1_to_stderr` test was removed in plan 05-04. It used
// `weather` as the last remaining NotImplemented stub; weather is now a real
// command, so NO stubs remain and the `BoxError::NotImplemented` variant +
// `commands::stub` module were deleted. The exit-1 RUNTIME-error contract those
// tested is now exercised by real command error paths (e.g. `tests/json.rs`
// invalid-JSON exit 1, `tests/color.rs` malformed-input exit 1, and
// `tests/weather.rs` offline exit 1).

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

/// SC4 — the five DISPLAY-ONLY commands (`matrix`, `pomodoro`, `lolcat`, `ascii`,
/// `clip`) OMIT the spine: the global `--json`/`--clip` flags PARSE (they are
/// global on `Cli`) but these commands never emit a machine-JSON spine document to
/// stdout. We verify the runnable, non-hanging subset LIVE — `clip` (piped stdin),
/// `ascii` (a tiny fixture image), and `lolcat` (a tiny arg) — asserting their
/// `--json` stdout does NOT parse into the `{results,count}` / spine envelope.
/// `matrix`/`pomodoro` loop or block (a real countdown / animation) and cannot be
/// run headless without hanging; their omission is guaranteed by source state
/// (their `mod.rs` never calls `emit_json`/`is_json_on`, confirmed at build time —
/// a grep gate in the plan acceptance criteria) and documented in each module's
/// `# Spine omission (SC4)` note.
#[test]
fn display_only_omit_json() {
    // A value carries the spine envelope if it is an object with a `results` array
    // + a `count` (the multi-capable shape) — the machine-JSON document SC4 forbids
    // these commands from emitting. A flat human line, raw bytes, or rainbow text
    // is NOT such a document.
    fn is_spine_document(bytes: &[u8]) -> bool {
        match serde_json::from_slice::<serde_json::Value>(bytes) {
            Ok(v) => v.get("results").map(|r| r.is_array()).unwrap_or(false)
                && v.get("count").is_some(),
            Err(_) => false,
        }
    }

    // clip (copy mode) with piped stdin → bounded, never blocks. `--json` must NOT
    // turn the clipboard copy into a JSON document on stdout (copy prints nothing).
    let clip_out = Command::cargo_bin("box")
        .unwrap()
        .args(["clip", "--json"])
        .write_stdin("sc4-display-only")
        .env("NO_COLOR", "1")
        .output()
        .expect("run box clip --json");
    assert!(
        clip_out.status.success(),
        "box clip --json should exit 0 (stderr: {})",
        String::from_utf8_lossy(&clip_out.stderr)
    );
    assert!(
        !is_spine_document(&clip_out.stdout),
        "box clip --json must NOT emit a JSON spine document to stdout (SC4)"
    );

    // ascii with the tiny fixture image → bounded render. `--json` must NOT wrap
    // the ASCII art in a JSON document.
    let ascii_out = Command::cargo_bin("box")
        .unwrap()
        .args(["ascii", "tests/cmd/ascii.in/tiny.png", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run box ascii --json");
    assert!(
        ascii_out.status.success(),
        "box ascii <tiny> --json should exit 0 (stderr: {})",
        String::from_utf8_lossy(&ascii_out.stderr)
    );
    assert!(
        !is_spine_document(&ascii_out.stdout),
        "box ascii <tiny> --json must NOT emit a JSON spine document to stdout (SC4)"
    );

    // lolcat with a tiny positional arg → bounded. Under NO_COLOR the rainbow is
    // plain text; `--json` must NOT wrap it in a JSON document.
    let lolcat_out = Command::cargo_bin("box")
        .unwrap()
        .args(["lolcat", "sc4-display-only", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run box lolcat --json");
    assert!(
        lolcat_out.status.success(),
        "box lolcat <text> --json should exit 0 (stderr: {})",
        String::from_utf8_lossy(&lolcat_out.stderr)
    );
    assert!(
        !is_spine_document(&lolcat_out.stdout),
        "box lolcat <text> --json must NOT emit a JSON spine document to stdout (SC4)"
    );
}
