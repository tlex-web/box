---
phase: 04-terminal-visuals
plan: 04
subsystem: ui
tags: [crossterm, terminal, raw-mode, animation, katakana, owo-colors, rand, RAII]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: RunCommand trait, clap-derive command registry (cli.rs/main.rs/commands/mod.rs), strict 0/1/2 exit-code policy, enable_ansi_support bootstrap
  - phase: 04-terminal-visuals
    provides: crossterm 0.29 (already a dep — no Cargo.toml change), the is_color_on()-gated pure-helper template (04-01) + per-Unicode-scalar width-aware emit pattern (04-02), unicode-width 0.2.2 (04-02)
provides:
  - "box matrix — a full-terminal halfwidth-katakana green digital-rain animation at ~20 FPS"
  - "RAII RawGuard: the locked terminal-state restore pattern (Show + LeaveAlternateScreen + disable_raw_mode on every exit path) — the first command to own raw-mode state; reusable by Phase-5 pomodoro"
  - "single-flush-per-frame render loop where event::poll(50ms) IS the frame timer (no background thread)"
  - "Press-only KeyEventKind filter for the Windows double-fire pitfall"
affects: [05-windows-platform-integration]

# Tech tracking
tech-stack:
  added: []  # NO new crate — crossterm/owo-colors/rand/unicode-width all already present
  patterns:
    - "RAII Drop guard for terminal-state restoration (armed immediately after enter; fires on normal return, ? early-return, and unwinding)"
    - "Buffer the WHOLE frame with queue! then stdout.flush() EXACTLY ONCE per frame (never per character — the STATE.md hard pitfall, ~5 FPS otherwise)"
    - "event::poll(Duration) doubles as the frame timer AND the keypress read — single-threaded, no separate sleep"
    - "Filter KeyEventKind::Press before matching the exit key (Windows fires Press AND Release)"
    - "Pure terminal-free helpers (drop/fade model, katakana glyph table, quit-key logic) unit-tested; the terminal-bound loop covered by a smoke test + human-verify"

key-files:
  created:
    - src/commands/matrix/mod.rs
    - tests/matrix.rs
  modified:
    - src/cli.rs
    - src/main.rs
    - src/commands/mod.rs

key-decisions:
  - "D-07: glyph set is pure halfwidth katakana (U+FF66–U+FF9D) — every glyph is East-Asian-Width Narrow / single-cell so the column/drop grid math stays clean; NO fallback flag or charset option (VIS-V2-01); the CJK-font requirement (tofu on bare Cascadia Mono = documented cosmetic limitation, not a bug) is surfaced in --help"
  - "D-08: the WHOLE frame is rendered into one buffer with queue! then flushed EXACTLY ONCE per frame; head cell bright, trail fades green→dark by distance, the cell past the tail is erased"
  - "D-09: frame rate ≈ 20 FPS and the frame timer IS the input poll — event::poll(Duration::from_millis(50)) gates both the frame and the keypress read; single-threaded, no background thread, no separate sleep"
  - "D-10: exit on Ctrl+C (KeyCode::Char('c') + KeyModifiers::CONTROL — in raw mode crossterm delivers Ctrl+C as a KeyEvent, NOT SIGINT), q, and Esc; filter KeyEventKind::Press ONLY; restoration via an RAII Drop guard built right after setup; NO ctrlc/signal crate"

patterns-established:
  - "RAII terminal-restore guard: the locked Show + LeaveAlternateScreen + disable_raw_mode drop, armed immediately after enter — Phase-5 pomodoro reuses this for its in-place countdown"
  - "Single-flush-per-frame animation loop with event::poll as the frame timer — the reference animation pattern for any future timed-redraw command"

requirements-completed: [MTRX-01]

# Metrics
duration: 5min
completed: 2026-06-24
---

# Phase 4 Plan 04: matrix Summary

**`box matrix` — a full-terminal halfwidth-katakana (U+FF66–FF9D) green digital-rain animation at ~20 FPS on crossterm 0.29, single-flush-per-frame, with an RAII restore guard that cleanly tears down raw mode / alt screen / hidden cursor on Ctrl+C / q / Esc (Press-only filter); ALL FOUR Phase-4 stubs are now gone.**

## Performance

- **Duration:** ~5 min (task commits 14:56 → 15:01 +0200) + a human-verify gate
- **Started:** 2026-06-24T14:56:37+02:00 (first task commit)
- **Completed:** 2026-06-24 (human-verify approved, plan closed out)
- **Tasks:** 2 (Task 1 auto/TDD RED→GREEN→help; Task 2 human-verify — PASSED via human approval)
- **Files modified:** 5

