//! The `roast` command: print one random programmer roast from an embedded list
//! (ROST-01). Identical shape to `fortune` — one of the three Phase-2 whimsy
//! "random pick" commands: `rand::rng()` (OS-seeded `ThreadRng`, ChaCha12) + the
//! unbiased [`IndexedRandom::choose`] over an embedded slice. NO fixed/manual seed
//! anywhere; each `box` run reseeds from the OS so repeated calls differ (D-08).
//! Decorative, not security — `choose` (over `% len`) keeps the draw unbiased as
//! a quality choice, not a security gate.
//!
//! Content (D-09): ~30-80 self-authored / CC0 programmer one-liners, one per line,
//! embedded with [`include_str!`] (path relative to THIS source file:
//! `src/commands/roast/mod.rs` → `../../data/roasts.txt`). Self-authored/CC0, so
//! no third-party attribution gate.
//!
//! Output: the chosen line prints as-is when it fits the terminal width; a longer
//! line is greedy soft-wrapped at word boundaries so it never runs past the edge
//! (wrapping inserts newlines only between words, so the text stays equal to the
//! source entry when whitespace-collapsed — the integration test relies on this).

use clap::Args;
use rand::seq::IndexedRandom; // brings .choose() onto slices (unbiased selection)

use crate::commands::RunCommand;

/// The `box roast --json` document (D-01 scalar → flat object, identical shape to
/// fortune). Field name (discretion): `text` carries the chosen one-liner — the
/// **UNWRAPPED** single string verbatim (wrapping is human-only — D-17). Not in
/// SPINE-04 (no `--clip`).
#[derive(serde::Serialize)]
struct RoastOutput {
    text: String,
}

/// The embedded roast list — one one-liner per line, self-authored / CC0.
/// `.gitattributes` forces `eol=lf` so no `\r` leaks in via `include_str!` on a
/// CRLF checkout (the loader also trims defensively).
const ROASTS_RAW: &str = include_str!("../../data/roasts.txt");

/// `box roast` — deliver a random programmer roast (ROST-01). No flags.
#[derive(Debug, Args)]
pub struct RoastArgs {}

impl RunCommand for RoastArgs {
    fn run(self) -> anyhow::Result<()> {
        let list = entries();
        // OS-seeded ThreadRng (rand 0.9: rand::rng()). Unbiased pick via choose —
        // never `% len`, never a fixed seed (D-08).
        let mut rng = rand::rng();
        let chosen = *list.choose(&mut rng).expect("roast list is non-empty");

        // Fork on `is_json_on()` FIRST (Pitfall 1), BEFORE the soft-wrap logic: the
        // JSON path emits the UNWRAPPED `chosen` string verbatim (wrapping is
        // human-only — D-17). The human branch keeps its soft-wrap.
        if crate::core::output::is_json_on() {
            let doc = RoastOutput {
                text: chosen.to_string(),
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

        let width = crate::core::output::terminal_width();
        if chosen.chars().count() <= width {
            crate::core::output::out_line(chosen);
        } else {
            for line in soft_wrap(chosen, width) {
                crate::core::output::out_line(&line);
            }
        }
        Ok(())
    }
}

/// Parse the embedded list into trimmed, non-empty entries (mirrors fortune).
/// Exposed `pub(crate)` for the membership/count tests.
pub(crate) fn entries() -> Vec<&'static str> {
    ROASTS_RAW
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

/// Greedy word-wrap to at most `width` columns, breaking only between words; a
/// single over-long word is left whole. Keeps the text whitespace-equal to the
/// source. `width` clamped to >= 1.
fn soft_wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.chars().count() + 1 + word.chars().count() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parsed list size is within the D-09 range [30, 80].
    #[test]
    fn entries_count_in_d09_range() {
        let n = entries().len();
        assert!(
            (30..=80).contains(&n),
            "roast entries() count {n} must be in [30, 80]"
        );
    }

    /// No parsed entry is empty.
    #[test]
    fn no_entry_is_empty() {
        for e in entries() {
            assert!(!e.is_empty(), "roast entry must not be empty");
        }
    }

    /// Soft-wrap preserves the words (whitespace-collapsed equality).
    #[test]
    fn soft_wrap_collapses_back_to_source() {
        let text = "your git history reads like a cry for help repeatedly";
        let wrapped = soft_wrap(text, 12).join(" ");
        assert_eq!(
            wrapped.split_whitespace().collect::<Vec<_>>(),
            text.split_whitespace().collect::<Vec<_>>(),
            "soft-wrap must preserve the words"
        );
    }
}
