---
phase: 1
slug: foundation-flatten
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-22
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `01-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness + `assert_cmd 2.2` / `predicates 3.1` / `assert_fs 1.1` / `tempfile 3.27` / `trycmd 1.2` (+ optional `insta 1.48`) |
| **Config file** | none — `[dev-dependencies]` block in `Cargo.toml` (added in Wave 0) |
| **Quick run command** | `cargo test --test cli` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10–30 seconds (cold build excluded) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test cli` + `cargo build`
- **After every plan wave:** Run `cargo test` (full suite) + `cargo clippy -- -D warnings`
- **Before `/gsd:verify-work`:** Full suite green AND a manual `install.ps1` → same-session `box --help` confirmation
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

> Task IDs are assigned during planning/execution. Rows below are keyed by the research test names and map 1:1 to forthcoming tasks; the gsd-nyquist-auditor / Wave-0 work fills in the `Task ID` column.

| Test (research name) | Wave | Requirement / SC | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|----------------------|------|------------------|------------|-----------------|-----------|-------------------|-------------|--------|
| `help_lists_23_commands` | 1 | FOUND-01 / SC1 | — | N/A | integration (trycmd snapshot) | `cargo test --test cli` | ❌ W0 | ⬜ pending |
| `version_is_semver` | 1 | FOUND-02 / SC2 | — | N/A | integration | `cargo test --test cli` | ❌ W0 | ⬜ pending |
| `badcmd_exits_2` | 1 | FOUND-03 / SC2 | — | clap parse error → exit 2, stderr only | integration | `cargo test --test cli` | ❌ W0 | ⬜ pending |
| `bare_box_exits_2` | 1 | FOUND-03,08 / SC2 | — | `try_parse()` overrides clap's exit-0 default (D-08) | integration | `cargo test --test cli` | ❌ W0 | ⬜ pending |
| `stub_exits_1_to_stderr` | 1 | FOUND-05 / SC2 | — | NotImplemented → exit 1, message to stderr | integration | `cargo test --test cli` | ❌ W0 | ⬜ pending |
| `piped_help_has_no_ansi` | 1 | FOUND-04 / SC3 | — | no `\x1b[` when not a TTY (NO_COLOR/is_terminal) | integration | `cargo test --test cli` | ❌ W0 | ⬜ pending |
| `dry_run_plans_collisions_writes_nothing` | 2 | FLAT-02,03 / SC4 | — | dry-run writes zero bytes | integration (assert_fs + trycmd) | `cargo test --test flatten` | ❌ W0 | ⬜ pending |
| `flatten_copies_all_files_flat` | 2 | FLAT-01 / SC5 | T-12 | copy-only, never move; no subdirs in out | integration | `cargo test --test flatten` | ❌ W0 | ⬜ pending |
| `preserves_mtime` | 2 | FLAT-04 / SC5 | — | `std::fs::FileTimes` preserves modified time | integration (compare mtime) | `cargo test --test flatten` | ❌ W0 | ⬜ pending |
| `originals_untouched` | 2 | FLAT-04 / SC5 | T-12 | source tree byte-identical after run | integration (file count + hash) | `cargo test --test flatten` | ❌ W0 | ⬜ pending |
| `no_silent_overwrite` | 2 | FLAT-04 / SC5 | T-12 | output-dir occupied-name set seeded from `read_dir` (D-14) | integration | `cargo test --test flatten` | ❌ W0 | ⬜ pending |
| `out_inside_src_aborts` | 2 | FLAT-04 | T-DoS | canonical containment guard before any I/O | integration | `cargo test --test flatten` | ❌ W0 | ⬜ pending |
| `skips_symlinks` | 2 | FLAT-04 | T-symlink | `follow_links(false)`; symlinks skipped, no loop | integration (symlink fixture) | `cargo test --test flatten` | ❌ W0 | ⬜ pending |
| `rename::sanitize_reserved` | 2 | FLAT-02 | T-reserved | CON/PRN/AUX/NUL/COM1-9/LPT1-9 sanitized, not lost | unit | `cargo test` | ❌ W0 | ⬜ pending |
| `rename::encode_no_separator` | 2 | FLAT-02 | T-pathinject | encoded name contains no `\`,`/`,`..` | unit | `cargo test` | ❌ W0 | ⬜ pending |
| `deep_path_no_silent_loss` | 2 | FOUND-06 | T-silentloss | >260-char path fails loudly, never silently dropped | integration | `cargo test --test flatten` | ❌ W0 | ⬜ pending |
| `install_path_idempotent` | 1 | FOUND-07,08 / SC1 | T-pathcorrupt | user-scope PATH added once; ExpandString when `%VARS%` present | manual + scripted (Pester/CI optional) | `pwsh -File install.ps1` then `box --help` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/cli.rs` — SC1, SC2, SC3, FOUND-01..05 (help listing, version, exit codes, piped-no-ANSI, stub error)
- [ ] `tests/flatten.rs` — SC4, SC5, FLAT-01..04, FOUND-06 (dry-run plan, real copy, timestamps, containment guard, symlink skip, deep path)
- [ ] Unit tests in `src/commands/flatten/rename.rs` — `encode_relative`, `sanitize_reserved` (all reserved names), `dedupe` numeric fallback, NTFS case-insensitive keying
- [ ] `[dev-dependencies]` block in `Cargo.toml` — assert_cmd, predicates, assert_fs, tempfile, trycmd, insta
- [ ] Optional: `tests/cmd/*.trycmd` transcript for the locked dry-run sample (D-09) and the 23-command `--help` listing
- [ ] Optional: Pester test (or CI step) running `install.ps1` against a sacrificial PATH asserting the bin dir is added exactly once (idempotency)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `install.ps1` makes `box` runnable in the **same** PS7 session | FOUND-07, FOUND-08 / SC1 | Session-PATH refresh + registry write can't be fully asserted from `cargo test`; needs a real PS7 process | In PS7: `.\install.ps1`, then in the same window `box --help` → all 23 commands listed; reopen a fresh PS7 → `box --help` still works |
| Terminal shows **colored** `flatten` output | FOUND-04 / SC3 | Color requires a real TTY; CI/piped runs are plain by design | Run `box flatten ./src ./out --dry-run` in a PS7 terminal → status glyphs colored; pipe to a file → byte-identical minus ANSI |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
