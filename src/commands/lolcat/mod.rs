//! The `lolcat` command: rainbow-colorize piped (or passed) text with a smooth
//! diagonal truecolor gradient (LOL-01), plus a bounded animated full-screen loop
//! (`--animate`, LOL-V2-01). The coloring is gated SOLELY on [`is_color_on`] so
//! piping to a file yields clean plain UTF-8 text, byte-identical to the input
//! minus the ANSI (D-14). There is exactly ONE color path and exactly ONE gradient
//! function ([`rgb_at`]): the static one-pass render AND every animated frame route
//! through it, so `--freq`/`--seed` govern both identically (D-11 single-color-path).
//!
//! Flow (Pattern 1 — thin orchestrator over a pure gradient helper):
//! `run()` acquires the input via [`crate::core::input::read_input`] (arg → piped
//! stdin → no-arg interactive TTY → exit 2), strips ALL pre-existing ANSI from it
//! UNCONDITIONALLY via [`strip_ansi_escapes::strip_str`] (D-13 — the terminal-
//! escape-injection mitigation T-04L-01), then either:
//!   * enters the [`run_animate`] alt-screen loop — ONLY when stdout is a real
//!     terminal AND color is on AND not under `--json`/`--clip` (the MANDATORY gate
//!     below), or
//!   * falls through to the existing static one-pass [`render_gradient`] (the
//!     degrade path; off-TTY it writes no escape — SC3/SC4).
//!
//! # The MANDATORY TTY/color AND-gate (T-09-03-PIPE / SC3)
//! `--animate` enters raw mode + the alternate screen, which write terminal
//! escapes. Those MUST NEVER reach a non-TTY stdout. The gate is therefore
//! `std::io::stdout().is_terminal() && is_color_on() && !is_json_on() && !is_clip_on()`.
//! `is_color_on()` ALONE is insufficient: `CLICOLOR_FORCE` can force color true on a
//! pipe, so the `is_terminal()` conjunct is load-bearing — without it a forced-color
//! pipe would enter raw mode and corrupt the stream. Off-TTY → the static renderer.
//!
//! Gradient (D-11): the classic lolcat sine-wave RGB — each channel is a sine of
//! the running `phase` offset 120°/240° apart, floored at 128 (full saturation).
//! `freq` (default 0.1) and the starting phase (`--seed`, default 0.0) are now
//! PARAMETERS of [`rgb_at`] so `--freq`/`--seed` shift the gradient on both paths.
//! The diagonal comes from seeding each line's phase with `line_index * spread`
//! (spread ≈ 3.0).
//!
//! Coloring granularity (D-12): per Unicode scalar, NEVER per byte. The phase
//! advances by [`UnicodeWidthChar::width`]`(c).unwrap_or(0)` (wide/CJK = 2,
//! combining marks = 0 so they inherit the base char's color), so multi-byte
//! UTF-8 is never split. Whitespace advances the phase but is emitted uncolored
//! (a colored space is invisible); newlines are emitted raw.
//!
//! # Animation loop (LOL-V2-01) — copies the `matrix` RAII template VERBATIM
//! The loop reuses `matrix`'s proven D-10 teardown: a [`RawGuard`] armed the INSTANT
//! raw mode is on (BEFORE the fallible alt-screen `execute!`), a single per-frame
//! `flush()`, and `event::poll(50ms)` as both the ~20-FPS frame timer AND the input
//! gate, with a `KeyEventKind::Press`-only [`is_quit`] (q / Esc / Ctrl+C). Each frame
//! advances ONE global phase offset (starting at `--seed`) and redraws the text in
//! place via [`rgb_at`]. It runs until an `Instant`-based `--duration` deadline
//! (default 3s; `--duration 0` = until a quit key) or a quit key. After teardown it
//! reprints ONE final colored frame to the NORMAL buffer so the rainbow PERSISTS.
//! The loop is kept panic-free (no `.unwrap()` on terminal ops) so `RawGuard::drop`
//! is the real restore backstop under the release profile `panic = "abort"`.
//!
//! Line breaks (WR-06): only `\n` is a supported line break. A lone/embedded
//! carriage return (`\r`) is a C0 control REMOVED by the unconditional D-13
//! `strip_str` above, BEFORE any coloring or emit, so a stray `\r` can never reach
//! the terminal to move the cursor to column 0 and overwrite the colored prefix.

