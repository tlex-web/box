---
status: partial
phase: 10-fun-system-depth
source: [10-VERIFICATION.md]
started: 2026-07-14
updated: 2026-07-14
---

## Current Test

[awaiting human testing]

## Tests

### 1. Pomodoro auto-cycle live UX
expected: `box pomodoro 1 --cycles 2 --label "deep work" --sound` runs 2 work sessions with a 5-min break between (use short MINUTES to test quickly), shows a live raw-mode countdown with a session counter (e.g. `Pomodoro 1/2`) and the color-gated label, plays an audible Win32 `MessageBeep` on each completion, and fires a labeled toast. `--loop` runs indefinitely until Ctrl+C. Cancel (Ctrl+C / q / Esc) restores the terminal cleanly, exits 1, and fires NO beep/toast for the cancelled segment.
result: [pending]

### 2. Weather live network + cache latency
expected: `box weather London --forecast` returns current conditions plus a 7-day daily outlook from the live Open-Meteo API; a second identical call within ~10 min is served from the response cache with a perceptible speed difference (no network). With `[weather] location = "London"` set in config, bare `box weather` uses the stored default.
result: [pending]

### 3. Visual quality of new ASCII art / color
expected: In a real color TTY, `box 8ball` sentiment tinting reads correctly (affirmative→green, non-committal→yellow, negative→red) on the compact ASCII 8-ball; `box cowsay --think "..."` renders a legible rounded thought bubble with the `o`-dot tether; `box cowsay --figure dragon "..."` (and other roster figures) render legibly.
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
