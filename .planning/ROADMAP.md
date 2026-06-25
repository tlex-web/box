# Roadmap: box — Rust CLI Toolbox

**Project:** box

## Milestones

- ✅ **v1.0 Full Toolbox** — Phases 1–5, all 23 commands (shipped 2026-06-24) → see [`milestones/v1.0-ROADMAP.md`](milestones/v1.0-ROADMAP.md)
- 🚧 **v2.0 Toolbox → Toolkit** — Phases 6–11, scriptable spine + per-command depth + meta-commands (in progress)

## Overview

v2.0 deepens the shipped 23-command `box` binary into a scriptable PowerShell-7 toolkit. It is a *deepening, not a rewrite*: the v1 architecture (single crate, `RunCommand` trait, `is_color_on()`-gated styling, 0/1/2 exit contract) is settled. The journey grafts a cross-cutting `--json`/`--clip`/config spine onto that base, proves it on the two cheapest commands, rolls it mechanically across every applicable command, then layers per-command depth flags, and finishes with the two meta-commands. Phases are ordered by integration risk per the v1 retrospective's #1 lesson — the only real architecture risk lives in the shared spine, so it is built once on `uuid`+`hash` before 21 commands adopt it. The breaking change (BLAKE3-default `hash`) co-ships with config so the `hash.default_algo = "sha256"` escape hatch exists the moment the default flips. Completions land dead last so the generated PS7 script reflects every flag added along the way.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.
v2.0 continues v1's numbering — v1.0 ended at Phase 5, so v2.0 starts at Phase 6.

<details>
<summary>✅ v1.0 Full Toolbox (Phases 1–5) — SHIPPED 2026-06-24</summary>

- [x] Phase 1: Foundation + Flatten (4/4 plans) — completed 2026-06-22 — binary scaffold, install.ps1, core infra, `flatten`
- [x] Phase 2: Pure Transform Utilities (5/5 plans) — completed 2026-06-22 — uuid, base64, epoch, color, passgen, cowsay, fortune, 8ball, roast
- [x] Phase 3: Filesystem Power Tools (5/5 plans) — completed 2026-06-23 — hash, tree, du, dupes, bulk-rename
- [x] Phase 4: Terminal Visuals (4/4 plans) — completed 2026-06-24 — json, lolcat, ascii, matrix
- [x] Phase 5: Windows Platform Integration (4/4 plans) — completed 2026-06-24 — qr, clip, pomodoro, weather

Full phase details, decisions, and outcomes archived in [`milestones/v1.0-ROADMAP.md`](milestones/v1.0-ROADMAP.md).

</details>

### 🚧 v2.0 Toolbox → Toolkit (In Progress)

**Milestone Goal:** Make the existing 23 commands deeper, scriptable, and frictionless in PowerShell 7 — `--json`/`--clip` everywhere applicable, comprehensive per-command depth, BLAKE3-default `hash`, config-file defaults, and PS7 completions. No new *tool* commands; only `config`/`completions` meta-commands.

- [x] **Phase 6: Scriptable-Core Foundation** - Build the entire shared `--json`/`--clip`/config spine once, proven on the two simplest pilot commands (`uuid` + `hash`), and flip `hash` to BLAKE3-default with a config escape hatch. ✅ 2026-06-25
- [ ] **Phase 7: Spine Rollout** - Apply the frozen Phase-6 template to roll `--json` and `--clip` across every remaining applicable value-producing command.
- [ ] **Phase 8: Filesystem Depth** - Add the deferred depth flags to the six filesystem commands (`hash`, `flatten`, `dupes`, `bulk-rename`, `tree`, `du`), including the three destructive flags under adversarial review.
- [ ] **Phase 9: Dev-Transform & Visual Depth** - Add depth to the dev-transform (`uuid`, `epoch`, `color`, `json`, `passgen`) and visual (`lolcat`, `matrix`, `qr`, `ascii`) commands.
- [ ] **Phase 10: Fun & System Depth** - Add depth to the fun (`cowsay`, `fortune`, `8ball`, `roast`) and system (`pomodoro`, `weather`) commands.
- [ ] **Phase 11: Meta-Commands** - Ship `box config` and `box completions powershell`, generated against the final, complete arg surface.

