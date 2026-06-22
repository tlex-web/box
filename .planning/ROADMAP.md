# Roadmap: box — Rust CLI Toolbox

**Project:** box
**Milestone:** v1 — Full 23-command toolbox
**Created:** 2026-06-22
**Granularity:** Standard (5 phases)
**Mode:** mvp

## Phases

- [x] **Phase 1: Foundation + Flatten** — Binary scaffold, install script, core infrastructure, and the anchor `flatten` command. Shipped 2026-06-22: install.ps1 + same-session flatten human-verified in PS7; verification passed (5/5); code-review silent-data-loss findings (CR-01/WR-01/WR-02) fixed
- [x] **Phase 2: Pure Transform Utilities** — Nine zero-external-dependency commands proving the RunCommand pattern: uuid, base64, epoch, color, passgen, cowsay, fortune, 8ball, roast (completed 2026-06-22)
- [ ] **Phase 3: Filesystem Power Tools** — Five commands sharing walkdir: hash, tree, du, dupes, bulk-rename
- [ ] **Phase 4: Terminal Visuals** — Four commands sharing crossterm and rendering libraries: lolcat, matrix, ascii, json
- [ ] **Phase 5: Windows Platform Integration** — Four commands with the highest external/API risk: qr, clip, pomodoro, weather

## Phase Details

### Phase 1: Foundation + Flatten

**Goal**: Users can install `box` globally from PowerShell 7 with one script and immediately use `flatten` as a real, safe file-flattening tool
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: FOUND-01, FOUND-02, FOUND-03, FOUND-04, FOUND-05, FOUND-06, FOUND-07, FOUND-08, FLAT-01, FLAT-02, FLAT-03, FLAT-04
**Success Criteria** (what must be TRUE):

  1. User runs `.\install.ps1` in PowerShell 7 and immediately types `box --help` in the same session to see all 23 subcommand stubs listed with one-line descriptions — no new terminal window required
  2. User runs `box --version` and sees the semantic version; `box badcmd` exits with code 2 and a helpful error; any command error writes to stderr only, never stdout
  3. User pipes `box flatten --help` output to a file and gets clean text (no ANSI escape sequences), then runs the same command in a terminal and sees colored output
  4. User runs `box flatten ./src ./out --dry-run` on a deeply nested folder tree containing duplicate filenames and sees a plan of collision-renamed paths without any files being copied
  5. User runs `box flatten ./src ./out` (no dry-run) and all source files appear flat in `./out` with original timestamps preserved, originals untouched, and no files silently lost or overwritten

**Plans**: 4 plans
Plans:
**Wave 1**

- [x] 01-01-PLAN.md — Crate scaffold: clap registry (23 commands), RunCommand trait, BoxError, exit-code mapping, Wave-0 cli tests (FOUND-01,02,03,05)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-02-PLAN.md — Core infra: color gating (output.rs) + UNC-safe path/hidden/timestamp helpers (fs.rs); piped-no-ANSI test (FOUND-04,06)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 01-03-PLAN.md — Anchor `flatten`: pure collision/reserved-name rename + canonicalize/guard/plan/dry-run/execute orchestration (FLAT-01,02,03,04)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 01-04-PLAN.md — install.ps1: build → copy → idempotent REG_EXPAND_SZ-safe PATH → session refresh → smoke test + human-verify (FOUND-07,08) — *human-verify gate CLEARED 2026-06-22 (install + same-session flatten verified in PS7)*

### Phase 2: Pure Transform Utilities

**Goal**: Users can run nine lightweight utility commands — uuid, base64, epoch, color, passgen, cowsay, fortune, 8ball, roast — each with correct output conventions and integration-tested behavior
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: UUID-01, B64-01, EPOC-01, COLR-01, PASS-01, COW-01, FORT-01, 8BAL-01, ROST-01
**Success Criteria** (what must be TRUE):

  1. User runs `box uuid`, `box uuid -n 5`, and `box uuid --upper` and gets correctly formatted UUIDs — one per line, lowercase v4 by default
  2. User pipes a string through `box base64` to encode it, then pipes the result through `box base64 --decode` and recovers the original; `--url-safe` flag produces URL-safe alphabet with no padding issues
  3. User runs `box epoch` with no args and gets the current Unix timestamp; supplies a timestamp and gets local + UTC human date; supplies a date string and gets the Unix timestamp back
  4. User runs `box passgen` and gets a 16-char cryptographically random password; `box passgen --words 4` gets a four-word passphrase; `box passgen --count 10` prints 10 passwords
  5. User runs `box cowsay "hello"`, `box fortune`, `box 8ball "Will it work?"`, and `box roast` — each prints the expected styled ASCII output, different random output on repeated calls for fortune/8ball/roast

**Plans**: 5 plans
Plans:
**Wave 1**

- [x] 02-01-PLAN.md — Foundation: `core::input` reader (read_input/read_input_bytes, D-04/D-05) + `BoxError::MissingInput`->exit-2 wiring + add the 4 locked crates to Cargo.toml (shared)

**Wave 2** *(blocked on Wave 1)*

- [x] 02-02-PLAN.md — uuid (v4, `-n`, `--upper`) + base64 (encode/decode round-trip, `--url-safe`, first read_input_bytes consumer) (UUID-01, B64-01)

**Wave 3** *(blocked on Wave 2 — shares cli.rs/main.rs registry)*

- [x] 02-03-PLAN.md — epoch (3-mode timestamp<->date, D-12) + color (hex/RGB<->HSL bidirectional + gated truecolor swatch, D-10/D-13) (EPOC-01, COLR-01)

