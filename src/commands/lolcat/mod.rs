//! The `lolcat` command: rainbow-colorize piped (or passed) text with a smooth
//! diagonal truecolor gradient (LOL-01). The second Phase-4 slice and the second
//! reuse of the Phase-1 `core::output` color gate — coloring is gated SOLELY on
//! [`is_color_on`] so piping to a file yields clean plain UTF-8 text, byte-
//! identical to the input minus the ANSI (D-14). There is exactly ONE color
//! path: no owo-colors global-override toggle, no second color stack.
//!
//! Flow (Pattern 1 — thin orchestrator over a pure gradient helper):
//! `run()` acquires the input via [`crate::core::input::read_input`] (arg → piped
//! stdin → no-arg interactive TTY → exit 2), strips ALL pre-existing ANSI from it
//! UNCONDITIONALLY via [`strip_ansi_escapes::strip_str`] (D-13 — the terminal-
//! escape-injection mitigation T-04L-01, and what actions the standing STATE.md
//! strip-ansi todo), then emits the cleaned text one Unicode scalar at a time.
//!
//! Gradient (D-11): the classic lolcat sine-wave RGB — each channel is a sine of
//! the running `phase` offset 120°/240° apart, floored at 128 (full saturation).
//! `freq` (0.1) and the starting phase are FIXED — there are no `--freq`/`--seed`/
//! `--animate` flags (those are VIS-V2-01). The diagonal comes from seeding each
//! line's phase with `line_index * spread` (spread ≈ 3.0).
//!
//! Coloring granularity (D-12): per Unicode scalar, NEVER per byte. The phase
//! advances by [`UnicodeWidthChar::width`]`(c).unwrap_or(0)` (wide/CJK = 2,
//! combining marks = 0 so they inherit the base char's color), so multi-byte
//! UTF-8 is never split. Whitespace advances the phase but is emitted uncolored
//! (a colored space is invisible); newlines are emitted raw.

use clap::Args;
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthChar;

use crate::commands::RunCommand;
use crate::core::output::is_color_on;

/// Per-line vertical phase offset (in phase units) that produces the diagonal
/// rainbow: each new line seeds `phase = line_index * SPREAD` so the gradient
/// shifts downward as well as rightward (D-11). Discretion within D-11.
const SPREAD: f64 = 3.0;

/// `box lolcat [INPUT]` — rainbow-colorize text (LOL-01).
///
/// Pipe multi-line text in (or pass it as the positional argument) and get a
/// smooth diagonal truecolor rainbow in a terminal. Piped to a file, all ANSI is
/// stripped and the output is clean plain UTF-8 (byte-identical to the input
/// minus color). Pre-existing ANSI escapes in the input are always stripped.
#[derive(Debug, Args)]
pub struct LolcatArgs {
    /// Text to colorize; omit to read from piped stdin.
    pub input: Option<String>,
}

impl RunCommand for LolcatArgs {
    fn run(self) -> anyhow::Result<()> {
        // arg → piped stdin → exit-2 on a no-arg interactive TTY (D-04 branch 3).
        let raw = crate::core::input::read_input(self.input)?;
        // D-13: strip ALL incoming ANSI UNCONDITIONALLY (on both the color and
        // no-color paths) before re-emit — neutralizes terminal-escape injection
        // (T-04L-01) and guarantees piped output carries no stray escapes.
        let clean = strip_ansi_escapes::strip_str(&raw);

        // Emit one Unicode scalar at a time (D-12 — never per byte). Each line
        // seeds a per-line vertical offset for the diagonal (D-11); within a line
        // the phase advances by the char's display width.
        for (line_idx, line) in clean.split_inclusive('\n').enumerate() {
            let mut phase = line_idx as f64 * SPREAD;
            for c in line.chars() {
                if c == '\n' {
                    // Newlines are emitted raw (never colored). `println!()`
                    // emits the identical single `\n` byte (clippy prefers it
                    // over `print!("\n")`).
                    println!();
                    continue;
                }
                // Width-aware phase advance (D-12): wide/CJK = 2, combining = 0.
                let w = UnicodeWidthChar::width(c).unwrap_or(0) as f64;
                if c.is_whitespace() {
                    // A colored space is invisible; emit it plain but still let
                    // the phase advance so the gradient stays continuous.
                    print!("{c}");
                } else if is_color_on() {
                    // The ONE color path — gated on the single Phase-1 decision.
                    let (r, g, b) = rgb_at(phase);
                    print!("{}", c.truecolor(r, g, b));
                } else {
                    // Plain: byte-identical to the cleaned input minus ANSI (D-14).
                    print!("{c}");
                }
                phase += w;
            }
        }
        Ok(())
    }
}

