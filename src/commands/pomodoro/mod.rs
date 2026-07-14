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
//! # Auto-cycle (POMO-V2-01, D-06/D-07)
//! Auto-cycling is OPT-IN: bare `box pomodoro [MINUTES]` stays a single blocking
//! timer that exits (backward compatible). `--cycles N` runs N work sessions, each
//! followed by a break; every 4th break is the 15-minute long break, all others are
//! 5-minute short breaks (the classic cadence — [`break_is_long`]). `--loop` runs
//! work/break indefinitely until cancel. A session counter shows in the countdown
//! line (`Pomodoro 3/4` under `--cycles`, `Pomodoro #3` under `--loop` —
//! [`session_counter`]) and an optional `--label` annotates the line and the
//! completion toast. Crucially the single [`RawGuard`] is armed ONCE before the
//! OUTER segment loop — one continuous raw-mode session across every sub-timer
//! (D-08), never re-enabled per segment. Cancel (Ctrl+C / q / Esc) at ANY segment
//! restores the terminal and exits 1 with NO toast; each COMPLETED segment fires its
//! toast; completing the full set exits 0.
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
/// Upper bound (inclusive) on `--cycles <N>` (POMO-V2-01). A value above this is
/// rejected by clap as a USAGE error (exit 2) BEFORE `run()` — the same
/// `RangedU64ValueParser` bound the `minutes` field uses, so an absurd cycle count
/// cannot spin the outer segment loop. 1000 is far beyond any real focus schedule
/// (a full workday is ~16 pomodoros) while staying trivially bounded. `--loop` is
/// the intentionally-unbounded escape hatch (user-cancellable at any time).
const MAX_CYCLES: u64 = 1000;

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
    /// Auto-cycle N work sessions, each followed by a break (every 4th is a 15-min
    /// long break). Bounded at parse time to `1..=1000` (an out-of-range value is a
    /// clap usage error, exit 2). Conflicts with `--loop`.
    #[arg(
        long,
        conflicts_with = "loop_",
        value_parser = clap::builder::RangedU64ValueParser::<u64>::new().range(1..=MAX_CYCLES)
    )]
    pub cycles: Option<u64>,
    /// Loop work/break sessions indefinitely until you cancel (Ctrl+C / q / Esc).
    #[arg(long = "loop")]
    pub loop_: bool,
    /// Label shown in the countdown line and carried into the completion toast.
    #[arg(long)]
    pub label: Option<String>,
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

/// The muted color used for the `--label` / session-counter text when color is on
/// (POMO-V2-01 discretion). Like the digits, gated on `is_color_on()` so piped
/// output is byte-identical minus ANSI — the locked color contract (D-00).
const LABEL_RGB: (u8, u8, u8) = (150, 180, 210);

/// The outcome of one timed segment's inner tick loop (POMO-V2-01): the countdown
/// either reached zero (`Completed`) or a cancel key was pressed (`Cancelled`).
/// The caller fires the toast/sound on the `Completed` branch ONLY — never on
/// `Cancelled` (D-08 / T-05-POMO-CANCEL).
enum SegmentEnd {
    Completed,
    Cancelled,
}

