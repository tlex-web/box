---
phase: 02-pure-transform-utilities
plan: 02
subsystem: commands
tags: [rust, clap, uuid, base64, stdin, anyhow, tdd, exit-codes]

# Dependency graph
requires:
  - phase: 02-pure-transform-utilities
    plan: 01
    provides: "core::input::read_input_bytes (arg→stdin→TTY-no-block); BoxError::MissingInput→exit 2; uuid/base64 crates in Cargo.toml"
  - phase: 01-foundation-flatten
    provides: "RunCommand trait; main.rs single exit-code owner; cli.rs Commands enum + doc-comment help lock"
provides:
  - "box uuid — v4 UUID generator (-n/--count, --upper); UuidArgs impl RunCommand"
  - "box base64 — encode/decode (--decode, --url-safe) over the 0.22 Engine API; first live core::input::read_input_bytes consumer"
  - "End-to-end proof of the Plan-01 exit-2 wiring: base64 is the first command that constructs BoxError::MissingInput (no-arg interactive TTY)"
  - "Restored strict dead-code gate on the byte input path: #[allow(dead_code)] removed from read_input_bytes/resolve_bytes + BoxError::MissingInput"
affects: [cowsay, epoch, color, passgen, hash, json, clip, lolcat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Command = clap Args struct + thin run() orchestrator + pure helper fns + #[cfg(test)] unit tests (Pattern 1)"
    - "base64 0.22 Engine API: STANDARD / URL_SAFE_NO_PAD engine objects (free fns removed in 0.21); .trim() piped input before decode (Pitfall 3); decoded bytes via stdout().write_all (byte-exact, never a String)"
    - "Anchored ^…$ regex assertions match per trimmed line, not against the raw stdout buffer (which carries a trailing \\n)"

key-files:
  created:
    - "src/commands/uuid/mod.rs — UuidArgs + format_one pure helper + 2 unit tests"
    - "src/commands/base64/mod.rs — Base64Args + encode/decode/engine helpers + 4 unit tests"
    - "tests/uuid.rs — UUID-01 integration tests (v4 regex, -n count, uniqueness, --upper)"
    - "tests/base64.rs — B64-01 integration tests (round-trip, url-safe, garbage→exit 1, empty arg)"
  modified:
    - "src/commands/mod.rs — pub mod uuid; pub mod base64;"
    - "src/cli.rs — Uuid/Base64 unit variants swapped for Args-carrying variants (doc comments byte-identical)"
    - "src/main.rs — Uuid/Base64 dispatch arms swapped from not_implemented to args.run()"
    - "src/core/input.rs — removed #[allow(dead_code)] on read_input_bytes + resolve_bytes (now live via base64)"
    - "src/core/errors.rs — removed #[allow(dead_code)] on BoxError::MissingInput (now constructed in the live base64 no-arg-TTY path)"

key-decisions:
  - "Anchored v4-regex assertions match the single trimmed line (uuid_lines helper), not the raw stdout buffer — raw stdout carries a trailing newline so ^…$ against the whole buffer never matches (test-side Rule 1 fix during GREEN)"
  - "base64 decode reads input as String::from_utf8_lossy + .trim() before engine.decode — any non-UTF-8 byte is already non-alphabet and is rejected cleanly either way; tolerates the piped trailing newline (Pitfall 3)"
  - "Removed the forward-compat allow ONLY from the byte path (read_input_bytes/resolve_bytes) + MissingInput; the String read_input/resolve keep their scoped allow until cowsay/epoch/color make that path live — surgical allow-then-remove per call-graph reachability (STATE.md [01-03] pattern)"
  - "url-safe negative-assertion payload chosen as 0xFB,0xFF,0xBF — forces +// in STANDARD so the -/_ substitution and no-padding are observable"

patterns-established:
  - "TDD per command: test(...) RED commit (fails against the not_implemented stub) → feat(...) GREEN commit (impl + module register + cli/main swap)"
  - "Allow-then-remove is per-item by call-graph reachability, not per-module — a foundation helper's allow comes off the instant ITS path goes live"

requirements-completed: [UUID-01, B64-01]

# Metrics
duration: 5min
completed: 2026-06-22
---

# Phase 2 Plan 02: uuid + base64 Commands Summary

**Two pure-transform commands on the Phase-1/Plan-01 foundation: `uuid` (arg/flag-only v4 generator) and `base64` (the first live `core::input::read_input_bytes` consumer and the end-to-end proof that the Plan-01 no-arg-TTY → exit-2 wiring works), each TDD-driven with RED→GREEN commits, integration + unit tests, and the strict dead-code gate restored on the byte input path.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-22T16:03:58Z
- **Completed:** 2026-06-22
- **Tasks:** 2 (both TDD, 4 commits total: 2 RED + 2 GREEN)
- **Files:** 9 (4 created, 5 modified)

## Accomplishments
- **`box uuid`** generates RFC-4122 v4 UUIDs: bare `box uuid` prints one lowercase line, `-n N`/`--count N` prints N distinct UUIDs, `--upper` prints the uppercase form. `run()` is a thin loop over `uuid::Uuid::new_v4()` with a pure `format_one(uuid, upper)` helper; no `core::input` (uuid takes no input).
- **`box base64`** encodes (default) and decodes (`--decode`) over the 0.22 `Engine` API, selecting `STANDARD` or `URL_SAFE_NO_PAD` (`--url-safe`). It acquires input via `core::input::read_input_bytes` (arg → piped stdin → no-arg-TTY exit 2), encodes without line-wrapping, and on decode trims the piped trailing newline (Pitfall 3) then writes raw bytes via `stdout().write_all` for byte-exact binary output (T-02-04). Malformed alphabet input propagates as `anyhow::Error` → exit 1, no panic (T-02-03).
- **First live `core::input` consumer:** base64 is the command the Plan-01 foundation was waiting for. It constructs `BoxError::MissingInput` on the no-arg interactive-TTY branch (mapped to exit 2 by `main.rs`) and calls `read_input_bytes`, so the forward-compat `#[allow(dead_code)]` came off the byte path (`read_input_bytes`, `resolve_bytes`, `BoxError::MissingInput`) — restoring the strict dead-code lint gate. The String path (`read_input`/`resolve`) keeps its scoped allow until cowsay/epoch/color go live (surgical, per call-graph reachability).
- **Stubs removed:** both `not_implemented("uuid")` and `not_implemented("base64")` arms are gone; both modules registered and dispatched. The `box --help` trycmd snapshot still passes — the `///` doc comments were preserved byte-identical.
- **Tests:** UUID-01 (4 integration) + B64-01 (6 integration + 4 unit) all green; full suite 61/61; `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean.

## Task Commits

Each task was committed atomically following the TDD RED→GREEN cycle:

1. **Task 1 (uuid) RED** — `2799ad1` (test): 4 failing UUID-01 integration tests against the stub.
2. **Task 1 (uuid) GREEN** — `286a168` (feat): UuidArgs + module register + cli/main swap; 4/4 green.
3. **Task 2 (base64) RED** — `ad6a6d1` (test): 6 failing B64-01 integration tests against the stub.
4. **Task 2 (base64) GREEN** — `d9f4123` (feat): Base64Args + read_input_bytes wiring + allow-removal; 6/6 integration + 4/4 unit green.

## Files Created/Modified
- `src/commands/uuid/mod.rs` (created) — `UuidArgs { count, upper }`, `format_one` pure helper, 2 unit tests.
- `src/commands/base64/mod.rs` (created) — `Base64Args { decode, url_safe, input }`, `engine`/`encode`/`decode` helpers, 4 unit tests.
- `tests/uuid.rs` (created) — v4-regex single/`-n 5`/`--upper`/`-n 100`-distinct integration tests.
- `tests/base64.rs` (created) — round-trip (stdin), binary both-engines, arg-no-wrap, url-safe alphabet, garbage→exit 1, empty-arg integration tests.
- `src/commands/mod.rs` (modified) — `pub mod uuid;` + `pub mod base64;`.
- `src/cli.rs` (modified) — `Uuid(UuidArgs)` + `Base64(Base64Args)` variant swaps (doc comments byte-identical, help.trycmd lock preserved).
- `src/main.rs` (modified) — `Commands::Uuid(args) => args.run()` + `Commands::Base64(args) => args.run()`.
- `src/core/input.rs` (modified) — removed `#[allow(dead_code)]` from `read_input_bytes` + `resolve_bytes`; updated comments.
- `src/core/errors.rs` (modified) — removed `#[allow(dead_code)]` from `BoxError::MissingInput`; updated doc.

## Decisions Made
- **Per-line regex matching:** anchored `^…$` v4 assertions match the single trimmed line via the `uuid_lines` helper, not the raw stdout buffer — the buffer carries a trailing `\n`, so `^…$` against the whole capture never matches. Caught during GREEN; the implementation output was always correct.
- **Lossy-then-trim decode:** `decode` interprets input as `String::from_utf8_lossy` + `.trim()`; non-UTF-8 bytes are already non-alphabet and rejected cleanly, and the piped trailing newline (Pitfall 3) is stripped — round-trips stay byte-exact.
- **Surgical allow-removal:** the dead-code allow came off only the byte path that base64 makes live; the String path keeps its allow. Allow-then-remove is per-item by reachability, not per-module.
- **url-safe negative payload:** `0xFB,0xFF,0xBF` forces `+`/`/` and padding under STANDARD, making the `-`/`_` substitution and no-padding observable in both the unit and integration assertions.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] RED test used an over-strict anchored regex against raw stdout**
- **Found during:** Task 1 GREEN (`single_uuid_is_v4` / `upper_is_uppercase_v4` failed even though `box uuid` emitted a correct v4 UUID).
- **Issue:** `predicate::str::is_match("^…$")` was applied to the whole captured stdout, which is `"<uuid>\n"`; with the default (non-multiline) regex, the trailing newline defeats the `$` anchor, so a correct UUID was rejected. The `-n 5`/`-n 100` tests already used the line-splitting `uuid_lines` helper and passed.
- **Fix:** reworked the two single-line tests to assert exit 0 + empty stderr via the assert handle, then match the v4 regex on the single trimmed line from `uuid_lines`. Pure test-side fix; no production change.
- **Files modified:** tests/uuid.rs
- **Verification:** `cargo test --test uuid` 4/4 green.
- **Committed in:** `286a168` (Task 1 GREEN).

