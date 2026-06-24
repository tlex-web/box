//! The `matrix` command: a full-terminal green digital-rain screensaver
//! (MTRX-01). The fourth and hardest Phase-4 slice — the ONLY command that
//! mutates terminal state (raw mode, alternate screen, hidden cursor) and so the
//! only one that owns an RAII restore guard.
//!
//! # Glyphs (D-07)
//! The rain is pure **halfwidth katakana** (U+FF66..=U+FF9D). Every glyph in that
//! block is East-Asian-Width *Narrow* — a single terminal cell — so the per-column
//! drop/grid math stays clean (no width bookkeeping). There is NO charset/fallback
//! flag (that is VIS-V2-01).
//!
//! **Font note (documented limitation, not a bug — D-07 / RESEARCH Pitfall 7):**
//! halfwidth katakana need a CJK-capable font. On the bare default PowerShell 7
//! font (Cascadia Mono) the glyphs render as tofu boxes (□) because Windows
//! conhost does no glyph fallback. This is cosmetic and does NOT affect the
//! "no visual artifacts" guarantee (that is about clean teardown — D-10 — which
//! works on any font). For authentic glyphs use a CJK-capable font such as
//! Cascadia Next JP. This is surfaced in the command's `--help`.
//!
//! # Animation (D-08 / D-09)
//! Per-column drop state: a head row (started at a randomized NEGATIVE stagger so
//! columns don't all begin at the top), a randomized trail length (~6–20) and a
//! per-column speed. Each frame the heads advance; a head that has cleared the
//! bottom plus its trail is reset to a fresh negative start. The WHOLE frame is
//! rendered into one buffered `stdout` with `queue!` and flushed **exactly once**
//! per frame (`stdout.flush()`), NEVER per character — per-character flush is the
//! STATE.md hard pitfall (~5 FPS). The head cell is bright; the trail fades the
//! green channel from bright down to dark by distance behind the head; the cell
//! just past the tail is erased.
//!
//! The frame timer IS the input poll (D-09): `event::poll(Duration::from_millis(50))`
//! gates both the ~20-FPS frame cadence and the keypress read. Single-threaded —
//! no background thread, no separate sleep.
//!
//! # Exit + teardown (D-10)
//! Exits on Ctrl+C, `q`, or Esc. In raw mode crossterm delivers Ctrl+C as a
//! `KeyEvent` (`Char('c')` + `CONTROL`), NOT a SIGINT — so there is no `ctrlc`
//! crate. Only `KeyEventKind::Press` events are honored: Windows fires both Press
//! AND Release, so without that filter an exit key would double-count (Pitfall 3).
//! Restoration is an RAII [`RawGuard`] armed the INSTANT raw mode is enabled —
//! BEFORE the fallible alternate-screen/cursor `execute!`, so a failure there
//! still restores the terminal (CR-01). Its `Drop` runs `cursor::Show` +
//! `LeaveAlternateScreen` then `disable_raw_mode()` (errors ignored) on normal
//! return, on a `?` early-return, and on unwinding. Under the
//! release profile `panic = "abort"` a true panic won't unwind, so the alternate
//! screen itself is the backstop (nothing persists in the real terminal); the
//! loop is therefore kept panic-free (no `.unwrap()` on terminal ops).

use std::io::Write;
use std::time::Duration;

use clap::Args;
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{cursor::MoveTo, queue};
use owo_colors::OwoColorize;
use rand::seq::IndexedRandom;
use rand::Rng;

use crate::commands::RunCommand;

/// First halfwidth-katakana code point used by the rain (U+FF66, `ｦ`).
const KATAKANA_START: u32 = 0xFF66;
/// Last halfwidth-katakana code point used by the rain (U+FF9D, `ﾝ`).
const KATAKANA_END: u32 = 0xFF9D;

/// Minimum random trail length (cells behind the head). Discretion within D-08.
const TRAIL_MIN: i32 = 6;
/// Maximum random trail length (cells behind the head). Discretion within D-08.
const TRAIL_MAX: i32 = 20;
/// Minimum per-column fall speed (rows advanced per frame). Discretion (D-08).
const SPEED_MIN: i32 = 1;
/// Maximum per-column fall speed (rows advanced per frame). Discretion (D-08).
const SPEED_MAX: i32 = 2;

