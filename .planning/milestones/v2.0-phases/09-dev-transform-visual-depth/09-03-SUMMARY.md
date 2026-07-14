---
phase: 09-dev-transform-visual-depth
plan: 03
subsystem: cli-commands
tags: [lolcat, animate, crossterm, raw-mode, alt-screen, raii, gradient, truecolor, is-terminal, human-verify, sc3, sc4]

# Dependency graph
requires:
  - phase: 07-spine-rollout
    provides: "the is_color_on()/is_json_on()/is_clip_on() spine consulted by the animate AND-gate; the SC4 display-only omission contract (D-21) lolcat already honored"
  - phase: 08-filesystem-depth
    provides: "the matrix RAII terminal loop (RawGuard / event::poll-as-frame-timer / KeyEventKind::Press-only is_quit) reused VERBATIM as the --animate template"
provides:
  - "box lolcat --animate — a bounded alternate-screen ~20-FPS rainbow loop reusing matrix's D-10 RAII teardown, gated behind a MANDATORY is_terminal() && is_color_on() && !is_json_on() && !is_clip_on() AND-gate (SC3/T-09-03-PIPE); off-gate degrades to the static one-pass render"
  - "rgb_at(phase, freq) — the SINGLE parameterized gradient function feeding the static render, every animated frame, and the persisted final frame (D-11 single-color-path)"
  - "--freq (default 0.1) / --seed (default 0.0) / --duration (default 3s; 0 = run until quit key) governing BOTH the animated and static paths"
affects: [11-meta-commands-completions]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "reuse of the matrix RAII loop template VERBATIM (RawGuard armed BEFORE the fallible alt-screen execute!; single per-frame flush; event::poll(50ms) as frame-timer-AND-input-gate; Press-only is_quit) so a second terminal loop introduces no new teardown risk (D-10)"
    - "MANDATORY is_terminal() conjunct in the raw-mode gate — is_color_on() ALONE is insufficient because CLICOLOR_FORCE can force color true on a pipe; is_terminal() is the load-bearing guard that keeps raw-mode escapes off a non-TTY stdout (SC3)"
    - "one pure crate-free gradient helper (rgb_at) unit-tested without a terminal; the on-screen smoothness + clean restore are the sole human-verify gate"

key-files:
  created: []
  modified:
    - "src/commands/lolcat/mod.rs"
    - "tests/lolcat.rs"

key-decisions:
  - "rgb_at parameterized from rgb_at(phase) to rgb_at(phase, freq) — the hardcoded f=0.1 became the --freq default; both the static render and the animated frames route through this single function so --freq/--seed shift both identically (D-11)"
  - "the raw-mode/alt-screen loop is entered ONLY behind a four-conjunct AND-gate (animate && is_terminal() && is_color_on() && !is_json_on() && !is_clip_on()); the is_terminal() conjunct is load-bearing (CLICOLOR_FORCE bypass of is_color_on() alone) — off-gate falls through to render_gradient, the byte-identical-minus-ANSI degrade path"
  - "RawGuard armed the INSTANT enable_raw_mode() returns, BEFORE the fallible EnterAlternateScreen execute!, so its Drop restores cooked mode + leaves alt-screen even if setup fails or the loop early-returns via ? — the real restore backstop under release panic=abort (matrix D-10 verbatim)"
  - "on exit the loop explicitly LeaveAlternateScreen + disable_raw_mode THEN reprints one final render_gradient frame + '\\n' to the NORMAL buffer so the colored rainbow PERSISTS after the animation ends"
  - "ANIMATE_PHASE_STEP (2.0) and SPREAD (3.0) are tuned flow-speed/diagonal consts (NOT flags), Discretion within D-11; DEFAULT_DURATION_SECS=3 resolves RESEARCH Q2"

patterns-established:
  - "a second RAII terminal loop can be added at ZERO new teardown risk by copying the matrix RawGuard/poll/Press-only-quit template verbatim rather than re-deriving it"
  - "human-verify checkpoint scoping: automate everything testable (rgb_at bounds/spacing/freq-seed effect, the AND-gate source assertion, piped-degrades-to-static byte-identity, non-hanging smoke) and gate ONLY the genuinely-visual residue (smoothness, clean raw-mode restore, persisted frame) on a human in a real PS7 TTY"

requirements-completed: [LOL-V2-01]

