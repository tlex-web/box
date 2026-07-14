//! The `fortune` command: print one random aphorism from an embedded list
//! (FORT-01 / FORT-V2-01). One of the three Phase-2 whimsy "random pick" commands,
//! all sharing one pattern: `rand::rng()` (OS-seeded `ThreadRng`, ChaCha12) + the
//! unbiased [`IndexedRandom::choose`] over an embedded slice. There is NO
//! fixed/manual seed anywhere — each `box` run is a fresh process that reseeds from
//! the OS, so repeated calls differ as required (D-08). This is decorative, not
//! security: no CSPRNG requirement, but `choose` is still used (over `% len`) to
//! keep the distribution unbiased — a quality choice, not a security gate.
//!
//! Categories (FORT-V2-01 / D-04): the corpus is split into three per-category
//! buckets — `wisdom`, `tech`, `humor` — each embedded from its own text asset
//! under `src/data/fortunes/`. `--category X` draws only from bucket X; bare
//! `box fortune` draws from the UNION of all three (today's behavior preserved);
//! `--list-categories` enumerates the taxonomy and exits without drawing. An
//! unknown `--category` value is a clap usage error (exit 2) that lists the valid
//! categories — no free-form string reaches a filter path (T-10-02-ENUM).
//!
//! Content (D-09): curated public-domain / CC0 / original aphorisms, one per line,
//! embedded at compile time (paths relative to THIS source file:
//! `src/commands/fortune/mod.rs` -> `../../data/fortunes/{wisdom,tech,humor}.txt`).
//! No third-party attribution gate — the content is CC0/public-domain/original.
//! `.gitattributes` forces `eol=lf` so no `\r` leaks in on a CRLF checkout (the
//! loader also trims defensively — T-10-02-EOL). There is NO runtime file read;
//! the buckets are compile-time `include_str!` (T-10-02-IO).
//!
//! Output (FORT-01 "fits the terminal", Open Question 3 — discretion): the chosen
//! line is printed as-is when it fits the current terminal width; a longer line is
//! greedy soft-wrapped at word boundaries so it never runs past the edge. Wrapping
//! only inserts newlines between words, so the printed text whitespace-collapses
//! back to the source entry (the integration test relies on this). Under `--json`
//! the document is a flat `{text, category}` object — the UNWRAPPED string plus the
//! concrete bucket the entry was drawn from (even on the bare union path).

use clap::{Args, ValueEnum};
use rand::seq::IndexedRandom; // brings .choose() onto slices (unbiased selection)

use crate::commands::RunCommand;

/// The `box fortune --json` document (D-01 scalar → flat object). `text` carries
/// the chosen aphorism — the **UNWRAPPED** single string verbatim (soft-wrapping is
/// a human-render concern, not data — D-17). `category` names the bucket the entry
/// was drawn from (FORT-V2-01) and is ALWAYS concrete, even on the bare
/// all-categories union path. Not in SPINE-04 (no `--clip`).
#[derive(serde::Serialize)]
struct FortuneOutput {
    text: String,
    category: Category,
}

/// The fortune category taxonomy (FORT-V2-01 / D-04). Mirrors the `hash::Algo`
/// `ValueEnum` shape: `ValueEnum` powers `--category` (and rejects any value outside
/// the fixed variant set → clap exit-2 usage error listing the valid values,
/// T-10-02-ENUM); `serde::Serialize` + `rename_all = "lowercase"` serialize the JSON
/// `category` field to the lowercase literal (`"tech"`), which is also the exact
/// `ValueEnum` spelling — one table for both directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// Proverbs and philosophical aphorisms (the original v1 corpus).
    Wisdom,
    /// Programming and computing aphorisms.
    Tech,
    /// Light, humorous one-liners.
    Humor,
}

impl Category {
    /// The canonical lowercase name — identical to the `ValueEnum` spelling and the
    /// serde serialization. Used by `--list-categories` and any human-facing name.
    fn name(self) -> &'static str {
        match self {
            Category::Wisdom => "wisdom",
            Category::Tech => "tech",
            Category::Humor => "humor",
        }
    }

    /// The embedded corpus bytes for this bucket.
    fn raw(self) -> &'static str {
        match self {
            Category::Wisdom => WISDOM_RAW,
            Category::Tech => TECH_RAW,
            Category::Humor => HUMOR_RAW,
        }
    }
}

/// Every category in canonical order — the `--list-categories` print order and the
/// union build order (so the bare-fortune pool is deterministic in composition).
const ALL_CATEGORIES: [Category; 3] = [Category::Wisdom, Category::Tech, Category::Humor];