impl RunCommand for PomodoroArgs {
    fn run(self) -> anyhow::Result<()> {
        // Auto-cycle is opt-in (D-06). Bare / `[MINUTES]` / `--break` / `--long-break`
        // (no `--cycles`/`--loop`) runs exactly ONE segment — byte-compatible with the
        // pre-cycle behavior.
        let auto = self.cycles.is_some() || self.loop_;
        let label = self.label.as_deref();

        // Enable raw mode FIRST, then arm the guard THE INSTANT raw mode is on,
        // BEFORE the fallible `cursor::Hide` execute! (matrix CR-01 ordering). If
        // `Hide` fails, its `?` early-returns from `run()` — but the guard already
        // exists, so its `Drop` runs `disable_raw_mode()` and the terminal is
        // restored. Were the guard armed AFTER `Hide`, a failure there would
        // early-return with raw mode still on and no guard to undo it. The guard is
        // armed ONCE here, BEFORE the outer segment loop — one continuous raw-mode
        // session across every sub-timer (D-08); it is NEVER re-enabled per segment.
        enable_raw_mode()?;
        let _guard = RawGuard;
        let mut out = std::io::stdout();
        // No EnterAlternateScreen — the in-place countdown lives on the NORMAL
        // screen (discretion); the guard restores Show + disable_raw_mode only.
        crossterm::execute!(out, cursor::Hide)?;

        // `cancelled` is set if ANY segment is cancelled. `single_completed` marks the
        // non-auto path's completion so its toast fires AFTER the guard restores the
        // terminal (preserving the pre-cycle D-07 ordering). Under auto-cycle each
        // segment's toast fires the instant that segment completes, inside the loop.
        let mut cancelled = false;
        let mut single_completed = false;

        if !auto {
            let total = resolve_duration(self.minutes, self.break_, self.long_break);
            match run_segment(total, None, label)? {
                SegmentEnd::Cancelled => cancelled = true,
                SegmentEnd::Completed => single_completed = true,
            }
        } else {
            // Auto-cycle: work → break → work … The break after work session K is a
            // 15-min long break on every 4th (break_is_long), else a 5-min short
            // break. `--cycles N` stops after N cycles; `--loop` runs until cancel.
            let mut session: u64 = 0;
            'outer: loop {
                session += 1;

                // Work segment — counter prefix ("Pomodoro 3/4" or "#3").
                let counter = session_counter(session, self.cycles);
                let work = resolve_duration(self.minutes, false, false);
                match run_segment(work, Some(&counter), label)? {
                    SegmentEnd::Cancelled => {
                        cancelled = true;
                        break 'outer;
                    }
                    SegmentEnd::Completed => self.notify("Time's up!"),
                }

                // Break segment — long on every 4th (D-06 cadence).
                let long = break_is_long(session);
                let brk = resolve_duration(None, !long, long);
                let brk_prefix = if long { "Long break" } else { "Break" };
                match run_segment(brk, Some(brk_prefix), label)? {
                    SegmentEnd::Cancelled => {
                        cancelled = true;
                        break 'outer;
                    }
                    SegmentEnd::Completed => self.notify("Break's over!"),
                }

                // Bounded stop for `--cycles`; `--loop` never breaks here.
                if let Some(n) = self.cycles {
                    if session >= n {
                        break 'outer;
                    }
                }
            }
        }

        // Restore the terminal BEFORE the final message or toast (D-07). Drop runs
        // cursor::Show + disable_raw_mode; the explicit drop makes the ordering
        // visible (the guard would otherwise drop at end of scope anyway).
        drop(_guard);
        // Move off the (now-restored) countdown line so stderr / the next prompt
        // start cleanly.
        println!();

        if cancelled {
            // Message → stderr, exit 1, and NO toast/sound fires on this path: each
            // notify above sat behind a `Completed` arm, and the single-segment
            // notify below is gated on `single_completed` (T-05-POMO-CANCEL).
            eprintln!("cancelled");
            std::process::exit(1);
        }

        // Non-auto COMPLETION → fire the Windows toast AFTER the guard restored the
        // terminal (D-07 ordering, backward compatible). A toast that fails to show
        // after a COMPLETED timer must NOT fail the run (RESEARCH A3 / D-07
        // "completion → exit 0") — `notify` is best-effort.
        if single_completed {
            self.notify("Time's up!");
        }
        Ok(())
    }
}

impl PomodoroArgs {
    /// Fire the completion notification for one COMPLETED segment (POMO-V2-01): a
    /// Windows toast carrying the `--label` (best-effort — a toast that fails to show
    /// never fails the run, D-07). Called ONLY on a `Completed` branch, never on
    /// cancel (T-05-POMO-CANCEL).
    fn notify(&self, body: &str) {
        fire_toast(body, self.label.as_deref());
    }
}

/// Show the completion toast (D-09). Title is `Pomodoro` (with the `--label`
/// appended when present, carrying the label into the toast per POMO-V2-01); `body`
/// is the segment-appropriate message. `POWERSHELL_APP_ID` needs no AUMID
/// registration. Best-effort: a failed `show()` is swallowed so it can never fail a
/// completed timer and never corrupts the continuous raw-mode countdown display.
fn fire_toast(body: &str, label: Option<&str>) {
    let title = match label {
        Some(l) => format!("Pomodoro — {l}"),
        None => "Pomodoro".to_string(),
    };
    let _ = Toast::new(Toast::POWERSHELL_APP_ID)
        .title(&title)
        .text1(body)
        .show();
}

