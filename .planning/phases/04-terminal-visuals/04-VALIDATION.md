---
phase: 4
slug: terminal-visuals
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-24
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `04-RESEARCH.md` § Validation Architecture (Nyquist enabled).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` (in-module `#[cfg(test)]` unit) + `assert_cmd` 2.2 / `predicates` 3.1 (integration) + `trycmd` 1.2 (CLI snapshots) |
| **Config file** | none — Cargo convention (`tests/<cmd>.rs`, `tests/cmd/*.trycmd`) |
| **Quick run command** | `cargo test --bin box <module>` — **`--bin box`, NEVER `--lib`** (binary-only crate, STATE.md [03-01]) |
| **Full suite command** | `cargo test` then `cargo clippy -- -D warnings` + `cargo fmt --check` |
| **Estimated runtime** | ~5–15 seconds (unit sub-second; integration dominated by 4 `assert_cmd` spawns) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --bin box <module>` (the touched command's unit tests) — sub-second.
- **After every plan wave:** Run `cargo test` (full unit + integration).
- **Before `/gsd:verify-work`:** `cargo test` green + `cargo clippy -- -D warnings` + `cargo fmt --check` (the established Phase-3 gate).
- **Max feedback latency:** ~15 seconds.

---

## Per-Task Verification Map

> Requirement-level map (task IDs assigned by the planner in §8 — each row maps to the plan
> that owns the requirement). Each command is a vertical MVP slice.

| Requirement | Behavior | Test Type | Automated Command | File | File Exists |
|-------------|----------|-----------|-------------------|------|-------------|
| LOL-01 | `rgb_at` gradient math (known phase→RGB, 120° spacing, floor 128) | unit | `cargo test --bin box lolcat` | `src/commands/lolcat/mod.rs` `#[cfg(test)]` | ❌ W0 |
| LOL-01 | piped → clean plain text, byte-identical minus ANSI; multi-byte UTF-8 intact | integration | `cargo test --test lolcat` | `tests/lolcat.rs` | ❌ W0 |
| LOL-01 | strip pre-existing ANSI before recolor (`strip_str` on `\x1b[31mx\x1b[0m` → `x`) | unit | `cargo test --bin box lolcat` | `src/commands/lolcat/mod.rs` | ❌ W0 |
| MTRX-01 | drop/fade model + glyph table (head advance, trail fade green→dark, reset on clear-bottom, katakana U+FF66–FF9D all width-1) | unit | `cargo test --bin box matrix` | `src/commands/matrix/mod.rs` `#[cfg(test)]` | ❌ W0 |
| MTRX-01 | exits cleanly + restores (no artifacts) — enter/exit contract | smoke (`assert_cmd`, non-TTY stdin or `q` feed → starts, exits non-hanging) | `cargo test --test matrix` | `tests/matrix.rs` | ❌ W0 |
| ASCI-01 | `luma_to_char` ramp (0→darkest, 255→lightest, monotonic) + `rows = cols*sh/sw/2` | unit | `cargo test --bin box ascii` | `src/commands/ascii/mod.rs` `#[cfg(test)]` | ❌ W0 |
| ASCI-01 | renders a real PNG + a real JPEG; missing/bad file → exit 1, no panic | integration | `cargo test --test ascii` | `tests/ascii.rs` + `tests/cmd/ascii.in/{tiny.png,tiny.jpg}` | ❌ W0 |
| JSON-01 | invalid → exit 1 + line/col on stderr, stdout empty | integration | `cargo test --test json` | `tests/json.rs` | ❌ W0 |
| JSON-01 | valid pretty (2-space) + `--compact` minify; piped → no ANSI, byte-identical minus color | integration + unit | `cargo test --test json` + `cargo test --bin box json` | `tests/json.rs` + module | ❌ W0 |
| JSON-01 | preserve key order (`{"b":1,"a":2}` → `b` before `a`) | unit | `cargo test --bin box json` | `src/commands/json/mod.rs` | ❌ W0 |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/lolcat.rs` — LOL-01 piped-plain + ANSI-strip + UTF-8 integration
- [ ] `tests/json.rs` — JSON-01 invalid / valid / `--compact` / key-order
- [ ] `tests/ascii.rs` + `tests/cmd/ascii.in/{tiny.png,tiny.jpg}` — ASCI-01 PNG+JPEG fixtures (tiny, checked-in)
- [ ] `tests/matrix.rs` — MTRX-01 enters/exits-cleanly smoke (feed `q` or non-TTY)
- [ ] In-module `#[cfg(test)]` blocks for the four pure helpers (`rgb_at`, `luma_to_char`, drop/fade + glyph table, json colorize/order)
- [ ] Optional `tests/cmd/json.trycmd` for the locked 2-space pretty layout (trycmd suits json; NOT lolcat ANSI or matrix)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Full-terminal green digital-rain renders and fills terminal width | MTRX-01 | Animation/raw-mode loop is not auto-snapshotable; the testable invariant is the enter/exit contract (covered above) | Run `box matrix` in PS7; confirm rain fills width, head bright + green→dark trail; press Ctrl+C / `q` / Esc → cursor restored, no leftover glyphs |
| Authentic katakana glyphs vs tofu | MTRX-01 | Depends on the user's terminal font (CJK-capable); documented cosmetic limitation, not a bug (D-07) | Run `box matrix` with a CJK font (e.g. Cascadia Next JP); on bare Cascadia Mono glyphs show as tofu — expected |
| Smooth truecolor rainbow + diagonal in a real TTY | LOL-01 | Visual smoothness/diagonal is a perceptual property; byte-level behavior is auto-tested | Pipe multi-line text through `box lolcat` in PS7; confirm smooth diagonal rainbow |
| ASCII art visually resembles the source image | ASCI-01 | Perceptual fidelity is subjective; exit-code + non-empty output + ramp monotonicity are auto-tested | `box ascii ./photo.jpg` — confirm recognizable, width-fitted, not vertically stretched |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
