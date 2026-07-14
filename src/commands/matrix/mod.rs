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
//!
//! # Spine omission (SC4)
//! `matrix` is a DISPLAY-ONLY command: it INTENTIONALLY does not honor the global
//! `--json`/`--clip` flags (roadmap SC4). The flags still PARSE (they are global on
//! `Cli`), but `run()` never calls `is_json_on()` / `emit_json` — so `box matrix
//! --json` runs the normal animation and emits NO JSON document to stdout. A
//! machine-JSON envelope for a screensaver would be meaningless; omitting it (vs.
//! emitting a malformed/empty doc) is the contract. Asserted by
//! `tests/cli.rs::display_only_omit_json`.

use std::io::Write;
use std::time::Duration;

use clap::{Args, ValueEnum};
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
use crate::core::output::is_color_on;

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

/// The value of an ACTIVE head channel (D-08 / MTRX-V2-01): full brightness so the
/// leading glyph pops. Inactive channels get [`HEAD_OFF`], giving the near-white
/// tint (the green preset's head is `(180, 255, 180)`, preserving the v1 look).
const HEAD_ON: u8 = 255;
/// The value of an INACTIVE head channel — the near-white tint on the head cell.
const HEAD_OFF: u8 = 180;
/// The brightest trail level (just behind the head). Fades toward [`FADE_DARK`].
const FADE_BRIGHT: u8 = 255;
/// The darkest trail level (at the tail). The cell past the tail is erased.
const FADE_DARK: u8 = 40;

/// `box matrix` — a full-terminal green digital-rain screensaver (MTRX-01).
///
/// Runs an interactive, full-screen halfwidth-katakana "digital rain" animation
/// at ~20 FPS. Press Ctrl+C, q, or Esc to exit — the cursor and terminal are
/// restored cleanly with no leftover artifacts.
///
/// FONT NOTE: the rain uses halfwidth katakana (U+FF66–U+FF9D) by default, which
/// require a CJK-capable font. On the bare default PowerShell 7 font (Cascadia
/// Mono) the glyphs show as tofu boxes (□) — this is a known cosmetic font
/// limitation, not a bug. For authentic glyphs, switch to a CJK-capable font (e.g.
/// Cascadia Next JP), or pass `--charset ascii`/`binary`/`digits` for a font-safe
/// glyph set. `--color` and `--speed` select display presets (MTRX-V2-01).
#[derive(Debug, Args)]
pub struct MatrixArgs {
    /// Rain color preset (MTRX-V2-01). Maps to a head/trail RGB; color is only
    /// emitted when color is enabled (a piped/`NO_COLOR` run stays plain).
    #[arg(long, value_enum, default_value_t = MatrixColor::Green)]
    pub color: MatrixColor,

    /// Fall-speed preset (MTRX-V2-01): the frame/poll interval and the per-column
    /// speed range. `normal` reproduces the v1 cadence.
    #[arg(long, value_enum, default_value_t = Speed::Normal)]
    pub speed: Speed,

    /// Glyph set (MTRX-V2-01): a preset name (`katakana` [default] / `ascii` /
    /// `binary` / `digits`) OR any literal string, whose characters become the
    /// rain glyphs.
    #[arg(long, default_value = "katakana")]
    pub charset: String,
}

/// The `--color` rain presets (MTRX-V2-01). Each maps — via [`head_rgb`] /
/// [`trail_rgb`] — to a head/trail RGB along one color axis; NOT arbitrary hex
/// (arbitrary-hex matrix color is VIS-V3, deferred). `Green` reproduces the v1
/// look. Modeled on the `hash::Algo` `pub ValueEnum` style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MatrixColor {
    /// Classic green rain (the v1 default).
    Green,
    /// Red rain.
    Red,
    /// Blue rain.
    Blue,
    /// Cyan rain.
    Cyan,
    /// Magenta rain.
    Magenta,
    /// Yellow rain.
    Yellow,
    /// White rain.
    White,
}