/// The bright head-cell green (D-08): near-white green so the leading glyph pops.
const HEAD_RGB: (u8, u8, u8) = (180, 255, 180);
/// The brightest trail green (just behind the head). Fades toward [`FADE_DARK`].
const FADE_BRIGHT: u8 = 255;
/// The darkest trail green (at the tail). The cell past the tail is erased.
const FADE_DARK: u8 = 40;

/// `box matrix` — a full-terminal green digital-rain screensaver (MTRX-01).
///
/// Runs an interactive, full-screen halfwidth-katakana "digital rain" animation
/// at ~20 FPS. Press Ctrl+C, q, or Esc to exit — the cursor and terminal are
/// restored cleanly with no leftover artifacts.
///
/// FONT NOTE: the rain uses halfwidth katakana (U+FF66–U+FF9D), which require a
/// CJK-capable font. On the bare default PowerShell 7 font (Cascadia Mono) the
/// glyphs show as tofu boxes (□) — this is a known cosmetic font limitation, not
/// a bug. For authentic glyphs, switch to a CJK-capable font (e.g. Cascadia Next
/// JP). There are no charset/speed/color flags in this version.
#[derive(Debug, Args)]
pub struct MatrixArgs {}

/// RAII terminal-restore guard (D-10). Constructed the INSTANT raw mode is
/// enabled — BEFORE the alternate-screen/cursor `execute!` — so it also covers a
/// failure of that fallible setup step (CR-01). Its `Drop` undoes the setup on
/// EVERY non-aborting exit path (normal return, `?` early-return, unwinding). All
/// teardown errors are deliberately ignored — there is nothing useful to do if
/// restore fails (e.g. `LeaveAlternateScreen` when the alternate screen was never
/// entered), and `drop` must not panic.
struct RawGuard;

impl Drop for RawGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, cursor::Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

impl RunCommand for MatrixArgs {
    fn run(self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        // Arm the guard THE INSTANT raw mode is on, BEFORE any further `?` that
        // could early-return (D-10 / CR-01). If `execute!(EnterAlternateScreen,
        // cursor::Hide)` below fails, its `?` returns from `run()` — but the
        // guard already exists, so its `Drop` runs `disable_raw_mode()` and the
        // terminal is restored. Were the guard armed AFTER the `execute!`, a
        // failure there would early-return with raw mode still on and no guard
        // to undo it, leaving the user's terminal stuck (no echo, no cooked
        // input). `RawGuard::drop` tolerates the alternate screen never having
        // been entered — `LeaveAlternateScreen` on a screen never entered is a
        // harmless, ignored error.
        let _guard = RawGuard;
        let mut out = std::io::stdout();
        crossterm::execute!(out, EnterAlternateScreen, cursor::Hide)?;

        let (cols, rows) = terminal::size()?;
        let cols = cols as usize;
        let rows = rows as i32;

        let mut rng = rand::rng();
        // One drop per column, each with a randomized negative-staggered start so
        // the columns don't all begin raining from the top at once.
        let mut drops: Vec<Drop_> = (0..cols)
            .map(|_| Drop_::new_random(rows, &mut rng))
            .collect();
        // The glyph currently shown at each column's head; refreshed each frame so
        // the leading character flickers/changes as it falls.
        let glyphs = katakana_glyphs();

        loop {
            // 1. Advance every column's head; reset any that cleared bottom+trail.
            for d in drops.iter_mut() {
                d.step(rows, &mut rng);
            }

            // 2. Queue the WHOLE frame into the buffered stdout (D-08). For each
            //    column draw the trail (bright→dark by distance), the bright head
            //    with a fresh random glyph, and erase the single cell past the
            //    tail. NOTHING is flushed until after this loop.
            for (x, d) in drops.iter().enumerate() {
                let x = x as u16;

                // Trail: distance 0 = head (drawn separately, bright); distance
                // `trail_len` = tail. Green fades FADE_BRIGHT→FADE_DARK by distance.
                for dist in 1..=d.trail_len {
                    let y = d.head - dist;
                    if y >= 0 && y < rows {
                        let g = fade(dist, d.trail_len);
                        let glyph = glyphs.choose(&mut rng).copied().unwrap_or('ﾝ');
                        queue!(out, MoveTo(x, y as u16), Print(glyph.truecolor(0, g, 0)))?;
                    }
                }

                // Head: the brightest cell, a fresh random katakana glyph.
                if d.head >= 0 && d.head < rows {
                    let glyph = glyphs.choose(&mut rng).copied().unwrap_or('ﾝ');
                    queue!(
                        out,
                        MoveTo(x, d.head as u16),
                        Print(glyph.truecolor(HEAD_RGB.0, HEAD_RGB.1, HEAD_RGB.2))
                    )?;
                }

                // Erase the cell just past the tail so the trail has a clean edge.
                let erase_y = d.head - d.trail_len - 1;
                if erase_y >= 0 && erase_y < rows {
                    queue!(out, MoveTo(x, erase_y as u16), Print(' '))?;
                }
            }

            // 3. Flush EXACTLY ONCE per frame (D-08 — never per character).
            out.flush()?;

            // 4. The 50ms poll IS the ~20-FPS frame timer AND the input gate (D-09).
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    // Press-only: Windows fires Press AND Release; filtering here
                    // stops the exit key double-counting (Pitfall 3 / D-10).
                    if key.kind == KeyEventKind::Press && is_quit(&key) {
                        break;
                    }
                }
            }
        }

        // `_guard` drops here → Show + LeaveAlternateScreen + disable_raw_mode.
        Ok(())
    }
}

