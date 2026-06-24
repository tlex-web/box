# Requirements: box — v2.0 Toolbox → Toolkit

**Defined:** 2026-06-25
**Core Value:** The toolbox must be globally available and instantly usable from PowerShell 7 — type `box <command>` from anywhere and the tool just works.
**Milestone goal:** Make the existing 23 commands deeper, scriptable, and frictionless in PS7 — turn the toolbox into a toolkit. No new *tool* commands; only `completions` / `config` meta-commands.

> Research basis: `.planning/research/SUMMARY.md` (HIGH confidence). The v1 architecture is settled; v2 grafts a cross-cutting `--json`/`--clip`/config spine onto it, then adds per-command depth, ordered by integration risk (v1 retrospective lesson #1).

---

## Milestone Decisions

Decisions locked at requirements time (carried into roadmap + plan-phase):

| # | Decision | Choice | Notes |
|---|----------|--------|-------|
| D-1 | Config implementation | **Hand-roll `toml` 1.1.2 + `dirs` 6.0.0** | `#[serde(default)]` `Option<T>` Config struct; `fs::read_to_string` + `NotFound → default`; manual `.or().or().unwrap_or()` per field. Matches v1's hand-roll ethos (humansize/colored_json/artem all rejected). |
| D-2 | `windows 0.61` Win32 dep | **GO — both features** | `du --on-disk` (`GetCompressedFileSizeW`) + `pomodoro --sound` (`MessageBeep`). Pin `0.61` to unify with the transitive `windows ^0.61` from `tauri-winrt-notification 0.7.2`. |
| D-3 | Large-number JSON policy | **Bare `u64`** | PS7 (`Int64`/`BigInteger`) handles it; document the JS/`>2^53` cross-consumer caveat. One rule for the whole spine. |
| D-4 | Non-UTF-8 path policy in `--json` | **`to_string_lossy()` + documented** | Never `to_str().unwrap()` (panics on non-UTF-8 NTFS names). |
| D-5 | `dupes --delete` | **GO — safe, non-interactive** | keep-first + dry-run-default + `--force` + hardlink-aware + abort-all-before-any pre-flight + snapshot-the-tree tests + mandatory adversarial code review. |
| D-6 | BLAKE3-default timing | **Co-ship config in Phase 6** | So `hash.default_algo = "sha256"` escape hatch exists when the breaking default flips. |
| D-7 | `completions` timing | **Phase 9 (last)** | Generated from the final `Cli` arg surface — must include all Phase-8 flags. |

**New dependencies committed:** `clap_complete 4.6.5`, `toml 1.1.2`, `dirs 6.0.0`, `indicatif 0.18.4`, `chrono-tz` (epoch `--tz`), `windows 0.61` (Win32 features), `uuid` `v7` feature flag. **No new crate** for `--json` (serde/serde_json), `--clip` (arboard), `tree --gitignore` (ignore), `lolcat`/`matrix` animation (crossterm).

---

## v2.0 Requirements

Each maps to exactly one roadmap phase. REQ-IDs continue the per-command mnemonics from v1 with a `-V2` suffix; cross-cutting work uses `SPINE-`, meta-commands use `CFG-`/`CMP-`.

### Scriptable Output Spine (cross-cutting)

- [ ] **SPINE-01**: `box <cmd> --json` emits exactly one well-formed JSON document on stdout — array for multi-item commands, object for scalar commands, recursive object for `tree`; `snake_case` fields; pretty-printed; UTF-8 with **no BOM**; **no ANSI, no progress, no human chrome** on stdout — established and contract-tested on the pilot commands (`uuid`, `hash`).
- [ ] **SPINE-02**: `--json` is available on every applicable value-producing command, all following the SPINE-01 house style: `base64`, `epoch`, `color`, `passgen`, `8ball`, `fortune`, `roast`, `cowsay`, `du`, `tree`, `dupes`, `flatten`, `bulk-rename`, `json`, `qr`, `weather`.
- [ ] **SPINE-03**: `box <cmd> --clip` copies the primary result to the Windows clipboard (copy **and** print; "Copied to clipboard" confirmation to stderr, suppressed when not a TTY; raw text, or the JSON document under `--json --clip`) — established on the pilot commands.
- [ ] **SPINE-04**: `--clip` is available on every single-textual-result command: `passgen`, `uuid`, `color`, `hash`, `base64`, `epoch`, `json`, `qr` (text payload).
- [ ] **SPINE-05**: Config-file defaults resolve with strict precedence **CLI flag > env var > config file > built-in default**; a missing or malformed config file falls back to built-in defaults without erroring a normal command (hand-rolled `toml` + `dirs`, `%APPDATA%\box\config.toml`).

### Meta-commands

- [ ] **CFG-01**: `box config show | get <key> | set <key> <value> | path` reads, edits, and locates the config-file defaults (`hash.default_algo`, `weather.units`, `weather.location`, `color`, …), with `--json` support.
- [ ] **CMP-01**: `box completions powershell` emits a static PS7 completion script (tab-complete subcommands, flags, and value hints), generated from the live `Cli`.

### Hashing depth

- [ ] **HASH-V2-01**: BLAKE3 is the default `hash` algorithm (**breaking change**; `--algo sha256` and the `hash.default_algo` config key preserve old behavior; the `--verify` 64-hex tie now maps to BLAKE3 with a transitional mismatch hint).
- [ ] **HASH-V2-02**: `hash` accepts multiple file arguments and prints coreutils `digest␣␣filename` (double-space) per line, with a stderr progress indicator for large inputs.

### Filesystem depth

- [ ] **FLAT-V2-01**: `flatten` gains `--extensions <list>`, `--separator <str>`, `--include-hidden`, and a stderr progress bar.
- [ ] **FLAT-V2-02**: `flatten --move` relocates files instead of copying — copy → verify (dest exists + size matches) → delete source; dry-run default, `--force` to execute.
- [ ] **DUPE-V2-01**: `dupes` uses multi-stage hashing (size → partial → full BLAKE3) and is hardlink-aware (paths sharing one file-index are collapsed, not counted as wasted space).
- [ ] **DUPE-V2-02**: `dupes --delete` removes duplicates safely — keep at least one per group, non-interactive, dry-run default, `--force` to execute, hardlink-safe, abort-all-before-any pre-flight.
- [ ] **RENM-V2-01**: `bulk-rename` gains case transforms (upper / lower / title) and sequential numbering (`{n}` token with zero-padding).
- [ ] **RENM-V2-02**: `bulk-rename --backup` writes pre-rename backups before executing a rename plan.
- [ ] **TREE-V2-01**: `tree` gains `.gitignore` respect, `--dirs-only`, `--ignore <glob>`, and sort-by-size.
- [ ] **DU-V2-01**: `du` gains an in-line percentage column and color-coded size ranges, plus `--exclude <glob>` (apparent size remains the default).
- [ ] **DU-V2-02**: `du --on-disk` reports allocated/compressed on-disk size via Win32 `GetCompressedFileSizeW` (correct for sparse/compressed NTFS files).

### Dev-transform depth

- [ ] **UUID-V2-01**: `uuid` gains v7 time-ordered output (`--v7`) and format flags (`--upper`, `--no-hyphens`, `--braces`, `--urn`).
- [ ] **EPOC-V2-01**: `epoch` gains relative time ("3 hours ago") and timezone output (`--tz <zone>`, via `chrono-tz`).
- [ ] **COLR-V2-01**: `color` gains CSS named-color lookup (both directions) and HSL input.
- [ ] **JSON-V2-01**: `json --sort-keys` sorts object keys (opt-in; `preserve_order` remains the default — never sorted implicitly).
- [ ] **PASS-V2-01**: `passgen` gains an entropy-bits estimate, `--no-similar` (drops `il1Lo0O`), and `--separator` for passphrases.

### Visual depth

- [ ] **LOL-V2-01**: `lolcat` gains `--animate`, `--freq`, and `--seed` (animation TTY-gated — static render when piped or under `--json`; RAII terminal restore; single-flush-per-frame; Press-only quit filter).
- [ ] **MTRX-V2-01**: `matrix` gains `--color`, `--speed`, and `--charset`.
- [ ] **QR-V2-01**: `qr` gains `--save <file>` (PNG / SVG) and `--error-correction L|M|Q|H`.
- [ ] **ASCI-V2-01**: `ascii` gains truecolor output, `--braille` (2×4 dot density), and `--invert`.

### Fun & system depth

- [ ] **COW-V2-01**: `cowsay` gains multiple figures and think-mode.
- [ ] **FORT-V2-01**: `fortune` gains selectable categories.
- [ ] **8BAL-V2-01**: `8ball` gains ASCII-art presentation and sentiment color.
- [ ] **ROST-V2-01**: `roast` gains `--language`.
- [ ] **POMO-V2-01**: `pomodoro` gains a session counter, auto-break cycling, and `--label`.
- [ ] **POMO-V2-02**: `pomodoro --sound` plays a completion beep via Win32 `MessageBeep` (zero-weight, no audio stack).
- [ ] **WTHR-V2-01**: `weather` gains `--forecast` (multi-day), a response cache, and a stored default location (reads the config default).

---

## Future Requirements (deferred beyond v2)

Acknowledged, not in the v2 roadmap.

- **VIS-V3**: `lolcat`/`matrix` advanced animation modes; `ascii` sixel/kitty-graphics output.
- **DUPE-V3**: interactive review UI for `dupes --delete`; cross-volume dedupe to hardlinks.
- **DIST-V3**: `box self-update`; Scoop / winget packaging (see Out of Scope — reconsider only if install friction becomes a real problem).
- **XPLAT-V3**: macOS / Linux portability (would require non-Windows backends for clipboard, toast, on-disk-size, sound).

---

## Out of Scope (v2)

Explicitly excluded. Documented to prevent scope creep and re-litigation.

| Feature | Reason |
|---------|--------|
| `box self-update` | Network + replacing a running `.exe` on Windows (file-lock hazard) = high integration risk; deferred to keep v2 focused on the toolkit spine |
| Scoop / winget packaging | `install.ps1` remains the chosen install path; packaging adds release overhead without changing the toolkit experience |
| Cross-platform (macOS / Linux) | Target remains Windows PowerShell 7; `--on-disk`, `--sound`, clipboard, and toast are Windows-specific |
| NDJSON / JSON-Lines output | PS7 `ConvertFrom-Json` cannot consume it inline; one buffered JSON document is the house style |
| Per-command bespoke JSON schemas | Inconsistency defeats the scriptable spine — all commands share the SPINE-01 house style |
| Interactive prompts in destructive commands | Breaks scriptability and the 0/1/2 exit contract; `dupes --delete` / `flatten --move` are non-interactive (`--force`) |
| Config interactive wizard / TUI | `box config` is flag-driven (show/get/set/path); a wizard is unscriptable |
| `json --sort-keys` as the default | Breaks the v1 `preserve_order` contract; `--sort-keys` is opt-in |

---

## Traceability

Populated during roadmap creation (Step 10). Each requirement maps to exactly one phase. Recommended phase grouping from `SUMMARY.md` build order is noted as a hint; the roadmapper finalizes.

| Requirement | Phase | Status | (Recommended phase) |
|-------------|-------|--------|---------------------|
| SPINE-01 | — | Pending | 6 (spine pilot: uuid+hash) |
| SPINE-03 | — | Pending | 6 (spine pilot) |
| SPINE-05 | — | Pending | 6 (config resolver) |
| HASH-V2-01 | — | Pending | 6 (BLAKE3 + config co-ship) |
| SPINE-02 | — | Pending | 7 (--json rollout) |
| SPINE-04 | — | Pending | 7 (--clip rollout) |
| HASH-V2-02 | — | Pending | 8 |
| FLAT-V2-01 | — | Pending | 8 |
| FLAT-V2-02 | — | Pending | 8 |
| DUPE-V2-01 | — | Pending | 8 |
| DUPE-V2-02 | — | Pending | 8 |
| RENM-V2-01 | — | Pending | 8 |
| RENM-V2-02 | — | Pending | 8 |
| TREE-V2-01 | — | Pending | 8 |
| DU-V2-01 | — | Pending | 8 |
| DU-V2-02 | — | Pending | 8 |
| UUID-V2-01 | — | Pending | 8 |
| EPOC-V2-01 | — | Pending | 8 |
| COLR-V2-01 | — | Pending | 8 |
| JSON-V2-01 | — | Pending | 8 |
| PASS-V2-01 | — | Pending | 8 |
| LOL-V2-01 | — | Pending | 8 (human-verify) |
| MTRX-V2-01 | — | Pending | 8 |
| QR-V2-01 | — | Pending | 8 |
| ASCI-V2-01 | — | Pending | 8 |
| COW-V2-01 | — | Pending | 8 |
| FORT-V2-01 | — | Pending | 8 |
| 8BAL-V2-01 | — | Pending | 8 |
| ROST-V2-01 | — | Pending | 8 |
| POMO-V2-01 | — | Pending | 8 |
| POMO-V2-02 | — | Pending | 8 |
| WTHR-V2-01 | — | Pending | 8 |
| CFG-01 | — | Pending | 9 (meta-command) |
| CMP-01 | — | Pending | 9 (meta-command) |

**Coverage:**
- v2.0 requirements: **34** total
- Mapped to phases: 0 (roadmap pending) ⚠️
- Unmapped: 34 (roadmapper will map all)

---
*Requirements defined: 2026-06-25 (v2.0 milestone kickoff)*
*Last updated: 2026-06-25 after initial definition*
