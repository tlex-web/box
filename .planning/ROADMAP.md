# Roadmap: box — Rust CLI Toolbox

**Project:** box
**Milestone:** v1 — Full 23-command toolbox
**Created:** 2026-06-22
**Granularity:** Standard (5 phases)
**Mode:** mvp

## Phases

- [x] **Phase 1: Foundation + Flatten** — Binary scaffold, install script, core infrastructure, and the anchor `flatten` command. Shipped 2026-06-22: install.ps1 + same-session flatten human-verified in PS7; verification passed (5/5); code-review silent-data-loss findings (CR-01/WR-01/WR-02) fixed
- [x] **Phase 2: Pure Transform Utilities** — Nine zero-external-dependency commands proving the RunCommand pattern: uuid, base64, epoch, color, passgen, cowsay, fortune, 8ball, roast (completed 2026-06-22)
- [x] **Phase 3: Filesystem Power Tools** — Five commands sharing walkdir: hash, tree, du, dupes, bulk-rename (completed 2026-06-22)
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

**Plans**: 5 plans
Plans:
**Wave 1**

- [x] 03-01-PLAN.md — `hash`: streaming enum-dispatch Hasher (SHA-256 default; blake3/sha512/md5 via `--algo`, D-02/D-03) + `--verify` length auto-detect with 0/1/2 exit codes (D-04) + the deferred `--file PATH` input layer (D-05) + typed exit-2 error variant (HASH-01) ✓ (1/2 tasks, TDD; 7/7 HASH-01 tests green)

**Wave 2** *(blocked on Wave 1 — shares cli.rs/main.rs registry)*

- [x] 03-02-PLAN.md — `tree`: dir-first Unicode box-drawing render + colored dir names + `--sizes`/`--depth` + `N directories, M files` summary (D-08/09/10); promotes flatten's `human_size` into `core::output` for shared use (D-12) (TREE-01) ✓ (2/2 tasks; 3/3 TREE-01 tests + tree.trycmd green; human_size promoted, flatten unbroken)

**Wave 3** *(blocked on Wave 2 — shares cli.rs/main.rs registry)*

- [x] 03-03-PLAN.md — `du`: per-immediate-child recursive totals, biggest-first deterministic sort, `--top`/`--depth`, full-scan total summary, trailing-`/` dir marker (D-11/D-12); reuses the promoted `human_size` (DU-01) ✓ (2/2 tasks, TDD-style; 3/3 DU-01 tests + 4 unit tests green; reuses human_size + is_hidden read-only walker)

**Wave 4** *(blocked on Wave 3 — shares cli.rs/main.rs registry; reuses 03-01 BLAKE3 infra)*

- [x] 03-04-PLAN.md — `dupes`: size pre-filter then rayon-parallel BLAKE3 content hash, deterministic sorted groups + wasted-space summary, strictly read-only (D-13, D-06/D-07) (DUPE-01) ✓ (2/2 tasks, TDD-style; 4/4 DUPE-01 tests + 6 unit tests green; reuses the 03-01 BLAKE3 update_reader path + human_size + is_hidden read-only walker; no write/rename/delete path)

**Wave 5** *(blocked on Wave 4 — shares cli.rs/main.rs registry)*

- [x] 03-05-PLAN.md — `bulk-rename`: regex plan (first-match `replace`, full-base-name, D-16/D-17) + ABORT-ALL pre-flight collision/cycle/path-separator detection (the only backstop vs `std::fs::rename`'s silent overwrite, D-18) + dry-run-default/`--force` execute (D-19) (RENM-01) ✓ (2/2 tasks, TDD-style; 7/7 RENM-01 tests + 9 unit tests green; pure I/O-free `preflight()` detector for all four D-18 rules; reuses flatten's `format_row`/`arrow_col` + case-folded occupied set + `encode_no_separator` invariant; every abort path snapshot-asserts the tree unchanged; LAST Phase-3 stub gone — phase feature-complete)

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

**Plans**: 4 plans
**UI hint**: yes
Plans:
**Wave 1**

- [x] 04-01-PLAN.md — `json`: serde_json (preserve_order) parse/validate (1-based line+col→exit 1) + 2-space pretty + hand-rolled is_color_on()-gated colorizer over Value + `--compact` minify (D-04/05/06) (JSON-01) ✓ (2 tasks, TDD RED→GREEN; 5/5 JSON-01 integration + 4 colorize unit tests + json.trycmd green; serde_json 1.0.150 preserve_order added, arbitrary_precision OFF; full suite 102 unit + all integration + clippy -D + fmt clean; json stub gone — 3 phase-4 stubs remain: lolcat/matrix/ascii)

**Wave 2** *(blocked on Wave 1 — shares cli.rs/main.rs/mod.rs registry)*

- [x] 04-02-PLAN.md — `lolcat`: classic sine-wave RGB gradient (per-char, width-aware, per-line diagonal) + unconditional strip-ansi-escapes + is_color_on()-gated truecolor (byte-identical minus ANSI when piped); actions the standing strip-ansi todo (D-11/12/13/14) (LOL-01) ✓ (2 tasks, TDD RED→GREEN; pure rgb_at — freq 0.1, 120°/240° offsets, floor 128, ·127+128 maps [-1,1]→[1,255] so as-u8 never wraps; per-Unicode-scalar emit, phase advances by UnicodeWidthChar::width, whitespace uncolored but phase-advancing, newlines raw; strip_str unconditional before recolor, D-13/T-04L-01; 4 unit + 3/3 LOL-01 integration green; unicode-width 0.2.2 + strip-ansi-escapes 0.2.1 added — strip-ansi todo actioned; one Rule-3 fix print!("\n")→println!() for clippy; full suite 106 unit + all integration + clippy -D + fmt clean; lolcat stub gone — 2 phase-4 stubs remain: matrix/ascii)

