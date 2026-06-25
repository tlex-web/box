---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Toolbox to Toolkit
status: executing
stopped_at: Phase 6 COMPLETE (both waves) — uuid+hash spine pilots + BLAKE3-default flip
last_updated: "2026-06-25T01:05:00.000Z"
last_activity: 2026-06-25 -- 06-02 executed (uuid+hash --json/--clip pilots, BLAKE3 default, D-05 probe)
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 2
  completed_plans: 2
  percent: 11
---

# Project State: box — Rust CLI Toolbox

**Last updated:** 2026-06-25
**Updated by:** execute-plan (06-02 complete — uuid+hash adopt the spine, BLAKE3 compute-default flipped with config/env escape hatch + D-05 verify probe; Phase 6 DONE, next: Phase 7 spine rollout)

---

## Project Reference

**Core Value:** The toolbox must be globally available and instantly usable from PowerShell 7 — type `box <command>` from anywhere and the tool just works.

**Current Focus:** Phase 07 — spine-rollout (Phase 06 complete)

**Milestone:** v2.0 Toolbox → Toolkit — EXECUTING. Phase 6 (scriptable-core foundation) complete. v1.0 Full Toolbox shipped & archived 2026-06-24 (all 23 commands; see `.planning/MILESTONES.md`).

See: .planning/PROJECT.md · .planning/ROADMAP.md · .planning/REQUIREMENTS.md (all current as of 2026-06-25)

---

## Current Position

Phase: 06 (scriptable-core-foundation) — COMPLETE; Phase 07 (spine-rollout) — NOT STARTED
Plan: 2 of 2 complete (06-01 + 06-02 done)
Status: Phase 06 fully complete (both waves) — spine built (06-01) and proven on uuid+hash pilots (06-02)
Last activity: 2026-06-25 -- 06-02 executed (uuid+hash --json/--clip pilots, BLAKE3-default flip, D-05 probe; full suite green)

Progress: [█░░░░░░░░░] 11% (2/18 v2.0 plans)

## Phase Map

v1.0 (Phases 1–5) complete & archived — see `.planning/milestones/v1.0-ROADMAP.md`. v2.0 phases:

| Phase | Name | Requirements | Status |
|-------|------|-------------|--------|
| 6 | Scriptable-Core Foundation | SPINE-01, SPINE-03, SPINE-05, HASH-V2-01 (4) | Complete (2/2 plans — all 4 reqs done) |
| 7 | Spine Rollout | SPINE-02, SPINE-04 (2) | Not started |
| 8 | Filesystem Depth | HASH-V2-02, FLAT-V2-01/02, DUPE-V2-01/02, RENM-V2-01/02, TREE-V2-01, DU-V2-01/02 (10) | Not started |
| 9 | Dev-Transform & Visual Depth | UUID-V2-01, EPOC-V2-01, COLR-V2-01, JSON-V2-01, PASS-V2-01, LOL-V2-01, MTRX-V2-01, QR-V2-01, ASCI-V2-01 (9) | Not started |
| 10 | Fun & System Depth | COW-V2-01, FORT-V2-01, 8BAL-V2-01, ROST-V2-01, POMO-V2-01/02, WTHR-V2-01 (7) | Not started |
| 11 | Meta-Commands | CFG-01, CMP-01 (2) | Not started |

---

## Performance Metrics

**Plans executed (v2.0):** 2 / 18 planned
**v1.0 (archived):** 22 plans, 22 succeeded, 0 failed, 5/5 phases — see `.planning/MILESTONES.md`.

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 6 | 06-01 | ~10 min | 3 | 11 |
| 6 | 06-02 | ~35 min | 2 | 7 |

---

## Accumulated Context

### Key Decisions (v2.0 — locked at requirements/roadmap time)