## Phase Details

### Phase 6: Scriptable-Core Foundation

**Goal**: Build the whole shared scriptable spine (`--json`, `--clip`, config-file defaults) and prove it end-to-end on `uuid` and `hash`, co-shipping the BLAKE3-default breaking change with its config escape hatch — so an architecture flaw costs 2 commands of rework, not 23.
**Depends on**: Phase 5 (v1.0 complete)
**Requirements**: SPINE-01, SPINE-03, SPINE-05, HASH-V2-01
**Success Criteria** (what must be TRUE):

  1. `box uuid --json | ConvertFrom-Json` yields a single well-formed JSON document on stdout with `snake_case` fields, no BOM, no ANSI, and no progress chrome (the JSON-purity contract that becomes the template for every later command).
  2. `box uuid --clip` copies the generated UUID to the Windows clipboard *and* prints it, emitting a "Copied to clipboard" confirmation to stderr that is suppressed when stdout is not a TTY; `box uuid --json --clip` puts the JSON document on the clipboard.
  3. `box hash file.bin` (no `--algo`) now emits a 64-hex BLAKE3 digest where v1 emitted SHA-256, while `box hash --algo sha256 file.bin` still emits SHA-256.
  4. With `%APPDATA%\box\config.toml` containing `hash.default_algo = "sha256"`, `box hash file.bin` emits SHA-256 again — and a CLI `--algo blake3` still wins over that config (CLI flag > env > config > built-in BLAKE3); a missing or malformed config file never errors a normal `box uuid` invocation.

**Plans**: 2 plans

Plans:
**Wave 1**

- [x] 06-01-PLAN.md — Shared spine: `core::output` primitives (`JSON_ON`/`CLIP_ON`/`CLIP_BUF`, `init_output`, `is_json_on`, `emit_json`, `out_line`, `flush_clip`) + NEW `core::config` (`Config`, `load`, `OnceLock`, `config()`, `init_config`) + `BoxError::Config` (exit-2) + global `--json`/`--clip` on `Cli` + `main.rs` ordering/flush wiring + precedence/missing/malformed tests. [wave 1] ✅ 2026-06-25

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 06-02-PLAN.md — First consumers: `uuid` (`UuidOutput` + `is_json_on()` fork + `out_line`) and `hash` (`HashOutput`, BLAKE3 compute-default flip with config+env tier, `Algo` serde, D-05 BLAKE3 verify probe; verify table UNCHANGED) + the reusable JSON-purity/`--clip`-capture/BLAKE3-default/config-precedence test template. [wave 2, depends_on 06-01] ✅ 2026-06-25

### Phase 7: Spine Rollout

**Goal**: Apply the now-frozen Phase-6 template to every remaining applicable command so `--json` and `--clip` are consistent and house-style across the toolkit; per-command cost is mechanical, ordered simplest→nested so any surprise surfaces on `base64`, not `flatten`.
**Depends on**: Phase 6
**Requirements**: SPINE-02, SPINE-04
**Success Criteria** (what must be TRUE):

  1. `box base64`, `epoch`, `color`, `passgen`, `8ball`, `fortune`, `roast`, `cowsay`, `du`, `tree`, `dupes`, `flatten`, `bulk-rename`, `json`, `qr`, and `weather` each accept `--json` and emit exactly one parseable JSON document (`box du C:\proj --json | ConvertFrom-Json` yields one object whose `results` array of PSCustomObjects carries size fields, with a sibling `count`; `box tree --json` yields a recursive object).
  2. `box passgen`, `uuid`, `color`, `hash`, `base64`, `epoch`, `json`, and `qr` each accept `--clip`, copying the primary textual result to the Windows clipboard while still printing it.
  3. Every newly `--json`-enabled command keeps the v1 "byte-identical minus ANSI when piped" contract — stdout under `--json` contains no `0x1B` byte and parses as a single value (the per-command JSON-purity test passes for all of them).
  4. Display-only commands (`matrix`, `pomodoro`, `lolcat`, `ascii`, `clip`) correctly omit `--json`/`--clip` rather than emitting a malformed or meaningless document.

