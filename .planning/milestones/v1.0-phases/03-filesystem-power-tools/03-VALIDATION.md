---
phase: 3
slug: filesystem-power-tools
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-22
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `03-RESEARCH.md` § Validation Architecture and the shipped Phase-1/2 test conventions.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` — integration via `assert_cmd` (`Command::cargo_bin("box")`), CLI snapshots via `trycmd` |
| **Config file** | `Cargo.toml` `[dev-dependencies]` (assert_cmd, trycmd, predicates, tempfile already present from Phase 1/2) |
| **Quick run command** | `cargo test --test <cmd>` (e.g. `cargo test --test hash`) |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30–60 seconds (cold build excluded) |

**Determinism rules (from RESEARCH § Validation Architecture — snapshots flap without these):**
- Set `NO_COLOR=1` (or assert the piped byte-identical-minus-ANSI path) so ANSI never leaks into snapshots.
- `dupes` (rayon) and `du` (walk order) must `collect` → `sort` by a stable key BEFORE printing; assert sorted output, never raw parallel/walk order.
- `trycmd` normalizes `\` → `/` in Windows path snapshots — author expected output with `/`.
- `hash` snapshots use known-vector inputs (empty file, fixed bytes) so the hex is deterministic across machines.

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test <cmd>` for the command touched
- **After every plan wave:** Run `cargo test` (full suite)
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 3-XX-XX | hash | — | HASH-01 | — | known-vector SHA-256/blake3/sha512/md5 match; `--verify` exits 0 match / 1 mismatch / 2 bad-len | integration | `cargo test --test hash` | ❌ W0 | ⬜ pending |
| 3-XX-XX | tree | — | TREE-01 | — | box-drawing glyphs, dirs-first sort, `N directories, M files` summary, `--depth`/`--sizes` | integration | `cargo test --test tree` | ❌ W0 | ⬜ pending |
| 3-XX-XX | du | — | DU-01 | — | biggest-first, trailing `/` on dirs, `--top`/`--depth`, full-scan total summary | integration | `cargo test --test du` | ❌ W0 | ⬜ pending |
| 3-XX-XX | dupes | — | DUPE-01 | — | content-hash groups + wasted-space; deterministic sorted output; no mutation | integration | `cargo test --test dupes` | ❌ W0 | ⬜ pending |
| 3-XX-XX | bulk-rename | — | RENM-01 | T-RENM-clobber | dry-run default writes nothing; `--force` applies; collision/cycle/path-sep aborts ALL before any rename | integration | `cargo test --test bulk_rename` | ❌ W0 | ⬜ pending |

*Task IDs and waves finalized by the planner; rows above bind requirement → command → automated command. Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/hash.rs` — assert_cmd + trycmd stubs for HASH-01 (known vectors, `--verify` exit codes)
- [ ] `tests/tree.rs` — stubs for TREE-01 (glyphs, summary, depth/sizes)
- [ ] `tests/du.rs` — stubs for DU-01 (sort order, top/depth, summary total)
- [ ] `tests/dupes.rs` — stubs for DUPE-01 (group identity, wasted space, no mutation)
- [ ] `tests/bulk_rename.rs` — stubs for RENM-01 (dry-run-vs-force, collision/cycle/path-sep abort)

*Framework (assert_cmd/trycmd/predicates/tempfile) already installed from Phase 1 — no framework install needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ANSI colors render in PowerShell 7 (dir-name color in `tree`, size accent in `du`) | TREE-01 / DU-01 | Color is terminal-dependent; automated tests assert the `NO_COLOR`/piped path, not live ANSI | Run `box tree ./src` and `box du ./project` in PS7; confirm colored dir names / cyan size accent; confirm `... | cat` (piped) is byte-identical minus ANSI |

*All exit-code and output-content behaviors have automated verification; only live-ANSI rendering is manual.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