// # Spine omission (SC4)
// `lolcat` is a DISPLAY-ONLY command: it INTENTIONALLY does not honor the global
// `--json`/`--clip` flags (roadmap SC4). The flags parse (global on `Cli`) but
// `run()` never calls `emit_json` to EMIT a document — it only CONSULTS
// `is_json_on()`/`is_clip_on()` as part of the animate AND-gate (to refuse the
// raw-mode loop under those flags). `box lolcat <text> --json` rainbow-prints the
// text and emits NO JSON document to stdout. Asserted by
// `tests/cli.rs::display_only_omit_json`.

use std::io::{BufWriter, IsTerminal, Write};
use std::time::{Duration, Instant};

use clap::Args;
use crossterm::cursor::{self, MoveTo};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthChar;

use crate::commands::RunCommand;
use crate::core::output::{is_clip_on, is_color_on, is_json_on};

/// Per-line vertical phase offset (in phase units) that produces the diagonal
/// rainbow: each new line seeds `phase = base + line_index * SPREAD` so the gradient
/// shifts downward as well as rightward (D-11). Discretion within D-11.
const SPREAD: f64 = 3.0;

/// The flow speed of `--animate`: how far the global phase offset advances each
/// frame (~20 FPS). A tuned constant (NOT a flag) — large enough that the rainbow
/// visibly flows, small enough to stay smooth. Discretion within D-11.
const ANIMATE_PHASE_STEP: f64 = 2.0;

/// Default `--duration` for `--animate`, in seconds (RESEARCH Q2 resolution). A
/// small lolcat-parity value; `--duration 0` overrides this to run until a quit key.
const DEFAULT_DURATION_SECS: u64 = 3;

/// `box lolcat [INPUT]` — rainbow-colorize text (LOL-01), optionally animated
/// (LOL-V2-01).
///
/// Pipe multi-line text in (or pass it as the positional argument) and get a
/// smooth diagonal truecolor rainbow in a terminal. Piped to a file, all ANSI is
/// stripped and the output is clean plain UTF-8 (byte-identical to the input
/// minus color). Pre-existing ANSI escapes in the input are always stripped.
#[derive(Debug, Args)]
pub struct LolcatArgs {
    /// Text to colorize; omit to read from piped stdin.
    pub input: Option<String>,

    /// Animate the rainbow as a bounded full-screen loop (real terminal only).
    /// Off-TTY (piped/redirected, or under --json/--clip) this degrades to the
    /// one-pass static render. Press q, Esc, or Ctrl+C to exit early.
    #[arg(long)]
    pub animate: bool,

    /// Gradient frequency — higher packs the color bands tighter. Governs BOTH the
    /// static and animated renders.
    #[arg(long, default_value_t = 0.1)]
    pub freq: f64,

    /// Initial gradient phase offset (the "seed") — shifts where the rainbow starts.
    /// Governs BOTH the static and animated renders.
    #[arg(long, default_value_t = 0.0)]
    pub seed: f64,

    /// With --animate: seconds to run before exiting (default 3). 0 = run until a
    /// quit key (q / Esc / Ctrl+C) is pressed. Ignored without --animate.
    #[arg(long, default_value_t = DEFAULT_DURATION_SECS)]
    pub duration: u64,
}