**Plans**: 3 plans

Plans:

**Wave 1**

- [ ] 07-01-PLAN.md — Wave 7a pure transforms: `--json` on `base64`, `epoch`, `color`, `passgen`, `8ball`, `fortune`, `roast`, `cowsay` + `--clip` on base64/color/epoch/passgen + A1 base64 decode policy + SC4 display-only omission + new `tests/cowsay.rs`. [wave 1]

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 07-02-PLAN.md — Wave 7b filesystem: `--json` on `du`, `dupes`, `flatten`, `bulk-rename` (`{results,count,…}`) + `tree` recursive `{name,type,size?,children}` (A4) + bulk-rename --force-emits-rows / abort-empty-stdout forks (A3). [wave 2, depends_on 07-01]

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 07-03-PLAN.md — Wave 7c odd-fits: `--json` on `json` (D-16 passthrough) / `qr` (D-14 metadata) / `weather` (current-only) + `--clip` on json/qr + new `core::output::clip_feed` primitive for qr D-15. [wave 3, depends_on 07-02]

### Phase 8: Filesystem Depth

**Goal**: Land the deferred filesystem depth flags now that every command already has its `--json` output struct (new fields slot into both human and JSON paths for free) — including the three destructive flags, each isolated to its own plan with mandatory adversarial code review.
**Depends on**: Phase 7
**Requirements**: HASH-V2-02, FLAT-V2-01, FLAT-V2-02, DUPE-V2-01, DUPE-V2-02, RENM-V2-01, RENM-V2-02, TREE-V2-01, DU-V2-01, DU-V2-02
**Success Criteria** (what must be TRUE):

  1. `box hash a.bin b.bin` prints one coreutils `digest␣␣filename` (double-space) line per file with a stderr progress indicator for large inputs; `box flatten` honors `--extensions`, `--separator`, and `--include-hidden` with a stderr progress bar; `box tree` honors `.gitignore`, `--dirs-only`, `--ignore <glob>`, and sort-by-size; `box du` shows an in-line percentage column with color-coded size ranges and `--exclude <glob>`, and `box du --on-disk` reports allocated/compressed NTFS size; `box dupes` collapses hardlinked paths instead of counting them as wasted space; `box bulk-rename` supports upper/lower/title case transforms and a zero-padded `{n}` numbering token.
  2. **Destructive — `flatten --move`**: defaults to a dry-run that writes nothing; `--force` executes a copy → verify (dest exists + size matches) → delete-source sequence; every abort path leaves the source tree byte-for-byte unchanged.
  3. **Destructive — `dupes --delete`**: defaults to a dry-run; `--force` removes duplicates non-interactively while always keeping at least one copy per group, is hardlink-safe (never deletes a shared-inode path as a "duplicate"), and aborts the entire plan before any deletion if the pre-flight finds a problem.
  4. **Destructive — `bulk-rename --backup`**: writes a recoverable pre-rename backup of each affected file before the rename plan executes, and a failed/aborted run leaves the directory recoverable.
  5. Every depth flag respects the spine: `box du --on-disk --json`, `box tree --dirs-only --json`, etc. still emit one clean JSON document with the new fields and no ANSI/progress contamination.

**Plans**: TBD

Plans:

- [ ] 08-01: TBD — `hash` multi-file + coreutils format + progress (HASH-V2-02); `flatten` filters + progress (FLAT-V2-01).
- [ ] 08-02: TBD — `tree` gitignore/dirs-only/ignore/sort-by-size (TREE-V2-01); `du` percentage/colors/exclude + `--on-disk` Win32 (DU-V2-01, DU-V2-02).
- [ ] 08-03: TBD — `dupes` multi-stage + hardlink-aware (DUPE-V2-01); `bulk-rename` case + numbering (RENM-V2-01).
- [ ] 08-04: TBD — **DESTRUCTIVE (own plan + adversarial review):** `flatten --move` (FLAT-V2-02).
- [ ] 08-05: TBD — **DESTRUCTIVE (own plan + adversarial review):** `dupes --delete` (DUPE-V2-02).
- [ ] 08-06: TBD — **DESTRUCTIVE (own plan + adversarial review):** `bulk-rename --backup` (RENM-V2-02).

