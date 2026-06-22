//! The `cowsay` command: wrap text in the classic ASCII-art speech bubble with
//! a cow underneath (COW-01). Pure ASCII (Phase-1 "the glyph is the source of
//! truth" rule) so the output is byte-identical piped or on a TTY.
//!
//! Layout (D-11): the text is greedy word-wrapped to a **fixed 40 columns** by
//! default (NOT `terminal_width()` — a fixed width keeps pipe-vs-TTY output
//! reproducible), overridable with `--width N`. A single content line uses the
//! single-line bubble (`< text >` between `_` and `-` borders); two or more
//! lines use the multi-line bubble (`/ \` top, `| line |` middles padded to the
//! longest, `\ /` bottom). A word longer than the width is hard-broken into
//! width-sized chunks BEFORE wrapping, so no rendered line ever exceeds the
//! width (Pitfall 5). `width == 0` is clamped to 1 to avoid an infinite loop /
//! panic (T-02-09 DoS-self).
//!
//! Flow (Pattern 1 — thin orchestrator over pure helpers): `run()` acquires the
//! text via [`crate::core::input::read_input`] (arg → piped stdin → no-arg
//! interactive TTY → exit 2), wraps it, builds the bubble + cow, prints it.

use clap::Args;

use crate::commands::RunCommand;

/// The classic cow art appended under every speech bubble. The four-space lead
/// aligns the tether `\` under the bubble's right edge, matching the CONTEXT
/// single-line target byte-for-byte.
const COW: &str = r"        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||";

/// `box cowsay [TEXT]` — wrap text in an ASCII-art speech bubble (COW-01).
///
/// `TEXT` is the message; omit it to read from piped stdin. `--width` overrides
/// the fixed 40-column wrap width.
#[derive(Debug, Args)]
pub struct CowsayArgs {
    /// The text to speak; omit to read from piped stdin.
    pub text: Option<String>,

    /// Wrap width in columns (fixed default 40, NOT the terminal width — D-11).
    #[arg(long, default_value_t = 40)]
    pub width: usize,
}

impl RunCommand for CowsayArgs {
    fn run(self) -> anyhow::Result<()> {
        // arg → piped stdin → exit-2 on a no-arg interactive TTY (D-04 branch 3).
        let raw = crate::core::input::read_input(self.text)?;
        let lines = wrap(raw.trim(), self.width);
        println!("{}", bubble(&lines));
        print!("{COW}");
        println!();
        Ok(())
    }
}

/// Greedy word-wrap `text` to at most `width` columns (by char count, not byte
/// length). A word whose own char-count exceeds `width` is hard-broken into
/// `width`-sized chunks before wrapping, so no returned line exceeds `width`
/// (Pitfall 5). `width == 0` is clamped to 1 so the loop always terminates
/// (T-02-09). Always returns at least one (possibly empty) line.
fn wrap(text: &str, width: usize) -> Vec<String> {
    todo!("GREEN: implement greedy wrap + hard-break")
}

/// Build the speech bubble around `lines`. One content line → the single-line
/// form (` _…_ ` top, `< text >`, ` -…- ` bottom). Two or more → the multi-line
/// form (`/ \` top, `| line |` middles each padded to the longest line, `\ /`
/// bottom). Pure ASCII (D-11).
fn bubble(lines: &[String]) -> String {
    todo!("GREEN: implement single/multi-line bubble")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A short phrase stays on a single line within a generous width.
    #[test]
    fn wrap_short_text_is_one_line() {
        let lines = wrap("a b c", 40);
        assert_eq!(lines, vec!["a b c".to_string()]);
    }

    /// A word longer than the width is hard-broken; no returned line exceeds the
    /// width, and the chunks reconstruct the original word (Pitfall 5).
    #[test]
    fn wrap_hard_breaks_overlong_word() {
        let long = "abcdefghijklmnopqrstuvwxyz"; // 26 chars
        let lines = wrap(long, 10);
        for l in &lines {
            assert!(
                l.chars().count() <= 10,
                "line {l:?} exceeds width 10"
            );
        }
        let rejoined: String = lines.join("");
        assert_eq!(rejoined, long, "hard-break must preserve the word");
    }

    /// Greedy wrap puts as many words as fit per line and breaks when the next
    /// word would overflow.
    #[test]
    fn wrap_greedy_breaks_at_width() {
        // Each word is 3 chars; width 7 fits two words ("foo bar" = 7) then breaks.
        let lines = wrap("foo bar baz", 7);
        assert_eq!(lines, vec!["foo bar".to_string(), "baz".to_string()]);
    }

    /// `width == 0` does not panic or loop forever (clamped to 1).
    #[test]
    fn wrap_width_zero_does_not_panic() {
        let lines = wrap("hello", 0);
        for l in &lines {
            assert!(l.chars().count() <= 1, "width-0 clamps to 1: {l:?}");
        }
        assert_eq!(lines.join(""), "hello");
    }

    /// The single-line bubble uses `<`/`>` delimiters and `_`/`-` borders sized
    /// to the text + 2 padding spaces.
    #[test]
    fn bubble_single_line_form() {
        let out = bubble(&["Hello, box!".to_string()]);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "single bubble is 3 lines: {out:?}");
        assert_eq!(lines[0], " _____________", "top border: {:?}", lines[0]);
        assert_eq!(lines[1], "< Hello, box! >", "content: {:?}", lines[1]);
        assert_eq!(lines[2], " -------------", "bottom border: {:?}", lines[2]);
    }

    /// The multi-line bubble uses `/ \`, `| |`, `\ /` borders and pads every
    /// middle line to the longest line's length (equal-length content columns).
    #[test]
    fn bubble_multi_line_form() {
        let out = bubble(&["short".to_string(), "a longer line".to_string()]);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 4, "two content lines → 4 bubble lines: {out:?}");
        assert!(lines[0].starts_with('/') && lines[0].ends_with('\\'), "top: {:?}", lines[0]);
        assert!(lines[1].starts_with('|') && lines[1].ends_with('|'), "mid1: {:?}", lines[1]);
        assert!(lines[2].starts_with('|') && lines[2].ends_with('|'), "mid2: {:?}", lines[2]);
        assert!(lines[3].starts_with('\\') && lines[3].ends_with('/'), "bot: {:?}", lines[3]);
        // Both middle lines are the same total length (padded to the longest).
        assert_eq!(
            lines[1].chars().count(),
            lines[2].chars().count(),
            "middle lines must be equal length"
        );
    }
}