/// The per-category embedded corpora — one aphorism per line, public-domain / CC0 /
/// original. `.gitattributes` forces `eol=lf` so no `\r` leaks in on a CRLF checkout
/// (the loader also trims defensively).
const WISDOM_RAW: &str = include_str!("../../data/fortunes/wisdom.txt");
const TECH_RAW: &str = include_str!("../../data/fortunes/tech.txt");
const HUMOR_RAW: &str = include_str!("../../data/fortunes/humor.txt");

/// `box fortune [--category C] [--list-categories]` — print a random fortune, or
/// enumerate the categories (FORT-01 / FORT-V2-01).
#[derive(Debug, Args)]
pub struct FortuneArgs {
    /// Draw only from this category. Omit to draw from the UNION of every category
    /// (the default, preserving today's all-categories behavior).
    #[arg(long, value_enum)]
    pub category: Option<Category>,

    /// List the available categories (one per line) and exit without drawing.
    #[arg(long)]
    pub list_categories: bool,
}

impl RunCommand for FortuneArgs {
    fn run(self) -> anyhow::Result<()> {
        // Fast-exit enumerator (before any draw), mirroring the JSON early-return
        // shape. Routed through `out_line` for spine consistency.
        if self.list_categories {
            for cat in ALL_CATEGORIES {
                crate::core::output::out_line(cat.name());
            }
            return Ok(());
        }

        // Build the candidate pool TAGGED with the bucket each entry came from, so
        // the JSON `category` is always concrete (even on the union path). Filter the
        // slice FIRST, then choose — never a modulo-indexed selection, never a fixed
        // seed (D-08). `--category X` narrows to that one bucket; `None` unions all.
        let candidates: Vec<(&'static str, Category)> = match self.category {
            Some(cat) => parse(cat.raw()).into_iter().map(|e| (e, cat)).collect(),
            None => union(),
        };
        let mut rng = rand::rng();
        let &(chosen, category) = candidates
            .choose(&mut rng)
            .expect("fortune category is non-empty");

        // Fork on `is_json_on()` FIRST (Pitfall 1), BEFORE the width/soft-wrap logic:
        // the JSON path emits the UNWRAPPED `chosen` string plus its concrete
        // `category` (wrapping is human-only — D-17). The human branch keeps its
        // soft-wrap and never renders the category (it is data, not decoration).
        if crate::core::output::is_json_on() {
            let doc = FortuneOutput {
                text: chosen.to_string(),
                category,
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

/// Parse one embedded bucket into trimmed, non-empty entries. The assets are stored
/// LF, one aphorism per line; we trim defensively (tolerating any stray `\r` from a
/// CRLF checkout) and drop empty lines. Takes a `'static` slice (the embedded
/// corpora) so the borrowed entries are `'static` too.
fn parse(raw: &'static str) -> Vec<&'static str> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

/// Every entry across all buckets, each tagged with its category — the bare-fortune
/// draw pool (today's all-categories behavior preserved, D-04).
fn union() -> Vec<(&'static str, Category)> {
    let mut all = Vec::new();
    for cat in ALL_CATEGORIES {
        for e in parse(cat.raw()) {
            all.push((e, cat));
        }
    }
    all
}

/// The flat union of entries (no category tags), for the in-module tests to assert
/// membership and counts across the whole corpus. Test-only: the runtime path uses
/// the tagged [`union`] directly, so this is `#[cfg(test)]` to stay dead-code clean.
#[cfg(test)]
pub(crate) fn entries() -> Vec<&'static str> {
    union().into_iter().map(|(e, _)| e).collect()
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

    /// The parsed UNION size stays within a sane range (was the D-09 [50, 150]
    /// single-file check; the split grows the corpus, so the upper bound is relaxed).
    #[test]
    fn union_count_in_range() {
        let n = entries().len();
        assert!(
            (50..=250).contains(&n),
            "fortune union count {n} must be in [50, 250]"
        );
    }

    /// Every category bucket is non-trivially populated (no empty bucket ships).
    #[test]
    fn each_bucket_is_non_empty() {
        for cat in ALL_CATEGORIES {
            assert!(
                !parse(cat.raw()).is_empty(),
                "category {cat:?} bucket must be non-empty"
            );
        }
    }

    /// No parsed entry is empty (the loader drops blank lines).
    #[test]
    fn no_entry_is_empty() {
        for e in entries() {
            assert!(!e.is_empty(), "fortune entry must not be empty");
        }
    }

    /// `name()` must equal the `ValueEnum` spelling (and thus the serde lowercase),
    /// so `--list-categories`, `--category`, and the JSON field never diverge.
    #[test]
    fn category_name_matches_value_enum() {
        for cat in ALL_CATEGORIES {
            let pv = cat.to_possible_value().expect("category has a possible value");
            assert_eq!(cat.name(), pv.get_name(), "name/value-enum spelling drift");
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
