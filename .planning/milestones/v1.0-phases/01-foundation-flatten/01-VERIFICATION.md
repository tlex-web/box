---
phase: 01-foundation-flatten
verified: 2026-06-22T00:00:00Z
status: passed
score: 5/5 success criteria verified (12/12 requirements satisfied)
overrides_applied: 0
mode: mvp
re_verification:
  previous_status: none
  note: initial verification
---

# Phase 1: Foundation + Flatten Verification Report

**Phase Goal:** Users can install `box` globally from PowerShell 7 with one script and immediately use `flatten` as a real, safe file-flattening tool.
**Verified:** 2026-06-22
**Status:** passed
**Mode:** MVP (goal is a capability statement; success verified against the 5 ROADMAP success criteria)
**Re-verification:** No — initial verification

## Goal Achievement

The phase goal decomposes into a two-part capability: (1) one-script global install from PS7 with same-session availability, and (2) a real, safe `flatten` tool. Both halves are verified true in the codebase. All 37 automated tests pass, clippy is clean under `-D warnings`, the live debug binary reproduces the exact locked output, and the install.ps1 human-verify gate was independently cleared by the orchestrator on 2026-06-22.

### Observable Truths (ROADMAP Success Criteria)

| #   | Truth (Success Criterion) | Status | Evidence |
| --- | ------------------------- | ------ | -------- |
| SC1 | `.\install.ps1` in PS7 → `box --help` in the SAME session lists all 23 subcommand stubs with one-line descriptions, no new terminal | ✓ VERIFIED | install.ps1 builds (MSVC+crt-static), copies to `%LOCALAPPDATA%\Programs\box`, refreshes `$env:Path` from User⊕Machine (line 61-62), smoke-tests `& box --help` (line 65). Human-verify gate CLEARED 2026-06-22: install ran, `box is ready`, same-session `box --help` listed 23 + `help`. Independently confirmed: live `box --help` emits 24 command lines (23 + `help`); `tests/cmd/help.trycmd` snapshots all 23 by name; `trycmd` test passes. |
| SC2 | `box --version` shows semver; `box badcmd` exits 2 with helpful error; any error to stderr only, never stdout | ✓ VERIFIED | Live: `box --version` → `box 0.1.0` (exit 0); `box zzznotacmd` → exit 2; `box qr` → exit 1 with message on stderr, stdout 0 bytes. Tests `version_is_semver`, `badcmd_exits_2`, `bare_box_exits_2`, `stub_exits_1_to_stderr` all pass. main.rs maps DisplayHelp/Version→0, DisplayHelpOnMissingArgumentOrSubcommand→2, parse errors via `e.exit()`→2, Err→`eprintln!`+exit 1. |
| SC3 | Piped `box flatten --help` is clean text (no ANSI); same command in a terminal is colored | ✓ VERIFIED | `piped_help_has_no_ansi` test asserts no `\x1b[` in captured (non-TTY) output — passes. `color_enabled` gates on `--no-color` ∧ `NO_COLOR` unset ∧ `stdout().is_terminal()` (output.rs:42-44); `format_row` consults `is_color_on()` so plain layout is byte-identical minus ANSI. Colored-terminal half is the manual-only item (TTY required) recorded cleared in the human-verify results (step 4: "colored status glyphs"). |
| SC4 | `box flatten ./src ./out --dry-run` on a nested tree with duplicate names shows a collision-renamed plan with NO files copied | ✓ VERIFIED | Live dry-run on a 3-file tree (duplicate `readme.txt`) printed `~ b\readme.txt -> b_readme.txt [collision]` and `Dry run: nothing was copied.` / `Plan: 2 to copy, 1 renamed for collisions, 0 skipped.`; output dir held 0 files. Test `dry_run_plans_collisions_writes_nothing` asserts `files_in(out)==0` — passes. |
| SC5 | `box flatten ./src ./out` lands all files flat with timestamps preserved, originals untouched, nothing silently lost/overwritten | ✓ VERIFIED | Live execute: 3 files flat in out (`readme.txt`, `b_readme.txt`, `note.md`), `Done: copied 3 files (1 renamed for collisions), skipped 0.`; source intact (3 files). Tests `flatten_copies_all_files_flat`, `preserves_mtime`, `originals_untouched`, `no_silent_overwrite`, `out_inside_src_aborts`, `skips_symlinks`, `deep_path_no_silent_loss` all pass. safe_copy preserves mtime via FileTimes; occupied-set seeded from `read_dir(out)` prevents clobber. |

