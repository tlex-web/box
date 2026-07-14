---
phase: 6
slug: scriptable-core-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-25
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `06-RESEARCH.md` § Validation Architecture. The validation surfaces
> below are the reusable templates Phase 7 copies across 23 commands.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness + `assert_cmd` 2.2 (black-box binary) + `assert_fs` 1.1 / `tempfile` 3.27 (fixtures) + `predicates` 3.1; `insta` 1.48 available for snapshots |
| **Config file** | none — `Cargo.toml [dev-dependencies]` only; integration tests live in `tests/*.rs` |
| **Quick run command** | `cargo test --bin box` (unit tests inside `src/` — precedence resolver, `out_line`/`flush_clip` capture, `Algo` serde round-trip) |
| **Full suite command** | `cargo test` (unit + all `tests/*.rs` integration; live-clipboard tests are `#[ignore]`d by default) |
| **Estimated runtime** | quick ~1–2 s · full ~10 s |

> **Critical invariant:** `box` is binary-only — unit tests run via `cargo test --bin box`, NOT `--lib` (there is no lib target). [VERIFIED: Cargo.toml `[[bin]]` only; STATE.md]

---

## Sampling Rate

- **After every task commit:** Run `cargo test --bin box` (the sub-second Nyquist quick sample)
- **After every plan wave:** Run `cargo test` (full unit + integration; clipboard tests `#[ignore]`d)
- **Before `/gsd:verify-work`:** Full suite green **AND** a real PS7 `box uuid --json | ConvertFrom-Json` human-verify **AND** a live `--clip` round-trip human-verify
- **Max feedback latency:** ~2 seconds (quick sample)

---

## Per-Task Verification Map

> Task IDs are assigned by the planner (sketch: **06-01** spine = `core::output` + `core::config` + `errors` + `cli` + `main`; **06-02** pilots = `uuid` + `hash`). Rows below map each phase requirement-behavior to its automated check from `06-RESEARCH.md`. The executor binds each row to a concrete task ID as plans are written.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | 06-02 | 2 | SPINE-01 | — | stdout is exactly one JSON value, no ANSI, no BOM, snake_case, `{results,count}` | integration | `cargo test --test uuid json_purity` | ❌ W0 (tests/uuid.rs) | ⬜ pending |
| TBD | 06-02 | 2 | SPINE-01 | — | `box hash file --json` → `{results:[{path,algo,digest}],count:1}` (Phase-8-compatible literal) | integration | `cargo test --test hash json_shape` | ❌ W0 (tests/hash.rs) | ⬜ pending |
| TBD | 06-02 | 2 | SPINE-01 | — | under `--json`, failure → stdout EMPTY + `error:` on stderr + exit 1/2 (D-09) | integration | `cargo test --test hash json_error_empty_stdout` | ❌ W0 | ⬜ pending |
| TBD | 06-01 | 1 | SPINE-03 | — | `box uuid -n 5 --clip` tees all 5 lines into `CLIP_BUF` (capture logic) | unit | `cargo test --bin box out_line_tees` | ❌ W0 (output.rs) | ⬜ pending |
| TBD | 06-01 | 1 | SPINE-03 | — | empty output → no clipboard op, no confirmation (D-08) | unit | `cargo test --bin box flush_clip_empty_noop` | ❌ W0 | ⬜ pending |
| TBD | 06-02 | 2 | SPINE-03 | T-clip-disclosure | live round-trip (`--clip` → read back) Unicode-exact | integration `#[ignore]` + human | `cargo test --test uuid -- --ignored --test-threads=1` | ❌ W0 (mirror tests/clip.rs) | ⬜ pending |
| TBD | 06-01 | 1 | SPINE-05 | T-config-injection | precedence: CLI `--algo sha256` ▸ env ▸ config `blake3` ▸ builtin | unit (pure resolver) | `cargo test --bin box precedence_matrix` | ❌ W0 | ⬜ pending |
| TBD | 06-01 | 1 | SPINE-05 | — | missing config → `box uuid` still prints a UUID (silent default) | integration | `cargo test --test config missing_is_silent` | ❌ W0 (tests/config.rs) | ⬜ pending |
| TBD | 06-01 | 1 | SPINE-05 | T-config-dos | malformed config → exit 2 BEFORE the op (D-10) | integration | `cargo test --test config malformed_exit2` | ❌ W0 | ⬜ pending |
| TBD | 06-02 | 2 | HASH-V2-01 | — | `box hash file` (no `--algo`) emits 64-hex BLAKE3 (was sha256) | integration | `cargo test --test hash default_is_blake3` | ❌ W0 | ⬜ pending |
| TBD | 06-02 | 2 | HASH-V2-01 | — | `box hash --algo sha256 file` still emits SHA-256 | integration | `cargo test --test hash algo_sha256_still_works` | ✅ (tests/hash.rs) | ⬜ pending |
| TBD | 06-02 | 2 | HASH-V2-01 | — | bare `--verify <64-hex sha256>` STILL passes (regression backstop — must pass UNCHANGED) | integration | `cargo test --test hash hash_verify_autodetect` | ✅ (tests/hash.rs:122-139) | ⬜ pending |
| TBD | 06-02 | 2 | HASH-V2-01 | — | 64-hex mismatch emits the BLAKE3-fallback hint on stderr (D-05) | integration | `cargo test --test hash verify_blake3_probe_hint` | ❌ W0 | ⬜ pending |
| TBD | 06-02 | 2 | HASH-V2-01 | T-config-injection | config `default_hash_algo = "sha256"` restores SHA-256; CLI `--algo blake3` still wins | integration | `cargo test --test config hash_default_override` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

