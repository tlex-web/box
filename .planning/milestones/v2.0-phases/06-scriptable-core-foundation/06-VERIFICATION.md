---
phase: 06-scriptable-core-foundation
verified: 2026-06-25T00:00:00Z
status: human_needed
score: 4/4 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run `box uuid --clip` in a PowerShell 7 TTY and confirm the UUID is printed to stdout, the clipboard contains the same UUID, and 'Copied to clipboard' appears on stderr."
    expected: "UUID printed; clipboard value equals printed UUID; 'Copied to clipboard' on stderr."
    why_human: "The live clipboard round-trip test (clip_roundtrip) is marked #[ignore] — it touches shared OS clipboard state not available in a headless runner. The code path is implemented (out_line tees to CLIP_BUF, flush_clip calls arboard) but correctness of the live OS clipboard write can only be confirmed in an interactive Windows TTY."
  - test: "Run `box uuid --json --clip` and confirm the JSON document (not the UUID string) is on the clipboard."
    expected: "Clipboard contains the pretty-printed JSON with 'results' and 'count' keys."
    why_human: "Same live-clipboard constraint as above. emit_json tees to CLIP_BUF under CLIP_ON (code verified), but the actual arboard write from the process exit path cannot be exercised headlessly."
  - test: "Run `box uuid --clip 2>log.txt` (redirect stderr) and confirm 'Copied to clipboard' does NOT appear in log.txt."
    expected: "log.txt is empty; confirmation is suppressed when stderr is not a TTY."
    why_human: "The stderr TTY gate (std::io::stderr().is_terminal()) cannot be tested in the automated integration suite without a real TTY."
---

# Phase 6: Scriptable-Core Foundation Verification Report

**Phase Goal:** Build the whole shared scriptable spine (`--json`, `--clip`, config-file defaults) and prove it end-to-end on `uuid` and `hash`, co-shipping the BLAKE3-default breaking change with its config escape hatch — so an architecture flaw costs 2 commands of rework, not 23.
**Verified:** 2026-06-25
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `box uuid --json \| ConvertFrom-Json` yields one well-formed JSON document with snake_case fields, no BOM, no ANSI, no progress chrome | VERIFIED | `tests/uuid.rs::json_purity` passes: parses with `serde_json::from_slice` on full stdout buffer (one-value guarantee), asserts `.results[0].version == "v4"`, `.count == 1`, 36-char UUID, no `0x1B`, no `EF BB BF` prefix. Code: `emit_json` in `src/core/output.rs` uses `serde_json::to_writer_pretty` (no BOM), `init_output` forces `owo_colors::set_override(false)` before dispatch. |
| 2 | `box uuid --clip` copies AND prints; `--json --clip` puts JSON on clipboard; confirmation on stderr suppressed when not TTY | VERIFIED (code) / HUMAN NEEDED (live) | Code: `out_line` tees `CLIP_BUF` under `CLIP_ON` (unit test `out_line_tees` passes); `emit_json` tees the serialized doc under `CLIP_ON`; `flush_clip` calls `arboard::Clipboard::new().set_text(...)` then `eprintln!("Copied to clipboard")` gated on `std::io::stderr().is_terminal()`. `#[ignore]`d `clip_roundtrip` test exists. Live clipboard round-trip requires human verification. |
| 3 | `box hash file` (no `--algo`) emits 64-hex BLAKE3 digest; `--algo sha256` still emits SHA-256 | VERIFIED | `tests/hash.rs::default_is_blake3` passes. Source: `hash/mod.rs` line 253–260: `cli_algo.or_else(|| env).or(config().default_hash_algo).unwrap_or(Algo::Blake3)`. `tests/hash.rs::hash_verify_autodetect` passes UNCHANGED confirming `algo_from_len` at 64→sha256 is byte-for-byte untouched. |
| 4 | Config `default_hash_algo = "sha256"` makes `box hash` emit SHA-256; CLI `--algo blake3` still wins; missing/malformed config never errors normal `box uuid` | VERIFIED | `tests/config.rs::hash_default_override` passes (config restores SHA-256, CLI blake3 beats config). `tests/config.rs::missing_is_silent` passes (no error, uuid exits 0, stderr empty). `tests/config.rs::malformed_exit2` passes (bogus key → exit 2, `error:` on stderr, stdout empty). |

**Score:** 4/4 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/core/config.rs` | Config{default_hash_algo}, init_config(), config(), load(), config_path(), resolve_algo() | VERIFIED | File exists with all 6 symbols. `config_path()` reads `APPDATA` env first (documented Rule 1 deviation), falls back to `dirs::config_dir()`. `load()` uses TOCTOU-free `NotFound` match. OnceLock<Config> process-global. |
| `src/core/output.rs` | JSON_ON/CLIP_ON atomics, CLIP_BUF Mutex, is_json_on, init_output, emit_json, out_line, flush_clip | VERIFIED | All 7 symbols present and substantive. `init_output` forces `COLOR_ON.store(false)` + `owo_colors::set_override(false)` after `init_color` (ordering enforced in main.rs). |
| `src/core/errors.rs` | BoxError::Config{path,message} exit-2 variant | VERIFIED | `Config { path: String, message: String }` with `#[error("config error in {path}: {message}")]` present at line 38–39. |
| `src/cli.rs` | Global --json and --clip bools on Cli | VERIFIED | `pub json: bool` and `pub clip: bool` both with `#[arg(long, global = true)]` at lines 27–33. |
| `src/main.rs` | init_config → init_color → init_output ordering; flush_clip on Ok; BoxError::Config in exit-2 downcast | VERIFIED | `init_config().and_then(|()| { init_color(...); init_output(...); match cli.command {...} }).and_then(|()| flush_clip())`. Downcast at lines 133–136 includes `Config { .. }`. |
| `tests/config.rs` | missing_is_silent, malformed_exit2, hash_default_override | VERIFIED | All 3 tests present and passing. APPDATA isolation via `cmd.env("APPDATA", appdata.path())`. |
| `Cargo.toml` | toml and dirs dependencies only | VERIFIED | `toml = "1.1.2"` at line 23, `dirs = "6.0.0"` at line 24. No other phase-6 additions confirmed by targeted grep. |
| `src/commands/uuid/mod.rs` | UuidRow{uuid,version}, UuidOutput{results,count}; is_json_on() fork; out_line human path | VERIFIED | Both structs present with `#[derive(serde::Serialize)]`. `run()` builds rows once via `format_one`, forks on `is_json_on()` first: `emit_json(&doc)` on JSON path, `out_line(&r.uuid)` on human path. |
| `src/commands/hash/mod.rs` | HashRow{path,algo,digest}, HashOutput{results,count}; Algo Serialize+Deserialize lowercase; BLAKE3 default flip; D-05 probe | VERIFIED | All structs present. `Algo` derives `serde::Serialize, serde::Deserialize` with `#[serde(rename_all = "lowercase")]`. Compute default line 260: `unwrap_or(Algo::Blake3)`. `emit_blake3_probe_hint` free fn at line 298. `algo_from_len` UNCHANGED (64→sha256). |
| `tests/uuid.rs` | json_purity, json_count_multi, human_output_unchanged, #[ignore]d clip_roundtrip | VERIFIED | All 4 tests present. `clip_roundtrip` marked `#[ignore = "touches shared OS clipboard..."]`. |
| `tests/hash.rs` | default_is_blake3, json_shape, json_error_empty_stdout, verify_blake3_probe_hint; hash_verify_autodetect UNCHANGED | VERIFIED | All 5 tests present and passing. `hash_verify_autodetect` not modified. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `crate::core::config::init_config` | Called first in `.and_then` chain before dispatch | WIRED | Line 81: `crate::core::config::init_config().and_then(...)` — its Err flows to the same downcast block |
| `src/core/config.rs::load` | `BoxError::Config` | `toml::from_str` error mapped to BoxError::Config | WIRED | Lines 82–88: `toml::from_str(&s).map_err(|e| BoxError::Config { path: ..., message: e.to_string() }.into())` |
| `src/core/output.rs::init_output` | `COLOR_ON / owo_colors::set_override` | Force color off under json\|\|clip, after init_color | WIRED | Lines 109–112: `COLOR_ON.store(false, Ordering::Relaxed); owo_colors::set_override(false)` |
| `src/main.rs` | `crate::core::output::flush_clip` | Chained on Ok dispatch branch only | WIRED | Line 111: `.and_then(\|()| crate::core::output::flush_clip())` — only runs when dispatch returned Ok |
| `src/commands/uuid/mod.rs` | `crate::core::output::{is_json_on, emit_json, out_line}` | is_json_on() fork in run() | WIRED | Lines 68–79: `if crate::core::output::is_json_on() { emit_json(&doc)? } else { out_line(&r.uuid) }` |
| `src/commands/hash/mod.rs` | `crate::core::config::config().default_hash_algo` | Precedence chain in no-verify branch | WIRED | Lines 258–260: `.or(crate::core::config::config().default_hash_algo).unwrap_or(Algo::Blake3)` |
| `src/commands/hash/mod.rs` (verify mismatch) | blake3 re-open probe | `emit_blake3_probe_hint` on 64-hex mismatch, no --algo, not --json | WIRED | Lines 234–239: condition `expected.len() == 64 && cli_algo.is_none() && !is_json_on()` followed by `emit_blake3_probe_hint(expected, path_for_probe.as_deref())` |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `src/commands/uuid/mod.rs` | `rows: Vec<UuidRow>` | `uuid::Uuid::new_v4()` via `format_one` — live CSPRNG | Yes — OS entropy, not hardcoded | FLOWING |
| `src/commands/hash/mod.rs` | `computed: String` | `digest_reader(algo, input.reader)` — streams file bytes | Yes — real file content, not static | FLOWING |
| `src/core/config.rs` | `Config.default_hash_algo` | `toml::from_str` of real file contents | Yes — live file read or Config::default() | FLOWING |

---

### Behavioral Spot-Checks

The full `cargo test` suite was run and produced:

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full test suite (157 unit + all integration tests) | `cargo test` | 0 failed, 4 ignored (live clipboard + matrix/pomodoro TTY), 0 regressions | PASS |
| Config integration: missing/malformed/precedence | `cargo test --test config` | 3/3 passed | PASS |
| UUID JSON purity + count + human regression | `cargo test --test uuid` (non-ignored) | 7/7 passed | PASS |
| Hash BLAKE3 default + JSON shape + verify probe + autodetect unchanged | `cargo test --test hash` | 11/11 passed | PASS |
| Unit: precedence_matrix, out_line_tees, flush_clip_empty_noop, emit_json_no_bom_no_ansi, init_output_forces_color_off | `cargo test --bin box` | All passed within the 157 total | PASS |

---

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` probes exist for this phase. The PLAN verification block uses `cargo test` commands, all confirmed passing above.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| SPINE-01 | 06-01 + 06-02 | `--json` emits one well-formed JSON document, snake_case, no BOM/ANSI | SATISFIED | `json_purity` (uuid) + `json_shape` (hash) both pass; `emit_json` implements the contract; REQUIREMENTS.md marks Complete |
| SPINE-03 | 06-01 + 06-02 | `--clip` copies primary result, prints it, confirmation on stderr TTY-gated | SATISFIED (code) / HUMAN NEEDED (live) | `out_line` tees, `flush_clip` writes arboard + stderr confirmation; `clip_roundtrip` exists as #[ignore]d test; REQUIREMENTS.md marks Complete |
| SPINE-05 | 06-01 | CLI > env > config > builtin precedence; missing config silent; malformed exits 2 | SATISFIED | `precedence_matrix` unit test passes; `missing_is_silent` + `malformed_exit2` integration tests pass; REQUIREMENTS.md marks Complete |
| HASH-V2-01 | 06-02 | BLAKE3 compute default; `--verify` length table UNCHANGED; D-05 probe | SATISFIED | `default_is_blake3` passes; `hash_verify_autodetect` passes UNCHANGED; `verify_blake3_probe_hint` passes; `hash_default_override` proves config escape hatch; REQUIREMENTS.md marks Complete |

---

### Anti-Patterns Found

A targeted scan was run on all files modified by this phase.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/core/config.rs` | 127 | `#[allow(dead_code)]` on `resolve_algo` | INFO | Forward-compat allow documented explicitly: `resolve_algo` is unit-tested by `precedence_matrix` and available for future callers; `hash` uses an inline `.or()` chain instead. Not a stub — the function has a full implementation and a passing test. No TBD/FIXME/XXX markers found. |
| All phase-6 files | — | No `TBD`, `FIXME`, or `XXX` markers found | — | Debt-marker gate: PASSED. |
| All phase-6 files | — | No unreferenced placeholder patterns (`return null`, `return []`, `println!("not implemented")`) | — | Stub detection: PASSED. |

---

### Human Verification Required

#### 1. Live clipboard write: `box uuid --clip`

**Test:** In a PowerShell 7 TTY, run `box uuid --clip`. Read the clipboard with `Get-Clipboard`.
**Expected:** The UUID printed to stdout equals the clipboard contents exactly. "Copied to clipboard" appears on stderr in the terminal.
**Why human:** `clip_roundtrip` test is `#[ignore]`d (touches shared OS clipboard; headless CI unsafe). The code path — `out_line` → `CLIP_BUF` → `flush_clip` → `arboard::Clipboard::new().set_text()` — is implemented and unit-tested for the capture behavior, but the live OS clipboard write requires an interactive Windows session.

#### 2. JSON-clip combination: `box uuid --json --clip`

**Test:** In a PowerShell 7 TTY, run `box uuid --json --clip`. Inspect clipboard with `Get-Clipboard`.
**Expected:** Clipboard contains the pretty-printed JSON `{"results":[{"uuid":"...","version":"v4"}],"count":1}`, not the UUID string.
**Why human:** `emit_json` tees `serde_json::to_string_pretty(value)` to `CLIP_BUF` when `CLIP_ON` is set — verified by unit test on the buffer; but the arboard live write path requires an interactive TTY.

#### 3. Stderr TTY gate for clipboard confirmation

**Test:** In a PowerShell 7 TTY, run `box uuid --clip 2>log.txt`. Open `log.txt`.
**Expected:** `log.txt` is empty (the "Copied to clipboard" confirmation is suppressed when stderr is not a TTY, i.e. when redirected).
**Why human:** `flush_clip` gates the `eprintln!` on `std::io::stderr().is_terminal()`. This conditional cannot be exercised in automated tests without a real TTY session. The code is correct (`is_terminal()` is the right predicate) but the behavior needs a live terminal to confirm.

---

### Gaps Summary

No gaps were found. All 4 observable truths are VERIFIED in the codebase. The 3 human verification items are for live-clipboard behavior that the codebase correctly implements but that cannot be exercised in a headless test environment by design (the `#[ignore]` pattern is the documented convention for this project, mirroring `tests/clip.rs`).

---

*Verified: 2026-06-25*
*Verifier: Claude (gsd-verifier)*
