//! Color gating + reusable terminal-output helpers shared by every command.
//!
//! Color is decided **once** at startup ([`init_color`]) from the conjunction of
//! `--no-color`, the `NO_COLOR` env var, and whether stdout is a TTY (D-10). The
//! decision is installed as an [`owo_colors`] global override, so every
//! `.green()` / `.yellow()` call elsewhere becomes a no-op when output is piped —
//! making the plain layout **byte-identical minus ANSI** (FOUND-04, D-10).
//!
//! The flatten command (plan 03) is the first consumer of the row/summary
//! helpers below. They are kept pure/string-returning where possible so they can
//! be unit-tested without a terminal, and so the leading status glyph (`+`/`~`/`-`)
//! is always emitted as the source of truth — color is decoration only (D-09).

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

use owo_colors::OwoColorize;

/// Width used when the real terminal width is unavailable (e.g. piped output,
/// where `crossterm::terminal::size()` has no console to query).
const FALLBACK_WIDTH: usize = 80;

/// Process-global color decision, set once by [`init_color`]. We gate coloring on
/// this rather than on `owo_colors::set_override`, because the plain
/// [`OwoColorize`] methods (`.green()` etc.) are **unconditional** — the global
/// override only affects the `if_supports_color` API. Consulting our own flag is
/// what makes the plain layout byte-identical minus ANSI (D-10).
static COLOR_ON: AtomicBool = AtomicBool::new(false);

/// Whether color is currently enabled for output (the decision [`init_color`]
/// installed). Defaults to `false` until `init_color` runs.
pub fn is_color_on() -> bool {
    COLOR_ON.load(Ordering::Relaxed)
}

/// True when colored output should be emitted: the `--no-color` flag is unset,
/// the `NO_COLOR` env var is unset, **and** stdout is a real terminal (D-10).
///
/// This is the single gate (FOUND-04). Piping `box <cmd>` to a file or another
/// process makes `stdout().is_terminal()` false, so color is suppressed without
/// any per-`println!` checks.
pub fn color_enabled(no_color_flag: bool) -> bool {
    !no_color_flag && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

/// Decide color once at startup and install it as the global owo-colors override.
///
/// Call this in `main()` after a successful parse and before dispatch. With the
/// override set to `false`, every owo-colors decoration no-ops, guaranteeing the
/// plain layout is byte-identical to the colored one minus the ANSI escapes
/// (D-10).
pub fn init_color(no_color_flag: bool) {
    let on = color_enabled(no_color_flag);
    COLOR_ON.store(on, Ordering::Relaxed);
    // Also install the owo-colors global override so any future
    // `if_supports_color` call agrees with our decision. (The plain `.green()`
    // path used by [`format_row`] ignores this and consults `COLOR_ON`.)
    owo_colors::set_override(on);
}

/// The status of one flatten row — the leading glyph is the machine-readable
/// source of truth (D-09); color is decoration only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowStatus {
    /// `+` — a plain copy with no collision.
    Copy,
    /// `~` — copied under a collision-renamed name.
    Rename,
    /// `-` — skipped (symlink, reserved name, …).
    Skip,
}

impl RowStatus {
    /// The ASCII glyph for this status (`+` / `~` / `-`). ASCII, never Unicode,
    /// so it renders reliably in PowerShell 7 regardless of font (D-09).
    pub fn glyph(self) -> char {
        match self {
            RowStatus::Copy => '+',
            RowStatus::Rename => '~',
            RowStatus::Skip => '-',
        }
    }
}

/// Width of the indent + glyph + space prefix on every row (`"  + "`), counted so
/// the body can be width-capped against the terminal width.
const ROW_PREFIX_WIDTH: usize = 4;

/// The arrow token joining a source label to its destination mapping (D-09).
const ARROW: &str = "->";

