---
phase: 11-meta-commands
verified: 2026-07-14T20:00:00Z
status: gaps_found
score: 4/6 must-haves verified
overrides_applied: 0
gaps:
  - truth: "`config show`/`config get hash.default_algo` accurately reflect the SAME value `box hash` will actually consume — i.e. the full CLI>env>config>builtin resolver (D-06 parity, as documented in the `commands/config/mod.rs` module doc: \"config show can never lie about what box hash/box weather will consume\")"
    status: failed
    reason: "Empirically reproduced: with BOX_HASH_DEFAULT_ALGO=md5 set and no config file, `box hash <file>` emits an MD5 digest, but `box config get hash.default_algo` prints `blake3` and `box config show --json` reports `.hash.default_algo == \"blake3\"` — both silently drop the live env tier. `effective()`/`get`'s hash arm resolve only `config().hash.default_algo.unwrap_or(Algo::Blake3)`, a 2-tier (config>builtin) resolution, while `hash::run_compute` resolves `cli.or(env).or(config).unwrap_or(builtin)` (4-tier). Weather has no env tier so its parity holds; this is specific to `hash.default_algo`. Matches code-review WR-01 (11-REVIEW.md), left unresolved (not marked RESOLVED, unlike the analogous Phase 10 WR-01/WR-02 which were closed via a dedicated 10-06 gap-closure plan before that phase's verification)."
    artifacts:
      - path: "src/commands/config/mod.rs"
        issue: "`effective()` (lines 109-120) and `get`'s `\"hash.default_algo\"` arm (line 159-162) omit the `BOX_HASH_DEFAULT_ALGO` env-var tier that `src/commands/hash/mod.rs`'s `run_compute` (line 301-308) consults"
    missing:
      - "Resolve `effective()`'s/`get`'s hash arm through the SAME env>config>builtin chain `run_compute` uses (e.g. via a shared/pub(crate) `parse_algo` + `std::env::var(\"BOX_HASH_DEFAULT_ALGO\")` lookup before falling back to `config().hash.default_algo`), OR explicitly narrow the module doc / `box config --help` text so it no longer claims parity with \"what box hash will consume\" when an env override is scoped out."
  - truth: "A malformed `%APPDATA%\\box\\config.toml` still leaves the config meta-commands usable as a repair path (`config path`/`config set`), and does not break `box completions` when registered into $PROFILE via `install.ps1 -RegisterCompletions`"
    status: failed
    reason: "Empirically reproduced: with a malformed config.toml (unknown top-level key), `main()`'s `init_config()` gate fires for EVERY subcommand before dispatch (`src/main.rs:81-84`), so `box config path`, `box config set hash.default_algo blake3` (the intended repair action), `box config show`, and `box completions powershell` all exit 2 with the same `error: config error in ...` line — none can locate/repair the file, and the install-registered `box completions powershell | Out-String | Invoke-Expression` line in $PROFILE would print that error on EVERY new PowerShell session. This directly undercuts the phase's stated core value (\"frictionless PS7\") even though the literal ROADMAP SC4 wording (\"a malformed config file produces a clear error rather than a panic\") is technically satisfied — the error IS clear and not a panic, just applied indiscriminately to commands that need no config at all. Matches code-review WR-02 (11-REVIEW.md), left unresolved."
    artifacts:
      - path: "src/main.rs"
        issue: "Lines 81-84: `init_config()` gates ALL dispatch (`.and_then(|()| { ... match cli.command { ... } })`), including `Config`/`Completions`, which do not need a successfully-parsed config to run `path`/`set`/`completions`"
    missing:
      - "Decouple config-independent commands from the global config gate — e.g. dispatch `Commands::Config`/`Commands::Completions` before/around `init_config()`, or make `config`'s `path`/`set` handlers tolerant of a load failure (load-or-default, surfacing the parse error only on `show`/`get` which need the effective value)."
human_verification:
  - test: "Run `box completions powershell | Out-String | Invoke-Expression` in a real PowerShell 7 session, then type `box <TAB>` and `box weather --<TAB>`"
    expected: "Subcommand names (incl. `config`/`completions`) and flag names (incl. `--json`/`--clip`/depth flags) tab-complete correctly"
    why_human: "Actual PSReadLine tab-completion behavior in a live terminal cannot be verified by grep/byte-pattern checks — only the generated script's textual content (Register-ArgumentCompleter, flag/subcommand names) was verified programmatically"
---

# Phase 11: Meta-Commands Verification Report

**Phase Goal:** Ship the two frictionless-PS7 meta-commands last, generated against the now-final and complete `Cli` arg surface so `completions` reflects every flag added in Phases 6-10.
**Verified:** 2026-07-14T20:00:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 (SC1) | `box config show/get <key>/set <key> <value>/path` read/edit/locate `%APPDATA%\box\config.toml`; `show --json` emits effective config as JSON | ✓ VERIFIED | `cargo test --test config_cmd` 10/10 pass; manual run: `config show --json` → `{"hash":{"default_algo":"blake3"},"weather":{"location":null,"units":"metric"}}`, exit 0; `config get`/`set`/`path` all exercised live |
| 2 (SC2, literal) | `box config set hash.default_algo sha256` then bare `box hash file.bin` emits SHA-256 | ✓ VERIFIED | `set_then_hash_roundtrips_sha256` integration test passes; manually reproduced: `set hash.default_algo sha256` → `hash <file>` emits the `BOX_SHA256` row |
| 3 (SC3) | `box completions powershell` emits a static PS7 script tab-completing subcommands/flags/hints incl. global `--json`/`--clip` and every Phase-8/9/10 depth flag, generated from the live final `Cli` | ✓ VERIFIED | Generated script (520 lines) contains `Register-ArgumentCompleter`, `config`/`completions` subcommands, global `--json`/`--clip`/`--no-color`, and depth flags from every phase: `--extensions`/`--move`/`--on-disk`/`--dirs-only`/`--backup` (Ph.8), `--v7`/`--tz`/`--sort-keys`/`--braille` (Ph.9), `--think`/`--sound`/`--forecast`/`--label` (Ph.10) — all present via `clap::CommandFactory` reflection, not manual enumeration |
| 4 (SC4, literal) | First-time user with no config runs every command unchanged; malformed config produces a clear error, not a panic | ✓ VERIFIED | Existing `tests/config.rs` missing-config coverage unaffected (full suite green); manually reproduced a malformed config → `error: config error in ...`, exit 2 (not a panic) for every command tested |
| 5 (derived, D-06 parity) | `config show`/`config get hash.default_algo` report the SAME value `box hash` will actually use, across ALL precedence tiers (the module's own documented guarantee) | ✗ FAILED | See `gaps[0]` — env-tier (`BOX_HASH_DEFAULT_ALGO`) is silently dropped by `config show`/`get`, reproduced live |
| 6 (derived, SC4 robustness) | A malformed config does not brick the config-repair path (`config set`/`path`) or the install-registered `completions` shell-start line | ✗ FAILED | See `gaps[1]` — `init_config()` gates ALL dispatch including `Config`/`Completions`, reproduced live |

**Score:** 4/6 truths verified (2 FAILED — both correspond to unresolved code-review warnings WR-01/WR-02)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/commands/config/mod.rs` | `ConfigArgs`/`ConfigCommand` nested subcommand; show/get/set/path handlers | ✓ VERIFIED | 229 lines (≥180 min); `is_json_on()`-first fork present in `show`/`get`/`emit_value` |
| `src/core/config.rs` | `pub config_path()`; `Serialize`+`skip_serializing_if`; `SETTABLE_KEYS`; `pub set_value()` | ✓ VERIFIED | All present; `set_value`/`build_config_toml` implement validate-before-write; 11 passing unit tests incl. `build_config_toml_invalid_enum_errs`, `build_config_toml_unknown_key_errs_config_usage` |
| `src/core/fs.rs` | `pub atomic_write(path, contents)` — parent-dir create, temp-write, rename | ✓ VERIFIED | Present (lines 132-152); unit tests `atomic_write_creates_parent_and_lands_bytes`/`atomic_write_replaces_existing` pass |
| `tests/config_cmd.rs` | Integration proofs incl. SC2 round-trip, D-06 human/JSON parity, exit-code contract | ✓ VERIFIED | 285 lines (≥120 min); 10/10 tests pass |
| `src/commands/completions/mod.rs` | `CompletionsArgs` (required positional `clap_complete::Shell`) + `generate()` against `Cli::command()` | ✓ VERIFIED | 129 lines (≥40 min); required positional (no `Option`), inert header + `clap_complete::generate` call present |
| `Cargo.toml` | `clap_complete` pinned to match clap 4.6 | ✓ VERIFIED | `clap_complete = "4.6"` beside `clap = { version = "4.6", ... }` |
| `install.ps1` | registration hint + opt-in idempotent `-RegisterCompletions` | ✓ VERIFIED | `param([switch]$RegisterCompletions)` first statement; default path only prints hint; `-RegisterCompletions` path is `Select-String`-guarded, appends sentinel + oneliner |
| `tests/completions.rs` | purity + live-Cli-reflection + shell-validation + `--help` recipe proofs | ✓ VERIFIED | 158 lines (≥60 min); 6/6 tests pass |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `commands/config/mod.rs` `set()` | `core::config::set_value` | direct call | ✓ WIRED | `set_value(key, value)?` at line 189 |
| `core/config.rs` `build_config_toml` | `toml::from_str::<Config>` | re-parse before write | ✓ WIRED | Line 270, same call `load()` uses |
| `core/config.rs` `set_value` | `core::fs::atomic_write` | persist validated text only | ✓ WIRED | Line 222, gated behind the successful re-parse |
| `cli.rs` `Commands::Config` | `main.rs` dispatch | `args.run()` arm | ✓ WIRED | `main.rs:109` |
| `commands/completions/mod.rs` | `clap_complete::generate` | direct call against `Cli::command()` | ✓ WIRED | Lines 63-68 |
| `commands/completions/mod.rs` | `crate::cli::Cli::command()` | `CommandFactory` | ✓ WIRED | Line 65 |
| `cli.rs` `Commands::Completions` | `main.rs` dispatch | `args.run()` arm | ✓ WIRED | `main.rs:110` |
| `install.ps1 -RegisterCompletions` | `$PROFILE` | sentinel-guarded `Add-Content` | ✓ WIRED | Lines 98-107, `Select-String -Pattern '# box completions'` guard |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| config show/get/set/path full cycle | `cargo test --test config_cmd` | 10/10 pass | ✓ PASS |
| completions purity/reflection/help/shell-validation | `cargo test --test completions` | 6/6 pass | ✓ PASS |
| help snapshot (25 commands, incl. config/completions) | `cargo test --test cli trycmd` | pass | ✓ PASS |
| SC2 live reproduction | `config set hash.default_algo sha256` → `hash <file>` | emits SHA-256 row | ✓ PASS |
| SC3 depth-flag coverage | grep generated PS script for Ph.8/9/10 flags | all present | ✓ PASS |
| WR-01 reproduction | `BOX_HASH_DEFAULT_ALGO=md5` + `config get hash.default_algo` vs `box hash` | `get` reports `blake3`, `hash` emits MD5 — mismatch confirmed | ✗ FAIL (gap) |
| WR-02 reproduction | malformed config.toml + `config path`/`config set`/`completions powershell` | all exit 2 with the same config-parse error | ✗ FAIL (gap) |
| Full workspace test suite | `cargo test` | 31+ binaries, 0 failures | ✓ PASS |
| Lint gate | `cargo clippy --bin box --all-targets -- -D warnings` | clean | ✓ PASS |
| Release build config | `cargo build --bin box` | succeeds | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CFG-01 | 11-01 | `box config show/get/set/path` reads/edits/locates config, with `--json` | ✓ SATISFIED (with gap) | Core CRUD verified live and by tests; the "effective config" guarantee (D-06 parity, part of the requirement's spirit) has a real gap for `hash.default_algo` under the env tier (see gap #1) |
| CMP-01 | 11-02 | `box completions powershell` emits a static completion script from the live `Cli` | ✓ SATISFIED | Fully verified — script content, purity, and live-Cli reflection all confirmed |

No orphaned requirements — `REQUIREMENTS.md`'s traceability table maps exactly CFG-01/CMP-01 to Phase 11, both accounted for in the two plan frontmatters.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/main.rs` | 81-84 | `init_config()` gates ALL dispatch unconditionally | 🛑 Blocker-adjacent (gap #2) | Malformed config bricks the very repair/inspection commands (`config path`/`set`) meant to fix it, and breaks shell-start `completions` if registered |
| `src/commands/config/mod.rs` | 109-120, 159-162 | `effective()`/`get` hash resolution omits the env tier | 🛑 Blocker-adjacent (gap #1) | `config show`/`get` can report a value that differs from what `box hash` actually uses |
| `src/core/fs.rs` | 141-143 | `atomic_write`'s temp sibling name is a fixed `<path>.tmp` | ℹ️ Info (pre-existing WR-03, not newly required by this task) | Two concurrent `config set` processes could interleave writes to the same temp path; a failed rename can leave a stray `.tmp`. Single-user local CLI use makes this low-probability but non-zero. |
| `install.ps1` | 76-81 | Smoke test `& box --help` under `$ErrorActionPreference='Stop'` throws instead of reaching the graceful "open a new terminal" else-branch | ℹ️ Info (pre-existing WR-04, not newly required by this task) | Cosmetic — an unhandled exception message instead of the intended warning, only on first install before PATH refresh |
| `src/core/config.rs` | 261, 270 | `config set` enum values are case-sensitive (`Imperial` rejected) while `--units`/`BOX_HASH_DEFAULT_ALGO` are case-insensitive | ℹ️ Info (pre-existing IN-02) | UX inconsistency across the three precedence tiers, not a correctness bug (loud, not silent) |
| `src/commands/config/mod.rs` | 157-182 | `get`'s hardcoded 3-arm match can drift from `SETTABLE_KEYS` if a 4th key is ever added | ℹ️ Info (pre-existing IN-03) | Latent maintenance risk, not a current defect (`SETTABLE_KEYS` has exactly 3 entries today, both matched) |

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` debt markers found in any Phase-11-created or -modified file.

### Human Verification Required

### 1. Live PowerShell 7 tab-completion behavior

**Test:** Run `box completions powershell | Out-String | Invoke-Expression` in a real PowerShell 7 session, then type `box <TAB>` repeatedly and `box weather --<TAB>`.
**Expected:** Subcommand names (including `config`/`completions`) and flag names (including `--json`/`--clip`/`--forecast` etc.) cycle through via tab-completion.
**Why human:** PSReadLine's actual tab-completion UX in a live terminal session cannot be verified by grep/byte-pattern checks. The generated script's textual content (the `Register-ArgumentCompleter` registration call and the presence of every subcommand/flag name) was verified programmatically and is strong indirect evidence, but the interactive behavior itself needs a human at a real prompt.

## Gaps Summary

Both Phase-11 meta-commands are functionally shipped and pass every automated test (31+ test binaries, 0 failures; `config_cmd` 10/10; `completions` 6/6; `cli trycmd` including the 25-command help snapshot; clippy clean). The four literal ROADMAP success criteria (SC1-SC4) all hold as worded.

However, two defects flagged by the Phase-11 code review (11-REVIEW.md WR-01/WR-02) remain unresolved and were independently reproduced here by direct execution against the built binary:

1. **`config show`/`config get hash.default_algo` silently omit the `BOX_HASH_DEFAULT_ALGO` env tier** that `box hash` actually consults. The module's own doc comment claims `config show` "can never lie about what box hash/box weather will consume" — this is demonstrably false for `hash.default_algo` when the env var is set (config reports `blake3`, `box hash` computes MD5 in the reproduction above). Weather has no env tier, so its parity holds; the gap is narrowly scoped to the one hash field.

2. **A malformed `config.toml` bricks every command, including the config meta-commands meant to inspect/repair it** (`config path`, `config set`) and the `completions` command that `install.ps1 -RegisterCompletions` wires into every new PowerShell session's `$PROFILE`. `main()`'s `init_config()` gate runs before dispatch unconditionally, so there is no way to run `box config set hash.default_algo blake3` (or even `box config path` to locate the broken file) once the file is externally corrupted, and a registered shell would print a config-parse error on every new terminal until the user manually finds and fixes/deletes the file outside `box`.

Both are real, reproducible correctness/robustness gaps rather than uncertain judgment calls. This project has an established precedent (Phase 10's WR-01/WR-02, closed via a dedicated 10-06 gap-closure plan before that phase's verification) of treating this class of code-review warning as blocking follow-up work rather than accepted scope. Recommend a similar Phase-11 gap-closure plan (e.g. 11-03) addressing both before the v2.0 milestone is considered fully closed — or, alternatively, an explicit override decision from the developer accepting the narrower scope (documenting that `config show`/`get` is a config-file view rather than a full-precedence view, and that config repair happens by hand-editing/deleting the file outside `box` until repaired).

---

_Verified: 2026-07-14T20:00:00Z_
_Verifier: Claude (gsd-verifier)_
