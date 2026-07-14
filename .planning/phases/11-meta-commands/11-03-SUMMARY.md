---
phase: 11-meta-commands
plan: 03
subsystem: config
tags: [config, hash, clap, toml, env-precedence, exit-codes, tdd, gap-closure]

# Dependency graph
requires:
  - phase: 11-meta-commands
    provides: "11-01 box config (show/get/set/path) + 11-02 box completions — the modules this plan amends"
  - phase: 06-scriptable-core-foundation
    provides: "core::config resolver (CLI>env>config>builtin), BoxError::Config exit-2 gate, hash BLAKE3-default + BOX_HASH_DEFAULT_ALGO env tier"
provides:
  - "pub(crate) hash::effective_default_algo() — the single env>config>builtin hash-algo resolver, consumed by BOTH run_compute (CLI layered over it) and config's effective view"
  - "core::config::init_config_default() — the config-independent Config::default() fallback initializer"
  - "ConfigArgs::tolerates_malformed_config() — the Path|Set predicate for the malformed-config gate"
  - "command-aware config gate in main.rs: a malformed config falls back to defaults ONLY for completions + config path/set"
affects: [milestone-close, v2.0-completion]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Single shared resolver consumed by both the consumer (run_compute) and the reporter (config view) so a reported value can never drift from the consumed value"
    - "Command-aware startup gate: compute a config_independent predicate from &cli.command BEFORE consuming cli, then fall back to defaults only for the config-independent set"
    - "Shared dispatch(cli) function factored out of main so the normal and fallback paths run the identical LOAD-BEARING spine (init_color→init_output→run→flush_clip)"

key-files:
  created: []
  modified:
    - "src/commands/hash/mod.rs — added pub(crate) effective_default_algo(); run_compute now layers CLI over it"
    - "src/commands/config/mod.rs — effective()/get hash arm route through the shared resolver; added ConfigArgs::tolerates_malformed_config()"
    - "src/core/config.rs — added pub fn init_config_default()"
    - "src/main.rs — command-aware config gate + shared dispatch(cli)"
    - "tests/config_cmd.rs — WR-01 env-parity + 3 WR-02 robustness tests; box_cmd env_remove; BOX_MD5 const"

key-decisions:
  - "WR-01 fixed via ONE pub(crate) hash::effective_default_algo() (not by making parse_algo pub) so BOX_HASH_DEFAULT_ALGO is looked up in exactly one source file"
  - "WR-02 scoped the malformed-config default-fallback to EXACTLY completions + config path/set; config show/get stay config-dependent (exit 2) so no tolerance hole opens"
  - "Kept parse_algo private — config calls the shared resolver, never the string parser"

patterns-established:
  - "Reporter-consumer parity: route a command's effective view and its actual consumption through one pub(crate) resolver"
  - "Config-independent dispatch gate: predicate computed pre-move, default-fallback only for the tolerant set, Err propagated otherwise"

requirements-completed: [CFG-01, CMP-01]

# Metrics
duration: 10min
completed: 2026-07-14
---

# Phase 11 Plan 03: WR-01/WR-02 Gap Closure Summary

**config show/get now report the env-tier value box hash actually consumes (one shared effective_default_algo resolver), and a malformed config.toml no longer bricks the config path/set repair path or shell-start completions — while every normal command and config show/get still exit 2.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-07-14T19:25:59Z
- **Completed:** 2026-07-14T19:35:25Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 5

## Accomplishments
- **WR-01 (env-tier parity):** Factored a single `pub(crate) hash::effective_default_algo()` (env `BOX_HASH_DEFAULT_ALGO` > config `[hash] default_algo` > builtin BLAKE3). `run_compute` now layers the CLI tier over it (`cli_algo.unwrap_or_else(effective_default_algo)`) and config's `effective()`/`get "hash.default_algo"` call the SAME function — so `config show`/`get` can never drift from what `box hash` consumes. `BOX_HASH_DEFAULT_ALGO` is now looked up in exactly one source file.
- **WR-02 (malformed-config decoupling):** Added `core::config::init_config_default()` (idempotent `Config::default()` base) and `ConfigArgs::tolerates_malformed_config()` (true only for `Path`/`Set`). `main.rs` computes `config_independent` from the command before consuming `cli`; on a malformed config it falls back to defaults and dispatches ONLY for `completions` + `config path`/`set`, otherwise propagates the `BoxError::Config` → exit 2. `config set` repairs the file via the unchanged D-03 validate-before-write.
- **No tolerance hole:** under the same malformed config, `box hash` and `box config show`/`get` still exit 2 (pinned by `malformed_config_still_bricks_normal_command`).
- Full suite green (507 passed / 0 failed across all test binaries); `cargo clippy --bin box --all-targets -- -D warnings` clean.

