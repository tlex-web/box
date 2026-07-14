---
phase: 01-foundation-flatten
plan: 01
subsystem: infra
tags: [rust, clap, clap-derive, anyhow, thiserror, exit-codes, assert_cmd, trycmd, cli-scaffold]

# Dependency graph
requires: []
provides:
  - "Single-binary `box` crate scaffold (clap-derive) with all 23 subcommands registered"
  - "RunCommand trait — the static-dispatch contract every command's Args struct implements"
  - "BoxError::NotImplemented typed error + stub::not_implemented helper"
  - "main() with strict 0/1/2 exit-code mapping (ANSI bootstrap, try_parse, D-06/D-07/D-08)"
  - "Wave-0 integration test harness (tests/cli.rs) + 23-command --help trycmd snapshot"
  - "Locked dependency set + release profile (lto thin, codegen-units 1, strip, panic=abort, opt-level 3)"
affects: [01-02 core-infra, 01-03 flatten, 01-04 install, phase-2, phase-3, phase-4, phase-5]

# Tech tracking
tech-stack:
  added: [clap 4.6.1, anyhow 1.0.102, thiserror 2.0.18, owo-colors 4.3.0, enable-ansi-support 0.3.1, crossterm 0.29.0, walkdir 2.5.0, dunce 1.0.5, assert_cmd 2.2.2, predicates 3.1.4, assert_fs 1.1.4, tempfile 3.27.0, trycmd 1.2.0, insta 1.48.0]
  patterns: ["clap-derive single Commands enum (doc-comment -> about)", "RunCommand static dispatch (no Box<dyn>)", "try_parse + explicit ExitCode mapping (not parse())", "typed BoxError mapped to exit codes by main()", "data->stdout / messages->stderr stream discipline"]

key-files:
  created:
    - "Cargo.toml"
    - ".gitignore"
    - "src/main.rs"
    - "src/cli.rs"
    - "src/core/mod.rs"
    - "src/core/errors.rs"
    - "src/commands/mod.rs"
    - "src/commands/stub.rs"
    - "tests/cli.rs"
    - "tests/cmd/help.trycmd"
  modified: []

key-decisions:
  - "Bare `box` prints help to stderr (not stdout) and exits 2 — resolves OQ-1 toward strict messages->stderr while satisfying D-08"
  - "Included Cargo.lock in the manifest commit (binary crate — lockfile is part of the build contract)"
  - "Pre-created src/commands/stub.rs in Task 1 (Rule 3) so the crate compiles before Task 2 wires dispatch"
  - "trycmd snapshot uses box[EXE] substitution so the Windows box.exe binary name does not make the snapshot brittle"

patterns-established:
  - "Single registry: all 23 commands live in one clap-derive `Commands` enum in src/cli.rs; doc comments are the --help one-liners"
  - "Exit-code policy owned solely by main(): 0 success, 1 runtime/NotImplemented error, 2 clap parse error / bare box"
  - "Stubs are real enum variants dispatched to a NotImplemented handler (D-05) — keeps all 23 visible in --help"
  - "TDD for behavioral tasks: RED (failing test commit) -> GREEN (impl commit)"

requirements-completed: [FOUND-01, FOUND-02, FOUND-03, FOUND-05]

# Metrics
duration: 5min
completed: 2026-06-22
---

# Phase 1 Plan 01: Crate Scaffold + Command Registry + Exit-Code Mapping Summary

**Single-binary `box` crate with all 23 clap-derive subcommands registered, the RunCommand trait, a typed NotImplemented error, and a main() that enforces the strict 0/1/2 exit-code contract — all covered by a passing Wave-0 test suite.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-22T13:24:22Z
- **Completed:** 2026-06-22T13:29:16Z
- **Tasks:** 3
- **Files modified:** 10 created

