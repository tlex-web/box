---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_plan: 3
status: executing
stopped_at: Completed 01-02-PLAN.md
last_updated: "2026-06-22T13:43:27.465Z"
progress:
  total_phases: 5
  completed_phases: 0
  total_plans: 4
  completed_plans: 2
  percent: 50
---

# Project State: box — Rust CLI Toolbox

**Last updated:** 2026-06-22
**Updated by:** roadmapper

---

## Project Reference

**Core Value:** The toolbox must be globally available and instantly usable from PowerShell 7 — type `box <command>` from anywhere and the tool just works.

**Current Focus:** Phase 01 — foundation-flatten

**Milestone:** v1 (all 23 commands)

---

## Current Position

**Phase:** 1 — Foundation + Flatten
**Current Plan:** 3
**Total Plans in Phase:** 4
**Status:** Ready to execute

**Progress:**

```
[█████░░░░░] 50%
Phase 1 [█████░░░░░] 2 / 4 plans
Phase 2 [          ] Not started
Phase 3 [          ] Not started
Phase 4 [          ] Not started
Phase 5 [          ] Not started

Overall: 0 / 5 phases complete
```

---

## Phase Map

| Phase | Name | Requirements | Status |
|-------|------|-------------|--------|
| 1 | Foundation + Flatten | FOUND-01..08, FLAT-01..04 (12 reqs) | In progress (2/4 plans) |
| 2 | Pure Transform Utilities | UUID-01, B64-01, EPOC-01, COLR-01, PASS-01, COW-01, FORT-01, 8BAL-01, ROST-01 (9 reqs) | Not started |
| 3 | Filesystem Power Tools | HASH-01, TREE-01, DU-01, DUPE-01, RENM-01 (5 reqs) | Not started |
| 4 | Terminal Visuals | LOL-01, MTRX-01, ASCI-01, JSON-01 (4 reqs) | Not started |
| 5 | Windows Platform Integration | QR-01, CLIP-01, POMO-01, WTHR-01 (4 reqs) | Not started |

---

## Performance Metrics

**Plans executed:** 2
**Plans succeeded:** 2
**Plans failed:** 0
**Phases completed:** 0 / 5

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 01 | P01 | 5min | 3 | 10 |
| 01 | P02 | 6min | 2 | 7 |

---

## Accumulated Context

### Key Decisions

