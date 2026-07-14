---
phase: 06-scriptable-core-foundation
plan: 01
subsystem: infra
tags: [rust, clap, serde, toml, dirs, arboard, json, clipboard, config, spine]

# Dependency graph
requires:
  - phase: 05-windows-platform-integration
    provides: v1.0 base — core::output COLOR_ON triad, clip/mod.rs arboard discipline, BoxError exit-2 downcast, 0/1/2 exit contract
provides:
  - core::output spine primitives (JSON_ON/CLIP_ON atomics, CLIP_BUF, is_json_on, init_output, emit_json, out_line, flush_clip)
  - core::config NEW module (Config{default_hash_algo}, init_config, config(), load(), config_path(), resolve_algo()) via OnceLock
  - BoxError::Config{path,message} exit-2 variant
  - global --json/--clip bools on Cli
  - main.rs spine wiring (init_config -> init_color -> init_output -> dispatch -> flush_clip-on-Ok), unified exit-code routing
affects: [06-02-uuid-hash-pilots, 07-spine-rollout, 11-meta-commands]

# Tech tracking
tech-stack:
  added: [toml 1.1.2, dirs 6.0.0]
  patterns:
    - "Process-global flag -> atomic/OnceLock -> reader-fn (COLOR_ON triad cloned to JSON_ON/CLIP_ON and promoted to OnceLock<Config>)"
    - "Option<T> + .or().or().unwrap_or() precedence resolver (CLI > env > config > builtin) by construction"
    - "Unified result chain: config-load Err AND dispatch Err share one exit-code downcast; flush_clip chained on the Ok branch only"
    - "TOCTOU-free missing-file handling (match ErrorKind::NotFound, never exists()-then-read)"

key-files:
  created:
    - src/core/config.rs
    - tests/config.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/core/output.rs
    - src/core/errors.rs
    - src/core/mod.rs
    - src/cli.rs
    - src/main.rs
    - src/commands/hash/mod.rs
    - tests/cmd/help.trycmd

key-decisions:
  - "config_path() reads %APPDATA% env var FIRST, dirs::config_dir() only as fallback (Rule 1 deviation — dirs 6.0 SHGetKnownFolderPath ignores APPDATA, breaking per-process config isolation)"
  - "Env-tier spelling locked to BOX_HASH_DEFAULT_ALGO; the .or(env) slot is DEFERRED to 06-02's hash compute-default flip"
  - "Algo: Deserialize + rename_all=lowercase added here as the minimal cross-plan edit (Serialize derive lands in 06-02)"
  - "Clipboard confirmation wording: \"Copied to clipboard\" (stderr-only, TTY-gated on STDERR)"
  - "SPINE-05 marked Complete (06-01); SPINE-01/SPINE-03 stay In-progress until the pilots adopt the spine in 06-02"

patterns-established:
  - "emit_json: one pretty serde document, no BOM, single trailing newline, never ANSI — the shared --json serializer for all 23 commands"
  - "out_line: primary-output print that tees lines into CLIP_BUF under --clip — replaces println! in clip-capable commands"
  - "flush_clip: single-shot main-thread arboard, empty->no-op, trim_end once, stderr-only TTY-gated confirmation"

requirements-completed: [SPINE-05]

# Metrics
duration: ~10 min
completed: 2026-06-25
---

# Phase 6 Plan 01: Scriptable-Core Foundation (Spine + Config) Summary

**The frozen scriptable spine: `emit_json`/`out_line`/`flush_clip` output primitives, a hand-rolled `toml`+`dirs` config resolver with strict CLI>env>config>builtin precedence, the `BoxError::Config` exit-2 variant, and global `--json`/`--clip` flags wired into `main.rs` — all unit/integration tested with NO command consuming it yet.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-06-25T00:00Z (approx)
- **Completed:** 2026-06-25T00:12Z
- **Tasks:** 3 (all `tdd="true"`)
- **Files modified:** 11 (2 created, 9 modified)

## Accomplishments

