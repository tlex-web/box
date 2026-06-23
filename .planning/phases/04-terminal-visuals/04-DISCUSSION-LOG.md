# Phase 4: Terminal Visuals - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-23
**Phase:** 4-terminal-visuals
**Mode:** advisor (USER-PROFILE.md present; calibration tier full_maturity; technical owner). Each selected area was researched by a parallel `gsd-advisor-researcher` agent; user chose from scored comparison tables.
**Areas discussed:** ascii engine & color, json ordering & coloring, matrix charset & exit, lolcat gradient & ANSI

---

## ascii — rendering engine

| Option | Description | Selected |
|--------|-------------|----------|
| Hand-roll on `image` | Zero-extra-stack; `image::open` → `resize_exact` → `to_luma8` → ramp; feeds `terminal_width()`; ~40–60 LOC. (Adds only `image`, the legitimate trap-to-hand-roll dep.) | ✓ |
| `artem` 3.0.0 crate | Turnkey `convert()` but unconditionally drags clap/colored/terminal_size/log/env_logger/anstyle-svg/once_cell (+ureq/TLS via default web_image); bypasses `terminal_width()`; MPL-2.0 | |
| rascii_art / image-to-ascii / artistic | Not viable (binary-only / BDF fonts / dead 2020) | |

**User's choice:** Hand-roll on `image` (Recommended).
**Notes:** Color is monochrome in v1 either way (colored ASCII = VIS-V2-01). Correction recorded in CONTEXT D-01: `image` is a NEW dep, not already vendored — hand-rolling adds exactly one crate vs artem's 8-crate tail.

---

## json — formatter internals

| Option | Description | Selected |
|--------|-------------|----------|
| `preserve_order` + default numbers | Keep input key order (IndexMap); default i64/u64/f64; avoids the arbitrary_precision × preserve_order landmine | ✓ |
| `preserve_order` + `arbitrary_precision` | Also exact big-number fidelity; accept serde feature-interaction risk (#505/#721/#845) | |
| Sorted keys (serde_json default) | One fewer dep, but BTreeMap reorders object keys alphabetically (surprising for a formatter) | |

**User's choice:** preserve_order + default numbers (Recommended).
**Notes:** Coloring locked to a hand-rolled owo-colors colorizer gated via `is_color_on()` regardless — `colored_json` was excluded up front because its `yansi` stack bypasses the gate and would break the byte-identical-minus-ANSI-when-piped rule. `arbitrary_precision` deferred behind a future flag.

---

## matrix — glyph set (loop + teardown locked best-practice)

| Option | Description | Selected |
|--------|-------------|----------|
| Latin letters + digits | Zero tofu on default Cascadia Mono; robustly meets "no visual artifacts"; least authentic (was the Recommended/safe option) | |
| Mixed: ASCII + sprinkled katakana | Matrix flavor where font supports; partial tofu on non-JP fonts | |
| Halfwidth katakana (authentic) | Iconic look, single-cell; renders as tofu on the default Windows font (conhost does no glyph fallback) | ✓ |
| ASCII printable (full symbols) | Universal but busier "hacker" look | |

**User's choice:** Halfwidth katakana (authentic) — the design-conscious call, accepting the font tradeoff.
**Follow-up (font-availability risk):** chose "Document font requirement, ship pure katakana" over "add a Latin fallback flag" or "blend with ASCII." → v1 ships pure katakana; `--help`/README notes a CJK-capable font is needed for the authentic look; default-font tofu is a documented cosmetic limitation; no charset flag (deferred to VIS-V2). Clarified that MTRX-01's "no visual artifacts" = clean teardown (RAII guard handles on any font), independent of tofu.
**Notes (locked, not chosen — no real alternative):** alt-screen + hide cursor + raw mode; full-frame buffer flushed once per frame @ ~20 FPS; `event::poll(50ms)` as combined frame-timer + input; Ctrl+C (KeyEvent in raw mode, not SIGINT) / q / Esc exit; `KeyEventKind::Press` filter (Windows double-fires); RAII Drop guard restores on normal/error/panic; no `ctrlc` crate.

---

## lolcat — gradient & input

| Option | Description | Selected |
|--------|-------------|----------|
| Full: +unicode-width +strip-ansi-escapes | Per-char coloring by display width (correct CJK/wide/combining) + strip pre-existing ANSI so already-colored input isn't garbled | ✓ |
| Strip ANSI only (skip unicode-width) | Handle colored input; chars() advance-by-1; accept slight CJK mis-spacing | |
| Zero new deps | Leanest; mis-spaces CJK and garbles already-ANSI input | |

**User's choice:** Full — +unicode-width +strip-ansi-escapes (Recommended).
**Notes:** Gradient algorithm near-locked to the classic diagonal sine-wave RGB (horizontal-only → vertical stripes fails the multi-line criterion; HSV → not the lolcat look + extra code/dep). Fixed freq/seed (flags = VIS-V2). Gated via `is_color_on()` so piping to a file yields clean plain text. strip-ansi-escapes actions the standing STATE.md todo.

---

## Claude's Discretion

- Module layout per command; `read_input` vs `read_file_or_stdin` for json/lolcat.
- ascii ramp string + FilterType + luma weighting + whether to trim `image` features.
- json per-token color shades + exact error wording within `error at line L column C: …`.
- matrix trail-length range, speed model, head/trail RGB shades, exact FPS (~15–25), katakana sub-range/density.
- lolcat exact freq/spread constants (freq≈0.1, spread≈3.0) + starting phase.

## Deferred Ideas

- VIS-V2-01: lolcat `--animate`/`--freq`/`--seed`; matrix color/speed/charset (incl. the considered Latin fallback flag); ascii color/braille/invert.
- json `arbitrary_precision` (behind a future flag).
- Out of Scope: ascii video/GIF/URL; json full jq query language.
