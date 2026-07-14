---
phase: 05-windows-platform-integration
plan: 03
subsystem: cli
tags: [pomodoro, countdown, raw-mode, rawguard, poll-as-timer, winrt-toast, tauri-winrt-notification]

# Dependency graph
requires:
  - phase: 01-foundation-flatten
    provides: "RunCommand trait, 0/1/2 exit policy, anyhow error→exit-1 mapping in main(), is_color_on() gate"
  - phase: 04-terminal-visuals
    plan: 04
    provides: "the matrix RawGuard RAII teardown + event::poll-as-timer + KeyEventKind::Press-only filter + KeyEvent test harness — reused WHOLESALE"
  - phase: 05-windows-platform-integration
    plan: 02
    provides: "the registry slice (cli.rs unit-variant→Args swap, main.rs not_implemented arm removal, commands/mod.rs pub mod) that this plan mirrors for pomodoro"
provides:
  - "box pomodoro: blocking foreground raw-mode in-place MM:SS countdown (default 25 / --break 5 / --long-break 15 / positional [MINUTES] override); completion → Windows toast + exit 0; Ctrl+C/q/Esc → 'cancelled' to stderr + exit 1, NO toast; terminal restored on every path incl. panic via RAII RawGuard (POMO-01)"
  - "tauri-winrt-notification 0.7.2 dependency (maintained fork of winrt-notification, D-09; POWERSHELL_APP_ID — no AUMID registration)"
  - "pure resolve_duration / fmt_mmss / is_cancel seams: countdown duration resolution (D-08 + long-break-wins tie-break), MM:SS format, and the Press-only cancel predicate are unit-tested without a terminal"
affects: [05-04-weather]

# Tech tracking
tech-stack:
  added: ["tauri-winrt-notification 0.7.2 (Windows WinRT toast; owned-self builder; Toast::POWERSHELL_APP_ID needs no AUMID registration; brings windows 0.61 + quick-xml + windows-version on msvc)"]
  patterns:
    - "Matrix raw-mode family (D-07): pomodoro is the second command (after matrix) to own terminal raw-mode state — reuses the RawGuard struct/Drop, the guard-armed-pre-execute! CR-01 ordering, the event::poll-as-timer loop, and the KeyEventKind::Press-only cancel filter wholesale; differs only in keeping the NORMAL screen (no EnterAlternateScreen → Drop restores Show + disable_raw_mode only) and using poll(min(1s, remaining)) for an exact final partial second (Pitfall POMO-2)"
    - "Completion-only side effect (T-05-POMO-CANCEL): the toast show() sits AFTER the `if cancelled { eprintln + exit(1) }` guard, so the cancel path can never fire it — auto-verifiable by code inspection (D-07)"
    - "Toast-failure is non-fatal on completion (RESEARCH A3 / D-07): a show() Err after a COMPLETED timer logs a note to stderr and still exits 0 — the successful work is not undone by a cosmetic toast failure"

key-files:
  created:
    - "src/commands/pomodoro/mod.rs"
    - "tests/pomodoro.rs"
  modified:
    - "Cargo.toml (tauri-winrt-notification dep)"
    - "Cargo.lock (tauri-winrt-notification + windows 0.61 + quick-xml + windows-version transitives)"
    - "src/cli.rs (Pomodoro unit variant → Pomodoro(PomodoroArgs); extended --help body)"
    - "src/main.rs (not_implemented(pomodoro) arm → args.run(); not_implemented import kept live for weather)"
    - "src/commands/mod.rs (pub mod pomodoro in alpha order)"