**Score:** 5/5 success criteria verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `Cargo.toml` | Locked deps + release profile | ✓ VERIFIED | clap 4.6, anyhow 1.0, thiserror 2.0, owo-colors 4.3, enable-ansi-support 0.3, crossterm 0.29, walkdir 2.5, dunce 1.0; no filetime/is-terminal; `opt-level = 3` (not "z"); all 6 dev-deps present. |
| `src/cli.rs` | Cli + Commands enum, 23 variants, doc-comments | ✓ VERIFIED | 23 variants counted; `arg_required_else_help = true`; `#[command(name = "8ball")]`/`EightBall`, `#[command(name = "bulk-rename")]`/`BulkRename`; every variant has a `///` about line. |
| `src/main.rs` | ANSI bootstrap, try_parse, 0/1/2 mapping | ✓ VERIFIED | `enable_ansi_support()` first; `Cli::try_parse()`; DisplayHelpOnMissingArgumentOrSubcommand→`ExitCode::from(2)`; `init_color` before dispatch; Flatten→`args.run()`, 22 stubs→`not_implemented(...)`. ~92 lines, no business logic. |
| `src/core/errors.rs` | BoxError::NotImplemented | ✓ VERIFIED | Exact locked message `'{cmd}' is not yet implemented — coming in a future release`. |
| `src/core/output.rs` | color gate + glyph/summary helpers | ✓ VERIFIED | `color_enabled` (is_terminal+NO_COLOR+flag), `init_color`, `RowStatus`/`format_row` (ASCII glyph source of truth), `truncate_middle`, `terminal_width` (crossterm, 80 fallback), locked `dry_run_summary`/`real_run_summary`. 7 unit tests. |
| `src/core/fs.rs` | normalize_path (dunce), is_hidden, safe_copy | ✓ VERIFIED | `dunce::canonicalize` (never std::fs); `is_hidden` depth-0 gate + `FILE_ATTRIBUTE_HIDDEN 0x2` under cfg(windows); `safe_copy` FileTimes + `.context(...)` per call. 5 unit tests incl. Windows hidden-attribute. |
| `src/commands/flatten/rename.rs` | pure encode/sanitize/dedupe | ✓ VERIFIED | `RESERVED` slice (CON/PRN/AUX/NUL/COM1-9/LPT1-9); `encode_relative` (separator→`_`, strips `..`/`.`), `sanitize_reserved` (trim-then-match), `dedupe` (NTFS case-insensitive). 10 unit tests incl. `encode_no_separator` property + every reserved class. |
| `src/commands/flatten/mod.rs` | FlattenArgs + RunCommand orchestration | ✓ VERIFIED | create_dir_all→normalize both roots→case-insensitive containment guard→seed occupied from read_dir→WalkDir follow_links(false)+filter_entry(!is_hidden), symlink skip→one Plan→dry-run/execute. 2 unit tests. |
| `tests/cli.rs` | Wave-0 CLI tests | ✓ VERIFIED | 7 tests (5 named + piped-no-ansi + trycmd), all pass. |
| `tests/flatten.rs` | 8 flatten integration tests | ✓ VERIFIED | All 8 named tests present and passing. |
| `install.ps1` | build→copy→PATH→refresh→smoke | ✓ VERIFIED | MSVC+crt-static, `%LOCALAPPDATA%\Programs\box`, `-inotcontains` dedup, ExpandString when `%` present, User⊕Machine refresh, `& box --help` smoke. Parse-clean. Never writes Machine scope (only reads it line 62). |
| `README.md` | install + execution-policy + usage | ✓ VERIFIED | Install section, RemoteSigned/Bypass note, flatten dry-run example, 0/1/2 + color conventions. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| main.rs | init_color | called after parse, before dispatch | ✓ WIRED | main.rs:55 before the `match cli.command`. |
| main.rs | NotImplemented | 22 stub arms → Err → exit 1 | ✓ WIRED | All 22 non-flatten arms call `not_implemented(...)`. |
| main.rs | exit 2 mapping | DisplayHelpOnMissingArgumentOrSubcommand | ✓ WIRED | main.rs:42-45 returns `ExitCode::from(2)`. |
| flatten/mod.rs | core::fs::safe_copy | execute copy loop | ✓ WIRED | mod.rs:168, import line 30. |
| flatten/mod.rs | containment guard | starts_with on lowercased canon paths | ✓ WIRED | mod.rs:109-118, before any copy. |
| flatten/mod.rs | occupied seed | read_dir(out_root) lowercased | ✓ WIRED | mod.rs:124-130. |
| output::color_enabled | stdout().is_terminal() | TTY gate | ✓ WIRED | output.rs:43. |
| fs::normalize_path | dunce::canonicalize | UNC-safe wrapper | ✓ WIRED | fs.rs:34. |
| install.ps1 | HKCU Path | DoNotExpandEnvironmentNames read + ExpandString write | ✓ WIRED | install.ps1:43,45,50. |
| install.ps1 | session $env:Path | User⊕Machine rebuild | ✓ WIRED | install.ps1:61-62. |
| install.ps1 | smoke test | `& box --help` + $LASTEXITCODE | ✓ WIRED | install.ps1:65-70. |

