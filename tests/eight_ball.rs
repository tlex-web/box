//! Integration tests for `box 8ball` (8BAL-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. `8ball` returns one of
//! the canonical 20 Magic 8-Ball answers (10 affirmative / 5 non-committal /
//! 5 negative), picked with `rand::rng()` + unbiased `IndexedRandom::choose` —
//! never `% len`, never a fixed seed (D-07/D-08). The question argument is
//! optional and is NOT used in the draw (classic 8-ball).
//!
//! Properties asserted (RESEARCH Pattern F), never a seeded value:
//! - with and without a question, output is one of the canonical 20;
//! - output varies across runs (N=10 fresh processes → ≥2 distinct;
//!   P(all 10 identical) ≈ 2e-12 for a 20-answer set).
//!
//! The CLI name is `8ball` (kept via `#[command(name = "8ball")]`) even though the
//! Rust module is `eight_ball` (identifiers cannot start with a digit).
//!
//! All runs set `NO_COLOR=1` so output is deterministic and ANSI-free.

use std::collections::HashSet;

use assert_cmd::Command;

/// The canonical 20 Magic 8-Ball answers, in the same order/wording the module
/// must use. Kept in sync with `EIGHT_BALL_ANSWERS` by the in-module unit tests;
/// this copy lets the integration test assert membership without exposing the
/// const across the crate boundary.
const CANONICAL_20: &[&str] = &[
    // 10 affirmative
    "It is certain.",
    "It is decidedly so.",
    "Without a doubt.",
    "Yes definitely.",
    "You may rely on it.",
    "As I see it, yes.",
    "Most likely.",
    "Outlook good.",
    "Yes.",
    "Signs point to yes.",
    // 5 non-committal
    "Reply hazy, try again.",
    "Ask again later.",
    "Better not tell you now.",
    "Cannot predict now.",
    "Concentrate and ask again.",
    // 5 negative
    "Don't count on it.",
    "My reply is no.",
    "My sources say no.",
    "Outlook not so good.",
    "Very doubtful.",
];

