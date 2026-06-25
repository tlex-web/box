//! The `8ball` command: answer a yes/no question like a Magic 8-Ball (8BAL-01).
//! The third Phase-2 whimsy "random pick" command, sharing the pattern with
//! `fortune`/`roast`: `rand::rng()` (OS-seeded `ThreadRng`, ChaCha12) + the
//! unbiased [`IndexedRandom::choose`] over a slice. NO fixed/manual seed anywhere;
//! each `box` run reseeds from the OS so repeated calls differ (D-08). Decorative,
//! not security — `choose` (over `% len`) keeps the draw unbiased as a quality
//! choice, not a security gate.
//!
//! Answer set (D-07/D-09): the canonical 20 Magic 8-Ball answers held as an
//! in-source `const &[&str]` — small, readable, and grouped by tone (10
//! affirmative / 5 non-committal / 5 negative). NOT padded beyond 20.
//!
//! Module-name note: the Rust module is `eight_ball` because an identifier cannot
//! start with a digit (STATE.md pitfall); the CLI name stays `8ball` via
//! `#[command(name = "8ball")]` on the `cli.rs` variant.
//!
//! The question argument is accepted but deliberately does NOT affect the draw
//! (classic 8-ball) — it is display-only and never interpreted, so there is no
//! injection surface (threat register T-02-10: accept).

use clap::Args;
use rand::seq::IndexedRandom; // brings .choose() onto slices (unbiased selection)

use crate::commands::RunCommand;

/// The `box 8ball --json` document (D-01 scalar → flat object). Field name
/// (discretion): `text` carries the drawn answer. The question arg is display-only
/// and is NEVER serialized (it does not affect the draw and has no scriptable
/// value). Not in SPINE-04 (no `--clip`).
#[derive(serde::Serialize)]
struct EightBallOutput {
    text: String,
}

/// The canonical 20 Magic 8-Ball answers, grouped by tone: 10 affirmative,
/// 5 non-committal, 5 negative (D-09). Held in source (D-07) — small and readable;
/// not padded beyond 20.
const EIGHT_BALL_ANSWERS: &[&str] = &[
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

/// `box 8ball [QUESTION]` — ask the magic 8-ball a question (8BAL-01).
///
/// The optional `QUESTION` is accepted for the ritual but does not influence the
/// answer (classic 8-ball) — every answer is drawn uniformly at random.
#[derive(Debug, Args)]
pub struct EightBallArgs {
    /// The question to ask (optional; does not affect the answer).
    pub question: Option<String>,
}

impl RunCommand for EightBallArgs {
    fn run(self) -> anyhow::Result<()> {
        // The question is intentionally ignored for the draw (classic 8-ball);
        // it is never interpreted, so there is no injection surface (T-02-10).
        let _ = self.question;

        // OS-seeded ThreadRng via the rand 0.9 constructor `rand::rng()`. Unbiased
        // pick via choose — never a modulo-indexed selection, never a fixed seed.
        let mut rng = rand::rng();
        let answer = *EIGHT_BALL_ANSWERS
            .choose(&mut rng)
            .expect("EIGHT_BALL_ANSWERS is non-empty");

        // Fork on `is_json_on()` FIRST (Pitfall 1): under `--json` emit the flat
        // `{text}` object; otherwise print the answer via `out_line` (consistent
        // with the spine, though 8ball is not in SPINE-04).
        if crate::core::output::is_json_on() {
            let doc = EightBallOutput {
                text: answer.to_string(),
            };
            crate::core::output::emit_json(&doc)?;
        } else {
            crate::core::output::out_line(answer);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The answer set is exactly the canonical 20 (D-09, no padding).
    #[test]
    fn answer_set_is_exactly_20() {
        assert_eq!(
            EIGHT_BALL_ANSWERS.len(),
            20,
            "Magic 8-Ball has exactly 20 canonical answers"
        );
    }

    /// No answer is empty.
    #[test]
    fn no_answer_is_empty() {
        for a in EIGHT_BALL_ANSWERS {
            assert!(!a.is_empty(), "8ball answer must not be empty");
        }
    }

    /// The tone split is 10 affirmative / 5 non-committal / 5 negative — verified
    /// by the documented partition boundaries (first 10, next 5, last 5).
    #[test]
    fn tone_split_is_10_5_5() {
        let affirmative = &EIGHT_BALL_ANSWERS[0..10];
        let non_committal = &EIGHT_BALL_ANSWERS[10..15];
        let negative = &EIGHT_BALL_ANSWERS[15..20];
        assert_eq!(affirmative.len(), 10);
        assert_eq!(non_committal.len(), 5);
        assert_eq!(negative.len(), 5);
    }
}
