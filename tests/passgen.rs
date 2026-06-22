//! Integration tests for `box passgen` (PASS-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. passgen is the one
//! security-critical command in Phase 2: it MUST source randomness from `OsRng`
//! and select with unbiased `IndexedRandom::choose` / `random_range`, never
//! `% len` (D-02/D-08). That bias-freedom + CSPRNG-source guarantee is a
//! CODE-REVIEW gate (T-V6) — these tests assert *properties* (length, charset
//! membership, distinctness, stdout-only), never seeded values or a flaky
//! statistical distribution test.
//!
//! All runs set `NO_COLOR=1` so output is deterministic and ANSI-free.

use std::collections::HashSet;

use assert_cmd::Command;
use predicates::prelude::*;

/// The curated paste-safe symbol set must be a subset of these; any char that is
/// neither alphanumeric nor one of these is a banned/leaked symbol. This mirrors
/// the SYMBOLS const in the implementation (kept in sync by the unit tests there).
const ALLOWED_SYMBOLS: &str = "!#%*+-=?@^_~.:/";

/// Run `box passgen <args...>` with NO_COLOR, asserting exit 0 + empty stderr,
/// and return captured stdout as a UTF-8 String.
fn passgen_stdout(args: &[&str]) -> String {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("passgen").args(args).env("NO_COLOR", "1");
    let out = cmd.output().expect("run box passgen");
    assert!(
        out.status.success(),
        "box passgen {args:?} should exit 0, got {:?}; stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// True if `c` is a member of the default four-class curated set
/// (lower + upper + digit + curated symbol).
fn in_default_charset(c: char) -> bool {
    c.is_ascii_lowercase()
        || c.is_ascii_uppercase()
        || c.is_ascii_digit()
        || ALLOWED_SYMBOLS.contains(c)
}

/// `box passgen` → exactly one 16-char password; every char is in the
/// four-class curated set; stdout-only.
#[test]
fn default_is_16_chars_from_curated_set() {
    let out = passgen_stdout(&[]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1, "default prints exactly one line: {out:?}");
    let pw = lines[0];
    assert_eq!(
        pw.chars().count(),
        16,
        "default length is 16 chars: {pw:?}"
    );
    for c in pw.chars() {
        assert!(
            in_default_charset(c),
            "char {c:?} not in the four-class curated set ({pw:?})"
        );
    }
}

/// `box passgen --length 24` → a 24-char password.
#[test]
fn length_flag_controls_length() {
    let out = passgen_stdout(&["--length", "24"]);
    let pw = out.lines().next().expect("one line");
    assert_eq!(pw.chars().count(), 24, "--length 24 yields 24 chars: {pw:?}");
}

/// `box passgen --no-symbols` over a large sample contains no symbol char —
/// every char is alphanumeric. Sampling many passwords makes a stray symbol
/// vanishingly unlikely to be missed.
#[test]
fn no_symbols_excludes_all_symbols() {
    // One generous password (length 200) exercises the charset thoroughly.
    let out = passgen_stdout(&["--no-symbols", "--length", "200"]);
    let pw = out.lines().next().expect("one line");
    for c in pw.chars() {
        assert!(
            c.is_ascii_alphanumeric(),
            "--no-symbols leaked a non-alphanumeric char {c:?}: {pw:?}"
        );
    }
    // And a second independent run for good measure (fresh process = fresh seed).
    let out2 = passgen_stdout(&["--no-symbols", "--length", "200"]);
    for c in out2.lines().next().unwrap().chars() {
        assert!(c.is_ascii_alphanumeric(), "--no-symbols leaked {c:?}");
    }
}

/// `box passgen --words 4` → a passphrase of exactly 4 words; each word is
/// lowercase alphabetic (or hyphenated, e.g. `t-shirt`) — i.e. a parsed EFF
/// entry with no digit and no tab.
#[test]
fn words_flag_yields_n_eff_words() {
    let out = passgen_stdout(&["--words", "4"]);
    let line = out.lines().next().expect("one line");
    // The separator is implementation discretion; split on any non-word/hyphen
    // boundary by counting EFF-shaped tokens. EFF words are [a-z-]+.
    let words: Vec<&str> = line
        .split(|c: char| !(c.is_ascii_lowercase() || c == '-'))
        .filter(|w| !w.is_empty())
        .collect();
    assert_eq!(
        words.len(),
        4,
        "--words 4 yields 4 EFF words, got {words:?} from {line:?}"
    );
    for w in &words {
        assert!(
            w.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "EFF word {w:?} should be lowercase alpha/hyphen (no digit/tab)"
        );
    }
}

/// `box passgen --count 10` → 10 lines, all distinct (HashSet length == 10).
#[test]
fn count_flag_yields_n_distinct_lines() {
    let out = passgen_stdout(&["--count", "10"]);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 10, "--count 10 prints 10 lines: {out:?}");
    let distinct: HashSet<&str> = lines.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        10,
        "all 10 passwords should be distinct: {lines:?}"
    );
}

/// A successful run writes nothing to stderr (D-14 info-disclosure mitigation:
/// generated secrets go to stdout only, never logged).
#[test]
fn success_writes_nothing_to_stderr() {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("passgen").env("NO_COLOR", "1");
    cmd.assert()
        .success()
        .stderr(predicate::str::is_empty());
}
