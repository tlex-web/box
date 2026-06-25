//! The `pomodoro` command: a blocking foreground raw-mode countdown timer with
//! break / long-break modes, a clean Ctrl+C / q / Esc cancel, and a Windows
//! toast notification on completion (POMO-01).
//!
//! # Timer model (D-07)
//! A single-threaded blocking loop. `event::poll(min(1s, remaining))` is BOTH the
//! ~1s countdown tick AND the keypress reader — there is NO background thread and
//! NO SIGINT race. Each tick recomputes `remaining = end - now` and redraws the
//! `MM:SS` line IN PLACE on the NORMAL screen (a single clean write per tick — the
//! matrix flush-once discipline; NEVER per character). When `remaining` hits zero
//! the loop falls through to COMPLETION.
//!
//! In raw mode crossterm delivers Ctrl+C as a `KeyEvent` (`Char('c')` + `CONTROL`),
//! NOT a SIGINT — so there is no `ctrlc` crate. Only `KeyEventKind::Press` events
//! are honored: Windows fires both Press AND Release, so without that filter a
//! cancel key would double-count (Pitfall POMO-1). Cancel (`q` / `Esc` / Ctrl+C)
//! → `"cancelled"` to stderr + exit 1, and the toast NEVER fires. Completion →
//! restore the terminal, fire the toast, exit 0.
//!
//! # Durations (D-08)
//! `box pomodoro [MINUTES]` defaults to 25 (work); `--break` → 5, `--long-break`
//! → 15. A positional `[MINUTES]` overrides any flag. If BOTH `--break` and
//! `--long-break` are set, `--long-break` wins (a documented, deterministic
//! tie-break; unit-tested).
//!
//! # Toast (D-09)
//! `tauri-winrt-notification` — the maintained drop-in fork of CLAUDE.md's
//! `winrt-notification` (D-09 OVERRIDE). `Toast::POWERSHELL_APP_ID` means no AUMID
//! registration is needed (the toast attributes to PowerShell — accepted, D-09).
//! A toast that fails to `show()` AFTER a completed timer does NOT fail the run —
//! it logs to stderr and still exits 0 (RESEARCH A3 / D-07 "completion → exit 0").
//!
//! # Teardown (D-07)
//! Restoration is an RAII [`RawGuard`] armed the INSTANT raw mode is enabled —
//! BEFORE the fallible `cursor::Hide` `execute!`, so a failure there still restores
//! the terminal (matrix CR-01 ordering). Its `Drop` runs `cursor::Show` +
//! `disable_raw_mode()` (errors ignored) on normal return, on a `?` early-return,
//! and on unwinding. The release profile is `panic = "abort"`, so the loop is kept
//! panic-free (no `.unwrap()` on terminal ops) — the `Drop` guard is the real
//! restore path. Pomodoro keeps the NORMAL screen (in-place countdown), so the
//! guard does NOT leave an alternate screen.
//!
//! # Spine omission (SC4)
//! `pomodoro` is a DISPLAY-ONLY command: it INTENTIONALLY does not honor the global
//! `--json`/`--clip` flags (roadmap SC4). The flags parse (global on `Cli`) but
//! `run()` never calls `is_json_on()` / `emit_json` — `box pomodoro --json` runs the
//! normal countdown and emits NO JSON document to stdout. Asserted (via source
//! state) by `tests/cli.rs::display_only_omit_json`.

use std::io::Write;
use std::time::{Duration, Instant};

use clap::Args;
use crossterm::cursor::{self, MoveToColumn};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::queue;
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use owo_colors::OwoColorize;
use tauri_winrt_notification::Toast;

use crate::commands::RunCommand;
use crate::core::output::is_color_on;

/// Default work-session length in minutes (D-08).
const WORK_MINUTES: u64 = 25;
/// `--break` short-break length in minutes (D-08).
const BREAK_MINUTES: u64 = 5;
/// `--long-break` long-break length in minutes (D-08).
const LONG_BREAK_MINUTES: u64 = 15;
/// Upper bound (inclusive) on an accepted positional `[MINUTES]` (WR-01): ~1 year
/// (366 days). A value above this is rejected by clap as a USAGE error (exit 2)
/// BEFORE `run()`. The cap is deliberately small relative to `u64::MAX` so that
/// `mins * 60` cannot wrap and `Instant::now() + Duration::from_secs(mins * 60)`
/// cannot overflow-panic — preserving the T-05 no-panic invariant against an
/// absurd numeric argument while staying far larger than any real focus session.
const MAX_MINUTES: u64 = 24 * 60 * 366;

