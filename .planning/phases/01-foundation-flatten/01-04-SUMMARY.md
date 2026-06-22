---
phase: 01-foundation-flatten
plan: 04
subsystem: distribution
tags: [powershell, install, path, reg-expand-sz, crt-static, msvc, smoke-test, readme, checkpoint]

# Dependency graph
requires:
  - "01-01: single-binary `box` scaffold, all 23 subcommands in `box --help`, strict 0/1/2 exit codes, Cargo.toml version 0.1.0"
  - "01-03: working `box flatten` — the command the installer's smoke test and the human-verify flatten step exercise"
provides:
  - "install.ps1 — build (MSVC + crt-static) -> copy to %LOCALAPPDATA%\\Programs\\box -> idempotent REG_EXPAND_SZ-safe user-PATH add -> User+Machine session refresh -> `box --help` smoke test"
  - "README.md — Install section (.\\install.ps1 + execution-policy note + box flatten usage), Usage, and the 0/1/2 + color conventions"
  - "Verified: the release MSVC + crt-static link compiles and the produced box.exe runs (box 0.1.0) — resolves the carried-over 'MSVC+crt-static unverified' todo from 01-01/01-03"
affects: [all future phases (every command becomes globally reachable once a user runs install.ps1), milestone ship (install.ps1 is the chosen v1 distribution path per PROJECT.md)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Raw user-PATH read via (Get-Item HKCU:\\Environment).GetValue('Path','','DoNotExpandEnvironmentNames') so the dedup decision and the REG type decision see the un-expanded %VARS% form (Pitfall 3)"
    - "ExpandString write only when the existing PATH contains '%', else [Environment]::SetEnvironmentVariable(...,'User') — preserves REG_EXPAND_SZ, never downgrades to REG_SZ"
    - "Idempotent PATH append guarded by split-on-';' + -inotcontains (not substring .Contains), so a re-install reports 'already in user PATH — skipped'"
    - "Live-session refresh rebuilds $env:Path from BOTH User and Machine scopes so System32 etc. are not dropped (D-03)"
    - "Never writes the Machine PATH scope — entirely user-scope, no admin/UAC (T-04-elevation eliminated)"

key-files:
  created:
    - "install.ps1"
    - "README.md"
  modified: []

key-decisions:
  - "Authored install.ps1 to match the 01-RESEARCH annotated example exactly; the only additions are two defensive guards (post-build Test-Path on the exe, Copy-Item -LiteralPath) that harden the documented flow without changing behavior"
  - "Ran `cargo build --release --target x86_64-pc-windows-msvc` with crt-static read-only to verify the link compiles and box.exe runs (box 0.1.0); did NOT install, did NOT mutate user PATH — those are reserved for the human-verify gate"

requirements-completed: []   # FOUND-07, FOUND-08 are PENDING the human-verify gate (not auto-completed)

# Metrics
duration: 2min
completed: 2026-06-22
---

# Phase 1 Plan 04: install.ps1 + README Summary

**`install.ps1` — the same-session global-install path: builds the MSVC + crt-static release binary, copies it to `%LOCALAPPDATA%\Programs\box`, adds that dir to the user PATH idempotently (raw `DoNotExpandEnvironmentNames` read + `ExpandString` write so an existing `%VARS%` PATH is never downgraded from `REG_EXPAND_SZ` to `REG_SZ`), refreshes the live `$env:Path` from User ⊕ Machine, and smoke-tests `box --help` — plus a README Install section. The release link was verified compiling read-only; the actual install + PATH mutation is the human-verify gate below.**

## Status: PAUSED at human-verify checkpoint

Task 1 (authoring) is complete and committed. **Task 2 is a blocking human-verification gate and is intentionally NOT auto-completed** — a person must run `install.ps1` in a real PowerShell 7 session and confirm same-session behavior. The exact steps are in "Human Verification Required" below.

## Performance

- **Duration:** ~2 min (authoring + read-only build verify)
- **Started:** 2026-06-22T13:57:44Z
- **Tasks:** 1 of 2 complete (Task 2 is the open human-verify gate)
- **Files modified:** 2 created, 0 modified

## Accomplishments

- **`install.ps1` authored (Task 1, FOUND-07/FOUND-08 build + PATH machinery).** Follows the 01-RESEARCH annotated example exactly:
  - `#Requires -Version 7`, `Set-StrictMode -Version Latest`, `$ErrorActionPreference = 'Stop'`.
  - **Build:** `$env:RUSTFLAGS = '-C target-feature=+crt-static'` then `cargo build --release --target x86_64-pc-windows-msvc`, with a `$LASTEXITCODE` throw and a post-build `Test-Path` on the produced exe.
  - **Copy:** `New-Item -Force` the bin dir, `Copy-Item -Force` to `%LOCALAPPDATA%\Programs\box\box.exe` (D-01, D-04 plain overwrite).
  - **Idempotent PATH:** raw read via `(Get-Item HKCU:\Environment).GetValue('Path','','DoNotExpandEnvironmentNames')`, split on `;`, `-inotcontains` dedup, `Set-ItemProperty ... -Type ExpandString` when `$rawPath -match '%'` else `[Environment]::SetEnvironmentVariable('Path',$newPath,'User')`. **Never** writes Machine scope (D-02, Pitfall 3, T-04-pathcorrupt / T-04-dup / T-04-elevation).
  - **Session refresh:** `$env:Path` rebuilt from `'User'` + `'Machine'` (D-03, Pitfall 9, T-04-sessionpath).
  - **Smoke test:** `& box --help | Out-Null` guarded by `$LASTEXITCODE` (FOUND-08).
- **`README.md` authored.** Install section with `.\install.ps1`, the one-time execution-policy note (`Set-ExecutionPolicy -Scope CurrentUser RemoteSigned` or `pwsh -ExecutionPolicy Bypass -File install.ps1`, Pitfall 9), a `box flatten ./src ./out --dry-run` usage example, and the 0/1/2 exit-code + color-gate conventions.
- **Automated verify passed:** `[System.Management.Automation.Language.Parser]::ParseFile` reports no syntax errors (exit 0).
- **Release link verified (read-only):** `RUSTFLAGS=-C target-feature=+crt-static cargo build --release --target x86_64-pc-windows-msvc` finished clean, and `target\x86_64-pc-windows-msvc\release\box.exe --version` printed `box 0.1.0`. This resolves the "MSVC + crt-static release link unverified" carryover from 01-01/01-03 — but is read-only: no install, no PATH change.

## Task Commits

1. **Task 1: install.ps1 + README** — `e3dbfe9` (feat) — install.ps1 (build/copy/idempotent PATH/session refresh/smoke test) + README Install section. Parse-clean; release MSVC+crt-static build verified compiling.
2. **Task 2: Human-verify same-session install + flatten** — **NO COMMIT (open checkpoint).** Manual verification only; left open per `gate="blocking-human"`.

## Acceptance Criteria (Task 1) — all met

- [x] `install.ps1` contains `#Requires -Version 7`, builds with `--target x86_64-pc-windows-msvc`, sets `RUSTFLAGS` to `-C target-feature=+crt-static`.
- [x] Reads PATH via `DoNotExpandEnvironmentNames`, dedups with `-inotcontains`, writes via `Set-ItemProperty ... -Type ExpandString` when `$rawPath -match '%'`.
- [x] Refreshes `$env:Path` from BOTH `'User'` and `'Machine'`, ends with a `box --help` smoke test guarded by `$LASTEXITCODE`.
- [x] Never writes the `'Machine'` PATH scope.
- [x] PowerShell parse check reports no syntax errors (exit 0).

## Human Verification Required (Task 2 — the open gate)

This is the one behavior that cannot be asserted from `cargo test` — it needs a real PS7 process + the user's registry + live environment, AND it makes outward-facing system changes (user PATH, a copied exe) that the executor must not perform. **A person must run the following in a PowerShell 7 window at the repo root** (`C:\Users\tim\MyDocuments\Programming\cli-tools`):

1. **Install:** `.\install.ps1` (if blocked by execution policy: `pwsh -ExecutionPolicy Bypass -File install.ps1`).
   Expect: a successful build, an `Installed to ...\Programs\box\box.exe` line, an `Added ... to user PATH` (or `already in user PATH — skipped`) line, and `box is ready`.
2. **Same-session help (success-criterion 1):** in the SAME window, `box --help` → all 23 subcommands listed with one-line descriptions, with NO new terminal opened.
3. **Version + exit codes:** `box --version` → `0.1.0`; `box; echo $LASTEXITCODE` → `2`; `box qr; echo $LASTEXITCODE` → `1` with the not-implemented message on stderr.
4. **Real flatten:** make a small nested folder with duplicate filenames, then `box flatten .\src .\out --dry-run` → colored status glyphs + the locked summary, and `.\out` stays empty. Then `box flatten .\src .\out` → files appear flat in `.\out` with original timestamps.
5. **Idempotent re-install:** re-run `.\install.ps1` → the PATH line reports `already in user PATH — skipped` (no duplicate). Optionally `reg query HKCU\Environment /v Path` → type is still `REG_EXPAND_SZ` if it was before.
6. **Persistence:** open a FRESH PS7 window → `box --help` still works.

**Resume signal:** Type `approved` if same-session install + flatten work, or describe what failed (e.g. `box not found in same session`, `PATH duplicated`, `build failed: link.exe missing`).

On `approved`, FOUND-07 and FOUND-08 are confirmed shippable and the phase gate is cleared (per VALIDATION.md "Manual-Only Verifications").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Post-build existence guard on the produced exe**
- **Found during:** Task 1 authoring.
- **Issue:** The RESEARCH sample throws only on `$LASTEXITCODE -ne 0`, then immediately `Copy-Item`s `$ExeSrc`. If cargo reports success but (e.g. on a misconfigured target) the expected `.\target\x86_64-pc-windows-msvc\release\box.exe` is absent, `Copy-Item` would emit a confusing low-level "cannot find path" error instead of a clear one.
- **Fix:** Added `if (-not (Test-Path -LiteralPath $ExeSrc)) { throw "build succeeded but $ExeSrc was not produced" }` between build and copy, and used `Copy-Item -LiteralPath ... -Destination ...` for unambiguous source/dest. Behavior is otherwise identical to the RESEARCH example.
- **Files modified:** install.ps1
- **Committed in:** e3dbfe9

**Total deviations:** 1 (a defensive guard that hardens the documented flow; no behavior change to the happy path, no scope expansion).

## Threat Surface

No new trust boundaries beyond the plan's `<threat_model>`. The script honors every `mitigate` disposition:
- **T-04-pathcorrupt:** raw `DoNotExpandEnvironmentNames` read + `ExpandString` write when `%` present — no `REG_EXPAND_SZ -> REG_SZ` downgrade.
- **T-04-dup:** split-on-`;` + `-inotcontains` dedup; re-install reports "already present".
- **T-04-elevation:** user-scope `HKCU` only; Machine scope is never written.
- **T-04-sessionpath:** `$env:Path` rebuilt from User ⊕ Machine.
- **T-04-policy (accept):** documented `RemoteSigned`/`Bypass` in README.

## Known Stubs

None introduced by this plan. (The 22 unimplemented subcommands are intentional Phase 1 scaffold stubs from 01-01, already documented there; install.ps1 and README add no stubs.)

## User Setup Required

- **PowerShell execution policy (one-time, on a fresh machine):** allow local scripts with `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned`, or run the installer via `pwsh -ExecutionPolicy Bypass -File install.ps1`. Documented in README.

## Next Phase Readiness

- **Pending human-verify:** once a person confirms the same-session install + flatten, Phase 1 (Foundation + Flatten) is shippable end-to-end and FOUND-07/FOUND-08 can be marked complete.
- **No code blockers.** The release MSVC + crt-static link is now verified compiling, so no late surprise on the build path.

## Self-Check: PASSED

All 2 created files verified on disk (`install.ps1`, `README.md`) plus this SUMMARY; Task 1 commit `e3dbfe9` verified in git history. Task 2 is an intentional open human-verify gate (no commit expected).

---
*Phase: 01-foundation-flatten*
*Completed (authoring): 2026-06-22 — Task 2 human-verify gate OPEN*
