# Requirements: box — Rust CLI Toolbox

**Defined:** 2026-06-22
**Core Value:** The toolbox must be globally available and instantly usable from PowerShell 7 — type `box <command>` from anywhere and the tool just works.

## v1 Requirements

Requirements for initial release. All 23 commands are in v1 scope. Each maps to roadmap phases.

### Foundation & Distribution

- [x] **FOUND-01**: User runs `box <command>` for any subcommand; `box --help` lists all commands with one-line descriptions and `box <cmd> --help` shows per-command usage
- [x] **FOUND-02**: `box --version` prints the toolbox semantic version
- [x] **FOUND-03**: All commands follow shared UX conventions — machine output to stdout, messages/errors to stderr, exit codes (0 success, 1 error, 2 bad args)
- [x] **FOUND-04**: Color/ANSI output auto-disables when stdout is not a TTY and when `NO_COLOR` (or `--no-color`) is set; ANSI is enabled correctly in PowerShell 7
- [x] **FOUND-05**: Commands fail with a clear stderr message instead of panicking on bad input or missing files
- [x] **FOUND-06**: Paths are normalized safely on Windows (no verbatim `\\?\` UNC leakage) and long/edge-case paths are handled without silent failure
- [x] **FOUND-07**: User installs globally by running `install.ps1`, which builds the release binary, copies it to a dedicated bin dir, and adds that dir to the user PATH idempotently
- [x] **FOUND-08**: `install.ps1` refreshes PATH in the current session and smoke-tests the install by running `box --help`

### flatten (anchor)

- [x] **FLAT-01**: User can recursively copy every file from a source folder tree into one flat output folder, leaving originals untouched and creating no subdirectories in the output
- [x] **FLAT-02**: On filename collision, files are renamed by encoding their source path (e.g. `docs_sub_report.txt`); deep-nesting double-collisions fall back to a numeric suffix
- [x] **FLAT-03**: `--dry-run` previews the planned copies/renames without writing anything; a completion summary reports files copied and collisions handled
- [x] **FLAT-04**: flatten refuses to loop when the output dir is inside the source dir, skips symlinks safely (no infinite loops), and preserves copied file timestamps

### Filesystem Tools

- [ ] **HASH-01**: User can hash a file (default SHA-256), choose an algorithm, hash stdin, and verify against a known hash (exit 0 match / 1 mismatch), output in `HASH  filename` format
- [ ] **TREE-01**: User can print a directory tree with box-drawing characters, optional file sizes, depth limit, colored dirs vs files, and a file/dir count summary
- [ ] **DU-01**: User can analyze disk usage as a size-sorted view (biggest first) with human-readable sizes, a depth limit, and top-N truncation
- [ ] **DUPE-01**: User can find duplicate files by content (size pre-filter then hash), displayed as groups of identical files with a summary of wasted space
- [ ] **RENM-01**: User can bulk-rename files with a regex pattern and capture-group replacement; dry-run preview is the default and `--force` is required to execute, with collision detection aborting before any rename

### Dev Utilities

- [x] **UUID-01**: User can generate one or more UUIDs (v4 default), lowercase by default, one per line
- [ ] **JSON-01**: User can pretty-print JSON from stdin or a file with color, validate it (exit 0 valid / 1 invalid with line/column on error), and minify with `--compact`
- [x] **B64-01**: User can base64-encode and `--decode` data from a file or stdin, no line-wrapping by default, with a URL-safe option
- [x] **EPOC-01**: User can convert a Unix timestamp to a human date (local + UTC) and a date string back to a timestamp; running with no args prints the current timestamp
- [x] **COLR-01**: User can convert colors between hex and RGB, see all representations at once, and view an in-terminal swatch of the color
- [ ] **CLIP-01**: User can pipe stdin to the clipboard and read the clipboard to stdout, with correct Unicode handling on Windows (bypassing clip.exe limitations)
- [x] **PASS-01**: User can generate cryptographically random passwords (length and character-set control) and passphrases (`--words N`), with `--count N` for bulk generation, printed to stdout only

### Terminal Visuals

- [ ] **QR-01**: User can render a scannable QR code for any text/URL in the terminal using Unicode half-block characters, accepting input as an argument or from stdin
- [ ] **LOL-01**: User can pipe text through `lolcat` to colorize it with a smooth truecolor rainbow gradient; color auto-disables when output is not a TTY
- [ ] **MTRX-01**: User can run a full-terminal Matrix digital-rain animation that exits cleanly on Ctrl+C and restores terminal state
- [ ] **ASCI-01**: User can convert an image file (PNG/JPEG) to ASCII art fitted to the terminal width

### Fun

- [x] **COW-01**: User can render an ASCII speech bubble (cowsay) for text given as an argument or piped via stdin, with word-wrapping
- [x] **FORT-01**: User can print a random fortune/quote, different each run, sized to fit the terminal
- [x] **8BAL-01**: User can ask a yes/no question and get a classic random Magic 8-Ball response (question optional)
- [x] **ROST-01**: User can print a random programmer roast, different each run

### Whimsy with Utility

- [ ] **POMO-01**: User can run a Pomodoro work timer with an in-place countdown, break/long-break modes, clean Ctrl+C cancel, and a Windows toast notification on completion
- [ ] **WTHR-01**: User can fetch current weather for a city name or lat/lon (temperature, conditions, wind, humidity) via a keyless API, with metric/imperial units and a graceful offline error

## v2 Requirements

Deferred to future release. Per-command "differentiator" features surfaced in research, tracked but not in the current roadmap.

### Command Enhancements

- **FLAT-V2-01**: `--separator`, `--include-hidden`, `--extensions` filter, `--json` mapping output, progress bar, `--move` mode
- **HASH-V2-01**: BLAKE3, multi-file hashing, progress bar, `--json` output
- **DUPE-V2-01**: Multi-stage prefix/suffix/full hashing, `--json`, `--delete`, hardlink awareness
- **RENM-V2-01**: Case transforms, sequential numbering, `--backup`, `--json` plan output
- **TREE-V2-01**: `.gitignore` respect, hidden/dirs-only/ignore-pattern flags, `--json`, sort-by-size
- **DU-V2-01**: Percentage bars, color-coded size ranges, exclude patterns, `--json`, apparent-size
- **DEV-V2-01**: `--clip` integration across passgen/uuid/color; UUID v7 + format flags; epoch relative/timezone; JSON `--sort-keys`; named CSS colors + HSL
- **VIS-V2-01**: lolcat `--animate`/`--freq`/`--seed`; matrix color/speed/charset options; QR `--save`/error-correction; ascii color/braille/invert modes
- **FUN-V2-01**: cowsay figures/think mode; fortune categories; 8ball ASCII art + sentiment color; roast `--language`
- **SYS-V2-01**: pomodoro session counter/auto-break/sound/`--label`; weather `--forecast`/`--json`/cache/stored location
- **PASS-V2-01**: `passgen` entropy estimate, `--no-similar`, `--separator`, `--clip` integration

## Out of Scope

Explicitly excluded. Documented to prevent scope creep. (Anti-features from research.)

| Feature | Reason |
|---------|--------|
| Cross-platform builds (macOS/Linux) | Target is Windows PowerShell 7 |
| Scoop/winget packaging | `install.ps1` is the v1 distribution path |
| `flatten` move/overwrite-silently modes | Data-loss risk; copy + collision-rename only |
| `dupes` auto-delete / interactive selection | Catastrophic if wrong; print groups, user decides |
| `bulk-rename` execute-by-default / undo log | Dry-run-first + `--force` is the safety model |
| `qr` decode/scan mode | Different tool; doubles complexity |
| `json` full jq query language | jq already exists; pretty-print + validate only |
| `du`/`dupes` interactive TUI | Scope creep; one-shot commands only |
| AI/network-backed roast/fortune/8ball | Network dependency for a fun toy; curated local lists |
| `ascii` video/GIF/URL input | Complex; file input, still images only |
| `weather` API key requirement, radar/maps | Keyless Open-Meteo; simple data display only |
| `clip` binary/image clipboard | Text only in v1 |
| Logging/data mixed on stdout | Breaks pipes; data→stdout, messages→stderr |

## Traceability

Which phases cover which requirements. Populated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| FOUND-01 | Phase 1 | Complete |
| FOUND-02 | Phase 1 | Complete |
| FOUND-03 | Phase 1 | Complete |
| FOUND-04 | Phase 1 | Complete |
| FOUND-05 | Phase 1 | Complete |
| FOUND-06 | Phase 1 | Complete |
| FOUND-07 | Phase 1 | Complete |
| FOUND-08 | Phase 1 | Complete |
| FLAT-01 | Phase 1 | Complete |
| FLAT-02 | Phase 1 | Complete |
| FLAT-03 | Phase 1 | Complete |
| FLAT-04 | Phase 1 | Complete |
| UUID-01 | Phase 2 | Complete |
| B64-01 | Phase 2 | Complete |
| EPOC-01 | Phase 2 | Complete |
| COLR-01 | Phase 2 | Complete |
| PASS-01 | Phase 2 | Complete |
| COW-01 | Phase 2 | Complete |
| FORT-01 | Phase 2 | Complete |
| 8BAL-01 | Phase 2 | Complete |
| ROST-01 | Phase 2 | Complete |
| HASH-01 | Phase 3 | Pending |
| TREE-01 | Phase 3 | Pending |
| DU-01 | Phase 3 | Pending |
| DUPE-01 | Phase 3 | Pending |
| RENM-01 | Phase 3 | Pending |
| LOL-01 | Phase 4 | Pending |
| MTRX-01 | Phase 4 | Pending |
| ASCI-01 | Phase 4 | Pending |
| JSON-01 | Phase 4 | Pending |
| QR-01 | Phase 5 | Pending |
| CLIP-01 | Phase 5 | Pending |
| POMO-01 | Phase 5 | Pending |
| WTHR-01 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 34 total
- Mapped to phases: 34
- Unmapped: 0

---
*Requirements defined: 2026-06-22*
*Last updated: 2026-06-22 after roadmap creation — traceability table populated*
