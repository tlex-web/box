---
phase: 05-windows-platform-integration
plan: 01
subsystem: cli
tags: [qr, qrcode, dense1x2, terminal-render, half-block, clap]

# Dependency graph
requires:
  - phase: 01-foundation-flatten
    provides: "core::input::read_input (arg / - stdin / no-arg-TTY → exit 2), RunCommand trait, 0/1/2 exit policy, init_color gate"
  - phase: 02-pure-transform-utilities
    provides: "core::input arg-vs-stdin precedence + the `-` sentinel (qr's input acquisition)"
provides:
  - "box qr: render a scannable QR for arg/stdin text/URL with pure Unicode half-blocks (QR-01)"
  - "qrcode 0.14.1 (default-features=false) dependency — the Dense1x2 monochrome-render slice"
  - "render_qr(&str) -> Result<String> pure seam: QrCode::with_error_correction_level(_, EcLevel::M)? → Dense1x2 → owned String, no ANSI"
affects: [05-02-clip, 05-03-pomodoro, 05-04-weather]

# Tech tracking
tech-stack:
  added: ["qrcode 0.14.1 (default-features=false — drops image/svg/pic; render::unicode::Dense1x2 always available)"]
  patterns:
    - "Thin orchestrator over a pure render seam: run() acquires input via core::input, delegates encode→render to render_qr(), println!s the owned String"
    - "Monochrome command — NO color path (D-03): qr never touches is_color_on()/owo-colors; output is pure half-blocks, no ANSI even without NO_COLOR"

key-files:
  created:
    - "src/commands/qr/mod.rs"
    - "tests/qr.rs"
  modified:
    - "Cargo.toml (qrcode dep)"
    - "src/cli.rs (Qr unit variant → Qr(QrArgs))"
    - "src/main.rs (not_implemented(qr) arm → args.run())"
    - "src/commands/mod.rs (pub mod qr)"
    - "tests/cli.rs (stub_exits_1_to_stderr: qr → weather)"

key-decisions:
  - "qrcode 0.14.1 + Dense1x2 (D-01) over qr2term — qr2term always emits ANSI + encodes polarity in the background color, breaking the byte-identical-minus-ANSI-when-piped invariant"
  - "EcLevel::M (qrcode default, 15% recovery) FIXED for v1 (D-02); .quiet_zone(true) for the full ISO-18004 4-module border"
  - "default-features=false on qrcode: drops the ['image','svg','pic'] pull-in; the Dense1x2 half-block path needs no feature (RESEARCH A2)"
  - "Monochrome, no color path (D-03); input via core::input::read_input (arg / - stdin / no-arg-TTY → exit 2)"

patterns-established:
  - "Pure render seam as the unit-test boundary: render_qr() is crate-pure and deterministic, so the no-ANSI/non-empty/deterministic invariants are unit-tested without the binary"
  - "Representative-stub test repointing: when a command goes live, the cli.rs stub-behavior test moves to the last-to-be-built stub (weather) so it stays valid through the rest of the phase"

requirements-completed: [QR-01]

# Metrics
duration: 2min
completed: 2026-06-24
---

# Phase 5 Plan 01: box qr (QR-01) Summary

**`box qr` renders a scannable QR for arg/stdin text/URL using qrcode 0.14.1's Dense1x2 Unicode half-block renderer at EcLevel::M with a full quiet zone — pure half-blocks, zero ANSI, owned String to stdout (the qr2term rejection made real).**

## Performance

- **Duration:** ~2 min (implementation), plus full-suite + clippy + fmt verification
- **Started:** 2026-06-24T20:08:49+02:00
- **Completed:** 2026-06-24T20:10:37+02:00
- **Tasks:** 2 of 3 (Task 3 is a human-verify checkpoint — see "Deferred to Phase UAT")
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments
- `box qr "https://example.com"` (and piped stdin) renders a QR with Unicode half-block glyphs (`▀▄█`), exits 0, contains NO ANSI even without `NO_COLOR`.
- `box qr "x" > qr.txt` yields clean half-block text with no escape sequences — the exact reason qr2term was rejected (D-01), now enforced by an automated byte-scan test.
- `qrcode` 0.14.1 dependency added with `default-features = false` (drops image/svg/pic); establishes the monochrome-render slice for the phase.
- Oversized input (exceeding QR capacity) exits 1 cleanly with `error: data too long` — no panic (T-05-QR-DoS verified).
- The `qr` `not_implemented` stub is gone; 3 Phase-5 stubs remain (clip/pomodoro/weather).

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1: Wave-0 test scaffold (RED)** - `4853941` (test) — `pub mod qr`, `QrArgs` + stubbed `render_qr`, in-module unit tests, `tests/qr.rs`, qrcode dep. 3 behavior tests confirmed FAILING.
2. **Task 2: Implement qr slice (GREEN)** - `5f77e1b` (feat) — real `render_qr` chain, `QrArgs::run`, cli.rs/main.rs registry wiring, polarity-flip doc, cli.rs stub-test repoint. All qr unit + integration tests pass.

**Plan metadata:** committed separately with this SUMMARY.

_Task 3 (human-verify phone-scan) carries no commit — it is a manual gate deferred to phase UAT._

