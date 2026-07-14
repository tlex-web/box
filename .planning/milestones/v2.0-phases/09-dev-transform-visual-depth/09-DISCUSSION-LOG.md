# Phase 9: Dev-Transform & Visual Depth - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-28
**Phase:** 9-Dev-Transform & Visual Depth
**Mode:** Advisor (research-backed comparison tables; standard calibration tier; technical owner)
**Areas discussed:** color naming & HSL input, epoch relative time & --tz, ascii color/braille/invert, lolcat --animate model

> Nine commands across two clusters were distilled into four genuinely-open gray areas. The user selected all four to discuss; each was researched in parallel by a `gsd-advisor-researcher` (model opus) surveying the box source + comparable CLI tools (pastel, artem/chafa/viu, git/gh, real lolcat). All four resolved to the research-recommended option.

---

## color: naming & HSL input (COLR-V2-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid: exact + nearest | `name` = exact CSS keyword or null; `nearest` = closest via hand-rolled weighted-RGB (redmean) distance, marked `~`; both always in JSON. Useful for any color, still honest (matches pastel). | ✓ |
| Exact-match only | Only the ~148 CSS keywords resolve; arbitrary hex → `name: null`. Honest/smallest but returns nothing for ~99% of real colors. | |
| Nearest only | Every color maps to its closest CSS name (no null). Always answers but labels arbitrary colors without an approximate marker. | |

**User's choice:** Hybrid: exact + nearest (Recommended)
**Locked-as-recommended sub-decisions (not separately polled):** HSL input = CSS functional `hsl(210, 100%, 50%)` (+ space form), routed by an `hsl(` prefix check BEFORE the RGB branch (avoids the bare-triple RGB collision) + a hand-rolled `hsl_to_rgb()`; named list = hand-rolled `const` ~148-entry CSS Color Level 4 table (no crate).
**Notes:** redmean distance is the chosen hand-roll (not Euclidean, not full CIEDE2000 — Lab/CIEDE2000 deferred as the accuracy upgrade). `name`/`nearest` always-present in the JSON for a stable PS7-script schema.

---

## epoch: relative time & --tz (EPOC-V2-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Always-on, date path only | Append `(3 hours ago)`/`(in 2 days)`/`(just now)` to the integer→date human lines by default; `now`/date-string modes stay bare integers for piping; JSON `relative` always present. | ✓ |
| Behind a `--relative` flag | Default output stays byte-stable; relative shown only when asked (git --date=relative style). Preserves golden output but hides the headline feature. | |

**User's choice:** Always-on, date path only (Recommended)
**Locked-as-recommended sub-decisions (not separately polled):** humanizer = hand-rolled ~30-line `relative_for(epoch, now)` (handles future + "just now", which `timeago` omits; no crate); `--tz <zone>` ADDS a third labeled line (Local/UTC/<zone>), validated via `chrono_tz::Tz::from_str` → clean exit 1; JSON gains a flat `tz` field via `skip_serializing_if`, rendered `%Z %z`.
**Notes:** `relative` is confined to the human date path so scripting (bare-integer) outputs stay clean. Add `chrono-tz` (committed in D-1, not yet in Cargo.toml).

---

## ascii: color / braille / invert (ASCI-V2-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Default-on, gated on is_color_on() | Truecolor in a TTY, auto-degrades to the mono ramp when piped/redirected/NO_COLOR. Matches lolcat+color and artem/chafa/viu. No new flag; SC4 byte-identity for free. | ✓ |
| Opt-in via `--color` | Mono stays the default; color only with `--color`. Conservative but contradicts ASCI-V2-01's "produces truecolor" wording and duplicates the gate. | |

