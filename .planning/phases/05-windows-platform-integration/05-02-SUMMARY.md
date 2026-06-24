---
phase: 05-windows-platform-integration
plan: 02
subsystem: cli
tags: [clip, clipboard, arboard, cf-unicodetext, win32, newline-trim]

# Dependency graph
requires:
  - phase: 01-foundation-flatten
    provides: "RunCommand trait, 0/1/2 exit policy, anyhow error→exit-1 mapping in main()"
  - phase: 05-windows-platform-integration
    plan: 01
    provides: "the registry slice (cli.rs unit-variant→Args swap, main.rs not_implemented arm removal, commands/mod.rs pub mod, Cargo.toml dep) that this plan mirrors for clip"
provides:
  - "box clip: pipe stdin → Windows clipboard (CF_UNICODETEXT, Unicode-correct, no elevation); box clip --paste: clipboard → stdout byte-exact (CLIP-01)"
  - "arboard 3.6.1 (default-features=false — text-only, drops image-data) dependency"
  - "trim_one_trailing_newline(Vec<u8>) -> Vec<u8> pure seam: strips at most one trailing \\r\\n / \\n (D-05)"
affects: [05-03-pomodoro, 05-04-weather]

# Tech tracking
tech-stack:
  added: ["arboard 3.6.1 (default-features=false — drops image-data; set_text/get_text only, Win32 CF_UNICODETEXT)"]
  patterns:
    - "Raw-stdin command (D-04): clip reads io::stdin().read_to_end DIRECTLY, NOT core::input::read_input — it owns its own UTF-8 validation + newline policy and introduces no exit-2 path (every error → anyhow → exit 1)"
    - "Single-shot main-thread arboard flow (D-06 / STATE.md pitfall): Clipboard::new() → one op (set_text/get_text on `let mut cb`) → return; never spawned onto a worker thread; copy persists via OS handle ownership (Windows-safe, no keep-alive pump)"
    - "Pure helper as the unit-test seam: trim_one_trailing_newline is crate-free + deterministic, so the D-05 newline policy is unit-tested without the binary or a clipboard"

key-files:
  created:
    - "src/commands/clip/mod.rs"
    - "tests/clip.rs"
  modified:
    - "Cargo.toml (arboard dep)"
    - "Cargo.lock (arboard + clipboard-win + windows-sys transitives)"
    - "src/cli.rs (Clip unit variant → Clip(ClipArgs); doc one-liner preserved)"
    - "src/main.rs (not_implemented(clip) arm → args.run())"
    - "src/commands/mod.rs (pub mod clip in alpha order)"

key-decisions:
  - "arboard 3.6.1 with default-features=false (D-06) — drops the default image-data feature (binary/image clipboard is Out of Scope); set_text/get_text remain available, resolved on the first build (no fallback to full defaults needed)"
  - "Copy reads RAW stdin via io::stdin().read_to_end, NOT core::input::read_input (D-04) — routing through read_input would inherit the no-arg-TTY → exit-2 contract, wrong for a clipboard copy"
  - "Single trailing terminator stripped on copy (\\r\\n then a lone \\n), byte-exact on paste (D-05) — fixes the implicit CRLF PowerShell appends when piping a string"
  - "UTF-8 validated BEFORE Clipboard::new() (D-04/FOUND-05) — non-UTF-8 stdin → clean exit 1, never a panic; the failure is the deterministic UTF-8 error, clipboard-independent"
  - "Single-shot main-thread arboard flow satisfies the STATE.md 'arboard main-thread only' pitfall (D-06 / CLIP-2); copy persists past process exit via OS handle ownership"

patterns-established:
  - "Raw-stdin command shape: a command that deliberately bypasses core::input to own UTF-8 + newline policy (the first such command; the read_input contract is correct for arg/stdin text commands but wrong for a clipboard copy)"
  - "Deterministic vs clipboard-bound test split: the environment-independent contract (newline trim, non-UTF-8 exit-1) is auto-tested; the live-clipboard round-trip is #[ignore]d (shared OS state, headless CI may lack a clipboard) and run locally with --ignored --test-threads=1"

requirements-completed: []   # CLIP-01 implementation done + locally automated; human-verify (Task 3) PENDING — deferred to phase UAT

# Metrics
duration: 4min
completed: 2026-06-24
---

# Phase 5 Plan 02: box clip (CLIP-01) Summary

**`box clip` copies raw stdin → the Windows clipboard via arboard's CF_UNICODETEXT path (Unicode-correct, no elevation, persists past exit) and `box clip --paste` reads the clipboard → stdout byte-exact — with a single-trailing-newline trim on copy (D-05) that kills the spurious CRLF PowerShell appends when piping a string, the exact `clip.exe` friction this command exists to fix.**

