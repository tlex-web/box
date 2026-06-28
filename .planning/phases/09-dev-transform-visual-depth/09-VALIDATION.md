---
phase: 9
slug: dev-transform-visual-depth
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-28
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Per-requirement test seams are detailed in `09-RESEARCH.md` § Validation Architecture;
> each PLAN.md embeds the concrete `<acceptance_criteria>` per task. This file is the
> sampling contract; the Per-Task Verification Map is reconciled during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness + `assert_cmd` / `trycmd` / `insta` / `assert_fs` / `predicates` / `tempfile` (all already dev-deps) |
| **Config file** | none — `[dev-dependencies]` in `Cargo.toml` |
| **Quick run command** | `cargo test --bin box` (binary-only crate — NOT `--lib`) |
| **Full suite command** | `cargo test` then `cargo clippy --all-targets -- -D warnings` (both must be green) |
| **Estimated runtime** | ~30–90 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --bin box <changed-seam>` (the fast pure-fn subset), plus the touched command's integration file (e.g. `cargo test --test qr`)
- **After every plan wave:** Run the affected `cargo test --test <cmd>` integration files
- **Before `/gsd:verify-work`:** Full `cargo test` green + `cargo clippy --all-targets -- -D warnings` clean, THEN the LOL-V2-01 PS7 human-verify
- **Max feedback latency:** ~90 seconds

---

## Per-Task Verification Map

> Filled per task during planning/execution. Source of truth for seams: `09-RESEARCH.md` § Validation Architecture.
> Every new colored path (`matrix --color`, `ascii` truecolor, animated `lolcat`) MUST carry a
> **byte-identical-minus-ANSI** piped-purity assertion (SC4) using `cli.rs::display_only_omit_json` as the template.

### Phase Requirements → Test Map (from RESEARCH § Validation Architecture)

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UUID-V2-01 | v7 version nibble; each wrapping form's shape; `--upper` composes; `--braces`/`--urn` conflict → exit 2 | unit + integration | `cargo test --bin box format_one` / `cargo test --test uuid` | ✅ uuid.rs |
| EPOC-V2-01 | `relative_for` FORMAT (just now / N hr / in N days); `%Z %z` for a known zone; bad `--tz` → exit 1; bare `now`/date-string stay integer | unit + integration | `cargo test --bin box relative_for` / `cargo test --test epoch` | ✅ epoch.rs |
| COLR-V2-01 | `hsl(...)`→rgb round-trip ±1; CSS anchors (`black`/`white`/`rebeccapurple`/`cornflowerblue`); exact `name` vs `~nearest` (redmean); JSON `{name,nearest}` always-present | unit + integration | `cargo test --bin box hsl` / `cargo test --test color` | ✅ color.rs |
| JSON-V2-01 | `--sort-keys` recursively sorts (nested) & arrays keep order; plain `box json` preserves order (unchanged); `--json --sort-keys` sorted | unit + integration | `cargo test --bin box sort_value` / `cargo test --test json` | ✅ json.rs |
| PASS-V2-01 | entropy_bits = `len*log2(pool)` / `words*12.925`; `--no-similar` drops `il1Lo0O` & recomputes pool; `--separator` join; entropy on STDERR not stdout | unit + integration | `cargo test --bin box entropy` / `cargo test --test passgen` | ✅ passgen.rs |
| MTRX-V2-01 | preset→HEAD/FADE RGB mapping; speed level→poll; charset preset/custom→glyph set (pure helpers) | unit | `cargo test --bin box matrix` | ✅ matrix.rs (+ display-only omit) |
| QR-V2-01 | `render_qr` honors each `EcLevel`; `--save out.png` writes a non-empty PNG; `--save out.svg` writes SVG; bad extension → exit 1; glyphs suppressed under `--save`; `saved_path` in JSON | unit + integration (`assert_fs` temp dir) | `cargo test --bin box render_qr` / `cargo test --test qr` | ✅ qr.rs |
| ASCI-V2-01 | braille bit-order (locked); `--invert` = `255-luma` at the seam; truecolor present in TTY / **byte-identical-minus-ANSI when piped** (SC4) | unit + integration (fixture img) | `cargo test --bin box braille` / `cargo test --test ascii` | ✅ ascii.rs (+ fixtures/) |
| LOL-V2-01 | **Automatable:** `rgb_at(phase,freq)` channel bounds + freq/seed effect; piped/`--json` does NOT enter raw mode (no `0x1B`, static, byte-identical); non-hanging smoke. **Human-gated:** smooth animation, clean Ctrl+C/q/Esc restore, visible `--freq`/`--seed` change — PS7 only. | unit + integration + **HUMAN-VERIFY** | `cargo test --bin box rgb_at` / `cargo test --test lolcat` + **manual PS7 checkpoint** | ✅ lolcat.rs |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · ❌ W0 = test fixture/seam created in this plan's Wave-0 task*

---

## Wave 0 Requirements

*None — existing test infrastructure (Rust harness + dev-deps + per-command unit+integration files + `fixtures/`) covers all 9 requirements. New tests are additive to the files listed above.*

Reminders for the executor:
- `assert_fs` / `tempfile` are the right tools for the `qr --save` file-write assertion (assert non-empty + correct magic bytes).
- `cli.rs::display_only_omit_json` is the SC4 template for the `ascii` / `matrix` / `lolcat` piped-purity (byte-identical-minus-ANSI) assertions.
- Confirm assumption **A1** with a cheap unit test: assert a known zone's `%Z` renders the abbreviation (e.g. `Asia/Tokyo` → `JST`) under `chrono 0.4.45` × `chrono-tz 0.10`. If `%Z` is blank, only the numeric offset shows — cosmetic, not a blocker.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `lolcat --animate` smooth ~20-FPS rainbow + clean terminal restore | LOL-V2-01 (SC3) | Automated tests prove the loop NEVER runs off-TTY and that the gradient math changes with `--seed`; they cannot prove the on-screen animation is smooth or that raw mode restores cleanly | In PS7: `box lolcat "Hello World" --animate` (MUST pass text — `read_input` exits 2 on a no-arg interactive TTY). Confirm: smooth rainbow; Ctrl+C / `q` / `Esc` each exit cleanly with no stuck raw mode; `--freq`/`--seed` visibly change the gradient; final colored frame persists; `box lolcat "x" --animate \| cat` degrades to a static render |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references *(none this phase)*
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter
- [ ] LOL-V2-01 PS7 human-verify cleared

**Approval:** pending