/// `box pomodoro` — a focus timer with a Windows toast on completion (POMO-01).
///
/// Runs a blocking in-place `MM:SS` countdown in the terminal. The default is a
/// 25-minute work session; `--break` runs 5 minutes and `--long-break` runs 15.
/// A positional `[MINUTES]` overrides the mode. Press Ctrl+C, q, or Esc to cancel
/// — "cancelled" prints to stderr, the command exits 1, and NO toast fires. When
/// the timer completes a Windows toast notification appears and the command exits
/// 0. The cursor and terminal are restored on every exit path.
#[derive(Debug, Args)]
pub struct PomodoroArgs {
    /// Minutes to run; defaults to 25 (work), 5 with --break, 15 with --long-break.
    ///
    /// Bounded at parse time to `1..=MAX_MINUTES` (WR-01). An out-of-range value
    /// (`0`, or an absurd `u64` such as `18446744073709551615`) is rejected by clap
    /// as a USAGE error (exit 2) BEFORE `run()` — the same `RangedU64ValueParser`
    /// pattern `du`/`tree` use for `--depth`/`--top`. This guarantees `mins * 60`
    /// cannot wrap and `Instant::now() + total` cannot overflow-panic, preserving
    /// the T-05 no-panic invariant against an attacker-supplied numeric argument.
    #[arg(value_parser = clap::builder::RangedU64ValueParser::<u64>::new().range(1..=MAX_MINUTES))]
    pub minutes: Option<u64>,
    /// Run a 5-minute short break instead of a 25-minute work session.
    #[arg(long = "break")]
    pub break_: bool,
    /// Run a 15-minute long break (wins over --break if both are given).
    #[arg(long = "long-break")]
    pub long_break: bool,
}

/// RAII terminal-restore guard (D-07). Constructed the INSTANT raw mode is enabled
/// — BEFORE the fallible `cursor::Hide` `execute!` — so it also covers a failure of
/// that setup step (matrix CR-01). Its `Drop` undoes the setup on EVERY
/// non-aborting exit path (normal return, `?` early-return, unwinding). Teardown
/// errors are deliberately ignored — there is nothing useful to do if restore
/// fails, and `drop` must not panic. Pomodoro keeps the NORMAL screen, so the guard
/// restores `cursor::Show` + `disable_raw_mode()` ONLY (no `LeaveAlternateScreen`).
struct RawGuard;

impl Drop for RawGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, cursor::Show);
        let _ = disable_raw_mode();
    }
}

/// The bright green used for the countdown digits when color is on (D-08
/// discretion). Gated on `is_color_on()` so piped output is byte-identical minus
/// ANSI — the locked color contract (D-00).
const DIGITS_RGB: (u8, u8, u8) = (120, 255, 120);

