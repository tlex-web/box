//! Integration proofs for the `box completions` meta-command (CMP-01), Phase 11
//! Plan 02.
//!
//! Black-box against the compiled binary via `assert_cmd`. Locks the CMP-01
//! contract: stdout purity (no ANSI/BOM/stderr chatter, the `--json`-grade
//! discipline — D-10), live-Cli reflection (SC3 — the script references every
//! subcommand incl. the two Phase-11 additions plus the global flags), the inert
//! `#`-comment registration header (D-10 first inert place), the `--help`
//! registration recipe (D-10 second inert place), best-effort non-powershell
//! generation (D-09), and clap shell validation.
//!
//! Every command forces `NO_COLOR=1` so the purity assertions are deterministic
//! regardless of the runner's TTY state (mirrors `tests/uuid.rs`).

use assert_cmd::Command;

/// The registration one-liner (D-10). Must appear VERBATIM in BOTH inert places:
/// the generated script's `#`-header AND the `completions` `--help` long help.
const PS_ONELINER: &str = "box completions powershell | Out-String | Invoke-Expression";

/// Capture `box completions <args>` raw stdout/stderr bytes + exit status. Forces
/// `NO_COLOR=1` for determinism (the JSON-purity-grade byte checks must inspect raw
/// bytes, not a trimmed String).
fn completions_output(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("completions");
    cmd.args(args);
    cmd.env("NO_COLOR", "1");
    cmd.output().expect("run box completions")
}

/// D-10 purity — `box completions powershell` emits a PURE PowerShell completion
/// script: exit 0; non-empty stdout carrying the `Register-ArgumentCompleter`
/// hallmark and the bin name `box`; EMPTY stderr (no per-run chatter, so the script
/// survives redirection); NO ANSI escape (0x1B); NO UTF-8 BOM at the front (the
/// `tests/uuid.rs::json_purity` byte checks, reused for the script channel).
#[test]
fn powershell_script_is_pure() {
    let out = completions_output(&["powershell"]);
    assert!(
        out.status.success(),
        "box completions powershell should exit 0"
    );

    // 1. Non-empty script carrying the PS completion hallmark + the bin name.
    assert!(!out.stdout.is_empty(), "the completion script must be non-empty");
    let text = String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8");
    assert!(
        text.contains("Register-ArgumentCompleter"),
        "a PowerShell completion script must call Register-ArgumentCompleter"
    );
    assert!(text.contains("box"), "the script must reference the bin name `box`");

    // 2. PURITY — stderr is EMPTY (fires even under redirection — D-10).
    assert!(
        out.stderr.is_empty(),
        "no stderr chatter may accompany the completions script, got: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 3. PURITY — no ANSI escape (0x1B) anywhere (mirrors uuid json_purity step 4).
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in the completions stdout"
    );
    // 4. PURITY — no UTF-8 BOM (EF BB BF) at the front (mirrors uuid json_purity step 5).
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM may prefix the completions stdout"
    );
}

/// SC3 live-Cli reflection — the powershell script is generated from the live final
/// `Cli`, not a stale hand-written list: it references `config` AND `completions`
/// (the two Phase-11 additions) AND `weather` (a Phase-5 command) AND the global
/// `--json` and `--clip` flags. A hand-maintained script could not stay in sync;
/// `CommandFactory` reflection is what makes these all present.
#[test]
fn script_reflects_live_cli() {
    let out = completions_output(&["powershell"]);
    assert!(out.status.success(), "box completions powershell should exit 0");
    let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");

    for needle in ["config", "completions", "weather", "--json", "--clip"] {
        assert!(
            text.contains(needle),
            "the live-Cli-generated script must reference `{needle}` (SC3)"
        );
    }
}

/// D-10 (first inert place) — the generated powershell script STARTS with a
/// `#`-prefixed line and carries the registration one-liner VERBATIM inside a `#`
/// comment, so the artifact is self-documenting even after redirection to a file
/// (a `#` comment is inert in PowerShell — the script still executes).
#[test]
fn header_is_inert_comment() {
    let out = completions_output(&["powershell"]);
    assert!(out.status.success(), "box completions powershell should exit 0");
    let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");

    let first = text.lines().next().expect("script has at least one line");
    assert!(
        first.starts_with('#'),
        "the script must start with an inert `#`-comment header, got: {first:?}"
    );
    // The one-liner sits inside a `#`-comment (the header's second line is `#  <recipe>`).
    let oneliner_line = text
        .lines()
        .find(|l| l.contains(PS_ONELINER))
        .expect("the header must carry the registration one-liner");
    assert!(
        oneliner_line.trim_start().starts_with('#'),
        "the registration one-liner must live inside an inert `#` comment, got: {oneliner_line:?}"
    );
}

/// D-10 (SECOND inert place) — `box completions powershell --help` exits 0 and its
/// long help documents the registration one-liner VERBATIM, so the recipe is
/// discoverable from `--help` and not only from the generated-script `#`-header.
#[test]
fn help_documents_registration_oneliner() {
    let out = completions_output(&["powershell", "--help"]);
    assert!(
        out.status.success(),
        "box completions powershell --help should exit 0"
    );
    let text = String::from_utf8(out.stdout).expect("stdout is UTF-8");
    assert!(
        text.contains(PS_ONELINER),
        "the completions --help long help must name the registration one-liner verbatim (D-10)"
    );
}

/// D-09 / Deferred — `box completions bash` generates a non-empty script and exits
/// 0 (best-effort: the other shells are supported but only powershell is
/// charter-tested, so only its non-emptiness + exit code are asserted here).
#[test]
fn bash_generates_best_effort() {
    let out = completions_output(&["bash"]);
    assert!(out.status.success(), "box completions bash should exit 0");
    assert!(
        !out.stdout.is_empty(),
        "box completions bash should emit a non-empty script"
    );
}

/// clap `ValueEnum` validation — `box completions notashell` is a usage error
/// (exit 2): the `clap_complete::Shell` positional rejects an unknown shell value.
#[test]
fn bad_shell_is_usage_error() {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.args(["completions", "notashell"]);
    cmd.env("NO_COLOR", "1");
    cmd.assert().failure().code(2);
}
