---
phase: 03-filesystem-power-tools
plan: 01
subsystem: cli-command
tags: [hash, sha256, blake3, sha512, md5, rustcrypto, digest, streaming, checksum, verify]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: "RunCommand trait, BoxError + exit-code downcast in main(), tests/<cmd>.rs assert_cmd convention"
  - phase: 02-pure-transform-utilities
    provides: "core::input is_tty/Read-injectable resolvers + the documented --file extension point (D-06)"
provides:
  - "Live `box hash` command: SHA-256 default, --algo blake3|sha512|md5, path/stdin/`-` input, --verify with 0/1/2 exit contract"
  - "Enum-dispatch streaming Hasher infrastructure (RustCrypto digest 0.11 + native blake3 update_reader) reusable by dupes (Plan 03-04)"
  - "core::input::read_file_or_stdin + ResolvedInput: a streaming --file-ahead-of-stdin reader with a coreutils label"
  - "BoxError::UnsupportedHashLength typed exit-2 variant + its main() downcast mapping"
affects: [dupes, hash-v2, any-future-file-streaming-command]

# Tech tracking
tech-stack:
  added: []  # all deps (blake3, sha2, md-5, const-hex, base16ct) pre-added in e33e6a6; this plan added no manifest lines
  patterns:
    - "Enum-dispatch hasher: one generic hash_rustcrypto<D: Digest> path for sha256/sha512/md5 + a separate native blake3 arm (NO dyn Digest, NO traits-preview)"
    - "Streaming input via a Box<dyn Read> + label struct (ResolvedInput) instead of buffering bytes, for path-or-stdin commands that must not read_to_end a multi-GB payload"
    - "const-hex::encode for digest-0.11 hybrid-array hex (no base16ct alloc feature toggle needed)"

key-files:
  created:
    - "src/commands/hash/mod.rs"
    - "tests/hash.rs"
  modified:
    - "src/core/input.rs"
    - "src/core/errors.rs"
    - "src/main.rs"
    - "src/cli.rs"
    - "src/commands/mod.rs"

key-decisions:
  - "Hex encoding open item resolved with const-hex::encode (already present) rather than enabling base16ct's alloc feature — zero Cargo.toml change"
  - "--file streaming layer returns a Box<dyn Read> + label (ResolvedInput) so hash never buffers the whole payload (T-03-03); positional PATH routes through it to inherit -/stdin/TTY precedence"
  - "ResolvedInput carries a manual Debug impl (Box<dyn Read> is not Debug) printing only the label, so test .unwrap_err() compiles without leaking reader bytes"
  - "--verify mismatch is a plain anyhow bail! (exit 1); only an unsupported length is the typed UnsupportedHashLength variant (exit 2) — Pitfall 1"

patterns-established:
  - "Streaming path-or-stdin input: read_file_or_stdin → ResolvedInput { reader, label }, --file branch ahead of stdin, injectable for unit tests"
  - "Algorithm-by-hex-length auto-detect (algo_from_len): 32→md5, 64→sha256 (wins the tie), 128→sha512, else typed exit-2"

requirements-completed: [HASH-01]

# Metrics
duration: 6min
completed: 2026-06-22
---

# Phase 3 Plan 01: hash Summary

**Live `box hash`: a streaming enum-dispatch checksum tool (SHA-256 default; blake3/sha512/md5 via `--algo`) with length-autodetecting `--verify` and a 0/1/2 exit contract, plus the `core::input` `--file` streaming layer it is the first consumer of.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-06-22T20:07:48Z
- **Completed:** 2026-06-22T20:14:01Z
- **Tasks:** 2 (Task 2 TDD)
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments
- `box hash` is a fully live command: SHA-256 default, `--algo blake3|sha512|md5`, path / piped-stdin / `-` input, and `--verify` exiting 0 (match) / 1 (mismatch) / 2 (unsupported length) — all 7 HASH-01 integration tests green.
- Stood up the reusable enum-dispatch streaming `Hasher` infra (RustCrypto incremental `update` + native `blake3::Hasher::update_reader`) that `dupes` (Plan 03-04) will reuse for its content-equality hash.
- Implemented the deferred `--file PATH` input layer (`core::input::read_file_or_stdin` → `ResolvedInput`) as a streaming `Box<dyn Read>` + coreutils label, ahead of the stdin branch, inheriting the `-`-sentinel and `MissingInput`→exit-2 semantics — `hash` is its first consumer.
- Added the typed `BoxError::UnsupportedHashLength` exit-2 variant and extended the `main()` downcast arm to map it (alongside `MissingInput`) to exit 2.

## Task Commits

1. **Task 1: Wave-0 hash tests + --file input branch + exit-2 variant** - `84f95e7` (test) — the RED gate
2. **Task 2: hash command — streaming Hasher enum + --verify + wiring** - `a10a72b` (feat) — the GREEN gate

**Plan metadata:** (final docs commit — this SUMMARY + STATE + ROADMAP + REQUIREMENTS)

_TDD gate sequence verified in git log: `test(03-01)` (RED) → `feat(03-01)` (GREEN). No REFACTOR commit was needed — the GREEN implementation was already clean (clippy `-D warnings` + `fmt --check` pass)._