impl RunCommand for LolcatArgs {
    fn run(self) -> anyhow::Result<()> {
        let LolcatArgs {
            input,
            animate,
            freq,
            seed,
            duration,
        } = self;

        // arg → piped stdin → exit-2 on a no-arg interactive TTY (D-04 branch 3).
        let raw = crate::core::input::read_input(input)?;
        // D-13: strip ALL incoming ANSI UNCONDITIONALLY (on both the color and
        // no-color paths) before re-emit — neutralizes terminal-escape injection
        // (T-04L-01) and guarantees piped output carries no stray escapes.
        let clean = strip_ansi_escapes::strip_str(&raw);

        // The MANDATORY TTY/color AND-gate (T-09-03-PIPE / SC3). All four conjuncts
        // are required: `is_terminal()` is load-bearing because CLICOLOR_FORCE can
        // force `is_color_on()` true on a pipe, and a raw-mode escape on a pipe is
        // the forbidden hazard; `!is_json_on()`/`!is_clip_on()` keep the loop out of
        // the scriptable channels (SC4). Off-gate → fall through to the static
        // renderer (the byte-identical-minus-ANSI degrade path, D-14).
        if animate
            && std::io::stdout().is_terminal()
            && is_color_on()
            && !is_json_on()
            && !is_clip_on()
        {
            return run_animate(&clean, freq, seed, duration);
        }

        // Static one-pass render (the degrade path / the non-animated default).
        // Buffer the whole render and flush ONCE (WR-04): a `BufWriter` over a single
        // locked stdout handle coalesces the per-scalar writes; the BYTES are
        // identical to the old per-scalar sequence, so D-14 is unchanged.
        let stdout = std::io::stdout();
        let mut out = BufWriter::new(stdout.lock());
        render_gradient(&mut out, &clean, freq, seed)?;
        out.flush()?;
        Ok(())
    }
}

/// The static one-pass render — the SINGLE gradient path shared by the non-animated
/// command, the `--animate` degrade path, and the final persisted frame after an
/// animation. Emits one Unicode scalar at a time (D-12 — never per byte); each line
/// seeds a per-line vertical offset `base_phase + line_idx * SPREAD` for the
/// diagonal (D-11); within a line the phase advances by the char's display width.
/// Coloring is gated SOLELY on [`is_color_on`] so the plain branch is byte-identical
/// to the input minus ANSI (D-14). Pure over its `Write` sink → terminal-free.
fn render_gradient<W: Write>(
    out: &mut W,
    text: &str,
    freq: f64,
    base_phase: f64,
) -> std::io::Result<()> {
    for (line_idx, line) in text.split_inclusive('\n').enumerate() {
        let mut phase = base_phase + line_idx as f64 * SPREAD;
        for c in line.chars() {
            if c == '\n' {
                // Newlines are emitted raw (never colored) — a single `\n` byte.
                out.write_all(b"\n")?;
                continue;
            }
            // Width-aware phase advance (D-12): wide/CJK = 2, combining = 0.
            let w = UnicodeWidthChar::width(c).unwrap_or(0) as f64;
            if c.is_whitespace() {
                // A colored space is invisible; emit it plain but still advance.
                write!(out, "{c}")?;
            } else if is_color_on() {
                // The ONE color path — gated on the single Phase-1 decision.
                let (r, g, b) = rgb_at(phase, freq);
                write!(out, "{}", c.truecolor(r, g, b))?;
            } else {
                // Plain: byte-identical to the cleaned input minus ANSI (D-14).
                write!(out, "{c}")?;
            }
            phase += w;
        }
    }
    Ok(())
}

/// RAII terminal-restore guard (D-10), copied VERBATIM from `matrix`. Constructed
/// the INSTANT raw mode is enabled — BEFORE the alternate-screen/cursor `execute!`
/// — so it also covers a failure of that fallible setup (CR-01). Its `Drop` undoes
/// the setup on EVERY non-aborting exit path (normal return, `?` early-return,
/// unwinding). All teardown errors are deliberately ignored — `drop` must not panic
/// and there is nothing useful to do if restore fails (e.g. `LeaveAlternateScreen`
/// when the alternate screen was never entered).
struct RawGuard;

