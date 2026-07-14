# box — A Rust CLI Toolbox

## What This Is

`box` is a single Rust binary that bundles 23 command-line tools — a mix of genuinely useful utilities and fun toys — exposed as subcommands (`box flatten`, `box qr`, `box cowsay`, …). It's built for a developer running Windows PowerShell 7, installed globally so any tool is one short command away. **v1.0 shipped 2026-06-24 — all 23 commands live.** **v2.0 (in progress) deepens these into a scriptable toolkit** — `--json`/`--clip` everywhere applicable, PS7 completions, and config-file defaults.

## Core Value

The toolbox must be **globally available and instantly usable from PowerShell 7** — type `box <command>` from anywhere and the tool just works. If the breadth of commands is impressive but they aren't reachable on PATH, the project has failed at its one job.

## Current Milestone: v2.0 — Toolbox → Toolkit

**Goal:** Make the existing 23 commands deeper, scriptable, and frictionless in PowerShell 7 — turning the toolbox into a toolkit. No new *tool* commands; only `completions` / `config` meta-commands.

**Target features:**
- **Scriptable spine (cross-cutting):** `--json` machine-readable output across all applicable commands (`box … | ConvertFrom-Json` → PS7 objects); `--clip` to push output to the Windows clipboard (reusing v1's arboard infra).
- **Per-command depth (comprehensive — the full deferred-V2 set):** filesystem (flatten / hash / dupes / bulk-rename / tree / du), dev transforms (uuid v7, epoch tz, color CSS+HSL, json `--sort-keys`, passgen entropy / `--no-similar` / `--separator`), visuals (lolcat `--animate`/`--freq`/`--seed`, matrix color/speed/charset, qr `--save`/EC, ascii color/braille/invert), fun (cowsay figures/think, fortune categories, 8ball art/sentiment, roast `--language`), system (pomodoro counter/auto-break/sound/`--label`, weather `--forecast`/cache/stored-location).
- **Frictionless PS7:** `box completions powershell` (clap_complete); config-file defaults (`box config` + config file for default algo / units / location / color).

**Breaking change:** `hash` switches its default algorithm to BLAKE3 (`HASH-V2-01`), superseding the v1 `HASH-01` SHA-256-default binding contract — the major-version trigger.

**Explicitly out of scope (this milestone):** self-update; Scoop / winget packaging (v1 distribution boundary preserved).

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

**Foundation** *(validated in Phase 1 — Foundation + Flatten, 2026-06-22)*
- [x] Single binary `box` with discoverable subcommands (`box --help`, `box <cmd> --help`)
- [x] `install.ps1` builds the release binary, copies it to a dedicated bin dir, and adds that dir to the user PATH so `box` works globally in PowerShell 7 (same-session, human-verified)

**Anchor command** *(validated in Phase 1)*
- [x] `flatten` — recursively copy every file from a folder tree into one flat output folder (originals untouched, folders dropped); on filename collision, rename by encoding the source path (e.g. `docs_sub_report.txt`). Silent-overwrite edge cases (Windows trailing dot/space, non-ASCII case, unconditional copy) hardened post-review.

**Pure transform utilities** *(validated in Phase 2 — Pure Transform Utilities, 2026-06-22)*
- [x] `uuid` — generate v4 UUIDs (`-n N` for count, `--upper`)
- [x] `base64` — encode / decode (`--decode`, `--url-safe`); byte-exact round-trip, missing input → exit 2
- [x] `epoch` — convert between Unix timestamps and human-readable dates (no-arg → now; timestamp → local + UTC; date string → timestamp)
- [x] `color` — convert between hex and RGB/HSL formats; gated truecolor swatch (plain glyphs when piped)
- [x] `passgen` — CSPRNG passwords + EFF passphrases (`--length`, `--words`, `--count`, `--no-symbols`); OsRng source, unbiased selection
- [x] `cowsay` — ASCII-character speech bubbles (greedy 40-col wrap + hard-break)
- [x] `fortune` — random fortune / quote
- [x] `8ball` — magic 8-ball oracle
- [x] `roast` — random programmer-roast generator

**Filesystem power tools** *(validated in Phase 3 — Filesystem Power Tools, 2026-06-23)*
- [x] `hash` — compute/verify file checksums (default SHA-256; `--algo blake3|sha512|md5`; stdin or `--file`; `--verify` with the 0/1/2 exit contract)
- [x] `tree` — box-drawing directory tree (dir-first, colored dirs, `--sizes`, `--depth`, `N directories, M files` summary)
- [x] `du` — size-sorted disk usage (biggest-first, recursive dir totals, `--top`, `--depth`, human-readable sizes)
- [x] `dupes` — content-duplicate finder (size pre-filter → parallel BLAKE3, grouped output, wasted-space summary; strictly read-only)
- [x] `bulk-rename` — regex bulk rename, dry-run preview by default + `--force` to execute, abort-all-before-any pre-flight (collisions / cycles / path-escape)

**Terminal visuals** *(validated in Phase 4 — Terminal Visuals, 2026-06-24)*
- [x] `json` — pretty-print + validate JSON (2-space pretty, syntax-colored TTY, `preserve_order`; invalid → exit 1 with 1-based line/column; `--compact` minify)
- [x] `lolcat` — rainbow-colorize piped text (per-Unicode-scalar truecolor sine gradient, width-aware; unconditional ANSI strip; byte-identical-minus-color when piped)
- [x] `ascii` — render an image (PNG/JPEG) as ASCII art fitted to terminal width (monochrome v1)
- [x] `matrix` — full-terminal green katakana digital-rain (~20 FPS, RAII terminal restore, exits cleanly on Ctrl+C/q/Esc; human-verified)

**Windows platform integration** *(validated in Phase 5 — Windows Platform Integration, 2026-06-24)*
- [x] `qr` — render scannable QR codes for text/URLs in the terminal (qrcode 0.14 Dense1x2 Unicode half-blocks, EcLevel::M, no ANSI; phone-scan human-verified in PS7)
- [x] `clip` — read from / write to the Windows clipboard (`box clip` copies raw stdin with a single-trailing-newline trim; `box clip --paste` writes byte-exact; arboard/Win32, no elevation; Unicode round-trip human-verified in PS7)
- [x] `pomodoro` — focus timer with a live in-place countdown + Windows 11 toast on completion (`--break` 5 / `--long-break` 15 / `[MINUTES]`); clean Ctrl+C/q/Esc cancel with no toast; human-verified in PS7
- [x] `weather` — keyless current weather via Open-Meteo (city geocode or `lat,lon`; `--units metric|imperial`; graceful offline error); unit labels read from the response, never hardcoded

**Scriptable-core foundation** *(validated in Phase 6 — Scriptable-Core Foundation, 2026-06-25)*
- [x] Shared scriptable spine built once — `core::output` primitives (`emit_json`/`out_line`/`flush_clip`/`init_output`/`is_json_on`, JSON-purity: one document, no BOM, no ANSI, no chrome) + hand-rolled `core::config` resolver (precedence **CLI > env > config > built-in**; missing/malformed config never errors a normal command) + `BoxError::Config` (exit 2) + global `--json`/`--clip` flags. Proven end-to-end on the `uuid` + `hash` pilots; the `{results,count}` serde struct and `json_purity` test are the frozen copy-me template for the remaining 21 commands (rollout in Phases 7–10).
- [x] BLAKE3-default `hash` (breaking change to the compute default only — `box hash file` now emits BLAKE3; `--algo sha256` and `hash.default_algo` config restore old behavior; `--verify` length table untouched so stored SHA-256 baselines never break; D-05 BLAKE3-fallback diagnostic on 64-hex mismatch).

**Scriptable spine rollout** *(validated in Phase 7 — Spine Rollout, 2026-06-25)*
- [x] `--json` on all 16 applicable value-producing commands (SPINE-02) — base64, epoch, color, passgen, 8ball, fortune, roast, cowsay, du, tree, dupes, flatten, bulk-rename, json, qr, weather — each emits exactly one parseable JSON document via the frozen Phase-6 `is_json_on()` fork + `{results,count}` (or D-17 recursive `tree`) template; per-command `json_purity` test (no `0x1B`, no BOM, single value) passes for all 16. Edge policies locked: `base64 --decode --json` re-encodes non-UTF-8 bytes losslessly (A1); `bulk-rename` conflict/abort path keeps stdout byte-empty under `--json` (A3/D-09).
- [x] `--clip` on the 6 new single-textual-result commands (SPINE-04) — color, base64, epoch, passgen, json, qr (joining the Phase-6 uuid/hash pilots) — copies the primary text to the Windows clipboard while still printing it; `qr --clip` copies the SOURCE TEXT, not glyphs (D-15), via the one sanctioned new `core::output::clip_feed` primitive. Display-only commands (matrix, pomodoro, lolcat, ascii, clip) correctly omit the spine flags (SC4). All 6 live clipboard round-trips human/machine-verified on Windows 2026-06-25.

**Dev-transform & visual depth** *(validated in Phase 9 — Dev-Transform & Visual Depth, 2026-07-14)*
- [x] Dev transforms — `uuid` v7 + `--upper`/`--no-hyphens`/`--braces`/`--urn` (UUID-V2-01); `epoch` relative time + `--tz <zone>` (EPOC-V2-01); `color` CSS named-color lookup both directions + HSL input (COLR-V2-01); `json --sort-keys` opt-in recursive key sort with default input order preserved (JSON-V2-01); `passgen` entropy-bits estimate + `--no-similar` (drops `il1Lo0O`) + `--separator` (PASS-V2-01). Each additive to its frozen Phase-7 `{results,count}` output struct.
- [x] Visual depth — `matrix` `--color`/`--speed`/`--charset` presets (MTRX-V2-01); `qr --save` PNG/SVG + `--error-correction L|M|Q|H` feeding both terminal + file (QR-V2-01); `ascii` truecolor default-on + `--braille` 2×4 engine + `--invert` (ASCI-V2-01); `lolcat --animate`/`--freq`/`--seed`/`--duration` — a bounded alternate-screen rainbow loop reusing `matrix`'s RAII restore behind a mandatory `is_terminal()` AND-gate, degrading to the static render off-TTY (LOL-V2-01; PS7 human-verified). All new colored paths stay `is_color_on()`-gated (SC4). Code-review critical BL-01 (`--duration` `Instant` overflow bypassing `RawGuard` under `panic = "abort"`) fixed with a `checked_add` guard + unit test.

### Active

<!-- Current scope. Building toward these. All are hypotheses until shipped. -->

**v2.0 Toolbox → Toolkit** *(scoped 2026-06-24; detailed REQ-IDs land in REQUIREMENTS.md)*
- [x] Scriptable spine — `--json` output + `--clip` clipboard across all applicable commands *(foundation + uuid/hash pilots shipped in Phase 6; full rollout across the remaining 16/6 commands completed in Phase 7 — SPINE-02 + SPINE-04 done)*
- [ ] Comprehensive per-command depth — the full deferred-V2 set (FLAT / HASH / DUPE / RENM / TREE / DU / DEV / VIS / FUN / SYS / PASS-V2) *(filesystem depth shipped in Phase 8; dev-transform + visual + PASS-V2 shipped in Phase 9; fun + system depth remain in Phase 10)*
- [x] BLAKE3-default `hash` (breaking; supersedes the `HASH-01` SHA-256 default) *(delivered in Phase 6 — compute-default flip with config/env escape hatch; verify length table unchanged)*
- [x] PS7 shell completions (`completions` meta-command) *(delivered in Phase 11 — `box completions <shell>` via `clap_complete::generate` against the live final `Cli`, so the emitted script auto-reflects every Phase 6–10 flag; `install.ps1 -RegisterCompletions` opt-in idempotent `$PROFILE` registration; CMP-01)*
- [x] Config-file defaults (`config` meta-command + config file) *(resolver + `%APPDATA%\box\config.toml` defaults shipped in Phase 6; `box config show/get/set/path` + `--json` effective-config view delivered in Phase 11 — CFG-01; gap-closure 11-03 hardened env-tier parity [`config` now reports exactly what `box hash` consumes via one shared resolver] and the malformed-config repair path [`config path`/`set` + `completions` stay usable, `set` repairs the file])*

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- Cross-platform support (macOS / Linux) — target is Windows PowerShell 7; portability can come later if wanted
- Package-manager distribution (Scoop / winget) — `install.ps1` is the chosen install path for v1; packaging adds overhead without changing the core experience
- GUI / TUI dashboard — these are discrete one-shot terminal commands, not an interactive app
- Per-tool standalone binaries — deliberately one binary with subcommands for a single, discoverable PATH entry

## Context

- **Current state (v1.0, 2026-06-24):** All 23 commands shipped and validated. ~7,748 Rust LOC across 32 `.rs` files; 150 bin-unit + all integration tests green; clippy `--all-targets -D warnings` + `fmt --check` clean. Release `x86_64-pc-windows-msvc` + `crt-static` `box.exe` builds at 5.1 MB (portable).
- **Environment:** Windows 11, PowerShell 7 terminal. Several commands are inherently Windows-flavored (clipboard via arboard, toast via tauri-winrt-notification, terminal color/Unicode rendering via crossterm) and rely on Windows-appropriate crates/APIs — all working in PS7.
- **Language:** Rust — a single fast, statically-linked distributable binary.
- **Network-dependent commands:** `weather` uses Open-Meteo (keyless geocode + forecast via ureq); the only networked command.
- **Architecture:** single crate, `src/commands/<cmd>/mod.rs` per command, `RunCommand` trait, ~40-line dispatch-only `main.rs`, shared `src/core/` (errors / output / fs / input).

## Constraints

- **Tech stack**: Rust, single binary with a subcommand CLI (e.g. `clap`) — keep the surface consistent across all 23 tools.
- **Platform**: Must run in Windows PowerShell 7. Windows-specific integrations (clipboard, toast, ANSI color) must work in that terminal.
- **Distribution**: Global availability achieved via `install.ps1` (build → copy to bin dir → add to user PATH). No assumption that `~/.cargo/bin` is already on PATH.
- **Consistency**: All subcommands share a coherent UX — `--help` everywhere, predictable flag conventions, sensible exit codes.

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Single binary `box` with subcommands | One discoverable PATH entry; avoids name clashes and PATH clutter from many small binaries | ✓ Good — one clap-derive registry, all 23 commands in `box --help` |
| Binary named `box` | Short, neutral, fast to type, no common Windows clash | ✓ Good — no clash observed in PS7 |
| `install.ps1` for global install | Doesn't assume cargo bin is on PATH; copies exe to a dedicated bin dir and updates user PATH | ✓ Good — same-session install + `box --help` smoke-test human-verified in PS7 (idempotent REG_EXPAND_SZ-safe PATH) |
| `flatten` collisions → prefix with source path | Predictable, traceable to origin, nothing silently lost or overwritten | ✓ Good — plus post-review hardening of trailing-dot/space + non-ASCII-case + `create_new` loud-fail |
| All 23 commands targeted for v1 | User wants the full toolbox to land together | ✓ Good — all 23 shipped & validated in one milestone |
| Rust as implementation language | Single fast native binary, easy global distribution | ✓ Good — 5.1 MB `crt-static` portable exe |
| Order phases by integration risk (pure → fs → visuals → platform) | Find architecture problems on `uuid`, not `flatten`; attempt riskiest Windows-API/network integrations last with 21 commands already working | ✓ Good — Phase 5's new Windows-API deps compiled first try against a stable base |
| SHA-256 (not BLAKE3) as `hash` default; BLAKE3 via `--algo` | HASH-01 binding contract; interop with `sha256sum`/Docker workflows | ✓ Good — BLAKE3-default deferred to HASH-V2-01 |
| `x86_64-pc-windows-msvc` + `crt-static` target | MinGW Tier-2 in Rust 1.88; MSVC required for arboard/winrt; static CRT = portable exe | ✓ Good — links all 4 Windows-API deps, runs standalone |
| Override two CLAUDE.md crate recs after slop-check (`qrcode` over qr2term, `tauri-winrt-notification` over winrt-notification) | Maintenance + modern `windows 0.61`; validated legitimate before adoption | ✓ Good — both compiled clean, toast + QR human-verified |
| Per-phase post-execution code review (not just verification) | Verification confirms the feature works; adversarial review finds path-escapes / terminal-restore bugs | ✓ Good — caught 2 BLOCKERs verification missed (bulk-rename `..` escape, matrix raw-mode-stuck) |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-07-14 after Phase 11 (Meta-Commands) completion — the LAST v2.0 phase. Moved both frictionless-PS7 meta-command requirements into Validated: `box config` show/get/set/path + `--json` effective-config view (CFG-01) and `box completions <shell>` generated against the live final `Cli` (CMP-01). Verified 6/6 must-haves by direct binary execution. Gap-closure plan 11-03 closed the two verification findings: WR-01 (env-tier parity — `config get/show hash.default_algo` now routes through one shared `pub(crate) hash::effective_default_algo()`, so it can never lie about what `box hash` consumes) and WR-02 (malformed-config decoupling — `config path/set` + `completions` fall back to defaults and stay usable, `config set` repairs the file, while normal commands and `config show/get` still exit 2 with no tolerance hole). Full `cargo test` (507+, 0 failures) + `clippy --all-targets -D warnings` clean. One manual PS7 tab-completion smoke test tracked in 11-HUMAN-UAT.md. Code-review re-pass flagged one pre-existing advisory (orphaned `core::config::resolve_algo` dead code, unrelated to 11-03). Next: v2.0 milestone close-out (`/gsd:complete-milestone`).*

*Last updated: 2026-07-14 after Phase 9 (Dev-Transform & Visual Depth) completion — moved the dev-transform (uuid v7 + format flags, epoch relative/`--tz`, color CSS+HSL, `json --sort-keys`, passgen entropy/`--no-similar`/`--separator`) and visual-depth (matrix `--color`/`--speed`/`--charset`, qr `--save` PNG/SVG + EC, ascii truecolor/`--braille`/`--invert`, lolcat `--animate`) requirements into Validated — all 9 reqs (UUID/EPOC/COLR/JSON/PASS/LOL/MTRX/QR/ASCI-V2-01) delivered and verified 13/13 must-haves, full `cargo test` green (207 unit + integration) + clippy clean. `lolcat --animate` cleared the sole PS7 human-verify gate; the code-review critical BL-01 (`--duration` `Instant` overflow that would bypass the `RawGuard` terminal restore under `panic = "abort"`) was fixed with a `checked_add` guard + a terminal-free unit test. Three advisory code-review warnings deferred (matrix `--charset` width validation, passgen `--separator` newline validation, lolcat double-teardown idempotency). Next: Phase 10 — Fun & System Depth.*

*Last updated: 2026-06-28 after Phase 8 (Filesystem Depth) completion — all 10 deferred depth requirements shipped and verified (HASH-V2-02 multi-file hash; FLAT-V2-01/02 flatten filters + `--move`; DUPE-V2-01/02 multi-stage/hardlink-aware dupes + `--delete`; RENM-V2-01/02 bulk-rename `--case`/`{n}` + `--backup`; TREE-V2-01 tree gitignore/ignore/dirs-only/sort; DU-V2-01/02 du percentage/exclude + `--on-disk`). The three destructive flags each passed a per-plan adversarial review (flatten `--move` two-phase copy→verify→delete; dupes `--delete` keep-first + hardlink-safe + abort-all-before-any; bulk-rename `--backup` fsync'd undo manifest outside the tree). The phase code-review caught and fixed a pre-existing v1 bulk-rename Windows trailing-dot/reserved-name silent-clobber data-loss path (CR-01) plus 4 warnings. Two empirical UAT items (PS7 progress bar; `du --on-disk` on a compressed file) remain tracked in 08-HUMAN-UAT.md. Next: Phase 9 — Dev-Transform & Visual Depth.*

*Last updated: 2026-06-25 after Phase 7 (Spine Rollout) completion — moved the full `--json`/`--clip` rollout into Validated (SPINE-02 on all 16 value commands, SPINE-04 on the 6 new clip commands; A1/A3/D-09/D-15/SC4 edge policies locked; `clip_feed` added as the one sanctioned new primitive). The milestone-level "Scriptable spine" Active item is now fully shipped. Next: Phase 8 — Filesystem Depth (the deferred depth flags on the six filesystem commands, including the three destructive flags under adversarial review).*

*Last updated: 2026-06-25 after Phase 6 (Scriptable-Core Foundation) completion — moved the shared `--json`/`--clip`/config spine foundation and the BLAKE3-default `hash` flip into Validated (proven on the `uuid`+`hash` pilots; `{results,count}`/`json_purity` frozen as the Phase-7 rollout template); annotated the milestone-level Active items with their Phase-6 progress. Next: Phase 7 — Spine Rollout (apply the template to the remaining applicable commands).*

*Last updated: 2026-06-24 after v2.0 milestone kickoff (/gsd:new-milestone) — added Current Milestone (Toolbox → Toolkit); Active set to v2.0 scope (scriptable `--json`/`--clip` spine + comprehensive per-command depth + PS7 completions + config-file defaults); BLAKE3-default `hash` recorded as the breaking change; self-update & Scoop/winget packaging held out of scope. Next: research → REQUIREMENTS.md (REQ-IDs) → ROADMAP.md (phases continue numbering from v1's Phase 5).*