impl RunCommand for PomodoroArgs {
    fn run(self) -> anyhow::Result<()> {
        let total = resolve_duration(self.minutes, self.break_, self.long_break);

        // Enable raw mode FIRST, then arm the guard THE INSTANT raw mode is on,
        // BEFORE the fallible `cursor::Hide` execute! (matrix CR-01 ordering). If
        // `Hide` fails, its `?` early-returns from `run()` — but the guard already
        // exists, so its `Drop` runs `disable_raw_mode()` and the terminal is
        // restored. Were the guard armed AFTER `Hide`, a failure there would
        // early-return with raw mode still on and no guard to undo it.
        enable_raw_mode()?;
        let _guard = RawGuard;
        let mut out = std::io::stdout();
        // No EnterAlternateScreen — the in-place countdown lives on the NORMAL
        // screen (discretion); the guard restores Show + disable_raw_mode only.
        crossterm::execute!(out, cursor::Hide)?;

        let end = Instant::now() + total;
        // `cancelled` is the loop's break value: true on a cancel key, false when
        // the countdown reaches zero (COMPLETION). The toast fires ONLY on the
        // false (completion) path, AFTER this loop and AFTER the guard restores the
        // terminal — the cancel path NEVER reaches `show()` (D-07 / T-05-POMO-CANCEL).
        let cancelled = loop {
            let remaining = end.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break false; // COMPLETION — loop fell through.
            }

            // Render the MM:SS line IN PLACE: return to column 0, clear the line,
            // print the digits, then ONE flush per tick (the matrix flush-once
            // discipline — NEVER per character, D-07). Color is gated on
            // is_color_on() so piped output is byte-identical minus ANSI (D-00).
            let label = fmt_mmss(remaining.as_secs());
            queue!(out, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
            if is_color_on() {
                queue!(
                    out,
                    Print(label.truecolor(DIGITS_RGB.0, DIGITS_RGB.1, DIGITS_RGB.2))
                )?;
            } else {
                queue!(out, Print(&label))?;
            }
            out.flush()?;

            // poll(min(1s, remaining)) IS the ~1s countdown timer AND the keypress
            // gate (D-07). The `min` makes the final partial second exact instead
            // of overshooting a fixed 1s poll (Pitfall POMO-2).
            let tick = remaining.min(Duration::from_secs(1));
            if event::poll(tick)? {
                if let Event::Key(key) = event::read()? {
                    // Press-only filter inside is_cancel: Windows fires Press AND
                    // Release, so an un-filtered cancel key would double-count.
                    if is_cancel(&key) {
                        break true; // CANCEL — exit 1, NO toast.
                    }
                }
            }
        };

        // Restore the terminal BEFORE any message or toast (D-07). Drop runs
        // cursor::Show + disable_raw_mode; the explicit drop makes the ordering
        // visible (the guard would otherwise drop at end of scope anyway).
        drop(_guard);
        // Move off the (now-restored) countdown line so stderr / the next prompt
        // start cleanly.
        println!();

        if cancelled {
            // Message → stderr, exit 1, and the toast below is NEVER reached on
            // this path (T-05-POMO-CANCEL: `show()` sits after this guard).
            eprintln!("cancelled");
            std::process::exit(1);
        }

        // COMPLETION → fire the Windows toast (owned-self builder; POWERSHELL_APP_ID
        // needs no AUMID registration, D-09) and exit 0. A toast that fails to show
        // after a COMPLETED timer must NOT fail the run — log to stderr and still
        // exit 0 (RESEARCH A3 / D-07 "completion → exit 0").
        if let Err(e) = Toast::new(Toast::POWERSHELL_APP_ID)
            .title("Pomodoro")
            .text1("Time's up!")
            .show()
        {
            eprintln!("note: could not show toast notification: {e}");
        }
        Ok(())
    }
}

