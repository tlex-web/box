# Phase 5: Windows Platform Integration - Research

**Researched:** 2026-06-24
**Domain:** Four highest-external-risk `box` commands — terminal QR render, Windows clipboard, Pomodoro timer + Windows toast, keyless weather — on the existing Phase-1 `core` (PowerShell 7 / `x86_64-pc-windows-msvc` / `+crt-static`)
**Confidence:** HIGH

## Summary

This phase is **confirmation + test-map, not design.** `05-CONTEXT.md` already locked 14 decisions (D-00..D-13) via dedicated advisor research — exact crate versions, the qrcode `Dense1x2` render chain, the arboard single-shot copy/paste flow, the matrix-derived blocking pomodoro loop, the Open-Meteo endpoints + field maps, and every error/exit policy. Two of those decisions deliberately **override** CLAUDE.md crate picks (D-01: `qrcode` replaces `qr2term`; D-09: `tauri-winrt-notification` replaces `winrt-notification`) — both overrides stand and are honored here, not relitigated.

The job here was to (1) confirm all four new crate versions still resolve on crates.io and pass slopcheck, (2) verify the exact API surface the locked decisions depend on (qrcode `Dense1x2` builder method names + `Pixel::Image = String`; arboard `Clipboard::{new, set_text, get_text}` signatures; tauri-winrt-notification `Toast` builder + `POWERSHELL_APP_ID`; ureq 3.x `Body::into_reader` + the `Error` variants that distinguish offline from non-2xx), (3) capture the **live** Open-Meteo forecast + geocoding JSON shapes so the planner can write precise `#[derive(Deserialize)]` structs, and (4) produce the mandatory Validation Architecture / test map (Nyquist enabled) that drives `05-VALIDATION.md`.

**Every locked API was verified against docs.rs for the exact pinned version and the crates.io sparse index — zero drift, zero deprecation.** All four new crates pass slopcheck `status: OK` (crates.io ecosystem, zero flags) and are canonical (kennytm / 1Password-tauri / algesten). The Open-Meteo shapes were pulled from the live API, surfacing three landmines the planner must encode: (a) the no-results geocoding response **omits the `results` key entirely** (not an empty array); (b) Open-Meteo labels the imperial wind unit **`"mp/h"`, not `"mph"`** — exactly why D-11 reads the authoritative label from `current_units`; (c) ureq 3.x returns **`Err(Error::StatusCode)` on non-2xx by default**, so D-13's status-error path is a `match` arm, never a post-success `resp.status()` check.

**Primary recommendation:** Implement each command as one thin vertical slice exactly per its locked decision block, sequenced one-per-wave (all four share `cli.rs`/`main.rs`/`commands/mod.rs` so they cannot run in the same wave — same as Phases 3/4). Trim default features on the three new crates that pull image/svg/png by default (qrcode, arboard) since this phase needs neither image output nor image clipboard. Unit-test the four pure pieces (qr build-to-String, clip newline-trim, weather lat/lon parse + WMO match + Deserialize, pomodoro countdown/format math); human-verify the two irreducibly-interactive pieces (pomodoro live countdown + real toast + Ctrl+C cancel; qr phone-scan).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| QR-01 | Render a scannable QR for any text/URL in the terminal using Unicode half-blocks; input as arg or stdin | `qrcode` 0.14.1 `Dense1x2` chain verified → owned `String`, no ANSI (Standard Stack, API Verification, Code Examples); input via `core::input::read_input` (D-03); dark-on-light polarity verify step (Pitfall QR-1) |
| CLIP-01 | Pipe stdin → clipboard; read clipboard → stdout; correct Unicode on Windows, no elevation | `arboard` 3.6.1 `set_text`/`get_text` signatures verified (API Verification); single-shot main-thread flow persists via OS handle ownership (D-06); single-trailing-newline trim (D-05); raw-stdin read, NOT `core::input` (D-04) |
| POMO-01 | In-place countdown, break/long-break modes, clean Ctrl+C cancel, Windows toast on completion | matrix `RawGuard` + `event::poll`-as-timer reuse verified against `src/commands/matrix/mod.rs` (Architecture Patterns); `tauri-winrt-notification` 0.7.2 `Toast` + `POWERSHELL_APP_ID` verified (API Verification); 25/5/15 defaults (D-08); cancel→exit 1 no-toast / completion→toast+exit 0 (D-07) |
| WTHR-01 | City-name or lat/lon weather (temp/conditions/wind/humidity) via keyless API, metric/imperial, graceful offline | `ureq` 3.3.0 (rustls default, no key) + live Open-Meteo forecast + geocoding JSON shapes captured (API Verification, Code Examples); server-side units + `current_units` label (D-11); WMO `match`→`&'static str` (D-13); offline vs non-2xx error split via ureq `Error` variants (Pitfall WTHR-1) |
</phase_requirements>

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Shared / carry-forward (D-00):** Every command follows the established slice — swap the unit variant in `src/cli.rs` for a real `Args` struct + `RunCommand` impl, delete its `not_implemented(...)` arm in `src/main.rs`, all styled output through `core::output::is_color_on()` (no parallel color path / no `owo_colors::set_override`), `data→stdout` / `messages→stderr`, exit `0/1/2`, ship `assert_cmd` (+ `trycmd` where snapshotable) tests. `box` is binary-only → unit tests via `cargo test --bin box`.

**qr (D-01/D-02/D-03):**
- D-01: Engine = `qrcode` 0.14.1 + its built-in `qrcode::render::unicode::Dense1x2` renderer, NOT `qr2term` 0.3.3 (OVERRIDES CLAUDE.md — `qr2term` always emits ANSI and encodes polarity in the background color, breaking the glyph-is-truth + byte-identical-minus-ANSI invariant). `Dense1x2` renders pure Unicode half-blocks, no ANSI, returns an owned `String` via `.build()`, exposes quiet-zone + polarity control.
- D-02: Render config = `.quiet_zone(true)` (full ISO-18004 4-module border) + explicit dark-on-light polarity (dark modules as the "on" glyph so contrast is independent of the dark PS theme — the #1 reason terminal QRs fail to scan). `EcLevel::M` (15%, the qrcode default) FIXED for v1. API: `QrCode::with_error_correction_level(input.as_bytes(), EcLevel::M)?.render::<Dense1x2>().quiet_zone(true).build()`. Verify `Dense1x2`'s default polarity is dark-on-light during impl; flip `.dark_color`/`.light_color` only if a quick phone-scan test fails.
- D-03: Monochrome, no color path. Input via `core::input` (positional arg / `-` stdin / `--file`). `box qr … > file.txt` yields clean half-block text.

**clip (D-04/D-05/D-06):**
- D-04: Mode = `--paste` flag, copy-by-default. No flag = stdin → clipboard; `box clip --paste` = clipboard → stdout. `clip` does NOT route through `core::input::read_input` (that would inherit the no-arg-TTY → exit-2 contract, wrong for a clipboard copy). Copy reads raw stdin bytes directly (`io::stdin().read_to_end`) so it controls UTF-8 validation + newline policy itself.
- D-05: Trailing-newline policy — on copy strip at most one trailing terminator (`\r\n`, then a lone `\n`), never interior/multiple; on `--paste` emit clipboard text byte-exact. PowerShell 7 appends an implicit CRLF when piping a string to a native command, so byte-verbatim copy would land a spurious trailing newline. Document the single-trim in `--help`; `--raw`/`--no-trim` opt-out deferred.
- D-06: Crate = `arboard` 3.6.1 (locked). Windows persistence is SAFE — `set_text` does `OpenClipboard → EmptyClipboard → SetClipboardData(CF_UNICODETEXT) → CloseClipboard` in one call; after `SetClipboardData` the system owns the handle, so copied text survives process exit. The "contents drop when last `Clipboard` drops" problem is X11/Wayland-specific, does NOT apply on Windows. The STATE.md "arboard main-thread only" pitfall is satisfied by the synchronous single-shot flow: create the `Clipboard` at the point of use, do one op, return; never spawn the arboard call onto a worker thread.

**pomodoro (D-07/D-08/D-09):**
- D-07: Timer model = blocking foreground raw-mode loop, reusing the matrix `RawGuard` pattern. `crossterm` raw mode; `event::poll(remaining_tick)` is BOTH the ~1s countdown timer AND the keypress reader (single-threaded, no background thread, no SIGINT race). Deterministically distinguishes CANCEL (Ctrl+C as `KeyEvent` `Char('c')+CONTROL` in raw mode — NOT SIGINT — or `Esc`/`q` → "cancelled" to stderr + exit 1, NO toast) from COMPLETION (loop falls through → fire toast + exit 0). RAII `Drop` guard restores cursor + disables raw mode on every path incl. panic; filter `KeyEventKind::Press` only. In-place countdown redraw via a single clean write per tick; reuse `terminal_width()`, stay pipe-safe.
- D-08: Flags/durations — `box pomodoro [MINUTES]` defaults to 25 (work); `--break` → 5; `--long-break` → 15. A positional minutes override is in scope; session counter / auto-break / sound / `--label` are SYS-V2-01 (out).
- D-09: Toast crate = `tauri-winrt-notification` 0.7.2 — the maintained drop-in fork of `winrt-notification` (OVERRIDES CLAUDE.md). API identical: `Toast::new(Toast::POWERSHELL_APP_ID).title("Pomodoro").text1("Break time!").show()`. `POWERSHELL_APP_ID` means no AUMID registration. Both fork and stale crate compile clean on rustc 1.90 / msvc — the axis is maintenance, fork wins (modern `windows 0.61`). Fallback ladder (same `ToastNotificationManager` API → swap localized to the pomodoro module): → `winrt-notification` 0.5.1 → `winrt-toast-reborn` 0.3.8 → PowerShell `New-BurntToastNotification` shell-out (last resort).

**weather (D-10/D-11/D-12/D-13):**
- D-10: Default units = metric (°C, km/h); `--units metric|imperial` as a clap `ValueEnum` explicit override. Deterministic + testable (no env/locale reads). Open-Meteo's own API default. OS-locale detection rejected (non-deterministic, poisons tests); per-user unit preference is v2.
- D-11: Units applied SERVER-SIDE. On `--units imperial` append `&temperature_unit=fahrenheit&wind_speed_unit=mph` (default path omits them). No client-side conversion; read the returned `current_units` object for the authoritative unit-label suffix rather than hardcoding `°C`/`°F`.
- D-12: Location input = parse-shape disambiguation. Treat the positional arg as `lat,lon` only if it matches `^\s*-?\d+(\.\d+)?\s*,\s*-?\d+(\.\d+)?\s*$` with `lat ∈ [-90,90]`, `lon ∈ [-180,180]`; otherwise geocode as a city name. Geocoding: `GET https://geocoding-api.open-meteo.com/v1/search?name={city}&count=1&language=en&format=json` (keyless; URL-encode the name) → `results[0].{latitude, longitude, name, admin1, country}`. Echo the resolved location to stderr (e.g. `Resolved "London" → London, England, United Kingdom (51.51, -0.13)`). Zero results → `error: no location found for "{name}"` to stderr + exit 1.
- D-13: Forecast call: `GET https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m` (+ unit params on imperial). Field map: `temperature_2m`→temperature, `relative_humidity_2m`→humidity (%), `wind_speed_10m`→wind, `weather_code`→conditions via a WMO `match` returning `&'static str` (`0 => "Clear sky" … _ => "Unknown"`). HTTP = `ureq` 3.3.0 (blocking rustls); deserialize via `serde_json::from_reader(resp.into_body().into_reader())` into a `#[derive(Deserialize)]` struct — avoid ureq's `json` feature (add `serde`'s `derive`; `serde_json` already present). Errors → stderr + exit 1: offline/DNS/connect → `error: could not reach weather service (offline?)`; non-2xx → `error: weather service returned {status}`. Exit 2 reserved for clap bad `--units`. Data → stdout, any color gated via `is_color_on()`.

