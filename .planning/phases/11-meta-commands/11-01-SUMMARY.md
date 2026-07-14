---
phase: 11-meta-commands
plan: 01
subsystem: cli
tags: [config, toml, serde, clap-subcommand, spine, json]

# Dependency graph
requires:
  - phase: 06-scriptable-core-foundation
    provides: "core::config typed nested Config schema, config_path(), load()/toml::from_str::<Config>, the CLI>env>config>builtin resolver, BoxError::Config"
  - phase: 06-scriptable-core-foundation
    provides: "core::output spine — is_json_on()/emit_json/out_line, JSON purity (one doc, no BOM/ANSI)"
  - phase: 07-spine-rollout
    provides: "the frozen uuid {results,count} + is_json_on()-first fork template"
provides:
  - "box config show|get <key>|set <key> <value>|path — read/edit/locate %APPDATA%\\box\\config.toml"
  - "core::config::set_value (validate-before-write) + SETTABLE_KEYS registry + Serialize on Config"
  - "core::fs::atomic_write (temp-write + rename replace, parent-dir create)"
  - "BoxError::ConfigUsage (exit-2 usage error for unknown config keys / bad set values)"
  - "the CLI's first NESTED #[command(subcommand)] (ConfigArgs/ConfigCommand)"
affects: [11-02-completions, config, meta-commands]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Nested clap subcommand: an Args struct carrying #[command(subcommand)] one level below the top-level parser (D-01)"
    - "Validate-before-write: reconstruct doc → re-parse via the SAME toml::from_str::<Config> startup uses → write nothing on any error (D-03)"
    - "Closed settable-key registry (SETTABLE_KEYS) gating get/set + the 'did you mean' text (D-04)"
    - "Human/JSON parity via one serde spelling: human rows read serde_json::to_value(&enum) so they can't drift from emit_json (D-06)"
    - "get exit-1 as a git-style data signal: std::process::exit(1) (NOT a BoxError) so stderr stays silent"

key-files:
  created:
    - src/commands/config/mod.rs
    - tests/config_cmd.rs
  modified:
    - src/core/config.rs
    - src/core/fs.rs
    - src/core/errors.rs
    - src/commands/weather/mod.rs
    - src/commands/mod.rs
    - src/cli.rs
    - src/main.rs
    - tests/cmd/help.trycmd

key-decisions:
  - "Factored a pure build_config_toml(base, key, value) core out of set_value so the D-03/D-04 validation contract is unit-testable without init_config()/config()/a real file; set_value = build_config_toml(config(),..) then atomic_write"
  - "Suppressed empty [hash]/[weather] tables via HashConfig/WeatherConfig::is_empty + skip_serializing_if on the sub-table fields, so a single-key set writes only the touched table (Claude's Discretion within D-02)"
  - "Reused BoxError::Config for bad-VALUE round-trip failures and added a distinct BoxError::ConfigUsage for unknown-KEY usage errors; both map to exit 2, get exit-1 stays a non-BoxError process::exit"
  - "Added serde::Serialize to weather::Units (it only derived Deserialize) — required for WeatherConfig/Config to derive Serialize"

patterns-established:
  - "core::fs::atomic_write: crash-safe config replace (temp sibling <path>.tmp → fs::rename over target)"
  - "config show/get read the effective config through config().<table>.<field>.unwrap_or(builtin) — the same resolution commands consume"

requirements-completed: [CFG-01]

# Metrics
duration: 14min
completed: 2026-07-14
---

# Phase 11 Plan 01: box config Meta-Command Summary

**`box config show|get|set|path` reads/edits/locates `%APPDATA%\box\config.toml` with validate-before-write (a self-inflicted exit-2 lockout is structurally impossible), `--json` on show/get, and the 0/1/2 exit contract — the CLI's first nested subcommand.**

## Performance

- **Duration:** ~14 min
- **Started:** 2026-07-14T17:59:37Z
- **Completed:** 2026-07-14T18:13:59Z
- **Tasks:** 3
- **Files modified:** 10 (2 created, 8 modified)

## Accomplishments
- `box config` nested subcommand — `show` (effective resolved config, `--json` nested doc), `get <key>` (D-07 three-case exit 0/1/2), `set <key> <value>` (validate-before-write atomic persist), `path` (locate, never read).
- `core::config::set_value` + a pure `build_config_toml` core: reject unknown keys (`ConfigUsage`, exit 2) → splice into a `toml::Table` reconstruction (preserves other keys) → re-parse via the SAME `toml::from_str::<Config>` startup uses → `atomic_write` only a validated document. T-11-02 self-lockout is structurally impossible.
- `core::fs::atomic_write` — crash-safe temp-write + `fs::rename` replace with parent-dir create (T-11-01).
- Full CLI/dispatch/exit wiring: `Config(ConfigArgs)` after `Weather`, `Commands::Config` dispatch arm, `BoxError::ConfigUsage` in the exit-2 downcast, `config` added to the help snapshot (24 commands).
- D-06 human/JSON parity proven — `show` and `show --json` render the same resolved values from one serde spelling.