impl Drop for RawGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = execute!(out, cursor::Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// The bounded alternate-screen animation loop (LOL-V2-01). Reached ONLY through the
/// MANDATORY AND-gate in [`LolcatArgs::run`], so stdout is guaranteed a real TTY
/// with color on. Copies `matrix`'s RAII / single-flush / Press-only-quit template
/// verbatim. `seed` is the initial global phase offset; `duration` 0 = run until a
/// quit key. On exit it leaves the alternate screen and reprints ONE final colored
/// frame to the normal buffer so the rainbow PERSISTS.
fn run_animate(text: &str, freq: f64, seed: f64, duration: u64) -> anyhow::Result<()> {
    enable_raw_mode()?;
    // Arm the guard THE INSTANT raw mode is on, BEFORE any further fallible `?`
    // (D-10 / CR-01). If the `execute!` below fails, the guard's Drop still restores.
    let _guard = RawGuard;
    let mut out = std::io::stdout();
    execute!(out, EnterAlternateScreen, cursor::Hide)?;

    // `--duration 0` = run until a quit key; otherwise an Instant deadline.
    let deadline = (duration != 0).then(|| Instant::now() + Duration::from_secs(duration));
    let mut phase = seed;

    loop {
        // 1. Redraw the whole frame in place (text is positionally constant, so each
        //    frame overwrites the previous glyphs — no Clear, no flicker), then
        //    flush EXACTLY ONCE per frame (the per-char-flush pitfall).
        render_frame(&mut out, text, freq, phase)?;
        out.flush()?;

        // 2. Deadline check AFTER showing at least one frame.
        if deadline.is_some_and(|dl| Instant::now() >= dl) {
            break;
        }

        // 3. The 50ms poll IS the ~20-FPS frame timer AND the input gate (D-09).
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Press-only: Windows fires Press AND Release; filtering here stops
                // the exit key double-counting (Pitfall 3 / D-10).
                if key.kind == KeyEventKind::Press && is_quit(&key) {
                    break;
                }
            }
        }

        // 4. Advance the single global phase offset → the rainbow flows.
        phase += ANIMATE_PHASE_STEP;
    }

    // Explicit teardown so the final colored frame lands in the NORMAL buffer (not
    // the alternate screen) and PERSISTS. Errors ignored (the guard is the backstop
    // and re-runs this idempotently on drop). disable_raw_mode FIRST so the final
    // reprint's `\n` returns the cursor to column 0 (cooked mode).
    let _ = execute!(out, cursor::Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let mut final_out = BufWriter::new(out.lock());
    render_gradient(&mut final_out, text, freq, phase)?;
    final_out.write_all(b"\n")?;
    final_out.flush()?;
    Ok(())
}

/// Draw one animation frame into the buffered `out` (queued, NOT flushed — the
/// caller flushes once). The text is redrawn in place: each line is positioned with
/// `MoveTo(0, line_idx)` (raw mode does NOT translate `\n` to `\r\n`, so explicit
/// positioning is required), then its scalars are colored via [`rgb_at`] at
/// `base_phase + line_idx * SPREAD`. Reached only with color on (the AND-gate), so
/// non-whitespace is always colored; whitespace is emitted plain (an invisible
/// colored space) but still advances the phase. Pure over its `Write` sink.
fn render_frame<W: Write>(
    out: &mut W,
    text: &str,
    freq: f64,
    base_phase: f64,
) -> std::io::Result<()> {
    for (line_idx, line) in text.split('\n').enumerate() {
        queue!(out, MoveTo(0, line_idx as u16))?;
        let mut phase = base_phase + line_idx as f64 * SPREAD;
        for c in line.chars() {
            let w = UnicodeWidthChar::width(c).unwrap_or(0) as f64;
            if c.is_whitespace() {
                queue!(out, Print(c))?;
            } else {
                let (r, g, b) = rgb_at(phase, freq);
                queue!(out, Print(c.truecolor(r, g, b)))?;
            }
            phase += w;
        }
    }
    Ok(())
}

