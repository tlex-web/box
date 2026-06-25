---
phase: 7
slug: spine-rollout
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-25
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `07-RESEARCH.md` → "## Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `assert_cmd` 2.2 (black-box binary) + `predicates` 3.1; `tempfile`/`assert_fs` for fs commands; `trycmd` 1.2 (cowsay transcript); `insta` 1.48 available |
| **Config file** | `Cargo.toml` `[dev-dependencies]` (no separate test config — all present, no install) |
| **Quick run command** | `cargo test --test <cmd>` (one command's integration tests) |
| **Full suite command** | `cargo test` (all integration tests) + `cargo test --bin box` (in-crate unit tests — `box` is binary-only, NOT `--lib`) |
| **Estimated runtime** | ~5s per command file; full suite well under a minute |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test <cmd>` for the command(s) touched in that task (< 5s each)
- **After every plan wave (7a/7b/7c):** Run `cargo test` (all integration) + `cargo test --bin box` (unit) — full suite green before merge
- **Before `/gsd:verify-work`:** Full suite green (`cargo test && cargo test --bin box`)
- **Max feedback latency:** < 5 seconds (per-command quick run)

---

## Per-Task Verification Map

> Task IDs are assigned by the planner (this VALIDATION.md is created pre-plan). The
> requirement→test mapping below is authoritative; `validate-phase` / the Nyquist
> auditor binds each row to a concrete `{N}-{plan}-{task}` ID after planning.

| Requirement | Command | Secure Behavior | Test Type | Automated Command | File Exists |
|-------------|---------|-----------------|-----------|-------------------|-------------|
| SPINE-02 | base64 | non-UTF-8 decode handled (no panic) | integration | `cargo test --test base64 json_purity` | ✅ (add test) |
| SPINE-02 | epoch | N/A | integration | `cargo test --test epoch json_purity` | ✅ |
| SPINE-02 | color | N/A | integration | `cargo test --test color json_purity` | ✅ |
| SPINE-02 | passgen | N/A | integration | `cargo test --test passgen json_purity` + `json_count_multi` | ✅ |
| SPINE-02 | 8ball | N/A | integration | `cargo test --test eight_ball json_purity` | ✅ |
| SPINE-02 | fortune | N/A | integration | `cargo test --test fortune json_purity` | ✅ |
| SPINE-02 | roast | N/A | integration | `cargo test --test roast json_purity` | ✅ |
| SPINE-02 | cowsay | N/A | integration | `cargo test --test cowsay json_purity` | ❌ W0 (new tests/cowsay.rs) |
| SPINE-02 | du | non-UTF-8 path via `to_string_lossy` | integration | `cargo test --test du json_purity` | ✅ |
| SPINE-02 | tree | non-UTF-8 path via `to_string_lossy` | integration | `cargo test --test tree json_purity` + `json_recursive_shape` | ✅ |
| SPINE-02 | dupes | N/A | integration | `cargo test --test dupes json_purity` | ✅ |
| SPINE-02 | flatten | `dry_run` bool honored | integration | `cargo test --test flatten json_dry_run` + `json_force_run` | ✅ |
| SPINE-02 | bulk-rename | abort keeps stdout EMPTY (D-09) | integration | `cargo test --test bulk_rename json_dry_run` + `json_force_emits_rows` + `json_abort_empty_stdout` | ✅ |
| SPINE-02 | json | identity passthrough, not wrapped | integration | `cargo test --test json json_identity_passthrough` | ✅ |
| SPINE-02 | qr | metadata not glyphs | integration | `cargo test --test qr json_metadata_not_glyphs` | ✅ |
| SPINE-02 | weather | offline seam `BOX_WEATHER_BASE_URL` | integration | `cargo test --test weather json_purity` | ✅ |
| SPINE-04 | base64 | clip copies printed result | integration (`#[ignore]`) | `cargo test --test base64 -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | color | clip copies printed block | integration (`#[ignore]`) | `cargo test --test color -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | epoch | clip copies result | integration (`#[ignore]`) | `cargo test --test epoch -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | passgen | clip puts secret on clipboard (opt-in) | integration (`#[ignore]`) | `cargo test --test passgen -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | json | clip copies pretty/compact JSON | integration (`#[ignore]`) | `cargo test --test json -- --ignored clip_roundtrip` | ✅ |
| SPINE-04 | qr | **clip copies SOURCE TEXT, not glyphs (D-15)** | integration (`#[ignore]`) | `cargo test --test qr -- --ignored clip_copies_source_text` | ✅ |
| SPINE-02/04 (SC4) | matrix/pomodoro/lolcat/ascii/clip | `--json`/`--clip` emit NO JSON document | integration | `cargo test --test cli` (assert display-only commands do not emit JSON to stdout) | ✅ |

*Status legend: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/cowsay.rs` — NEW file (cowsay has only `tests/cmd/*.trycmd` + in-source unit tests; needs an `assert_cmd` `json_purity` test) — covers SPINE-02 for cowsay
- [ ] Per-command `json_purity` test added to each existing `tests/<cmd>.rs` (15 files), copied from `tests/uuid.rs:135`, adapted to each command's locked shape
- [ ] Per-command `clip_roundtrip` (`#[ignore]`d) added to the 6 SPINE-04 `tests/<cmd>.rs`, copied from `tests/uuid.rs:237`; qr's variant asserts pasted == input (D-15)
- [ ] `tree` recursive-shape test (`json_recursive_shape`) — asserts `children` nesting + `type`/`size?` per D-17
- [ ] `flatten`/`bulk-rename` dual tests (`json_dry_run` + `json_force_run`/`json_force_emits_rows` + `json_abort_empty_stdout`) — D-12/D-13 plan-vs-result + D-09 abort-empty-stdout fork
- [ ] `weather` `--json` test using the `BOX_WEATHER_BASE_URL` offline seam (`weather/mod.rs:36`) + a forecast fixture (`tests/fixtures/weather/`) — network-free, deterministic
- [ ] `core::output` unit test IF a `clip_feed(&str)` primitive is added for qr (D-15) — assert it tees to `CLIP_BUF` only under `--clip`, no stdout write (mirrors `out_line_tees` at `src/core/output.rs:491`)

*Framework is fully present (`assert_cmd`/`predicates`/`tempfile`/`assert_fs`/`trycmd`/`insta` in `[dev-dependencies]`) — no install needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live `--clip` round-trips actually hit the OS clipboard | SPINE-04 | Clipboard is shared OS state; CI/headless cannot assert paste reliably | Run once locally: `cargo test -- --ignored --test-threads=1`, then paste in a PS7 window to confirm |
| `passgen --clip` confirmation goes to stderr, secret to clipboard | SPINE-04 | Visual confirmation that the secret is not echoed where a script would capture it | `box passgen --clip` then inspect clipboard + confirm "Copied" line is on stderr |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (cowsay test file)
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