key-decisions:
  - "tauri-winrt-notification 0.7.2 (D-09 OVERRIDE of CLAUDE.md's winrt-notification) — resolved + compiled clean on x86_64-pc-windows-msvc on the FIRST build; NO fallback rung needed. Locked at 0.7.2 in Cargo.lock"
  - "Pomodoro keeps the NORMAL screen (no EnterAlternateScreen) for an in-place countdown — the RawGuard Drop restores cursor::Show + disable_raw_mode ONLY (no LeaveAlternateScreen), the documented matrix-family adaptation (RESEARCH:313-326)"
  - "poll(min(1s, remaining)) is BOTH the ~1s tick AND the keypress gate (D-07) — the min makes the final partial second exact instead of overshooting a fixed 1s poll (Pitfall POMO-2)"
  - "Toast is completion-ONLY (T-05-POMO-CANCEL / D-07): cancel → drop guard, eprintln cancelled, std::process::exit(1), NEVER reaches show(); verified by code inspection (show() at line 188 sits after the cancel guard at line 174-178)"
  - "Both-flags tie-break (Claude discretion within D-08): --long-break wins over --break when both are set — deterministic and unit-tested (resolve_duration_long_break_beats_break)"
  - "Countdown-digit color is gated on is_color_on() (D-00) — DIGITS_RGB truecolor when on, plain label otherwise; byte-identical-minus-ANSI when piped (pomodoro is interactive so piping is a non-case, but the gate is honored anyway)"

patterns-established:
  - "Second member of the matrix raw-mode family: a blocking foreground command owning raw-mode state via the RAII RawGuard armed pre-execute! (CR-01), single-flush-per-tick render discipline, and the Press-only cancel filter — pomodoro reuses the matrix shape verbatim, the planner's predicted strongest analog match in the phase"
  - "Completion-only WinRT side effect with non-fatal failure: a one-shot toast fired AFTER terminal restore, on the completion path only, whose failure logs but does not fail a successfully-completed run"

requirements-completed: []   # POMO-01 implementation done + deterministic parts automated-green; human-verify (Task 3: live countdown + real toast + clean Ctrl+C/q/Esc cancel) PENDING — deferred to phase UAT

# Metrics
duration: 5min
completed: 2026-06-24
---

# Phase 5 Plan 03: box pomodoro (POMO-01) Summary

**`box pomodoro` runs a blocking foreground raw-mode in-place `MM:SS` countdown (default 25 min, `--break` 5, `--long-break` 15, positional `[MINUTES]` override) that reuses the matrix `RawGuard` + `event::poll`-as-timer + `KeyEventKind::Press`-only family wholesale: `poll(min(1s, remaining))` is BOTH the ~1s tick and the keypress gate, completion fires a Windows toast (`tauri-winrt-notification` / `POWERSHELL_APP_ID`) and exits 0, and a clean Ctrl+C / q / Esc cancels with "cancelled" to stderr + exit 1 and NO toast — the toast `show()` sitting after the cancel guard makes it completion-only by construction.**

## Performance

- **Duration:** ~5 min (implementation), plus full-suite + clippy + fmt verification
- **Started:** 2026-06-24T18:20:20Z
- **Completed:** 2026-06-24T18:25:11Z
- **Tasks:** 2 of 3 (Task 3 is a human-verify checkpoint — see "Verification Status")
- **Files modified:** 7 (2 created, 5 modified — incl. Cargo.lock)