# Metrics
duration: ~15min (code 2026-06-28) + human-verify gate cleared 2026-07-14
completed: 2026-07-14
---

# Phase 9 Plan 03: Animated lolcat (`--animate`) Summary

**`box lolcat --animate` — the phase headline and its one PS7 human-verify gate. A bounded alternate-screen ~20-FPS rainbow loop that reuses `matrix`'s D-10 RAII teardown VERBATIM, persists one final colored frame after exit, and degrades to the existing static one-pass renderer behind a MANDATORY `is_terminal()` AND-gate whenever stdout is not a real TTY (SC3/SC4). `rgb_at` is now parameterized to `(phase, freq)` so `--freq`/`--seed` govern BOTH the animated and static paths through one gradient function (D-11).**

## Performance

- **Code duration:** ~15 min (tasks 1-2, session 2026-06-28)
- **Human-verify gate cleared:** 2026-07-14 (user "approved")
- **Tasks:** 3 (2 automated code/test + 1 blocking human-verify checkpoint)
- **Files modified:** 2 (`src/commands/lolcat/mod.rs`, `tests/lolcat.rs`)
- **No Cargo.toml change** — `crossterm`/`owo-colors`/`strip-ansi-escapes` were all already in the manifest.

## Accomplishments
- **`rgb_at` parameterized (Task 1):** `rgb_at(phase: f64) -> (u8,u8,u8)` became `rgb_at(phase, freq)` — the hardcoded `f = 0.1` is now the `--freq` default. The classic lolcat sine-wave (each channel `sin(freq*phase + offset)*127 + 128`, 120°/240° spaced) is the SINGLE gradient path shared by `render_gradient` (static + degrade + persisted frame) and `render_frame` (per animated frame).
- **`--animate` loop (Task 1):** `run_animate` copies `matrix`'s template verbatim — `enable_raw_mode()?` then `let _guard = RawGuard;` armed BEFORE the fallible `execute!(EnterAlternateScreen, Hide)?`; each frame redraws in place via `MoveTo(0, line_idx)` + `rgb_at`, flushes EXACTLY once, then `event::poll(50ms)` doubles as the ~20-FPS timer and the `KeyEventKind::Press`-only `is_quit` (q / Esc / Ctrl+C) gate. Runs to an `Instant` `--duration` deadline (`0` = until a quit key). On exit: `LeaveAlternateScreen` + `disable_raw_mode` THEN one final `render_gradient` frame to the normal buffer so the rainbow PERSISTS.
- **The MANDATORY AND-gate (Task 1, SC3/T-09-03-PIPE):** the loop is entered ONLY when `animate && std::io::stdout().is_terminal() && is_color_on() && !is_json_on() && !is_clip_on()`. `is_terminal()` is load-bearing — `CLICOLOR_FORCE` can force `is_color_on()` true on a pipe, and a raw-mode escape on a pipe is the forbidden hazard. Off-gate falls through to the static renderer (byte-identical minus ANSI). Incoming ANSI is stripped unconditionally on BOTH paths via `strip_ansi_escapes::strip_str` (D-13 / T-04L-01).
- **Automatable tests (Task 2):** `tests/lolcat.rs` proves the LOL-V2-01 subset a human can't be asked to eyeball — piped/non-TTY `--animate` (both the arg form and stdin-piped form) exits 0, emits no `0x1b` byte, never emits the alternate-screen escape, and is byte-identical to the static render; the piped invocation does NOT hang on `event::poll` without a TTY; and `--seed`/`--freq` change the static output. In-source unit tests lock `rgb_at` channel bounds, phase-0 floor, 120° spacing, the `--freq`/`--seed` effect, the `strip_str` ANSI/CR removal, and the Press-only `is_quit` key-set.
- **PS7 human-verify (Task 3, cleared):** a human confirmed in a real PowerShell 7 terminal — smooth ~20-FPS rainbow, clean exit on q / Esc / Ctrl+C with no stuck raw mode and the cursor restored, a persisted final colored frame, a visible `--freq`/`--seed` gradient change, `--duration 0` running until a keypress, and a static (no-escape, non-hanging) degrade when piped. Approved 2026-07-14.

## Task Commits

1. **Task 1: parameterize rgb_at + bounded alt-screen animate loop with TTY/color AND-gate** — `65bac81` (feat, 2026-06-28)
2. **Task 2: automatable lolcat tests — piped degrades to static, non-hanging smoke, byte-identity** — `3cc7ac7` (test, 2026-06-28)
3. **Task 3: PS7 human-verify — animated lolcat smoothness + clean terminal restore** — no code commit; blocking human-verify checkpoint, approved by the user 2026-07-14.