/// Quit predicate (D-10), copied VERBATIM from `matrix`. Pure so the Press-only +
/// key-set logic is unit-tested without a terminal: true iff the event is a key
/// **Press** AND the code is `q` or Esc, OR is `c` held with CONTROL (Ctrl+C, which
/// raw mode delivers as a KeyEvent, not a signal).
fn is_quit(key: &KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

/// The classic lolcat sine-wave gradient (D-11) — pure and crate-free so it is
/// unit-testable without a terminal. The SINGLE gradient function: the static
/// render, every animated frame, and the persisted final frame all call it.
///
/// Each channel is `sin(freq * phase + offset) * 127 + 128`, with the three
/// channels offset by 0, 120°, and 240° (`2π/3`, `4π/3`). `freq` controls the band
/// spacing (the static/animated default is 0.1); the floor of 128 gives full-
/// saturation rainbow hues. The `* 127 + 128` maps the sine's `[-1, 1]` into
/// `[1, 255]`, so the `as u8` cast never wraps. The starting `phase` is the `--seed`
/// offset (plus the per-line diagonal), so changing either `freq` or `phase` shifts
/// the returned triple.
fn rgb_at(phase: f64, freq: f64) -> (u8, u8, u8) {
    let r = ((freq * phase).sin() * 127.0 + 128.0) as u8;
    let g = ((freq * phase + 2.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    let b = ((freq * phase + 4.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    /// The default gradient frequency (the prior hard-coded `f = 0.1`), used by the
    /// tests that lock the historical channel math.
    const DEFAULT_FREQ: f64 = 0.1;

    /// Every channel of `rgb_at` stays in `[0, 255]` across a full sweep of the
    /// gradient — the `* 127 + 128` mapping can never overflow the `as u8` cast
    /// (the sine is bounded to `[-1, 1]`, so the value is bounded to `[1, 255]`).
    #[test]
    fn rgb_at_channels_in_range() {
        // Sweep well past one full period of the default-frequency sine.
        for i in 0..1000 {
            let phase = i as f64 * 0.5;
            let (r, g, b) = rgb_at(phase, DEFAULT_FREQ);
            // u8 is already [0,255]; the assertion documents the invariant and
            // guards against a future math change that could panic on cast.
            let _ = (r, g, b);
        }
        // Spot-check the extreme of the sine maps to the documented bounds.
        // sin = +1 → 255; sin = -1 → 1 (floor 128 - 127), never 0 or wrap.
        let max_phase = (std::f64::consts::FRAC_PI_2) / DEFAULT_FREQ; // f*phase = π/2 → sin = 1
        let (r, _, _) = rgb_at(max_phase, DEFAULT_FREQ);
        assert_eq!(r, 255, "sin=+1 channel should saturate to 255, got {r}");
    }

    /// At phase 0 the red channel is the floor value 128 (`sin(0) = 0` →
    /// `0 * 127 + 128`), and the green/blue channels carry the 120°/240° offsets
    /// — i.e. they are NOT all equal, proving the rainbow spacing (D-11).
    #[test]
    fn rgb_at_phase_zero_is_floor_red() {
        let (r, g, b) = rgb_at(0.0, DEFAULT_FREQ);
        assert_eq!(r, 128, "phase-0 red is the floor 128, got {r}");
        // g = sin(120°)*127+128 ≈ 0.866*127+128 ≈ 238; b = sin(240°)*127+128 ≈ 18.
        assert!(
            r != g || g != b,
            "channels must differ at phase 0 (120° spacing): ({r},{g},{b})"
        );
    }

    /// The three channels at a fixed phase match the 120°-spaced sine reference
    /// within ±1 (the `as u8` truncation rounds toward zero, so allow ±1).
    #[test]
    fn rgb_at_matches_120_degree_spacing() {
        let phase = 12.34_f64;
        let f = DEFAULT_FREQ;
        let expect = |off: f64| ((f * phase + off).sin() * 127.0 + 128.0) as u8;
        let (r, g, b) = rgb_at(phase, f);
        assert_eq!(r, expect(0.0), "red channel mismatch");
        assert_eq!(
            g,
            expect(2.0 * std::f64::consts::PI / 3.0),
            "green offset 120°"
        );
        assert_eq!(
            b,
            expect(4.0 * std::f64::consts::PI / 3.0),
            "blue offset 240°"
        );
    }

    /// LOL-V2-01 — changing `freq` changes the returned triple (the gradient
    /// re-spaces), and changing the starting `phase` (the `--seed` offset) ALSO
    /// changes it (the gradient shifts). This is the automatable proof that
    /// `--freq`/`--seed` are wired into the single gradient path; the *visible*
    /// on-screen effect is the PS7 human-verify gate (Task 3).
    #[test]
    fn rgb_at_freq_and_seed_change_output() {
        // Same phase, different freq → different triple (re-spaced bands).
        let phase = 7.5_f64;
        assert_ne!(
            rgb_at(phase, 0.1),
            rgb_at(phase, 0.5),
            "changing freq must change the gradient triple"
        );
        // Same freq, different starting phase (a --seed shift) → different triple.
        assert_ne!(
            rgb_at(5.0, DEFAULT_FREQ),
            rgb_at(55.0, DEFAULT_FREQ),
            "a --seed phase shift must change the gradient triple"
        );
    }

    /// `strip_ansi_escapes::strip_str` removes a wrapping SGR pair, leaving only
    /// the visible char — the D-13 unconditional-strip primitive this command
    /// relies on (T-04L-01 terminal-escape-injection mitigation).
    #[test]
    fn strip_str_removes_incoming_ansi() {
        assert_eq!(strip_ansi_escapes::strip_str("\x1b[31mx\x1b[0m"), "x");
        // Multi-byte content is preserved while the escapes around it vanish.
        assert_eq!(strip_ansi_escapes::strip_str("\x1b[32m世\x1b[0m"), "世");
    }

    /// WR-06 — a lone/embedded carriage return (old-Mac line ending, mid-line
    /// `\r`, or the `\r` of a `\r\n`) is REMOVED by the same unconditional
    /// `strip_str` that runs before any coloring/emit. So a stray `\r` never
    /// reaches the terminal to move the cursor to column 0 and overwrite the
    /// already-printed colored prefix — the concern WR-06 raised cannot occur.
    /// Only `\n` line breaks survive (the supported line-break, D-11/D-12). This
    /// test LOCKS that behavior so a future `strip_str` swap can't silently
    /// reintroduce the cursor-overwrite glitch.
    #[test]
    fn strip_str_removes_lone_carriage_returns() {
        // Old-Mac/CRLF lone CR before a newline: dropped, the `\n` stays.
        assert_eq!(strip_ansi_escapes::strip_str("a\rb\n"), "ab\n");
        // Mid-line CR with no following newline: dropped, no column-0 jump left.
        assert_eq!(strip_ansi_escapes::strip_str("mid\rline"), "midline");
        // The CR of a CRLF pair is removed; the LF survives as the line break.
        assert_eq!(strip_ansi_escapes::strip_str("crlf\r\n"), "crlf\n");
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

    /// The three quit keys (q, Esc, Ctrl+C) on a Press event all quit — the
    /// matrix-verbatim predicate the `--animate` loop relies on.
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

    /// A bare `c` without CONTROL is NOT a quit (only Ctrl+C is), and Release/Repeat
    /// events never quit — the Windows double-fire filter (D-10 / Pitfall 3).
    #[test]
    fn non_quit_events_do_not_quit() {
        assert!(!is_quit(&key(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
            KeyEventKind::Press
        )));
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
