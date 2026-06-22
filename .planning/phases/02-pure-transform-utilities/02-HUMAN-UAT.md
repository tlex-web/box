---
status: partial
phase: 02-pure-transform-utilities
source: [02-VERIFICATION.md]
started: "2026-06-22T20:05:00Z"
updated: "2026-06-22T20:05:00Z"
---

## Current Test

[awaiting human testing]

## Tests

### 1. Cowsay backslash rendering in a PS7 TTY
command: `box cowsay "hello"`
expected: The speech bubble and cow render with correct backslashes — backslashes appear as `\` in the cow art. (trycmd snapshots convert `\` to `/` on Windows, documented in `cowsay/mod.rs:24`, so the snapshot is a layout lock only, not a byte-exact lock.)
why_human: trycmd normalizes backslashes to forward-slashes on Windows; only a live PS7 TTY confirms true backslash rendering.
result: [pending]

### 2. Base64 byte-exactness through a real PS7 pipeline
command: `echo -n 'hello' | box base64 | box base64 --decode`
expected: Exactly `hello` is printed to stdout with no extra bytes.
why_human: PowerShell 7 re-encodes through UTF-16 at pipe boundaries (Pitfall 4 / RESEARCH:402-406); `assert_cmd write_stdin` bypasses the shell and proves the Rust reader, not the PS7 pipeline.
result: [pending]

### 3. Passgen paste-safety in PS7
command: `box passgen` then paste the output directly into a PS7 prompt without quoting it
expected: The password pastes without triggering PS7 shell interpretation (no backtick, quote, dollar, pipe, etc.).
why_human: Paste-safety is a shell-interaction property; unit tests confirm banned chars are absent from SYMBOLS, but the actual PS7 paste experience cannot be automated.
result: [pending]

### 4. Truecolor swatch renders blue in a PS7 TTY
command: `box color '#3b82f6'` (run in a TTY, not piped)
expected: The swatch `██████████` renders in truecolor blue matching #3B82F6. (Under NO_COLOR or piped, the swatch is plain glyphs — that path is already locked by trycmd.)
why_human: Truecolor swatch rendering depends on PS7 TTY truecolor support; only a live TTY session confirms the colored path works.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