/// Run ONE timed segment: the in-place `MM:SS` countdown + keypress tick loop
/// (POMO-V2-01). This is the pre-cycle single-timer body factored out so the outer
/// cycle loop reuses it verbatim (D-08). Assumes raw mode is ALREADY enabled and the
/// cursor hidden — the caller arms the `RawGuard` ONCE and this function never
/// enables/disables raw mode. `prefix` is the session counter / break label printed
/// before the digits; `user_label` is the optional `--label` (color-gated like the
/// digits). Returns whether the segment `Completed` or was `Cancelled`; it does NOT
/// fire the toast/sound — the caller does that on the `Completed` branch only.
fn run_segment(
    total: Duration,
    prefix: Option<&str>,
    user_label: Option<&str>,
) -> anyhow::Result<SegmentEnd> {
    let mut out = std::io::stdout();
    let end = Instant::now() + total;
    loop {
        let remaining = end.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(SegmentEnd::Completed); // COMPLETION — loop fell through.
        }

        // Render the countdown line IN PLACE: return to column 0, clear the line,
        // print the optional counter prefix + the MM:SS digits + the optional label,
        // then ONE flush per tick (the matrix flush-once discipline — NEVER per
        // character, D-07). The digits and the label are color-gated on
        // is_color_on() so piped output is byte-identical minus ANSI (D-00).
        let mmss = fmt_mmss(remaining.as_secs());
        queue!(out, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
        if let Some(p) = prefix {
            queue!(out, Print(p), Print(" "))?;
        }
        if is_color_on() {
            queue!(
                out,
                Print(mmss.truecolor(DIGITS_RGB.0, DIGITS_RGB.1, DIGITS_RGB.2))
            )?;
        } else {
            queue!(out, Print(&mmss))?;
        }
        if let Some(l) = user_label {
            queue!(out, Print(" "))?;
            if is_color_on() {
                queue!(out, Print(l.truecolor(LABEL_RGB.0, LABEL_RGB.1, LABEL_RGB.2)))?;
            } else {
                queue!(out, Print(l))?;
            }
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
                    return Ok(SegmentEnd::Cancelled); // CANCEL — exit 1, NO toast.
                }
            }
        }
    }
}

/// The work-session counter shown in the countdown line (POMO-V2-01/D-06): under
/// `--cycles N` (`Some(total)`) it is `Pomodoro {session}/{total}`; under `--loop`
/// (`None`) it is `Pomodoro #{session}`. `session` is 1-based. Pure → unit-tested.
fn session_counter(session: u64, total: Option<u64>) -> String {
    match total {
        Some(n) => format!("Pomodoro {session}/{n}"),
        None => format!("Pomodoro #{session}"),
    }
}

/// The classic Pomodoro cadence (POMO-V2-01/D-06): every 4th break is the 15-minute
/// long break, all others are 5-minute short breaks. `break_number` is 1-based (the
/// Kth break taken, which equals the work-session number it follows). Pure →
/// unit-tested. `break_number == 0` never occurs (the first break is number 1).
fn break_is_long(break_number: u64) -> bool {
    break_number != 0 && break_number.is_multiple_of(4)
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

    /// The classic cadence (D-06): every 4th break is the 15-min long break, all
    /// others are 5-min short breaks. 1-based break numbers 1..3 are short, 4 is
    /// long, 5..7 short, 8 long, … — the every-4th rule.
    #[test]
    fn break_is_long_every_fourth() {
        assert!(!break_is_long(1), "break 1 is a short break");
        assert!(!break_is_long(2), "break 2 is a short break");
        assert!(!break_is_long(3), "break 3 is a short break");
        assert!(break_is_long(4), "break 4 is the long break");
        assert!(!break_is_long(5), "break 5 is a short break");
        assert!(!break_is_long(6), "break 6 is a short break");
        assert!(!break_is_long(7), "break 7 is a short break");
        assert!(break_is_long(8), "break 8 is the long break");
        assert!(break_is_long(12), "break 12 is the long break");
        assert!(
            !break_is_long(0),
            "break 0 never occurs (breaks are 1-based) and must not be treated as long"
        );
    }

    /// The session counter (D-06): `Pomodoro N/M` under `--cycles M`, `Pomodoro #N`
    /// under `--loop` (no total). `session` is 1-based.
    #[test]
    fn session_counter_cycles_vs_loop() {
        assert_eq!(
            session_counter(3, Some(4)),
            "Pomodoro 3/4",
            "--cycles shows N/M"
        );
        assert_eq!(
            session_counter(1, Some(1)),
            "Pomodoro 1/1",
            "a single --cycles 1 still shows 1/1"
        );
        assert_eq!(
            session_counter(3, None),
            "Pomodoro #3",
            "--loop shows #N (no total)"
        );
        assert_eq!(session_counter(1, None), "Pomodoro #1");
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
