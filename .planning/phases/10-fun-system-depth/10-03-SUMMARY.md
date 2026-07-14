---
phase: 10-fun-system-depth
plan: 03
subsystem: cli-commands
tags: [pomodoro, cycles, loop, label, sound, messagebeep, win32, ffi, raw-mode, raii, cadence, display-only, sc4, backward-compatible]

# Dependency graph
requires:
  - phase: 05-windows-platform-integration
    provides: "the pomodoro POMO-01 single-timer: the RawGuard RAII teardown, resolve_duration overflow-clamped resolver, is_cancel Press-only filter, and the completion toast — all reused/extended VERBATIM here"
  - phase: 08-filesystem-depth
    provides: "the committed `windows 0.61` dependency (D-2) and its #[cfg(windows)] Win32-FFI-with-non-Windows-fallback pattern (du/dupes) that `beep()` mirrors"
provides:
  - "box pomodoro --cycles N — N work sessions each followed by a break, every 4th break the 15-min long break, over ONE continuous raw-mode session (single RawGuard, D-08)"
  - "box pomodoro --loop — work/break indefinitely until cancel"
  - "box pomodoro --label <text> — annotates the countdown line (color-gated) and the completion toast title"
  - "box pomodoro --sound — a best-effort Win32 MessageBeep(MB_OK) on each session COMPLETION only, composing with the toast"
  - "break_is_long(n) / session_counter(session, total) — pure, unit-tested cadence + counter seams; run_segment — the factored single-segment tick loop the outer cycle loop reuses"
affects: [11-meta-commands-completions]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "outer segment loop over a SINGLE RawGuard armed once (D-08) — the pre-cycle tick loop factored into run_segment and driven N (or ∞) times without ever re-enabling raw mode"
    - "minimal, documented unsafe Win32 FFI (MessageBeep) behind a #[cfg(windows)] fn with a no-op non-Windows fallback — the du/dupes cfg-gating pattern extended to a UI/sound call"
    - "reuse of resolve_duration for the cadence durations (work / short break / long break) so the MAX_MINUTES overflow clamp covers every segment; a separate pure break_is_long owns the every-4th rule"
    - "opt-in depth: bare/[MINUTES] stays byte-compatible (exactly one segment); the counter/label/sound surface only activates under the new flags"

key-files:
  created: []
  modified:
    - "src/commands/pomodoro/mod.rs"
    - "Cargo.toml"
    - "tests/pomodoro.rs"

key-decisions:
  - "the single RawGuard is armed ONCE before the OUTER segment loop (D-08); the pre-cycle tick body became run_segment(total, prefix, user_label) -> SegmentEnd and is driven per work/break segment — raw mode is never enabled/disabled per segment"
  - "cadence via a pure break_is_long(break_number) (every 4th break is long) PLUS resolve_duration reused for the actual work/short/long durations (keeps the MAX_MINUTES clamp on every segment); session_counter renders 'Pomodoro N/M' (--cycles) vs 'Pomodoro #N' (--loop)"
  - "--cycles bound 1..=1000 via RangedU64ValueParser (same pattern as [MINUTES]) and conflicts_with --loop; --loop is the intentionally-unbounded, always-cancellable escape hatch (T-10-03-CYCLES mitigation)"
  - "MessageBeep needs BOTH Win32_System_Diagnostics_Debug (the fn) AND Win32_UI_WindowsAndMessaging (MB_OK) — the plan named only the latter; verified against windows-0.61.3 source that neither feature pulls the other (Rule 3 auto-fix)"
  - "beep() is a minimal #[cfg(windows)] unsafe FFI (only the compile-time MB_OK constant crosses the boundary, T-10-03-BEEP) with a no-op non-Windows fallback; it is called ONLY inside PomodoroArgs::notify on a Completed branch, gated on self.sound — never reachable from the cancel early-return"
  - "fire_toast is now silently best-effort (the original per-run 'note: could not show toast' eprintln was dropped) so a failed toast never staircases/spams the continuous multi-segment countdown; the run still exits 0 (D-07 'completion → exit 0' upheld)"
  - "pomodoro stays display-only (SC4): run() adds no is_json_on/emit_json; the matrix_pomodoro_have_no_spine_calls source-scan guard stays green"