## Accomplishments
- `box matrix` is live — the `not_implemented("matrix")` arm is deleted from `src/main.rs`, so ALL FOUR Phase-4 stubs (json/lolcat/ascii/matrix) are now gone
- Full-terminal green katakana digital-rain at ~20 FPS, rendered with a single `stdout.flush()` per frame (D-08/D-09 — the STATE.md per-character-flush ~5-FPS pitfall avoided)
- RAII `RawGuard` (`impl Drop`) restores cursor + leaves the alternate screen + disables raw mode on every exit path (normal return, `?` early-return, unwinding) — the first command to own terminal raw-mode state (D-10, threat T-04M-01 mitigated)
- Exits on Ctrl+C / q / Esc with a `KeyEventKind::Press`-only filter (Windows double-fire / sticky-exit pitfall closed, D-10 / threat T-04M-02)
- Pure helpers unit-tested without a terminal: the drop/fade model (head advances monotonically, fade bright→dark by distance, reset to a negative start after clearing bottom+trail), the katakana glyph table (all in U+FF66..=U+FF9D and all width-1), and the quit-key logic (Press-only, q/Esc/Ctrl+C); the terminal-bound loop is covered by a non-hanging smoke test + the human-verify gate
- CJK-font requirement documented in `--help` (D-07): tofu on bare Cascadia Mono is a cosmetic limitation, not a defect

## Task Commits

Each task was committed atomically (TDD RED→GREEN, then a Rule-2 help fix):

1. **Task 1 (RED): MTRX-01 enter/exit smoke test** — `8a76791` (test) — failing `tests/matrix.rs` (non-hanging / no-panic enter/exit contract) compiled against the stub
2. **Task 1 (GREEN): implement the matrix slice** — `780982e` (feat) — `src/commands/matrix/mod.rs` (RawGuard, pure drop/fade model, katakana table, single-flush loop) + registry wiring (`cli.rs`/`main.rs`/`commands/mod.rs`); stub deleted
3. **Task 1 (Rule-2 fix): surface the CJK-font note in `--help`** — `7f40936` (docs) — D-07 cosmetic-font limitation documented on the clap long help

**Plan metadata:** _this commit_ (docs: complete plan — SUMMARY + STATE + ROADMAP + REQUIREMENTS)

_Note: TDD task — multiple commits (test → feat → docs)._

## Files Created/Modified
- `src/commands/matrix/mod.rs` — `MatrixArgs` + `RunCommand` impl + RAII `RawGuard` + pure drop/fade model + katakana glyph table (U+FF66–FF9D) + single-flush render loop + `#[cfg(test)]` units; `//!` + clap `///` doc carry the D-07 CJK-font note
- `tests/matrix.rs` — MTRX-01 smoke: the binary enters/exits cleanly (fed `q` / non-TTY stdin, bounded `.timeout`), proving it starts and exits non-hanging without panicking
- `src/cli.rs` — `Matrix` unit variant swapped for `Matrix(crate::commands::matrix::MatrixArgs)` (doc preserved)
- `src/main.rs` — `Commands::Matrix(args) => args.run()` replaces the `not_implemented("matrix")` arm
- `src/commands/mod.rs` — `pub mod matrix;` added in alpha order

