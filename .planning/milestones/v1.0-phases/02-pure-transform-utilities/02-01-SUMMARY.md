---
phase: 02-pure-transform-utilities
plan: 01
subsystem: infra
tags: [rust, stdin, clap, anyhow, thiserror, uuid, base64, chrono, rand, is-terminal]

# Dependency graph
requires:
  - phase: 01-foundation-flatten
    provides: "core::errors::BoxError + thiserror; core::output IsTerminal gate shape; main.rs single exit-code owner; RunCommand trait"
provides:
  - "core::input::read_input / read_input_bytes — shared stdin-or-arg reader with the TTY no-block guard (D-04/D-05/D-06)"
  - "BoxError::MissingInput variant + main.rs downcast mapping it to exit code 2 (usage error)"
  - "uuid v4, base64, chrono (clock,std), rand 0.9 dependencies in Cargo.toml — manifest now complete for all of Phase 2"
affects: [base64, cowsay, epoch, color, passgen, hash, json, clip, lolcat]

# Tech tracking
tech-stack:
  added: ["uuid 1.23.3 (v4)", "base64 0.22.1", "chrono 0.4.45 (no-default, clock+std)", "rand 0.9"]
  patterns:
    - "Injectable-decision testing: thread is_tty:bool + Read into an inner resolver so terminal-dependent branches are unit-testable without a real TTY"
    - "Typed-error-to-exit-code: return a BoxError variant via .into() (never a type-erased anyhow macro) so main.rs can downcast and map a specific exit code"

key-files:
  created:
    - "src/core/input.rs — read_input / read_input_bytes + inner resolve/resolve_bytes + 4 unit tests"
  modified:
    - "Cargo.toml — four Phase-2 crates added"
    - "Cargo.lock — locked resolution of the new crates"
    - "src/core/mod.rs — pub mod input; + doc-comment line"
    - "src/core/errors.rs — BoxError::MissingInput variant"
    - "src/main.rs — Err-arm downcast → ExitCode::from(2)"

key-decisions:
  - "chrono added with default-features = false, features = [clock, std] to trim oldtime/wasmbind while keeping Local (D-01)"
  - "MissingInput returned via .into() (typed variant), never anyhow::bail!, so the exit-2 downcast in main.rs works"
  - "Forward-compat #[allow(dead_code)] on core::input public readers + the MissingInput variant until a Wave-2 command becomes a live caller (mirrors Phase-1 allow-then-remove pattern)"
  - "input.rs created as a compile-only placeholder in the Task-1 commit (so cargo build passes once core/mod.rs references it), then fully implemented in the Task-3 commit"

patterns-established:
  - "Injectable is_tty + Read inner resolver makes the TTY-no-block branch deterministically unit-testable"
  - "Typed BoxError variant + main.rs downcast is the single-owner mechanism for non-1 exit codes"

requirements-completed: []

# Metrics
duration: 5min
completed: 2026-06-22
---

# Phase 2 Plan 01: core::input Foundation Slice Summary

**Shared `core::input` stdin-or-arg reader with a TTY no-block guard, a typed `BoxError::MissingInput` mapped to exit 2 by `main.rs`, and the four Phase-2 crates (uuid/base64/chrono/rand) added once so no Wave-2 command plan touches the manifest.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-22T15:53:59Z
- **Completed:** 2026-06-22T15:58:58Z
- **Tasks:** 3
- **Files modified:** 5 (1 created, 4 modified) + Cargo.lock