/// The `--speed` presets (MTRX-V2-01) → the `event::poll` frame interval (ms) plus
/// the per-column `speed` range, resolved by [`speed_params`]. `Normal` reproduces
/// the v1 cadence (50 ms poll, `SPEED_MIN..=SPEED_MAX`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Speed {
    /// Slower cadence: a longer frame interval and a single-step fall.
    Slow,
    /// The v1 cadence (default).
    Normal,
    /// Faster cadence: a shorter frame interval and a quicker fall.
    Fast,
}

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
        // Resolve the display presets ONCE up front (pure, terminal-free). `color`
        // is Copy; `charset` is consumed into the glyph table below.
        let MatrixArgs {
            color,
            speed,
            charset,
        } = self;
        let (poll_ms, speed_min, speed_max) = speed_params(speed);

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
        // the columns don't all begin raining from the top at once. The per-column
        // fall speed is drawn from the `--speed` preset's range.
        let mut drops: Vec<Drop_> = (0..cols)
            .map(|_| Drop_::new_random(rows, speed_min, speed_max, &mut rng))
            .collect();
        // The glyph set for the rain: the `--charset` preset table or the literal
        // custom string's characters (pure resolution, no terminal needed).
        let glyphs = resolve_charset(&charset);

        loop {
            // 1. Advance every column's head; reset any that cleared bottom+trail.
            for d in drops.iter_mut() {
                d.step(rows, speed_min, speed_max, &mut rng);
            }

            // 2. Queue the WHOLE frame into the buffered stdout (D-08). For each
            //    column draw the trail (bright→dark by distance), the bright head
            //    with a fresh random glyph, and erase the single cell past the
            //    tail. NOTHING is flushed until after this loop.
            for (x, d) in drops.iter().enumerate() {
                let x = x as u16;

                // Trail: distance 0 = head (drawn separately, bright); distance
                // `trail_len` = tail. The trail level fades FADE_BRIGHT→FADE_DARK by
                // distance; `--color` maps that level onto the preset's color axis.
                for dist in 1..=d.trail_len {
                    let y = d.head - dist;
                    if y >= 0 && y < rows {
                        let level = fade(dist, d.trail_len);
                        let glyph = glyphs.choose(&mut rng).copied().unwrap_or('ﾝ');
                        // is_color_on() is the SOLE color gate (SC4): a piped /
                        // NO_COLOR / redirected run emits the plain glyph, so its
                        // bytes carry no ANSI escape.
                        if is_color_on() {
                            let (r, g, b) = trail_rgb(color, level);
                            queue!(out, MoveTo(x, y as u16), Print(glyph.truecolor(r, g, b)))?;
                        } else {
                            queue!(out, MoveTo(x, y as u16), Print(glyph))?;
                        }
                    }
                }

                // Head: the brightest cell, a fresh random glyph.
                if d.head >= 0 && d.head < rows {
                    let glyph = glyphs.choose(&mut rng).copied().unwrap_or('ﾝ');
                    if is_color_on() {
                        let (r, g, b) = head_rgb(color);
                        queue!(out, MoveTo(x, d.head as u16), Print(glyph.truecolor(r, g, b)))?;
                    } else {
                        queue!(out, MoveTo(x, d.head as u16), Print(glyph))?;
                    }
                }

                // Erase the WHOLE band the tail swept this frame so the trail has
                // a clean edge (WR-03). The head advanced by `speed` (up to
                // SPEED_MAX = 2) since the last frame, so the tail moved down
                // `speed` rows too; erasing only the single cell at
                // `head - trail_len - 1` would leave the `speed - 1` rows above it
                // (former trail glyphs now past the tail) un-erased, smearing a
                // faint trail of stale glyphs on fast columns. `erase_band`
                // returns every vacated row that falls on-screen.
                for erase_y in erase_band(d.head, d.trail_len, d.speed, rows) {
                    queue!(out, MoveTo(x, erase_y as u16), Print(' '))?;
                }
            }

            // 3. Flush EXACTLY ONCE per frame (D-08 — never per character).
            out.flush()?;

            // 4. The poll IS the frame timer AND the input gate (D-09); the
            //    interval is the `--speed` preset's cadence (Normal = 50ms/~20 FPS).
            if event::poll(Duration::from_millis(poll_ms))? {
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
    /// times instead of all at the top edge. `speed_min`/`speed_max` come from the
    /// `--speed` preset (MTRX-V2-01) so a slower/faster preset draws a
    /// slower/faster per-column fall.
    fn new_random<R: Rng + ?Sized>(rows: i32, speed_min: i32, speed_max: i32, rng: &mut R) -> Self {
        let trail_len = rng.random_range(TRAIL_MIN..=TRAIL_MAX);
        let speed = rng.random_range(speed_min..=speed_max);
        // Start somewhere above the top edge so the heads enter staggered.
        let head = -rng.random_range(0..=rows.max(1));
        Self {
            head,
            trail_len,
            speed,
        }
    }

    /// Advance the head by `speed`; once the head has cleared the bottom plus its
    /// whole trail, reset to a fresh randomized negative start (a new column run)
    /// drawn from the same `--speed` range.
    fn step<R: Rng + ?Sized>(&mut self, rows: i32, speed_min: i32, speed_max: i32, rng: &mut R) {
        self.head += self.speed;
        if self.head - self.trail_len > rows {
            *self = Self::new_random(rows, speed_min, speed_max, rng);
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

/// The rows a column's tail vacated this frame and that must be blanked so the
/// trail keeps a clean edge (WR-03). The head advanced `speed` rows since the
/// last frame, so the tail did too; the cells now just past the tail are
/// `(head - trail_len - speed)..=(head - trail_len - 1)`. Returns only the rows
/// that fall on-screen (`0..rows`), in top-to-bottom order. Pure and
/// terminal-free → unit-testable: for `speed == 1` it yields the single cell
/// `head - trail_len - 1` (the old behavior); for `speed >= 2` it yields the
/// full band so no gap row is left un-erased.
fn erase_band(head: i32, trail_len: i32, speed: i32, rows: i32) -> Vec<i32> {
    let mut band = Vec::new();
    // k = speed (topmost vacated row) down to 1 (cell just past the tail);
    // pushing in descending k gives ascending y (top-to-bottom).
    for k in (1..=speed.max(1)).rev() {
        let y = head - trail_len - k;
        if y >= 0 && y < rows {
            band.push(y);
        }
    }
    band
}

/// The pure katakana glyph table: every halfwidth katakana scalar in
/// U+FF66..=U+FF9D, in order. All are single-cell (East-Asian-Width Narrow) so
/// the grid math needs no width bookkeeping (D-07).
fn katakana_glyphs() -> Vec<char> {
    (KATAKANA_START..=KATAKANA_END)
        .filter_map(char::from_u32)
        .collect()
}

/// Which of the (R, G, B) channels are ACTIVE for a `--color` preset (MTRX-V2-01).
/// The active channels carry the trail level / head brightness; inactive ones are
/// `0` (trail) or [`HEAD_OFF`] (head). Pure so the whole preset→RGB mapping is
/// unit-testable without a terminal.
fn color_axes(color: MatrixColor) -> (bool, bool, bool) {
    match color {
        MatrixColor::Green => (false, true, false),
        MatrixColor::Red => (true, false, false),
        MatrixColor::Blue => (false, false, true),
        MatrixColor::Cyan => (false, true, true),
        MatrixColor::Magenta => (true, false, true),
        MatrixColor::Yellow => (true, true, false),
        MatrixColor::White => (true, true, true),
    }
}

/// The bright head-cell RGB for a `--color` preset: active channels at [`HEAD_ON`],
/// inactive channels at [`HEAD_OFF`] (the near-white tint). `Green` yields the v1
/// `(180, 255, 180)`. Pure.
fn head_rgb(color: MatrixColor) -> (u8, u8, u8) {
    let (r, g, b) = color_axes(color);
    let pick = |on: bool| if on { HEAD_ON } else { HEAD_OFF };
    (pick(r), pick(g), pick(b))
}

/// The trail-cell RGB for a `--color` preset at a given fade `level` ([`fade`]):
/// the active channels carry `level`, inactive channels are `0`. `Green` yields
/// the v1 `(0, level, 0)`. Pure.
fn trail_rgb(color: MatrixColor, level: u8) -> (u8, u8, u8) {
    let (r, g, b) = color_axes(color);
    let pick = |on: bool| if on { level } else { 0 };
    (pick(r), pick(g), pick(b))
}

/// Resolve a `--speed` preset (MTRX-V2-01) to `(poll_ms, speed_min, speed_max)`:
/// the `event::poll` frame interval and the per-column fall-speed range.
/// `Normal` reproduces the v1 cadence (50 ms, `SPEED_MIN..=SPEED_MAX`). Pure.
fn speed_params(speed: Speed) -> (u64, i32, i32) {
    match speed {
        Speed::Slow => (80, 1, 1),
        Speed::Normal => (50, SPEED_MIN, SPEED_MAX),
        Speed::Fast => (28, 2, 3),
    }
}

/// Resolve the `--charset` argument (MTRX-V2-01) to the rain's glyph table: a known
/// preset name maps to its table, otherwise the literal string's characters become
/// the glyph set. An empty custom string falls back to the katakana default so the
/// rain is never glyph-less. Pure so it unit-tests without a terminal.
fn resolve_charset(spec: &str) -> Vec<char> {
    match spec {
        "katakana" => katakana_glyphs(),
        "ascii" => ('!'..='~').collect(),
        "binary" => vec!['0', '1'],
        "digits" => ('0'..='9').collect(),
        other => {
            let chars: Vec<char> = other.chars().collect();
            if chars.is_empty() {
                katakana_glyphs()
            } else {
                chars
            }
        }
    }
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
            d.step(100, SPEED_MIN, SPEED_MAX, &mut rng);
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
        d.step(rows, SPEED_MIN, SPEED_MAX, &mut rng);
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

    /// WR-03 — at `speed == 1` the erase band is exactly the single cell just
    /// past the tail (`head - trail_len - 1`), preserving the original behavior.
    #[test]
    fn erase_band_speed_one_is_single_cell() {
        // head=30, trail_len=8 → tail at row 22, the cell past it is row 21.
        assert_eq!(erase_band(30, 8, 1, 100), vec![21]);
    }

    /// WR-03 — at `speed == 2` the head jumped 2 rows this frame, so TWO rows
    /// were vacated. The band must cover BOTH (`head-trail_len-2` and
    /// `head-trail_len-1`) so no stale glyph is left in the gap. The old
    /// single-cell erase missed the upper row (`head-trail_len-2`).
    #[test]
    fn erase_band_speed_two_covers_the_gap() {
        // head=30, trail_len=8, speed=2 → rows 20 and 21 (top-to-bottom).
        let band = erase_band(30, 8, 2, 100);
        assert_eq!(band, vec![20, 21], "both vacated rows must be erased");
        // The cell the old code would NOT have erased (head-trail_len-speed).
        assert!(
            band.contains(&(30 - 8 - 2)),
            "the gap row {} must be in the erase set: {band:?}",
            30 - 8 - 2
        );
        // For any speed, the band size never exceeds `speed` rows.
        assert!(band.len() <= 2, "band must not exceed speed rows: {band:?}");
    }

    /// WR-03 — rows that fall off-screen (negative, or `>= rows`) are dropped:
    /// the band only ever contains on-screen rows, so the `MoveTo` is always in
    /// bounds. A band whose every row is off-screen yields an empty set.
    #[test]
    fn erase_band_clamps_off_screen_rows() {
        // head=9, trail_len=8, speed=2 → candidates rows 0 and -1; only 0 is
        // on-screen, the negative row is dropped.
        assert_eq!(erase_band(9, 8, 2, 100), vec![0]);
        // Head still above the top edge → nothing on-screen yet → empty band.
        assert_eq!(erase_band(0, 8, 2, 100), Vec::<i32>::new());
        // A row at exactly `rows` is off the bottom edge (valid rows are 0..rows):
        // head=109, trail_len=8, speed=1 → candidate row 100, which is dropped.
        assert_eq!(erase_band(109, 8, 1, 100), Vec::<i32>::new());
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

    /// MTRX-V2-01 — each `--color` preset resolves to the expected head/trail RGB
    /// (pure, no terminal). `Green` reproduces the v1 look exactly.
    #[test]
    fn color_presets_map_to_expected_rgb() {
        // Green (the v1 default): head (180,255,180), trail (0, level, 0).
        assert_eq!(head_rgb(MatrixColor::Green), (180, 255, 180));
        assert_eq!(trail_rgb(MatrixColor::Green, FADE_BRIGHT), (0, 255, 0));
        assert_eq!(trail_rgb(MatrixColor::Green, FADE_DARK), (0, 40, 0));
        // Single-axis presets put the level on exactly their channel.
        assert_eq!(head_rgb(MatrixColor::Red), (255, 180, 180));
        assert_eq!(trail_rgb(MatrixColor::Red, 200), (200, 0, 0));
        assert_eq!(head_rgb(MatrixColor::Blue), (180, 180, 255));
        assert_eq!(trail_rgb(MatrixColor::Blue, 100), (0, 0, 100));
        // Two-axis presets light two channels; White lights all three.
        assert_eq!(trail_rgb(MatrixColor::Cyan, 90), (0, 90, 90));
        assert_eq!(trail_rgb(MatrixColor::Magenta, 90), (90, 0, 90));
        assert_eq!(trail_rgb(MatrixColor::Yellow, 90), (90, 90, 0));
        assert_eq!(head_rgb(MatrixColor::White), (255, 255, 255));
        assert_eq!(trail_rgb(MatrixColor::White, 77), (77, 77, 77));
    }

    /// MTRX-V2-01 — each `--speed` level resolves to a deterministic poll interval
    /// and fall range; faster levels poll sooner. `Normal` is the v1 cadence.
    #[test]
    fn speed_presets_map_to_expected_params() {
        assert_eq!(speed_params(Speed::Normal), (50, SPEED_MIN, SPEED_MAX));
        let (slow_ms, _, _) = speed_params(Speed::Slow);
        let (normal_ms, _, _) = speed_params(Speed::Normal);
        let (fast_ms, fast_min, fast_max) = speed_params(Speed::Fast);
        assert!(
            slow_ms > normal_ms && normal_ms > fast_ms,
            "poll interval must shrink from slow→normal→fast: {slow_ms} {normal_ms} {fast_ms}"
        );
        assert!(
            fast_min <= fast_max && fast_min >= 1,
            "fast range must be a valid, non-empty range: {fast_min}..={fast_max}"
        );
    }

    /// MTRX-V2-01 — `--charset` resolves a preset name to its table and any other
    /// string to that string's characters (pure). Empty → the katakana fallback.
    #[test]
    fn charset_resolves_presets_and_custom_strings() {
        assert_eq!(resolve_charset("katakana"), katakana_glyphs());
        assert_eq!(resolve_charset("binary"), vec!['0', '1']);
        assert_eq!(resolve_charset("digits"), ('0'..='9').collect::<Vec<_>>());
        let ascii = resolve_charset("ascii");
        assert!(!ascii.is_empty() && ascii.iter().all(|c| c.is_ascii_graphic()));
        // A literal custom string becomes its own glyph set (order preserved).
        assert_eq!(resolve_charset("AB01"), vec!['A', 'B', '0', '1']);
        // An empty custom string is never glyph-less — it falls back to katakana.
        assert_eq!(resolve_charset(""), katakana_glyphs());
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
