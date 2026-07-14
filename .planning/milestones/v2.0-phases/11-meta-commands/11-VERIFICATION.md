---
phase: 11-meta-commands
verified: 2026-07-14T21:00:00Z
status: verified
score: 6/6 must-haves verified
overrides_applied: 0
human_verification_completed: 2026-07-14T20:47:27Z  # tab-completion confirmed live via CommandCompletion::CompleteInput (the engine TAB invokes) — see 11-HUMAN-UAT.md
re_verification:
  previous_status: gaps_found
  previous_score: 4/6
  gaps_closed:
    - "WR-01 (D-06 env-tier parity): `config get`/`config show --json` now resolve `hash.default_algo` through the SAME `pub(crate) hash::effective_default_algo()` (env>config>builtin) that `run_compute` consumes — reproduced live with `BOX_HASH_DEFAULT_ALGO=md5` + no config: `config get` → `md5`, `config show --json` → `.hash.default_algo == \"md5\"`, `box hash` → MD5 digest. All three agree."
    - "WR-02 (SC4 robustness): a malformed `%APPDATA%\\box\\config.toml` no longer bricks `config path`/`config set`/`completions powershell` (all exit 0 live), `config set` repairs the file (verified: file content changes from `bogus_key = 1` to a clean `[hash]\\ndefault_algo = \"blake3\"`, and a subsequent `get` reads it back), while `box hash` and `config show`/`get` still exit 2 under the same malformed file (no tolerance hole) — reproduced live via direct binary execution."
  gaps_remaining: []
  regressions: []
---

# Phase 11: Meta-Commands Verification Report

**Phase Goal:** Ship the two frictionless-PS7 meta-commands (`box config` + `box completions`) last, generated against the now-final and complete `Cli` arg surface so `completions` reflects every flag added in Phases 6-10.
**Verified:** 2026-07-14T21:00:00Z (automated) · 2026-07-14 (human item closed)
**Status:** verified — the one human item (live PS7 tab-completion) was confirmed 2026-07-14 via `CommandCompletion::CompleteInput` (the completion engine PSReadLine's TAB invokes); all subcommands + flags returned. See 11-HUMAN-UAT.md.
**Re-verification:** Yes — after gap closure (11-03-PLAN.md); human portion closed 2026-07-14 via /gsd-verify-work

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 (SC1) | `box config show/get <key>/set <key> <value>/path` read/edit/locate `%APPDATA%\box\config.toml`; `show --json` emits effective config as JSON | ✓ VERIFIED | `cargo test --test config_cmd` 14/14 pass (regression, unaffected by 11-03); live manual run confirms `show --json` → clean JSON object, exit 0 |
| 2 (SC2, literal) | `box config set hash.default_algo sha256` then bare `box hash file.bin` emits SHA-256 | ✓ VERIFIED | `set_then_hash_roundtrips_sha256` passes; unaffected by 11-03's changes (CLI/`--algo` unset path is algebraically identical) |
| 3 (SC3) | `box completions powershell` emits a static PS7 script tab-completing subcommands/flags/hints incl. global `--json`/`--clip` and every Phase-8/9/10 depth flag, generated from the live final `Cli` | ✓ VERIFIED | `cargo test --test completions` 6/6 pass; live run of `box completions powershell` under a malformed config still emits `Register-ArgumentCompleter` (now decoupled from the config gate per WR-02) |
| 4 (SC4, literal) | First-time user with no config runs every command unchanged; malformed config produces a clear error, not a panic | ✓ VERIFIED | Regression: `tests/config.rs` malformed-config tests stay green; a normal command under a malformed config still prints `error: config error in ...` and exits 2 (not a panic) — reproduced live |
| 5 (WR-01, D-06 env-tier parity) | `config show`/`config get hash.default_algo` report the SAME value `box hash` actually consumes, across ALL precedence tiers (env included) | ✓ VERIFIED (was FAILED) | Reproduced live with `BOX_HASH_DEFAULT_ALGO=md5` + no config: `config get hash.default_algo` → `md5`, `config show --json` → `.hash.default_algo == "md5"`, `box hash <file>` → `34be958a921e43d813a2075297d8e862  <path>` (the known MD5 of `b"box"`). All three agree. Also confirmed by `env_tier_parity_hash_default_algo` (config_cmd.rs) passing. Source: `hash::effective_default_algo()` is the single resolver, consumed by both `run_compute` (`cli_algo.unwrap_or_else(effective_default_algo)`, hash/mod.rs:320) and config's `effective()`/`get` (config/mod.rs:135, :184). `grep -rl BOX_HASH_DEFAULT_ALGO src/` returns only `hash/mod.rs` — no second lookup exists. |
| 6 (WR-02, SC4 robustness) | A malformed config does not brick the config-repair path (`config set`/`path`) or the install-registered `completions` shell-start line, while normal commands and `config show`/`get` still exit 2 (no tolerance hole) | ✓ VERIFIED (was FAILED) | Reproduced live with a malformed `bogus_key = 1` config: `config path` → exit 0; `config set hash.default_algo blake3` → exit 0 AND rewrites the file to a clean `[hash]\ndefault_algo = "blake3"` (confirmed by direct file read + a subsequent `config get` reading `blake3`, exit 0); `completions powershell` → exit 0, emits the registration script. Under the SAME malformed file: `box hash <file>` → exit 2 with `error: config error in ...`; `config show`/`get` → exit 2. Also confirmed by `malformed_config_path_and_completions_exit0`, `malformed_config_set_repairs_file`, `malformed_config_still_bricks_normal_command` (config_cmd.rs) all passing. Source: `main.rs`'s `config_independent` predicate (`Commands::Completions(_) => true`, `Commands::Config(args) => args.tolerates_malformed_config()`) gates a fallback to `core::config::init_config_default()` only for the tolerant set; `ConfigArgs::tolerates_malformed_config()` returns true only for `Path`/`Set` (config/mod.rs:78-83). |

**Score:** 6/6 truths verified (0 FAILED — both previously-failing truths reproduced VERIFIED live)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/commands/hash/mod.rs` | `pub(crate) fn effective_default_algo()` — env>config>builtin resolver; `run_compute` layers CLI over it | ✓ VERIFIED | Present at line 144-150; `run_compute` line 320: `cli_algo.unwrap_or_else(effective_default_algo)`; `parse_algo` stays private (line 132) |
| `src/commands/config/mod.rs` | `effective()`/`get` hash arm call `hash::effective_default_algo()`; `ConfigArgs::tolerates_malformed_config()` | ✓ VERIFIED | Both call sites present (lines 135, 184); `tolerates_malformed_config()` present (lines 78-83), body matches `ConfigCommand::Path \| ConfigCommand::Set { .. }` exactly |
| `src/core/config.rs` | `pub fn init_config_default()` — default-fallback initializer | ✓ VERIFIED | Present at line 146-148, idempotent `CONFIG.set(Config::default())` |
| `src/main.rs` | Command-aware config gate: malformed config falls back to `Config::default()` ONLY for `Completions` + `Config` path/set | ✓ VERIFIED | `config_independent` computed pre-move (lines 75-79); gate logic at lines 95-105; shared `dispatch(cli)` fn (lines 156-187) runs the identical spine for both branches |
| `tests/config_cmd.rs` | Env-tier parity regression test (WR-01) + malformed-config robustness tests (WR-02) | ✓ VERIFIED | `env_tier_parity_hash_default_algo`, `malformed_config_path_and_completions_exit0`, `malformed_config_set_repairs_file`, `malformed_config_still_bricks_normal_command` all present and pass; 14/14 tests in the file pass |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `hash/mod.rs::run_compute` | `hash/mod.rs::effective_default_algo` | `cli_algo.unwrap_or_else(effective_default_algo)` | ✓ WIRED | Line 320 |
| `config/mod.rs::effective` | `hash::effective_default_algo` | direct call | ✓ WIRED | Line 135 |
| `config/mod.rs::get` (hash arm) | `hash::effective_default_algo` | direct call | ✓ WIRED | Line 184 |
| `main.rs` dispatch gate | `core::config::init_config_default` | malformed-but-independent fallback branch | ✓ WIRED | Line 99, gated by `config_independent` (lines 75-79, 98) |
| `main.rs` `config_independent` | `ConfigArgs::tolerates_malformed_config` | `Commands::Config(args) => args.tolerates_malformed_config()` | ✓ WIRED | main.rs:77 → config/mod.rs:78-83 |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| WR-01 live reproduction | `BOX_HASH_DEFAULT_ALGO=md5` (no config) + `config get`/`config show --json`/`box hash` | all three report/emit `md5` — agree | ✓ PASS |
| WR-02 live reproduction (independent commands) | malformed config + `config path`/`config set hash.default_algo blake3`/`completions powershell` | all exit 0; `set` rewrites file to clean `[hash]\ndefault_algo = "blake3"` | ✓ PASS |
| WR-02 live reproduction (no-hole guard) | SAME malformed config + `box hash <file>`/`config show`/`config get` | all exit 2 with `error: config error in ...` | ✓ PASS |
| `config_cmd` full suite | `cargo test --test config_cmd` | 14/14 pass (10 pre-existing + 4 new WR-01/WR-02 tests) | ✓ PASS |
| `completions` suite (regression) | `cargo test --test completions` | 6/6 pass | ✓ PASS |
| Full workspace test suite | `cargo test` | all binaries, 0 failures | ✓ PASS |
| Lint gate | `cargo clippy --bin box --all-targets -- -D warnings` | clean | ✓ PASS |
| Source assertions (WR-01) | `grep -rl BOX_HASH_DEFAULT_ALGO src/` | returns only `src/commands/hash/mod.rs` | ✓ PASS |
| Source assertions (WR-02) | `grep -n init_config_default\|tolerates_malformed_config\|config_independent src/main.rs src/core/config.rs src/commands/config/mod.rs` | all match | ✓ PASS |
| Commit provenance | `git cat-file -t <hash>` for 7d18396/d5728b5/137a403/12f277a/bde5158 | all resolve to real commits, correct RED→GREEN→style order in `git log` | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CFG-01 | 11-01, 11-03 | `box config show/get/set/path` reads/edits/locates config, with `--json`; effective view (incl. env tier) never drifts from consumption | ✓ SATISFIED | Core CRUD verified live and by tests; the D-06 env-tier parity gap is now closed (WR-01) — `REQUIREMENTS.md` line 43 marks CFG-01 `[x]` Complete, mapped to Phase 11 |
| CMP-01 | 11-02, 11-03 | `box completions powershell` emits a static completion script from the live `Cli`; not bricked by a malformed config | ✓ SATISFIED | Script content, purity, live-Cli reflection all confirmed; WR-02 decoupling verified live — `REQUIREMENTS.md` line 44 marks CMP-01 `[x]` Complete, mapped to Phase 11 |

No orphaned requirements — `REQUIREMENTS.md`'s traceability table (lines 156-157) maps exactly CFG-01/CMP-01 to Phase 11, both accounted for in the 11-01/11-02/11-03 plan frontmatters.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` debt markers found in any file modified by 11-03 (`src/commands/hash/mod.rs`, `src/commands/config/mod.rs`, `src/core/config.rs`, `src/main.rs`, `tests/config_cmd.rs`).

The three pre-existing ℹ️ Info items from the initial verification (`atomic_write` fixed temp-name, `install.ps1` smoke-test exception path, case-sensitive `config set` enum values) are unchanged by 11-03 and remain out of scope for this gap-closure plan (not required by CFG-01/CMP-01, not flagged as blockers in either verification pass).

### Human Verification Required

### 1. Live PowerShell 7 tab-completion behavior

**Test:** Run `box completions powershell | Out-String | Invoke-Expression` in a real PowerShell 7 session, then type `box <TAB>` repeatedly and `box weather --<TAB>`.
**Expected:** Subcommand names (including `config`/`completions`) and flag names (including `--json`/`--clip`/`--forecast` etc.) cycle through via tab-completion.
**Why human:** PSReadLine's actual tab-completion UX in a live terminal session cannot be verified by grep/byte-pattern checks. The generated script's textual content (the `Register-ArgumentCompleter` registration call and the presence of every subcommand/flag name) was verified programmatically and is strong indirect evidence, but the interactive behavior itself needs a human at a real prompt. This item is carried forward unchanged from the initial verification — 11-03 did not touch `commands/completions/mod.rs` and this check was never part of the WR-01/WR-02 gap-closure scope.

## Gaps Summary

Both gaps from the initial verification are closed and independently reproduced here by direct binary execution (not just trusting SUMMARY.md or the test suite):

1. **WR-01 (env-tier parity) — CLOSED.** `config get hash.default_algo` and `config show --json` now resolve through the same `pub(crate) hash::effective_default_algo()` function `box hash` consumes. Live repro with `BOX_HASH_DEFAULT_ALGO=md5` and no config file: all three commands agree on `md5`. Source assertion confirms `BOX_HASH_DEFAULT_ALGO` is looked up in exactly one file (`src/commands/hash/mod.rs`).

2. **WR-02 (malformed-config robustness) — CLOSED.** Under a malformed `config.toml`, `config path`, `config set hash.default_algo blake3`, and `completions powershell` all now exit 0 (live-verified), and `config set` demonstrably rewrites the malformed file to a clean, re-parseable one. The no-hole guard also holds live: `box hash` and `config show`/`get` still exit 2 under the identical malformed file.

Full regression: the 507-test-plus workspace suite passes with 0 failures (`cargo test`), `config_cmd` is 14/14, `completions` is 6/6, and `cargo clippy --bin box --all-targets -- -D warnings` is clean. All five files 11-03 touched were reviewed directly (not just the SUMMARY's description of them) and match the plan's `must_haves` exactly, including the specific function names, call sites, and doc-comment corrections.

The ONLY open item is a human-verification requirement carried forward unchanged from the initial report: confirming live PSReadLine tab-completion behavior in a real PowerShell 7 session. This was never a code gap — it is an inherent limitation of static analysis against interactive terminal UX — and is unrelated to the WR-01/WR-02 fixes. Per the standard verification decision tree, an open human-verification item routes the status to `human_needed` rather than `passed`, even though all 6 observable truths and all requirements are now fully verified in code.

---

_Verified: 2026-07-14T21:00:00Z_
_Verifier: Claude (gsd-verifier)_