_Task 1 is `tdd="true"`: the pure `rgb_at`/`is_quit` seams and their inline `#[cfg(test)]` unit tests were authored together and verified RED→GREEN, then committed atomically with the feature (Rust inline tests share the source file — the 09-01/09-02 precedent). Task 2's terminal-free integration tests were committed separately as `test(09-03)`._

## Files Created/Modified
- `src/commands/lolcat/mod.rs` — `rgb_at(phase, freq)` parameterization; `LolcatArgs` gains `--animate`/`--freq`/`--seed`/`--duration`; the `run_animate` alt-screen loop + `render_frame` + the `RawGuard`/`is_quit` matrix-verbatim teardown; the four-conjunct AND-gate in `run()`; `SPREAD`/`ANIMATE_PHASE_STEP`/`DEFAULT_DURATION_SECS` consts. Added `crossterm`/`IsTerminal`/`is_clip_on` imports (the module did not import crossterm before this plan).
- `tests/lolcat.rs` — the automatable `--animate` subset: piped-degrades-to-static byte-identity (arg + stdin forms), the no-`0x1b`/no-alt-screen-escape SC3 scan, the non-hanging smoke, and the `--seed`/`--freq` static-output-differs assertions.

## Decisions Made
- **Single gradient path (D-11 upheld):** rather than a second animate-only gradient, `rgb_at` was parameterized so `render_gradient` and `render_frame` call the identical function — `--freq`/`--seed` therefore provably affect both the static and animated renders, and the static path IS the degrade path (no drift possible).
- **`is_terminal()` is a required conjunct, not a nicety:** the gate is four-way (`animate && is_terminal() && is_color_on() && !is_json_on() && !is_clip_on()`). `is_color_on()` alone would let a `CLICOLOR_FORCE=1 | pipe` invocation enter raw mode and corrupt the stream — the `is_terminal()` conjunct is the SC3 backstop. The automatable test asserts the piped run never emits the alt-screen escape, proving the branch is unreachable off-TTY.
- **RawGuard armed before the fallible setup (D-10 / CR-01):** `let _guard = RawGuard;` is placed immediately after `enable_raw_mode()?` and BEFORE `execute!(EnterAlternateScreen, ...)?`, so a failure of the alt-screen setup — or any `?` early-return / unwind inside the loop — still restores cooked mode via Drop under the release `panic = "abort"` profile.
- **Final-frame persist:** the loop tears down (leave alt-screen, disable raw mode) BEFORE reprinting the last frame, so the persisted rainbow lands in the normal buffer at column 0 in cooked mode.

## Deviations from Plan

None — plan executed as written. The plan explicitly scoped Task 3 to a human gate (smoothness + clean restore cannot be proven by automated test alone); that gate was presented and the human approved all seven PS7 checks.

## Issues Encountered
None. The matrix-verbatim template reuse meant the terminal-loop correctness (RAII restore, single-flush, Press-only quit) carried over without re-derivation; the automatable tests were green from first run and the full suite stayed green.

## User Setup Required
None — no external service configuration required. `--animate` requires a real TTY (PowerShell 7); off-TTY it silently degrades to the static render.

## Next Phase Readiness
- LOL-V2-01 complete: code merged (tasks 1-2, session 2026-06-28) and the blocking PS7 human-verify gate cleared 2026-07-14. Full `cargo test` green and `cargo clippy --all-targets -- -D warnings` clean on integrated main.
- This was Phase 9's SOLE human-verify gate. With 09-01 (dev-transform depth) and 09-02 (matrix/qr/ascii visual depth) complete, all 9 Phase-9 requirements are delivered.
- Phase 11 (`completions`) will generate against the live final `Cli`, which now includes lolcat's `--animate`/`--freq`/`--seed`/`--duration`.

## Self-Check

- Created files: `.planning/phases/09-dev-transform-visual-depth/09-03-SUMMARY.md` (this file).
- Commits verified present: `65bac81` (feat, Task 1), `3cc7ac7` (test, Task 2). Task 3 is a human-verify checkpoint with no code commit.

---
*Phase: 09-dev-transform-visual-depth*
*Completed: 2026-07-14*