**2. [Rule 3 - Blocking] `cargo fmt --check` drift in tests/uuid.rs**
- **Found during:** Task 2 verification (the plan's `cargo fmt --check` quality gate).
- **Issue:** the Task-1 test edits left two statements slightly over rustfmt's width, so `cargo fmt --check` (a phase quality gate) reported a diff and would block.
- **Fix:** ran `cargo fmt` (cosmetic re-wrap only — whitespace/line-breaks, no logic change); re-verified the suite green afterward.
- **Files modified:** tests/uuid.rs
- **Verification:** `cargo fmt --check` exits 0; full suite still 61/61.
- **Committed in:** `d9f4123` (Task 2 GREEN).

---

**Total deviations:** 2 auto-fixed (1 test bug, 1 blocking fmt). No scope creep — both were test/formatting corrections needed to pass the plan's own gates; no command behavior changed.

## Threat Model Compliance
- **T-02-03 (DoS-self / decode panic):** mitigated — `engine.decode` returns `Result`, wrapped with `.context("invalid base64 input")` and propagated via `anyhow` to exit 1; no `.unwrap()` on user input. Unit-tested (`decode_garbage_is_err_not_panic`) + integration-tested (`decode_garbage_exits_1_clean` asserts `.code(1)`, empty stdout, non-empty stderr).
- **T-02-04 (byte corruption):** mitigated — decoded bytes written via `stdout().write_all`, never through a `String`; input read via `read_input_bytes` (`read_to_end`). Unit-tested byte-exact with `0x00`/`0xFF` for both engines + trailing-newline tolerance.
- **uuid (accept):** trivial surface — pure CSPRNG generation, no input/network/storage. No mitigation needed.
- **PS7 pipe boundary (D-06):** byte-exactness of the *PowerShell* pipeline is a shell property `assert_cmd` cannot reproduce (Pitfall 4); the integration tests prove the *Rust* reader is byte-exact and document the scope boundary in `tests/base64.rs`. The PS7 round-trip stays a manual/human-verify item in 02-VALIDATION.md.
- No new security surface beyond the plan's threat register. No threat flags.

## Known Stubs / Forward-compat Surface
- `core::input::{read_input, resolve}` (the **String** readers) remain implemented-but-caller-less with a documented `#[allow(dead_code)]`, awaiting the Wave-2 text commands cowsay/epoch/color. Their byte siblings are now live, so this is the only remaining forward-compat allow in `core::input`. Not a data/UI stub — the functions are complete and unit-tested.
- The seven Phase-2 commands not in this plan (epoch, color, passgen, cowsay, fortune, 8ball, roast) remain `not_implemented(...)` stubs by design; they are the remaining Phase-2 waves.

## User Setup Required
None — no external service or configuration.

## TDD Gate Compliance
Both tasks followed RED→GREEN: a `test(...)` commit (failing against the stub) precedes each `feat(...)` commit (implementation). Verified in git log: `2799ad1` (test) → `286a168` (feat) for uuid; `ad6a6d1` (test) → `d9f4123` (feat) for base64. No REFACTOR commits — both implementations were minimal and clean as written.

## Self-Check: PASSED

- FOUND: src/commands/uuid/mod.rs
- FOUND: src/commands/base64/mod.rs
- FOUND: tests/uuid.rs
- FOUND: tests/base64.rs
- FOUND: .planning/phases/02-pure-transform-utilities/02-02-SUMMARY.md
- FOUND commit: 2799ad1 (Task 1 RED)
- FOUND commit: 286a168 (Task 1 GREEN)
- FOUND commit: ad6a6d1 (Task 2 RED)
- FOUND commit: d9f4123 (Task 2 GREEN)

---
*Phase: 02-pure-transform-utilities*
*Completed: 2026-06-22*
