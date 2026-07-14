---
phase: 11-meta-commands
reviewed: 2026-07-14T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/commands/config/mod.rs
  - src/commands/hash/mod.rs
  - src/core/config.rs
  - src/main.rs
  - tests/config_cmd.rs
findings:
  critical: 0
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Phase 11: Code Review Report (gap-closure 11-03 re-review)

**Reviewed:** 2026-07-14
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Scoped re-review of the Phase-11 gap-closure plan 11-03, which set out to close the
two prior findings:

- **WR-01 (env-tier parity)** — factor a single `hash::effective_default_algo()`
  (env `BOX_HASH_DEFAULT_ALGO` > config > builtin) consumed by both `hash::run_compute`
  (CLI layered over it) and config's `effective()` / `get "hash.default_algo"`.
- **WR-02 (config-gate decoupling)** — let config-independent commands (`completions`,
  `config path`/`set`) run under a malformed config via `init_config_default()` +
  `ConfigArgs::tolerates_malformed_config()`, while keeping malformed → exit 2 for every
  normal command and for `config show`/`get`.

**Both prior findings are genuinely resolved.** The precedence resolution is correct and
now single-sourced through `effective_default_algo` (env literal appears in exactly one
source file — `src/commands/hash/mod.rs`); the CLI-over-resolver layering is
algebraically identical to the former inline `.or()` chain. The malformed-config
decoupling has no tolerance hole: `Show`/`Get` correctly return `false` from
`tolerates_malformed_config`, so they and every normal command still abort at exit 2, and
the tests in `tests/config_cmd.rs` pin all four axes (path/completions tolerate, set
repairs, normal command + `config show` still brick). No panic path was introduced:
`config()` is only reachable through `dispatch`, which runs only after `CONFIG` is set
(either `init_config` Ok or the `init_config_default` fallback); the config-dependent
error path returns before `dispatch` and never calls `config()`.

No blockers. One warning: the WR-01 refactor **left `core::config::resolve_algo` behind as
orphaned dead code whose doc comment is now factually false and which re-encodes the same
CLI>env>config>BLAKE3 precedence** `effective_default_algo` now owns — a latent drift risk
that partially undercuts WR-01's own single-source goal. Three carried-forward INFO
papercuts remain in the in-scope files.

> **Scope note.** The prior full-phase review (10 findings) also carried WR-03
> (`src/core/fs.rs` `atomic_write` fixed temp name) and WR-04 (`install.ps1` smoke test).
> Those files were **not** part of the 11-03 gap-closure and are **outside this scoped
> re-review's file set**, so they are not re-evaluated here and should be considered still
> open per the earlier report — not silently resolved.

## Verification of prior findings

- **WR-01 — RESOLVED.** `effective_default_algo()` (`src/commands/hash/mod.rs:144-150`)
  implements env > config > builtin; `run_compute` layers CLI via
  `cli_algo.unwrap_or_else(effective_default_algo)` (`:320`); `effective()`
  (`src/commands/config/mod.rs:135`) and the `get "hash.default_algo"` arm (`:184`) both
  call the same function. Grep confirms `BOX_HASH_DEFAULT_ALGO` is looked up in exactly
  one source file. `env_tier_parity_hash_default_algo` (`tests/config_cmd.rs:289-326`)
  proves `config get`, `config show --json`, and `box hash` all agree on `md5`.
- **WR-02 — RESOLVED.** `main.rs:75-105` computes `config_independent` from
  `Commands::Completions` / `ConfigArgs::tolerates_malformed_config()`
  (`src/commands/config/mod.rs:78-84`, true only for `Path`/`Set`), then falls back to
  `init_config_default()` only on the malformed-config Err path for those commands, else
  propagates the `BoxError::Config` → exit 2. `malformed_config_still_bricks_normal_command`
  (`tests/config_cmd.rs:427-462`) locks the no-hole guarantee for a normal command and for
  `config show`.

## Warnings

### WR-01: `core::config::resolve_algo` is orphaned dead code with a false doc comment, re-encoding the precedence `effective_default_algo` now owns

**File:** `src/core/config.rs:300-305` (and stale references at `:15`, `:206`)
**Issue:** After the WR-01 refactor, the live resolver for `hash.default_algo` is
`hash::effective_default_algo()`. `resolve_algo` is now called by **nothing** in the
binary — only by its own `precedence_matrix` unit test (confirmed by grep across `src/`;
the `core::cache` reference is a comment, not a call). Two concrete defects:

1. **The doc comment is factually false.** It reads:
   ```rust
   /// Forward-compat `#[allow(dead_code)]`: exercised by the `precedence_matrix` unit
   /// test now; `hash` adopts it as the live resolver in Plan 06-02 (allow removed there).
   #[allow(dead_code)]
   pub fn resolve_algo(cli: Option<Algo>, env: Option<Algo>, cfg: Option<Algo>) -> Algo {
       cli.or(env).or(cfg).unwrap_or(Algo::Blake3)
   }
   ```
   `hash` never adopted `resolve_algo`, and the `#[allow(dead_code)]` was never removed —
   it is on the very next line. Comments at `:15` ("See `resolve_algo` for the canonical
   resolver shape") and `:206` ("mirroring `resolve_algo`") also now point maintainers at
   the wrong function as canonical.
2. **Duplicated precedence / drift risk.** `resolve_algo` and `effective_default_algo`
   both hardcode the `Algo::Blake3` builtin and the CLI>env>config ordering. WR-01's
   stated goal was to single-source the chain so the config view "provably cannot
   re-implement (and drift from)" it — but a second, independent encoding of that exact
   chain now sits in `config.rs`. Changing the builtin default (or the ordering) requires
   editing both, or they drift — the precise failure class WR-01 was closing, reintroduced
   on the builtin-default axis. This is pre-existing (flagged as 06-REVIEW IN-02) but was
   aggravated, not addressed, by 11-03.
**Fix:** Delete `resolve_algo` (and its `precedence_matrix` test, or repoint that test at
`effective_default_algo`), since `effective_default_algo` is now the single canonical
resolver; update the `:15` / `:206` doc references to name it. If `resolve_algo` is kept
as a pure/testable core, have `effective_default_algo` *call* it
(`resolve_algo(None, env, config().hash.default_algo)`) so the `Algo::Blake3` builtin and
the ordering live in exactly one place, and correct the false "allow removed there"
comment.

## Info

### IN-01: `config get`'s hardcoded key `match` can drift from `SETTABLE_KEYS`

**File:** `src/commands/config/mod.rs:179-206`
**Issue:** `get` matches the three key literals inline with a `_ => Err(unknown_key(key))`
catch-all, while `set`/`set_value` derive acceptance from the `SETTABLE_KEYS` constant
(`src/core/config.rs:207`). Adding a fourth settable key to `SETTABLE_KEYS` (and `set`)
without adding a `get` arm would make `box config get <newkey>` report it as unknown even
though `set` accepts it — a silent get/set divergence. Carried forward from the prior
review; still present in this in-scope file.
**Fix:** Add a unit test asserting every `SETTABLE_KEYS` entry resolves under `get`, or
drive `get` from a single key→resolver table shared with the registry.

### IN-02: enum config values are case-sensitive while the env tier is case-insensitive

**File:** `src/core/config.rs:284-289` (re-parse validation) vs `src/commands/hash/mod.rs:132-134` (`parse_algo`)
**Issue:** `build_config_toml` validates the spliced value via
`toml::from_str::<Config>`, which for `Algo`/`Units` uses `rename_all = "lowercase"` —
a case-sensitive match. So `box config set hash.default_algo SHA256` fails exit 2, while
`BOX_HASH_DEFAULT_ALGO=SHA256` succeeds (`parse_algo` uses `Algo::from_str(s, true)`,
case-insensitive). The rejection is loud (nothing written — no lockout), so this is a UX
papercut, not a correctness bug. Carried forward.
**Fix:** Normalize enum-valued keys through the same `ValueEnum::from_str(_, true)` parser
before validation, or document that config values must be lowercase.

### IN-03: `serde_str` silently renders an empty string on non-string serialization

**File:** `src/commands/config/mod.rs:147-152`
**Issue:** `serde_str` does `.and_then(|v| v.as_str()...).unwrap_or_default()`; a future
config value that serializes to a non-string (number/bool/struct) would make
`config show` print `key = ` (empty) with no error rather than failing loudly. Safe for
today's string-only `Algo`/`Units` enums, but a latent silent-failure landmine. Carried
forward.
**Fix:** Fall back to `serde_json::to_string(v)` so a non-string still shows something
diagnostic, or `debug_assert!` on the `None` branch.

---

_Reviewed: 2026-07-14_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