> **Atomic-test-isolation note:** tests that mutate the process-global `JSON_ON`/`CLIP_ON`/`COLOR_ON` atomics MUST serialize via a `Mutex` lock, exactly like the existing `COLOR_LOCK` at `src/core/output.rs:255`. The default parallel runner will otherwise interleave a `true`/`false` store between another test's store and read. Reuse the proven v1 pattern. [VERIFIED: src/core/output.rs:250-256, 302-303]

---

## Wave 0 Requirements

- [ ] `tests/config.rs` (NEW) — SPINE-05 missing / malformed / precedence-via-binary
- [ ] `tests/uuid.rs` (extend) — JSON-purity + `--clip` capture template (the copy-me template Phase 7 reuses)
- [ ] `tests/hash.rs` (extend) — BLAKE3-default flip + JSON shape + D-05 probe hint; keep `hash_verify_autodetect` **PASSING UNCHANGED** (regression backstop)
- [ ] `src/core/output.rs` unit tests — `out_line` tee, `flush_clip` empty-no-op, `emit_json` no-BOM (mirror the `COLOR_LOCK`-serialized atomic-mutation pattern at output.rs:247-358)
- [ ] `src/core/config.rs` (NEW) unit tests — pure precedence resolver matrix + `toml::from_str` malformed → `BoxError::Config`
- [ ] No framework install needed — `assert_cmd` / `assert_fs` / `predicates` / `insta` all present

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `box uuid --json \| ConvertFrom-Json` yields one well-formed object in real PowerShell 7 | SPINE-01 | PS7 `ConvertFrom-Json` auto-enumeration behavior + no-BOM only observable in the real shell | In PS7: `box uuid --json \| ConvertFrom-Json` → expect a single object with `results`/`count`; `(box uuid --json).Length` byte-check for no BOM |
| `box uuid --clip` live round-trip is Unicode-exact | SPINE-03 | Live Windows clipboard is environment-specific; automated test is `#[ignore]`d | In PS7: `box uuid --clip`; then `Get-Clipboard` → must equal the printed UUID exactly |
| "Copied to clipboard" confirmation present on TTY, suppressed when piped | SPINE-03 | TTY detection differs interactively vs piped; stderr stream behavior | In PS7: `box uuid --clip` (interactive) shows confirmation on stderr; `box uuid --clip 2>$null \| Out-Null` then `box uuid --clip > $null` piped → no confirmation |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING (❌) references
- [ ] No watch-mode flags
- [ ] Feedback latency < 2s (quick sample)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
