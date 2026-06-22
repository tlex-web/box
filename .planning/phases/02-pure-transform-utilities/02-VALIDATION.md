---
phase: 2
slug: pure-transform-utilities
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-22
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `02-RESEARCH.md` § Validation Architecture. Task IDs are filled by the planner; rows here are keyed by requirement/slice.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness + `assert_cmd` (integration) + `trycmd` (CLI snapshots) + `predicates`. All already in `[dev-dependencies]` (verified) — no Wave 0 framework install. |
| **Config file** | none — Cargo conventions; integration tests in `tests/<cmd>.rs`, unit tests in `#[cfg(test)]` modules, snapshots in `tests/cmd/*.trycmd` |
| **Quick run command** | `cargo test <module>::` (unit) or `cargo test --test <cmd>` (one command's integration suite) |
| **Full suite command** | `cargo test && cargo clippy -- -D warnings && cargo fmt --check` |
| **Estimated runtime** | ~30 seconds (pure logic + lightweight binary spawns) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test <module>::` for the command touched + `cargo clippy -- -D warnings` (Phase-1 quality gate stays).
- **After every plan wave (slice):** Run `cargo test --test <cmd>` (that command's integration suite) + `cargo fmt --check`.
- **Before `/gsd:verify-work`:** Full suite must be green — `cargo test` + `cargo clippy -- -D warnings` clean + `cargo fmt --check` clean. Re-run the `help.trycmd` snapshot (`TRYCMD=overwrite cargo test`) and review the diff if any `--help` text drifted.
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

> Keyed by requirement until the planner assigns task IDs. `Test Type` + `Automated Command` are the binding contract; the planner maps these onto its task breakdown.

| Req / Slice | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| core::input (foundation) | 1 | (shared) | — | TTY + no arg → exit 2, never hang (D-04 br.3) | unit + integration | `cargo test input::` ; per-cmd no-input integration | ❌ W0 | ⬜ pending |
| uuid | 2 | UUID-01 | — | v4 format, lowercase, one/line, `-n N`, uniqueness, `--upper` | integration | `cargo test --test uuid` | ❌ W0 | ⬜ pending |
| base64 | 2 | B64-01 | T-V5 input-validation | byte-exact round-trip, `--url-safe`, decode rejects non-alphabet | unit + integration | `cargo test base64::` ; `cargo test --test base64` | ❌ W0 | ⬜ pending |
| epoch | 2 | EPOC-01 | T-V5 input-validation | 3 modes round-trip, reject ambiguous formats (no panic) | unit + integration | `cargo test epoch::` ; `cargo test --test epoch` | ❌ W0 | ⬜ pending |
| color | 2 | COLR-01 | T-V5 input-validation | hex/RGB + `#abc` parse, HSL round-trip, swatch ANSI-gated | unit + trycmd | `cargo test color::` ; `cargo test --test cli` | ❌ W0 | ⬜ pending |
| passgen | 2 | PASS-01 | **T-V6 crypto** | OsRng CSPRNG + unbiased selection (no `% len`), curated paste-safe charset, stdout-only | unit + integration + **code review** | `cargo test passgen::` ; `cargo test --test passgen` | ❌ W0 | ⬜ pending |
| cowsay | 2 | COW-01 | — | single/multi-line bubble, hard-break long word, arg+stdin | unit + trycmd | `cargo test cowsay::` ; trycmd snapshot | ❌ W0 | ⬜ pending |
| fortune | 2 | FORT-01 | — | output ∈ list, non-empty, varies across N runs | integration | `cargo test --test fortune` | ❌ W0 | ⬜ pending |
| 8ball | 2 | 8BAL-01 | — | output ∈ the 20 answers, question optional, varies across N runs | unit + integration | `cargo test eight_ball::` ; `cargo test --test eight_ball` | ❌ W0 | ⬜ pending |
| roast | 2 | ROST-01 | — | output ∈ list, varies across N runs | integration | `cargo test --test roast` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Non-determinism anti-flake rule (fortune / 8ball / roast / uuid / passgen)
Each `box` run is a fresh OS-seeded process (D-08, no fixed seed). Tests assert **properties, not values**:
- **Membership** — output ∈ a known set (expose the parsed list via `pub(crate) fn entries() -> &'static [&str]`).
- **Varies-across-runs** — run N=10 times, assert `HashSet::len() >= 2`. (For 8ball, P(all 10 identical) ≈ 2e-12.) Never compare just two runs (1/20 collision for 8ball).
- **uuid uniqueness** — `box uuid -n 100` → 100 distinct lines matching `^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`.

### Crypto-sampling split (passgen — T-V6)
"Unbiased CSPRNG selection" is a **construction guarantee, not an output assertion**:
- **Automated (tests):** length, charset membership, `--no-symbols` exclusion, `--words N` count + EFF-list membership, wordlist size == 7776, `--count N` distinct.
- **Code-review gate (NOT a flaky statistical test):** grep that no `% len` appears near selection; grep that the RNG is `OsRng`-backed (D-02/D-08). A statistical chi-square test is explicitly NOT the gate.

---

## Wave 0 Requirements

- [ ] `tests/uuid.rs` — UUID-01 (format regex, count, uniqueness, `--upper`)
- [ ] `tests/base64.rs` — B64-01 (round-trip bytes, url-safe, stdin)
- [ ] `tests/epoch.rs` — EPOC-01 (3 modes, no-arg, integer)
- [ ] `tests/passgen.rs` — PASS-01 (length, charset, no-symbols, words, count distinct, stdout-only)
- [ ] `tests/fortune.rs`, `tests/eight_ball.rs`, `tests/roast.rs` — membership + varies-across-runs
- [ ] `tests/cowsay.rs` + `tests/cmd/cowsay*.trycmd` — bubble layouts + hard-break (unit) + snapshot
- [ ] `tests/cmd/color*.trycmd` — locked color block layout under `NO_COLOR=1`; parse/HSL unit-tested in-module
- [ ] `core::input` unit tests (decision logic with injected `is_tty`) in `src/core/input.rs` `#[cfg(test)]`
- [ ] Framework install: **none** — `assert_cmd`/`predicates`/`trycmd`/`assert_fs`/`tempfile` already present.
- [ ] Add `rand_core` as a direct dep **only if** `OsRng.unwrap_err()` (`rand::TryRngCore`) does not resolve through `rand` 0.9 re-exports (verify with one build check first).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| PS7 pipe byte-exactness | B64-01 (D-06) | `assert_cmd::write_stdin` bypasses the shell — it proves the Rust reader is byte-exact but does NOT reproduce PS7's UTF-16 pipeline re-encoding | In a real PowerShell 7: `Get-Content -AsByteStream file \| box base64` then decode and compare bytes. (Sidestepped long-term by the deferred `--file PATH` path in Phase 3.) |
| passgen paste-safety in PS7 | PASS-01 (D-14) | Whether a generated password pastes cleanly is a shell-interaction property | Generate several `box passgen`, paste into a PS7 prompt as a quoted string, confirm no quoting breakage. |

---

## Validation Sign-Off

- [ ] All requirements have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] passgen crypto split documented (automated structural + code-review bias/CSPRNG gate)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
