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
use std::sync::Mutex;

use anyhow::Context;
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

// ---------------------------------------------------------------------------
// Scriptable spine (SPINE-01 / SPINE-03) — mirrors the COLOR_ON triad above.
//
// `--json` and `--clip` are global bools on `Cli`, lifted ONCE in `main()` by
// [`init_output`] into these process-global atomics (the same pattern as
// `COLOR_ON`/`init_color`). Commands consult `is_json_on()` and route their
// primary output through `emit_json` / `out_line`; `main()` calls `flush_clip`
// once after a successful dispatch. No per-command field, no `RunCommand::run`
// churn (the load-bearing spine idiom — see 06-PATTERNS).
// ---------------------------------------------------------------------------

/// Whether `--json` is active (machine-readable stdout). Set once by [`init_output`].
static JSON_ON: AtomicBool = AtomicBool::new(false);

/// Whether `--clip` is active (tee primary output to the clipboard). Set once by
/// [`init_output`].
static CLIP_ON: AtomicBool = AtomicBool::new(false);

/// Accumulates the command's primary output for [`flush_clip`] when `--clip` is on.
/// Teed by [`out_line`] (one line at a time) and [`emit_json`] (the whole document).
/// A `Mutex` because the global is shared and `set_text` runs once on the main
/// thread at the end (arboard main-thread discipline).
static CLIP_BUF: Mutex<String> = Mutex::new(String::new());

/// Whether `--json` is active. Commands check this FIRST and fork their output:
/// `emit_json(&doc)` on true, the human render via [`out_line`] otherwise (Pitfall 1).
///
/// Live as of Plan 06-02: `uuid` and `hash` are the first consumers, so the
/// forward-compat `#[allow(dead_code)]` has been removed (allow-then-remove),
/// restoring the strict dead-code gate on this primitive.
pub fn is_json_on() -> bool {
    JSON_ON.load(Ordering::Relaxed)
}

/// Lift the two global spine bools into atomics ONCE in `main()`, mirroring
/// [`init_color`].
///
/// **Ordering is load-bearing (Pitfall 7):** this MUST run AFTER [`init_color`].
/// When `--json` or `--clip` is set we force color OFF (so the clipboard / JSON
/// channel never receives ANSI escapes — D-03 / Pitfall 1) using the EXACT
/// mechanism `init_color` uses (`COLOR_ON` store + `owo_colors::set_override`).
/// Because color's TTY decision was already installed by `init_color`, running
/// last is what lets this force-off win.
pub fn init_output(json: bool, clip: bool) {
    JSON_ON.store(json, Ordering::Relaxed);
    CLIP_ON.store(clip, Ordering::Relaxed);
    if json || clip {
        COLOR_ON.store(false, Ordering::Relaxed);
        owo_colors::set_override(false);
    }
}

/// The single `--json` serializer for every command (no-drift guarantee). Writes
/// ONE pretty serde document to stdout: no BOM (`to_writer_pretty` never emits
/// one), a single trailing newline, and never any ANSI — raw serde escapes
/// control chars in string values, and `.green()` is never reached (D-03 /
/// Pitfall 1/2). Under `--clip`, also tees the whole document into `CLIP_BUF`
/// (D-08 — `box … --json --clip` copies the machine-readable doc).
///
/// Live as of Plan 06-02 (first consumed by `uuid`/`hash`): the forward-compat
/// `#[allow(dead_code)]` has been removed (allow-then-remove).
pub fn emit_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, value).context("serializing --json output")?;
    out.write_all(b"\n")?;
    if CLIP_ON.load(Ordering::Relaxed) {
        let s = serde_json::to_string_pretty(value)?;
        CLIP_BUF.lock().unwrap().push_str(&s);
    }
    Ok(())
}

/// THE primary-output print primitive for `--clip`-capable commands (replaces
/// bare `println!`). Always prints the line to stdout; when `--clip` is on it also
/// tees the line (plus a `\n`) into `CLIP_BUF` so [`flush_clip`] can copy the full
/// output later (SPINE-03 / D-07).
///
/// Live as of Plan 06-02 (first consumed by `uuid`/`hash`): the forward-compat
/// `#[allow(dead_code)]` has been removed (allow-then-remove).
pub fn out_line(s: &str) {
    println!("{s}");
    if CLIP_ON.load(Ordering::Relaxed) {
        let mut b = CLIP_BUF.lock().unwrap();
        b.push_str(s);
        b.push('\n');
    }
}