### Claude's Discretion
- Module layout under `src/commands/<cmd>/mod.rs` per command; exact stdout layout/labels for `weather` (temp / conditions / wind / humidity lines) and `qr` (whether to print the input/URL alongside).
- `pomodoro`: exact countdown render format (`MM:SS`), whether to color the countdown digits (gated), exact phase/glyph styling within the single-flush-per-tick rule, the toast title/body wording.
- `weather`: exact stderr wording within the locked shapes; whether to color conditions/temperature (gated); whether to also surface the geocoded `admin1`/`country` in the stdout header.
- `clip`: exact `--help` wording for the trailing-newline trim note.
- `qr`: choice of `with_error_correction_level` vs `new` (M is the same either way) and exact `--help` examples.

### Deferred Ideas (OUT OF SCOPE)
- **VIS-V2-01:** QR `--save` to an image file + configurable error-correction level. (The fixed `EcLevel::M` + owned-`String` render is the clean seam.)
- **SYS-V2-01:** pomodoro session counter / auto-break / sound / `--label`; weather `--forecast` / `--json` / response cache / stored default location — including per-user unit preference and OS-locale detection.
- **DEV-V2-01:** `--clip` integration across `passgen`/`uuid`/`color`.
- **Out of Scope (not v2):** `qr` decode/scan mode; `clip` binary/image clipboard (text only v1); `weather` API-key requirement / radar / maps.
</user_constraints>

## Project Constraints (from CLAUDE.md)

- **Single Rust binary, subcommand CLI** (`clap` derive) — keep the surface consistent across all 23 tools.
- **Must run in Windows PowerShell 7**; Windows integrations (clipboard, toast, ANSI) must work there.
- **Build target** `x86_64-pc-windows-msvc` with `RUSTFLAGS="-C target-feature=+crt-static"`; avoid `x86_64-pc-windows-gnu`. (`arboard`/`tauri-winrt-notification` use the Win32/WinRT APIs directly — compatible with `+crt-static`, no DLL/async runtime.)
- **`--help` everywhere, predictable flags, 0/1/2 exit codes** — already enforced by the established `RunCommand` + `main.rs` mapping.
- **No async runtime** — `ureq` (blocking) is the locked HTTP client; `reqwest`+`tokio` is on the "What NOT to Use" list. `ureq` 3.x uses rustls by default (no OpenSSL/system DLL).
- **Color via `owo-colors` gated on `is_color_on()`** — never `termcolor`/`colored`, never a parallel color path.
- **Two CLAUDE.md overrides stand (per CONTEXT D-01/D-09):** `qrcode` replaces `qr2term`; `tauri-winrt-notification` replaces `winrt-notification`. These are documented overrides, not violations.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| qr encode + half-block render | CLI command logic (`src/commands/qr`) | `qrcode` (encode + `Dense1x2`) + `core::input` (text in) + `core::output` (data→stdout) | Pure encode→render→print; no network, no terminal-state mutation. Output is plain text (no color path, D-03). |
| clip copy / paste | CLI command logic (`src/commands/clip`) | `arboard` (Win32 clipboard) + raw `io::stdin`/stdout | Single-shot main-thread Win32 op; deliberately bypasses `core::input` (D-04). Owns its own UTF-8 + newline policy. |
| pomodoro countdown + toast | CLI command logic (`src/commands/pomodoro`) | `crossterm` (raw mode / poll-as-timer) + `tauri-winrt-notification` (WinRT toast) | The only Phase-5 command that owns terminal raw-mode state (RAII guard, matrix family); toast is a one-shot WinRT call on completion only. |
| weather fetch + format | CLI command logic (`src/commands/weather`) | `ureq` (blocking HTTPS) + `serde`/`serde_json` (Deserialize) + Open-Meteo (geocode + forecast) | The only Phase-5 command that touches the network. Two sequential GETs (geocode → forecast); pure WMO `match` + label formatting locally. |

All four are **CLI-command-tier** capabilities. `pomodoro` is the only one mutating terminal state (needs a restore guard); `weather` is the only one with a network tier; `clip` is the only one touching an OS-native clipboard surface.

## Standard Stack

> Locked by `05-CONTEXT.md` D-01/D-06/D-09/D-13 and `CLAUDE.md`. NOT re-derived. Versions below re-verified against the crates.io sparse index on 2026-06-24 — all are the current latest stable for their line.

### Core (new this phase)
| Library | Version (locked) | Latest verified | Purpose | Why Standard |
|---------|------------------|-----------------|---------|--------------|
| `qrcode` | 0.14.1 | 0.14.1 (2024-07-05) [VERIFIED: crates.io] | qr: encode + `Dense1x2` Unicode half-block render → owned `String` (D-01/D-02) | kennytm canonical Rust QR crate; `Dense1x2` is purpose-built for terminal half-blocks. `qr2term` REJECTED (always emits ANSI, polarity in bg color — D-01). |
| `arboard` | 3.6.1 | 3.6.1 (2025-08-23) [VERIFIED: crates.io] | clip: read/write Windows `CF_UNICODETEXT` clipboard (D-06) | 1Password/tauri-maintained; Win32 clipboard API directly, correct UTF-8↔UTF-16. `copypasta` rejected (less maintained). |
| `tauri-winrt-notification` | 0.7.2 | 0.7.2 (2025-04-08) [VERIFIED: crates.io] | pomodoro: Windows 10/11 toast on completion (D-09) | Maintained fork of `winrt-notification` (modern `windows 0.61`). `POWERSHELL_APP_ID` → no AUMID registration. |
| `ureq` | 3.3.0 | 3.3.0 (2026-03-21) [VERIFIED: crates.io] | weather: blocking HTTPS GET to Open-Meteo (D-13) | algesten canonical blocking client; rustls default (no OpenSSL/DLL), tiny tree vs reqwest+tokio. |
| `serde` | 1 (derive) | (current 1.x) [VERIFIED: crates.io] | weather: `#[derive(Deserialize)]` for the Open-Meteo response structs (D-13) | Standard. Only the `derive` feature is new; `serde_json` (1.0.150) is already a dep. |

### Supporting (already present — no manifest change)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `crossterm` | 0.29 | pomodoro: raw mode, `event::poll`/`read`, `queue!`/`execute!`, cursor, `terminal::size` | pomodoro only (matrix already uses it; `terminal_width()` wraps `terminal::size`) |
| `serde_json` | 1.0.150 (`preserve_order`) | weather: `from_reader` into the Deserialize struct (D-13) | weather only; `preserve_order` is irrelevant here but harmless |
| `owo-colors` | 4.3 (`supports-colors`) | gated color for any pomodoro/weather decoration | gate on `is_color_on()` — `.truecolor`/`.green()` are unconditional |
| `enable-ansi-support` | 0.3 | already called first in `main()` | nothing new — ANSI already bootstrapped for PS7 |