## Files Created/Modified
- `src/commands/qr/mod.rs` - `QrArgs` + `render_qr` (qrcode → Dense1x2 → owned String) + `#[cfg(test)] mod tests`; module doc carries the dark-on-light polarity flip instructions (Pitfall QR-1).
- `tests/qr.rs` - Integration: arg-render, piped-stdin-render, no-ANSI-when-piped (run without `NO_COLOR`).
- `Cargo.toml` - `qrcode = { version = "0.14.1", default-features = false }` with the feature-trim rationale comment.
- `src/cli.rs` - `Qr` unit variant → `Qr(crate::commands::qr::QrArgs)` (doc-comment preserved, so the help snapshot is unchanged).
- `src/main.rs` - `Commands::Qr => Err(not_implemented("qr"))` → `Commands::Qr(args) => args.run()`; `not_implemented` import left live for clip/pomodoro/weather.
- `src/commands/mod.rs` - `pub mod qr;` in alpha order (after `passgen`, before `roast`).
- `tests/cli.rs` - `stub_exits_1_to_stderr` repointed from `qr` to `weather` (deviation Rule 1 — see below).

## Decisions Made
None beyond the locked CONTEXT decisions (D-01/D-02/D-03). Implementation followed the plan's interfaces and RESEARCH code example exactly. Chose `with_error_correction_level(_, EcLevel::M)` over `new()` (explicit per D-02; identical EC level either way — Claude's discretion).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Repointed `tests/cli.rs::stub_exits_1_to_stderr` from `qr` to `weather`**
- **Found during:** Task 2 (GREEN — full suite run after wiring the live `qr` command)
- **Issue:** `tests/cli.rs::stub_exits_1_to_stderr` used `box qr` as its representative not-yet-implemented stub, asserting exit 1 + "not yet implemented" on stderr + empty stdout. Making `qr` real (this plan's whole purpose) flipped that command to exit 0 with a rendered QR on stdout, so the test failed with "Unexpected success". The test asserts stale behavior directly caused by this task's change.
- **Fix:** Repointed the test to `box weather` — a command that remains a stub until plan 05-04 (the last Phase-5 command to be built), so the assertion stays valid through the rest of the phase. Updated the doc-comment to explain the choice.
- **Files modified:** `tests/cli.rs`
- **Verification:** `cargo test --test cli` → 7/7 pass; full suite green (no regression).
- **Committed in:** `5f77e1b` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — stale stub-behavior test)
**Impact on plan:** The fix was a necessary consequence of the slice going live (the plan's interfaces note flagged that 3 stubs remain after this plan, which is why `weather` is the correct new target). No scope creep; the trycmd help snapshot needed no change because the `qr` doc-comment one-liner was preserved verbatim.

## Issues Encountered
None. The `qrcode` 0.14.1 `default-features = false` feature trim resolved the `Dense1x2` path on the first build (no fallback to full defaults needed). RED→GREEN cycle went cleanly: 3 behavior tests failed with "not implemented" in RED, all passed after the `render_qr` chain landed in GREEN.

## Verification Status

**Automated (all GREEN):**
- `cargo test --bin box qr` — 3 `render_qr` unit tests (non-empty, no-ANSI, deterministic) pass.
- `cargo test --test qr` — 3 integration tests (arg-render, piped-render, no-ANSI-when-piped) pass.
- `cargo test` — full suite green: 128 bin-unit tests + all integration suites, **no regression**.
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt --check` — clean.
- Manual sanity: `box qr "https://example.com"` renders half-blocks, exit 0, 0 ANSI bytes; oversized input → `error: data too long`, exit 1, no panic.

**Task 3 — Human-verify (PENDING — deferred to phase UAT):**
- **QR-01 phone-scan is NOT yet human-verified.** Task 3 is a `checkpoint:human-verify` (blocking, physical) gate: a phone camera must read `box qr "https://example.com"` rendered in PowerShell 7 on the dark theme and resolve it to the URL. Per the phase-UAT batching plan (mirroring Phase 4's deferred matrix human-verify), this gate is deferred to phase-end UAT and was NOT self-approved.
- **Flip-instructions ready (Pitfall QR-1):** the module doc records the dark-on-light polarity flip — if the phone-scan fails or the QR looks inverted on the dark theme, insert `.dark_color(Dense1x2::Dark).light_color(Dense1x2::Light)` into the `render_qr` chain (between `.quiet_zone(true)` and `.build()`), rebuild, and re-scan.
- **What automated coverage already proves:** the byte-level invariants Task 3 depends on (half-blocks present, no ANSI, exit 0, deterministic, quiet zone via `.quiet_zone(true)`) are all covered above. Only camera-readability (a physical/perceptual property) remains for the human.

## User Setup Required
None — no external service configuration. (`box qr` is local, monochrome, no network, no secrets.)

## Next Phase Readiness
- `box qr` is live and fully automated-tested; the `qrcode` dependency and the monochrome-render slice are established.
- The 3 remaining Phase-5 stubs (clip/pomodoro/weather) are untouched and independent — each swaps its own unit variant + stub arm; they share only `cli.rs`/`main.rs`/`commands/mod.rs` (sequenced by wave, no same-wave file overlap).
- **Open gate:** QR-01 phone-scan human-verify pending at phase UAT (see above). Do not mark QR-01 as human-verified until then.

## Self-Check: PASSED

- `src/commands/qr/mod.rs` — FOUND
- `tests/qr.rs` — FOUND
- `.planning/phases/05-windows-platform-integration/05-01-SUMMARY.md` — FOUND
- Commit `4853941` (Task 1, test RED) — FOUND
- Commit `5f77e1b` (Task 2, feat GREEN) — FOUND

---
*Phase: 05-windows-platform-integration*
*Completed: 2026-06-24*