| Decision | Rationale |
|----------|-----------|
| Phase 1 bundles Foundation + Flatten | Foundation is not deliverable without a real command; flatten is the anchor command per PROJECT.md; shipping both together means Phase 1 ends with an installable, usable tool |
| Pure transforms in Phase 2 before filesystem tools | These 9 commands have zero external integration risk and prove the RunCommand pattern; finding architecture problems on `uuid` is cheaper than finding them on `flatten` |
| Filesystem tools in Phase 3 (after Phase 1 anchor) | All 5 share walkdir infrastructure already established by flatten; collision-rename and dry-run patterns are proven before being reused |
| Terminal visuals in Phase 4 (parallel-eligible with Phase 3) | crossterm dependency group is independent of walkdir group; lolcat teaches frame-buffered output before matrix |
| Platform commands in Phase 5 (last) | arboard, winrt-notification, and Open-Meteo carry the highest Windows API / external service risk; building them last means 21 other commands are working before the riskiest integrations are attempted |
| BLAKE3 as default hash in `hash` command | Faster than SHA-256 on modern CPUs; SHA-256 available via --algo flag for interoperability |
| `x86_64-pc-windows-msvc` target with crt-static | MinGW demoted to Tier 2 in Rust 1.88; MSVC required for arboard and winrt-notification; static CRT makes exe portable |
| winrt-notification needs Phase 1 compile spike | Maintenance status uncertain; validate it compiles before Phase 5 planning to avoid late-phase blocker |
| [01-01] Bare `box` prints help to stderr and exits 2 | Resolves OQ-1 toward strict "messages -> stderr" while satisfying D-08 (clap's `arg_required_else_help` default is exit 0) |
| [01-01] Stubs are real clap-derive enum variants dispatched to a NotImplemented handler | D-05 — keeps all 23 commands visible in `box --help` while only `flatten` will become functional |
| [01-01] main() owns the strict 0/1/2 exit-code policy via `Cli::try_parse` | `parse()` auto-exits 0 on bare box; `try_parse` lets main() preserve clap's exit 2 for parse errors (D-07) and force exit 2 for bare box (D-08) |
| [01-01] Committed `Cargo.lock` with the manifest | Binary crate — the lockfile is part of the reproducible-build contract |
| [01-02] Gate row coloring on our own `COLOR_ON` flag, not `owo_colors::set_override` | The plain `.green()` trait method is unconditional; `set_override` only affects the `if_supports_color` API. Consulting our own AtomicBool is what makes piped output byte-identical minus ANSI (D-10) |
| [01-02] Enabled the `owo-colors` `supports-colors` feature | Required for `set_override`/`with_override` to compile; the locked default feature set excluded it |
| [01-02] `safe_copy` preserves atime best-effort, mtime always | Some filesystems don't report `accessed()`; only mtime is mandated by FLAT-04, so a missing atime must not fail the copy (Assumption A3) |

### Critical Pitfalls to Remember

- Use `dunce::canonicalize` everywhere — never `std::fs::canonicalize` (produces UNC paths)
- Call `enable_ansi_support::enable_ansi_support()` as first line of `main()` before any output
- `install.ps1` must refresh `$env:Path` in the current session (merge user + machine PATH from registry)
- `flatten` must canonicalize both src and dest before walker starts; abort if dest is inside src
- `matrix` must buffer full frame and flush once per frame (not per character — causes ~5 FPS)
- `arboard` clipboard must run on main thread only
- Windows reserved filenames (`CON.txt`, `NUL.txt`) must be sanitized in `flatten` output
- `8ball` maps to Rust module `eight_ball` (identifiers cannot start with a digit)
- Build target: `x86_64-pc-windows-msvc` with `RUSTFLAGS="-C target-feature=+crt-static"`

### Architecture Established

- Single Rust crate (not workspace); `src/commands/<cmd>/mod.rs` per command
- `RunCommand` trait: `fn run(self) -> anyhow::Result<()>` implemented by each Args struct
- `src/core/`: `errors.rs` (BoxError + thiserror), `output.rs` (color init + print helpers), `fs.rs` (walkdir wrapper, safe_copy, collision rename)
- `src/main.rs`: ~40 lines, parse + dispatch + exit code only, no business logic
- Integration tests via `assert_cmd` in `tests/<cmd>.rs`; snapshot tests via `insta`/`trycmd`

### Todos

- [ ] Spike `winrt-notification 0.5` compilation against project MSRV during Phase 1
- [ ] Decide `pomodoro` blocking vs non-blocking timer before Phase 5 planning
- [ ] Decide `weather` default unit system (metric / imperial / locale-detect) before Phase 5 planning
- [ ] Add `strip-ansi-escapes` crate to Cargo.toml for `lolcat` during Phase 4 planning

### Blockers

None.

---

## Session Continuity

**To resume:** Read `.planning/ROADMAP.md` for phase goals, then read `.planning/STATE.md` (this file) for current position and context.

**Last session:** 2026-06-22 — completed 01-02-PLAN.md (core infra: `output.rs` color gating + reusable glyph/summary helpers; `fs.rs` UNC-safe `normalize_path`, `is_hidden`, `safe_copy`; `piped_help_has_no_ansi` test green)
**Stopped At:** Completed 01-02-PLAN.md
**Resume File:** None

**Next action:** Execute 01-03-PLAN.md (flatten: consume `core::output` rows/summaries + `core::fs` normalize/is_hidden/safe_copy; remove the two `#[allow(dead_code)]`).

---
*State initialized: 2026-06-22 by roadmapper*
*Updated: 2026-06-22 by executor — 01-02 complete*