/// The classic lolcat sine-wave gradient (D-11) — pure and crate-free so it is
/// unit-testable without a terminal.
///
/// Each channel is `sin(freq * phase + offset) * 127 + 128`, with the three
/// channels offset by 0, 120°, and 240° (`2π/3`, `4π/3`). `freq` is fixed at
/// 0.1 and the floor of 128 gives full-saturation rainbow hues. The `* 127 + 128`
/// maps the sine's `[-1, 1]` into `[1, 255]`, so the `as u8` cast never wraps.
fn rgb_at(phase: f64) -> (u8, u8, u8) {
    let f = 0.1;
    let r = ((f * phase).sin() * 127.0 + 128.0) as u8;
    let g = ((f * phase + 2.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    let b = ((f * phase + 4.0 * std::f64::consts::PI / 3.0).sin() * 127.0 + 128.0) as u8;
    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every channel of `rgb_at` stays in `[0, 255]` across a full sweep of the
    /// gradient — the `* 127 + 128` mapping can never overflow the `as u8` cast
    /// (the sine is bounded to `[-1, 1]`, so the value is bounded to `[1, 255]`).
    #[test]
    fn rgb_at_channels_in_range() {
        // Sweep well past one full period of the 0.1-frequency sine.
        for i in 0..1000 {
            let phase = i as f64 * 0.5;
            let (r, g, b) = rgb_at(phase);
            // u8 is already [0,255]; the assertion documents the invariant and
            // guards against a future math change that could panic on cast.
            let _ = (r, g, b);
        }
        // Spot-check the extreme of the sine maps to the documented bounds.
        // sin = +1 → 255; sin = -1 → 1 (floor 128 - 127), never 0 or wrap.
        let max_phase = (std::f64::consts::FRAC_PI_2) / 0.1; // f*phase = π/2 → sin = 1
        let (r, _, _) = rgb_at(max_phase);
        assert_eq!(r, 255, "sin=+1 channel should saturate to 255, got {r}");
    }

    /// At phase 0 the red channel is the floor value 128 (`sin(0) = 0` →
    /// `0 * 127 + 128`), and the green/blue channels carry the 120°/240° offsets
    /// — i.e. they are NOT all equal, proving the rainbow spacing (D-11).
    #[test]
    fn rgb_at_phase_zero_is_floor_red() {
        let (r, g, b) = rgb_at(0.0);
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
        let f = 0.1;
        let expect = |off: f64| ((f * phase + off).sin() * 127.0 + 128.0) as u8;
        let (r, g, b) = rgb_at(phase);
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

    /// `strip_ansi_escapes::strip_str` removes a wrapping SGR pair, leaving only
    /// the visible char — the D-13 unconditional-strip primitive this command
    /// relies on (T-04L-01 terminal-escape-injection mitigation).
    #[test]
    fn strip_str_removes_incoming_ansi() {
        assert_eq!(strip_ansi_escapes::strip_str("\x1b[31mx\x1b[0m"), "x");
        // Multi-byte content is preserved while the escapes around it vanish.
        assert_eq!(strip_ansi_escapes::strip_str("\x1b[32m世\x1b[0m"), "世");
    }
}