## Accomplishments
- `box pomodoro` (default 25 min), `box pomodoro --break` (5 min), `box pomodoro --long-break` (15 min), and `box pomodoro 50` (positional override) all resolve to the correct duration (D-08) — unit-tested across all four paths plus override-beats-flag plus the long-break-wins tie-break.
- The countdown is a blocking foreground raw-mode loop on the NORMAL screen: `MoveToColumn(0)` + `Clear(CurrentLine)` + a single `out.flush()?` per tick (the matrix flush-once discipline — never per character), with `poll(min(1s, remaining))` as both the ~1s tick and the keypress gate so the final partial second is exact (Pitfall POMO-2).
- Cancel (q / Esc / Ctrl+C delivered as a `KeyEvent` `Char('c')+CONTROL` in raw mode — NOT SIGINT — filtered to `KeyEventKind::Press` only against the Windows Press+Release double-fire) drops the guard to restore the terminal, prints "cancelled" to stderr, and exits 1 — NEVER firing the toast (T-05-POMO-CANCEL).
- Completion (loop falls through when `remaining` hits zero) drops the guard, fires `Toast::new(Toast::POWERSHELL_APP_ID).title("Pomodoro").text1("Time's up!").show()`, and exits 0; a `show()` failure after a completed timer logs a note to stderr and STILL exits 0 (RESEARCH A3 / D-07).
- The RAII `RawGuard` is armed the INSTANT `enable_raw_mode()?` succeeds, BEFORE the fallible `cursor::Hide` `execute!` (matrix CR-01 ordering), so its `Drop` (cursor::Show + disable_raw_mode) restores the terminal on normal return, `?` early-return, and unwinding — the realistic failure mode for this command (T-05-POMO-WEDGE) is mitigated. The loop is panic-free (no `.unwrap()` on terminal ops) so the Drop guard is the real restore path under `panic = "abort"`.
- `tauri-winrt-notification` 0.7.2 (D-09 override of CLAUDE.md's `winrt-notification`) resolved and compiled clean on `x86_64-pc-windows-msvc` on the FIRST build — no fallback rung needed; locked at 0.7.2.
- The `pomodoro` `not_implemented` stub arm is gone; **1 Phase-5 stub remains (weather)**, and the `not_implemented` import stays live for it.

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1: Wave-0 test scaffold (pure green, behavior red)** — `59cc030` (test) — `pub mod pomodoro` (alpha order, after `passgen`/before `qr`), `PomodoroArgs { minutes, break_, long_break }` + the three pure helpers (`fmt_mmss`, `resolve_duration`, `is_cancel`), 7 in-module unit tests (MM:SS format ×3, 4 duration paths + positional override ×3 + long-break tie-break, cancel-key Press / plain-c-not / Release-Repeat-not), and `tests/pomodoro.rs` (matrix-shape `write_stdin("q")` + `.timeout(5s)` non-hanging smoke). The placeholder `RunCommand` impl `bail!`s, so the smoke test is RED; the 7 pure helper tests are GREEN immediately (real logic).
2. **Task 2: Implement pomodoro loop (GREEN)** — `eecbfa3` (feat) — `tauri-winrt-notification` dep, the real `RawGuard` + `RunCommand::run` (poll-as-timer loop, single-flush-per-tick MM:SS render, gated digit color, cancel→exit-1-no-toast, completion→toast+exit-0), and the cli.rs/main.rs registry wiring. All 7 pure tests + the smoke test green; clippy + fmt clean.

**Plan metadata:** committed separately with this SUMMARY.

_Task 3 (human-verify live countdown + real toast + clean Ctrl+C/q/Esc cancel) carries no commit — it is a manual gate deferred to phase UAT._

## Files Created/Modified
- `src/commands/pomodoro/mod.rs` — `PomodoroArgs { minutes, break_, long_break }` + `RawGuard` (Drop = Show + disable_raw_mode, NO alt-screen) + `RunCommand::run` (CR-01 guard ordering, poll-as-timer loop, single-flush MM:SS render, gated `DIGITS_RGB` color, completion-only toast) + the three pure helpers + `#[cfg(test)] mod tests` (7 cases). Module doc records D-07 (timer model / cancel-vs-completion / teardown), D-08 (durations + long-break-wins tie-break), D-09 (toast crate / POWERSHELL_APP_ID / non-fatal-on-completion).
- `tests/pomodoro.rs` — the matrix-shape non-hanging smoke: `box pomodoro 1` fed `q` on piped non-TTY stdin, bounded by `.timeout(5s)`, asserting no `"panicked"` marker and not pinning an exit code (a non-TTY may legitimately error out of raw-mode setup); header documents that the live render + real toast + interactive cancel are human-verify (Task 3).
- `Cargo.toml` — `tauri-winrt-notification = "0.7.2"` with the D-09 fork-rationale + fallback-ladder comment.
- `Cargo.lock` — `tauri-winrt-notification` 0.7.2 + `windows` 0.61.3 + `quick-xml` + `windows-version` + the windows-collections/numerics transitives pinned.
- `src/cli.rs` — `Pomodoro` unit variant → `Pomodoro(crate::commands::pomodoro::PomodoroArgs)`; the FIRST `///` line ("Run a focus timer with Windows toast notifications") is preserved verbatim so the `box --help` trycmd snapshot is unchanged, with an added multi-paragraph `--help` body documenting the modes / cancel / toast behavior.
- `src/main.rs` — `Commands::Pomodoro => Err(not_implemented("pomodoro"))` → `Commands::Pomodoro(args) => args.run()`; the `not_implemented` import is left live for the still-stubbed weather command.
- `src/commands/mod.rs` — `pub mod pomodoro;` in alpha order (after `passgen`, before `qr`).

## Decisions Made
Beyond the locked CONTEXT decisions (D-07/D-08/D-09), the Claude's-discretion items were resolved as:
- **Render format / placement:** `MM:SS` zero-padded, rendered in place on the NORMAL screen via `MoveToColumn(0)` + `Clear(CurrentLine)` + one flush per tick. No alternate screen (in-place countdown — the RESEARCH recommendation for Open Question 1).
- **Countdown-digit color:** gated on `is_color_on()` (D-00) — `DIGITS_RGB = (120, 255, 120)` truecolor when on, plain label otherwise.
- **Both-flags tie-break:** `--long-break` wins over `--break` when both are set — deterministic and unit-tested.
- **Toast wording:** title "Pomodoro", body "Time's up!" (D-09 discretion).
- **Completion newline:** a `println!()` after the guard drop moves off the restored countdown line so the next prompt / stderr start cleanly.

## Deviations from Plan

None — plan executed exactly as written. The two implementation tasks (1-2) followed the locked interfaces, the matrix analog, and the RESEARCH code example verbatim. `tauri-winrt-notification` 0.7.2 resolved on the first `x86_64-pc-windows-msvc` build (no fallback ladder rung needed), the RawGuard / poll-as-timer / Press-filter family was reused wholesale from matrix, and the registry wiring mirrored 05-02's clip slice. No bugs, no missing critical functionality, no blocking issues, no architectural changes. The one fmt fix (re-ordering `use crossterm::queue;` before `use crossterm::style::Print;` to satisfy `cargo fmt --check`) was a cosmetic import-ordering correction within Task 2, not a deviation.

## Issues Encountered
None of substance. The RED→GREEN cycle went cleanly: in RED the smoke test failed (the placeholder `RunCommand` `bail!`ed, so `box pomodoro 1` exited 1 immediately) while the 7 pure helper tests passed at once; the RED-phase dead-code warnings (`RawGuard`, `fmt_mmss`, `resolve_duration`, `is_cancel` — all consumed by the real `run()`) cleared in GREEN. The only correction during GREEN was the import-ordering fmt nit, fixed before the Task 2 commit; clippy `-D warnings` and `cargo fmt --check` are both clean post-GREEN.

## Verification Status

**Automated (all GREEN):**
- `cargo test --bin box pomodoro` — the 7 pure unit tests pass: `fmt_mmss` (00:00 / 01:05 / 25:00), `resolve_duration` (default 25 / break 5 / long-break 15 / positional override ×3 / long-break-beats-break), `is_cancel` (q+Esc+Ctrl+C on Press cancel / plain-c does NOT / Release+Repeat do NOT — the Windows double-fire filter).
- `cargo test --test pomodoro` — the smoke test passes: `box pomodoro 1` fed `q` on non-TTY stdin terminates non-hanging within the 5s bound with no panic marker.
- `cargo test` — full suite green: **141 bin-unit tests** (was 134 + 7 new pomodoro) + all integration suites + the `trycmd` `box --help` snapshot (UNCHANGED — the `Pomodoro` one-liner was preserved), **no regression**. `tests/cli.rs::stub_exits_1_to_stderr` (still pointing at `weather`) stays valid.
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt --check` — clean.
- **T-05-POMO-CANCEL verified by code inspection:** `show()` (line 188) sits AFTER the cancel guard `if cancelled { eprintln!("cancelled"); std::process::exit(1); }` (lines 174-178) — the cancel path calls `std::process::exit(1)` and can NEVER reach the toast. Toast is completion-only by construction.

**Task 3 — Human-verify (PENDING — deferred to phase UAT):**
- **POMO-01 live countdown + real toast + clean Ctrl+C/q/Esc cancel is NOT yet human-verified.** Task 3 is a `checkpoint:human-verify` (blocking, physical) gate requiring a real interactive run in PowerShell 7: (1) `box pomodoro 1` shows a live in-place `MM:SS` countdown updating without scroll/flicker; (2) on completion a real Windows toast fires and the command exits 0 with the cursor + terminal fully restored; (3) re-running and pressing Ctrl+C mid-countdown prints "cancelled" to stderr, exits 1, fires NO toast, and leaves the terminal fully restored (echo works, no stuck raw mode); (4) the same clean cancel for `q` and `Esc`; (5) `--break` (~5 min) and `--long-break` (~15 min) start at the right value. Per the phase-UAT batching plan (mirroring 05-01's deferred phone-scan, 05-02's deferred clip round-trip, and Phase 4's deferred matrix human-verify), this gate is deferred to phase-end UAT and was NOT self-approved.
- **What automated coverage already proves:** the deterministic invariants Task 3 depends on (duration resolution across all modes, MM:SS format, the Press-only cancel predicate, and the enter/exit non-hanging-no-panic contract) are all covered above; the completion-only-toast invariant (T-05-POMO-CANCEL) is verified by code inspection. Only the irreducibly-manual aspects remain for the human: the live in-place render quality (no flicker/scroll), the real WinRT toast firing on completion, the interactive Ctrl+C/q/Esc clean cancel + terminal-restore, and the no-toast-on-cancel confirmation in a live session.
- **Do NOT mark POMO-01 as human-verified until the phase UAT confirms it.**

## User Setup Required
None — no external service configuration. (`box pomodoro` is local: raw-mode terminal + a one-shot WinRT toast via `POWERSHELL_APP_ID`, no network, no secrets, no elevation, no AUMID registration.)

## Next Phase Readiness
- `box pomodoro` + `--break` / `--long-break` / `[MINUTES]` are live and fully automated-tested (deterministic parts + the non-hanging smoke green; the live countdown + real toast + interactive cancel are the deferred human-verify). The `tauri-winrt-notification` dependency and the second member of the matrix raw-mode family are established.
- The 1 remaining Phase-5 stub (weather) is untouched and independent — it swaps its own unit variant + stub arm; pomodoro shared only `cli.rs`/`main.rs`/`commands/mod.rs` with it (sequenced by wave, no same-wave file overlap). The `not_implemented` import remains live for weather; after weather lands, the import becomes dead and should be removed (per the LAST-phase note in 05-PATTERNS:183).
- **Open gate:** POMO-01 live countdown + real toast + clean Ctrl+C/q/Esc cancel human-verify pending at phase UAT (see above). Do not mark POMO-01 as human-verified until then.

## Self-Check: PASSED

- `src/commands/pomodoro/mod.rs` — FOUND
- `tests/pomodoro.rs` — FOUND
- `.planning/phases/05-windows-platform-integration/05-03-SUMMARY.md` — FOUND
- Commit `59cc030` (Task 1, test) — FOUND
- Commit `eecbfa3` (Task 2, feat) — FOUND

---
*Phase: 05-windows-platform-integration*
*Completed: 2026-06-24*
