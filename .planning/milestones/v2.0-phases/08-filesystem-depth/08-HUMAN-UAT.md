---
status: partial
phase: 08-filesystem-depth
source: [08-VERIFICATION.md]
started: "2026-06-28T01:51:49Z"
updated: "2026-07-14T20:47:27Z"
---

## Current Test

[testing paused — 1 item outstanding: test 1 needs a human to watch the bar render on a real TTY]

## Tests

### 1. Large-batch stderr progress bar (hash / flatten) in a real PS7 terminal
expected: In Windows PowerShell 7, `box hash` on 9+ files shows a stderr progress indicator during hashing, and `box flatten <src> <out>` shows a stderr copy progress bar for plans over 16 items. Under `--json` (e.g. `box hash *.bin --json`) NO progress/ANSI bytes appear on stdout (the document stays one clean parseable value). The structural guarantee (progress suppressed under `--json`) is already verified by the `json_purity` tests + source inspection; this item only confirms the bar visually renders on a real TTY.
result: [pending]
note: Visual-only — the bar draws to stderr on an interactive TTY, which a captured/piped tool run cannot observe. The structural guarantee (suppressed under --json) is already test-verified. Run interactively to confirm the bar renders.

### 2. `du --on-disk` vs Explorer on an NTFS-compressed file
expected: Mark a file/dir compressed (`compact /c <path>`), then `box du --on-disk <path>` reports a SMALLER allocated size than the apparent size, matching Explorer's "Size on disk" column. The `on_disk_size` automated test exercises the Win32 `GetCompressedFileSizeW` path on a non-compressed file; this item confirms the smaller-than-apparent case for a genuinely compressed file.
result: pass
verified: Driven live 2026-07-14. Wrote an 8,404,992-byte highly-compressible file, `compact /c` compressed it 8:1 ("stored in 1,052,672 bytes"). `box du --on-disk --json` reported total_bytes=1,052,672 (on_disk marker true) vs apparent total_bytes=8,404,992 — on-disk is 12.5% of apparent, and 1,052,672 matches Windows' own `compact` "stored" figure (= Explorer's Size-on-disk) byte-for-byte.

## Summary

total: 2
passed: 1
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps
