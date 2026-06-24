# Research Summary - box v2.0 Toolbox to Toolkit

**Project:** box - single-binary Rust CLI toolbox (Windows PowerShell 7)
**Domain:** Scriptable developer CLI - adding `--json`/`--clip` spine, config-file defaults, PS7 completions, and per-command depth to a shipped 23-command binary
**Researched:** 2026-06-24
**Confidence:** HIGH

---

## Executive Summary

box v2.0 is a *deepening*, not a rewrite. The v1 architecture (single crate, `src/commands/<cmd>/mod.rs`, `RunCommand` trait, `is_color_on()`-gated styling, strict 0/1/2 exit codes) is settled and correct. v2 grafts three cross-cutting capabilities (`--json`, `--clip`, config-file defaults) and two meta-commands (`completions`, `config`) onto that foundation, then adds focused depth flags to the existing 23 commands. The stack change is deliberately crate-light: `--json` and `--clip` need zero new crates (serde/serde_json/arboard are already present). Only `clap_complete`, a config solution (Decision 1), `indicatif`, and optionally `windows 0.61` are genuinely new.

The single most important architectural decision is the `--json` house style. All four research files agree: stdout under `--json` must carry exactly one JSON document (array for multi-item commands, object for scalars, recursive object for `tree`) with no ANSI, no progress chrome, and a single trailing newline. NDJSON is an explicit anti-feature - PowerShell 7 `ConvertFrom-Json` cannot consume it inline. Every cross-cutting flag follows the proven `COLOR_ON` atomic pattern: declared once as `global = true` on `Cli`, initialized once in `main`, consulted via `is_*_on()` - no per-command field, no `RunCommand` signature change.

The primary risks are all about discipline, not technology. The `--json` STDOUT contamination pitfall (stray progress bytes breaking `ConvertFrom-Json`) is the number-one failure mode and must be addressed structurally in the shared spine before any per-command work. Destructive flags (`dupes --delete`, `flatten --move`, `bulk-rename --backup`) must inherit v1 exact dry-run-default + abort-all-before-any + snapshot-the-tree-unchanged ritual. The BLAKE3-default change to `hash` is the only user-visible breaking change and needs loud documentation plus a config-file escape hatch.

---

## Stack Additions

Authoritative consolidated dependency table. All four reports are consistent; tensions are reconciled here.

| Dependency | Version | Tag | For |
|------------|---------|-----|-----|
| `clap_complete` | `4.6.5` | [NEW CRATE] | `box completions powershell` - static `Register-ArgumentCompleter` script |
| `config` | `0.15.24` | [NEW CRATE - see Decision 1] | Layered flag>env>file>default merge |
| `dirs` | `6.0.0` | [NEW CRATE] | `%APPDATA%` resolution for config file location (needed under both config approaches) |
| `indicatif` | `0.18.4` | [NEW CRATE - CONDITIONAL] | Progress bars/spinners for flatten/hash/dupes; named in CLAUDE.md but never pulled in v1 |
| `windows` | `0.61` | [NEW CRATE - CONDITIONAL, see Decision 2] | `MessageBeep` (pomodoro sound) + `GetCompressedFileSizeW` (du on-disk size) only |
| `uuid "v7"` | existing `1.23.3` | [FEATURE FLAG] | `box uuid --v7` (time-ordered UUIDs via `Uuid::now_v7()`) |
| `serde` (already `derive`) | existing | [REUSE-EXISTING] | `#[derive(Serialize)]` on per-command output structs - zero change |
| `serde_json` (already `preserve_order`) | existing | [REUSE-EXISTING] | `--json` serialization + `json --sort-keys` (no feature change) |
| `arboard` | existing `3.6.1` | [REUSE-EXISTING] | `--clip` on every applicable command - proven v1 infra |
| `ignore` | existing `0.4` | [REUSE-EXISTING] | `tree --gitignore` via `ignore::WalkBuilder` |
| `crossterm` | existing `0.29` | [REUSE-EXISTING] | `lolcat --animate`, matrix color/speed/charset |
| `rayon` / `blake3` / `walkdir` | existing | [REUSE-EXISTING] | dupes multi-stage hashing, hash multi-file, flatten progress |
| `toml` | `1.1.2` | [NEW CRATE - hand-roll path only, see Decision 1] | Config file parsing if `config` crate is rejected |

**What NOT to use:**

| Avoid | Why |
|-------|-----|
| `filesize 0.2.0` | Unmaintained since 2020; wraps the same Win32 call as `windows` crate |
| `rodio` / `cpal` / `rusty_audio` | Full audio stack for a single beep - use `MessageBeep` instead |
| `windows 0.62.2` as a direct dep | Duplicates `windows ^0.61` pulled by `tauri-winrt-notification 0.7.2`; pin `0.61` to unify |
| `clap_complete unstable-dynamic` | PS7 native dynamic completion still unstable (clap #3918); use static `generate(Shell::PowerShell)` |
| `figment 0.10.19` | Viable but last release 2024-05-17 vs `config` 2026-06-16; no functional advantage here |
| Any second gitignore/animation/clipboard crate | `ignore`, `crossterm`, `arboard` already present and sufficient |

**Minimum v2 (neither du --on-disk nor pomodoro --sound):** only `clap_complete` + config solution + `indicatif` + `uuid "v7"` feature flag. No `windows` dep at all.

---

## Decision 1 - Config Approach (REQUIRES REQUIREMENTS CONFIRMATION)

**The tension:** STACK.md recommends the `config` crate (0.15.24). ARCHITECTURE.md recommends a hand-roll with bare `toml` + an `Option`-merge resolver.

**`config 0.15.24`** (STACK.md position): native `flag>env>file>default` precedence via ordered `add_source`, native `File::required(false)` (missing file = defaults, not error), native `Environment` source, TOML-only via `default-features=false, features=["toml"]`. Maintained by `epage` (the clap maintainer). 2 new crates: `config` + `dirs`.

**Hand-roll `toml` + `dirs`** (ARCHITECTURE.md position): a `#[serde(default)] Config` struct of `Option<T>` fields, `fs::read_to_string` + `io::ErrorKind::NotFound => Config::default()`, a manual `.or().or().unwrap_or()` merge per field. Reuses existing `serde derive`. 2 new crates: `toml` + `dirs`. More in-repo transparency; re-implements the solved merge and missing-file branch.

**Recommendation: `config 0.15.24` + `dirs 6.0.0`.**

Rationale: the precedence merge + optional-file + env-overlay is exactly what `config` exists to do, verified correct on docs.rs. The hand-roll re-implements a solved, tested problem for only ~4 keys. The `config` crate is actively maintained (2026-06-16), same ecosystem as clap. With `default-features=false, features=["toml"]` it trims non-TOML parsers, matching the project lean-bundle discipline.

The hand-roll is the right pick if the project lean-dep ethos overrides this, or if explicit in-repo precedence code is preferred for auditability. Both paths need `dirs 6.0.0` for `%APPDATA%` resolution.

**Requirements must confirm which path to take.** The ARCHITECTURE.md `Option<T>`-no-default + `.or().or().unwrap_or()` per-command resolver is correct and mandatory regardless of path.

Config keys: `hash.default_algo`, `weather.units`, `weather.location`, `color` (on/off/auto). Extensible: `uuid.default_version`, `passgen.length`.

---

## Decision 2 - Go/No-Go: `windows` Win32 Dependency (REQUIRES REQUIREMENTS CONFIRMATION)

Only two specific features pull in the Win32 `windows` crate:

- **`du --on-disk`** (allocated/compressed size): needs `GetCompressedFileSizeW` from `Win32::Storage::FileSystem`.
- **`pomodoro --sound`** (completion beep): needs `MessageBeep` from `Win32::System::Diagnostics::Debug`.

If both ship: `windows = { version = "0.61", features = ["Win32_Storage_FileSystem", "Win32_System_Diagnostics_Debug"] }`. Pin `0.61` (not `0.62`) to unify with `windows ^0.61` already pulled by `tauri-winrt-notification 0.7.2`.

**Requirements must make these an explicit go/no-go.** `du` defaults to apparent size (`metadata().len()`, std, free); the `windows` dep is only for `--on-disk`.

---

## Key Findings

### `--json` House Style (the v2 contract - all four reports agree)

- **Global `--json` flag on `Cli` with `global = true`** stored in `JSON_ON: AtomicBool` in `core::output`, consulted via `is_json_on()`. Mirrors the proven `no_color` to `COLOR_ON` pattern exactly. Zero new field on any command struct.
- **One `#[derive(Serialize)]` output struct per command.** Both human and JSON paths read from the same struct - they cannot drift.
- **Shape:** single top-level array for multi-item commands (`hash N files`, `du`, `dupes`, `flatten`, `bulk-rename`); single object for scalar commands (`color`, `epoch`, `passgen`, `weather`); recursive object for `tree`. Single-item invocations emit a 1-element array for shape stability.
- **NOT NDJSON.** PS7 `ConvertFrom-Json` cannot consume NDJSON inline. box results are bounded and buffered before printing - no streaming need exists.
- **Pretty by default** (`serde_json::to_string_pretty`). PS7 `ConvertFrom-Json` is whitespace-agnostic; pretty output is human-readable when eyeballed.
- **`snake_case` fields** everywhere. Rust/serde default, jq idiom, PS7 property access is case-insensitive.
- **`--json` forces `COLOR_ON = false`** via `init_output` - stray `.green()` calls cannot corrupt the document.

### `--clip` Behavior

- **Copy AND print.** Copy primary result to clipboard, still print to stdout. "Copied to clipboard" confirmation to stderr (suppressed when not a TTY).
- Copies raw result text, never ANSI. With `--json --clip`, copies the JSON.
- Reuses `arboard 3.6.1` via `core::output::out_line` + `flush_clip()` (main thread, after successful dispatch).
- **Applicable:** `passgen`, `uuid`, `color`, `hash`, `qr` (text payload only), `base64`, `epoch`, `json`.
- **Not applicable:** `matrix`, `pomodoro`, `lolcat`, `tree`/`du`/`dupes`/`flatten`/`bulk-rename`.

### Feature Table Stakes vs Differentiators

**Table stakes (must ship in v2):**
- `--json` on every applicable value-producing command with the unified house style
- `--clip` on single-textual-result commands
- `box completions powershell` - static script via `clap_complete`
- `box config` show/get/set/path with flag>env>config>default precedence
- BLAKE3 as the new `hash` default (breaking, documented, config-overridable)
- `hash` multi-file, coreutils double-space format (`digest  filename`)
- `tree` `.gitignore` respect, `--dirs-only`, `--ignore <glob>`
- `du` percentage column, `--exclude <glob>`, apparent-size default
- `passgen` `--no-similar` (il1Lo0O), `--separator` for passphrases
- `uuid` format flags (`--upper`, `--no-hyphens`, `--braces`, `--urn`)
- `epoch --json {epoch,utc,local,relative}`
- `bulk-rename` case transforms (upper/lower/title), sequential numbering token + padding
- `flatten` `--extensions`, `--separator`, progress bar

**Differentiators (v2, prioritize after spine):**
- `uuid --version 7` (time-ordered, B-tree-friendly) - one feature flag on existing dep
- `epoch` relative time ("3 hours ago"), timezone support (`--tz`, needs `chrono-tz` new dep)
- `du` color-coded size ranges, in-line percentage bar, `--on-disk` (go/no-go)
- `dupes` multi-stage hashing, hardlink awareness, optional `--delete` (go/no-go)
- `passgen` entropy bits estimate
- `color` CSS named-color lookup both directions, HSL input
- `lolcat --animate`, `--freq`, `--seed`
- `matrix` `--color`, `--speed`, `--charset`
- `qr --save PNG/SVG`, `--error-correction L|M|Q|H`
- `ascii` color (truecolor), `--braille`, `--invert`
- `cowsay` multiple figures, think-mode; `fortune` categories; `8ball` art/sentiment; `roast --language`
- `pomodoro` session counter, auto-break, `--label`, `--sound` (go/no-go)
- `weather --forecast`, response cache, stored default location (depends on config)

**Anti-features (do not build):**
- NDJSON / JSON Lines (breaks `ConvertFrom-Json`)
- Per-command bespoke JSON schemas (inconsistency defeats the spine)
- Interactive prompts in any destructive command (unscriptable, breaks 0/1/2 contract)
- `flatten --move` that deletes source before confirming copy
- Config wizard / interactive TUI
- `--clip` on animations or timer commands
- `json --sort-keys` as default (breaks `preserve_order` contract)

### Architecture Approach

v2 grafts three atomics (`JSON_ON`, `CLIP_ON`, plus existing `COLOR_ON`) and a config `OnceLock` onto `core::output` / `main.rs`. The `RunCommand::run(self)` signature is unchanged - no per-command field, no trait churn. Each applicable command gains one `#[derive(Serialize)]` output struct and an `if is_json_on() { emit_json(&result) } else { out_line(&human_render) }` fork over the same data source.

**Modified core files:** `src/core/output.rs` (atomics + emit_json + out_line + flush_clip), `src/core/config.rs` (NEW), `src/core/errors.rs` (BoxError::Config), `src/cli.rs` (global flags + 2 new Commands variants), `src/main.rs` (init_config + init_output + flush_clip), `Cargo.toml`.

**New command modules:** `src/commands/completions/mod.rs`, `src/commands/config/mod.rs`.

**Per command (incremental, Phases 6-8):** one output struct, `is_json_on()` fork, `out_line` for primary output, config-tier merge where applicable, depth flags.

---

## Watch Out For

**1. `--json` STDOUT contamination (the number-one failure mode)**
Under `--json`, stdout MUST contain exactly one thing: the JSON document, UTF-8, no BOM, terminated by a single newline. Progress bars, summaries, banners, ANSI escapes go to stderr or are suppressed. Construct `indicatif` bars with `ProgressDrawTarget::stderr()` explicitly; use `ProgressBar::hidden()` under `--json`. Per-command test: stdout first 3 bytes not `EF BB BF`, no `0x1B` byte, `serde_json::from_slice` succeeds on full stdout. This is the v2 analogue of v1 piped-no-ANSI test.

**2. `--json` correctness - BOM, large numbers, NaN/Infinity, non-UTF-8 paths, shape contract**
Write via `serde_json::to_writer(stdout_lock, &value)` + newline (never via a BOM-injecting path). File sizes (`u64`) and epoch nanos can exceed 2^53 (JS `Number.MAX_SAFE_INTEGER`) - decide one policy for the spine. Guard all float fields against NaN/Infinity (emit `null` for undefined). Standardize on `to_string_lossy()` for Windows paths with an explicit documented decision; never `to_str().unwrap()` in `--json` context (panics on non-UTF-8 NTFS names).

**3. Config precedence - explicit flag must always win**
Every config-overridable CLI flag must be `Option<T>` with NO `default_value`. Resolve: `cli_flag.or(env_var).or(config.field).unwrap_or(BUILTIN)`. Unit-test all 16 present/absent combinations. A missing or malformed config MUST fall back to defaults, never error on `box uuid`.

**4. Destructive flags bypassing v1 safety discipline**
`dupes --delete`, `flatten --move`, `bulk-rename --backup` are the only data-loss surface in v2. Dry-run is the default; `--force` executes only after a clean pure pre-flight; abort-all-before-any (full plan as a pure I/O-free preflight() before any filesystem write); snapshot-the-tree-unchanged test for every abort path. `flatten --move` = copy then verify (dest exists + size matches) then only remove source. `dupes --delete` = keep at least one copy per group; hardlink-aware.

**5. `dupes --delete` hardlink false-positive**
Hardlinked paths have identical content but share one inode. Reporting them as wasted space is wrong; deleting one is data loss with no disk reclaim. Detect via `(volume_serial_number(), file_index())` from `fs::metadata(path)` - NOT `DirEntry::metadata()` (returns `None` for those fields, a confirmed std gotcha). Collapse shared-identity paths before computing wasted space.

**6. BLAKE3-default breaking change**
`box hash file` now emits BLAKE3 where it emitted SHA-256. Users with scripted checksum workflows, stored baselines, or `--verify` round-trips will be silently broken. Mitigations: loud `--help`/changelog note; `--algo sha256` explicit override preserved; config `hash.default_algo = "sha256"` restores old behavior globally. Ship config before or with the BLAKE3 flip. The `--verify` 64-hex tie **stays SHA-256** — the length→algo table is unchanged, because the breaking change is the COMPUTE default only; on a 64-hex `--verify` mismatch, probe the file's BLAKE3 and emit a diagnostic hint pointing at `--algo blake3`. *(Amended 2026-06-25 per Phase-6 discuss decision D-04/D-05; the original "now maps to BLAKE3" wording would have silently broken stored SHA-256 baselines — verify must not reinterpret already-stored digests.)*

**7. ANSI leaking into `--json`/piped output**
Every new colored feature must pass through `is_color_on()`. `init_output` forces `COLOR_ON = false` when `--json` or `--clip` is set. Add per-command `_piped_no_ansi` stdout scan for every newly colored command.

**8. Terminal-loop discipline (`lolcat --animate`, `matrix` extensions)**
Arm the existing `RawGuard` RAII type immediately after `enable_raw_mode()?`. Detect TTY first; degrade to static render when piped or under `--json`. Single-flush-per-frame (`queue!` + `flush()` once). `KeyEventKind::Press`-only quit filter for Ctrl+C/q/Esc (avoids Windows press+release double-fire).

---

## Decisions for REQUIREMENTS

| # | Decision | Research Recommendation | Status |
|---|----------|------------------------|--------|
| D-1 | **Config crate vs hand-roll** | `config 0.15.24` + `dirs 6.0.0` | Needs requirements confirmation |
| D-2 | **`windows` Win32 dep** - `du --on-disk` + `pomodoro --sound` go/no-go | Both scope-optional; `windows 0.61` only ships if at least one does | Needs explicit go/no-go per feature |
| D-3 | **Large-number JSON policy** | Decide per-field: bare `u64` (PS7-first, documented JS caveat) or string-encoded for >2^53 | Needs one rule for the whole spine before Phase 7 |
| D-4 | **Non-UTF-8 path policy in `--json`** | `to_string_lossy()` + documented, or refuse + exit 1 | Needs one rule for the whole spine |
| D-5 | **`dupes --delete` scope** | Non-interactive `--keep first` + `--force` + hardlink-aware, or defer to v3 | Needs explicit go/no-go |
| D-6 | **BLAKE3-default timing** | Ship config in same phase or before, so `hash.default_algo` is available when default flips | Ordering dependency - coordinate phases |
| D-7 | **`completions` timing** | Must land AFTER `--json`/`--clip` are in `Cli` so generated script includes them | Phase 9 ordering constraint |

---

## Recommended Build Order

Phase numbers continue from v1 Phase 5. Rationale: order by integration risk (v1 retrospective #1 lesson).

### Phase 6 - Scriptable-core foundation (spine + 2 pilot commands)

**Rationale:** Build the entire shared spine once, prove it on `uuid` (zero input, single value) and `hash` (already `Option`-shaped, home of the BLAKE3 flip). An architecture flaw costs 2 commands of rework, not 23.
**Delivers:** `JSON_ON`/`CLIP_ON` atomics, `emit_json`, `out_line`, `CLIP_BUF`/`flush_clip` in `core::output`; `core::config` with `OnceLock` + `init_config()`; `BoxError::Config`; `--json`/`--clip` on `uuid` and `hash`; BLAKE3-default flip with config-tier override; config-precedence unit tests; JSON-purity + `--clip` capture tests as templates for every later command.
**Addresses:** Pitfalls 1, 2, 3, 6; Decision 1 (config approach implemented); Decision 6 (BLAKE3 + config co-shipped).
**Research flag:** STANDARD PATTERNS - all architecture decisions resolved. No research phase needed.

### Phase 7 - Roll `--json`/`--clip` across value-producing commands

**Rationale:** Apply the Phase-6 template to all remaining applicable commands, simplest-to-nested. Contract is frozen; per-command cost is mechanical. Surfaces surprises on `base64`, not `flatten`.
**Wave 7a (pure transforms):** `base64`, `epoch`, `color`, `passgen`, `8ball`, `fortune`, `roast`, `cowsay`.
**Wave 7b (filesystem buffered-rows):** `du`, `tree`, `dupes`, `flatten`, `bulk-rename` - validate buffer-then-serialize, top-level-object decision.
**Wave 7c (remaining):** `json`, `qr`, `weather`. Skip/N/A: `matrix`, `pomodoro`, `ascii`, `lolcat`, `clip`.
**Delivers:** Consistent `--json`/`--clip` on all applicable commands; per-command JSON-purity + `--clip` capture tests.
**Addresses:** Pitfall 7 (ANSI gate per new colored command); Decisions 3, 4 (number/path policy applied per field).
**Research flag:** STANDARD PATTERNS - template fixed from Phase 6. No research phase needed.

### Phase 8 - Per-command depth flags

**Rationale:** Every command already has its output struct; a new depth field slots into both human + JSON paths for free. Each flag is local to one command, low integration risk.
**Filesystem:** flatten `--move` + filters + progress; hash multi-file + progress + coreutils format; dupes multi-stage + hardlink awareness + optional `--delete`; bulk-rename case + numbering + `--backup`; tree `.gitignore` + `--dirs-only`; du percentage bar + colors + `--exclude` + optional `--on-disk`.
**Dev transforms:** uuid v7; epoch relative time + tz; color CSS names + HSL input; json `--sort-keys`; passgen entropy + `--no-similar` + `--separator`.
**Visuals:** lolcat `--animate`/`--freq`/`--seed`; matrix color/speed/charset; qr `--save`/EC; ascii color/braille/invert.
**Fun/system:** cowsay figures/think; fortune categories; 8ball art/sentiment; roast `--language`; pomodoro counter/auto-break/`--label`/`--sound`; weather `--forecast`/cache/stored-location.
**Addresses:** Pitfalls 4, 5 (destructive flags + hardlink); Pitfall 8 (lolcat/matrix animate discipline); Decision 2 (windows dep if du-on-disk or pomodoro-sound).
**Research flag:** Destructive flags (`--move`, `--delete`, `--backup`) require adversarial code review - not a research phase (same gate as v1 Phase 3 bulk-rename). Lolcat animate requires human-verify in PS7.

### Phase 9 - Meta-command polish: `config` + `completions`

**Rationale:** Both depend on the finished arg surface. `completions` must be last - it generates from the live `Cli` and must include all Phase-8 flags.
**Delivers:** `box config show/get/set/path` (with `--json`); `box completions powershell`; `install.ps1` optional completion-registration hint; final `--help`/PROJECT note for BLAKE3 breaking change.
**Addresses:** Pitfall 3 (meta-command exercises precedence resolver); Decision 7 (completions after full arg surface).
**Research flag:** STANDARD PATTERNS - both read-only over a settled `Cli`. No research phase needed.

### Phase Ordering Rationale

- Spine built once on 2 commands (Phase 6) before 21 others adopt it - flaw costs 2 commands of rework, not 23.
- Each command edited for spine once (Phase 7) and for depth once (Phase 8) - no thrashing as the contract evolves.
- BLAKE3 flip and config co-scheduled in Phase 6 so the escape hatch (`hash.default_algo = "sha256"`) is available when the breaking change ships.
- Completions/config last (Phase 9) - they consume the final arg surface; building them earlier guarantees regenerating them.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against crates.io API 2026-06-24; `windows 0.61` pin verified against tauri-winrt-notification transitive dep |
| Features | HIGH | `--json`/`--clip`/config/completions conventions verified against official PS7 docs, gh, ripgrep, bat; per-command norms verified against authoritative manuals |
| Architecture | HIGH | Recommendations checked against v1 source files; clap global/AtomicBool pattern verified via Context7 + docs.rs |
| Pitfalls | HIGH | Windows/PS7/serde_json behaviors verified against official Microsoft docs, Rust std docs, serde-rs/json issue tracker; mapped onto v1 proven patterns |

**Overall confidence: HIGH**

### Gaps to Address

- **Config crate choice (Decision 1):** project ethos decision (lean-dep vs solved-problem adoption), not a technical uncertainty. Requirements must confirm.
- **`windows 0.61` go/no-go (Decision 2):** API calls are straightforward; scope decision. Requirements must confirm `du --on-disk` and `pomodoro --sound` explicitly.
- **Large-number JSON policy (Decision 3):** must be decided once for the spine and held consistently. PS7 handles `Int64`/`BigInteger` correctly; caveat is JS/cross-language consumers.
- **`dupes --delete` scope (Decision 5):** highest-risk new surface; deferring to v3 is equally valid. Requirements must decide.
- **`chrono-tz` for epoch timezone support:** new dep (IANA database bundled); only needed if epoch `--tz` ships in Phase 8. Low technical risk; scope/timing call.

---

## Sources

### Primary (HIGH confidence)
- `docs.rs/config` 0.15.24 - `File::required(false)`, `add_source` ordered precedence, `Environment::with_prefix` - verified
- `docs.rs/clap_complete` 4.6.5 - `generate(Shell::PowerShell)`, `CommandFactory`, static AOT vs `unstable-dynamic`
- `docs.rs/uuid` 1.23.3 - `now_v7()` gated on `std`+`v7`; format methods always-available `const fn`
- `doc.rust-lang.org/std/os/windows/fs/MetadataExt` - `number_of_links()`/`file_index()` return `None` from `DirEntry::metadata()`; need `fs::metadata(path)`
- crates.io API (2026-06-24) - version pins for all new deps confirmed
- `tauri-winrt-notification/0.7.2/dependencies` - transitive `windows ^0.61` confirmed (drives 0.61 pin)
- Microsoft Learn - `ConvertFrom-Json` (PS 7.6): array enumeration; NDJSON not natively supported
- Microsoft Learn - `about_Character_Encoding` (PS7): `utf8NoBOM` default, BOM handling
- v1 source files (`src/cli.rs`, `src/main.rs`, `src/core/output.rs`, `src/commands/{uuid,hash,clip,tree,du,weather}/mod.rs`) - authoritative for existing architecture
- `.planning/RETROSPECTIVE.md` - #1 lesson: order by integration risk
- `.planning/STATE.md` - accumulated pitfalls (RawGuard, single-flush-per-frame, `is_color_on()` gate, arboard main-thread)

### Secondary (MEDIUM confidence)
- clap GitHub issues #3918, #3166 - PowerShell native/dynamic completion gap; static generation recommended
- clap-rs/clap issues #5525, #3269 - `from_global` is opt-in per consumer (no `flatten, from_global`)
- serde-rs/json issues #505/#721/#845 - `arbitrary_precision` round-trip; #329 (64-bit-as-string interop)
- Rust CLI recommendations (sunshowers.io) - `Option<T>`-no-default + `.or()` merge pattern

### Tertiary (LOW confidence)
- `dust -j` field names - confirmed as one document/nested tree; exact field names not published
- `eza` JSON rejection - issue #1064 "not planned" (useful negative signal)

---
*Research completed: 2026-06-24*
*Ready for roadmap: yes*