**User's choice:** Default-on, gated on is_color_on() (Recommended)
**Locked-as-recommended sub-decisions (not separately polled):** `--braille` = hand-rolled 2×4 Unicode-braille engine (`U+2800 + bitmask`, fixed 50% per-dot luma threshold) that REPLACES the ramp, color via one averaged `.truecolor()` per cell; `--invert` = `255 - luma` at the single luma seam, orthogonal to color/braille.
**Notes:** Sample per-cell RGB via `.to_rgb8()` (keep luma for the ramp index). Braille threshold kept a swappable const (Otsu/mean deferred). No crate (drawille-style rejected per the image-only hand-roll exception).

---

## lolcat: --animate model (LOL-V2-01 — PS7 HUMAN-VERIFY gate)

| Option | Description | Selected |
|--------|-------------|----------|
| Bounded alt-screen, persist final frame | Reuse matrix's RAII RawGuard/poll/Press-only verbatim; run until `--duration` or q/Esc/Ctrl+C, then leave alt-screen and reprint ONE static frame so the rainbow persists. `--duration 0` = until-keypress. Cleanest restore + passes human-verify. | ✓ |
| Infinite alt-screen (matrix-style) | Run until keypress only; simplest reuse but alt-screen restores the pre-run buffer on exit so the rainbow vanishes (ephemeral, no persisted result). | |
| Real-lolcat in-place reprint | Authentic; final frame persists in scrollback. But MoveUp miscounts on line-wrap (known lolcat bug) → corrupts scrollback, fragile across PS7 widths, risks failing the human-verify gate. | |

**User's choice:** Bounded alt-screen, persist final frame (Recommended)
**Locked-as-recommended sub-decisions (not separately polled):** animation = advance one global phase offset per frame through a parameterized `rgb_at(phase, freq)`; `--seed` = initial offset; `--freq`/`--seed` apply to the static render too (one gradient path); degrade to the existing static one-pass render when `!(is_terminal() && is_color_on())` or under `--json`/`--clip` (the `is_terminal()` AND-gate is mandatory — `is_color_on()` alone can be force-true on a pipe).
**Notes:** Reuses matrix's RawGuard + `event::poll(50ms)`-as-timer + single-flush-per-frame + Press-only quit verbatim. Avoids the authentic in-place `MoveUp` reprint specifically because its line-wrap miscount (busyloop/lolcat#116, kitty#2813) is fragile to human-verify on PS7.

---

## Claude's Discretion

Pre-stated with defaults (recorded in CONTEXT.md so the planner has the whole phase in one place) — not discussed:

- **`uuid` v7 + format flags (UUID-V2-01)** — `--v7` via `Uuid::now_v7()`; `--braces`/`--urn` mutually exclusive (`conflicts_with`); `--upper`/`--no-hyphens` are modifiers; `uuid` JSON field = the formatted value (single-source `format_one`).
- **`json --sort-keys` (JSON-V2-01)** — opt-in, recursive key sort before the pretty/compact/colorize/emit_json fork; `preserve_order` stays default (never implicit).
- **`passgen` entropy + `--no-similar` + `--separator` (PASS-V2-01)** — pool-based entropy bits (stderr + `entropy_bits` JSON); `--no-similar` drops `il1Lo0O`; `--separator` overrides the hardcoded `.`.
- **`matrix --color`/`--speed`/`--charset` (MTRX-V2-01)** — named-preset ValueEnum color, discrete speed levels, preset-or-custom-string charset.
- **`qr --save` + EC (QR-V2-01)** — extension-detected PNG/SVG, `--error-correction L|M|Q|H` (default M); re-enable qrcode `image`/`svg` features or use the present `image` crate.

## Deferred Ideas

- `color` perceptual nearest via CIEDE2000/Lab (redmean ships first; Lab is the accuracy upgrade).
- `ascii` adaptive braille threshold (Otsu/per-cell-mean); sixel/kitty-graphics output (VIS-V3).
- `lolcat`/`matrix` advanced animation modes; matrix arbitrary-hex `--color` (presets first) — VIS-V3.
- `qr` output formats beyond PNG/SVG.