| Decision | Rationale |
|----------|-----------|
| Order phases by integration risk: spine first, then rollout, then depth, then meta | v1 retrospective #1 lesson — the only architecture risk lives in the shared `--json`/`--clip`/config spine, so build it ONCE on the 2 cheapest commands (uuid+hash) before 21 adopt it (a flaw costs 2 commands of rework, not 23) |
| BLAKE3-default (HASH-V2-01) + config resolver (SPINE-05) co-located in Phase 6 (D-6) | So the `hash.default_algo = "sha256"` escape hatch exists the moment the breaking default flips |
| 27 depth reqs split into 3 area-grouped phases (8 fs / 9 dev+visual / 10 fun+system) | One 27-req phase is unworkable for plan/wave sizing; area grouping keeps each phase a sane unit. Meta-command phase renumbered to 11, stays strictly last (D-7) |
| 3 destructive depth flags each get own plan + adversarial review (08-04/05/06) | FLAT-V2-02 `--move`, DUPE-V2-02 `--delete`, RENM-V2-02 `--backup` — the v1 Phase-3 bulk-rename gate: dry-run default, `--force`, abort-all-before-any pre-flight, snapshot-tree-unchanged test per abort path |
| `completions` (CMP-01) in Phase 11, after all depth (D-7) | Generated from the live final `Cli` — must include every Phase-8/9/10 flag |
| Config: hand-roll `toml` 1.1.2 + `dirs` 6.0.0, `Option<T>` + `.or().or().unwrap_or()` (D-1) | Matches v1 hand-roll ethos; precedence CLI > env > config > builtin; missing/malformed file → defaults, never errors a normal command |
| `--json` house style: ONE document, no NDJSON, no BOM, no ANSI/progress on stdout (SPINE-01) | PS7 `ConvertFrom-Json` needs one buffered doc; `init_output` forces `COLOR_ON=false` under json/clip; per-command JSON-purity test is the regression backstop |
| `windows 0.61` GO both features (D-2): `du --on-disk` + `pomodoro --sound` | Pin 0.61 to unify with transitive `windows ^0.61` from tauri-winrt-notification 0.7.2 |
| Bare `u64` for large JSON numbers (D-3); `to_string_lossy()` for non-UTF-8 paths (D-4) | One rule for the whole spine; never `to_str().unwrap()` (panics on non-UTF-8 NTFS names) |
| `config_path()` reads `%APPDATA%` env var FIRST, `dirs::config_dir()` only as fallback (06-01 Rule 1 deviation from the planned dirs-first form) | `dirs` 6.0 → `dirs-sys` 0.5 resolves `config_dir()` via `SHGetKnownFolderPath`, which IGNORES the `APPDATA` env var → per-process config isolation impossible (integration tests + any APPDATA-relocating CI couldn't point the lookup at a temp dir). The plan's "var_os(APPDATA) is the fallback only unless dirs resists" clause applies; dirs resists. Identical `%APPDATA%\box\config.toml` target. |
| Env-tier spelling locked to `BOX_HASH_DEFAULT_ALGO`; `.or(env)` slot DEFERRED to 06-02 (06-01 Claude's Discretion) | 06-01 wires only the pure resolver + config tier; the live env→Algo parse lands with `hash`'s compute-default flip in 06-02 (reuses one FromStr/ValueEnum parse for env+config). Clipboard confirmation wording locked to `"Copied to clipboard"`. |
| `BOX_HASH_DEFAULT_ALGO` env tier wired live in 06-02 via `parse_algo(s) = Algo::from_str(s,true).ok()` (clap ValueEnum, case-insensitive), reused for both env + config — single-sourced spelling table | The hash compute default now resolves `self.algo.or_else(env).or(config().default_hash_algo).unwrap_or(Blake3)` (CLI>env>config>builtin). An unrecognized env value returns None and falls through (never errors a normal `box hash`). `hash` uses this inline `.or()` chain, NOT `resolve_algo` (per the plan's line-162 spec); `resolve_algo` keeps its forward-compat allow. |
| D-05 verify probe: capture the path string into `path_for_probe` BEFORE `read_file_or_stdin(self.path)` consumes it; re-open + blake3 on a 64-hex no-`--algo` mismatch (06-02) | `ResolvedInput.reader` is single-pass `Box<dyn Read>`. `path_for_probe` is `Some` only for a real path (`p != "-"`), `None` for stdin (no second read → static hint). Decisive hint when blake3 matches the file, static otherwise; stderr-only, suppressed under `--json`, exit STAYS 1. Hint token `--algo blake3` styled `.yellow()` when `is_color_on()`. |
| `{Row}/{Output}` serde struct feeds BOTH human + JSON paths; always-wrapped `{results,count}` even for N=1 (06-02 pilot literals, frozen for Phase 7) | `uuid` → `{results:[{uuid,version:"v4"}],count}`; `hash` → `{results:[{path,algo,digest}],count}`. The pure renderer fills the struct, `is_json_on()` forks (emit_json | out_line) — no drift. Object never a bare array (Phase-8 multi-item compatible). `tests/uuid.rs::json_purity` is the copy-me JSON-purity test for all 23 commands. |

Full v1.0 decision log preserved in PROJECT.md Key Decisions + `.planning/milestones/v1.0-ROADMAP.md`.

### Critical Pitfalls to Remember (carried from v1 + new for v2)

- **v2 #1 failure mode:** `--json` stdout contamination — stray progress/ANSI/BOM bytes break `ConvertFrom-Json`. Progress → stderr via `ProgressDrawTarget::stderr()`, suppressed entirely under `is_json_on()`.
- **Config precedence:** every config-overridable flag is `Option<T>` with NO `default_value`; resolve `cli.or(env).or(config).unwrap_or(builtin)`. A missing/malformed config must never error a normal `box uuid`.
- **`dupes --delete` hardlink false-positive:** detect shared identity via `(volume_serial, file_index)` from `fs::metadata(path)` — NOT `DirEntry::metadata()` (returns `None` for those fields). Collapse before computing wasted space.
- **`flatten --move`:** copy → verify (dest exists + size matches) → delete source. Never delete before confirming the copy.
- **`std::fs::rename` SILENTLY OVERWRITES on Windows** (no `create_new` for moves) — the abort-all-before-any pre-flight is the only backstop (v1 bulk-rename pattern; reuse for `--backup`).
- **`arboard` clipboard must run on the main thread only** — `flush_clip()` runs once in `main()` after successful dispatch.
- **`--clip`/`--json` force `COLOR_ON=false`** so the clipboard/JSON never gets ANSI escapes.
- **Terminal loops (`lolcat --animate`, `matrix` extensions):** arm RAII `RawGuard` immediately after `enable_raw_mode()?`; detect TTY first, degrade to static when piped/`--json`; single-flush-per-frame; `KeyEventKind::Press`-only quit filter.
- `box` is binary-only — unit tests run via `cargo test --bin box`, NOT `--lib`.
- Build target: `x86_64-pc-windows-msvc` with `RUSTFLAGS="-C target-feature=+crt-static"`.
- BLAKE3-default breaking change: loud `--help`/PROJECT note; `--algo sha256` + `hash.default_algo` config restore old behavior; `--verify` 64-hex still maps to sha256 (transitional mismatch hint).

### Architecture Established (v1 base — v2 grafts onto, does NOT rewrite)

- Single Rust crate; `src/commands/<cmd>/mod.rs` per command; `RunCommand` trait `fn run(self) -> anyhow::Result<()>` (signature UNCHANGED in v2).
- `src/core/`: `errors.rs`, `output.rs` (`COLOR_ON` + `is_color_on()` + `human_size` + `terminal_width`), `fs.rs`, `input.rs`. `main.rs` ~123 lines: parse + dispatch + 0/1/2 exit only.
- **v2 NEW/MODIFIED:** `core/output.rs` (+`JSON_ON`/`CLIP_ON` atomics, `init_output`, `is_json_on`, `emit_json`, `out_line`, `CLIP_BUF`, `flush_clip`); `core/config.rs` (NEW); `core/errors.rs` (+`BoxError::Config`); `cli.rs` (+global `--json`/`--clip`, +`Completions`/`Config` variants); `main.rs` (+`init_config`/`init_output`/`flush_clip`); per command: one `#[derive(Serialize)]` output struct + `is_json_on()` fork + `out_line` primary output. New crates: `clap_complete`, `toml`, `dirs`, `indicatif`, `chrono-tz` (epoch tz), `windows 0.61`; `uuid "v7"` feature.

### Todos

- [ ] Code-review advisory follow-ups (01-REVIEW.md, non-blocking, carried from v1): WR-03/WR-04 install.ps1 PATH empty-segment + smoke-test-by-abspath; IN-02/IN-03 share one flatten render path between dry-run and real run.

### Blockers

None.

---

## Session Continuity

**To resume:** Read `.planning/ROADMAP.md` for phase goals, then this file for position/context.

**Last session:** 2026-06-25 — execute-plan (06-02: uuid+hash spine pilots, BLAKE3-default flip, D-05 probe)
**Stopped at:** Phase 6 COMPLETE (both waves) — spine built (06-01) and proven on uuid+hash (06-02); SPINE-01/SPINE-03/HASH-V2-01 done; full test suite green (25 binaries), clippy `-D warnings` + fmt clean
**Resume file:** .planning/phases/06-scriptable-core-foundation/06-02-SUMMARY.md

**Next action:** `/gsd:execute-phase 7` — Spine Rollout (SPINE-02/SPINE-04). Copy the frozen Phase-6 template across the remaining applicable commands: the `{Row}/{Output}` serde struct + `is_json_on()` fork + `out_line` routing (uuid is the cleanest reference, hash adds the `path`-bearing variant), the `tests/uuid.rs::json_purity` JSON-purity test, and the `#[ignore]`d clip round-trip per command. `emit_json`/`out_line`/`is_json_on`/`config()` are all live (forward-compat allows removed). v1.0 context archived in `.planning/MILESTONES.md`.

---
*State reset to v2.0 phase map: 2026-06-25 by roadmapper (v1.0 plan-by-plan execution log archived with the milestone; v2.0 accumulated context — locked decisions D-1..D-7, v2 pitfalls, the v1→v2 architecture graft — preserved above).*
