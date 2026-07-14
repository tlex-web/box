//! The `roast` command: print one random programmer roast from an embedded list
//! (ROST-01 / ROST-V2-01). Identical shape to `fortune` — one of the three Phase-2
//! whimsy "random pick" commands: `rand::rng()` (OS-seeded `ThreadRng`, ChaCha12) +
//! the unbiased [`IndexedRandom::choose`] over an embedded slice. NO fixed/manual
//! seed anywhere; each `box` run reseeds from the OS so repeated calls differ
//! (D-08). Decorative, not security — `choose` (over `% len`) keeps the draw
//! unbiased as a quality choice, not a security gate.
//!
//! Language buckets (ROST-V2-01 / D-01): the corpus is split into per-language
//! ecosystem buckets — `general` (the default), `python`, `javascript`, `rust` —
//! each embedded from its own text asset under `src/data/roasts/`. `--language`
//! selects a PROGRAMMING ecosystem (NOT a spoken language); the roasts themselves
//! stay in English (D-01). Omitting `--language` resolves to the `general` bucket,
//! preserving today's no-flag behavior. An unknown `--language` value is a clap
//! usage error (exit 2) that lists the valid languages — no free-form string
//! reaches a filter path (T-10-02-ENUM).
//!
//! Content (D-09): self-authored / CC0 programmer one-liners, one per line, embedded
//! with [`include_str!`] (paths relative to THIS source file:
//! `src/commands/roast/mod.rs` -> `../../data/roasts/{general,python,javascript,rust}.txt`).
//! Self-authored/CC0, so no third-party attribution gate. `.gitattributes` forces
//! `eol=lf` so no `\r` leaks in on a CRLF checkout (the loader also trims
//! defensively — T-10-02-EOL). There is NO runtime file read; the buckets are
//! compile-time `include_str!` (T-10-02-IO).
//!
//! Output: the chosen line prints as-is when it fits the terminal width; a longer
//! line is greedy soft-wrapped at word boundaries so it never runs past the edge
//! (wrapping inserts newlines only between words, so the text stays equal to the
//! source entry when whitespace-collapsed — the integration test relies on this).
//! Under `--json` the document is a flat `{text, language}` object — the UNWRAPPED
//! string plus the resolved bucket name (e.g. `"general"` when the flag is omitted).

use clap::{Args, ValueEnum};
use rand::seq::IndexedRandom; // brings .choose() onto slices (unbiased selection)

use crate::commands::RunCommand;

/// The `box roast --json` document (D-01 scalar → flat object, identical shape to
/// fortune). `text` carries the chosen one-liner — the **UNWRAPPED** single string
/// verbatim (wrapping is human-only — D-17). `language` names the resolved bucket
/// (ROST-V2-01), always concrete (`"general"` when `--language` is omitted). Not in
/// SPINE-04 (no `--clip`).
#[derive(serde::Serialize)]
struct RoastOutput {
    text: String,
    language: Language,
}

/// The roast programming-language taxonomy (ROST-V2-01 / D-01). Mirrors the
/// `hash::Algo` `ValueEnum` shape: `ValueEnum` powers `--language` (and rejects any
/// value outside the fixed variant set → clap exit-2 usage error listing the valid
/// values, T-10-02-ENUM); `serde::Serialize` + `rename_all = "lowercase"` serialize
/// the JSON `language` field to the lowercase literal (`"python"`), which is also the
/// exact `ValueEnum` spelling — one table for both directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// The default bucket — general programmer roasts (the original v1 corpus).
    General,
    /// Python-ecosystem roasts (in English).
    Python,
    /// JavaScript-ecosystem roasts (in English).
    Javascript,
    /// Rust-ecosystem roasts (in English).
    Rust,
}

impl Language {
    /// The embedded corpus bytes for this bucket.
    fn raw(self) -> &'static str {
        match self {
            Language::General => GENERAL_RAW,
            Language::Python => PYTHON_RAW,
            Language::Javascript => JAVASCRIPT_RAW,
            Language::Rust => RUST_RAW,
        }
    }
}

/// The per-language embedded corpora — one one-liner per line, self-authored / CC0.
/// `.gitattributes` forces `eol=lf` so no `\r` leaks in on a CRLF checkout (the
/// loader also trims defensively).
const GENERAL_RAW: &str = include_str!("../../data/roasts/general.txt");
const PYTHON_RAW: &str = include_str!("../../data/roasts/python.txt");
const JAVASCRIPT_RAW: &str = include_str!("../../data/roasts/javascript.txt");
const RUST_RAW: &str = include_str!("../../data/roasts/rust.txt");

/// `box roast [--language L]` — deliver a random programmer roast (ROST-01 /
/// ROST-V2-01).
#[derive(Debug, Args)]
pub struct RoastArgs {
    /// Draw from this programming-language ecosystem bucket. Omit for the `general`
    /// default bucket (today's behavior). The roasts stay in English — this selects a
    /// DEV ecosystem, not a spoken language (D-01).
    #[arg(long, value_enum)]
    pub language: Option<Language>,
}

impl RunCommand for RoastArgs {
    fn run(self) -> anyhow::Result<()> {
        // Resolve the bucket FIRST: `None` -> the general default (preserves today's
        // no-flag behavior, D-01). Then filter to that one bucket and choose —
        // never `% len`, never a fixed seed (D-08).
        let language = self.language.unwrap_or(Language::General);
        let list = parse(language.raw());
        let mut rng = rand::rng();
        let chosen = *list
            .choose(&mut rng)
            .expect("roast language bucket is non-empty");

        // Fork on `is_json_on()` FIRST (Pitfall 1), BEFORE the soft-wrap logic: the
        // JSON path emits the UNWRAPPED `chosen` string plus its resolved `language`
        // (wrapping is human-only — D-17). The human branch keeps its soft-wrap.
        if crate::core::output::is_json_on() {
            let doc = RoastOutput {
                text: chosen.to_string(),
                language,
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

/// Parse one embedded bucket into trimmed, non-empty entries (mirrors fortune).
/// Takes a `'static` slice (the embedded corpora) so the borrowed entries are
/// `'static` too; trims defensively against a stray `\r` from a CRLF checkout.
fn parse(raw: &'static str) -> Vec<&'static str> {
    raw.lines()
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

    /// The `general` default bucket stays within the original D-09 range [30, 80]
    /// (it is byte-for-byte the v1 corpus — no-flag behavior preserved).
    #[test]
    fn general_bucket_count_in_d09_range() {
        let n = parse(GENERAL_RAW).len();
        assert!(
            (30..=80).contains(&n),
            "roast general bucket count {n} must be in [30, 80]"
        );
    }

    /// Every language bucket is non-trivially populated (no empty bucket ships).
    #[test]
    fn each_bucket_is_non_empty() {
        for lang in [
            Language::General,
            Language::Python,
            Language::Javascript,
            Language::Rust,
        ] {
            assert!(
                !parse(lang.raw()).is_empty(),
                "language {lang:?} bucket must be non-empty"
            );
        }
    }

    /// No parsed entry is empty (the loader drops blank lines), across every bucket.
    #[test]
    fn no_entry_is_empty() {
        for lang in [
            Language::General,
            Language::Python,
            Language::Javascript,
            Language::Rust,
        ] {
            for e in parse(lang.raw()) {
                assert!(!e.is_empty(), "roast entry must not be empty");
            }
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