## Performance

- **Duration:** ~4 min (implementation), plus full-suite + clippy + fmt + local live-clipboard verification
- **Started:** 2026-06-24T18:13:37Z
- **Completed:** 2026-06-24T18:17:31Z
- **Tasks:** 2 of 3 (Task 3 is a human-verify checkpoint — see "Verification Status")
- **Files modified:** 7 (2 created, 5 modified — incl. Cargo.lock)

## Accomplishments
- `"héllo 世界" | box clip` lands the text on the Windows clipboard with correct Unicode (UTF-8 → CF_UNICODETEXT), exit 0, and the implicit trailing CRLF stripped (D-05) — no spurious blank line.
- `box clip --paste` reads the clipboard to stdout byte-exact (no added/stripped newline, D-05).
- The copy→paste round-trip preserves Unicode byte-exact minus the one trailing terminator — verified locally via the live Win32 clipboard (the `#[ignore]`d automated round-trip passes with `--ignored --test-threads=1`).
- Non-UTF-8 stdin on copy (`\xff\xfe`) → clean exit 1 with a stderr message and NO panic (T-05-CLIP-DoS / FOUND-05) — deterministically auto-tested, clipboard-independent (UTF-8 validation precedes `Clipboard::new()`).
- `arboard` 3.6.1 added with `default-features = false` (drops `image-data`; text only) — resolved `set_text`/`get_text` on the first build.
- The `clip` `not_implemented` stub is gone; 2 Phase-5 stubs remain (pomodoro/weather), and the `not_implemented` import stays live for them.

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1: Wave-0 test scaffold (RED)** — `58f729b` (test) — `pub mod clip`, `ClipArgs { paste }` + pure `trim_one_trailing_newline`, 6 in-module D-05 trim unit tests (GREEN immediately — the helper is real), `tests/clip.rs` (round-trip + paste `#[ignore]`d, non-UTF-8 deterministic). The `RunCommand` impl `bail!`s and the main.rs arm is not yet swapped, so the behavior-defining round-trip integration tests are RED (copy exits 1 via the stub).
2. **Task 2: Implement clip slice (GREEN)** — `d18c8f5` (feat) — `arboard` dep, real `ClipArgs::run` (raw-stdin copy + paste), cli.rs/main.rs registry wiring. Trim + non-UTF-8 tests green; the live round-trip passes locally.

**Plan metadata:** committed separately with this SUMMARY.

_Task 3 (human-verify real PS7 copy→paste round-trip) carries no commit — it is a manual gate deferred to phase UAT._

## Files Created/Modified
- `src/commands/clip/mod.rs` — `ClipArgs { paste }` + the pure `trim_one_trailing_newline` helper + `RunCommand::run` (copy = raw stdin → trim → UTF-8 validate → `set_text`; paste = `get_text` → byte-exact stdout) + `#[cfg(test)] mod tests` (6 D-05 cases). Module doc records D-04 (raw stdin, not `read_input`), D-05 (single-trim), D-06 (single-shot main-thread, OS-handle persistence).
- `tests/clip.rs` — integration: `#[ignore]`d live round-trip (single-threaded) + `#[ignore]`d paste-exits-0 + the deterministic non-UTF-8 copy exit-1 no-panic test; header documents the `--ignored --test-threads=1` run command and why the clipboard tests are gated.
- `Cargo.toml` — `arboard = { version = "3.6.1", default-features = false }` with the feature-trim rationale comment.
- `Cargo.lock` — arboard + `clipboard-win` + `windows-sys`/`windows-targets` transitives pinned.
- `src/cli.rs` — `Clip` unit variant → `Clip(crate::commands::clip::ClipArgs)`; the doc one-liner (`Read from or write to the system clipboard`) is preserved verbatim so the `box --help` trycmd snapshot is unchanged, with an added multi-paragraph note on the copy-trim.
- `src/main.rs` — `Commands::Clip => Err(not_implemented("clip"))` → `Commands::Clip(args) => args.run()`; `not_implemented` import left live for pomodoro/weather.
- `src/commands/mod.rs` — `pub mod clip;` in alpha order (after `bulk_rename`, before `color`).

## Decisions Made
None beyond the locked CONTEXT decisions (D-04/D-05/D-06). Implementation followed the plan's interfaces and the RESEARCH code example (lines 420-437) exactly. Claude's-discretion items resolved: the `--help` trim-note wording (D-05) is phrased as "a single trailing newline is stripped … interior and multiple newlines are preserved … on paste the text is emitted byte-exact."