**Wave 3** *(blocked on Wave 2 — shares cli.rs/main.rs/mod.rs registry)*

- [ ] 04-03-PLAN.md — `ascii`: hand-rolled on image 0.25.10 (artem rejected) — image::open → resize_exact(Triangle) → to_luma8 → dark→light ramp; cols=terminal_width(), rows aspect-corrected /2; monochrome v1; tiny checked-in PNG/JPEG fixtures (D-01/02/03) (ASCI-01)

**Wave 4** *(blocked on Wave 3 — shares cli.rs/main.rs/mod.rs registry; has human-verify checkpoint)*

- [ ] 04-04-PLAN.md — `matrix`: full-terminal halfwidth-katakana (U+FF66–FF9D) green rain on crossterm 0.29; single-flush-per-frame @~20 FPS (poll=timer); RAII Drop guard restore; exit Ctrl+C/q/Esc with KeyEventKind::Press filter; pure drop/fade+glyph+quit units + smoke test + human-verify animation (D-07/08/09/10) (MTRX-01)

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
| 3. Filesystem Power Tools | 5/5 | Complete    | 2026-06-22 |
| 4. Terminal Visuals | 2/4 | In Progress|  |
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
*Last updated: 2026-06-22 — Phase 3 PLANNED: 5 plans across 5 waves (hash → tree → du → dupes → bulk-rename), one vertical slice per command; shared-core changes folded into first consumer (--file→hash, human_size→tree); all 19 CONTEXT decisions covered*
*Last updated: 2026-06-22 — Phase 3 Plan 03-01 (hash) COMPLETE: live `box hash` (SHA-256 default, --algo blake3/sha512/md5, --verify 0/1/2 exit contract); streaming enum-dispatch Hasher + core::input --file layer shipped; HASH-01 satisfied; 7/7 HASH-01 tests + full suite green*
*Last updated: 2026-06-22 — Phase 3 Plan 03-02 (tree) COMPLETE: live `box tree` (dir-first Unicode box-drawing render, is_color_on-gated blue dir names, --sizes/--depth, `N directories, M files` summary); flatten's `human_size` promoted into `core::output` (shared, D-12) with flatten left unbroken; TREE-01 satisfied; 3/3 TREE-01 tests + tree.trycmd + full suite (77 unit + all integration) green*
*Last updated: 2026-06-22 — Phase 3 Plan 03-04 (dupes) COMPLETE: live `box dupes` (size pre-filter HashMap<u64,Vec<PathBuf>> → rayon par_iter BLAKE3 content hash reusing the 03-01 update_reader path, D-13 → sort (hash,path) before grouping for determinism, RESEARCH Pitfall 6 → groups ≥2 + wasted-space summary via human_size); strictly read-only, NO write/rename/delete path (T-03-13), reuses is_hidden + follow_links(false), no noise list / no ignore crate (D-06/D-07); DUPE-01 satisfied; 4/4 DUPE-01 tests + 6 unit tests + full suite (87 unit + all integration) + clippy -D warnings + fmt --check green; dupes stub gone (1 phase-3 stub remains: bulk-rename)*
*Last updated: 2026-06-24 — Phase 4 PLANNED: 4 plans across 4 waves (json → lolcat → ascii → matrix), one vertical MVP slice per command; the four commands are independent but share the cli.rs/main.rs/commands/mod.rs registry so they sequence by wave (zero same-wave file overlap); all 15 CONTEXT decisions D-00..D-14 covered; matrix carries the only human-verify checkpoint*
*Last updated: 2026-06-22 — Phase 3 Plan 03-05 (bulk-rename) COMPLETE → PHASE 3 FEATURE-COMPLETE (5/5 plans): live `box bulk-rename` (regex first-match `replace` over the FULL base name, D-16/D-17 → in-memory ABORT-ALL-BEFORE-ANY-RENAME pre-flight detecting collisions/cycles/path-separator injection, the ONLY backstop vs std::fs::rename's silent overwrite, D-18 → dry-run preview is the DEFAULT, --force executes, D-19); the pre-flight is a PURE I/O-free preflight()->Vec<Conflict> unit-tested for every rule; reuses flatten's format_row/arrow_col + case-folded occupied set + encode_no_separator invariant VERBATIM; every abort path snapshot-asserts the directory byte-for-byte unchanged; RENM-01 satisfied; 7/7 RENM-01 tests + 9 unit tests + full suite (96 unit + all integration) + clippy -D warnings + fmt --check green; ALL 5 Phase-3 not_implemented arms gone — phase ready for verification (8 stubs remain: Phase-4 lolcat/matrix/ascii/json + Phase-5 qr/clip/pomodoro/weather)*
*Last updated: 2026-06-24 — Phase 4 Plan 04-01 (json) COMPLETE: live `box json` (JSON-01) — serde_json::from_str::<Value> with preserve_order (input key order kept, arbitrary_precision OFF) → invalid `bail!`s with 1-based line/column (exit 1) → `--compact` minify (to_string) / plain 2-space pretty (to_string_pretty) / colored TTY via a pure hand-rolled colorize(&Value) walker gated SOLELY on is_color_on() so piped output is byte-identical minus ANSI (D-04/05/06); 6-variant walker + 4 unit tests; 5/5 JSON-01 integration + json.trycmd green; full suite 102 unit + all integration + clippy -D warnings + fmt --check clean; serde_json 1.0.150 transitively pulls dtolnay's `zmij` (verified-legitimate ryu-successor float crate); json stub gone (3 phase-4 stubs remain: lolcat/matrix/ascii)*