### Data-Flow Trace (Level 4)

Flatten renders dynamic data (the plan, copied bytes) sourced from a real WalkDir of the source tree and real `safe_copy` byte counts — not hardcoded. The live execute spot-check confirmed real data flows: 3 files discovered from disk, 17 B written (debug build; 25 B in the orchestrator's CRLF run), source file count 3. No static/empty returns. ✓ FLOWING.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Version is semver | `box --version` | `box 0.1.0` (exit 0) | ✓ PASS |
| Help lists all commands | `box --help` line count | 24 (23 + help) | ✓ PASS |
| Bare box exits 2 | `box` | exit 2 | ✓ PASS |
| Stub → stderr + exit 1 | `box qr` | message on stderr, 0 bytes stdout, exit 1 | ✓ PASS |
| Bad command → exit 2 | `box zzznotacmd` | exit 2 | ✓ PASS |
| Dry-run writes nothing | `box flatten src out --dry-run` | plan printed, out dir 0 files | ✓ PASS |
| Execute flattens safely | `box flatten src out` | 3 flat files, 1 collision-renamed, source intact | ✓ PASS |
| Full test suite | `cargo test` | 37 passed (22 unit + 7 cli + 8 flatten), 0 failed | ✓ PASS |
| Lint gate | `cargo clippy --all-targets -- -D warnings` | clean | ✓ PASS |
| Installer syntax | PowerShell `ParseFile` on install.ps1 | PARSE OK | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| FOUND-01 | 01-01 | `box --help` lists all commands; per-command `--help` | ✓ SATISFIED | 24-line listing; trycmd snapshot; `flatten --help` works (piped test). |
| FOUND-02 | 01-01 | `box --version` prints semver | ✓ SATISFIED | Live `box 0.1.0`; `version_is_semver` passes. |
| FOUND-03 | 01-01 | data→stdout, msgs→stderr, exit 0/1/2 | ✓ SATISFIED | `badcmd_exits_2`, `bare_box_exits_2`, `stub_exits_1_to_stderr`; live stub stdout empty. |
| FOUND-04 | 01-02 | color auto-off off-TTY / NO_COLOR / --no-color; ANSI in PS7 | ✓ SATISFIED | `piped_help_has_no_ansi`; `color_enabled` gate; colored-terminal half cleared in human-verify. |
| FOUND-05 | 01-01 | clear stderr message, no panic on bad input | ✓ SATISFIED | NotImplemented → `eprintln!` exit 1; no panic in dispatch; clippy clean. |
| FOUND-06 | 01-02/03 | UNC-safe paths, no silent long-path failure | ✓ SATISFIED | `normalize_path` dunce; `safe_copy` `.context(...)`; `deep_path_no_silent_loss` passes. |
| FOUND-07 | 01-04 | install.ps1 builds, copies to bin dir, idempotent user PATH | ✓ SATISFIED | install.ps1 verified; human-verify gate CLEARED (install + idempotent re-install confirmed). |
| FOUND-08 | 01-04 | refreshes session PATH + smoke-tests box --help | ✓ SATISFIED | User⊕Machine refresh + `& box --help` smoke; same-session availability confirmed in human-verify. |
| FLAT-01 | 01-03 | recursive flat copy, originals untouched, no out subdirs | ✓ SATISFIED | `flatten_copies_all_files_flat`, `originals_untouched`; live spot-check. |
| FLAT-02 | 01-03 | collision rename by encoded path + numeric fallback | ✓ SATISFIED | `encode_relative`/`dedupe`; live `b_readme.txt [collision]`. |
| FLAT-03 | 01-03 | --dry-run previews, writes nothing; completion summary | ✓ SATISFIED | `dry_run_plans_collisions_writes_nothing`; live dry-run 0 files. |
| FLAT-04 | 01-03 | refuse out-inside-src, skip symlinks, preserve timestamps | ✓ SATISFIED | `out_inside_src_aborts`, `skips_symlinks`, `preserves_mtime`, `no_silent_overwrite`. |

All 12 phase requirement IDs are claimed by a plan's `requirements` frontmatter and satisfied. No orphaned requirements: REQUIREMENTS.md maps exactly FOUND-01..08 + FLAT-01..04 to Phase 1, all covered. (Note: REQUIREMENTS.md traceability table still shows FOUND-07/08 as "Pending" and their checkboxes unticked — a stale doc-status lag, not a functional gap; the human-verify gate was cleared 2026-06-22 per 01-04-SUMMARY. Flagged as Info below.)

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| src/core/errors.rs | 10 | "not yet implemented" string | ℹ️ Info | Intentional locked stub message (D-06), not a debt marker. No impact. |
| .planning/REQUIREMENTS.md | 18-19,115-116 | FOUND-07/08 marked Pending / unticked | ℹ️ Info | Stale traceability lag; functionally complete (gate cleared). Recommend ticking to keep docs in sync. |

No `TODO`/`HACK`/`FIXME`/`TBD`/`XXX`/`todo!`/`unimplemented!` markers in `src/`. The TDD RED `unimplemented!()` bodies were fully replaced in GREEN commits (confirmed absent). No stub returns, no orphaned helpers (both forward-compat `#[allow(dead_code)]` removed; clippy `-D warnings` clean proves every core helper is reached).

### Human Verification Required

None outstanding. The two manual-only items from VALIDATION.md were both cleared:

1. **install.ps1 same-session `box --help`** (FOUND-07/08) — CLEARED by orchestrator in real PS7 on 2026-06-22 with explicit user authorization (install ran, `box is ready`, 23 commands listed same-session, idempotent re-install, fresh-window persistence). Recorded in 01-04-SUMMARY "Human Verification — Results".
2. **Colored flatten output in a terminal** (FOUND-04 TTY half) — covered by human-verify step 4 ("colored status glyphs"); the piped/plain half is independently asserted by `piped_help_has_no_ansi`.

### Gaps Summary

No gaps. Both halves of the phase goal are observably true in the codebase:
- **Install + same-session availability:** install.ps1 is parse-clean, builds the MSVC/crt-static binary, installs user-scope only, refreshes the live session, and smoke-tests `box --help` — empirically confirmed via the cleared human-verify gate.
- **Real, safe flatten:** 18 flatten-related tests (10 unit + 8 integration) pass, and a live binary run reproduces the exact locked D-09/D-11 output with dry-run writing nothing and execute flattening 3 files (1 collision-renamed) while leaving the source intact.

All 5 ROADMAP success criteria, all 12 requirements, and all PLAN must-have truths/artifacts/key-links are verified. Full suite green (37/37), clippy clean. The only findings are two Info-level documentation-status lags (stub message string by design; REQUIREMENTS.md FOUND-07/08 still marked Pending).

---

_Verified: 2026-06-22_
_Verifier: Claude (gsd-verifier)_