- **`core::output` spine primitives** — `JSON_ON`/`CLIP_ON` atomics + `CLIP_BUF` mutex, cloning the existing `COLOR_ON` triad; `emit_json` (one pretty serde doc, no BOM, single trailing newline, no ANSI), `out_line` (clip-tee print), `flush_clip` (single-shot main-thread arboard, empty→no-op, stderr-only confirmation), `init_output` (forces color off under json||clip, runs after `init_color`).
- **`core::config` NEW module** — `Config{default_hash_algo: Option<Algo>}` via `OnceLock`, `init_config`/`config()`/`load()`/`config_path()`, and the pure `resolve_algo()` precedence resolver (CLI > env > config > built-in BLAKE3). Missing file → silent default (TOCTOU-free `NotFound` match); malformed/unknown-key → `BoxError::Config` → exit 2.
- **`BoxError::Config{path,message}`** exit-2 variant joined to the `main()` downcast.
- **Global `--json`/`--clip`** bools on `Cli` (mirroring `--no-color`), and `main.rs` restructured so `init_config` → `init_color` → `init_output` → dispatch → `flush_clip` all fold into one `result` chain with unified exit-code routing (config-load Err and dispatch Err share the same `error:` print + downcast; `flush_clip` runs only on the Ok branch).
- **Tests:** 5 new unit tests (`precedence_matrix`, `valid_config_parses`, `malformed_maps_to_config_error`, `out_line_tees`, `flush_clip_empty_noop`, `emit_json_no_bom_no_ansi`, `init_output_forces_color_off`) + 3 new integration tests (`missing_is_silent`, `malformed_exit2`, `hash_default_override`). Full suite green across all 25 test binaries; `clippy --all-targets -D warnings` and `fmt --check` both clean.

## Task Commits

Each task was committed atomically (TDD tasks; resolver + tests + supporting infra form one compiling unit per task):

1. **Task 1: toml+dirs deps, BoxError::Config, core::config module** — `24c558f` (feat)
2. **Task 2: output spine primitives (emit_json/out_line/flush_clip)** — `29b6a19` (feat)
3. **Task 3: global --json/--clip flags, main spine ordering, config tests** — `4014f06` (feat)

**Plan metadata:** appended in the final docs commit.

## Files Created/Modified

- `src/core/config.rs` (NEW) — config provider: `Config`, `OnceLock`, `init_config`/`config()`/`load()`/`config_path()`, pure `resolve_algo()`, unit tests.
- `tests/config.rs` (NEW) — black-box integration tests with per-process `APPDATA` isolation.
- `src/core/output.rs` — added the spine primitives + 4 unit tests beside the `COLOR_ON` triad.
- `src/core/errors.rs` — `BoxError::Config{path,message}` exit-2 variant.
- `src/core/mod.rs` — registered `pub mod config;` (alphabetical).
- `src/cli.rs` — global `--json`/`--clip` bools.
- `src/main.rs` — folded `init_config`/`init_color`/`init_output`/dispatch/`flush_clip` into one `result` chain; added `Config` to the exit-2 downcast.
- `src/commands/hash/mod.rs` — `Algo` gains `serde::Deserialize` + `#[serde(rename_all = "lowercase")]` (minimal cross-plan edit).
- `Cargo.toml` / `Cargo.lock` — `toml 1.1.2` + `dirs 6.0.0` (the only manifest change this phase).
- `tests/cmd/help.trycmd` — snapshot updated for the two new Options lines.

## Decisions Made

