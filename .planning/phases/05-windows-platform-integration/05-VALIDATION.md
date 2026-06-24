---
phase: 5
slug: windows-platform-integration
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-24
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `05-RESEARCH.md` § Validation Architecture (Nyquist enabled).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` (in-module `#[cfg(test)]` unit) + `assert_cmd` 2.2 / `predicates` 3.1 (integration) + `trycmd` 1.2 (CLI snapshots) |
| **Config file** | none — Cargo convention (`tests/<cmd>.rs`, `tests/cmd/*.trycmd`, `tests/cmd/<cmd>.in/` fixtures) |
| **Quick run command** | `cargo test --bin box <module>` — **`--bin box`, NEVER `--lib`** (binary-only crate, STATE.md [03-01]) |
| **Full suite command** | `cargo test` then `cargo clippy -- -D warnings` + `cargo fmt --check` |
| **Estimated runtime** | ~5–15 seconds (unit sub-second; integration dominated by `assert_cmd` spawns; `tests/clip.rs` serialized) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --bin box <module>` (the touched command's unit tests) — sub-second.
- **After every plan wave:** Run `cargo test` (full unit + integration). `tests/clip.rs` touches the shared OS clipboard — keep it to a single test fn or run that file with `--test-threads=1` to avoid cross-test races.
- **Before `/gsd:verify-work`:** `cargo test` green + `cargo clippy -- -D warnings` + `cargo fmt --check` (the established Phase-3/4 gate).
- **Max feedback latency:** ~15 seconds.

---

## Per-Task Verification Map

> Requirement-level map (task IDs assigned by the planner in §8 — each row maps to the plan
> that owns the requirement). Each command is a vertical MVP slice.

| Requirement | Behavior | Test Type | Automated Command | File | File Exists |
|-------------|----------|-----------|-------------------|------|-------------|
| QR-01 | encode arg/stdin → owned `String` of pure half-blocks, NO ANSI (`render::<Dense1x2>().quiet_zone(true).build()` non-empty; assert no `\x1b`) | unit | `cargo test --bin box qr` | `src/commands/qr/mod.rs` `#[cfg(test)]` | ❌ W0 |
| QR-01 | `box qr "https://example.com"` (arg + piped stdin) exits 0, output contains a half-block glyph `▀`/`▄`/`█`, no `\x1b[` when piped | integration | `cargo test --test qr` | `tests/qr.rs` | ❌ W0 |
| QR-01 | stable rendered layout for a fixed input (quiet-zone + polarity locked) | snapshot (optional — only if half-block layout/polarity proves byte-stable across machines) | `cargo test --test cli` (trycmd) | `tests/cmd/qr.trycmd` | ❌ W0 (optional) |
| CLIP-01 | copy strips AT MOST ONE trailing `\r\n`/`\n`, never interior/multiple (`b"x\r\n"`→`b"x"`, `b"x\n"`→`b"x"`, `b"x\n\n"`→`b"x\n"`, `b"a\nb"`→`b"a\nb"`) | unit | `cargo test --bin box clip` | `src/commands/clip/mod.rs` `#[cfg(test)]` | ❌ W0 |
| CLIP-01 | round-trip copy→paste preserves Unicode byte-exact (`héllo 世界`), no elevation; paste = input minus the one trailing newline | integration (serialize: shared OS clipboard — single test or `--test-threads=1`) | `cargo test --test clip` | `tests/clip.rs` | ❌ W0 |
| CLIP-01 | non-UTF-8 stdin on copy (`\xff\xfe`) → clean exit 1, stderr message, no panic | integration | `cargo test --test clip` | `tests/clip.rs` | ❌ W0 |
| POMO-01 | duration resolve: default 25, `--break` 5, `--long-break` 15, `[MINUTES]` override (pure flag/arg → seconds) | unit | `cargo test --bin box pomodoro` | `src/commands/pomodoro/mod.rs` `#[cfg(test)]` | ❌ W0 |
| POMO-01 | countdown `MM:SS` format + per-tick `remaining` math (`fmt_mmss(0)="00:00"`, `fmt_mmss(65)="01:05"`, `fmt_mmss(1500)="25:00"`; remaining-tick clamp, no drift) | unit | `cargo test --bin box pomodoro` | same `#[cfg(test)]` | ❌ W0 |
| POMO-01 | quit predicate: Press-only `Ctrl+C`/`q`/`Esc`; Release/Repeat ignored; bare `c` not quit (reuse matrix `is_quit`-shape tests) | unit | `cargo test --bin box pomodoro` | same `#[cfg(test)]` | ❌ W0 |
| POMO-01 | a very short / test-tiny timer exits cleanly + restores terminal (non-TTY/piped stdin → starts, exits non-hanging, exit 0, no panic) | smoke (`assert_cmd`) | `cargo test --test pomodoro` | `tests/pomodoro.rs` | ❌ W0 |
| WTHR-01 | lat/lon parse-shape: matches `lat,lon` in range only, else city (`"51.5,-0.13"`=Some, `"London"`=None, `"91,0"`=None out of range, `"1,2,3"`=None) | unit | `cargo test --bin box weather` | `src/commands/weather/mod.rs` `#[cfg(test)]` | ❌ W0 |
| WTHR-01 | WMO `weather_code` → `&'static str` incl. `_`→"Unknown" (`wmo_to_str(0)="Clear sky"`, `wmo_to_str(95)="Thunderstorm"`, `wmo_to_str(9999)="Unknown"`) | unit | `cargo test --bin box weather` | same `#[cfg(test)]` | ❌ W0 |
| WTHR-01 | Deserialize live forecast + geocoding shapes; no-results geocode (absent `results` key) → "no location found" (`#[serde(default)]`) | unit (`serde_json::from_str` over checked-in fixtures) | `cargo test --bin box weather` | same `#[cfg(test)]` + fixtures | ❌ W0 |
| WTHR-01 | `--units imperial` appends `temperature_unit=fahrenheit&wind_speed_unit=mph`; default omits them | unit (pure URL-builder) | `cargo test --bin box weather` | same `#[cfg(test)]` | ❌ W0 |
| WTHR-01 | offline / non-2xx → graceful stderr error + exit 1 (NOT exit 2); bad `--units` value → exit 2 | integration (`assert_cmd`: bogus host via test seam OR offline → exit 1, stderr "offline"/"service") | `cargo test --test weather` | `tests/weather.rs` | ❌ W0 |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/qr.rs` — QR-01 arg + piped render, half-block present, no-ANSI-when-piped
- [ ] `tests/clip.rs` — CLIP-01 round-trip Unicode (single-threaded), newline trim, non-UTF-8 exit-1
- [ ] `tests/pomodoro.rs` — POMO-01 short-timer enters/exits-cleanly smoke (non-TTY)
- [ ] `tests/weather.rs` — WTHR-01 offline/non-2xx exit-1 + bad-`--units` exit-2
- [ ] In-module `#[cfg(test)]` blocks for the pure helpers: qr build-to-String, `trim_one_trailing_newline`, pomodoro duration-resolve + `fmt_mmss` + quit predicate, weather `parse_lat_lon` + `wmo_to_str` + Deserialize-from-fixture + URL-builder
- [ ] Captured Open-Meteo JSON fixtures (forecast metric, forecast imperial, geocode hit, geocode no-match) checked in for the weather Deserialize unit tests (live samples in 05-RESEARCH.md Code Examples are the source)
- [ ] Optional `tests/cmd/qr.trycmd` for a fixed-input render — ONLY if the half-block layout/polarity proves byte-stable across machines (trycmd suits plain text; NOT the interactive pomodoro or the toast)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Rendered QR is scannable by a phone camera from the terminal | QR-01 | Camera-readability is a physical/perceptual property; byte-level (half-blocks present, no ANSI, exit 0) is auto-tested | Run `box qr "https://example.com"` in PS7 on the dark default theme; scan with a phone camera → resolves to the URL. If it fails, flip `.dark_color`/`.light_color` polarity (D-02) and re-test |
| Live in-place countdown renders + a real Windows 11 toast fires on completion + Ctrl+C / `q` / Esc cancels cleanly (no toast, terminal restored) | POMO-01 | Interactive raw-mode loop + WinRT toast are not auto-snapshotable; the testable invariants (duration math, MM:SS, quit predicate, enter/exit smoke) are auto-tested | Run `box pomodoro 1` (or `--break`) in PS7; confirm live `MM:SS` countdown, then a toast titled per D-09 on completion; re-run and press Ctrl+C mid-countdown → "cancelled" to stderr, exit 1, NO toast, cursor restored, no leftover glyphs |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