/// Format a single flatten status row (D-09/D-10) — pure and testable.
///
/// Layout: `"  {glyph} {src}{pad}{-> dst}{ reason}"`. The glyph is always
/// printed (source of truth); only the glyph is color-wrapped via the global
/// owo-colors override. The `->` arrow is aligned into a column at `arrow_col`
/// (the caller passes the max source-label width across the plan so arrows line
/// up). The whole line is capped at the terminal width; an over-long source
/// label is middle-truncated with `…` so the filename stays visible (D-10).
///
/// * `status`     — the row glyph + color.
/// * `src_label`  — left-hand source path label (already source-relative).
/// * `dst`        — optional `-> dst` mapping (the copied/renamed name).
/// * `reason`     — optional trailing reason, e.g. `[collision]`, `[collision x2]`,
///   `(skipped: symlink)`, `(skipped: reserved name)`.
/// * `arrow_col`  — column (in chars, measured from the start of `src_label`) at
///   which to align the arrow across rows.
/// * `term_width` — total line width to cap at (use [`terminal_width`]).
pub fn format_row(
    status: RowStatus,
    src_label: &str,
    dst: Option<&str>,
    reason: Option<&str>,
    arrow_col: usize,
    term_width: usize,
) -> String {
    let glyph = status.glyph();
    // Color only the glyph, and only when color is enabled; the plain
    // [`OwoColorize`] methods always emit ANSI, so we gate on our own decision
    // (`is_color_on`) to keep the plain layout byte-identical minus ANSI (D-10).
    let glyph_str = if is_color_on() {
        match status {
            RowStatus::Copy => glyph.green().to_string(),
            RowStatus::Rename => glyph.yellow().to_string(),
            RowStatus::Skip => glyph.red().to_string(),
        }
    } else {
        glyph.to_string()
    };

    // Budget for the body (everything after "  {glyph} ").
    let body_budget = term_width.saturating_sub(ROW_PREFIX_WIDTH);

    // Reserve room for the fixed-width tail (arrow + dst + reason) so the source
    // label is the part that gets truncated, keeping the destination readable.
    let arrow_part = dst.map(|d| format!("{ARROW} {d}")).unwrap_or_default();
    let reason_part = reason.map(|r| format!(" {r}")).unwrap_or_default();
    // +1 for the space between an aligned source column and the arrow.
    let tail_len = if arrow_part.is_empty() {
        reason_part.chars().count()
    } else {
        1 + arrow_part.chars().count() + reason_part.chars().count()
    };

    // The column the source label is padded to (so arrows align), but never wider
    // than what the body budget allows after reserving the tail.
    let src_col_budget = body_budget.saturating_sub(tail_len).max(1);
    let target_col = arrow_col.min(src_col_budget);

    let shown_src = truncate_middle(src_label, target_col.max(1));

    let mut line = String::new();
    line.push_str("  ");
    line.push_str(&glyph_str);
    line.push(' ');
    line.push_str(&shown_src);

    if !arrow_part.is_empty() {
        // Pad the source label out to the alignment column, then the arrow.
        let shown_width = shown_src.chars().count();
        if shown_width < target_col {
            line.extend(std::iter::repeat_n(' ', target_col - shown_width));
        }
        line.push(' ');
        line.push_str(&arrow_part);
    }
    line.push_str(&reason_part);
    line
}

/// Middle-truncate `s` to at most `max` chars, inserting `…` so the head and
/// tail (the filename) stay visible (D-10). Returns `s` unchanged if it already
/// fits. For `max <= 1` returns just the ellipsis (or the single char).
pub fn truncate_middle(s: &str, max: usize) -> String {
    let len = s.chars().count();
    if len <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    // Reserve one char for the ellipsis; bias the tail (filename) to stay whole.
    let keep = max - 1;
    let head = keep / 2;
    let tail = keep - head;
    let chars: Vec<char> = s.chars().collect();
    let head_str: String = chars[..head].iter().collect();
    let tail_str: String = chars[len - tail..].iter().collect();
    format!("{head_str}…{tail_str}")
}

/// The current terminal width in columns, or [`FALLBACK_WIDTH`] (80) when the
/// width cannot be determined — e.g. when stdout is piped and there is no console
/// to query (D-10).
pub fn terminal_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(FALLBACK_WIDTH)
        .max(ROW_PREFIX_WIDTH + 1)
}