/// Run `box 8ball [question]` with NO_COLOR, assert exit 0 + empty stderr, return
/// the trimmed single-line stdout.
fn eight_ball_stdout(args: &[&str]) -> String {
    let out = Command::cargo_bin("box")
        .unwrap()
        .arg("8ball")
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run box 8ball");
    assert!(
        out.status.success(),
        "box 8ball {args:?} should exit 0, got {:?}; stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "box 8ball must write nothing to stderr; got {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("stdout is UTF-8")
        .trim()
        .to_string()
}

/// `box 8ball "Will it work?"` → the rendered output CONTAINS one of the canonical
/// 20 (the answer is now shown inside the ASCII 8-ball art, 8BAL-V2-01 / D-05, so
/// it is a substring of stdout rather than the whole trimmed buffer).
#[test]
fn with_question_returns_one_of_the_20() {
    let out = eight_ball_stdout(&["Will it work?"]);
    assert!(
        CANONICAL_20.iter().any(|a| out.contains(a)),
        "8ball output {out:?} must contain one of the canonical 20"
    );
}

/// `box 8ball` (no question) → still renders one of the canonical 20 in the art.
#[test]
fn without_question_returns_one_of_the_20() {
    let out = eight_ball_stdout(&[]);
    assert!(
        CANONICAL_20.iter().any(|a| out.contains(a)),
        "8ball (no question) output {out:?} must contain one of the canonical 20"
    );
}

/// 8BAL-V2-01 / D-05 — `box 8ball` renders a compact ASCII 8-ball (multi-line art)
/// around the drawn answer; the answer appears verbatim on its own line so it stays
/// a findable substring. Piped/NO_COLOR output is plain ASCII (no ANSI escape).
#[test]
fn renders_ascii_ball_around_answer() {
    let out = eight_ball_output(&[]);
    assert!(out.status.success(), "box 8ball should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() >= 3,
        "the ASCII 8-ball is multi-line art, got {} line(s): {stdout:?}",
        lines.len()
    );
    assert!(
        CANONICAL_20.iter().any(|a| stdout.contains(a)),
        "the drawn answer must appear in the art: {stdout:?}"
    );
    assert!(
        !stdout.as_bytes().contains(&0x1Bu8),
        "NO_COLOR output must contain no ANSI escape: {stdout:?}"
    );
}

/// N=10 fresh processes yield ≥2 distinct answers — proves the OS-seeded RNG
/// reseeds per process (no fixed seed).
#[test]
fn varies_across_runs() {
    let mut seen = HashSet::new();
    for _ in 0..10 {
        seen.insert(eight_ball_stdout(&["will it work?"]));
    }
    assert!(
        seen.len() >= 2,
        "8ball must vary across runs (got {} distinct in 10: {seen:?})",
        seen.len()
    );
}

// --- Scriptable spine (SPINE-02) — copied from tests/uuid.rs ---------------------
//
// 8ball is a SCALAR command → a flat `{text}` object. The question arg is
// display-only/ignored, so it NEVER appears in the JSON. Not in SPINE-04 (no clip).

/// Capture `box 8ball <args>` raw stdout bytes + exit status for the purity
/// assertions (which inspect raw bytes for ANSI/BOM, not a trimmed String).
fn eight_ball_output(args: &[&str]) -> std::process::Output {
    Command::cargo_bin("box")
        .unwrap()
        .arg("8ball")
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run box 8ball")
}

/// SPINE-02 / D-01 — `box 8ball <q> --json` emits EXACTLY one flat `{text}` JSON
/// object whose `text` is one of the canonical 20; the question arg never appears;
/// no ANSI, no UTF-8 BOM. Adapted from `tests/uuid.rs::json_purity`.
#[test]
fn json_purity() {
    let out = eight_ball_output(&["Will it work?", "--json"]);
    assert!(out.status.success(), "box 8ball --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .expect("`.text` must be a string");
    assert!(
        CANONICAL_20.contains(&text),
        "`.text` {text:?} must be one of the canonical 20"
    );
    assert!(
        v.get("results").is_none(),
        "8ball is scalar — no `results` wrapper"
    );
    // The question arg is display-only — it must NOT leak into the JSON anywhere.
    assert!(
        !out.stdout
            .windows(b"Will it work?".len())
            .any(|w| w == b"Will it work?"),
        "the question arg must never appear in the JSON"
    );

    assert!(!out.stdout.contains(&0x1Bu8), "no ANSI in --json stdout");
    assert_ne!(
        &out.stdout[..3.min(out.stdout.len())],
        b"\xEF\xBB\xBF",
        "no UTF-8 BOM"
    );
}

/// 8BAL-V2-01 / D-05 — `box 8ball --json` carries a `sentiment` field alongside
/// `text`, whose value is EXACTLY one of the three snake_case tones
/// (`affirmative` / `non_committal` / `negative`); the machine document stays
/// ANSI-free (sentiment color is `is_color_on()`-gated, forced off under `--json`).
#[test]
fn json_carries_sentiment_field() {
    let out = eight_ball_output(&["Will it work?", "--json"]);
    assert!(out.status.success(), "box 8ball --json should exit 0");

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be exactly one JSON value");

    let sentiment = v
        .get("sentiment")
        .and_then(|s| s.as_str())
        .expect("`.sentiment` must be a string");
    assert!(
        matches!(sentiment, "affirmative" | "non_committal" | "negative"),
        "`.sentiment` {sentiment:?} must be one of affirmative|non_committal|negative"
    );

    // The text is still the flat scalar answer; sentiment is derived from it.
    let text = v.get("text").and_then(|t| t.as_str()).expect("`.text`");
    assert!(
        CANONICAL_20.contains(&text),
        "`.text` {text:?} must be one of the canonical 20"
    );

    // PURITY — the sentiment color decoration must NEVER leak into --json stdout.
    assert!(
        !out.stdout.contains(&0x1Bu8),
        "no ANSI escape may appear in --json stdout (sentiment color is gated off)"
    );
}