### Alternatives Considered (already rejected in CONTEXT — do not relitigate)
| Instead of | Could Use | Tradeoff / why rejected |
|------------|-----------|--------------------------|
| `qrcode` + `Dense1x2` | `qr2term` 0.3.3 (CLAUDE.md pick) | D-01: always emits ANSI (no TTY gate), polarity in the background color → breaks glyph-is-truth + byte-identical-minus-ANSI when piped |
| `tauri-winrt-notification` | `winrt-notification` 0.5.1 (CLAUDE.md pick) | D-09: stale (abandoned `windows 0.24`, dead `syn 1.x`). Fork wins on maintenance; same API → it IS the fallback rung if the fork ever breaks |
| `ureq` (blocking, rustls) | `reqwest` + `tokio` | CLAUDE.md "What NOT to Use": async runtime for 2 sequential GETs = 2-4 MB bloat + compile time, zero benefit |
| `arboard` | `copypasta` / `clip.exe` shell-out | D-06: `clip.exe` mangles Unicode + no paste; `copypasta` less maintained |
| ureq `json` feature | (built-in serde_json) | D-13: stay lean — add `serde` `derive` + reuse the existing `serde_json`, deserialize via `from_reader` |

**Installation (Cargo.toml `[dependencies]` additions):**
```toml
# qr (QR-01 / D-01/D-02). Terminal-only: disable defaults to drop the `image`+`svg`+`pic`
# pull-in (default features are ["image","svg","pic"]). No feature is needed for the
# `render::unicode::Dense1x2` path — it is always available.
qrcode = { version = "0.14.1", default-features = false }
# clip (CLIP-01 / D-06). Text-only: disable the default `image-data` feature so arboard
# does NOT pull image/png/bmp + the extra windows-sys image deps (binary/image clipboard
# is Out of Scope). set_text/get_text remain available without it.
arboard = { version = "3.6.1", default-features = false }
# pomodoro toast (POMO-01 / D-09). Windows-only WinRT toast. (Cargo will only build it on
# the msvc target; it brings windows 0.61 + quick-xml + windows-version.)
tauri-winrt-notification = "0.7.2"
# weather (WTHR-01 / D-13). Blocking HTTPS; default features ["rustls","gzip"] (rustls = no
# OpenSSL/DLL). Do NOT enable the `json` feature (D-13 — deserialize via serde_json::from_reader).
ureq = "3.3.0"
# weather Deserialize derive (D-13). serde_json 1.0.150 is ALREADY a dep; only the derive macro is new.
serde = { version = "1", features = ["derive"] }
```
`crossterm` / `owo-colors` / `serde_json` / `enable-ansi-support` are already in the manifest — pomodoro and weather's JSON path need no further manifest change.

**Version verification (run 2026-06-24, crates.io sparse index):**
- `qrcode` max = **0.14.1** (pubtime 2024-07-05; rust_version 1.67.1; default features `["image","svg","pic"]`).
- `arboard` max = **3.6.1** (pubtime 2025-08-23; rust_version 1.71.0; default feature `image-data`).
- `tauri-winrt-notification` max = **0.7.2** (pubtime 2025-04-08; rust_version 1.74; deps `windows ^0.61`, `quick-xml ^0.37`, `thiserror ^2`).
- `ureq` max = **3.3.0** (pubtime 2026-03-21; rust_version 1.85; default features `["rustls","gzip"]`; `json` is opt-in).
Every locked version IS the current latest stable — no upgrade and no stale-pin risk. `ureq` 3.3.0 requires rustc ≥ 1.85 (project builds on 1.90 — fine).

## Package Legitimacy Audit

> slopcheck WAS available and ran successfully against the `crates.io` ecosystem (`slopcheck scan --pkg crates.io <name> --json`). Note: slopcheck's `install` subcommand targets npm/pip and would attempt a real install — for Rust the correct invocation is `scan --pkg crates.io`. All four returned `status: OK` with zero flags. Versions/source-repo maturity cross-checked against the crates.io sparse index.

| Package | Registry | Age / Pubtime | Source Repo | slopcheck | Disposition |
|---------|----------|---------------|-------------|-----------|-------------|
| `qrcode` | crates.io | 0.14.1 @ 2024-07-05 (0.x line since ~2016) | github.com/kennytm/qrcode-rust | OK (0 flags) | Approved [VERIFIED: crates.io] |
| `arboard` | crates.io | 3.6.1 @ 2025-08-23 (actively maintained) | github.com/1Password/arboard | OK (0 flags) | Approved [VERIFIED: crates.io] |
| `tauri-winrt-notification` | crates.io | 0.7.2 @ 2025-04-08 (tauri-maintained fork) | github.com/tauri-apps/winrt-notification | OK (0 flags) | Approved [VERIFIED: crates.io] |
| `ureq` | crates.io | 3.3.0 @ 2026-03-21 (mature, very high downloads) | github.com/algesten/ureq | OK (0 flags) | Approved [VERIFIED: crates.io] |
| `serde` | crates.io | mature (>2B downloads) | github.com/serde-rs/serde | (canonical, not separately scanned) | Approved [VERIFIED: crates.io] |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

All four new crates pass slopcheck (`status: OK`, no flags) AND are confirmed canonical against their official GitHub source repos AND verified on the crates.io sparse index — they qualify for `[VERIFIED: crates.io]`. No `checkpoint:human-verify` gate is required. (`serde`/`serde_json` are dtolnay-canonical and already in the tree.)

## API Verification (the de-risking deliverable)

Every locked-decision API was checked against docs.rs **for the exact pinned version**. Results:

### `qrcode` 0.14.1 (qr — D-01/D-02)
| Symbol | Status | Note for planner |
|--------|--------|------------------|
| `QrCode::new<D: AsRef<[u8]>>(data) -> QrResult<Self>` | [VERIFIED: docs.rs/qrcode/0.14.1] | Encodes with the default `EcLevel::M`. `D: AsRef<[u8]>` so `input.as_bytes()` or `&input` both work. |
| `QrCode::with_error_correction_level<D: AsRef<[u8]>>(data, ec_level) -> QrResult<Self>` | [VERIFIED] | The D-02 form; `EcLevel::M` is the same level `new` uses, so either is fine (Claude's discretion). |
| `EcLevel` {L, M, Q, H} | [VERIFIED — by usage] | docs show `EcLevel::H`/`EcLevel::L` in examples; `M` is the qrcode default (15% recovery, D-02). |
| `QrCode::render::<P: Pixel>(&self) -> Renderer<'_, P>` | [VERIFIED] | Parameterize with `Dense1x2`: `.render::<Dense1x2>()`. Import: `use qrcode::render::unicode::Dense1x2;`. |
| `qrcode::render::unicode::Dense1x2` {Dark, Light} | [VERIFIED: docs.rs/.../enum.Dense1x2.html] | Implements `Pixel` with **`type Image = String`** (so `.build()` returns an **owned `String`** — D-01 confirmed) and `Canvas = Canvas1x2`. Has `default_color()` (the dark-vs-light default). |
| `Renderer::quiet_zone(&mut self, bool) -> &mut Self` | [VERIFIED: docs.rs/.../struct.Renderer.html] | D-02 `.quiet_zone(true)`. **Builder methods take `&mut self` and return `&mut Self`** (not `self`) — chain works on the temporary, but `.build()` is `&self` so the full chain `…render::<Dense1x2>().quiet_zone(true).build()` compiles as a single expression. |
| `Renderer::dark_color(&mut self, color: P) -> &mut Self` / `light_color(&mut self, P) -> &mut Self` | [VERIFIED] | The polarity controls (D-02): `.dark_color(Dense1x2::Dark).light_color(Dense1x2::Light)` to force dark-on-light if the default scans wrong. Only set if the verify step (Pitfall QR-1) fails. |
| `Renderer::build(&self) -> P::Image` | [VERIFIED] | For `Dense1x2`, `P::Image = String` → an owned `String` of pure half-block glyphs (`▀▄█`/space), **no ANSI** (D-01). Route through the standard `data→stdout` path. |
| `Renderer::module_dimensions(&mut self, w, h)` | [VERIFIED] | Not needed for v1 (Dense1x2 default is 1×2 → half-blocks). |

### `arboard` 3.6.1 (clip — D-06)
| Symbol | Status | Note for planner |
|--------|--------|------------------|
| `Clipboard::new() -> Result<Self, Error>` | [VERIFIED: docs.rs/arboard/3.6.1] | Create at the point of use (D-06). On Windows the inner Win32 handle is per-call; one create→op→drop is the whole flow. |
| `Clipboard::set_text<'a, T: Into<Cow<'a, str>>>(&mut self, text: T) -> Result<(), Error>` | [VERIFIED] | ⚠️ **Takes `&mut self`** → the `Clipboard` binding must be `let mut cb = …`. Accepts `String`, `&str`, or `Cow` (so the trimmed copy buffer passes directly). |
| `Clipboard::get_text(&mut self) -> Result<String, Error>` | [VERIFIED] | ⚠️ Also `&mut self`. Returns an owned `String` (UTF-8) → write byte-exact to stdout for `--paste` (D-05: no added/stripped newline). |
| `arboard::Error` | [VERIFIED] | Map any clipboard error to a clean `anyhow` error (exit 1, no panic — FOUND-05). `.context("clipboard")?` is enough. |
| Windows persistence after process exit | [CITED: D-06 / Win32 docs] | `set_text` performs the synchronous `OpenClipboard→EmptyClipboard→SetClipboardData(CF_UNICODETEXT)→CloseClipboard`; the system owns the handle after `SetClipboardData`, so the copied text survives exit. No keep-alive pump (that's an X11/Wayland concern). |

### `tauri-winrt-notification` 0.7.2 (pomodoro toast — D-09)
| Symbol | Status | Note for planner |
|--------|--------|------------------|
| `Toast::POWERSHELL_APP_ID: &'static str` | [VERIFIED: docs.rs/.../struct.Toast.html] | The PowerShell AUMID (`{1AC14E77-…}\…\powershell.exe`). Use it → **no AUMID registration needed** (D-09). The toast will attribute its origin to PowerShell (documented, acceptable for a CLI). |
| `Toast::new(app_id: &str) -> Toast` | [VERIFIED] | `Toast::new(Toast::POWERSHELL_APP_ID)`. |
| `Toast::title(self, &str) -> Toast` / `text1(self, &str) -> Toast` / `text2(self, &str) -> Toast` | [VERIFIED] | ⚠️ **Owned-`self` builder** (returns `Toast` by value) — unlike qrcode's `&mut self`. Chain `Toast::new(...).title(...).text1(...)` moves through. title = white, text1/text2 = grey. Wording is Claude's discretion. |
| `Toast::show(&self) -> Result<()>` | [VERIFIED] | Fire-and-return. **Fire ONLY on completion** (loop fell through), AFTER the `RawGuard` has restored the terminal — never on cancel (D-07). Map `Err` to a clean anyhow error (exit 1); but a toast that fails to show should not undo the successful work — consider logging to stderr and still exiting 0 if the timer completed (planner's call within D-07's "completion → exit 0"). |

### `ureq` 3.3.0 (weather — D-13)
| Symbol | Status | Note for planner |
|--------|--------|------------------|
| `ureq::get(url).call() -> Result<Response<Body>, Error>` | [VERIFIED: docs.rs/ureq/3.3.0] | One blocking GET. URL-encode the city name (D-12) — `percent-encoding` is a transitive ureq dep but NOT re-exported; either build the query carefully or add a tiny encode. (See Pitfall WTHR-2.) |
| **Non-2xx → `Err(Error::StatusCode(u16))` by DEFAULT** | [VERIFIED: docs.rs/.../enum.Error.html] | ⚠️ ureq 3.x treats 4xx/5xx as an error by default (`http_status_as_error()` defaults true). So D-13's "non-2xx → `error: weather service returned {status}`" is a **`match Error::StatusCode(code)` arm**, NOT a post-success `resp.status()` check. The `Ok` branch is always 2xx. |
| `Error::{Io(_), ConnectionFailed, HostNotFound}` | [VERIFIED] | The offline/DNS family → D-13's `error: could not reach weather service (offline?)`. Match these (plus a catch-all) separately from `StatusCode`. |
| `Response::into_body() -> Body` | [VERIFIED] | Consumes the response into the owned body. |
| `Body::into_reader(self) -> BodyReader<'static>` | [VERIFIED: docs.rs/.../struct.Body.html] | ⚠️ Use **`into_reader`** (owned, `'static`) — passes to `serde_json::from_reader`. (`as_reader(&mut self)` borrows; CONTEXT D-13 specifies `into_reader`.) Full chain: `serde_json::from_reader(resp.into_body().into_reader())`. |
| `Body::read_to_string()` | [VERIFIED] | Alternative if a string is wanted, but `from_reader` is the D-13 path (no intermediate `String`). |

### Reused crossterm 0.29 surface (pomodoro — same as matrix, already in-tree)
| Symbol | Status | Note |
|--------|--------|------|
| `enable_raw_mode()` / `disable_raw_mode()` | [VERIFIED — in use in matrix] | Arm a `RawGuard` (Drop = `cursor::Show` + `disable_raw_mode`) THE INSTANT raw mode is on, before any fallible `?` (the CR-01 ordering matrix already encodes). |
| `event::poll(Duration) -> Result<bool>` + `event::read() -> Result<Event>` | [VERIFIED — in use] | `poll(remaining_tick)` is the 1s countdown timer AND the keypress gate (D-07). Compute `remaining_tick` as `min(1s, time_until_completion)` so the final partial second is exact. |
| `KeyEventKind::Press` filter + `is_quit`-style match | [VERIFIED — in use] | Reuse the exact matrix quit predicate shape: Press-only; `Char('c')+CONTROL` / `q` / `Esc`. Cancel → stderr "cancelled" + exit 1, NO toast (D-07). |
| `core::output::terminal_width()` | [VERIFIED: src/core/output.rs:215] | For the in-place countdown render width; 80 when piped. Pomodoro is interactive so piping is a non-case, but stay pipe-safe. |

## Architecture Patterns

### System Architecture Diagram

```
                box <cmd>  (clap parse in src/cli.rs → src/main.rs dispatch)
                       │  init_color(no_color) ── COLOR_ON set once (TTY ∧ ¬NO_COLOR ∧ ¬--no-color)
                       ▼
   ┌──────────────┬────────────────┬─────────────────────┬──────────────────────────┐
   │     qr       │      clip       │      pomodoro        │        weather           │
   ▼              ▼                 ▼                      ▼                          ▼
core::input    copy: io::stdin   enable_raw_mode +       parse arg shape (D-12):
read_input     .read_to_end      RawGuard armed          lat,lon?  ──yes──┐
(arg/-/--file)     │             EnterRaw + Hide cursor   │ no              │
   │            strip ≤1 trailing  │ [RAII Drop guard]    geocode GET ──────┤  ureq::get(...).call()
QrCode::with_  \r\n / \n           ▼                      (Open-Meteo)      │   Ok(2xx)        Err
 error_correc-     │            loop each ~1s tick:       results absent?   │     │             │
 tion_level    arboard::         remaining = end - now    →exit 1 (D-12)    │  into_body()  match Error:
 (…, EcLevel::M)  Clipboard       ┌────────────────────┐  results[0]:      │  .into_reader  StatusCode→
   │            ::new()?          │ render MM:SS in place│  lat/lon/name ────┘  serde_json::   "service {n}"
 .render::<     .set_text(buf)?   │ poll(remaining)=timer│  + echo "Resolved   from_reader     Io/Connection
  Dense1x2>()   (OS owns handle,  │   +keypress gate     │   …" → STDERR        │  Deserialize   /HostNotFound→
 .quiet_zone(    persists exit)   │ Ctrl+C/q/Esc?─cancel │                  forecast GET ──────┤  "offline?"
  true)         paste: get_text() └──────────┬──────────┘                  (+units on imperial)  → exit 1
 .build()  →    → stdout              completed?  cancelled?                    │
 String         byte-exact            │ toast      │ "cancelled"           current_units (label) +
   │ (no ANSI)  (no newline           │ .show()    │ →STDERR               current.{temp,humidity,
   ▼            policy on paste)      │ exit 0     │ exit 1, NO toast       weather_code→WMO match,wind}
 STDOUT ◄───────────┴── (Drop: Show + disable_raw_mode on every exit path) ──┴─► STDOUT (data) ; messages►STDERR
                                      exit 0 / 1 / 2  (main() maps BoxError → exit 2; clap bad --units → exit 2)
```

### Component Responsibilities
| File | Responsibility |
|------|----------------|
| `src/cli.rs` | Swap 4 unit variants (`Qr` / `Clip` / `Pomodoro` / `Weather`, lines 86–92) for real `Args` structs |
| `src/main.rs` | Delete the 4 `not_implemented(...)` arms (lines 95–98); add `=> args.run()` |
| `src/commands/mod.rs` | Register 4 `pub mod` lines |
| `src/commands/qr/mod.rs` | `QrArgs` + `core::input` read + `QrCode…Dense1x2…build()` → `println!`; no color path (D-03) |
| `src/commands/clip/mod.rs` | `ClipArgs { paste: bool }` + raw stdin read + single-trailing-newline trim + `arboard` set/get |
| `src/commands/pomodoro/mod.rs` | `PomodoroArgs { minutes: Option<u64>, break_: bool, long_break: bool }` + duration resolve + matrix-style `RawGuard` + poll-as-timer loop + `MM:SS` render + completion toast |
| `src/commands/weather/mod.rs` | `WeatherArgs { location: String, units: Units }` (clap `ValueEnum`) + lat/lon parse + geocode + forecast + WMO `match` + Deserialize structs |
| `tests/{qr,clip,pomodoro,weather}.rs` | `assert_cmd` integration; `tests/cmd/*.trycmd` only where output is stable (qr render, weather error paths) |

### Pattern 1: Thin orchestrator over pure helpers (established — all commands)
**What:** `run()` acquires input, delegates the testable math to pure crate-free functions (lat/lon parse, WMO map, countdown format, newline trim), prints, returns `Result`. Any color gated once via `is_color_on()`.
**When to use:** Every command.
**Example (the exact color-gate precedent):**
```rust
// Source: src/commands/color/mod.rs:53-58 (the ONE color path, gated)
let swatch = "██████████";
if is_color_on() {
    println!("  {}", swatch.truecolor(r, g, b));
} else {
    println!("  {swatch}");
}
```

### Pattern 2: Pure helpers as the unit-test seam (the four testable pieces)
```rust
// clip (D-05) — strip AT MOST ONE trailing terminator (\r\n, then a lone \n).
fn trim_one_trailing_newline(mut s: Vec<u8>) -> Vec<u8> {
    if s.last() == Some(&b'\n') {
        s.pop();
        if s.last() == Some(&b'\r') { s.pop(); }
    }
    s
}
// weather (D-13) — WMO code → &'static str, no alloc, _ future-proofs.
fn wmo_to_str(code: u32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 | 2 | 3 => "Partly cloudy",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        61 | 63 | 65 => "Rain",
        71 | 73 | 75 => "Snow",
        80 | 81 | 82 => "Rain showers",
        95 => "Thunderstorm",
        _ => "Unknown",
    } // exact labels are discretion; the _ arm is mandated by D-13
}
// weather (D-12) — parse arg as lat,lon ONLY if it matches the shape AND is in range.
fn parse_lat_lon(s: &str) -> Option<(f64, f64)> {
    let re = regex::Regex::new(r"^\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*$").ok()?;
    let c = re.captures(s.trim())?;
    let lat: f64 = c[1].parse().ok()?;
    let lon: f64 = c[2].parse().ok()?;
    (( -90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
}
// pomodoro — MM:SS from remaining seconds (discretion format; pure + testable).
fn fmt_mmss(secs: u64) -> String { format!("{:02}:{:02}", secs / 60, secs % 60) }
```
(`regex` 1.12.4 is already a dep — see Cargo.toml:41 — so the lat/lon parser needs no new crate.)

### Pattern 3: RAII restore guard for pomodoro (D-07 — reuse matrix verbatim)
**What:** Construct a `RawGuard` right after `enable_raw_mode()?`, BEFORE the fallible `EnterAlternateScreen`/`Hide` `execute!` (the CR-01 ordering matrix already encodes). Its `Drop` runs `cursor::Show` + `disable_raw_mode()` (errors ignored) on normal return, `?` early-return, AND unwinding.
**Why:** Same teardown contract as matrix; pomodoro joins the raw-mode family. Note pomodoro may not need the *alternate screen* (an in-place countdown can live on the normal screen with `cursor::Hide` + a single-line redraw) — that is Claude's discretion; if the alt screen is skipped, the guard restores `Show` + `disable_raw_mode` only.
```rust
// Source pattern: src/commands/matrix/mod.rs:101-116 + 118-133 (RawGuard armed pre-execute!)
struct RawGuard;
impl Drop for RawGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, crossterm::cursor::Show);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}
```

### Anti-Patterns to Avoid
- **Routing `clip` copy through `core::input::read_input`** — that inherits the no-arg-TTY → exit-2 contract, wrong for a clipboard copy (D-04). Read raw stdin bytes directly.
- **Hardcoding `°C`/`°F` or `km/h`/`mph`** in weather — read the authoritative label from `current_units` (D-11). Open-Meteo even returns `"mp/h"` (not `"mph"`) for imperial wind, so the request param ≠ the label.
- **Checking `resp.status()` after a successful `call()` for the non-2xx path** — ureq 3.x already returned `Err(StatusCode)`. Match the error variant (Pitfall WTHR-1).
- **A second color path / `owo_colors::set_override` toggling** — all color via `is_color_on()` gating (D-00).
- **Firing the pomodoro toast on cancel** — toast is completion-only; cancel exits 1 with NO toast (D-07).
- **`cargo test --lib`** — `box` is binary-only; unit tests run via `cargo test --bin box` (STATE.md [03-01]).
- **Spawning the arboard call onto a worker thread** — STATE.md hard pitfall; the synchronous single-shot main-thread flow is the mitigation (D-06).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| QR encoding + half-block layout | a QR encoder / ECC + bitmap | `qrcode` + `Dense1x2` (D-01) | Reed-Solomon ECC, masking, version selection, and the 1×2 half-block packing are a large correct-by-construction substrate |
| Windows clipboard (Unicode) | Win32 `OpenClipboard`/`SetClipboardData` calls / `clip.exe` shell-out | `arboard` (D-06) | correct `CF_UNICODETEXT` UTF-8↔UTF-16, handle ownership, no elevation; `clip.exe` mangles Unicode + can't paste |
| Windows toast | raw WinRT `ToastNotificationManager` XML | `tauri-winrt-notification` (D-09) | WinRT COM init + toast XML schema + AUMID handling; the crate's `POWERSHELL_APP_ID` skips registration |
| Blocking HTTPS GET + TLS | a socket + TLS handshake | `ureq` (D-13) | rustls handshake, redirects, status-as-error, chunked/gzip body — all handled; no async runtime |
| Geocoding / weather data | a geocoder / weather model | Open-Meteo (keyless) | keyless public API; client just maps `weather_code` → text |
| lat/lon regex parse | (none — but use the in-tree crate) | `regex` 1.12.4 (already a dep) | already present; D-12's exact shape is one pattern |

**Key insight:** All five of this phase's hard substrates (QR ECC, Win32 clipboard, WinRT toast, TLS HTTP, public geo/weather data) are exactly the "deceptively complex, do-not-hand-roll" category. The hand-rolled parts are tiny pure functions — newline trim, WMO `match`, lat/lon parse, `MM:SS` format — which is consistent with the project's "lean on crates for the hard substrate, hand-roll the trivial logic" pattern.

## Common Pitfalls

### Pitfall QR-1: Terminal QR fails to scan — polarity + quiet zone (qr)
**What goes wrong:** A phone camera can't read the rendered QR.
**Why:** (1) Insufficient quiet zone (border); (2) inverted polarity on a dark terminal theme — if the "dark" modules render as the *background* color, contrast is lost against a dark PS7 background.
**How to avoid:** `.quiet_zone(true)` (full 4-module ISO border, D-02) and ensure the dark modules are the printed *glyph* (dark-on-light), so contrast is independent of the terminal theme. `Dense1x2` carries a `default_color()`; **verify the default polarity scans during impl** and only flip `.dark_color(Dense1x2::Dark)`/`.light_color(Dense1x2::Light)` if a quick phone-scan test fails (D-02).
**Warning signs:** scanner won't lock; QR looks "inverted" (light modules on dark).

### Pitfall CLIP-1: Spurious trailing newline from the PowerShell pipe (clip)
**What goes wrong:** `"x" | box clip` lands `x\r\n` on the clipboard (a trailing blank).
**Why:** PowerShell 7 appends an implicit CRLF when piping a string to a native command (D-05) — the exact `clip.exe` friction this command exists to fix.
**How to avoid:** On copy, strip **at most one** trailing terminator (`\r\n`, then a lone `\n`) — never interior or multiple (D-05). On `--paste`, emit byte-exact (no added/stripped newline). Mirror `pbcopy`/`xclip`. Document the single-trim in `--help`.
**Warning signs:** pasted text has an extra blank line; round-trip copy→paste grows a newline.

### Pitfall CLIP-2: `arboard::Clipboard` needs `&mut self` and main-thread (clip)
**What goes wrong:** Compile error (`set_text`/`get_text` need `&mut`), or intermittent failure if moved off-thread.
**Why:** Both `set_text` and `get_text` take `&mut self`; STATE.md records "arboard clipboard must run on main thread only."
**How to avoid:** `let mut cb = arboard::Clipboard::new()?;` then one op, return. Never spawn the call onto a worker thread (D-06). The single-shot synchronous flow satisfies both constraints.
**Warning signs:** "cannot borrow as mutable"; flaky clipboard ops under threading.

### Pitfall POMO-1: Ctrl+C is a KeyEvent (not SIGINT), Windows double-fire, toast-on-cancel (pomodoro)
**What goes wrong:** Ctrl+C kills the process without restoring the cursor; or the exit key double-counts; or a toast fires on a cancel.
**Why:** Raw mode delivers Ctrl+C as `KeyCode::Char('c')+CONTROL`, not SIGINT (so no `ctrlc` crate); Windows fires Press AND Release; cancel must NOT toast (D-07).
**How to avoid:** Reuse the matrix predicate verbatim — `KeyEventKind::Press`-only filter; match `Char('c')+CONTROL` / `q` / `Esc` → cancel ("cancelled" to stderr, exit 1, NO toast). Completion (loop falls through) → toast + exit 0. The `RawGuard` restores on every path.
**Warning signs:** parked cursor after Ctrl+C; "sticky" exit; toast appears when you cancelled.

### Pitfall POMO-2: Countdown drift / final-second precision (pomodoro)
**What goes wrong:** The countdown ends a fraction early/late, or `poll` blocks longer than the remaining time.
**Why:** Using a fixed `poll(1s)` overshoots on the final partial second.
**How to avoid:** Compute `remaining = end_instant - Instant::now()` each tick and `poll(min(1s, remaining))`; break to completion when `remaining` hits 0 (D-07). Single-flush-per-tick redraw (the matrix discipline) keeps it pipe-safe and flicker-free.
**Warning signs:** timer finishes ±1s off; visible stutter.

### Pitfall WTHR-1: ureq 3.x returns Err on non-2xx by default (weather)
**What goes wrong:** Code waits for a success then checks `resp.status()` for the error path — but the success branch is always 2xx, so the non-2xx message never fires; or an offline error is mislabeled as a status error.
**Why:** ureq 3.x has `http_status_as_error()` true by default → 4xx/5xx become `Err(Error::StatusCode(code))`; offline/DNS are `Error::{Io, ConnectionFailed, HostNotFound}` (D-13 split).
**How to avoid:** `match` the error: `Error::StatusCode(code) => "error: weather service returned {code}"` (exit 1); `Error::Io(_) | Error::ConnectionFailed | Error::HostNotFound | _ => "error: could not reach weather service (offline?)"` (exit 1). Exit 2 is reserved for clap's bad `--units` only.
**Warning signs:** offline test prints a status message (or vice versa); the non-2xx branch is dead code.

### Pitfall WTHR-2: Geocoding no-results omits the `results` key entirely (weather)
**What goes wrong:** Deserialize panics/errors on a not-found city because the struct expects `results: Vec<...>`.
**Why (VERIFIED live):** Open-Meteo's no-match geocoding response is `{"generationtime_ms":...}` — the `results` key is **absent**, NOT an empty array.
**How to avoid:** `#[serde(default)] results: Vec<GeoHit>` (or `Option<Vec<GeoHit>>`) on the geocode struct; treat absent/empty as "no location found" → `error: no location found for "{name}"` (exit 1, D-12). Also URL-encode the city name in the query.
**Warning signs:** "missing field `results`" deserialize error on a typo'd city.

### Pitfall WTHR-3: Unit label ≠ request param; humidity is an integer (weather)
**What goes wrong:** Output shows `mph` but the API labels it `mp/h`; or humidity deserializes wrong.
**Why (VERIFIED live):** the imperial wind request param is `wind_speed_unit=mph` but Open-Meteo's `current_units.wind_speed_10m` label is `"mp/h"`. `current.relative_humidity_2m` comes back as an integer (e.g. `35`), `temperature_2m`/`wind_speed_10m` as floats, `weather_code` as an integer.
**How to avoid:** Read the suffix from `current_units` (D-11), never hardcode. Deserialize `temperature_2m: f64`, `wind_speed_10m: f64`, `relative_humidity_2m: f64` (or `i64` — but `f64` is the safest, JSON numbers may render either way), `weather_code: u32`. Map `weather_code` via the WMO `match` (D-13).
**Warning signs:** label says `mph` not what the API returned; a type-mismatch deserialize error on humidity.

## Code Examples

Verified patterns from official sources + live API.

### qr: encode → Dense1x2 half-blocks → stdout (QR-01)
```rust
// Source: docs.rs/qrcode/0.14.1 (QrCode::with_error_correction_level, render, Dense1x2)
use qrcode::{QrCode, EcLevel};
use qrcode::render::unicode::Dense1x2;

let input = crate::core::input::read_input(self.input)?; // arg / - / --file (D-03)
let code = QrCode::with_error_correction_level(input.as_bytes(), EcLevel::M)?; // M = qrcode default (D-02)
let rendered: String = code
    .render::<Dense1x2>()
    .quiet_zone(true)                 // full ISO 4-module border (D-02)
    // .dark_color(Dense1x2::Dark).light_color(Dense1x2::Light) // ONLY if phone-scan fails (Pitfall QR-1)
    .build();                         // owned String, pure half-blocks, NO ANSI (D-01)
println!("{rendered}");              // data → stdout; no color path (D-03)
```

### clip: copy (trim ≤1 newline) / paste (byte-exact) (CLIP-01)
```rust
// Source: docs.rs/arboard/3.6.1 (Clipboard::new/set_text/get_text — all need &mut)
use std::io::{Read, Write};

if self.paste {
    let mut cb = arboard::Clipboard::new().context("open clipboard")?;
    let text = cb.get_text().context("read clipboard")?;     // owned String
    std::io::stdout().write_all(text.as_bytes())?;           // byte-exact, no newline policy (D-05)
} else {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;                 // raw bytes, NOT core::input (D-04)
    let buf = trim_one_trailing_newline(buf);               // ≤1 trailing \r\n / \n (D-05)
    let text = String::from_utf8(buf).context("clipboard input must be UTF-8")?;
    let mut cb = arboard::Clipboard::new().context("open clipboard")?;
    cb.set_text(text).context("write clipboard")?;          // persists via OS handle (D-06)
}
```

### pomodoro: completion toast (POMO-01)
```rust
// Source: docs.rs/tauri-winrt-notification/0.7.2 (owned-self builder; POWERSHELL_APP_ID)
use tauri_winrt_notification::Toast;
// ... after the poll-as-timer loop falls through to COMPLETION (D-07), guard already dropped ...
Toast::new(Toast::POWERSHELL_APP_ID)            // no AUMID registration (D-09)
    .title("Pomodoro")                          // wording = discretion
    .text1("Time's up — take a break!")
    .show()
    .context("show toast")?;                    // completion → exit 0; cancel path never reaches here
```

### weather: geocode + forecast with the live JSON shapes (WTHR-01)
```rust
// Deserialize structs match the LIVE Open-Meteo responses (verified 2026-06-24).
#[derive(serde::Deserialize)]
struct GeoResp { #[serde(default)] results: Vec<GeoHit> }   // `results` ABSENT on no-match (Pitfall WTHR-2)
#[derive(serde::Deserialize)]
struct GeoHit { latitude: f64, longitude: f64, name: String,
                admin1: Option<String>, country: Option<String> }

#[derive(serde::Deserialize)]
struct Forecast { current: Current, current_units: Units }  // read units label, don't hardcode (D-11)
#[derive(serde::Deserialize)]
struct Current { temperature_2m: f64, relative_humidity_2m: f64,
                 weather_code: u32, wind_speed_10m: f64 }
#[derive(serde::Deserialize)]
struct Units { temperature_2m: String, wind_speed_10m: String } // e.g. "°C"/"°F", "km/h"/"mp/h"

// Source: docs.rs/ureq/3.3.0 — non-2xx is Err(StatusCode); offline is Io/ConnectionFailed/HostNotFound.
fn fetch<T: serde::de::DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    match ureq::get(url).call() {
        Ok(resp) => Ok(serde_json::from_reader(resp.into_body().into_reader())?), // D-13 into_reader
        Err(ureq::Error::StatusCode(code)) => {
            anyhow::bail!("weather service returned {code}")                       // non-2xx (Pitfall WTHR-1)
        }
        Err(_) => anyhow::bail!("could not reach weather service (offline?)"),    // offline/DNS
    }
}
// geocode: https://geocoding-api.open-meteo.com/v1/search?name={url-encoded}&count=1&language=en&format=json
// forecast: https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}
//           &current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m
//           (+ &temperature_unit=fahrenheit&wind_speed_unit=mph  ONLY on --units imperial, D-11)
```

**Live forecast response (metric, captured 2026-06-24):**
```json
{"latitude":51.5,"longitude":-0.25,"current_units":{"temperature_2m":"°C","relative_humidity_2m":"%","weather_code":"wmo code","wind_speed_10m":"km/h"},"current":{"time":"2026-06-24T14:45","interval":900,"temperature_2m":36.1,"relative_humidity_2m":35,"weather_code":0,"wind_speed_10m":9.0}}
```
**Live imperial wind label is `"mp/h"` (not `"mph"`); live geocoding hit:**
```json
{"results":[{"name":"London","latitude":51.50853,"longitude":-0.12574,"country":"United Kingdom","admin1":"England", ...}]}
```
**Live no-match geocoding (note: NO `results` key):** `{"generationtime_ms":0.458}`

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `qr2term` (ANSI-colored terminal QR) | `qrcode` + `Dense1x2` (plain half-blocks) | CONTEXT D-01 | byte-identical-minus-ANSI preserved; clean `box qr … > file.txt` |
| `winrt-notification` 0.5.1 (`windows 0.24`, `syn 1.x`) | `tauri-winrt-notification` 0.7.2 (`windows 0.61`) | CONTEXT D-09 | maintained; same `Toast` API → fallback rung if fork breaks |
| ureq 2.x (`resp.into_reader()` on the response) | ureq 3.x (`resp.into_body().into_reader()`; non-2xx = `Err(StatusCode)`) | ureq 3.0 | the body is now a separate `Body`; status-as-error is default (Pitfall WTHR-1) |
| `clip.exe` shell-out for clipboard | `arboard` Win32 `CF_UNICODETEXT` | CONTEXT D-06 | correct Unicode + paste support, no elevation |

**Deprecated/outdated:** none affecting this phase. `qr2term`, `winrt-notification`, `reqwest`+`tokio`, `copypasta`, and `clip.exe` were evaluated and rejected (CONTEXT/CLAUDE.md). No locked crate is deprecated.

## Runtime State Inventory

> Rename/refactor phases only. **This is a greenfield feature phase (four new commands), not a rename/refactor — section included for completeness; nothing to migrate.**

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore; `clip` writes the OS clipboard (transient, OS-owned), `weather` is stateless | None |
| Live service config | None — Open-Meteo is keyless/config-less; no external service config in git or a UI | None |
| OS-registered state | None — `tauri-winrt-notification` uses `POWERSHELL_APP_ID` (no AUMID registration, D-09); no Task Scheduler/pm2/service registration | None |
| Secrets/env vars | None — keyless weather API (no key), no new env vars; honors existing `NO_COLOR` | None |
| Build artifacts | None new — four new `src/commands/<cmd>/mod.rs` modules + Cargo.toml deps; `Cargo.lock` updates on build (committed per [01-01]) | Reinstall via `install.ps1` after build (existing flow) |

**Verified:** these are four new vertical-slice commands consuming existing `core` helpers and new crates; no pre-existing runtime state is renamed, moved, or migrated.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `Dense1x2`'s default polarity renders dark-on-light (dark modules as the printed glyph) and scans on a dark PS7 background without flipping `.dark_color`/`.light_color` | Standard Stack / Pitfall QR-1 | LOW — D-02 already mandates a phone-scan verify step during impl; the flip is a one-line fix if the default is wrong. Documented as a verify-then-decide, not an asserted fact |
| A2 | Disabling default features on `qrcode` (`["image","svg","pic"]` off) and `arboard` (`image-data` off) still compiles the `Dense1x2` render path and `set_text`/`get_text` respectively | Standard Stack / Installation | LOW — `Dense1x2` lives in `render::unicode` (not behind `image`/`svg`); arboard `set_text`/`get_text` are core, `image-data` only adds image clipboard (Out of Scope). If a build error surfaces, keep full defaults (a heavier binary, not a failure). The planner should make the first build of each command verify the trimmed feature set resolves |
| A3 | A toast that fails to `show()` after a *completed* timer should still exit 0 (the work succeeded); only log to stderr | API Verification (Toast) | LOW — within D-07's "completion → exit 0"; the exact handling (bail vs warn-and-exit-0) is a small planner decision. Flagged so it is decided explicitly, not by accident |
| A4 | `relative_humidity_2m` is safe to deserialize as `f64` even though the live sample returned an integer (`35`) | Code Examples / Pitfall WTHR-3 | LOW — serde_json parses an integer literal into `f64` fine; `f64` tolerates both forms. `u32`/`i64` would also work for the observed integer but risks a future fractional value |

**All other claims are [VERIFIED] against docs.rs / the crates.io sparse index / the live Open-Meteo API, or [CITED] to official docs. These four are the only assumptions, all LOW risk and all aligned with the locked decisions.**

## Open Questions

1. **Pomodoro: alternate screen vs in-place single-line redraw** — Claude's discretion (D-07/discretion). An in-place `MM:SS` redraw on the normal screen (`cursor::Hide` + carriage-return-rewrite or `MoveTo`) is lighter than the matrix alt-screen approach and reads naturally for a timer. Recommendation: in-place on the normal screen, `RawGuard` restores `Show` + `disable_raw_mode`. Not blocking.
2. **Weather stdout layout** — discretion (D-13/discretion): the exact temp/conditions/wind/humidity line format, whether to color (gated), whether to surface `admin1`/`country` in the header. Recommendation: a 4-line labeled block (like `color`), conditions/temp optionally `.cyan()`-gated, header `London, England, United Kingdom`. Not blocking.
3. **qr: print the input/URL alongside the code?** — discretion (D-03/discretion). Recommendation: print just the QR (clean `> file.txt`); a `--help` example shows the URL. Not blocking.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (msvc) | build all four | ✓ (per Phase 1–4) | rustc/cargo 1.90 | — (ureq 3.3.0 needs ≥1.85; satisfied) |
| crates.io network (first build) | fetch 5 new crates | assumed ✓ | — | `cargo` offline cache if pre-fetched |
| Internet egress to Open-Meteo (HTTPS 443) | `weather` runtime | runtime-dependent | — | graceful offline error is the SPEC (D-13) — not a build blocker; tests use a bogus host / pure-fn coverage |
| Windows 10/11 WinRT toast surface | `pomodoro` toast | ✓ (Windows 11 target) | — | fallback ladder D-09 (winrt-notification → winrt-toast-reborn → BurntToast shell-out) |
| Win32 clipboard | `clip` | ✓ (no elevation, D-06) | — | none needed |
| CJK font | (N/A this phase) | — | — | — (matrix-only concern) |

**Missing dependencies with no fallback:** none at build time. The only runtime external dependency is Open-Meteo for `weather`, and its absence is an explicitly-specified graceful error (not a failure).

**Missing dependencies with fallback:** Open-Meteo offline → graceful stderr error (D-13). Toast surface unavailable → D-09 fallback ladder (localized to the pomodoro module).

## Validation Architecture

> `workflow.nyquist_validation: true` (config.json) → this section is required. Mirrors the 04-RESEARCH format so `05-VALIDATION.md` can be generated directly.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (unit, in-module `#[cfg(test)]`) + `assert_cmd` 2.2 / `predicates` 3.1 (integration) + `trycmd` 1.2 (CLI snapshots) |
| Config file | none — Cargo convention (`tests/<cmd>.rs`, `tests/cmd/*.trycmd`, `tests/cmd/<cmd>.in/` fixtures) |
| Quick run command | `cargo test --bin box <module>` (unit) — **`--bin box`, NEVER `--lib`** (binary-only crate, STATE.md [03-01]) |
| Full suite command | `cargo test` (all unit + integration) then `cargo clippy -- -D warnings` + `cargo fmt --check` |

### Phase Requirements → Test Map
| Req | Behavior | Test Type | Automated Command | File |
|-----|----------|-----------|-------------------|------|
| QR-01 | encode arg/stdin → owned `String` of pure half-blocks, NO ANSI | unit (`render::<Dense1x2>().quiet_zone(true).build()` returns non-empty `String`; assert no `\x1b`) | `cargo test --bin box qr` | `src/commands/qr/mod.rs` #[cfg(test)] ❌ Wave 0 |
| QR-01 | `box qr "https://example.com"` exits 0, non-empty half-block output, no ANSI when piped | integration (`assert_cmd`: arg + piped-stdin both render; output contains a half-block glyph `▀`/`▄`/`█`; no `\x1b[`) | `cargo test --test qr` | `tests/qr.rs` ❌ Wave 0 |
| QR-01 | stable rendered layout for a fixed input | snapshot (a short fixed string → checked-in expected block; quiet-zone + polarity locked) | `cargo test --test cli` (trycmd) | `tests/cmd/qr.trycmd` ❌ Wave 0 (optional — only if polarity/layout proves byte-stable) |
| QR-01 | a phone camera reads the rendered QR | (not auto-testable) | manual / human-verify | — |
| CLIP-01 | copy strips AT MOST ONE trailing `\r\n`/`\n`, never interior/multiple | unit (`trim_one_trailing_newline`: `b"x\r\n"`→`b"x"`, `b"x\n"`→`b"x"`, `b"x\n\n"`→`b"x\n"`, `b"a\nb"`→`b"a\nb"`) | `cargo test --bin box clip` | `src/commands/clip/mod.rs` #[cfg(test)] ❌ Wave 0 |
| CLIP-01 | round-trip copy→paste preserves Unicode byte-exact; no elevation | integration (`assert_cmd`: pipe `héllo 世界` to `box clip`, then `box clip --paste` → stdout equals input minus the one trailing newline; exit 0) | `cargo test --test clip` | `tests/clip.rs` ❌ Wave 0 (serialize: shared OS clipboard — single test or `--test-threads=1`) |
| CLIP-01 | non-UTF-8 stdin on copy → clean exit 1, no panic | integration (`assert_cmd`: pipe `\xff\xfe` → exit 1, stderr message, no panic) | `cargo test --test clip` | `tests/clip.rs` ❌ Wave 0 |
| POMO-01 | duration resolve: default 25, `--break` 5, `--long-break` 15, `[MINUTES]` override | unit (pure resolver: flag/arg → seconds) | `cargo test --bin box pomodoro` | `src/commands/pomodoro/mod.rs` #[cfg(test)] ❌ Wave 0 |
| POMO-01 | countdown `MM:SS` format + per-tick `remaining` math (no drift) | unit (`fmt_mmss(0)="00:00"`, `fmt_mmss(65)="01:05"`, `fmt_mmss(1500)="25:00"`; remaining-tick clamp) | `cargo test --bin box pomodoro` | same #[cfg(test)] ❌ Wave 0 |
| POMO-01 | quit predicate: Press-only `Ctrl+C`/`q`/`Esc`; Release/Repeat ignore; bare `c` not quit | unit (reuse matrix `is_quit`-shape tests verbatim) | `cargo test --bin box pomodoro` | same #[cfg(test)] ❌ Wave 0 |
| POMO-01 | a very short timer (e.g. `box pomodoro 0` or a test-only tiny duration) exits cleanly + restores terminal | smoke (`assert_cmd`: non-TTY/piped stdin → starts and exits non-hanging, exit 0, no panic) | `cargo test --test pomodoro` | `tests/pomodoro.rs` ❌ Wave 0 |
| POMO-01 | live countdown renders in-place + real Windows toast on completion + Ctrl+C cancels cleanly (no toast) | (not auto-testable — interactive + toast) | manual / human-verify | — |
| WTHR-01 | lat/lon parse-shape: matches `lat,lon` in range only; else city | unit (`parse_lat_lon("51.5,-0.13")=Some`, `"London"=None`, `"91,0"=None` out of range, `"1,2,3"=None`) | `cargo test --bin box weather` | `src/commands/weather/mod.rs` #[cfg(test)] ❌ Wave 0 |
| WTHR-01 | WMO `weather_code` → `&'static str` (incl. `_` → "Unknown") | unit (`wmo_to_str(0)="Clear sky"`, `wmo_to_str(95)="Thunderstorm"`, `wmo_to_str(9999)="Unknown"`) | `cargo test --bin box weather` | same #[cfg(test)] ❌ Wave 0 |
| WTHR-01 | Deserialize the live forecast + geocoding shapes; no-results geocode (absent `results`) → "no location found" | unit (`serde_json::from_str` on the captured fixtures: forecast parses; `{"generationtime_ms":0.4}` → empty results) | `cargo test --bin box weather` | same #[cfg(test)] ❌ Wave 0 |
| WTHR-01 | offline / non-2xx → graceful stderr error + exit 1 (NOT exit 2) | integration (`assert_cmd`: point at a bogus host via a test seam, OR run offline → exit 1, stderr contains "offline"/"service"; bad `--units` value → exit 2) | `cargo test --test weather` | `tests/weather.rs` ❌ Wave 0 |
| WTHR-01 | `--units imperial` appends the unit params; default omits them | unit (pure URL-builder: imperial URL contains `temperature_unit=fahrenheit&wind_speed_unit=mph`; default does not) | `cargo test --bin box weather` | same #[cfg(test)] ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --bin box <module>` (the touched command's unit tests) — sub-second.
- **Per wave merge:** `cargo test` (full unit + integration). Note `tests/clip.rs` touches the shared OS clipboard — keep it to a single test fn or run that file with `--test-threads=1` to avoid cross-test races.
- **Phase gate:** `cargo test` green + `cargo clippy -- -D warnings` + `cargo fmt --check` before `/gsd:verify-work` (the established Phase-3/4 gate). The two human-verify items (pomodoro live/toast/cancel; qr phone-scan) gate phase completion like matrix did.

### Wave 0 Gaps
- [ ] `tests/qr.rs` — QR-01 arg + piped render, half-block present, no-ANSI-when-piped
- [ ] `tests/clip.rs` — CLIP-01 round-trip Unicode (single-threaded), newline trim, non-UTF-8 exit-1
- [ ] `tests/pomodoro.rs` — POMO-01 short-timer enters/exits-cleanly smoke (non-TTY)
- [ ] `tests/weather.rs` — WTHR-01 offline/non-2xx exit-1 + bad-`--units` exit-2
- [ ] In-module `#[cfg(test)]` blocks for the pure helpers: qr build-to-String, `trim_one_trailing_newline`, pomodoro duration-resolve + `fmt_mmss` + quit predicate, weather `parse_lat_lon` + `wmo_to_str` + Deserialize-from-fixture + URL-builder
- [ ] Captured Open-Meteo JSON fixtures (forecast metric, forecast imperial, geocode hit, geocode no-match) checked in for the weather Deserialize unit tests (the live samples in Code Examples are the source)
- [ ] Optional `tests/cmd/qr.trycmd` for a fixed-input render — ONLY if the half-block layout/polarity proves byte-stable across machines (trycmd is snapshot-friendly for plain text; NOT for the interactive pomodoro or the toast)

*The pure-logic helpers (qr render-to-String, newline trim, duration/format/quit, lat-lon/WMO/Deserialize/URL) unit-test cleanly without a terminal, network, or clipboard hardware. The irreducibly-interactive pieces — pomodoro's live countdown + real toast + Ctrl+C cancel, and qr's phone-scan — are human-verify, exactly as matrix was in Phase 4. `weather`'s network path is covered by pure-fn tests over captured fixtures plus an offline/exit-code integration test (no live API call in CI).*

## Security Domain

> `security_enforcement` key is absent from config.json → treat as enabled. Narrow surface: four local commands, one keyless read-only network call, one OS clipboard write, one local toast. No auth, no persistence, no secrets.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no auth; Open-Meteo is keyless) |
| V3 Session Management | no | — |
| V4 Access Control | no | — (reads only user-supplied stdin/args; clipboard is the user's own session) |
| V5 Input Validation | yes | lat/lon parse is shape+range validated (D-12); city name URL-encoded; `--units` is a clap `ValueEnum` (invalid → exit 2); malformed JSON / clipboard / QR input → `anyhow` error → exit 1, never a panic (FOUND-05) |
| V6 Cryptography | no (consume only) | — (TLS is handled inside ureq/rustls; no crypto implemented here) |
| V9 Communications | yes (partial) | weather over HTTPS via ureq's rustls default (no plaintext, no OpenSSL/DLL); offline/cert failures surface as a clean exit-1 error (D-13) |
| V12 Files/Resources | yes (partial) | `qr` may read a `--file` (same posture as hash/json `--file`); clip reads stdin; no traversal logic |

### Known Threat Patterns for {Rust local CLI: terminal QR, OS clipboard, WinRT toast, keyless HTTPS}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed input (QR text, non-UTF-8 clip stdin, bad JSON) → panic | Denial of Service | every fallible op returns `Result` → mapped to exit 1, never `.unwrap()` (FOUND-05 / T-02-06 precedent) |
| Untrusted server response (Open-Meteo) | Tampering | strict `#[derive(Deserialize)]` (unknown fields ignored by default; the `_` WMO arm future-proofs); no `eval`, no shell-out from the response |
| Terminal-escape injection via clipboard/QR content re-emit | Tampering (output spoofing) | `qr` emits only half-block glyphs (the renderer produces no control bytes from arbitrary input); `clip --paste` writes the clipboard byte-exact to a pipe — a consumer rendering it to a TTY is out of this tool's scope (same posture as any `cat`/`xclip`). No new lolcat-style re-color path here |
| HTTP status/connection error mishandled (offline mislabeled, dead branch) | DoS / Information (confusing UX) | ureq 3.x `Error::StatusCode` vs `Io/ConnectionFailed/HostNotFound` matched explicitly (Pitfall WTHR-1); offline is an expected, graceful path (D-13) |
| Clipboard data exposure | Information Disclosure | single-user local tool; the clipboard is the user's own session — reading/writing it is the explicit, requested function (CLIP-01), not a leak |
| WinRT toast spoofing origin (PowerShell AUMID) | Spoofing | accepted/documented (D-09): `POWERSHELL_APP_ID` makes the toast attribute to PowerShell — a known, acceptable cosmetic for a registration-free CLI |

No new secrets, no persistence, no credentials. The genuine security-relevant controls (input validation → no-panic, HTTPS-only fetch, explicit error classification) are already part of the locked decisions and the established codebase pattern.

## Sources

### Primary (HIGH confidence)
- docs.rs/qrcode/0.14.1 — `QrCode::{new, with_error_correction_level}`, `render::<P>()`, `render::unicode::Dense1x2` (`Pixel`, `Image = String`, `Dark`/`Light`, `default_color`), `Renderer::{quiet_zone, dark_color, light_color, module_dimensions, build}`, `EcLevel`
- docs.rs/arboard/3.6.1 — `Clipboard::{new, set_text<T: Into<Cow<str>>>, get_text}` (all `&mut self`), `Error`
- docs.rs/tauri-winrt-notification/0.7.2 — `Toast::{new, title, text1, text2, show}` (owned-self builder), `POWERSHELL_APP_ID`
- docs.rs/ureq/3.3.0 — `get().call() -> Result<Response<Body>, Error>` (non-2xx = `Err(StatusCode)` default), `Error::{StatusCode, Io, ConnectionFailed, HostNotFound}`, `Body::{into_reader('static), as_reader, read_to_string}`, `Response::into_body`
- crates.io sparse index (queried 2026-06-24) — confirmed max versions: qrcode 0.14.1, arboard 3.6.1, tauri-winrt-notification 0.7.2, ureq 3.3.0 + default-feature sets + dep trees
- slopcheck `scan --pkg crates.io <name> --json` — all four `status: OK`, zero flags
- **Live Open-Meteo API (queried 2026-06-24)** — forecast (metric + imperial) and geocoding (hit + no-match) response shapes: field names, types, the `current_units` labels (incl. imperial `"mp/h"`), and the absent-`results`-on-no-match behavior
- Local codebase (read in full) — `src/main.rs`, `src/cli.rs`, `src/core/output.rs`, `src/core/input.rs`, `src/commands/{matrix,color}/mod.rs`, `tests/json.rs`, `Cargo.toml`, `.planning/{STATE.md,REQUIREMENTS.md,config.json}`, `05-CONTEXT.md`, `04-RESEARCH.md`

### Secondary (MEDIUM confidence)
- `Dense1x2` default polarity (dark-on-light, verify-during-impl) — inferred from `default_color()` existence + D-02's own verify step; not a hard guarantee (A1)
- arboard Windows-persistence-after-exit — D-06's Win32 reasoning (cross-checked against the documented `SetClipboardData` ownership model)

### Tertiary (LOW confidence)
- none — every claim cross-verified against docs.rs, the crates.io sparse index, the live API, or the in-tree source

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all 4 (+serde) versions are the current crates.io latest for their line; all pass slopcheck OK; all canonical with verified source repos
- API surface: HIGH — every locked symbol (qrcode `Dense1x2`/`Renderer`, arboard `Clipboard`, Toast builder, ureq `Body`/`Error`) verified against docs.rs for the exact pinned version; the two override decisions confirmed sound
- Open-Meteo shapes: HIGH — captured from the LIVE API (not training data); the three landmines (absent `results`, `"mp/h"` label, ureq non-2xx-as-error) are observed facts
- Architecture/reuse mapping: HIGH — read the actual `core::{output,input}` + the matrix `RawGuard`/poll-as-timer source pomodoro reuses; symbols cited by line
- Pitfalls: HIGH — each tied to a verified API fact (`&mut self`, `Error::StatusCode` default, absent `results`) or a recorded STATE.md/CONTEXT decision

**Research date:** 2026-06-24
**Valid until:** ~2026-07-24 (30 days; all crates stable, pinned versions are current latest, Open-Meteo schema stable)