/// The two-line **dry-run** summary, locked verbatim by D-11.
///
/// ```text
/// Dry run: nothing was copied.
/// Plan: {to_copy} to copy, {renamed} renamed for collisions, {skipped} skipped.
/// ```
pub fn dry_run_summary(to_copy: usize, renamed: usize, skipped: usize) -> String {
    format!(
        "Dry run: nothing was copied.\nPlan: {to_copy} to copy, {renamed} renamed for collisions, {skipped} skipped."
    )
}

/// The one-line **real-run** summary, locked verbatim by D-11.
///
/// ```text
/// Done: copied {copied} files ({renamed} renamed for collisions), skipped {skipped}. {size} written.
/// ```
///
/// `size` is the human-facing byte string the caller formats (e.g. `1.2 MB`).
pub fn real_run_summary(copied: usize, renamed: usize, skipped: usize, size: &str) -> String {
    format!(
        "Done: copied {copied} files ({renamed} renamed for collisions), skipped {skipped}. {size} written."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes tests that mutate the process-global `COLOR_ON`, so the default
    /// parallel test runner can't interleave a `true`/`false` store between
    /// another test's store and its read.
    static COLOR_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn color_disabled_by_flag() {
        // The flag short-circuits regardless of TTY / NO_COLOR.
        assert!(!color_enabled(true));
    }

    #[test]
    fn glyphs_are_ascii_source_of_truth() {
        assert_eq!(RowStatus::Copy.glyph(), '+');
        assert_eq!(RowStatus::Rename.glyph(), '~');
        assert_eq!(RowStatus::Skip.glyph(), '-');
    }

    #[test]
    fn truncate_middle_keeps_ends() {
        assert_eq!(truncate_middle("short", 10), "short");
        let t = truncate_middle("aaaaaaaaaaaaaaaaaaaa.txt", 12);
        assert!(t.chars().count() <= 12);
        assert!(t.contains('…'));
        assert!(t.ends_with("txt"));
    }

    #[test]
    fn summaries_match_locked_wording() {
        assert_eq!(
            dry_run_summary(4, 3, 2),
            "Dry run: nothing was copied.\nPlan: 4 to copy, 3 renamed for collisions, 2 skipped."
        );
        assert_eq!(
            real_run_summary(4, 3, 2, "1.2 MB"),
            "Done: copied 4 files (3 renamed for collisions), skipped 2. 1.2 MB written."
        );
    }

    #[test]
    fn row_has_glyph_and_arrow() {
        let _g = COLOR_LOCK.lock().unwrap();
        // With color disabled the row is plain text (byte-identical minus ANSI).
        COLOR_ON.store(false, Ordering::Relaxed);
        let row = format_row(
            RowStatus::Copy,
            "src\\readme.md",
            Some("readme.md"),
            None,
            20,
            80,
        );
        assert!(row.starts_with("  + "));
        assert!(row.contains("-> readme.md"));
        assert!(!row.contains('\x1b'), "plain row must contain no ANSI: {row:?}");
    }

    #[test]
    fn row_carries_reason() {
        let _g = COLOR_LOCK.lock().unwrap();
        COLOR_ON.store(false, Ordering::Relaxed);
        let row = format_row(
            RowStatus::Skip,
            "src\\bin\\link.txt",
            None,
            Some("(skipped: symlink)"),
            20,
            80,
        );
        assert!(row.starts_with("  - "));
        assert!(row.ends_with("(skipped: symlink)"));
        assert!(!row.contains('\x1b'), "plain row must contain no ANSI: {row:?}");
    }

    #[test]
    fn row_colors_glyph_when_enabled() {
        let _g = COLOR_LOCK.lock().unwrap();
        // When color is on, the glyph carries ANSI but the plain glyph char is
        // still present (glyph is the source of truth, color is decoration).
        COLOR_ON.store(true, Ordering::Relaxed);
        let row = format_row(RowStatus::Rename, "a.txt", Some("b.txt"), None, 8, 80);
        COLOR_ON.store(false, Ordering::Relaxed); // restore for other tests
        assert!(row.contains('\x1b'), "colored row should contain ANSI: {row:?}");
        assert!(row.contains('~'), "glyph char must still be present: {row:?}");
    }
}
