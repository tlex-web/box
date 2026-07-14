//! The `cowsay` command: wrap text in the classic ASCII-art speech bubble with
//! a figure underneath (COW-01 / COW-V2-01). Pure ASCII (Phase-1 "the glyph is the
//! source of truth" rule) so the output is byte-identical piped or on a TTY.
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
//! # Figures & think-mode (COW-V2-01)
//! `--figure <name>` swaps the classic cow for another **built-in** figure from a
//! curated, source-embedded roster (D-02): `cow` (default), `tux`, `dragon`,
//! `ghost`, `dog`, `stegosaurus`. `--list-figures` enumerates the roster names.
//! There is deliberately **NO external `.cow` file loading** — that would add file
//! I/O + parsing and break the pure-ASCII / byte-identical rule (D-02); an unknown
//! `--figure` name is therefore a USAGE error ([`BoxError::UnknownFigure`] → exit 2)
//! that lists the available names, never a missing-file lookup.
//!
//! `--think` renders the classic *thought* bubble (`cowthink`) instead of the
//! speech bubble (D-03): rounded `( )` content delimiters and an `o`-dot tether in
//! place of the `< >` delimiters and the `\` speech tether. It reuses the single
//! [`bubble`] builder (parameterized by the `think` flag) rather than adding a
//! second renderer, and swaps the figure's tether glyph via [`render_figure`].
//!
//! Flow (Pattern 1 — thin orchestrator over pure helpers): `run()` handles the
//! `--list-figures` fast-return, resolves the figure, acquires the text via
//! [`crate::core::input::read_input`] (arg → piped stdin → no-arg interactive TTY →
//! exit 2), wraps it, builds the bubble + figure, prints it.
//!
//! Snapshot note (RESEARCH A4): trycmd normalizes a literal `\` to `/` in its
//! captured transcripts (Windows path handling), so the cow's backslashes appear
//! as forward slashes in `tests/cmd/cowsay-*.trycmd`. The TRUE byte-exact bubble
//! (with real `\`) is locked by the `bubble` unit tests below; the trycmd files
//! are the end-to-end "command runs and renders a bubble" lock. The default
//! no-flag output is byte-identical to Phase 1 (the roster's `cow` template renders
//! back to the exact classic art).

use clap::Args;

use crate::commands::RunCommand;
use crate::core::errors::BoxError;

/// The `box cowsay --json` document (D-01 scalar → flat object). Field names
/// (discretion, A6): `text` carries the raw spoken message and `figure` names the
/// selected figure (default `cow`). The ASCII bubble + figure art is a *visual*
/// (like qr glyphs) and is deliberately NOT serialized. Not in SPINE-04 (no
/// `--clip`), so the human bubble path keeps `println!`/`print!`.
#[derive(serde::Serialize)]
struct CowsayOutput {
    text: String,
    figure: String,
}

/// Placeholder character marking the **tether** glyph inside every figure template.
/// [`render_figure`] substitutes it with `\` for the speech bubble or `o` for the
/// `--think` thought bubble (D-03). Chosen because it appears in none of the figure
/// bodies, so only the intended tether positions are swapped. Substituting `\` into
/// [`COW_ART`] reproduces the Phase-1 classic cow byte-for-byte.
const TETHER: char = '@';

/// The classic cow (default figure). The four-space lead aligns the tether under
/// the bubble's right edge; substituting [`TETHER`] → `\` reproduces the Phase-1
/// `COW` const byte-for-byte (the default no-flag output is unchanged).
const COW_ART: &str = r"        @   ^__^
         @  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||";

/// Tux the penguin — a small legible alternate (pure ASCII, byte-stable).
const TUX_ART: &str = r"    @   .--.
     @  |o_o|
        |:_/|
       //   \\
      (|     |)
      /'\_ _/`\
      \___)=(_/";

/// A compact dragon — planner-discretion art, kept small and legible (D-02).
const DRAGON_ART: &str = r"      @      __
       @   _/ o\_
           \____/
           /|  |\
          (_|  |_)";

/// A little ghost.
const GHOST_ART: &str = r"     @   .---.
      @  |o o|
         | > |
         |   |
         '~~~'";

