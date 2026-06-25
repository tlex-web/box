//! The `fortune` command: print one random aphorism from an embedded list
//! (FORT-01). One of the three Phase-2 whimsy "random pick" commands, all sharing
//! one pattern: `rand::rng()` (OS-seeded `ThreadRng`, ChaCha12) + the unbiased
//! [`IndexedRandom::choose`] over an embedded slice. There is NO fixed/manual seed
//! anywhere — each `box` run is a fresh process that reseeds from the OS, so
//! repeated calls differ as required (D-08). This is decorative, not security:
//! no CSPRNG requirement, but `choose` is still used (over `% len`) to keep the
//! distribution unbiased — a quality choice, not a security gate.
//!
//! Content (D-09): ~50-150 curated public-domain / CC0 / original aphorisms, one
//! per line, embedded at compile time from the bundled text asset (path relative
//! to THIS source file: `src/commands/fortune/mod.rs` → `../../data/fortunes.txt`).
//! No third-party attribution gate (unlike the EFF wordlist) — the content is
//! CC0/public-domain.
//!
//! Output (FORT-01 "fits the terminal", Open Question 3 — discretion): the chosen
//! line is printed as-is when it fits the current terminal width; a longer line is
//! greedy soft-wrapped at word boundaries so it never runs past the edge. Wrapping
//! only inserts newlines between words, so the printed text whitespace-collapses
//! back to the source entry (the integration test relies on this).

use clap::Args;
use rand::seq::IndexedRandom; // brings .choose() onto slices (unbiased selection)

use crate::commands::RunCommand;

/// The `box fortune --json` document (D-01 scalar → flat object). Field name
/// (discretion): `text` carries the chosen aphorism — the **UNWRAPPED** single
/// string verbatim (soft-wrapping is a human-render concern, not data — D-17).
/// Not in SPINE-04 (no `--clip`).
#[derive(serde::Serialize)]
struct FortuneOutput {
    text: String,
}

/// The embedded fortune list — one aphorism per line, public-domain / CC0 / orig.
/// The macro below embeds the working-copy bytes at compile time; `.gitattributes`
/// forces `eol=lf` so no `\r` leaks in on a CRLF checkout (the loader also trims).
const FORTUNES_RAW: &str = include_str!("../../data/fortunes.txt");

/// `box fortune` — print a random fortune or quote (FORT-01). No flags.
#[derive(Debug, Args)]
pub struct FortuneArgs {}

impl RunCommand for FortuneArgs {
    fn run(self) -> anyhow::Result<()> {
        let list = entries();
        // OS-seeded ThreadRng via the rand 0.9 constructor `rand::rng()` (the
        // pre-0.9 free-function spelling is gone). Unbiased pick via choose —
        // never a modulo-indexed selection, never a fixed seed (D-08).
        let mut rng = rand::rng();
        let chosen = *list.choose(&mut rng).expect("fortune list is non-empty");

        // Fork on `is_json_on()` FIRST (Pitfall 1), BEFORE the width/soft-wrap
        // logic: the JSON path emits the UNWRAPPED `chosen` string verbatim
        // (wrapping is human-only — D-17). The human branch keeps its soft-wrap.
        if crate::core::output::is_json_on() {
            let doc = FortuneOutput {
                text: chosen.to_string(),
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

        // FORT-01 "fits the terminal": soft-wrap at word boundaries only if the
        // chosen line is wider than the terminal; otherwise print verbatim. Routed
        // through `out_line` for spine consistency (fortune is not in SPINE-04).
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

/// Parse the embedded list into trimmed, non-empty entries. The asset is stored
/// LF, one aphorism per line; we trim defensively (tolerating any stray `\r` from
/// a CRLF checkout) and drop empty lines, so `entries()` yields only real
/// aphorisms. Exposed `pub(crate)` so the in-module tests (and, by mirroring the
/// same parse, the integration tests) can assert membership and counts.
pub(crate) fn entries() -> Vec<&'static str> {
    FORTUNES_RAW
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

/// Greedy word-wrap `text` to at most `width` columns (by char count), inserting
/// breaks only between words. A single word longer than `width` is left whole
/// (overflowing rather than mangling the aphorism); this keeps the printed text
/// whitespace-equal to the source entry. `width` is clamped to >= 1.
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

    /// The parsed list size is within the D-09 range [50, 150].
    #[test]
    fn entries_count_in_d09_range() {
        let n = entries().len();
        assert!(
            (50..=150).contains(&n),
            "fortune entries() count {n} must be in [50, 150]"
        );
    }

    /// No parsed entry is empty (the loader drops blank lines).
    #[test]
    fn no_entry_is_empty() {
        for e in entries() {
            assert!(!e.is_empty(), "fortune entry must not be empty");
        }
    }

    /// Soft-wrap never exceeds the width (except an unbreakable single long word)
    /// and preserves the text when whitespace-collapsed.
    #[test]
    fn soft_wrap_collapses_back_to_source() {
        let text = "the quick brown fox jumps over the lazy dog repeatedly";
        let wrapped = soft_wrap(text, 12).join(" ");
        assert_eq!(
            wrapped.split_whitespace().collect::<Vec<_>>(),
            text.split_whitespace().collect::<Vec<_>>(),
            "soft-wrap must preserve the words"
        );
    }
}