/// Format a whole number of seconds as `MM:SS` (zero-padded). Pure and
/// terminal-free → unit-testable. (RESEARCH:309.)
fn fmt_mmss(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// Resolve the countdown duration from the parsed args (D-08): a positional
/// `minutes` override ALWAYS wins; otherwise `--long-break` → 15, `--break` → 5,
/// else the 25-minute default. `--long-break` beats `--break` when both are set (a
/// documented, deterministic tie-break). Pure → unit-testable without a terminal.
fn resolve_duration(minutes: Option<u64>, break_: bool, long_break: bool) -> Duration {
    let mins = match minutes {
        Some(m) => m,
        None if long_break => LONG_BREAK_MINUTES,
        None if break_ => BREAK_MINUTES,
        None => WORK_MINUTES,
    };
    // Defense in depth (WR-01): the clap `RangedU64ValueParser` already rejects
    // `minutes > MAX_MINUTES` (exit 2) before this is reached, but clamp here too so
    // this pure, separately-unit-tested seam can NEVER produce a `mins * 60` that
    // wraps `u64` — the multiply is overflow-safe for any input. `MAX_MINUTES * 60`
    // is ~31.6M seconds, far below `u64::MAX`, so the product is always exact.
    let mins = mins.min(MAX_MINUTES);
    Duration::from_secs(mins * 60)
}

/// Cancel predicate (D-07), pure so the Press-only + key-set logic is unit-tested
/// without a terminal: returns true iff the event is a key **Press** AND the code
/// is `q` or Esc, OR is `c` held with CONTROL (Ctrl+C, which raw mode delivers as a
/// `KeyEvent`, not a signal — Pitfall POMO-1). Copied verbatim from the matrix
/// `is_quit` predicate (src/commands/matrix/mod.rs:307-313).
fn is_cancel(key: &KeyEvent) -> bool {
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

    /// `fmt_mmss` zero-pads both fields and rolls minutes over at 60 seconds.
    #[test]
    fn fmt_mmss_pads_and_rolls_over() {
        assert_eq!(fmt_mmss(0), "00:00");
        assert_eq!(fmt_mmss(65), "01:05");
        assert_eq!(fmt_mmss(1500), "25:00");
    }

    /// The four duration paths (D-08): default 25, `--break` 5, `--long-break` 15.
    #[test]
    fn resolve_duration_default_and_flags() {
        assert_eq!(
            resolve_duration(None, false, false),
            Duration::from_secs(25 * 60),
            "default is a 25-minute work session"
        );
        assert_eq!(
            resolve_duration(None, true, false),
            Duration::from_secs(5 * 60),
            "--break is 5 minutes"
        );
        assert_eq!(
            resolve_duration(None, false, true),
            Duration::from_secs(15 * 60),
            "--long-break is 15 minutes"
        );
    }

    /// A positional `[MINUTES]` ALWAYS overrides the flags (D-08).
    #[test]
    fn resolve_duration_positional_overrides_flags() {
        assert_eq!(
            resolve_duration(Some(50), false, false),
            Duration::from_secs(50 * 60),
            "positional overrides the default"
        );
        assert_eq!(
            resolve_duration(Some(3), true, false),
            Duration::from_secs(3 * 60),
            "positional beats --break"
        );
        assert_eq!(
            resolve_duration(Some(7), false, true),
            Duration::from_secs(7 * 60),
            "positional beats --long-break"
        );
    }

    /// The documented tie-break: when BOTH flags are set, `--long-break` wins.
    #[test]
    fn resolve_duration_long_break_beats_break() {
        assert_eq!(
            resolve_duration(None, true, true),
            Duration::from_secs(15 * 60),
            "--long-break wins over --break when both are set"
        );
    }

    /// WR-01 — an absurd positional `minutes` (here `u64::MAX`) is clamped to
    /// `MAX_MINUTES` so the internal `mins * 60` can NEVER wrap and the resulting
    /// `Duration` stays representable. This is the pure-seam half of the no-panic
    /// fix (the clap parser rejects it as exit 2 at the binary boundary; this proves
    /// the function itself is overflow-safe even if called directly).
    #[test]
    fn resolve_duration_clamps_absurd_minutes() {
        assert_eq!(
            resolve_duration(Some(u64::MAX), false, false),
            Duration::from_secs(MAX_MINUTES * 60),
            "an out-of-range minutes value is clamped to MAX_MINUTES, never overflowed"
        );
        // The capped duration is well below the range where `Instant::now() + d`
        // could overflow, so the run loop's `Instant` add is safe by construction.
        assert!(
            resolve_duration(Some(u64::MAX), false, false) <= Duration::from_secs(MAX_MINUTES * 60)
        );
    }

    /// Build a `KeyEvent` for the cancel-key tests.
    fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind,
            state: KeyEventState::NONE,
        }
    }

    /// The three cancel keys (q, Esc, Ctrl+C) on a Press event all cancel.
    #[test]
    fn cancel_keys_on_press() {
        assert!(is_cancel(&key(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Press
        )));
        assert!(is_cancel(&key(
            KeyCode::Esc,
            KeyModifiers::NONE,
            KeyEventKind::Press
        )));
        assert!(is_cancel(&key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press
        )));
    }

    /// A bare `c` without CONTROL is NOT a cancel (only Ctrl+C is).
    #[test]
    fn plain_c_is_not_cancel() {
        assert!(!is_cancel(&key(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
            KeyEventKind::Press
        )));
    }

    /// Release/Repeat events never cancel — the Windows double-fire filter (D-07).
    #[test]
    fn release_and_repeat_do_not_cancel() {
        assert!(
            !is_cancel(&key(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                KeyEventKind::Release
            )),
            "a Release of q must not cancel (Windows double-fire filter)"
        );
        assert!(
            !is_cancel(&key(KeyCode::Esc, KeyModifiers::NONE, KeyEventKind::Repeat)),
            "a Repeat of Esc must not cancel"
        );
    }
}