## Files Created/Modified
- `src/commands/hash/mod.rs` (created) - `HashArgs` + `Algo` ValueEnum + enum-dispatch `Hasher` (`hash_rustcrypto` generic + native `hash_blake3` arm) + `algo_from_len` + `--verify` logic + `RunCommand` impl + co-located known-answer unit tests
- `tests/hash.rs` (created) - 7 HASH-01 known-answer + exit-code integration tests (b"box" vectors), `NO_COLOR=1`
- `src/core/input.rs` - `read_file_or_stdin` + `ResolvedInput` (streaming `--file`-ahead-of-stdin reader) + 4 new `--file`-branch unit tests
- `src/core/errors.rs` - `UnsupportedHashLength { len }` typed variant
- `src/main.rs` - downcast arm extended so `MissingInput` + `UnsupportedHashLength` both map to exit 2; `Commands::Hash(args) => args.run()`
- `src/cli.rs` - `Hash` variant now carries `HashArgs` (was a unit stub)
- `src/commands/mod.rs` - registered `pub mod hash;` (alphabetical)

## Decisions Made
- **Hex encoding:** Used `const-hex::encode` (already a dependency) for the RustCrypto digest-0.11 hybrid-array output rather than enabling `base16ct`'s `alloc` feature — resolves the plan's open item with zero `Cargo.toml` change. Probed independently: `const_hex::encode(Sha256::finalize)` matches `sha256sum`.
- **Streaming `--file`:** Modeled the new layer as `ResolvedInput { reader: Box<dyn Read>, label: String }` rather than returning bytes, so `hash` streams (no whole-file `read_to_end` — T-03-03). The positional `PATH` routes through it, inheriting `-`/stdin/TTY precedence from the existing resolvers.
- **`--verify` exit mapping:** A well-formed-but-mismatched hash is a plain `bail!` (exit 1); only an *unsupported length* is the typed `UnsupportedHashLength` (exit 2). This keeps the Pitfall-1 distinction crisp.
- **64-tie:** `algo_from_len(64)` returns `Sha256` (wins the sha256/blake3 tie); `--algo blake3` is the only way to verify a 64-hex blake3 digest, as specified by D-04.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `ResolvedInput` needed a manual `Debug` impl**
- **Found during:** Task 1 (unit tests for the `--file` branch)
- **Issue:** `resolve_reader(...).unwrap_err()` in a test requires `ResolvedInput: Debug`, but the struct holds a `Box<dyn Read>` which is not `Debug`, so `#[derive(Debug)]` won't compile.
- **Fix:** Added a hand-written `impl Debug for ResolvedInput` that prints only the `label` (via `finish_non_exhaustive()`), never the reader's bytes.
- **Files modified:** src/core/input.rs
- **Verification:** `cargo test --bin box core::input` — all 8 tests green.
- **Committed in:** `84f95e7` (Task 1 commit)

**2. [Rule 3 - Blocking] Verify command adjusted for a binary-only crate**
- **Found during:** Task 1 verification
- **Issue:** The plan's verify command `cargo test --lib core::input` fails — `box` is a binary crate (`[[bin]]`, no `[lib]`), so there is no `--lib` target (`error: no library targets found in package 'box'`).
- **Fix:** Ran the equivalent `cargo test --bin box core::input` to exercise the same in-module unit tests. No code change; only the invocation differed.
- **Files modified:** none
- **Verification:** `cargo test --bin box core::input` → `test result: ok. 8 passed`.
- **Committed in:** n/a (process-only deviation)

### Setup-context note (not a code deviation)

- The phase dependencies (`blake3`, `sha2`, `md-5`, `rayon`, `regex`, `base16ct`, `const-hex`) were already present in `Cargo.toml`/`Cargo.lock` from the prior setup commit `e33e6a6`, exactly as the sequential-execution brief stated. This plan added **no** manifest lines (the `base16ct` `alloc` open item was resolved by using `const-hex` instead), so the `Cargo.toml` was left untouched — treated as "already done."
- `.planning/STATE.md` arrived with a pre-existing uncommitted edit from the planning session (the orchestrator's position/focus update). It was left for the final docs commit, where it is reconciled by hand alongside the new state advance (per MEMORY: gsd-sdk state handlers can corrupt STATE.md).

---

**Total deviations:** 2 auto-fixed (both Rule 3 - blocking; one a 4-line manual `Debug` impl, one a verify-command substitution for a binary crate).
**Impact on plan:** Both were mechanical unblocks with no scope or design change. The `Debug` impl is the minimal correct fix; the verify substitution exercises identical tests.

## Issues Encountered
- None beyond the two blocking unblocks above. The known-answer vectors (`b"box"` digests) were computed independently via coreutils + a one-off blake3/const-hex probe, confirming the implementation against external references before asserting them in tests.

## User Setup Required
None - no external service configuration required. `box hash` works offline against the local filesystem and stdin.

## Next Phase Readiness
- The enum-dispatch streaming `Hasher` (specifically the native `blake3::Hasher::update_reader` arm) is ready for `dupes` (Plan 03-04) to reuse for content-equality hashing.
- `core::input::read_file_or_stdin` / `ResolvedInput` is available as the streaming input pattern for any later file command.
- No blockers introduced. ROADMAP Phase-3 success criterion #1 (the `box hash` contract) is met.

## TDD Gate Compliance
- RED gate: `84f95e7` (`test(03-01): ...`) — `tests/hash.rs` committed failing against the stub (6/7 red).
- GREEN gate: `a10a72b` (`feat(03-01): ...`) — implementation lands; all 7 tests green.
- REFACTOR: not required (GREEN was already clippy/fmt clean).

## Self-Check: PASSED

- FOUND: src/commands/hash/mod.rs
- FOUND: tests/hash.rs
- FOUND: .planning/phases/03-filesystem-power-tools/03-01-SUMMARY.md
- FOUND commit: 84f95e7 (Task 1, RED)
- FOUND commit: a10a72b (Task 2, GREEN)

---
*Phase: 03-filesystem-power-tools*
*Completed: 2026-06-22*
