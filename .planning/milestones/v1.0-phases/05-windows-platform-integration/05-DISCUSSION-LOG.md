# Phase 5: Windows Platform Integration - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-24
**Phase:** 5-windows-platform-integration
**Areas discussed:** Pomodoro & toast, Weather units/API, QR rendering, Clipboard UX
**Mode:** advisor (calibration: full_maturity; 4 parallel research agents, opus)

---

## Pomodoro — timer model

| Option | Description | Selected |
|--------|-------------|----------|
| Raw-mode `RawGuard` (reuse matrix) | `event::poll(remaining)` as both timer + keypress reader; distinguishes cancel (exit 1, no toast) from completion (toast, exit 0); RAII Drop restore on every path; 0 new deps | ✓ |
| `\r` line-rewrite + default Ctrl+C | ~15-line print loop; SIGINT kills mid-line (dangling line/cursor), can't listen for Esc, clean cancel needs new `ctrlc` dep | |

**User's choice:** Raw-mode `RawGuard`.
**Notes:** POMO-01 binds *both* "cancels cleanly" and "toast on completion" — only the poll-as-timer loop tells the two outcomes apart deterministically, and it reuses the proven matrix pattern.

---

## Pomodoro — toast delivery (standing compile-spike)

| Option | Description | Selected |
|--------|-------------|----------|
| `tauri-winrt-notification` 0.7.2 | Maintained drop-in fork; identical API, no AUMID registration, modern `windows 0.61`, ~1.75M recent downloads; LOW risk | ✓ |
| `winrt-notification` 0.5.1 (CLAUDE.md literal) | Same API, verified compiles, but stale since 2022 on abandoned `windows 0.24` + dead `syn 1.x`; MEDIUM risk | |
| PowerShell `New-BurntToastNotification` shell-out | Zero Rust deps, but BurntToast needs `Install-Module` + PS7 Core dropped built-in WinRT; HIGH runtime risk | |

**User's choice:** `tauri-winrt-notification` 0.7.2.
**Notes:** Standing spike resolved empirically — both Rust crates compile clean on rustc 1.90 / MSVC, so the axis is maintenance, not compilation. Fallback ladder recorded (→ winrt-notification 0.5.1 → winrt-toast-reborn 0.3.8 → PowerShell). Overrides CLAUDE.md's documented winrt-notification → winrt-toast fallback.

---

## Weather — default unit system (standing todo)

| Option | Description | Selected |
|--------|-------------|----------|
| Metric °C/km/h (`--units imperial` overrides) | Deterministic + testable (no env reads); global+SI majority; Open-Meteo's own API default (shortest no-flag request); 0 new deps | ✓ |
| Imperial °F/mph | Convenient if US-based; wrong for ~95% of the world; forces unit params on the common path | |
| OS-locale detect | Zero-flag local convenience; non-deterministic across machines (poisons snapshot tests); needs a Windows region dep; map is "metric + 3-country exception" | |

**User's choice:** Metric °C/km/h default.
**Notes:** Locked sub-recommendations (no decision needed): server-side unit params (no client conversion); parse-shape city-vs-`lat,lon` disambiguation; geocode `count=1` + echo resolved location to stderr; WMO `weather_code` → `&'static str` match; `serde_json::from_reader` (avoids ureq `json` feature); offline/not-found → stderr + exit 1. Per-user unit preference deferred to v2 stored-config.

---

## QR — rendering engine

| Option | Description | Selected |
|--------|-------------|----------|
| `qrcode` 0.14.1 + `Dense1x2` | Pure Unicode half-blocks, NO ANSI (matches glyph-is-truth + `is_color_on()`); full 4-module quiet zone; explicit dark-on-light polarity; owned String → stdout + `--file`; `EcLevel::M` | ✓ |
| `qrcode` direct (hand-render) | Total control, but reinvents what `Dense1x2` already does correctly | |
| `qr2term` 0.3.3 (CLAUDE.md pick) | One-line API, but always emits ANSI (ungated crossterm) → `box qr > file` writes raw escapes; polarity in bg color not glyph; sub-spec 2-module quiet zone | |

**User's choice:** `qrcode` 0.14.1 + `Dense1x2`.
**Notes:** Overrides the CLAUDE.md `qr2term` pre-pick for the same reason `ascii` overrode `artem` — qr2term's unconditional ANSI + bg-color polarity break the locked output contracts.

---

## Clipboard — mode / flag design

| Option | Description | Selected |
|--------|-------------|----------|
| `--paste` flag, copy-by-default | Matches CLIP-01 verbatim; lowest surface; sidesteps the `read_input` no-arg-TTY→exit-2 contract (copy reads raw stdin bytes) | ✓ |
| Auto-detect by stream | Most ergonomic, but collides with the `read_input` TTY contract; Windows `is_terminal()` fragile under nested pipes/ptys | |
| Subcommands `copy`/`paste` | Self-documenting, scales to v2 image clipboard, but heavier than warranted for a 2-state v1 tool (image clipboard is Out of Scope) | |

**User's choice:** `--paste` flag, copy-by-default.
**Notes:** arboard Windows persistence confirmed safe (OS owns the handle after `set_text`; X11 drop problem doesn't apply); main-thread-only satisfied by the synchronous flow.

---

## Clipboard — trailing-newline policy

| Option | Description | Selected |
|--------|-------------|----------|
| Strip one trailing newline on copy | Strips at most one `\r\n`/`\n` (never interior/multiple); `--paste` emits bytes exactly; mirrors pbcopy/xclip; documented in `--help`, future `--raw` opt-out | ✓ |
| Preserve bytes exactly | Maximal byte-fidelity, but PowerShell's appended CRLF lands on the clipboard → stray newline when pasting into a field | |

**User's choice:** Strip one trailing newline on copy.
**Notes:** Load-bearing fact — PS7 appends an implicit CRLF when piping a string to a native command, so `"text" | box clip` actually sends `text\r\n`.

---

## Claude's Discretion

- Per-command module layout (`src/commands/<cmd>/mod.rs`); stdout layout/labels for `weather` and `qr`.
- `pomodoro`: countdown render format (`MM:SS`), whether/how to color the digits (gated), toast title/body wording.
- `weather`: exact stderr wording within the locked shapes; whether to color conditions/temperature (gated); whether to surface geocoded `admin1`/`country` in the stdout header.
- `clip`: exact `--help` wording for the trailing-newline trim note.

## Deferred Ideas

- **VIS-V2-01:** QR `--save`-to-image + configurable error-correction.
- **SYS-V2-01:** pomodoro session counter/auto-break/sound/`--label`; weather `--forecast`/`--json`/cache/stored-location incl. per-user unit preference + OS-locale detection.
- **DEV-V2-01:** `--clip` integration across passgen/uuid/color.
- **Out of Scope:** qr decode/scan; clip binary/image clipboard; weather API-key/radar/maps.