/// A small dog.
const DOG_ART: &str = r"     @   __
      @ /o o\__
        \    __>
        /  ||
       (___||";

/// A stubby stegosaurus.
const STEG_ART: &str = r"      @    /\/\/\
       @  /      \
      ____|  o   |____
     (________________)";

/// The curated built-in figure roster (D-02). `cow` is the default; the rest are
/// planner-discretion alternates. Name → pure-ASCII art template (with the
/// [`TETHER`] placeholder). This fixed table is the SOLE source of figures — there
/// is no external `.cow` file loading.
const FIGURES: &[(&str, &str)] = &[
    ("cow", COW_ART),
    ("tux", TUX_ART),
    ("dragon", DRAGON_ART),
    ("ghost", GHOST_ART),
    ("dog", DOG_ART),
    ("stegosaurus", STEG_ART),
];

/// Resolve a figure name against the fixed roster. `None` when the name matches no
/// built-in figure (the caller turns that into an exit-2 usage error listing the
/// available names — no external file lookup, D-02).
fn find_figure(name: &str) -> Option<&'static str> {
    FIGURES.iter().find(|&&(n, _)| n == name).map(|&(_, art)| art)
}

/// Enumerate the roster's figure names in declared order (`cow` first). Backs
/// `--list-figures` and the `available figures: …` text in the unknown-figure
/// error.
fn list_figures() -> Vec<&'static str> {
    FIGURES.iter().map(|&(n, _)| n).collect()
}

/// Substitute a figure template's [`TETHER`] placeholder with the concrete tether
/// glyph: `o` for the `--think` thought bubble (D-03), `\` for the speech bubble.
/// Only the tether markers change; the figure body (which may itself contain `\`)
/// is untouched. `render_figure(COW_ART, false)` equals the Phase-1 classic cow.
fn render_figure(art: &str, think: bool) -> String {
    art.replace(TETHER, if think { "o" } else { "\\" })
}

/// `box cowsay [TEXT]` — wrap text in an ASCII-art speech (or thought) bubble
/// (COW-01 / COW-V2-01).
///
/// `TEXT` is the message; omit it to read from piped stdin. `--width` overrides
/// the fixed 40-column wrap width. `--figure` selects a built-in figure,
/// `--list-figures` enumerates them, and `--think` renders a thought bubble.
#[derive(Debug, Args)]
pub struct CowsayArgs {
    /// The text to speak; omit to read from piped stdin.
    pub text: Option<String>,

    /// Wrap width in columns (fixed default 40, NOT the terminal width — D-11).
    #[arg(long, default_value_t = 40)]
    pub width: usize,

    /// Built-in figure to render (default: cow). See `--list-figures` for the set.
    #[arg(long)]
    pub figure: Option<String>,

    /// List the available built-in figures, one per line, and exit.
    #[arg(long)]
    pub list_figures: bool,

    /// Use a thought bubble (rounded `( )` delimiters + `o` tether) like `cowthink`.
    #[arg(long)]
    pub think: bool,
}