## Decisions Made
- Followed the plan's locked decisions D-00 / D-07 / D-08 / D-09 / D-10 exactly (glyph set, single-flush loop, poll-as-timer, RAII teardown + Press-only filter). No new decisions, no library swaps.
- NO Cargo.toml change — crossterm/owo-colors/rand/unicode-width were all already present (confirmed: build resolves with zero manifest edit).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Surfaced the CJK-font note in `box matrix --help`**
- **Found during:** Task 1 (GREEN, after implementing the katakana render)
- **Issue:** Plan must-have D-07 requires the CJK-font requirement (tofu on bare Cascadia Mono is a cosmetic limitation, not a bug) to be visible in `--help` so a user seeing tofu boxes isn't misled into reporting a defect; the initial module doc carried it but the clap long help did not yet expose it to the user.
- **Fix:** Added the font note to the clap `///` long help (and the `//!` module doc) — "halfwidth katakana need a CJK-capable font; on bare Cascadia Mono they render as tofu □ — known cosmetic limitation; for authentic glyphs use e.g. Cascadia Next JP."
- **Files modified:** src/commands/matrix/mod.rs
- **Verification:** `grep` confirms the note in both the `//!` doc and the clap `///` long help; the human-verify step (#3) treats the tofu case as the documented font note, not a defect.
- **Committed in:** `7f40936` (separate docs commit)

---

**Total deviations:** 1 auto-fixed (1 missing-critical, Rule 2 — a documentation completeness requirement from must-have D-07).
**Impact on plan:** The fix satisfies a plan must-have (D-07 help visibility). No scope creep; no behavior change to the render path.

## Issues Encountered
None during the planned work. The drop/fade model, katakana table, and quit-key logic were unit-testable without a terminal exactly as the plan anticipated, so the only terminal-bound surface (the render loop) was deferred to the smoke test + human-verify gate.

## Human-Verify Verdict (Task 2 — PASSED via human approval)

The `checkpoint:human-verify` gate (Task 2) was confirmed by the human (NOT a self-approval). Running `cargo run --release -- matrix` in PowerShell 7, the human verified and responded **"approved"**, confirming all of the plan's `resume-signal` criteria:

- The rain **fills the full terminal width** (columns span the whole window).
- It **animates smoothly (~20 FPS)** — no per-character-flush stutter (D-08 single-flush-per-frame confirmed by eye).
- The head cell is **bright** and the trail **fades green→dark** behind it; glyphs **change each frame** (D-08 fade model + per-frame random head glyph).
- **Ctrl+C**, **q**, and **Esc** each **exit cleanly** — the cursor is restored, the alternate screen is left, and the real terminal shows **no leftover rain/glyphs and no hidden cursor** (D-10 RAII teardown / MTRX-01 "no visual artifacts" criterion; threat T-04M-01 mitigated).
- No exit key double-fires or feels sticky (D-10 Press-only filter; threat T-04M-02 mitigated).
- The katakana-tofu-on-bare-Cascadia-Mono case is the **documented cosmetic font note (D-07)**, NOT a defect.

This satisfies ROADMAP Phase-4 success criterion #2 and requirement **MTRX-01**.

## Verification Summary
- `cargo test --bin box matrix` — 8/8 green (drop/fade model: head-advance-monotonic, fade bright→dark, reset on clear-bottom; katakana table: all U+FF66..=U+FF9D and all width-1; quit-key: Press-only, q/Esc/Ctrl+C, plain-`c`-is-not-quit, Release/Repeat-do-not-quit)
- `cargo test --test matrix` — 1/1 green (enter/exit smoke: starts + exits non-hanging in ≤5s, no panic)
- Full suite green per the prior executor (119 unit + 20 integration); the matrix slice re-verified intact this session
- `not_implemented("matrix")` arm gone from `src/main.rs`; `impl Drop for RawGuard` present; exactly one `flush()` per frame; `KeyEventKind::Press` filter present
- Human-verify gate: APPROVED (see verdict above)

## TDD Gate Compliance
Task 1 followed the RED→GREEN cycle correctly: a `test(04-04)` commit (`8a76791`) landed the failing smoke test against the stub, then a `feat(04-04)` commit (`780982e`) made it pass by implementing the slice and deleting the stub. A subsequent `docs(04-04)` commit (`7f40936`) surfaced the D-07 help note (no behavior change → no REFACTOR commit needed). RED and GREEN gates both present and ordered.

## Known Stubs
None introduced by this plan. With `matrix` shipped, ALL FOUR Phase-4 stubs (json/lolcat/ascii/matrix) are gone. Four stubs remain in the binary — all Phase-5 (qr/clip/pomodoro/weather) — and are out of scope for this phase.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- **Phase 4 (terminal-visuals) is FEATURE-COMPLETE** — 4/4 plans shipped (json/lolcat/ascii/matrix), all four requirements satisfied (JSON-01, LOL-01, ASCI-01, MTRX-01). The phase is ready for verification.
- The RAII `RawGuard` terminal-restore pattern and the single-flush-per-frame poll-as-timer loop are the established references for Phase-5 `pomodoro` (in-place countdown + clean Ctrl+C cancel).
- No blockers. Standing Phase-5 todos remain (winrt-notification compile spike, pomodoro blocking-vs-non-blocking, weather default units) — unrelated to this plan.

## Self-Check: PASSED
- FOUND: `src/commands/matrix/mod.rs`
- FOUND: `tests/matrix.rs`
- FOUND commit `8a76791` (test RED)
- FOUND commit `780982e` (feat GREEN)
- FOUND commit `7f40936` (docs help note)

---
*Phase: 04-terminal-visuals*
*Completed: 2026-06-24*
