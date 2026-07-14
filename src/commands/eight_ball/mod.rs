//! The `8ball` command: answer a yes/no question like a Magic 8-Ball (8BAL-01 /
//! 8BAL-V2-01). The third Phase-2 whimsy "random pick" command, sharing the pattern
//! with `fortune`/`roast`: `rand::rng()` (OS-seeded `ThreadRng`, ChaCha12) + the
//! unbiased [`IndexedRandom::choose`]. NO fixed/manual seed anywhere; each `box` run
//! reseeds from the OS so repeated calls differ (D-08). Decorative, not security —
//! `choose` (over `% len`) keeps the draw unbiased as a quality choice, not a
//! security gate.
//!
//! Answer set (D-07/D-09): the canonical 20 Magic 8-Ball answers held as an
//! in-source `const &[&str]` — small, readable, and grouped by tone (10
//! affirmative / 5 non-committal / 5 negative). NOT padded beyond 20.
//!
//! # ASCII art + sentiment (8BAL-V2-01)
//! The human render draws a compact, pure-ASCII 8-ball (byte-stable) with the drawn
//! answer shown on its own line. The answer's INDEX into [`EIGHT_BALL_ANSWERS`] maps
//! onto the existing 10/5/5 tone partition — a pure [`sentiment`] lookup, no new
//! data (D-05): `[0..10]` → affirmative (green), `[10..15]` → non-committal
//! (yellow), `[15..20]` → negative (red). The color is emitted ONLY inside an
//! `is_color_on()` arm (the frozen glyph-is-source-of-truth / color-is-decoration
//! gate — `output::format_row`), so piped/`--json` output is byte-identical minus
//! ANSI. `--json` extends the flat document with a derivable `sentiment` field
//! (snake_case); the art itself is a visual and is never serialized (A6).
//!
//! Module-name note: the Rust module is `eight_ball` because an identifier cannot
//! start with a digit (STATE.md pitfall); the CLI name stays `8ball` via
//! `#[command(name = "8ball")]` on the `cli.rs` variant.
//!
//! The question argument is accepted but deliberately does NOT affect the draw
//! (classic 8-ball) — it is display-only and never interpreted, so there is no
//! injection surface (threat register T-02-10 / T-10-01-Q: accept).

use clap::Args;
use owo_colors::OwoColorize;
use rand::seq::IndexedRandom; // brings .choose() onto slices (unbiased selection)

use crate::commands::RunCommand;

/// The `box 8ball --json` document (D-01 scalar → flat object). `text` carries the
/// drawn answer; `sentiment` is the tone derived from the answer's index
/// (`affirmative`/`non_committal`/`negative`, frozen snake_case). The question arg
/// is display-only and is NEVER serialized (it does not affect the draw and has no
/// scriptable value). The ASCII art is a visual and is never serialized either
/// (A6). Not in SPINE-04 (no `--clip`).
#[derive(serde::Serialize)]
struct EightBallOutput {
    text: String,
    sentiment: Sentiment,
}

/// The tone of a drawn answer, mapped from its index onto the fixed 10/5/5
/// partition (D-05/D-09). Serialized in frozen snake_case for the `--json`
/// `sentiment` field: `affirmative` / `non_committal` / `negative`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Sentiment {
    /// `EIGHT_BALL_ANSWERS[0..10]` — rendered green.
    Affirmative,
    /// `EIGHT_BALL_ANSWERS[10..15]` — rendered yellow.
    NonCommittal,
    /// `EIGHT_BALL_ANSWERS[15..20]` — rendered red.
    Negative,
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

/// The compact ASCII 8-ball drawn above the answer (pure ASCII, byte-stable — the
/// answer is printed on its own line below it so it stays a findable substring and
/// the icon itself never changes).
const BALL_ART: &str = r"     _____
   .`     `.
  /   ___   \
 |   / 8 \   |
 |   \___/   |
  \         /
   `._____.`";

/// Map a drawn answer's INDEX onto its sentiment via the documented 10/5/5 tone
/// partition (`EIGHT_BALL_ANSWERS[0..10]` / `[10..15]` / `[15..20]`, D-05). A pure
/// lookup over the existing const — no new data. The boundaries are the same ones
/// the `tone_split_is_10_5_5` unit test pins.
fn sentiment(index: usize) -> Sentiment {
    match index {
        0..10 => Sentiment::Affirmative,
        10..15 => Sentiment::NonCommittal,
        _ => Sentiment::Negative,
    }
}

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

        // OS-seeded ThreadRng via the rand 0.9 constructor `rand::rng()`. Draw the
        // answer with the unbiased `choose` (never a modulo-indexed selection, never
        // a fixed seed), then recover its INDEX so the sentiment is a pure partition
        // lookup (answers are all distinct, so `position` is exact — D-05).
        let mut rng = rand::rng();
        let answer = *EIGHT_BALL_ANSWERS
            .choose(&mut rng)
            .expect("EIGHT_BALL_ANSWERS is non-empty");
        let index = EIGHT_BALL_ANSWERS
            .iter()
            .position(|&a| a == answer)
            .expect("the drawn answer is a member of EIGHT_BALL_ANSWERS");
        let mood = sentiment(index);

        // Fork on `is_json_on()` FIRST (Pitfall 1): under `--json` emit the flat
        // `{text, sentiment}` object (the ASCII art is a visual, never serialized —
        // A6). `init_output` forces color off under `--json`, so the document is
        // ANSI-free without any extra guard here.
        if crate::core::output::is_json_on() {
            let doc = EightBallOutput {
                text: answer.to_string(),
                sentiment: mood,
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

        // Human render: the ASCII 8-ball then the drawn answer. The answer is tinted
        // by sentiment ONLY inside an `is_color_on()` arm (copying the
        // glyph-is-source-of-truth / color-is-decoration shape from
        // `output::format_row`), so piped/NO_COLOR output is byte-identical minus
        // ANSI. Routed through `out_line` (the spine primitive), one line at a time.
        crate::core::output::out_line(BALL_ART);
        let answer_line = if crate::core::output::is_color_on() {
            match mood {
                Sentiment::Affirmative => format!("   {}", answer.green()),
                Sentiment::NonCommittal => format!("   {}", answer.yellow()),
                Sentiment::Negative => format!("   {}", answer.red()),
            }
        } else {
            format!("   {answer}")
        };
        crate::core::output::out_line(&answer_line);
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

    /// The pure `sentiment(index)` mapper honors the 10/5/5 partition: every index
    /// in `[0..10]` → affirmative, `[10..15]` → non-committal, `[15..20]` → negative
    /// (D-05, a free lookup with no new data).
    #[test]
    fn sentiment_maps_partition_bands() {
        for i in 0..10 {
            assert_eq!(sentiment(i), Sentiment::Affirmative, "index {i} is affirmative");
        }
        for i in 10..15 {
            assert_eq!(sentiment(i), Sentiment::NonCommittal, "index {i} is non-committal");
        }
        for i in 15..20 {
            assert_eq!(sentiment(i), Sentiment::Negative, "index {i} is negative");
        }
    }

    /// Every answer's index maps to the tone its position implies, and the JSON
    /// wire form is the frozen snake_case string (`non_committal`, not `NonCommittal`).
    #[test]
    fn sentiment_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&Sentiment::NonCommittal).unwrap(),
            "\"non_committal\""
        );
        assert_eq!(
            serde_json::to_string(&Sentiment::Affirmative).unwrap(),
            "\"affirmative\""
        );
        assert_eq!(
            serde_json::to_string(&Sentiment::Negative).unwrap(),
            "\"negative\""
        );
    }
}