/// One column's falling-rain state (D-08). Pure data + a pure `step`/`reset` so
/// the fall/reset logic is unit-testable WITHOUT a terminal.
///
/// Named `Drop_` to avoid colliding with the `Drop` trait used by [`RawGuard`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Drop_ {
    /// The head's current row. May be NEGATIVE while the column is still
    /// "above" the visible area (the staggered start) — that is intentional.
    head: i32,
    /// How many cells of trail follow the head (the fade length).
    trail_len: i32,
    /// Rows the head advances per frame.
    speed: i32,
}

impl Drop_ {
    /// A fresh drop with a randomized negative-staggered start, trail length, and
    /// speed. The negative start (`-rows..0`) means columns begin at different
    /// times instead of all at the top edge.
    fn new_random<R: Rng + ?Sized>(rows: i32, rng: &mut R) -> Self {
        let trail_len = rng.random_range(TRAIL_MIN..=TRAIL_MAX);
        let speed = rng.random_range(SPEED_MIN..=SPEED_MAX);
        // Start somewhere above the top edge so the heads enter staggered.
        let head = -rng.random_range(0..=rows.max(1));
        Self {
            head,
            trail_len,
            speed,
        }
    }

    /// Advance the head by `speed`; once the head has cleared the bottom plus its
    /// whole trail, reset to a fresh randomized negative start (a new column run).
    fn step<R: Rng + ?Sized>(&mut self, rows: i32, rng: &mut R) {
        self.head += self.speed;
        if self.head - self.trail_len > rows {
            *self = Self::new_random(rows, rng);
        }
    }
}

/// Interpolate the green channel for a trail cell `distance` cells behind the
/// head, given the column's `trail_len`. Distance 0 (the head) is the brightest
/// ([`FADE_BRIGHT`]); distance == `trail_len` (the tail) is the darkest
/// ([`FADE_DARK`]); values in between are linearly interpolated. Pure and
/// terminal-free → unit-testable.
fn fade(distance: i32, trail_len: i32) -> u8 {
    if trail_len <= 0 {
        return FADE_BRIGHT;
    }
    let distance = distance.clamp(0, trail_len);
    let span = (FADE_BRIGHT - FADE_DARK) as i32;
    // Linear: bright at distance 0, dark at distance == trail_len.
    let g = FADE_BRIGHT as i32 - (span * distance) / trail_len;
    g.clamp(FADE_DARK as i32, FADE_BRIGHT as i32) as u8
}

/// The pure katakana glyph table: every halfwidth katakana scalar in
/// U+FF66..=U+FF9D, in order. All are single-cell (East-Asian-Width Narrow) so
/// the grid math needs no width bookkeeping (D-07).
fn katakana_glyphs() -> Vec<char> {
    (KATAKANA_START..=KATAKANA_END)
        .filter_map(char::from_u32)
        .collect()
}