/// Tee `s` into `CLIP_BUF` (plus a trailing `\n`, matching [`out_line`]'s tee
/// shape) ONLY when `--clip` is on, **without writing to stdout** (SPINE-04 /
/// D-15).
///
/// This is the one sanctioned spine addition of Phase 7. It exists for the single
/// "print X, copy Y" case that [`out_line`] cannot express: `qr` prints the
/// rendered half-block glyph block to stdout (a visual) but must copy the *source
/// text* to the clipboard — routing the glyphs through `out_line` would copy
/// useless ▀▄ characters (Pitfall 4). `qr` therefore keeps its own `println!` for
/// the display and calls `clip_feed(&input)` for the clipboard payload. A no-op
/// when `--clip` is off (mirrors `out_line`'s tee gate). Do NOT add other
/// `core::output` primitives this phase — this is the sole exception.
pub fn clip_feed(s: &str) {
    if CLIP_ON.load(Ordering::Relaxed) {
        let mut b = CLIP_BUF.lock().unwrap();
        b.push_str(s);
        b.push('\n');
    }
}

/// Flush the accumulated `CLIP_BUF` to the Windows clipboard ONCE — called in
/// `main()` after a successful dispatch (never on a worker thread; arboard
/// main-thread discipline, Pitfall 6). A no-op when `--clip` is off, and a no-op
/// (no clipboard write, no confirmation) when the captured output is empty /
/// whitespace-only (D-08). On a real write the trailing whitespace is trimmed
/// exactly once (D-07, reusing `clip/mod.rs`'s single-shot arboard flow) and a
/// concise confirmation is printed to **stderr only**, TTY-gated on STDERR (D-08)
/// so `box uuid --clip 2>log` does not write the confirmation into the log.
pub fn flush_clip() -> anyhow::Result<()> {
    if !CLIP_ON.load(Ordering::Relaxed) {
        return Ok(());
    }
    let text = CLIP_BUF.lock().unwrap();
    if text.trim_end().is_empty() {
        return Ok(());
    }
    let mut cb = arboard::Clipboard::new().context("open clipboard")?;
    cb.set_text(text.trim_end().to_string())
        .context("write clipboard")?;
    if std::io::stderr().is_terminal() {
        eprintln!("Copied to clipboard");
    }
    Ok(())
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

/// Human-readable byte size (`1.2 MB`, `512 B`) — 1024-based with decimal-style
/// `B`/`KB`/`MB`/`GB`/`TB` labels, capping at TB.
///
/// Promoted from `flatten` (D-12) so it is shared by every size-formatting
/// consumer (flatten's real-run summary, `tree --sizes`, and du). Kept pure and
/// string-returning so it is unit-testable without a terminal.
pub fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.1} {}", UNITS[unit])
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
    fn human_size_scales() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(1024 * 1024), "1.0 MB");
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
        assert!(
            !row.contains('\x1b'),
            "plain row must contain no ANSI: {row:?}"
        );
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
        assert!(
            !row.contains('\x1b'),
            "plain row must contain no ANSI: {row:?}"
        );
    }

    #[test]
    fn row_colors_glyph_when_enabled() {
        let _g = COLOR_LOCK.lock().unwrap();
        // When color is on, the glyph carries ANSI but the plain glyph char is
        // still present (glyph is the source of truth, color is decoration).
        COLOR_ON.store(true, Ordering::Relaxed);
        let row = format_row(RowStatus::Rename, "a.txt", Some("b.txt"), None, 8, 80);
        COLOR_ON.store(false, Ordering::Relaxed); // restore for other tests
        assert!(
            row.contains('\x1b'),
            "colored row should contain ANSI: {row:?}"
        );
        assert!(
            row.contains('~'),
            "glyph char must still be present: {row:?}"
        );
    }

    // --- Scriptable spine (SPINE-01 / SPINE-03) -------------------------------
    //
    // Each test mutates a process-global atomic (`JSON_ON`/`CLIP_ON`/`COLOR_ON`)
    // and/or `CLIP_BUF`, so all take `COLOR_LOCK` to serialize against each other
    // and the color tests under the parallel runner (VALIDATION atomic-isolation
    // note), and reset every mutated atomic + buffer at the end. None of them
    // touch a live clipboard — the capture/no-op logic is fully headless-safe.

    /// Drain `CLIP_BUF` to empty so a test starts (and leaves) the buffer clean.
    fn reset_clip_buf() {
        CLIP_BUF.lock().unwrap().clear();
    }

    /// SPINE-03 / D-07 — `out_line` tees every line into `CLIP_BUF` only when
    /// `--clip` is on. Runnable via `cargo test --bin box out_line_tees`.
    #[test]
    fn out_line_tees() {
        let _g = COLOR_LOCK.lock().unwrap();

        // --clip ON: 5 lines accumulate, newline-joined, in CLIP_BUF.
        CLIP_ON.store(true, Ordering::Relaxed);
        reset_clip_buf();
        for i in 0..5 {
            out_line(&format!("line{i}"));
        }
        let captured = CLIP_BUF.lock().unwrap().clone();
        assert_eq!(captured, "line0\nline1\nline2\nline3\nline4\n");

        // --clip OFF: CLIP_BUF stays empty even though out_line still prints.
        CLIP_ON.store(false, Ordering::Relaxed);
        reset_clip_buf();
        out_line("not captured");
        assert!(
            CLIP_BUF.lock().unwrap().is_empty(),
            "CLIP_BUF must stay empty when --clip is off"
        );

        reset_clip_buf();
    }

    /// D-15 / Pitfall 4 — `clip_feed` tees `s` into `CLIP_BUF` ONLY when `--clip`
    /// is on, and NEVER writes to stdout (the "print X, copy Y" split `out_line`
    /// cannot express, used by `qr` to copy the source text while printing the
    /// glyph block). Mirrors `out_line_tees` minus the stdout write.
    /// Runnable via `cargo test --bin box clip_feed_tees_only`.
    #[test]
    fn clip_feed_tees_only() {
        let _g = COLOR_LOCK.lock().unwrap();

        // --clip ON: each fed string accumulates in CLIP_BUF with a trailing '\n'
        // (the same tee shape as out_line), with NO stdout write.
        CLIP_ON.store(true, Ordering::Relaxed);
        reset_clip_buf();
        clip_feed("https://example.com");
        let captured = CLIP_BUF.lock().unwrap().clone();
        assert_eq!(
            captured, "https://example.com\n",
            "clip_feed must tee the source text plus a trailing newline under --clip"
        );

        // --clip OFF: clip_feed is a complete no-op (CLIP_BUF stays empty).
        CLIP_ON.store(false, Ordering::Relaxed);
        reset_clip_buf();
        clip_feed("not captured");
        assert!(
            CLIP_BUF.lock().unwrap().is_empty(),
            "CLIP_BUF must stay empty when --clip is off"
        );

        reset_clip_buf();
    }

    /// D-08 — with `--clip` on but the buffer empty/whitespace-only, `flush_clip`
    /// returns Ok and performs NO clipboard op (the empty guard returns before
    /// `arboard::Clipboard::new()`, which is what makes this headless-CI-safe).
    /// Runnable via `cargo test --bin box flush_clip_empty_noop`.
    #[test]
    fn flush_clip_empty_noop() {
        let _g = COLOR_LOCK.lock().unwrap();
        CLIP_ON.store(true, Ordering::Relaxed);

        // Empty buffer → Ok, no arboard call.
        reset_clip_buf();
        assert!(
            flush_clip().is_ok(),
            "empty buffer must flush as a no-op Ok"
        );

        // Whitespace-only buffer → also a no-op Ok (trim_end().is_empty()).
        CLIP_BUF.lock().unwrap().push_str("   \n\t  ");
        assert!(
            flush_clip().is_ok(),
            "whitespace-only buffer must flush as a no-op Ok"
        );

        CLIP_ON.store(false, Ordering::Relaxed);
        reset_clip_buf();
    }

    /// SPINE-01 / Pitfall 1+2 — `emit_json`'s serde output carries no UTF-8 BOM
    /// and no ANSI escape. `emit_json` writes to the real stdout, so this mirrors
    /// its serde call into a Vec and asserts on the bytes.
    #[test]
    fn emit_json_no_bom_no_ansi() {
        #[derive(serde::Serialize)]
        struct Probe {
            results: Vec<&'static str>,
            count: usize,
        }
        let doc = Probe {
            results: vec!["a", "b"],
            count: 2,
        };
        // Same serde call emit_json uses (to_writer_pretty), captured to bytes.
        let mut bytes = Vec::new();
        serde_json::to_writer_pretty(&mut bytes, &doc).unwrap();
        bytes.push(b'\n');

        // No UTF-8 BOM (EF BB BF) at the front.
        assert_ne!(
            &bytes[..3.min(bytes.len())],
            b"\xEF\xBB\xBF",
            "emit_json output must have no UTF-8 BOM"
        );
        // No ANSI escape (0x1B) anywhere.
        assert!(
            !bytes.contains(&0x1Bu8),
            "emit_json output must contain no ANSI escape"
        );
        // Sanity: it is one parseable JSON document with the expected shape.
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v.get("count"), Some(&serde_json::json!(2)));
    }

    /// Pitfall 7 — `init_output` forces `COLOR_ON=false` under `--json` (and
    /// `--clip`), even if color was on. Verified via `is_color_on()` after the call.
    #[test]
    fn init_output_forces_color_off() {
        let _g = COLOR_LOCK.lock().unwrap();
        // Start with color ON, as if init_color decided a TTY was present.
        COLOR_ON.store(true, Ordering::Relaxed);
        owo_colors::set_override(true);

        // --json forces it off (mirrors --clip).
        init_output(true, false);
        assert!(
            !is_color_on(),
            "init_output must force COLOR_ON=false under --json"
        );

        // Reset every mutated atomic for other tests.
        JSON_ON.store(false, Ordering::Relaxed);
        CLIP_ON.store(false, Ordering::Relaxed);
        COLOR_ON.store(false, Ordering::Relaxed);
        owo_colors::set_override(false);
    }
}