requirements-completed: [POMO-V2-01, POMO-V2-02]

# Metrics
duration: ~11min
completed: 2026-07-14
---

# Phase 10 Plan 03: Pomodoro auto-cycle + `--sound` Summary

**`box pomodoro` gains an opt-in auto-cycle model — `--cycles N` / `--loop` run work→break segments over ONE continuous raw-mode session (single `RawGuard`, D-08), with the classic every-4th-break long-break cadence, a session counter (`Pomodoro 3/4` or `#3`), and a color-gated `--label` carried into the completion toast — plus `--sound`, a best-effort Win32 `MessageBeep(MB_OK)` on each session COMPLETION only (never on cancel), composing with the toast with zero audio stack. Bare `box pomodoro [MINUTES]` stays byte-compatible, and the command remains display-only (SC4 — no `--json`/`--clip`).**

## Performance

- **Duration:** ~11 min (2 tasks, both `tdd="true"`)
- **Tasks:** 2 (both automated code/test)
- **Files modified:** 3 (`src/commands/pomodoro/mod.rs`, `Cargo.toml`, `tests/pomodoro.rs`)
- **No new crate** — `--sound` adds two feature modules to the already-committed `windows 0.61` dep (D-2); `cargo tree -i windows` stays a single `0.61.3`.

## Accomplishments
- **Auto-cycle segment loop (Task 1, POMO-V2-01):** `run()` now arms the single `RawGuard` ONCE before an OUTER segment loop (D-08). The pre-cycle tick body was factored into `run_segment(total, prefix, user_label) -> SegmentEnd` and is driven per work/break segment; raw mode is never re-enabled per segment. Bare / `[MINUTES]` / `--break` / `--long-break` (no `--cycles`/`--loop`) runs exactly ONE segment — byte-compatible with the shipped behavior.
- **Cadence + counter (Task 1):** pure, unit-tested `break_is_long(n)` (long break on every 4th, `is_multiple_of(4)`) sits alongside `resolve_duration`, which is reused for the work (25/`[MINUTES]`), short-break (5), and long-break (15) durations so the `MAX_MINUTES` overflow clamp covers every segment. `session_counter(session, total)` renders `Pomodoro N/M` under `--cycles` and `Pomodoro #N` under `--loop`; break segments show `Break` / `Long break`.
- **`--cycles` / `--loop` / `--label` (Task 1):** `--cycles` is bounded `1..=1000` via the same `RangedU64ValueParser` pattern as `[MINUTES]` and `conflicts_with = "loop_"` (both → clap exit 2, T-10-03-CYCLES). `--label` threads into the in-place `MM:SS` redraw color-gated like the digits (`is_color_on()`, D-00) and into the completion toast title. Cancel (Ctrl+C / q / Esc) at ANY segment restores the terminal and exits 1 with NO toast/sound; each completed segment fires its toast; the full set exits 0.
- **`--sound` completion beep (Task 2, POMO-V2-02):** `notify()` fires a best-effort Win32 `MessageBeep(MB_OK)` alongside the toast when `--sound` is set, on the completion branch only. `beep()` is a minimal `#[cfg(windows)]` `unsafe` FFI — the ONLY argument is the compile-time `MB_OK` constant (T-10-03-BEEP) — with a no-op `#[cfg(not(windows))]` fallback matching the du/dupes cfg-gating pattern. The beep is never reachable from the cancel early-return.
- **Cargo.toml (Task 2):** added `Win32_System_Diagnostics_Debug` (where `MessageBeep` lives) AND `Win32_UI_WindowsAndMessaging` (where `MB_OK`/`MESSAGEBOX_STYLE` live) to the existing `windows` dep — no new crate, and `cargo tree -i windows` stays a single `0.61.x` (D-2 unification held).
- **Tests:** in-source unit tests lock `break_is_long` (every-4th, plus the 0-guard) and `session_counter` (both `--cycles` and `--loop` forms). `tests/pomodoro.rs` gains the `--cycles` out-of-range → exit 2, `--cycles`+`--loop` conflict → exit 2, the `--help` flag surface, and a `--sound` wired-and-composes-with-cancel smoke. Full suite green (209 unit + all integration files, 0 failures); `cargo clippy --all-targets -- -D warnings` clean; the `matrix_pomodoro_have_no_spine_calls` SC4 source guard stays green.