/// Quit predicate (D-10), pure so the Press-only + key-set logic is unit-tested
/// without a terminal: returns true iff the event is a key **Press** AND the
/// code is `q` or Esc, OR is `c` held with CONTROL (Ctrl+C, which raw mode
/// delivers as a KeyEvent, not a signal — Pitfall 4).
fn is_quit(key: &KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    /// A drop's head advances monotonically by `speed` each step, until a reset.
    #[test]
    fn drop_head_advances_monotonically() {
        let mut rng = rand::rng();
        let mut d = Drop_ {
            head: 0,
            trail_len: 8,
            speed: 1,
        };
        let mut prev = d.head;
        // Step a handful of times well short of the reset boundary (rows=100).
        for _ in 0..5 {
            d.step(100, &mut rng);
            assert!(d.head > prev, "head must advance: {} !> {}", d.head, prev);
            prev = d.head;
        }
    }

    /// A head that has cleared the bottom plus its trail resets to a NEGATIVE row
    /// (a fresh staggered start above the visible area).
    #[test]
    fn drop_resets_to_negative_after_clearing_bottom() {
        let mut rng = rand::rng();
        let rows = 50;
        let mut d = Drop_ {
            head: rows + 8, // already past bottom (rows) by its trail (8) - 1
            trail_len: 8,
            speed: 5,
        };
        d.step(rows, &mut rng);
        assert!(
            d.head <= 0,
            "after clearing bottom+trail the head resets to a negative start, got {}",
            d.head
        );
    }

    /// `fade` is brightest at the head (distance 0) and darkest at the tail
    /// (distance == trail_len), and decreases with distance.
    #[test]
    fn fade_bright_at_head_dark_at_tail() {
        let trail_len = 10;
        assert_eq!(fade(0, trail_len), FADE_BRIGHT, "head is brightest");
        assert_eq!(fade(trail_len, trail_len), FADE_DARK, "tail is darkest");
        // Monotonically non-increasing from head to tail.
        let mut prev = fade(0, trail_len);
        for dist in 1..=trail_len {
            let g = fade(dist, trail_len);
            assert!(
                g <= prev,
                "fade must not increase with distance: dist={dist} g={g} prev={prev}"
            );
            prev = g;
        }
    }

    /// `fade` clamps out-of-range distances and never panics on a zero trail.
    #[test]
    fn fade_is_clamped_and_safe() {
        assert_eq!(
            fade(-5, 10),
            FADE_BRIGHT,
            "negative distance clamps to head"
        );
        assert_eq!(
            fade(999, 10),
            FADE_DARK,
            "over-long distance clamps to tail"
        );
        assert_eq!(
            fade(3, 0),
            FADE_BRIGHT,
            "zero trail does not divide by zero"
        );
    }

    /// Every glyph is within the inclusive halfwidth-katakana range AND is a
    /// single display cell (width 1) — the D-07 grid-cleanliness invariant.
    #[test]
    fn katakana_table_in_range_and_single_cell() {
        use unicode_width::UnicodeWidthChar;
        let glyphs = katakana_glyphs();
        assert_eq!(
            glyphs.len(),
            (KATAKANA_END - KATAKANA_START + 1) as usize,
            "table covers the whole inclusive range"
        );
        for c in glyphs {
            let cp = c as u32;
            assert!(
                (KATAKANA_START..=KATAKANA_END).contains(&cp),
                "glyph U+{cp:04X} is outside U+FF66..=U+FF9D"
            );
            assert_eq!(
                UnicodeWidthChar::width(c),
                Some(1),
                "glyph U+{cp:04X} must be single-cell (East-Asian-Width Narrow)"
            );
        }
    }

    /// Build a `KeyEvent` for the quit-key tests.
    fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind,
            state: KeyEventState::NONE,
        }
    }

    /// The three quit keys (q, Esc, Ctrl+C) on a Press event all quit.
    #[test]
    fn quit_keys_on_press() {
        assert!(is_quit(&key(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Press
        )));
        assert!(is_quit(&key(
            KeyCode::Esc,
            KeyModifiers::NONE,
            KeyEventKind::Press
        )));
        assert!(is_quit(&key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press
        )));
    }

    /// A bare `c` without CONTROL is NOT a quit (only Ctrl+C is).
    #[test]
    fn plain_c_is_not_quit() {
        assert!(!is_quit(&key(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
            KeyEventKind::Press
        )));
    }

    /// Release/Repeat events never quit — the Windows double-fire filter (D-10).
    #[test]
    fn release_and_repeat_do_not_quit() {
        assert!(
            !is_quit(&key(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                KeyEventKind::Release
            )),
            "a Release of q must not quit (Windows double-fire filter)"
        );
        assert!(
            !is_quit(&key(KeyCode::Esc, KeyModifiers::NONE, KeyEventKind::Repeat)),
            "a Repeat of Esc must not quit"
        );
    }
}