- **`main.rs` structure (output spec request):** `crate::core::config::init_config().and_then(|()| { init_color(cli.no_color); init_output(cli.json, cli.clip); match cli.command { …dispatch… } }).and_then(|()| flush_clip())`. This routes `init_config`'s `Err` through the SAME `match result { Err(e) => { eprintln!("error: {e:#}"); downcast } }` block as dispatch errors (so a malformed config still prints the `error:` line — D-10 — never a bare `return ExitCode::from(2)`), and `flush_clip` runs only when dispatch returned `Ok` (the `.and_then` short-circuits on dispatch `Err`, so a failed command never clobbers the clipboard — D-08).
- **Env-tier spelling (Claude's Discretion):** `BOX_HASH_DEFAULT_ALGO` (uppercase `BOX_<SECTION>_<KEY>`). The `.or(env)` slot is **deferred to 06-02** — 06-01 wires only the pure resolver + config tier; the live env→`Algo` parse lands with `hash`'s compute-default flip in 06-02, reusing one parse fn for env+config.
- **`Algo: Deserialize` minimal cross-plan edit:** added here (with `rename_all = "lowercase"` so `default_hash_algo = "sha256"` round-trips). The matching `serde::Serialize` derive (for `--json`) lands in 06-02.
- **Clipboard confirmation wording:** `"Copied to clipboard"` — stderr-only, TTY-gated on STDERR (`box uuid --clip 2>log` does not write it into the log).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `config_path()` reads `%APPDATA%` env var first, `dirs::config_dir()` only as fallback**
- **Found during:** Task 3 (config integration tests)
- **Issue:** The planned `dirs::config_dir()`-first form makes config isolation impossible on Windows. `dirs` 6.0 → `dirs-sys` 0.5 resolves `config_dir()` via `SHGetKnownFolderPath` (the Known-Folder API), which **ignores the `APPDATA` environment variable**. The integration tests set `APPDATA` to a `TempDir` to isolate the lookup, but `dirs` resolved to the real `%APPDATA%` regardless — so `malformed_exit2` saw exit 0 (no config found) instead of exit 2. This would also break any user/CI that relocates `APPDATA`.
- **Fix:** Resolve `%APPDATA%` via `std::env::var_os("APPDATA")` first (the standard, per-process-overridable Windows roaming-appdata location), falling back to `dirs::config_dir()` only when `APPDATA` is unset. The `%APPDATA%\box\config.toml` target is identical. The plan explicitly sanctioned this: "`std::env::var_os("APPDATA")` is the documented fallback only — do not use it unless dirs resists." **dirs resists.** `dirs` remains a justified dependency (the non-Windows / `APPDATA`-unset fallback).
- **Files modified:** src/core/config.rs (`config_path()` + doc)
- **Verification:** All 3 config integration tests pass; full suite green.
- **Committed in:** `4014f06` (Task 3 commit)

**2. [Rule 1 - Bug] trycmd `box --help` snapshot drift**
- **Found during:** Task 3 (full-suite run)
- **Issue:** Adding the global `--json`/`--clip` flags changed the `box --help` Options block, breaking the `tests/cmd/help.trycmd` snapshot.
- **Fix:** Updated the snapshot's Options block with the two new flag lines (declaration order: `--no-color`, `--json`, `--clip`, `-h`, `-V`).
- **Files modified:** tests/cmd/help.trycmd
- **Verification:** `cargo test --test cli` passes (trycmd green).
- **Committed in:** `4014f06` (Task 3 commit)

**3. [Rule 1 - Cleanliness] forward-compat `#[allow(dead_code)]` + clippy nit fix**
- **Found during:** Task 3 (clippy `-D warnings` gate)
- **Issue:** The spine primitives consumed only in 06-02 (`is_json_on`, `emit_json`, `out_line`, `config`, `resolve_algo`) tripped dead-code warnings, which the project's `-D warnings` CI gate would reject; a test helper used `.err().expect()` (clippy `err_expect`).
- **Fix:** Added forward-compat `#[allow(dead_code)]` to the five primitives with an allow-then-remove note (the repo's documented convention, mirroring `errors.rs::MissingInput` history); rewrote the test helper to `expect_err(...)`.
- **Files modified:** src/core/output.rs, src/core/config.rs
- **Verification:** `clippy --all-targets -D warnings` exits 0; `fmt --check` clean.
- **Committed in:** `4014f06` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (3 Rule 1 — one a genuine design bug in the config-path resolver, two CI-gate cleanliness). 
**Impact on plan:** Deviation #1 is the only substantive one — it corrects an untestable/unworkable config-path resolution on Windows and is explicitly sanctioned by the plan's fallback clause. No scope creep; the spine shape is exactly as specified.

## Known Stubs

None. All primitives are fully implemented. Five spine functions (`is_json_on`, `emit_json`, `out_line`, `config`, `resolve_algo`) carry forward-compat `#[allow(dead_code)]` because their first consumers (`uuid`/`hash`) land in 06-02 — this is the planned cross-plan state ("NO command consuming it yet"), not a stub. `init_config`, `init_output`, and `flush_clip` are already live in `main.rs`.

## Threat Flags

None. No new security surface beyond the plan's `<threat_model>`. The config-path change (deviation #1) strengthens T-06-01 (config read from user-scoped `%APPDATA%` only — `var_os("APPDATA")` is the canonical roaming-appdata location, still never CWD or an arbitrary path).

## Issues Encountered

- The `dirs`-vs-`APPDATA` isolation bug (deviation #1) — diagnosed via `cargo tree -p dirs` (→ `dirs-sys` 0.5 → `SHGetKnownFolderPath`) and resolved within the plan's documented fallback clause.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **06-02 (wave 2) is ready.** The frozen spine is in place and the hand-offs are explicit:
  - `Algo: Serialize` derive (for `--json`) — to be added in 06-02 (Deserialize already here).
  - The env-tier `.or(env)` slot using `BOX_HASH_DEFAULT_ALGO` — wired with `hash`'s compute-default flip in 06-02 (reuse one env+config parse fn).
  - SPINE-01 / SPINE-03 are "established/contract-tested on the pilots" → marked complete once `uuid`/`hash` adopt `is_json_on()`/`emit_json`/`out_line` in 06-02.
- **No blockers.** Full test suite green (25 binaries), clippy `-D warnings` clean, fmt clean.

## Self-Check: PASSED

- FOUND: src/core/config.rs
- FOUND: tests/config.rs
- FOUND: commit 24c558f (Task 1)
- FOUND: commit 29b6a19 (Task 2)
- FOUND: commit 4014f06 (Task 3)

---
*Phase: 06-scriptable-core-foundation*
*Completed: 2026-06-25*