**Code-review gate**: Plans 08-04, 08-05, 08-06 each carry mandatory adversarial code review (the v1 Phase-3 bulk-rename gate) — dry-run default, `--force`, abort-all-before-any pre-flight, and a snapshot-the-tree-unchanged test for every abort path.
**UI hint**: yes

### Phase 9: Dev-Transform & Visual Depth

**Goal**: Add the deferred dev-transform and visual depth flags, each local to one command and additive to its existing output struct — including the animated `lolcat` path, which carries a PS7 human-verify gate.
**Depends on**: Phase 8
**Requirements**: UUID-V2-01, EPOC-V2-01, COLR-V2-01, JSON-V2-01, PASS-V2-01, LOL-V2-01, MTRX-V2-01, QR-V2-01, ASCI-V2-01
**Success Criteria** (what must be TRUE):

  1. `box uuid --v7` emits a time-ordered UUID and the format flags `--upper`, `--no-hyphens`, `--braces`, `--urn` each reshape the output; `box epoch` shows relative time ("3 hours ago") and `--tz <zone>` renders the timestamp in the named zone; `box color` resolves CSS named colors in both directions and accepts HSL input; `box json --sort-keys` sorts object keys while plain `box json` still preserves input key order; `box passgen` prints an entropy-bits estimate and honors `--no-similar` (drops `il1Lo0O`) and `--separator`.
  2. `box matrix` honors `--color`, `--speed`, and `--charset`; `box qr --save out.png` writes a PNG (and SVG) and `--error-correction L|M|Q|H` changes the EC level; `box ascii` produces truecolor output and honors `--braille` (2×4 dot density) and `--invert`.
  3. **Human-verify (PS7):** `box lolcat --animate` produces a smooth animated rainbow in the PowerShell 7 terminal with RAII terminal restore (clean exit on Ctrl+C/q/Esc, no stuck raw mode), and degrades to a static render when piped or under `--json`; `--freq` and `--seed` visibly change the gradient. This criterion is cleared by a human in PS7, not by automated test alone.
  4. New colored output paths (`matrix --color`, `ascii` truecolor, animated `lolcat`) stay gated on `is_color_on()` — piped/`--json` output remains byte-identical minus ANSI.

**Plans**: TBD

Plans:

- [ ] 09-01: TBD — dev transforms: `uuid` v7 + format flags (UUID-V2-01); `epoch` relative + tz (EPOC-V2-01); `color` CSS + HSL (COLR-V2-01); `json --sort-keys` (JSON-V2-01); `passgen` entropy + `--no-similar` + `--separator` (PASS-V2-01).
- [ ] 09-02: TBD — visuals: `matrix` color/speed/charset (MTRX-V2-01); `qr` `--save`/EC (QR-V2-01); `ascii` color/braille/invert (ASCI-V2-01).
- [ ] 09-03: TBD — **HUMAN-VERIFY (PS7):** `lolcat` `--animate`/`--freq`/`--seed` with RAII terminal restore + TTY-gated static fallback (LOL-V2-01).

**UI hint**: yes

### Phase 10: Fun & System Depth

**Goal**: Add the remaining fun and system depth flags, including the two Win32-touching system features (`pomodoro --sound`, weather config-default location) layered onto the now-settled config resolver.
**Depends on**: Phase 9
**Requirements**: COW-V2-01, FORT-V2-01, 8BAL-V2-01, ROST-V2-01, POMO-V2-01, POMO-V2-02, WTHR-V2-01
**Success Criteria** (what must be TRUE):

  1. `box cowsay` offers multiple figures and a think-mode bubble; `box fortune` accepts selectable categories; `box 8ball` renders an ASCII-art presentation with sentiment color; `box roast --language <lang>` changes the roast language.
  2. `box pomodoro` tracks a session counter and auto-cycles work/break sessions, accepts `--label`, and `box pomodoro --sound` plays a completion beep via Win32 `MessageBeep` (no audio stack pulled in).
  3. `box weather --forecast` returns a multi-day forecast, repeated calls are served from a response cache, and with `weather.location` set in config, bare `box weather` uses the stored default location (reading the config resolver from Phase 6).
  4. The new colored/sentiment output (`8ball` sentiment color) stays gated on `is_color_on()`, and every applicable command here still emits a clean `--json` document with the new fields.

