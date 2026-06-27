---
phase: 8
slug: filesystem-depth
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-27
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Per-requirement test seams are detailed in `08-RESEARCH.md` § Validation Architecture;
> each PLAN.md embeds the concrete `<acceptance_criteria>` per task. This file is the
> sampling contract; the Per-Task Verification Map is reconciled during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) + `assert_fs` / `predicates` / `assert_cmd` (already dev-deps) |
| **Config file** | none — `[dev-dependencies]` in `Cargo.toml` |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30–90 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib` (plus the touched command's integration tests, e.g. `cargo test --test du`)
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green; `cargo build --release` succeeds
- **Max feedback latency:** ~90 seconds

---

## Per-Task Verification Map

> Filled per task during planning/execution. Source of truth for seams: `08-RESEARCH.md` § Validation Architecture.
> Destructive plans (08-04 `flatten --move`, 08-05 `dupes --delete`, 08-06 `bulk-rename --backup`)
> MUST carry a **snapshot-the-tree-unchanged** assertion for every abort path (Code-review gate).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 8-01-xx | 01 | 1 | HASH-V2-02, FLAT-V2-01 | T-8-01 / — | best-effort multi-file hash; `--json` no progress/ANSI leak | integration | `cargo test --test hash --test flatten` | ❌ W0 | ⬜ pending |
| 8-02-xx | 02 | 1 | TREE-V2-01, DU-V2-01, DU-V2-02 | — | gitignore/exclude filter parity human+JSON; on-disk size correct | integration | `cargo test --test tree --test du` | ❌ W0 | ⬜ pending |
| 8-03-xx | 03 | 1 | DUPE-V2-01, RENM-V2-01 | — | hardlink-collapse never counts shared inode; `{n}` over sorted order | integration | `cargo test --test dupes --test bulk_rename` | ❌ W0 | ⬜ pending |
| 8-04-xx | 04 | 2 | FLAT-V2-02 | T-8-04 | dry-run default; abort leaves source tree byte-identical | integration | `cargo test --test flatten_move` | ❌ W0 | ⬜ pending |
| 8-05-xx | 05 | 2 | DUPE-V2-02 | T-8-05 | keep-≥1; hardlink-safe; abort-all-before-any | integration | `cargo test --test dupes_delete` | ❌ W0 | ⬜ pending |
| 8-06-xx | 06 | 2 | RENM-V2-02 | T-8-06 | manifest fsync'd before first rename; dir recoverable on abort | integration | `cargo test --test bulk_rename_backup` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · ❌ W0 = test fixture/seam created in this plan's Wave-0 task*

---

## Wave 0 Requirements

- [ ] Per-command integration test files (temp-dir fixtures via `assert_fs`) for each new flag's golden behavior — created within each plan's first task
- [ ] `--json` parity assertions (piped output byte-identical-minus-ANSI; no progress on stderr under `--json`) reused across commands
- [ ] Snapshot-the-tree-unchanged helper for the three destructive abort paths (08-04/05/06)

*Existing `cargo test` harness + `assert_fs`/`predicates`/`assert_cmd` dev-deps cover the framework; no framework install needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `du --on-disk` compressed-size correctness on a real NTFS-compressed file | DU-V2-02 | `GetCompressedFileSizeW` returns allocation that depends on live NTFS compression state; hard to fixture deterministically in CI | Mark a file/dir compressed (`compact /c`), run `box du --on-disk` and compare to Explorer's "Size on disk" |
| stderr progress bar appears for large inputs only | HASH-V2-02, FLAT-V2-01 | indicatif timing/terminal behavior; assert presence/absence, not pixels | Run on a large input with a TTY; confirm bar on stderr and absent under `--json` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