## Task Commits

TDD Task 1 ran a RED → GREEN cycle; Tasks 2–3 one commit each:

1. **Task 1 (RED): failing tests for set_value + atomic_write** - `352a53d` (test)
2. **Task 1 (GREEN): implement set_value + atomic_write** - `735eaae` (feat)
3. **Task 2: wire box config command + CLI/dispatch/exit + help snapshot** - `d5edd1e` (feat)
4. **Task 3: box config integration proofs** - `54e0880` (test)

**Plan metadata:** committed separately (docs: complete plan).

## Files Created/Modified
- `src/commands/config/mod.rs` (NEW) - ConfigArgs + nested ConfigCommand; show/get/set/path handlers with the is_json_on()-first fork.
- `tests/config_cmd.rs` (NEW) - 10 black-box proofs (show JSON purity, D-06 parity, get 0/1/2, set validate-before-write, SC2 hash round-trip, path locate-not-read).
- `src/core/config.rs` - Serialize + skip_serializing_if on Config/HashConfig/WeatherConfig (+ is_empty sub-table suppression), config_path made pub, SETTABLE_KEYS, set_value + build_config_toml, 6 unit tests.
- `src/core/fs.rs` - atomic_write + 2 unit tests.
- `src/core/errors.rs` - BoxError::ConfigUsage variant + exit-2 doc.
- `src/commands/weather/mod.rs` - derive serde::Serialize on Units.
- `src/commands/mod.rs` - pub mod config (alphabetical).
- `src/cli.rs` - Config(ConfigArgs) variant (nested subcommand) after Weather.
- `src/main.rs` - Config dispatch arm + ConfigUsage exit-2 downcast entry.
- `tests/cmd/help.trycmd` - config line after weather.

## Decisions Made
- **Pure `build_config_toml` core** extracted from `set_value` so the validation contract is unit-testable without `init_config()`/`config()`/disk. `set_value` stays the public API commands call.
- **Empty sub-table suppression** (`is_empty` + `skip_serializing_if` on the `[hash]`/`[weather]` fields) so `set weather.units imperial` writes only `[weather]\nunits = "imperial"` — no stray empty `[hash]` header (D-02 minimal write, Claude's Discretion; empty tables re-parse clean either way).
- **Two error variants:** `BoxError::Config` for bad-value round-trip failures (reused), new `BoxError::ConfigUsage` for unknown-key usage errors; both exit 2. `get` unset-no-default is deliberately NOT a `BoxError` — it `std::process::exit(1)`s so stderr stays silent (the git-style "not set" signal).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `serde::Serialize` to `weather::Units`**
- **Found during:** Task 1 (config.rs Serialize derives)
- **Issue:** `WeatherConfig`/`Config` cannot derive `serde::Serialize` unless `Units` implements `Serialize`; it previously derived only `Deserialize`. The plan's Task 1 action ("Add serde::Serialize to Config, HashConfig, WeatherConfig") is unbuildable without this.
- **Fix:** Added `serde::Serialize` to the `Units` derive list (mirroring `hash::Algo`, which already derived both). Also serves Task 2's `config show --json` effective doc.
- **Files modified:** src/commands/weather/mod.rs
- **Verification:** `cargo build` + config.rs Serialize derive compile; the existing weather tests still pass.
- **Committed in:** 352a53d (Task 1 RED commit, as a compile prerequisite)

---

**Total deviations:** 1 auto-fixed (1 blocking).
**Impact on plan:** Necessary for the plan's own Serialize directive to compile. No scope creep.

## Issues Encountered
- **Repo-wide rustfmt drift (pre-existing, out of scope):** `cargo fmt --check` reports diffs across ~100 files (nearly the entire repo, including files this plan never touched). The diffs are a rustfmt version mismatch — e.g. rustfmt flags a `matches!(...)` line in my code that is byte-identical to the already-committed `malformed_maps_to_config_error` test. My new/edited files follow the repo's actual committed style; reformatting only them would make them the inconsistent ones. This is already tracked as a deferred repo-root `cargo fmt` sweep (STATE.md / deferred-items). The load-bearing gates — `cargo test` (full suite, 30 test binaries) and `cargo clippy --all-targets -D warnings` — are both clean.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CFG-01 delivered and locked by green unit + integration + trycmd tests; `clap::Cli` now carries the final `config` surface.
- Plan 11-02 (`completions`, CMP-01) is ready — it generates against the live `Cli::command()`, which now includes `config`. Only `completions` remains for Phase 11.
- Full `cargo test` green (30 test binaries), `cargo clippy --all-targets -D warnings` clean.

## Self-Check: PASSED
- Created files exist: `src/commands/config/mod.rs`, `tests/config_cmd.rs`.
- Commits exist: `352a53d`, `735eaae`, `d5edd1e`, `54e0880`.

---
*Phase: 11-meta-commands*
*Completed: 2026-07-14*