**Plans**: TBD

Plans:

- [ ] 10-01: TBD — fun: `cowsay` figures/think (COW-V2-01); `fortune` categories (FORT-V2-01); `8ball` art/sentiment (8BAL-V2-01); `roast --language` (ROST-V2-01).
- [ ] 10-02: TBD — system: `pomodoro` counter/auto-break/`--label` + `--sound` Win32 MessageBeep (POMO-V2-01, POMO-V2-02); `weather` `--forecast`/cache/stored-location (WTHR-V2-01).

### Phase 11: Meta-Commands

**Goal**: Ship the two frictionless-PS7 meta-commands last, generated against the now-final and complete `Cli` arg surface so `completions` reflects every flag added in Phases 6-10.
**Depends on**: Phase 10
**Requirements**: CFG-01, CMP-01
**Success Criteria** (what must be TRUE):

  1. `box config show`, `box config get <key>`, `box config set <key> <value>`, and `box config path` read, edit, and locate `%APPDATA%\box\config.toml` (keys `hash.default_algo`, `weather.units`, `weather.location`, `color`, …); `box config show --json` emits the effective config as a JSON document (dogfooding the spine).
  2. `box config set hash.default_algo sha256` followed by a bare `box hash file.bin` round-trips through the resolver and emits SHA-256, proving the meta-command exercises the same precedence chain the spine uses.
  3. `box completions powershell` emits a static PS7 completion script that, once registered, tab-completes subcommands, flags, and value hints — including the global `--json`/`--clip` and *every* Phase-8/9/10 depth flag (the script is generated from the live, final `Cli`).
  4. A first-time user with no config file still runs every command unchanged (config remains purely additive), and a malformed config file produces a clear error rather than a panic.

**Plans**: TBD

Plans:

- [ ] 11-01: TBD — `box config` show/get/set/path with `--json` (CFG-01).
- [ ] 11-02: TBD — `box completions powershell` via `clap_complete` against the final `Cli` + `install.ps1` registration hint + BLAKE3 breaking-change help/PROJECT note (CMP-01).

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation + Flatten | v1.0 | 4/4 | Complete | 2026-06-22 |
| 2. Pure Transform Utilities | v1.0 | 5/5 | Complete | 2026-06-22 |
| 3. Filesystem Power Tools | v1.0 | 5/5 | Complete | 2026-06-23 |
| 4. Terminal Visuals | v1.0 | 4/4 | Complete | 2026-06-24 |
| 5. Windows Platform Integration | v1.0 | 4/4 | Complete | 2026-06-24 |
| 6. Scriptable-Core Foundation | v2.0 | 2/2 | Complete | 2026-06-25 |
| 7. Spine Rollout | v2.0 | 0/3 | Planned | - |
| 8. Filesystem Depth | v2.0 | 0/6 | Not started | - |
| 9. Dev-Transform & Visual Depth | v2.0 | 0/3 | Not started | - |
| 10. Fun & System Depth | v2.0 | 0/2 | Not started | - |
| 11. Meta-Commands | v2.0 | 0/2 | Not started | - |

---
*Roadmap created: 2026-06-22*
*v1.0 milestone closed 2026-06-24 — Phases 1–5 collapsed; full detail in `milestones/v1.0-ROADMAP.md`.*
*v2.0 phases (6–11) added 2026-06-25 by roadmapper — scriptable-core spine (6) → spine rollout (7) → filesystem depth (8) → dev-transform & visual depth (9) → fun & system depth (10) → meta-commands (11). All 34 v2.0 requirements mapped; coverage 34/34.*
