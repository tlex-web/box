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

/// `box 8ball "Will it work?"` → one of the canonical 20.
#[test]
fn with_question_returns_one_of_the_20() {
    let answer = eight_ball_stdout(&["Will it work?"]);
    assert!(
        CANONICAL_20.contains(&answer.as_str()),
        "8ball answer {answer:?} is not one of the canonical 20"
    );
}

/// `box 8ball` (no question) → still one of the canonical 20.
#[test]
fn without_question_returns_one_of_the_20() {
    let answer = eight_ball_stdout(&[]);
    assert!(
        CANONICAL_20.contains(&answer.as_str()),
        "8ball (no question) answer {answer:?} is not one of the canonical 20"
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