## Accomplishments
- `core::input` provides both reader shapes — `read_input` (UTF-8 `String`) and `read_input_bytes` (binary-exact `Vec<u8>` via `read_to_end`) — following the D-04 layered precedence (arg → piped stdin → TTY-no-block).
- The interactive-TTY-with-no-arg case returns `BoxError::MissingInput` (never blocks/hangs), which `main.rs` downcasts and maps to exit code 2; all other errors still exit 1; the clap pre-dispatch exit-2 path is untouched.
- The four locked Phase-2 crates resolve and the crate builds clean — `Cargo.toml` is now complete for the entire phase, removing it from every Wave-2 plan's file-ownership footprint.
- Four `core::input` unit tests prove: arg-verbatim, MissingInput-on-TTY downcasts to the typed variant (exit-2 path reachable), `0xFF` byte-exact round-trip (`read_to_end`, not `read_to_string`), and the `-` sentinel reads piped stdin.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add the four locked crates and register the input module** — `1b1d36d` (feat)
2. **Task 2: Add BoxError::MissingInput and the main.rs exit-2 downcast** — `9b70bce` (feat)
3. **Task 3: Implement core::input (read_input / read_input_bytes) with tests** — `a4a4a15` (feat)

_Note: TDD-tagged tasks here are config/foundation slices; their RED/GREEN signal is the build + `cargo test input::` gate rather than separate test→feat commits. The behavior-bearing Task 3 ships its tests in the same commit as the implementation because the unit tests call inner helpers directly (injection) rather than the not-yet-callable public API._

## Files Created/Modified
- `src/core/input.rs` (created) — `read_input` / `read_input_bytes`, inner `resolve` / `resolve_bytes` with injected `is_tty` + `Read`, and 4 unit tests.
- `Cargo.toml` (modified) — added `uuid` (v4), `base64`, `chrono` (no-default, clock+std), `rand` 0.9.
- `Cargo.lock` (modified) — locked resolution of the four crates and transitives (getrandom, rand_core, rand_chacha, ppv-lite86, zerocopy, iana-time-zone).
- `src/core/mod.rs` (modified) — `pub mod input;` + doc-comment line.
- `src/core/errors.rs` (modified) — `BoxError::MissingInput` variant.
- `src/main.rs` (modified) — `Err`-arm downcast of `BoxError::MissingInput` → `ExitCode::from(2)`.

## Decisions Made
- **chrono trimmed:** `default-features = false, features = ["clock", "std"]` keeps `Local` (D-12 needs it) while dropping `oldtime`/`wasmbind` (D-01); verified `cargo build` resolves `Local`.
- **No `rand_core` yet:** deferred to the passgen plan, which will verify whether `use rand::TryRngCore` resolves through rand 0.9 re-exports (RESEARCH A1). Only a NOTE comment references it.
- **Typed return, not `bail!`:** branch 3 returns `Err(BoxError::MissingInput.into())` so the `main.rs` downcast succeeds and yields exit 2; a `bail!` would type-erase and wrongly map to exit 1 (RESEARCH Pitfall 2).
- **Bonus 4th unit test** (`-` sentinel reads piped stdin) added beyond the three the plan named, to lock the `"-"`-means-stdin contract that `read_input` callers rely on.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Forward-compat `#[allow(dead_code)]` on core::input + the MissingInput variant**
- **Found during:** Task 2 (and again Task 3) — the per-task `cargo clippy -- -D warnings` gate.
- **Issue:** `core::input` is a foundation slice committed *before* its callers (the Wave-2 commands). In the non-test bin build, the public `read_input`/`read_input_bytes` have no live caller and `MissingInput` has no live constructor, so clippy's dead-code lint (escalated to error by `-D warnings`) blocked the build. The plan's Task-2 acceptance criterion assumed the `main.rs` *reference* would satisfy the lint, but a downcast pattern-match is a read, not a constructor, so the variant remained "never constructed".
- **Fix:** Added tightly-scoped `#[allow(dead_code)]` (with an explanatory comment on each) to the two public readers, the two inner resolvers, and the `MissingInput` variant — mirroring the Phase-1 forward-compat pattern (STATE.md [01-03]), where such allows are removed once a command becomes a live caller. The allows are documented to come off when the first Wave-2 command (base64/cowsay/epoch/color) consumes `core::input`.
- **Files modified:** src/core/input.rs, src/core/errors.rs
- **Verification:** `cargo clippy -- -D warnings` and `cargo clippy --all-targets -- -D warnings` both exit 0; `cargo test input::` green.
- **Committed in:** the variant allow in `9b70bce` (Task 2); the function allows in `a4a4a15` (Task 3).