## Deviations from Plan

None — plan executed exactly as written. The two implementation tasks (1-2) followed the locked interfaces and code example verbatim; the `arboard` `default-features = false` feature trim resolved `set_text`/`get_text` on the first build (no fallback to full defaults needed), and the registry wiring mirrored 05-01's qr slice. No bugs, no missing critical functionality, no blocking issues, no architectural changes.

## Issues Encountered
None. The RED→GREEN cycle went cleanly: in RED the behavior-defining round-trip integration tests failed (copy exited 1 via the still-active `not_implemented("clip")` stub — the main.rs arm is only swapped in GREEN), while the 6 pure trim unit tests passed immediately; after the GREEN wiring the trim + non-UTF-8 tests passed and the live round-trip passed locally. The RED-phase dead-code/unused-import warnings (the helper + `Read`/`Write`/`Context` imports, consumed by the real `run()`) cleared in GREEN; clippy `-D warnings` is clean post-GREEN.

## Verification Status

**Automated (all GREEN):**
- `cargo test --bin box clip` — the 6 `trim_one_trailing_newline` D-05 cases (CRLF→trim, LF→trim, no-terminator no-op, only-one-of-two stripped, empty no-op, interior preserved) pass.
- `cargo test --test clip -- --test-threads=1` — the deterministic non-UTF-8 copy exit-1 (no-panic) test passes; the live round-trip + paste tests are `#[ignore]`d (shared OS clipboard / headless CI).
- `cargo test --test clip -- --ignored --test-threads=1` — run LOCALLY: the live round-trip (`héllo 世界\r\n` copy → `--paste` → `héllo 世界` byte-exact, one CRLF trimmed) and paste-exits-0 both pass.
- `cargo test` — full suite green: 134 bin-unit tests (was 128 + 6 new clip trim) + all integration suites + the `trycmd` help snapshot (unchanged — the `Clip` doc one-liner was preserved), **no regression**. `cli.rs::stub_exits_1_to_stderr` (still pointing at `weather`) stays valid.
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt --check` — clean.

**Task 3 — Human-verify (PENDING — deferred to phase UAT):**
- **CLIP-01 real PS7 copy→paste round-trip is NOT yet human-verified.** Task 3 is a `checkpoint:human-verify` (blocking, physical) gate requiring a real interactive round-trip in PowerShell 7 over the live Win32 clipboard: `"héllo 世界" | box clip`, then **Ctrl+V** into a text field (or `box clip --paste`) confirming correct Unicode with NO extra trailing blank line, reading another program's copied text back (proving paste reads the live OS clipboard), and confirming the copy **persists after the terminal closes** (OS handle ownership, D-06) — all WITHOUT elevation. Per the phase-UAT batching plan (mirroring 05-01's deferred phone-scan and Phase 4's deferred matrix human-verify), this gate is deferred to phase-end UAT and was NOT self-approved.
- **What automated coverage already proves:** the byte-level invariants Task 3 depends on (single-trailing-newline trim, Unicode-preserving programmatic copy→paste round-trip, non-UTF-8 → exit 1 no panic) are all covered above — the live round-trip even passes locally. Only the irreducibly-manual aspects remain for the human: the physical **Ctrl+V** paste into a GUI field, cross-program clipboard read, persistence after terminal close, and the no-elevation confirmation.
- **Do NOT mark CLIP-01 as human-verified until the phase UAT confirms it.**

## User Setup Required
None — no external service configuration. (`box clip` is local, no network, no secrets, no elevation.)

## Next Phase Readiness
- `box clip` + `--paste` are live and fully automated-tested (deterministic parts green; live round-trip green locally); the `arboard` dependency and the text-only clipboard slice are established.
- The 2 remaining Phase-5 stubs (pomodoro/weather) are untouched and independent — each swaps its own unit variant + stub arm; they share only `cli.rs`/`main.rs`/`commands/mod.rs` (sequenced by wave, no same-wave file overlap). The `not_implemented` import remains live for both.
- **Open gate:** CLIP-01 real PS7 copy→paste human-verify pending at phase UAT (see above). Do not mark CLIP-01 as human-verified until then.

## Self-Check: PASSED

- `src/commands/clip/mod.rs` — FOUND
- `tests/clip.rs` — FOUND
- `.planning/phases/05-windows-platform-integration/05-02-SUMMARY.md` — FOUND
- Commit `58f729b` (Task 1, test RED) — FOUND
- Commit `d18c8f5` (Task 2, feat GREEN) — FOUND

---
*Phase: 05-windows-platform-integration*
*Completed: 2026-06-24*