## Accomplishments
- Compiling single-binary crate `box` 0.1.0 with the full locked dependency set and the correct release profile (`opt-level = 3`, not `"z"`; `lto = "thin"`, `codegen-units = 1`, `strip`, `panic = "abort"`).
- All 23 subcommands registered in one clap-derive `Commands` enum, each with a one-line `about` doc comment; `8ball` -> `EightBall` and `bulk-rename` -> `BulkRename` keep their CLI names via `#[command(name = ...)]`.
- `RunCommand` trait, `BoxError::NotImplemented`, and the `stub::not_implemented` helper define the dispatch + error contract for every future command.
- `main()` enforces the 0/1/2 policy: `--help`/`--version` -> stdout exit 0; bare `box` -> help to stderr exit 2 (D-08 override of clap's exit-0 default — Pitfall 2); bad command/args -> clap exit 2 (D-07); any command `Err` -> `error: …` to stderr exit 1 (D-06, FOUND-03).
- Wave-0 `tests/cli.rs` (5 named tests + a trycmd transcript) passes; `cargo test` and `cargo clippy --all-targets -- -D warnings` both green.

## Task Commits

Each task was committed atomically:

1. **Task 1: Cargo manifest + crate skeleton + dependencies** - `6405653` (feat)
2. **Task 2: Command registry (23 variants) + NotImplemented error + stub handler** - `5a86df6` (feat)
3. **Task 3: main() exit-code mapping + Wave-0 cli tests** (TDD)
   - `346cc28` (test) — RED: 5 failing Wave-0 tests
   - `7640cc9` (feat) — GREEN: main() dispatch + exit-code mapping, all tests pass

**Plan metadata:** committed separately (docs: complete plan)

_TDD task 3 followed RED -> GREEN; no separate refactor commit was needed (the `#[allow(dead_code)]` forward-compat note on RunCommand was folded into GREEN)._

## Files Created/Modified
- `Cargo.toml` - Crate manifest: binary `box` 0.1.0, locked runtime + dev deps, release profile.
- `Cargo.lock` - Pinned dependency graph (committed for the binary crate).
- `.gitignore` - Ignores `/target`.
- `src/main.rs` - ANSI bootstrap, `Cli::try_parse`, 0/1/2 exit-code mapping, 23-arm dispatch to the stub.
- `src/cli.rs` - `Cli` parser + `Commands` enum (all 23 variants, doc-comment about text).
- `src/core/mod.rs` - Declares `pub mod errors;` (output/fs land in plan 02).
- `src/core/errors.rs` - `BoxError::NotImplemented { cmd }` (thiserror).
- `src/commands/mod.rs` - `RunCommand` trait + module declarations.
- `src/commands/stub.rs` - `not_implemented(cmd)` helper returning the typed error.
- `tests/cli.rs` - 5 Wave-0 tests + trycmd harness.
- `tests/cmd/help.trycmd` - Snapshot of the full 23-command `box --help` listing.

## Decisions Made
- **Bare-`box` help goes to stderr, exit 2.** Resolves OQ-1 toward the strict "messages -> stderr" reading (D-03/FOUND-03) while satisfying D-08. The `DisplayHelpOnMissingArgumentOrSubcommand` branch uses `eprint!("{e}")` then `ExitCode::from(2)`.
- **`Cargo.lock` committed** with the manifest — for a binary crate the lockfile is part of the reproducible-build contract.
- **trycmd transcript uses `box[EXE]`** substitution so the Windows `box.exe` binary name does not make the snapshot fail on other platforms / spacing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pre-created `src/commands/stub.rs` during Task 1**
- **Found during:** Task 1 (crate skeleton)
- **Issue:** Task 1's `src/commands/mod.rs` declares `pub mod stub;`, but the plan only specifies creating `stub.rs` in Task 2 — leaving Task 1 unable to compile (`cargo build` would fail on the missing module file), which violates Task 1's `cargo build exits 0` acceptance criterion.
- **Fix:** Created `src/commands/stub.rs` with the full `not_implemented` helper (matching Task 2's spec) during Task 1. `BoxError::NotImplemented` already existed from Task 1's errors.rs, so no forward reference was unresolved.
- **Files modified:** src/commands/stub.rs
- **Verification:** `cargo build` exited 0 at the Task 1 checkpoint.
- **Committed in:** 6405653 (Task 1 commit)

**2. [Rule 3 - Blocking] `#[allow(dead_code)]` on the `RunCommand` trait**
- **Found during:** Task 3 (GREEN)
- **Issue:** No command implements `RunCommand` yet (plan 03 adds the first impl on `FlattenArgs`), so the compiler emitted `trait RunCommand is never used`. The wave-merge gate runs `cargo clippy -- -D warnings`, which would fail on this warning.
- **Fix:** Added a targeted `#[allow(dead_code)]` with a comment noting plan 03 should remove it once an impl exists.
- **Files modified:** src/commands/mod.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` is clean.
- **Committed in:** 7640cc9 (Task 3 GREEN commit)

**3. [Rule 1 - Bug] Fixed trailing-newline mismatch in trycmd snapshot**
- **Found during:** Task 3 (GREEN, optional trycmd transcript)
- **Issue:** The hand-written `help.trycmd` ended with an extra trailing newline after `Print version`, which trycmd flagged as an expected-but-missing newline diff (test failed).
- **Fix:** Regenerated the snapshot with `TRYCMD=overwrite` to make it byte-exact, then re-ran without overwrite to confirm a stable pass.
- **Files modified:** tests/cmd/help.trycmd
- **Verification:** `cargo test --test cli` passes the `trycmd` case on a clean run.
- **Committed in:** 7640cc9 (Task 3 GREEN commit)

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 bug). The optional trycmd transcript (plan called it "optional but recommended") was implemented.
**Impact on plan:** All auto-fixes were necessary to satisfy the plan's own acceptance criteria (Task 1 `cargo build` exit 0, wave-merge clippy gate, working trycmd snapshot). No scope creep — no functionality beyond the plan was added.

## Issues Encountered
- Git reports `LF will be replaced by CRLF` warnings on every staged file (Windows checkout). Cosmetic only; no `.gitattributes` policy is in scope for this plan.

## Threat Surface
All threat-register dispositions for this plan are satisfied by the test suite:
- **T-01-01** (stream routing): `stub_exits_1_to_stderr` asserts stdout is empty on error; manual spot check confirmed bare-box and `box qr` write nothing to stdout.
- **T-01-02** (exit-code contract): `bare_box_exits_2` + `badcmd_exits_2` lock exit 2; `stub_exits_1_to_stderr` locks exit 1 — a regression collapsing all errors to 1 fails the suite.
- **T-01-03** (panic DoS): no `unwrap`/`panic!` in dispatch; all error paths return `Err` mapped to exit 1.
- **T-01-SC** (dependency installs): all crates were slopcheck-clean per RESEARCH; `cargo build` resolved them at the expected versions. No blocking checkpoint required.

No new security surface was introduced beyond the plan's threat model.

## Known Stubs
The 22 unbuilt commands (every variant except `flatten`, plus `flatten` itself until plan 03) intentionally return `BoxError::NotImplemented` and exit 1. **These stubs are the deliverable of this walking-skeleton plan (D-05/D-06), not accidental placeholders.** Each is tracked to a future plan/phase:

| Stub command(s) | Resolved by |
|-----------------|-------------|
| flatten | Plan 01-03 (real FlattenArgs + RunCommand impl) |
| uuid, base64, epoch, color, passgen, cowsay, fortune, 8ball, roast | Phase 2 |
| hash, tree, du, dupes, bulk-rename | Phase 3 |
| lolcat, matrix, ascii, json | Phase 4 |
| qr, clip, pomodoro, weather | Phase 5 |

`src/core/output::init_color` is referenced by a TODO in `main.rs` but not yet called (the `--no-color` flag is parsed but inert) — wired in plan 01-02 (FOUND-04). This is a documented intentional stub.

## Next Phase Readiness
- **Ready for plan 01-02:** add `src/core/output.rs` (`init_color`, color gating) and `src/core/fs.rs` (dunce normalize, hidden predicate, safe_copy + FileTimes). `main.rs` already has the TODO marker where `init_color(cli.no_color)` plugs in.
- **Ready for plan 01-03:** swap the `Flatten` unit variant for `Flatten(FlattenArgs)` and route it to `args.run()`; the `RunCommand` trait + `#[allow(dead_code)]` removal are the only main.rs touch-points.
- No blockers. The MSVC linker / `x86_64-pc-windows-msvc` release build (used by install.ps1 in plan 01-04) was not exercised here — debug builds and tests run clean; confirm the crt-static release link during plan 01-04.

## Self-Check: PASSED

All 11 created files verified on disk; all 4 task commits (6405653, 5a86df6, 346cc28, 7640cc9) verified in git history.

---
*Phase: 01-foundation-flatten*
*Completed: 2026-06-22*
