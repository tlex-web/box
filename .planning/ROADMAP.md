# Roadmap: box — Rust CLI Toolbox

**Project:** box

## Milestones

- ✅ **v1.0 Full Toolbox** — Phases 1–5, all 23 commands (shipped 2026-06-24) → see [`milestones/v1.0-ROADMAP.md`](milestones/v1.0-ROADMAP.md)
- ✅ **v2.0 Toolbox → Toolkit** — Phases 6–11, scriptable `--json`/`--clip`/config spine + per-command depth + meta-commands (shipped 2026-07-14) → see [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md)
- 📋 **Next milestone** — not yet defined (`/gsd:new-milestone`)

## Overview

`box` is a single Rust binary bundling 23 command-line tools (+ 2 meta-commands) as subcommands for a developer on Windows PowerShell 7. **v1.0** shipped all 23 commands, installable globally via `install.ps1`. **v2.0** deepened them into a scriptable toolkit — a cross-cutting `--json`/`--clip`/config spine across every applicable command, comprehensive per-command depth flags (including three safety-gated destructive filesystem operations), a BLAKE3-default `hash` with a config escape hatch, PS7 completions, and the `config`/`completions` meta-commands — a deepening, not a rewrite, of the settled v1 architecture. Both milestones are shipped and archived; full phase detail lives in the per-milestone archives under `milestones/`.

## Phases

**Phase Numbering:** Integer phases (1, 2, 3) are planned milestone work; decimal phases (2.1) are urgent insertions (none used to date). v2.0 continued v1's numbering — v1.0 ended at Phase 5, so v2.0 ran Phases 6–11.

<details>
<summary>✅ v1.0 Full Toolbox (Phases 1–5) — SHIPPED 2026-06-24</summary>

- [x] Phase 1: Foundation + Flatten (4/4 plans) — completed 2026-06-22 — binary scaffold, install.ps1, core infra, `flatten`
- [x] Phase 2: Pure Transform Utilities (5/5 plans) — completed 2026-06-22 — uuid, base64, epoch, color, passgen, cowsay, fortune, 8ball, roast
- [x] Phase 3: Filesystem Power Tools (5/5 plans) — completed 2026-06-23 — hash, tree, du, dupes, bulk-rename
- [x] Phase 4: Terminal Visuals (4/4 plans) — completed 2026-06-24 — json, lolcat, ascii, matrix
- [x] Phase 5: Windows Platform Integration (4/4 plans) — completed 2026-06-24 — qr, clip, pomodoro, weather

Full phase details, decisions, and outcomes archived in [`milestones/v1.0-ROADMAP.md`](milestones/v1.0-ROADMAP.md).

</details>

<details>
<summary>✅ v2.0 Toolbox → Toolkit (Phases 6–11) — SHIPPED 2026-07-14</summary>

- [x] Phase 6: Scriptable-Core Foundation (2/2 plans) — completed 2026-06-25 — the whole shared `--json`/`--clip`/config spine, proven on `uuid`+`hash`; BLAKE3-default `hash` flip with config escape hatch (SPINE-01/03/05, HASH-V2-01)
- [x] Phase 7: Spine Rollout (3/3 plans) — completed 2026-06-25 — `--json` on all 16 applicable commands + `--clip` on the 8 single-textual-result commands (SPINE-02/04)
- [x] Phase 8: Filesystem Depth (6/6 plans) — completed 2026-06-28 — 10 filesystem depth flags across hash/flatten/dupes/bulk-rename/tree/du, incl. the three destructive flags under adversarial review (HASH-V2-02, FLAT/DUPE/RENM-V2-01/02, TREE-V2-01, DU-V2-01/02)
- [x] Phase 9: Dev-Transform & Visual Depth (3/3 plans) — completed 2026-07-14 — uuid v7/format, epoch tz, color CSS+HSL, json sort-keys, passgen entropy; matrix/qr/ascii depth; PS7-verified animated lolcat (9 reqs)
- [x] Phase 10: Fun & System Depth (6/6 plans) — completed 2026-07-14 — cowsay/fortune/8ball/roast depth; pomodoro counter/cycle/sound; weather forecast/cache/stored-location (7 reqs)
- [x] Phase 11: Meta-Commands (3/3 plans) — completed 2026-07-14 — `box config` show/get/set/path + `box completions <shell>` generated from the live final `Cli` (CFG-01, CMP-01; incl. 11-03 WR-01/WR-02 gap closure)

Full phase details, plans, decisions, and outcomes archived in [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md).

</details>

### 📋 Next milestone (not yet defined)

Start with `/gsd:new-milestone` (questioning → research → requirements → roadmap). Deferred candidates recorded in `milestones/v2.0-REQUIREMENTS.md` (Future Requirements) and STATE.md Deferred Items.

## Progress

**Execution Order:** Phases executed in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11.

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation + Flatten | v1.0 | 4/4 | Complete | 2026-06-22 |
| 2. Pure Transform Utilities | v1.0 | 5/5 | Complete | 2026-06-22 |
| 3. Filesystem Power Tools | v1.0 | 5/5 | Complete | 2026-06-23 |
| 4. Terminal Visuals | v1.0 | 4/4 | Complete | 2026-06-24 |
| 5. Windows Platform Integration | v1.0 | 4/4 | Complete | 2026-06-24 |
| 6. Scriptable-Core Foundation | v2.0 | 2/2 | Complete | 2026-06-25 |
| 7. Spine Rollout | v2.0 | 3/3 | Complete | 2026-06-25 |
| 8. Filesystem Depth | v2.0 | 6/6 | Complete | 2026-06-28 |
| 9. Dev-Transform & Visual Depth | v2.0 | 3/3 | Complete | 2026-07-14 |
| 10. Fun & System Depth | v2.0 | 6/6 | Complete | 2026-07-14 |
| 11. Meta-Commands | v2.0 | 3/3 | Complete | 2026-07-14 |

---
*Roadmap created: 2026-06-22*
*v1.0 milestone closed 2026-06-24 — Phases 1–5 collapsed; full detail in `milestones/v1.0-ROADMAP.md`.*
*v2.0 milestone closed 2026-07-14 — Phases 6–11 collapsed; full detail in `milestones/v2.0-ROADMAP.md`. All 34 v2.0 requirements shipped (coverage 34/34). Next milestone via `/gsd:new-milestone`.*
