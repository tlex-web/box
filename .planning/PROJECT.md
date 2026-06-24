# box — A Rust CLI Toolbox

## What This Is

`box` is a single Rust binary that bundles ~23 command-line tools — a mix of genuinely useful utilities and fun toys — exposed as subcommands (`box flatten`, `box qr`, `box cowsay`, …). It's built for a developer running Windows PowerShell 7, installed globally so any tool is one short command away.

## Core Value

The toolbox must be **globally available and instantly usable from PowerShell 7** — type `box <command>` from anywhere and the tool just works. If the breadth of commands is impressive but they aren't reachable on PATH, the project has failed at its one job.

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

### Active

<!-- Current scope. Building toward these. All are hypotheses until shipped. -->

None — all 23 v1 commands shipped and validated. **v1 milestone complete (2026-06-24).** Future ideas (BLAKE3-default hash `HASH-V2-01`, colored ASCII / animated lolcat `VIS-V2-01`, etc.) are deferred to a v2 milestone.

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- Cross-platform support (macOS / Linux) — target is Windows PowerShell 7; portability can come later if wanted
- Package-manager distribution (Scoop / winget) — `install.ps1` is the chosen install path for v1; packaging adds overhead without changing the core experience
- GUI / TUI dashboard — these are discrete one-shot terminal commands, not an interactive app
- Per-tool standalone binaries — deliberately one binary with subcommands for a single, discoverable PATH entry

## Context

- **Environment:** Windows 11, PowerShell 7 terminal. Several commands are inherently Windows-flavored (clipboard access, toast notifications, terminal color/Unicode rendering) and will rely on Windows-appropriate crates/APIs.
- **Language:** Rust — chosen for a single fast, dependency-free distributable binary.
- **Network-dependent commands:** `weather` (and potentially others) require an external web API; the research phase should identify a suitable no-/low-friction API and the right HTTP/QR/clipboard/image crates.
- **Greenfield:** brand-new project, no existing code. Repo initialized fresh.

## Constraints

- **Tech stack**: Rust, single binary with a subcommand CLI (e.g. `clap`) — keep the surface consistent across all 23 tools.
- **Platform**: Must run in Windows PowerShell 7. Windows-specific integrations (clipboard, toast, ANSI color) must work in that terminal.
- **Distribution**: Global availability achieved via `install.ps1` (build → copy to bin dir → add to user PATH). No assumption that `~/.cargo/bin` is already on PATH.
- **Consistency**: All subcommands share a coherent UX — `--help` everywhere, predictable flag conventions, sensible exit codes.

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Single binary `box` with subcommands | One discoverable PATH entry; avoids name clashes and PATH clutter from many small binaries | — Pending |
| Binary named `box` | Short, neutral, fast to type, no common Windows clash | — Pending |
| `install.ps1` for global install | Doesn't assume cargo bin is on PATH; copies exe to a dedicated bin dir and updates user PATH | — Pending |
| `flatten` collisions → prefix with source path | Predictable, traceable to origin, nothing silently lost or overwritten | — Pending |
| All 23 commands targeted for v1 | User wants the full toolbox to land together | — Pending |
| Rust as implementation language | Single fast native binary, easy global distribution | — Pending |

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
*Last updated: 2026-06-24 — Phase 5 (Windows Platform Integration) complete: four commands (qr, clip, pomodoro, weather) shipped, verified 16/16, human-UAT 3/3 cleared in PS7; code-review WR-01 (pomodoro out-of-range no-panic) + WR-02 (clip empty-clipboard message) fixed with covering tests. **All 23/23 v1 commands live → v1 MILESTONE COMPLETE.** Release x86_64-pc-windows-msvc +crt-static box.exe builds (5.1 MB). Next = `/gsd:complete-milestone` to archive v1, or `./install.ps1` to ship box.exe to PATH.*
