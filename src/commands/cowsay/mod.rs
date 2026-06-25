//! The `cowsay` command: wrap text in the classic ASCII-art speech bubble with
//! a cow underneath (COW-01). Pure ASCII (Phase-1 "the glyph is the source of
//! truth" rule) so the output is byte-identical piped or on a TTY.
//!
//! Layout (D-11): the text is greedy word-wrapped to a **fixed 40 columns** by
//! default — deliberately NOT the detected terminal width, since a fixed value
//! keeps pipe-vs-TTY output reproducible — overridable with `--width N`. A
//! single content line uses the single-line bubble (`< text >` between `_` and
//! `-` borders); two or more lines use the multi-line bubble (`/ \` top,
//! `| line |` middles padded to the longest, `\ /` bottom). A word longer than
//! the width is hard-broken into
//! width-sized chunks BEFORE wrapping, so no rendered line ever exceeds the
//! width (Pitfall 5). `width == 0` is clamped to 1 to avoid an infinite loop /
//! panic (T-02-09 DoS-self).
//!
//! Flow (Pattern 1 — thin orchestrator over pure helpers): `run()` acquires the
//! text via [`crate::core::input::read_input`] (arg → piped stdin → no-arg
//! interactive TTY → exit 2), wraps it, builds the bubble + cow, prints it.
//!
//! Snapshot note (RESEARCH A4): trycmd normalizes a literal `\` to `/` in its
//! captured transcripts (Windows path handling), so the cow's backslashes appear
//! as forward slashes in `tests/cmd/cowsay-*.trycmd`. The TRUE byte-exact bubble
//! (with real `\`) is locked by the `bubble` unit tests below; the trycmd files
//! are the end-to-end "command runs and renders a bubble" lock.

use clap::Args;

use crate::commands::RunCommand;

/// The `box cowsay --json` document (D-01 scalar → flat object). Field name
/// (discretion, A6): `text` carries the raw spoken message — the ASCII bubble + cow
/// art is a *visual* (like qr glyphs) and is deliberately NOT serialized. Not in
/// SPINE-04 (no `--clip`), so the human bubble path keeps `println!`/`print!`.
#[derive(serde::Serialize)]
struct CowsayOutput {
    text: String,
}

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

        // Fork on `is_json_on()` FIRST (Pitfall 1): under `--json` emit the flat
        // `{text}` object carrying the raw spoken message (A6 — the bubble/cow is a
        // visual, never serialized). The bubble's leading/trailing whitespace is
        // trimmed to match the wrap input, but the message is otherwise verbatim.
        if crate::core::output::is_json_on() {
            let doc = CowsayOutput {
                text: raw.trim().to_string(),
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

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
    // Clamp width to >= 1 so the hard-break chunk loop always advances (T-02-09:
    // a 0-width would never consume a char and loop forever).
    let width = width.max(1);

    // First, hard-break any single word longer than the width into width-sized
    // chunks (Pitfall 5 — do this BEFORE greedy wrapping so an over-long word
    // never forces a line past the width). Whitespace-delimited tokens.
    let mut tokens: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let chars: Vec<char> = word.chars().collect();
        if chars.len() <= width {
            tokens.push(word.to_string());
        } else {
            for chunk in chars.chunks(width) {
                tokens.push(chunk.iter().collect());
            }
        }
    }

    // Greedy wrap: append a token to the current line if it still fits (with a
    // joining space), otherwise start a new line.
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for token in tokens {
        if current.is_empty() {
            current = token;
        } else if current.chars().count() + 1 + token.chars().count() <= width {
            current.push(' ');
            current.push_str(&token);
        } else {
            lines.push(std::mem::take(&mut current));
            current = token;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    // Always return at least one line so the bubble has content (e.g. empty text
    // → a single empty line, which renders an empty `<  >` bubble).
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Build the speech bubble around `lines`. One content line → the single-line
/// form (` _…_ ` top, `< text >`, ` -…- ` bottom). Two or more → the multi-line
/// form (`/ \` top, `| line |` middles each padded to the longest line, `\ /`
/// bottom). Pure ASCII (D-11).
fn bubble(lines: &[String]) -> String {
    // Width of the content column = the longest line's char count.
    let inner = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

    if lines.len() <= 1 {
        // Single-line form: ` _…_ ` / `< text >` / ` -…- `. Borders span the
        // text + the two padding spaces (so they sit under the `< ` and ` >`).
        let text = lines.first().map(String::as_str).unwrap_or("");
        let border: String = "_".repeat(inner + 2);
        let dashes: String = "-".repeat(inner + 2);
        format!(" {border}\n< {text} >\n {dashes}")
    } else {
        // Multi-line form: a `/ … \` top decoration, one `| line |` per content
        // line (each padded to `inner` so the right border aligns), and a
        // `\ … /` bottom decoration. The decorations span `inner + 2` to match
        // the padded middle width.
        let mut out = String::new();
        out.push('/');
        out.push_str(&" ".repeat(inner + 2));
        out.push('\\');
        out.push('\n');
        for line in lines {
            let pad = inner - line.chars().count();
            out.push_str("| ");
            out.push_str(line);
            out.push_str(&" ".repeat(pad));
            out.push_str(" |");
            out.push('\n');
        }
        out.push('\\');
        out.push_str(&" ".repeat(inner + 2));
        out.push('/');
        out
    }
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
            assert!(l.chars().count() <= 10, "line {l:?} exceeds width 10");
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
        assert_eq!(
            lines.len(),
            4,
            "two content lines → 4 bubble lines: {out:?}"
        );
        assert!(
            lines[0].starts_with('/') && lines[0].ends_with('\\'),
            "top: {:?}",
            lines[0]
        );
        assert!(
            lines[1].starts_with('|') && lines[1].ends_with('|'),
            "mid1: {:?}",
            lines[1]
        );
        assert!(
            lines[2].starts_with('|') && lines[2].ends_with('|'),
            "mid2: {:?}",
            lines[2]
        );
        assert!(
            lines[3].starts_with('\\') && lines[3].ends_with('/'),
            "bot: {:?}",
            lines[3]
        );
        // Both middle lines are the same total length (padded to the longest).
        assert_eq!(
            lines[1].chars().count(),
            lines[2].chars().count(),
            "middle lines must be equal length"
        );
    }
}