impl RunCommand for CowsayArgs {
    fn run(self) -> anyhow::Result<()> {
        // --list-figures: enumerate the roster and exit 0. No text input is needed
        // and no bubble is rendered — a fast meta-return mirroring the JSON
        // early-return shape (does not touch stdin, so it never blocks on a TTY).
        if self.list_figures {
            for name in list_figures() {
                crate::core::output::out_line(name);
            }
            return Ok(());
        }

        // Resolve the figure against the fixed embedded roster (D-02). An unknown
        // name is a USAGE error (exit 2) whose message names the available figures
        // — resolved BEFORE reading input so a bad `--figure` fails fast without
        // consuming stdin.
        let figure_name = self.figure.as_deref().unwrap_or("cow");
        let art = find_figure(figure_name).ok_or_else(|| BoxError::UnknownFigure {
            name: figure_name.to_string(),
            available: list_figures().join(", "),
        })?;

        // arg → piped stdin → exit-2 on a no-arg interactive TTY (D-04 branch 3).
        let raw = crate::core::input::read_input(self.text)?;

        // Fork on `is_json_on()` FIRST (Pitfall 1): under `--json` emit the flat
        // `{text, figure}` object (A6 — the bubble/figure is a visual, never
        // serialized). The message is trimmed to match the wrap input, otherwise
        // verbatim; `figure` names the resolved selection.
        if crate::core::output::is_json_on() {
            let doc = CowsayOutput {
                text: raw.trim().to_string(),
                figure: figure_name.to_string(),
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

        let lines = wrap(raw.trim(), self.width);
        println!("{}", bubble(&lines, self.think));
        print!("{}", render_figure(art, self.think));
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

/// Build the bubble around `lines`. One content line → the single-line form
/// (` _…_ ` top, `< text >`, ` -…- ` bottom). Two or more → the multi-line form
/// (`/ \` top, `| line |` middles each padded to the longest line, `\ /` bottom).
/// Pure ASCII (D-11).
///
/// `think` swaps the content delimiters for the thought bubble (D-03): the
/// single-line `< >` becomes `( )`, and the multi-line side `|` becomes `( )`. The
/// speech form (`think == false`) is byte-identical to the Phase-1 renderer.
fn bubble(lines: &[String], think: bool) -> String {
    // Width of the content column = the longest line's char count.
    let inner = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

    if lines.len() <= 1 {
        // Single-line form: ` _…_ ` / `< text >` (or `( text )`) / ` -…- `. Borders
        // span the text + the two padding spaces (so they sit under the delimiters).
        let text = lines.first().map(String::as_str).unwrap_or("");
        let border: String = "_".repeat(inner + 2);
        let dashes: String = "-".repeat(inner + 2);
        let (l, r) = if think { ('(', ')') } else { ('<', '>') };
        format!(" {border}\n{l} {text} {r}\n {dashes}")
    } else {
        // Multi-line form: a `/ … \` top decoration, one `| line |` (or `( line )`)
        // per content line (each padded to `inner` so the right border aligns), and
        // a `\ … /` bottom decoration. The decorations span `inner + 2` to match
        // the padded middle width.
        let (l, r) = if think { ('(', ')') } else { ('|', '|') };
        let mut out = String::new();
        out.push('/');
        out.push_str(&" ".repeat(inner + 2));
        out.push('\\');
        out.push('\n');
        for line in lines {
            let pad = inner - line.chars().count();
            out.push(l);
            out.push(' ');
            out.push_str(line);
            out.push_str(&" ".repeat(pad));
            out.push(' ');
            out.push(r);
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

    /// The single-line speech bubble uses `<`/`>` delimiters and `_`/`-` borders
    /// sized to the text + 2 padding spaces.
    #[test]
    fn bubble_single_line_form() {
        let out = bubble(&["Hello, box!".to_string()], false);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "single bubble is 3 lines: {out:?}");
        assert_eq!(lines[0], " _____________", "top border: {:?}", lines[0]);
        assert_eq!(lines[1], "< Hello, box! >", "content: {:?}", lines[1]);
        assert_eq!(lines[2], " -------------", "bottom border: {:?}", lines[2]);
    }

    /// The single-line THOUGHT bubble swaps `< >` for rounded `( )` (D-03) while
    /// keeping the same `_`/`-` borders — no `<`/`>` may appear.
    #[test]
    fn bubble_single_line_think_form() {
        let out = bubble(&["Hello, box!".to_string()], true);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "single think bubble is 3 lines: {out:?}");
        assert_eq!(lines[1], "( Hello, box! )", "think content: {:?}", lines[1]);
        assert!(
            !out.contains('<') && !out.contains('>'),
            "think bubble has no speech delimiters: {out:?}"
        );
    }

    /// The multi-line bubble uses `/ \`, `| |`, `\ /` borders and pads every
    /// middle line to the longest line's length (equal-length content columns).
    #[test]
    fn bubble_multi_line_form() {
        let out = bubble(&["short".to_string(), "a longer line".to_string()], false);
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

    /// The roster has >= 5 named figures, `cow` is present, and `find_figure`
    /// resolves a known name but not an unknown one (D-02, fixed set).
    #[test]
    fn roster_has_named_figures_and_resolves() {
        let names = list_figures();
        assert!(names.len() >= 5, "roster >= 5 figures: {names:?}");
        assert!(names.contains(&"cow"), "cow is in the roster");
        assert!(names.contains(&"dragon"), "dragon is in the roster");
        assert!(find_figure("dragon").is_some(), "known name resolves");
        assert!(find_figure("__nope__").is_none(), "unknown name does not");
    }

    /// The default `cow` template renders back to the Phase-1 classic cow
    /// byte-for-byte (speech tether = `\`), so the no-flag output is unchanged.
    #[test]
    fn cow_figure_is_byte_identical_to_classic() {
        let classic = "        \\   ^__^\n         \\  (oo)\\_______\n            (__)\\       )\\/\\\n                ||----w |\n                ||     ||";
        assert_eq!(render_figure(COW_ART, false), classic);
    }

    /// Think mode swaps a figure's tether `\` for an `o` dot (D-03) without
    /// touching the body glyphs.
    #[test]
    fn think_tether_is_o_dot() {
        let think = render_figure(COW_ART, true);
        assert!(think.contains('o'), "think tether is an o-dot: {think:?}");
        // The two leading tether positions are now `o`, not `\`.
        assert!(
            think.lines().next().unwrap().trim_start().starts_with('o'),
            "first line tether is o: {think:?}"
        );
    }
}