**Wave 4** *(blocked on Wave 3 — shares cli.rs/main.rs registry)*

- [x] 02-04-PLAN.md — passgen (OsRng CSPRNG, unbiased selection, EFF wordlist, paste-safe charset, T-V6) + cowsay (40-col wrap + hard-break + classic bubble, D-11); creates src/data/ (PASS-01, COW-01)

**Wave 5** *(blocked on Wave 4 — shares cli.rs/main.rs registry)*

- [x] 02-05-PLAN.md — fortune + roast (include_str! CC0 lists) + 8ball (canonical 20, eight_ball module) — rand::rng() + membership/varies-across-runs tests (FORT-01, 8BAL-01, ROST-01)

### Phase 3: Filesystem Power Tools

**Goal**: Users can hash files, explore disk usage visually, find duplicate files, and bulk-rename files with safe dry-run-first workflows
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: HASH-01, TREE-01, DU-01, DUPE-01, RENM-01
**Success Criteria** (what must be TRUE):

  1. User runs `box hash somefile.zip`, gets SHA-256 in `HASH  filename` format; `--algo blake3` switches algorithm; `box hash --verify EXPECTEDHASH somefile.zip` exits 0 on match and 1 on mismatch
  2. User runs `box tree ./src` and sees a box-drawing character tree with colored directory names vs files, optional file sizes via `--sizes`, depth limit via `--depth N`, and a file/dir count summary
  3. User runs `box du ./project` and sees a size-sorted list (biggest first) with human-readable sizes; `--top 10` truncates to 10 entries; `--depth 2` limits traversal depth
  4. User runs `box dupes ./downloads` and sees groups of identical files identified by content hash with wasted-space summary; no files are deleted or modified
  5. User runs `box bulk-rename ./photos "(\d+)" "img_$1"` and gets a dry-run preview by default showing every planned rename; `--force` executes the renames; collision detection aborts before any rename if a conflict is found

**Plans**: TBD

### Phase 4: Terminal Visuals

**Goal**: Users can colorize piped text with a rainbow gradient, run a Matrix digital-rain animation, render image files as ASCII art, and pretty-print/validate JSON
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: LOL-01, MTRX-01, ASCI-01, JSON-01
**Success Criteria** (what must be TRUE):

  1. User pipes multi-line text through `box lolcat` and sees a smooth truecolor rainbow gradient in the terminal; piping to a file strips all ANSI and produces clean plain text
  2. User runs `box matrix` and sees a full-terminal green digital-rain animation that fills the terminal width; pressing Ctrl+C exits cleanly and restores the cursor and terminal state with no visual artifacts
  3. User runs `box ascii ./photo.jpg` and sees an ASCII art rendering fitted to the current terminal width; PNG and JPEG inputs both work
  4. User pipes invalid JSON to `box json` and gets exit code 1 with a line/column error on stderr; valid JSON pretty-prints with syntax coloring; `--compact` minifies

**Plans**: TBD
**UI hint**: yes

### Phase 5: Windows Platform Integration

**Goal**: Users can render QR codes in the terminal, read/write the Windows clipboard, run a Pomodoro timer with toast notifications, and fetch live weather — all working correctly in PowerShell 7
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: QR-01, CLIP-01, POMO-01, WTHR-01
**Success Criteria** (what must be TRUE):

  1. User runs `box qr "https://example.com"` and sees a scannable QR code rendered with Unicode half-block characters that a phone camera can read from the terminal
  2. User pipes text to `box clip` and it lands on the Windows clipboard with correct Unicode; `box clip --paste` reads the clipboard to stdout; the tool works without elevated permissions
  3. User runs `box pomodoro` and sees a live countdown in the terminal; when the timer completes a Windows 11 toast notification appears; Ctrl+C cancels cleanly; `--break` and `--long-break` modes work
  4. User runs `box weather "London"` and sees current temperature, conditions, wind, and humidity from Open-Meteo with no API key; `--units imperial` switches to Fahrenheit; a graceful error appears when offline

**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation + Flatten | 4/4 | Complete    | 2026-06-22 |
| 2. Pure Transform Utilities | 5/5 | Complete   | 2026-06-22 |
| 3. Filesystem Power Tools | 0/? | Not started | - |
| 4. Terminal Visuals | 0/? | Not started | - |
| 5. Windows Platform Integration | 0/? | Not started | - |

## Coverage

**Total v1 requirements:** 34
**Mapped to phases:** 34
**Unmapped:** 0

| Phase | Requirements |
|-------|-------------|
| Phase 1 | FOUND-01, FOUND-02, FOUND-03, FOUND-04, FOUND-05, FOUND-06, FOUND-07, FOUND-08, FLAT-01, FLAT-02, FLAT-03, FLAT-04 |
| Phase 2 | UUID-01, B64-01, EPOC-01, COLR-01, PASS-01, COW-01, FORT-01, 8BAL-01, ROST-01 |
| Phase 3 | HASH-01, TREE-01, DU-01, DUPE-01, RENM-01 |
| Phase 4 | LOL-01, MTRX-01, ASCI-01, JSON-01 |
| Phase 5 | QR-01, CLIP-01, POMO-01, WTHR-01 |

---
*Roadmap created: 2026-06-22*
*Last updated: 2026-06-22 — Phase 1 COMPLETE: human-verify gate cleared, verification passed (5/5), flatten review findings CR-01/WR-01/WR-02 fixed*
*Last updated: 2026-06-22 — Phase 2 PLANNED: 5 plans across 5 waves (uuid/base64/epoch/color/passgen/cowsay/fortune/8ball/roast on a shared core::input foundation)*