**2. [Rule 1 - Bug] Reworded two comments so `grep -c 'bail!' src/core/input.rs` returns 0**
- **Found during:** Task 3 acceptance-criteria check.
- **Issue:** The criterion `grep -c 'bail!' == 0` was tripped by two explanatory comments that contained the literal token `bail!` while explaining we deliberately did *not* use it — a false positive (the code never calls `bail!`).
- **Fix:** Reworded both comments to say "a plain anyhow macro" / "type-erased anyhow error" instead of the literal token, keeping the semantic guidance intact.
- **Files modified:** src/core/input.rs
- **Verification:** `grep -c 'bail!' src/core/input.rs` now returns 0; tests still green.
- **Committed in:** `a4a4a15` (Task 3 commit).

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug).
**Impact on plan:** Both were necessary to pass the plan's own `-D warnings` and `grep` gates on a foundation slice that intentionally lands ahead of its callers. No scope creep — no behavior was added or changed beyond what the three tasks specified.

## Known Stubs / Forward-compat Surface
- `core::input::{read_input, read_input_bytes}` and their inner resolvers are fully implemented and unit-tested but currently have **no live caller** — they are consumed by the Wave-2 command plans (base64/cowsay/epoch/color). This is the intended foundation-slice state, tracked by documented `#[allow(dead_code)]` attributes. Not a data/UI stub: the functions are complete and correct, awaiting consumers. The `#[allow(dead_code)]` attributes must be removed once the first Wave-2 command becomes a live caller (per the in-code comments and the Phase-1 [01-03] precedent).

## Issues Encountered
- The `core::input` placeholder created in Task 1 had to exist as a compiling module (not just a reference) because adding `pub mod input;` to `core/mod.rs` makes `cargo build` require the file. Resolved by writing a doc-only placeholder in the Task-1 commit and replacing it with the full implementation in the Task-3 commit — keeping each task's `cargo build` gate honest and the commits atomic.

## Threat Model Compliance
- **T-02-01 (DoS-self / TTY hang):** mitigated — branch 3 returns `MissingInput` (exit 2) via the `is_terminal()` guard instead of blocking on a read; unit-tested (`missing_input_on_tty_downcasts_to_box_error`).
- **T-02-02 (byte mangling):** mitigated — `read_input_bytes` uses `read_to_end` with no UTF-8 validation; unit-tested with a `0xFF` byte (`piped_bytes_are_byte_exact_including_non_utf8`).
- **T-02-SC (crate supply chain):** accept — all four crates are `[VERIFIED: crates.io registry]` canonical with a committed Cargo.lock; no blocking human checkpoint required.
- No new security surface introduced beyond the plan's threat register. No threat flags.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `Cargo.toml` is complete for all of Phase 2 — Wave-2 command plans (base64/cowsay/epoch/color/uuid/passgen) do not need to touch the manifest.
- `core::input` is ready to be consumed; the first command that uses it should remove the forward-compat `#[allow(dead_code)]` attributes (on the readers it calls and, once `MissingInput` is constructed in the live path, on the variant) to restore the strict dead-code gate.
- End-to-end exit-2 integration coverage lands with the first stdin-consuming command (Wave-2 base64), as the plan's verification section notes.

## Self-Check: PASSED

- FOUND: src/core/input.rs
- FOUND: .planning/phases/02-pure-transform-utilities/02-01-SUMMARY.md
- FOUND commit: 1b1d36d (Task 1)
- FOUND commit: 9b70bce (Task 2)
- FOUND commit: a4a4a15 (Task 3)

---
*Phase: 02-pure-transform-utilities*
*Completed: 2026-06-22*
