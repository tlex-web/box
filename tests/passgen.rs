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
    assert_eq!(pw.chars().count(), 16, "default length is 16 chars: {pw:?}");
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
    assert_eq!(
        pw.chars().count(),
        24,
        "--length 24 yields 24 chars: {pw:?}"
    );
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
    // The separator is implementation discretion (a dot — unambiguous because no
    // EFF word contains one; some EFF words ARE hyphenated, e.g. `t-shirt`, so a
    // hyphen separator would not split cleanly). Split on the dot.
    let words: Vec<&str> = line.split('.').filter(|w| !w.is_empty()).collect();
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
/// generated secrets go to stdout only, never logged). The PASS-V2-01 entropy
/// summary is STDERR + TTY-gated, so under the (piped) test harness it is
/// suppressed — keeping this contract intact.
#[test]
fn success_writes_nothing_to_stderr() {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("passgen").env("NO_COLOR", "1");
    cmd.assert().success().stderr(predicate::str::is_empty());
}

// --- PASS-V2-01: entropy + --no-similar + --separator --------------------------

/// `box passgen --no-similar --length 200` → not one of the look-alike chars
/// `il1Lo0O` appears (the prune drops them from the pool). A long sample makes a
/// missed leak vanishingly unlikely.
#[test]
fn no_similar_drops_lookalikes() {
    let out = passgen_stdout(&["--no-similar", "--length", "200"]);
    let pw = out.lines().next().expect("one line");
    for c in pw.chars() {
        assert!(
            !"il1Lo0O".contains(c),
            "--no-similar leaked a look-alike char {c:?}: {pw:?}"
        );
    }
}

/// `box passgen --words 4 --separator +` → the passphrase is joined with `+`
/// instead of the default `.`, splitting into exactly 4 words. `+` is not an EFF
/// word character, so the split is unambiguous.
#[test]
fn separator_overrides_passphrase_join() {
    let out = passgen_stdout(&["--words", "4", "--separator", "+"]);
    let line = out.lines().next().expect("one line");
    assert!(line.contains('+'), "custom separator must appear: {line:?}");
    assert!(!line.contains('.'), "default dot must not appear: {line:?}");
    let words: Vec<&str> = line.split('+').filter(|w| !w.is_empty()).collect();
    assert_eq!(
        words.len(),
        4,
        "--separator + must join 4 words, got {words:?} from {line:?}"
    );
}

/// PASS-V2-01 — `box passgen --json` stdout carries a top-level numeric
/// `entropy_bits`, parses as ONE JSON document, and contains no ANSI escape (the
/// entropy is a structured field, never prose on stdout — Pitfall 5).
#[test]
fn json_carries_entropy_bits() {
    let out = passgen_output(&["--json"]);
    assert!(out.status.success(), "box passgen --json should exit 0");

    // Raw-byte purity: the literal field name is present and no ANSI byte leaks.
    assert!(
        out.stdout.windows(b"entropy_bits".len()).any(|w| w == b"entropy_bits"),
        "stdout must carry the entropy_bits field"
    );
    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI in --json stdout");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    let bits = v
        .get("entropy_bits")
        .and_then(|b| b.as_f64())
        .expect("`.entropy_bits` must be a number");
    assert!(bits > 0.0, "entropy must be a positive bit count, got {bits}");
}

/// PASS-V2-01 / D-14 — the entropy estimate NEVER contaminates stdout on the
/// human (non-JSON) path: stdout is exactly the one password line, with no
/// `entropy`/`bits` prose mixed in (the summary is STDERR-only + TTY-gated).
#[test]
fn entropy_is_not_on_stdout() {
    let out = passgen_stdout(&[]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1, "human stdout is exactly the secret line: {out:?}");
    assert!(
        !out.contains("entropy") && !out.contains("bits"),
        "entropy prose must never appear on stdout: {out:?}"
    );
}

// --- Scriptable spine (SPINE-02 / SPINE-04) — copied from tests/uuid.rs ----------
//
// passgen is MULTI-CAPABLE (`--count N` lines) → the always-wrapped
// `{results:[{password}], count}` shape (EXACT uuid copy). SPINE-04: the password
// tees to the clipboard via out_line.

/// Capture `box passgen <args>` raw stdout bytes + exit status for purity/shape.
fn passgen_output(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::cargo_bin("box").unwrap();
    cmd.arg("passgen").args(args).env("NO_COLOR", "1");
    cmd.output().expect("run box passgen")
}

/// SPINE-02 / D-01 — `box passgen --json` emits the always-wrapped
/// `{results:[{password}], count}` shape even for N=1: `.count == 1`, a 1-element
/// results array, each element carries a non-empty `password` string; no ANSI,
/// no UTF-8 BOM. Adapted from `tests/uuid.rs::json_purity`.
#[test]
fn json_purity() {
    let out = passgen_output(&["--json"]);
    assert!(out.status.success(), "box passgen --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 1, "default → one result element");
    assert_eq!(v.get("count"), Some(&serde_json::json!(1)), "`.count` == 1");
    let pw = results[0]
        .get("password")
        .and_then(|p| p.as_str())
        .expect("`.results[0].password` must be a string");
    assert_eq!(pw.chars().count(), 16, "default password length is 16");

    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI in --json stdout");
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM"
    );
}

/// SPINE-02 — `box passgen --count 3 --json` → `.count == 3` and a 3-element
/// `results` array of `{password}` objects, all distinct. Copied from
/// `tests/uuid.rs::json_count_multi`.
#[test]
fn json_count_multi() {
    let out = passgen_output(&["--count", "3", "--json"]);
    assert!(out.status.success(), "box passgen --count 3 --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");
    assert_eq!(v.get("count"), Some(&serde_json::json!(3)), "`.count` == 3");
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .expect("`.results` must be an array");
    assert_eq!(results.len(), 3, "`--count 3` → 3 result elements");
    let mut seen = HashSet::new();
    for elem in results {
        let pw = elem
            .get("password")
            .and_then(|p| p.as_str())
            .expect("every element carries a password string");
        assert!(!pw.is_empty(), "password must be non-empty");
        seen.insert(pw.to_string());
    }
    assert_eq!(seen.len(), 3, "3 passwords must be distinct");
}

/// SPINE-04 / D-07 — live Windows-clipboard round-trip for `box passgen --clip`.
/// `#[ignore]`d (touches shared OS clipboard; also copies a SECRET — opt-in only).
/// Run locally with: cargo test --test passgen -- --ignored --test-threads=1
#[test]
#[ignore = "touches shared OS clipboard; run locally with --ignored --test-threads=1"]
fn clip_roundtrip() {
    let printed = {
        let out = passgen_output(&["--clip"]);
        assert!(out.status.success(), "box passgen --clip should exit 0");
        String::from_utf8(out.stdout)
            .expect("stdout is UTF-8")
            .trim()
            .to_string()
    };
    let pasted = {
        let out = Command::cargo_bin("box")
            .unwrap()
            .args(["clip", "--paste"])
            .output()
            .expect("run box clip --paste");
        assert!(out.status.success(), "box clip --paste should exit 0");
        String::from_utf8(out.stdout)
            .expect("clipboard text is UTF-8")
            .trim()
            .to_string()
    };
    assert_eq!(
        pasted, printed,
        "--clip must copy exactly the printed password to the clipboard"
    );
}