## Task Commits

TDD tasks — RED (failing test) then GREEN (implementation):

1. **Task 1: WR-01 env-tier parity** — RED `7d18396` (test), GREEN `d5728b5` (fix)
2. **Task 2: WR-02 gate decoupling** — RED `137a403` (test), GREEN `12f277a` (fix)
3. **Fmt fixup (own new test code)** — `bde5158` (style)

**Plan metadata:** `8c5139e` (docs: plan) — this SUMMARY committed with the final metadata commit.

## Files Created/Modified
- `src/commands/hash/mod.rs` — new `pub(crate) fn effective_default_algo()`; `run_compute` layers CLI over it (`parse_algo` stays private)
- `src/commands/config/mod.rs` — `effective()` hash arm + `get "hash.default_algo"` arm route through `hash::effective_default_algo()`; added `ConfigArgs::tolerates_malformed_config()`; corrected `effective()`/`EffectiveConfig` doc comments to name the env tier for hash
- `src/core/config.rs` — new `pub fn init_config_default()` (config-independent default-fallback initializer)
- `src/main.rs` — command-aware config gate (`config_independent` predicate + malformed-but-independent fallback); factored the spine into a shared `dispatch(cli)` fn
- `tests/config_cmd.rs` — `BOX_MD5` const; `box_cmd` now `.env_remove("BOX_HASH_DEFAULT_ALGO")`; `env_tier_parity_hash_default_algo` (WR-01); `write_malformed_config` helper + `malformed_config_path_and_completions_exit0` / `malformed_config_set_repairs_file` / `malformed_config_still_bricks_normal_command` (WR-02)

## Decisions Made
- **WR-01 fix shape:** chose the shared-resolver option (a `pub(crate)` function) over making `parse_algo` public, so the env literal `BOX_HASH_DEFAULT_ALGO` lives in exactly one place and the config view provably cannot re-implement (and drift from) the chain. Verified by the source assertion `grep -rl BOX_HASH_DEFAULT_ALGO src/` → only `hash/mod.rs`.
- **WR-02 fix shape:** scoped the default-fallback to exactly the config-independent set (`completions` + `config path`/`set`). `config show`/`get` deliberately return `false` from `tolerates_malformed_config()` because they report the effective value and must not silently run on unparseable config — preserving the missing→silent-default / malformed→exit-2 contract for everything that reads config.
- **Shared `dispatch(cli)`:** factored the `init_color`/`init_output`/match/`flush_clip` body out of the old `and_then` chain so the Ok path and the malformed-independent fallback run the identical LOAD-BEARING spine with no duplication.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] rustfmt drift in newly-authored test code**
- **Found during:** Task 2 (post-implementation fmt verification)
- **Issue:** My new `malformed_config_path_and_completions_exit0` assertion used an unwrapped method chain that rustfmt wanted wrapped — new drift on top of the repo's pre-existing (deferred, out-of-scope) fmt drift.
- **Fix:** Wrapped the `String::from_utf8_lossy(...).trim_end().ends_with(...)` chain to the rustfmt canonical form (whitespace-only).
- **Files modified:** tests/config_cmd.rs
- **Verification:** `rustfmt --edition 2021 --check` no longer flags line 374; the only remaining flag in the file is the pre-existing `config_file` helper (not authored here).
- **Committed in:** `bde5158` (style commit)

---

**Total deviations:** 1 auto-fixed (1 bug — self-authored fmt drift)
**Impact on plan:** Trivial whitespace fix on own new code; no scope creep. Pre-existing repo-wide rustfmt drift (ascii/color/etc.) left untouched per the deferred fmt sweep noted in STATE.md.

## Issues Encountered
- **Source-assertion trap (WR-01):** my first pass named the literal `BOX_HASH_DEFAULT_ALGO` in two `config/mod.rs` doc comments, which would have failed the `grep -rl BOX_HASH_DEFAULT_ALGO src/ → only hash/mod.rs` acceptance assertion. Reworded the doc comments to reference "the env tier" / the shared resolver so the env literal appears in exactly one source file. Resolved before the GREEN commit.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Both Phase-11 code-review warnings (WR-01, WR-02) are now resolved in code with regression tests; VERIFICATION truths #5 and #6 should flip to VERIFIED on re-verification.
- CFG-01 + CMP-01 fully satisfied. Phase 11 is ready for re-verification and milestone close-out.
- No blockers. One out-of-scope follow-up carried forward from prior phases: a repo-root `cargo fmt` sweep for the pre-existing formatting drift (deferred-items.md).

## Self-Check: PASSED

- All 5 modified files present on disk; SUMMARY.md created.
- All 5 task commits (2 RED, 2 GREEN, 1 style) present in git history.

---
*Phase: 11-meta-commands*
*Completed: 2026-07-14*