## Task Commits

1. **Task 1: auto-cycle segment loop + cadence + session counter + `--label` (POMO-V2-01)** — `3a2eac1` (feat)
2. **Task 2: `--sound` completion beep via Win32 `MessageBeep` (POMO-V2-02)** — `c3cd652` (feat)

_Both tasks are `tdd="true"`: the pure `break_is_long`/`session_counter` seams and their inline `#[cfg(test)]` unit tests were authored together and verified RED→GREEN, then committed atomically with the feature (Rust inline tests share the source file — the module's own precedent). Terminal-free integration assertions ride in the same task commits._

## Files Created/Modified
- `src/commands/pomodoro/mod.rs` — `PomodoroArgs` gains `--cycles`/`--loop`/`--label`/`--sound`; `run()` refactored to the single-guard outer segment loop; new `run_segment`/`SegmentEnd`, `session_counter`, `break_is_long`, `fire_toast`, `PomodoroArgs::notify`, and the `#[cfg(windows)]` `beep()` + non-Windows fallback; `MAX_CYCLES`/`LABEL_RGB` consts; module doc updated (Auto-cycle + Sound sections; toast now documented as silently best-effort).
- `Cargo.toml` — `windows 0.61` feature list expanded to include `Win32_System_Diagnostics_Debug` + `Win32_UI_WindowsAndMessaging` (MessageBeep + MB_OK); comment updated to explain the two-namespace requirement.
- `tests/pomodoro.rs` — `--cycles` bound (exit 2), `--cycles`/`--loop` conflict (exit 2), `--help` flag-surface, and `--sound` wired/composes-with-cancel tests.

## Decisions Made
- **Single continuous raw-mode session (D-08 upheld):** the guard is armed once and dropped once; `run_segment` never touches raw mode. `resolve_duration` is reused for every segment's duration so the overflow clamp is uniform, and `break_is_long` is the only new cadence primitive.
- **Two `windows` features, not one (Rule 3):** the plan/PATTERNS said add `Win32_UI_WindowsAndMessaging`; verified against `windows-0.61.3` source that `MessageBeep` is only in `Win32::System::Diagnostics::Debug` (a win32metadata namespace quirk) and that neither feature transitively enables the other, so both are required for the beep to resolve. The build now compiles the beep on the msvc target.
- **Toast is silently best-effort:** the original per-run `note: could not show toast` eprintln was removed so a failing toast can't staircase or spam the continuous multi-segment countdown; the "completion → exit 0, toast never fails the run" contract (D-07) is preserved, and no test asserted the note.
- **Display-only (SC4):** `run()` adds no `is_json_on`/`emit_json`; the doc-comment mentions (stripped by the source-scan guard) are the only occurrences.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Worktree spawned from a stale base (phase 06)**
- **Found during:** execution start (before Task 1).
- **Issue:** the worktree branch was created from `986c841` (phase 06 complete), which lacked the phase-07/08/09 source (~5350 lines), the phase-10 plan artifacts (`10-03-PLAN.md`, `10-PATTERNS.md`, `10-CONTEXT.md`), and — critically — the explicit `windows 0.61` dependency the plan's "add a feature to the EXISTING windows dep" assumption relies on (that dep was added on `main` in phase 08). Editing the stale source would have regressed phases 07-09 on merge-back.
- **Fix:** the worktree branch had ZERO commits and was a direct ancestor of `main` (`d2802e3`), so a non-destructive `git merge --ff-only main` fast-forwarded it to the correct base (nothing lost, no divergence). This is the only safe path — a fast-forward that moves an empty branch forward, not a reset onto a protected ref.
- **Files modified:** none (base correction only, before any task edits).
- **Commit:** n/a (pre-work base fast-forward; the two task commits sit on top of `d2802e3`).

**2. [Rule 3 - Blocking issue] `MessageBeep` requires a second `windows` feature**
- **Found during:** Task 2 (before wiring the FFI).
- **Issue:** the plan/PATTERNS specified adding only `Win32_UI_WindowsAndMessaging`, but `MessageBeep` lives in `Win32::System::Diagnostics::Debug` and is gated by `Win32_System_Diagnostics_Debug`; without it the build would not resolve the symbol.
- **Fix:** added BOTH features to the existing `windows` dep. No new crate; `cargo tree -i windows` stays a single `0.61.3`.
- **Files modified:** `Cargo.toml`.
- **Commit:** `c3cd652`.

**3. [Interpretation] Task 2 "beep after `drop(_guard)`" reconciled with D-08 per-segment notify**
- **Found during:** Task 2.
- **Issue:** Task 2's acceptance parenthetical describes the beep sitting "after `drop(_guard)`", which conflicts, for the multi-segment case, with D-08's authoritative "one continuous raw-mode session (guard armed once) where EACH completed segment fires its toast/sound".
- **Resolution:** the single `beep()` call site lives inside `PomodoroArgs::notify`, invoked on a `SegmentEnd::Completed` branch only, gated on `self.sound`, and never reachable from the cancel early-return — the PRIMARY, testable invariant. Under `--cycles`/`--loop` it necessarily fires per segment while the single guard is held (D-08); the beep is a pure Win32 call that writes nothing to the terminal, so the continuous display stays clean. Resolved in favor of the authoritative D-08 decision.
- **Files modified:** `src/commands/pomodoro/mod.rs`.
- **Commit:** `c3cd652`.

## Threat Flags

None — every surface this plan adds (`MessageBeep` unsafe FFI, the `--cycles` bound, the `--label` echo, the display-only SC4 omission) is already enumerated in the plan's `<threat_model>` STRIDE register, and no new network endpoint, auth path, file access, or schema surface was introduced.

## Known Stubs

None — every flag is fully wired (no hardcoded empty/placeholder values, no unwired data source).

## Issues Encountered
Only the two blocking issues above (stale worktree base; the second `windows` feature), both auto-resolved under Rule 3. One clippy nit (`manual_is_multiple_of`) was fixed by switching `% 4 == 0` to `.is_multiple_of(4)` while keeping the defensive `!= 0` guard. The full suite was green after each fix.

## User Setup Required
None — no external service or configuration. `--sound` uses the Windows default system sound via `MessageBeep`; on a non-Windows host it is a silent no-op.

## Next Phase Readiness
- POMO-V2-01 and POMO-V2-02 complete: `--cycles`/`--loop`/`--label`/`--sound` are on the live `PomodoroArgs`. Phase 11 (`completions`) will generate against this final surface.
- pomodoro remains display-only (no `--json`/`--clip`) — SC4 intact.

## Self-Check

- Created file present: `.planning/phases/10-fun-system-depth/10-03-SUMMARY.md` (this file).
- Commits verified present on the worktree branch: `3a2eac1` (feat, Task 1), `c3cd652` (feat, Task 2), both atop `d2802e3`.

## Self-Check: PASSED

All claimed files exist (`10-03-SUMMARY.md`, `src/commands/pomodoro/mod.rs`, `tests/pomodoro.rs`, `Cargo.toml`) and both task commits (`3a2eac1`, `c3cd652`) are present on the worktree branch. Full `cargo test` green (209 unit + all integration files), `cargo clippy --all-targets -- -D warnings` clean.

---
*Phase: 10-fun-system-depth*
*Completed: 2026-07-14*
