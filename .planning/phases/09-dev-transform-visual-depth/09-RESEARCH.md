# Phase 9: Dev-Transform & Visual Depth - Research

**Researched:** 2026-06-28
**Domain:** Rust CLI depth flags (clap/uuid/chrono-tz/qrcode/image/crossterm) on a frozen `--json`/`is_color_on()` spine — Windows PowerShell 7
**Confidence:** HIGH (every seam verified against the real source at the claimed file/line; every new-dependency mechanic verified against the cached crate source or crates.io API)

<user_constraints>
## User Constraints (from CONTEXT.md)

> This is a "deepening not rewrite" phase. CONTEXT.md already named every code seam, every reuse-verbatim pattern, and every new dependency. The four discussed areas (D9-01..D9-04) are LOCKED; the five non-discussed commands have pre-stated defaults under Claude's Discretion. **Do not re-litigate.**

### Locked Decisions

**D9-01 — `color`: hybrid naming + CSS-functional HSL input (COLR-V2-01)**
- HSL input syntax = CSS functional `hsl(H, S%, L%)` (also accept the modern space form `hsl(H S% L%)`). Route it by an `hsl(` **prefix check placed BEFORE** the comma/whitespace RGB branch in `parse_color`. Add a hand-rolled `hsl_to_rgb()` inverse mirroring the existing `rgb_to_hsl()`. `H` 0–360, `S`/`L` 0–100 (percent). Name→color resolves via the table → rgb.
- Named-color list = hand-rolled `const` table of the ~148 CSS Color Module Level 4 names (incl. `rebeccapurple`), as `&[(&str, (u8,u8,u8))]`. **No crate.** Guard with a unit test against anchors (`black`, `white`, `rebeccapurple`, `cornflowerblue`).
- hex→name policy = HYBRID (honest + useful). `name` = exact CSS keyword for the resolved RGB, or `null`; `nearest` = closest keyword via a hand-rolled weighted-RGB "redmean" distance (~10 lines — NOT plain Euclidean, NOT CIEDE2000/Lab). Human block marks an approximate as `~name` vs an exact `name`.
- JSON (additive to the LOCKED `{hex, rgb:{r,g,b}, hsl:{h,s,l}}`, D-17): add `name` (`string|null`) and `nearest` (`string`), **both always-present** (stable schema).

**D9-02 — `epoch`: always-on relative time + additive `--tz` (EPOC-V2-01)**
- Relative time = ALWAYS-ON, confined to the integer→date human path. Append `(3 hours ago)` / `(in 2 days)` / `(just now)` to the `Local:`/`UTC:` lines. The `now` and date-string modes keep emitting bare integers (scripting-clean).
- Humanizer = hand-rolled ~30-line `relative_for(epoch, now)` threshold ladder with a future `in N …` sign branch. **No crate.** ONE helper feeds both the human suffix and the JSON `relative` field.
- `--tz <zone>` = ADD a third labeled line (`Local:` / `UTC:` / `<zone>:`) — Local/UTC stay anchors. Validate IANA name via `chrono_tz::Tz::from_str` → `bail!` clean exit 1 with a hint, never a panic. Zoned datetime shares `DateTime::from_timestamp`.
- JSON (additive to LOCKED `{epoch, utc, local}`, D-20): `relative: String` always-present (clock-dependent → assert FORMAT not value); `tz` field present **only under `--tz`** via `#[serde(skip_serializing_if = "Option::is_none")]`, rendered `%Z %z`. Flat object (D-01).
- Dep: add `chrono-tz` with its default tz-database feature.

**D9-03 — `ascii`: truecolor default-on + hand-rolled braille + invert (ASCI-V2-01)**
- truecolor = DEFAULT-ON, gated SOLELY on `is_color_on()`. Auto-degrade to the existing mono `RAMP` when piped/redirected/`NO_COLOR`/`--json`. **No `--color` flag.** Sample per-cell RGB via `.to_rgb8()` (keep luma for the ramp index); emit `.truecolor(r,g,b)` on the ramp glyph.
- `--braille` = hand-rolled 2×4 Unicode-braille engine that REPLACES the ramp. `char = U+2800 + bitmask`, little-endian dot→bit mapping (dots 1-2-3 = bits 0-1-2 left column, 4-5-6 = bits 3-4-5 right column, 7/8 = bits 6/7 bottom row). Resize to `2*cols × 4*rows`; per-dot fixed 50% luma threshold as a swappable `const`. Color = average the 8 sub-pixels into one RGB → one gated `.truecolor()` per braille glyph. Lock the bit-order with a unit test. **No crate.**
- `--invert` = `255 - luma` at the single luma seam BEFORE the ramp index / braille per-dot threshold.
- Stays display-only (SC4 — no `--json`); new color paths gated on `is_color_on()`.

**D9-04 — `lolcat --animate`: bounded alt-screen, persist final frame (LOL-V2-01, HUMAN-VERIFY)**
- Loop model = BOUNDED alternate-screen + persist-final-frame. Reuse `matrix`'s teardown VERBATIM: `RawGuard` RAII armed the instant `enable_raw_mode()` succeeds (BEFORE the fallible `EnterAlternateScreen`/`Hide`), `event::poll(50ms)` as BOTH the ~20-FPS frame timer AND input gate, a single `stdout.flush()` per frame, `KeyEventKind::Press`-only `is_quit` (q/Esc/Ctrl+C). Run until a `--duration` deadline (`Instant`-based, default a few seconds) OR a quit key; then `LeaveAlternateScreen` and reprint ONE final static frame to the normal buffer so the colored text PERSISTS. `--duration 0` = run until keypress. (Avoids the line-wrap-fragile in-place `MoveUp` reprint.)
- Animation mechanism: advance ONE global phase offset per frame; parameterize `rgb_at(phase, freq)` (replaces the hard-coded `0.1`). `--seed` = the initial phase offset. `--freq`/`--seed` ALSO govern the static one-pass render (ONE gradient path).
- Degradation (SC3/SC4): enter the loop ONLY when `std::io::stdout().is_terminal() && is_color_on()` AND not under `--json`/`--clip`; otherwise dispatch to the EXISTING static one-pass renderer. The `is_terminal()` AND-gate is MANDATORY (`is_color_on()` alone can be forced true on a pipe via `CLICOLOR_FORCE`).
- Human-verify (PS7): smooth animated rainbow, clean exit on Ctrl+C/q/Esc with no stuck raw mode, `--freq`/`--seed` visibly change the gradient; degrades to static when piped/`--json`. Cleared by a human in PS7, not by automated test alone.

### Claude's Discretion (planner/executor latitude — defaults pre-stated)

- **`uuid` v7 + format flags (UUID-V2-01):** add the `uuid` `v7` feature; `--v7` → `Uuid::now_v7()` (else `new_v4`); `version` JSON field becomes `"v4"`/`"v7"`. Format flags: `--upper` (exists), `--no-hyphens`, `--braces` (`{…}`), `--urn` (`urn:uuid:…`). **`--braces` and `--urn` are `conflicts_with` each other** (fail loud, clap exit 2); `--upper`/`--no-hyphens` compose with any form. Extend the pure `format_one` to take the format options. Default: `uuid` JSON field carries the SAME formatted value the human line prints.
- **`json --sort-keys` (JSON-V2-01):** opt-in `--sort-keys`; **recursively** sort object keys (nested too) of the parsed `Value`; arrays keep order. `preserve_order` STAYS default — never sorted implicitly. Mechanism: a recursive `Value`-rewrite rebuilding each `Map` in sorted-key order BEFORE the pretty/`--compact`/`colorize`/`emit_json` fork. `--json --sort-keys` emits a sorted document too.
- **`passgen` entropy + `--no-similar` + `--separator` (PASS-V2-01):** entropy = theoretical pool-based bits — char mode `length * log2(pool_size)`, passphrase mode `words * log2(7776)` (~12.92 bits/word). **Display to STDERR** for the human path (keeps secret-only-on-stdout D-14 + JSON purity) and add a top-level `entropy_bits` to the `{results,count}` JSON (per-config). `--no-similar` drops `il1Lo0O` and recomputes `pool_size`. `--separator <str>` overrides the hardcoded `.` passphrase join (default stays `.`). RNG untouched (T-V6).
- **`matrix --color`/`--speed`/`--charset` (MTRX-V2-01):** `--color` = named-preset `ValueEnum` (green [default]/red/blue/cyan/magenta/yellow/white) → head/fade RGB (NOT arbitrary hex). `--speed` = discrete levels (slow/normal [default]/fast) → poll interval / `SPEED_MIN..MAX`. `--charset` = named presets (katakana [default]/ascii/binary/digits) OR a literal custom string. All color gated on `is_color_on()`; `RawGuard`/loop/quit logic untouched. Display-only (SC4).
- **`qr --save` + EC (QR-V2-01):** `--save <file>` infers format from extension — `.png` (raster) and `.svg` (text). Re-enable the qrcode `image`/`svg` features OR rasterize via the present `image` crate (planner picks the lighter binary path — see Standard Stack below). `--error-correction L|M|Q|H` → `EcLevel::{L,M,Q,H}` (default stays M), applied to BOTH terminal render and `--save`. PNG module scale / quiet-zone = sensible default (~8 px/module, 4-module quiet zone). On `--save`: write the file + a stderr confirmation and suppress the terminal glyph block; JSON `error_correction` reflects the chosen level + add `saved_path` when `--save`.

### Deferred Ideas (OUT OF SCOPE)
- `color` perceptual nearest-color (CIEDE2000/Lab) — redmean is the v2 hand-roll.
- `ascii` adaptive braille threshold (Otsu / per-cell mean); sixel / kitty-graphics = VIS-V3.
- `lolcat`/`matrix` advanced animation modes (lolcat vertical-scroll, matrix arbitrary-hex `--color`) = VIS-V3.
- `qr` formats beyond PNG/SVG (PDF/EPS).
- `uuid` v6/v8 and wrapping combinatorics beyond the two forms.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UUID-V2-01 | `uuid --v7` + `--upper`/`--no-hyphens`/`--braces`/`--urn` | uuid `v7` feature gates `Uuid::now_v7()` (verified); `.hyphenated()`/`.simple()`/`.braced()`/`.urn()` are always-available `const fn` (verified, no feature). Seam: extend pure `format_one` at `uuid/mod.rs:88`. |
| EPOC-V2-01 | `epoch` relative time + `--tz <zone>` | `chrono-tz 0.10.4` (`Tz: FromStr`, requires chrono ^0.4.25 — project has 0.4.45, compatible). Seam: extend `epoch_output`/`format_timestamp` (`epoch/mod.rs:141`/`155`), add `relative_for`. |
| COLR-V2-01 | `color` CSS names (both ways) + HSL input | `parse_color` auto-detect ordering (`color/mod.rs:131-147`) is exactly where the `hsl(` branch + named-color branch slot in; `rgb_to_hsl` (`color/mod.rs:199`) is the template the new `hsl_to_rgb` mirrors. Hand-rolled CSS table + redmean. |
| JSON-V2-01 | `json --sort-keys` (opt-in) | serde_json `preserve_order` makes `Value`'s map an `IndexMap`; a recursive rebuild-in-sorted-order pass before the fork at `json/mod.rs:69-90`. No dep change. |
| PASS-V2-01 | `passgen` entropy + `--no-similar` + `--separator` | Pure `build_charset` (`passgen/mod.rs:176`) + the `phrase.join(".")` seam (`passgen/mod.rs:138`). entropy → stderr + `entropy_bits` JSON field. `log2(7776)=12.925`. |
| LOL-V2-01 | `lolcat --animate`/`--freq`/`--seed` (HUMAN-VERIFY PS7) | `matrix`'s `RawGuard`/`event::poll(50ms)`/single-flush/`is_quit` (`matrix/mod.rs:117-322`) copied verbatim; `rgb_at` (`lolcat/mod.rs:128`) parameterized to `rgb_at(phase, freq)`. Static render is the degrade path. |
| MTRX-V2-01 | `matrix --color`/`--speed`/`--charset` | `HEAD_RGB`/`FADE_*` consts (`matrix/mod.rs:90-94`), `SPEED_MIN/MAX` (`:85-87`), `katakana_glyphs()` (`:306`) become preset-driven; `RawGuard`/loop/`is_quit` untouched. |
| QR-V2-01 | `qr --save` (PNG/SVG) + `--error-correction` | Pure `render_qr` seam (`qr/mod.rs:106`) + `EcLevel` (already imported `:43`). Re-enable qrcode `["image","svg"]` (verified: unifies with the present `image 0.25.10`; `svg` is zero-dep). `saved_path` JSON addition. |
| ASCI-V2-01 | `ascii` truecolor + `--braille` (2×4) + `--invert` | `luma_to_char` (`ascii/mod.rs:104`) is the single luma seam (`--invert` flips before it); `to_rgb8()` for color; braille replaces the ramp. Module must newly import `is_color_on` + `owo_colors`. |
</phase_requirements>

## Summary

Phase 9 adds depth flags to nine commands across two clusters — dev-transform (`uuid`, `epoch`, `color`, `json`, `passgen`) and visual (`lolcat`, `matrix`, `qr`, `ascii`). It is verification research, not design: CONTEXT.md already locked every decision and named every seam. I read all nine command sources, the frozen spine (`core/output.rs`, `core/input.rs`), `Cargo.toml`, and the three milestone blueprints, then verified each claimed seam exists at the claimed line and each new-dependency mechanic resolves. **No BLOCKER found — every locked decision is cleanly buildable against the real code.**

The phase touches exactly **one new crate** (`chrono-tz`, already committed in REQUIREMENTS D-1), **one feature-flag edit** (`uuid` gains `"v7"`), and **one feature re-enable** (`qrcode` regains `["image","svg"]`). Everything else — CSS color names, HSL inverse, relative-time humanizer, recursive key-sort, entropy math, braille bitmask, matrix presets, lolcat animation — is a hand-roll over crates already in the manifest, exactly matching the project's hand-roll ethos. The two genuinely-new surfaces are the **`lolcat --animate` terminal loop** (the headline PS7 human-verify risk) and the **`qr --save` PNG/SVG file write** (the only new filesystem I/O). Every new colored path (matrix preset, ascii truecolor, animated lolcat) attaches in the single `is_color_on()` slot, so SC4 byte-identity-minus-ANSI falls out for free.

**Primary recommendation:** Build it as the 3-plan ROADMAP sketch (09-01 dev transforms / 09-02 visuals / 09-03 lolcat human-verify). For `qr --save`, **re-enable the qrcode `["image","svg"]` features** (NOT a hand-rolled rasterizer) — the `image` feature unifies with the already-linked `image 0.25.10` (zero new build), `svg` is a zero-dependency String builder, and you get both formats from the crate's tested renderers with built-in quiet-zone/module-sizing. Add `chrono-tz = "0.10"`, flip `uuid` to `features = ["v4","v7"]`, and confirm the one format detail that needs a unit test: `chrono-tz`'s `%Z` abbreviation output.

## Architectural Responsibility Map

This is a single Rust binary, so "tiers" are the internal layers a flag's logic must split across. The map below is the sanity-check the planner and plan-checker use to confirm each new flag lands in the right layer — and stays out of the frozen spine.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Flag declaration / `ValueEnum` / `conflicts_with` | CLI/Arg (clap derive on each `Args` struct) | — | Per-command args only; **no `cli.rs`/`main.rs` change** (global `--json`/`--clip` + `init_output`/`flush_clip` already wired). |
| Pure transform/render (the unit-test seam) | Pure fn tier (`format_one`, `rgb_at`, `luma_to_char`, `render_qr`, `hsl_to_rgb`, `relative_for`, `sort_value`, braille bitmask, entropy) | — | Every new behavior must extend an existing pure, terminal-free, crate-light fn so it stays unit-testable WITHOUT spawning the binary and feeds BOTH human + JSON (no-drift). |
| JSON document assembly | Output spine (`emit_json` via the command's `#[derive(Serialize)]` struct) | Pure fn tier | New fields are ADDITIVE to the frozen Phase-7 struct; the spine is **consumed, never modified** this phase. |
| Color gating | Output spine (`is_color_on()` — the SOLE gate) | — | `matrix --color`, `ascii` truecolor, animated `lolcat` ALL attach here → piped/`--json` byte-identical minus ANSI (SC4). No second color stack, no `owo_colors::set_override`. |
| Terminal animation loop | Terminal-loop tier (`crossterm` + `RawGuard` RAII) | Output spine (TTY+color gate decides entry) | ONLY `lolcat --animate`. Copies `matrix`'s loop verbatim; degrade path is the existing static renderer. |
| File output (PNG/SVG) | Filesystem I/O tier (`qrcode` image/svg → `std::fs`) | Pure fn tier (`render_qr` produces the matrix) | ONLY `qr --save`. The single genuinely-new I/O surface; must error cleanly (exit 1) on a bad extension / unwritable path, never panic. |
| Timezone resolution | Pure fn tier (`chrono_tz::Tz::from_str`) | — | ONLY `epoch --tz`. Validation `bail!`s exit 1 with a hint (mirrors `parse_date` discipline). |

**Anti-pattern guard (carried from ARCHITECTURE.md):** no new colored token may bypass `is_color_on()`; no JSON branch may re-derive values independently of the human render; no config-overridable default may use clap `default_value` (N/A this phase — no config tiers added).

## Standard Stack

### Core (dependency changes this phase)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `chrono-tz` | `0.10.4` | `epoch --tz <IANA zone>` | The canonical IANA-tz crate for `chrono`; `Tz: FromStr` + `Tz: chrono::TimeZone`. Pure-Rust (`phf`-based bundled DB), no C/system deps → `+crt-static`/PS7-clean. Committed in REQUIREMENTS D-1. `[CITED: crates.io/api/v1/crates/chrono-tz/0.10.4/dependencies]` |
| `uuid` | `1.23.3` + `"v7"` | `box uuid --v7` (`Uuid::now_v7()`) | Feature-flag edit only: `features = ["v4", "v7"]`. `now_v7()` is `#[cfg(feature = "v7")]` (verified in cached `uuid-1.23.3/src/v7.rs:17`). `[VERIFIED: crate source]` |
| `qrcode` | `0.14.1` + `["image","svg"]` | `qr --save out.png` / `out.svg` | Re-enable the two features dropped via `default-features = false`. `image` feature → transitive `image ^0.25` **unifies with the present `image 0.25.10`** (one build). `svg` feature = pure String builder, **zero new deps**. `[VERIFIED: crate source qrcode-0.14.1/Cargo.toml + render/{image,svg}.rs]` |

### Supporting (already present — reuse verbatim, NO change)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `image` | `0.25.10` (`png`,`jpeg`) | ascii `to_rgb8()` truecolor + braille resize; qr `--save` PNG encode | `to_rgb8()` for per-cell RGB; `resize_exact(2c,4r)` for braille; `.save("x.png")` works (png feature present). |
| `crossterm` | `0.29` | `lolcat --animate` loop; `matrix` (already uses it) | Reuse `RawGuard`/`event::poll`/`queue!`/`is_quit`. No new animation crate. |
| `owo-colors` | `4.3` | `ascii`/`lolcat`/`matrix` `.truecolor(r,g,b)` | `ascii` must newly `use owo_colors::OwoColorize;` (it currently imports neither owo nor `is_color_on`). |
| `serde_json` | `1.0.150` (`preserve_order`) | `json --sort-keys` recursive `Value` rewrite | `preserve_order` IS the reason sort must be explicit; arrays keep order, objects rebuilt sorted. No feature change. |
| `unicode-width` | `0.2` | lolcat width-aware phase advance (already used) | The animated path inherits the same per-scalar width logic. |
| `rand` | `0.9` (`OsRng`) | passgen RNG | **UNTOUCHED** (T-V6) — `--no-similar`/`--separator` only change the charset/join, never the draw. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Re-enable qrcode `["image","svg"]` | Hand-roll a rasterizer over the present `image` crate (iterate `code` modules → `ImageBuffer<Luma<u8>>`) | **Rejected.** Saves only the qrcode `image` *feature flag*, but `image 0.25.10` is ALREADY a direct dep (ascii) so that flag costs ~0 binary. And the SVG half of QR-V2-01 would STILL need the `svg` feature (or a second hand-rolled SVG writer) — so the hand-roll does NOT avoid a qrcode feature change. Net: more code, no binary saving. Re-enabling is the lighter path. |
| `chrono-tz` for `--tz` | Hand-roll fixed-offset parsing | Rejected — IANA names need DST rules; D-01 says never hand-roll TZ math (the existing `epoch` already delegates DST to chrono `Local`). |
| Named-color crate (e.g. `css-color`/`palette`) | — | Rejected per CONTEXT.md D9-01: a crate adds a dep, duplicates the existing hex/RGB parsers, AND lacks the reverse rgb→name lookup. Hand-rolled `const` table matches the humansize/qr2term/artem hand-roll ethos. |
| Relative-time crate (`timeago`/`chrono-humanize`) | — | Rejected per D9-02: `timeago` lacks future + "just now" and drags `isolang`; `chrono-humanize` is stale. ~30-line hand-roll. |
| Braille crate (`drawille`/`make_it_braille`) | — | Rejected per D9-03 + STACK.md: a trivial 2×4 bitmask doesn't justify a dep (the `image`-only hand-roll exception). |

**Installation (the only manifest edits this phase):**
```toml
# epoch --tz (EPOC-V2-01) — IANA tz database, default feature. Requires chrono ^0.4.25 (project: 0.4.45).
chrono-tz = "0.10"
# uuid --v7 (UUID-V2-01) — add "v7"; format methods (.hyphenated/.simple/.braced/.urn) need NO feature.
uuid = { version = "1.23.3", features = ["v4", "v7"] }
# qr --save PNG/SVG (QR-V2-01) — re-enable the two features dropped via default-features=false.
# `image` unifies with the present image 0.25.10 (one build); `svg` is a zero-dep String builder. Drop `pic`.
qrcode = { version = "0.14.1", default-features = false, features = ["image", "svg"] }
```

**Version verification performed:**
- `chrono-tz 0.10.4` — latest on the index (`cargo search`); requires `chrono ^0.4.25` (crates.io deps API). Project chrono = `0.4.45` ✓. Deps: `phf ^0.12` (tz table), `chrono ^0.4.25`; `serde`/`arbitrary`/`uncased` optional. Default feature = bundled IANA DB. `[VERIFIED: crates.io API + cargo search]`
- `uuid 1.23.3` — cached; `now_v7` gated on `feature="v7"` (`src/v7.rs:17`); `.hyphenated/.simple/.urn/.braced` are `pub const fn` in `src/fmt.rs:135-171`, **no feature gate** ✓. `[VERIFIED: crate source]`
- `qrcode 0.14.1` — cached; `[features] default = ["image","svg","pic"]`; `image = { version="0.25", default-features=false, optional=true }`; `svg`/`pic` are dep-free flags. `render/image.rs` is `#![cfg(feature="image")]`, `render/svg.rs` `#![cfg(feature="svg")]`. `[VERIFIED: crate source]`

## Package Legitimacy Audit

> slopcheck targets npm/PyPI; for Rust the equivalent gate is crates.io + `cargo`/lockfile verification. The one genuinely-new crate is `chrono-tz`, already committed in REQUIREMENTS D-1.

| Package | Registry | Age / Status | Downloads | Source Repo | Verification | Disposition |
|---------|----------|--------------|-----------|-------------|--------------|-------------|
| `chrono-tz` | crates.io | mature, actively maintained (0.10.4) | tens of millions (canonical chrono-tz crate) | github.com/chronotope/chrono-tz | crates.io deps API + cargo search | Approved (new) |
| `uuid` (+`v7`) | crates.io | already a dep (`1.23.3`) | de-facto standard | github.com/uuid-rs/uuid | crate source (cached) | Approved (feature edit) |
| `qrcode` (+`image`,`svg`) | crates.io | already a dep (`0.14.1`) | standard | github.com/kennytm/qrcode-rust | crate source (cached) | Approved (feature re-enable) |

**Packages removed (slopcheck [SLOP]):** none.
**Packages flagged suspicious ([SUS]):** none.
**Cross-ecosystem confusion risk:** none — all three are crates.io packages verified on the correct registry. No `postinstall`-equivalent (Rust crates have `build.rs`; `chrono-tz`'s `chrono-tz-build` is a build-dep only when the `filter-by-regex`/non-default path is used — the default bundled-DB path needs no build script run beyond the standard table generation, vendored in the published crate).

## Seam Verification (the load-bearing deliverable)

Each locked decision confirmed buildable against the real source. **All PASS — no BLOCKER.**

| Decision | Claimed seam | Verified | Notes for planner |
|----------|-------------|----------|-------------------|
| `color` `hsl(` branch BEFORE RGB | `parse_color` auto-detect, `color/mod.rs:131-147` | PASS | RGB branch is `if trimmed.contains(',') \|\| split_whitespace().count() > 1` (`:132`). `hsl(210, 100%, 50%)` contains commas → WOULD mis-hit RGB. The `hsl(` prefix check MUST precede `:132`. Insert `parse_hsl` + named-color lookup; named lookup goes BEFORE the final `bail!` (`:143`) and AFTER the hex check — **no collision** because no CSS name is an all-hex string of length 3/6. |
| `color` `hsl_to_rgb` mirrors `rgb_to_hsl` | `rgb_to_hsl`, `color/mod.rs:199` | PASS | Pure, crate-free, returns `(u16 h, u8 s, u8 l)`. The inverse is the standard closed form. Add a round-trip unit test (`rgb→hsl→rgb` within ±1). |
| `color` JSON additive `{name, nearest}` | `ColorOutput` struct, `color/mod.rs:50-55` | PASS | Add `name: Option<String>` (`null` when no exact) + `nearest: String`. D-19 hex stays lowercase-locked; human `Hex` row stays uppercase. |
| `epoch` shared-math `--tz` + `relative` | `epoch_output` `:141` / `format_timestamp` `:155` | PASS | Both already share `DateTime::from_timestamp` + `with_timezone(&Local)`. Add `relative: String` (always) + `tz: Option<String>` (`skip_serializing_if`) to `EpochOutput` (`:31`). `relative_for(epoch, now)` is new; `now` from `Utc::now()`. The human relative suffix lives ONLY in the integer-arg branch (`:88-92`); `now`/date-string branches stay bare. |
| `epoch --tz` validation | new arg + `chrono_tz::Tz::from_str` | PASS | Add `tz: Option<String>` to `EpochArgs` (`:43`). `Tz::from_str` returns `Result` → `bail!` on Err (mirror `parse_date` exit-1 hint at `:185`). Third line = `dt_utc.with_timezone(&tz).format("%Z %z")`. **See Pitfall 2 for the `%Z` confirmation.** |
| `uuid` `--v7` + format flags | pure `format_one`, `uuid/mod.rs:88` | PASS | Extend `format_one(u, opts)` to apply `--upper`/`--no-hyphens`/`--braces`/`--urn`. `--braces`/`--urn` `conflicts_with` each other (clap exit 2). `version` field → `"v4"`/`"v7"`. `Uuid::now_v7()` behind the new `v7` feature. The `{results:[{uuid,version}],count}` struct (`:27-39`) takes the formatted value (single-source). |
| `json --sort-keys` recursive | pre-fork point, `json/mod.rs:69-90` | PASS | `preserve_order` makes the map an `IndexMap`. A recursive `sort_value(Value)->Value` rebuilds each `Map` in sorted order, applied to `value` BEFORE the `is_json_on()` fork (`:69`) so it feeds emit_json/pretty/compact/colorize identically. Add `--sort-keys` to `JsonArgs` (`:38`). |
| `passgen` separator + entropy + no-similar | `phrase.join(".")` `:138`; `build_charset` `:176` | PASS | Replace `.join(".")` with `.join(&self.separator)` (default `"."`). `--no-similar` prunes `il1Lo0O` from `build_charset` + recomputes pool. entropy → `eprintln!` (stderr, TTY-gated) for human; `entropy_bits` added to `PassgenOutput` (`:47`). RNG (`OsRng`, `:116`) untouched. |
| `qr` EC + `--save` | `render_qr` `:106`; `EcLevel` imported `:43` | PASS | Add `ec: EcLevel` param to `render_qr` (default M). Add `--error-correction` (ValueEnum→EcLevel) + `--save: Option<PathBuf>` to `QrArgs` (`:54`). `QrOutput` (`:64`) gains `saved_path: Option<String>` and `error_correction` reflects the chosen level (currently hardcoded `"M"` at `:83`). |
| `qr --save` PNG/SVG render | qrcode `image`/`svg` features | PASS | `code.render::<image::Luma<u8>>().quiet_zone(true).build()` → `ImageBuffer` → `.save(path)` (PNG; default module size already 8×8). `code.render::<qrcode::render::svg::Color>().build()` → `String` → write file. **See Pitfall 3.** |
| `ascii` luma seam invert + truecolor + braille | `luma_to_char` `:104`; pipeline `:77-93` | PASS | `--invert` = `255 - luma` applied to the byte BEFORE `luma_to_char`/braille threshold. Color: keep `to_luma8` for the ramp index but ALSO `to_rgb8()` for the cell RGB → gated `.truecolor()` on the glyph. `--braille` replaces the `luma_to_char` line loop with the 2×4 engine (resize `2c×4r`). Module must add `use crate::core::output::is_color_on;` + `use owo_colors::OwoColorize;` (currently imports NEITHER — see Pitfall 4). |
| `matrix` color/speed/charset | consts `:85-94`; `katakana_glyphs()` `:306` | PASS | `--color` ValueEnum → `HEAD_RGB`/`FADE_*`. `--speed` level → `poll(...)` ms + `SPEED_MIN/MAX`. `--charset` preset-or-literal → replaces `katakana_glyphs()` result. `RawGuard`/`is_quit`/loop (`:117-322`) untouched. All color stays at the existing `.truecolor` sites (`:178`,`:188`), gated. |
| `lolcat --animate` loop | `rgb_at` `:128`; matrix template `:117-322` | PASS | Parameterize `rgb_at(phase, freq)` (drop hard-coded `f=0.1`). Copy `RawGuard`+`event::poll(50ms)`+single-flush+`is_quit` from matrix. lolcat does NOT currently import crossterm — add it. Static path (`:67-118`) is the degrade target. **See Pitfall 1 — headline risk + the read_input-vs-stdout-TTY subtlety.** |

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| QR → PNG raster | A manual module→pixel loop + PNG encoder wiring | `qrcode` `image` feature (`render::<Luma<u8>>().build()` → `ImageBuffer::save`) | The crate's renderer already does quiet-zone, module-sizing, and polarity correctly; the `image` dep is already linked. |
| QR → SVG | A hand-written `<svg>`/`<path>` string | `qrcode` `svg` feature (`render::<svg::Color>().build()`) | Zero-dependency, tested path-merge output. |
| IANA timezone + DST | Fixed-offset parsing / a zone table | `chrono-tz` `Tz::from_str` + `with_timezone` | DST rules are not hand-rollable correctly (D-01). |
| UUID v7 / formatting | Manual timestamp packing or string surgery | `Uuid::now_v7()` + `.hyphenated/.simple/.braced/.urn` | RFC-9562 layout + the four wrapping forms are const-fn in the crate. |
| JSON key sort | A custom serializer | Recursive `serde_json::Value` rebuild | `Value`'s `Map` is already an ordered `IndexMap`; just rebuild it sorted. |
| Terminal restore for `--animate` | A hand-rolled cleanup path | `matrix`'s `RawGuard` RAII verbatim | The Drop-guard-before-fallible-setup pattern is the proven D-10 backstop under `panic="abort"`. |

**Key insight:** this phase's hand-rolls (CSS table, redmean, relative-time ladder, braille bitmask, entropy) are deliberate per CONTEXT.md — each is ~10–30 lines of pure, testable logic where a crate would add a dep and STILL not fit (no reverse lookup, no future-tense, etc.). The hand-roll line is drawn exactly at the project's existing ethos: hand-roll trivial pure logic; never hand-roll DST, raster encoders, or CSPRNGs.

## Common Pitfalls

### Pitfall 1: `lolcat --animate` — the headline PS7 human-verify risk (LOL-V2-01)
**What goes wrong:** terminal left in raw mode / alternate screen after Ctrl+C/q/Esc; ~5 FPS stutter from per-char flush; the loop entered when piped → garbage / hang; OR the user can't even reach the animation.
**Why it happens:** `--animate` looks like "loop the recolor with a sleep," skipping the raw-mode state machine and the TWO independent TTY gates.
**How to avoid:**
- Copy `matrix`'s `RawGuard` (armed the instant after `enable_raw_mode()?`, BEFORE `EnterAlternateScreen`/`Hide`), `event::poll(50ms)` as frame-timer-AND-input-gate, single `flush()` per frame, `KeyEventKind::Press`-only `is_quit`. Keep the loop panic-free (no `.unwrap()` on terminal ops) so the Drop guard is the real restore under `panic="abort"`.
- Enter the loop ONLY when `std::io::stdout().is_terminal() && is_color_on() && !is_json_on() && !is_clip_on()`. The `is_terminal()` AND-gate is MANDATORY — `is_color_on()` can be forced true on a pipe via `CLICOLOR_FORCE`, and a raw-mode escape on a pipe is the SC3-forbidden hazard. Off-TTY → dispatch to the EXISTING static renderer (byte-identical-minus-ANSI, D-14).
- Persist-final-frame: after the deadline/quit, `LeaveAlternateScreen` then reprint ONE static frame to the normal buffer.
**Warning signs:** cursor invisible after exit; `q` needs two presses (Release double-fire — the Press-only filter prevents it); ANSI escapes in `box lolcat "x" --animate > file`.
**⚠️ Non-obvious gotcha for the human-verify script:** `read_input` (`core/input.rs`) gates on **STDIN** being a TTY and **exits 2 (MissingInput)** for a no-arg interactive invocation. So `box lolcat --animate` (typed, no text) exits 2 BEFORE the animate branch — the animate gate is on **STDOUT** `is_terminal()`, which is independent of stdin. The PS7 human-verify MUST pass text as an argument: **`box lolcat "Hello World" --animate`** (or pipe: `echo hi | box lolcat --animate`). Document this in the 09-03 checkpoint instructions or the human will see "exit 2" and report a false failure.

### Pitfall 2: `epoch --tz` — the `%Z` abbreviation format detail (the one thing to unit-test)
**What goes wrong:** `dt.format("%Z %z")` produces an empty or wrong abbreviation, so the third line / JSON `tz` reads `" +0900"` instead of `"JST +0900"`.
**Why it happens:** chrono's `%Z` requires the offset type to expose an abbreviation; chrono-tz's `TzOffset` provides it, but the exact rendering (`%Z`→`JST`) is the single behavior that depends on the chrono×chrono-tz pairing rather than on our code.
**How to avoid:** after wiring `dt_utc.with_timezone(&tz)`, add a unit test asserting a KNOWN zone+instant renders the expected `%Z %z` (e.g. `Asia/Tokyo` at `1700000000` → contains `"JST"` and `"+0900"`). chrono-tz `0.10.4` + chrono `0.4.45` support this; the test is a cheap lock against a future bump regressing it. `[ASSUMED — A1: confirm via the unit test]`
**Warning signs:** a `tz` line/field with a leading space and no zone code.

### Pitfall 3: `qr --save` — the only new filesystem I/O (clean errors, suppressed glyphs)
**What goes wrong:** a panic on a bad extension / unwritable path; the terminal glyph block printed alongside a `--save`; the `--json` document missing `saved_path`; PNG `.save()` failing silently because an encoder feature is off.
**Why it happens:** `qr` was pure (render→stdout) in v1; `--save` adds a write path with new failure modes.
**How to avoid:**
- Infer format from the extension: `.png` → raster, `.svg` → text, anything else → `bail!` clean exit 1 with a hint (never panic). Wrap `std::fs::write`/`ImageBuffer::save` with `.with_context(...)` (FOUND-05 discipline, like `read_file_or_stdin`).
- On `--save`: write the file + a stderr confirmation, and SUPPRESS the terminal glyph block (avoid noise) — gate the existing `println!("{rendered}")` (`qr/mod.rs:91`) behind `if save.is_none()`.
- PNG encode works: project `image` has `png` ✓ (`.save("x.png")` infers PNG, or use `save_with_format(p, ImageFormat::Png)`). SVG is a plain `String` write.
- `--save` + `--json`: the file is still written (the action) AND the metadata-only document (D-14) gains `saved_path`. Confirm this interaction in the plan.
**Warning signs:** a stack-trace on `box qr "x" --save C:\nope\x.png`; glyphs printed when `--save` was given.

### Pitfall 4: New colored paths leaking ANSI into piped/`--json` output (SC4)
**What goes wrong:** `matrix --color`, `ascii` truecolor, or animated `lolcat` emit ANSI without the `is_color_on()` gate → corrupts a redirect / breaks the byte-identical-minus-ANSI contract.
**Why it happens:** a new `.truecolor(...)` is added directly, forgetting the single-gate rule. **`ascii` is the sharp edge: it currently imports NEITHER `owo_colors` NOR `is_color_on`** — adding color means importing both for the first time, and the gate is easy to forget.
**How to avoid:** every new colored token is reached ONLY after `is_color_on()` returns true; the plain branch is byte-identical minus the escape. `ascii`/`matrix`/`lolcat` are display-only (SC4) so there is no `--json` document — but the piped (`> file`) path MUST still strip to plain. Add/extend the `_piped_no_ansi`-style test scanning stdout for `0x1B`.
**Warning signs:** ANSI in `box ascii img.png > out.txt`; a colored matrix frame visible in a redirect.

### Pitfall 5: `passgen` entropy leaking the secret or contaminating `--json`
**What goes wrong:** the entropy line printed to STDOUT mixes with the password (breaks the secret-only-on-stdout D-14 contract and `--json` purity).
**Why it happens:** entropy feels like "output" so it lands on stdout.
**How to avoid:** entropy → **STDERR** for the human path (per D9-default), TTY-gated like the clip confirmation; under `--json` it is a structured `entropy_bits` field, never stderr. The password stays the ONLY stdout content. `--clip` still copies only the secret.
**Warning signs:** `box passgen | Set-Clipboard` copying the entropy line; `passgen --json` stdout carrying a non-JSON entropy string.

## Code Examples

Verified patterns / signatures from the cached crate sources and the real project code.

### uuid v7 + the four wrapping forms (UUID-V2-01)
```rust
// Source: cached uuid-1.23.3/src/v7.rs:17 + src/fmt.rs:135-171 (format methods are const fn, NO feature)
let u = if v7 { uuid::Uuid::now_v7() } else { uuid::Uuid::new_v4() }; // now_v7 needs feature "v7"
// Wrapping forms (mutually exclusive: --braces conflicts_with --urn at the clap layer):
let s = match form {
    Form::Plain   => u.hyphenated().to_string(),        // 36 chars
    Form::Simple  => u.simple().to_string(),            // --no-hyphens (32 hex)
    Form::Braces  => u.braced().to_string(),            // {…}
    Form::Urn     => u.urn().to_string(),               // urn:uuid:…
};
let s = if upper { s.to_uppercase() } else { s };       // --upper composes with any form
```

### epoch --tz validation + zoned line (EPOC-V2-01)
```rust
// Source: chrono-tz Tz: FromStr + chrono::TimeZone (docs.rs/chrono-tz/0.10.4); mirrors epoch/mod.rs:185 hint discipline
use std::str::FromStr;
let tz = chrono_tz::Tz::from_str(name)
    .map_err(|_| anyhow::anyhow!("unknown timezone '{name}'; expected an IANA name like 'Asia/Tokyo'"))?;
let dt_utc = chrono::DateTime::from_timestamp(epoch, 0).ok_or_else(|| anyhow::anyhow!("timestamp {epoch} out of range"))?;
let zoned = dt_utc.with_timezone(&tz);
let tz_line = zoned.format("%Y-%m-%d %H:%M:%S %Z %z").to_string(); // ← unit-test the %Z output (Pitfall 2)
```

### qr --save PNG and SVG (QR-V2-01)
```rust
// Source: cached qrcode-0.14.1/src/render/{image,svg}.rs — features image + svg re-enabled
let code = qrcode::QrCode::with_error_correction_level(input.as_bytes(), ec)?; // ec: EcLevel
match ext {
    "png" => {
        // image::Luma<u8>: Pixel::Image = ImageBuffer; default module size is 8×8, quiet_zone default true.
        let img = code.render::<image::Luma<u8>>().quiet_zone(true).build();
        img.save(path).with_context(|| format!("writing {}", path.display()))?; // png feature present
    }
    "svg" => {
        let svg: String = code.render::<qrcode::render::svg::Color>().quiet_zone(true).build();
        std::fs::write(path, svg).with_context(|| format!("writing {}", path.display()))?;
    }
    other => anyhow::bail!("unsupported --save extension '.{other}'; use .png or .svg"),
}
```

### json --sort-keys recursive rewrite (JSON-V2-01)
```rust
// Source: serde_json preserve_order (Value::Object is an ordered Map) — applied before json/mod.rs:69 fork
fn sort_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().map(|(k, val)| (k, sort_value(val))).collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));            // keys sorted; arrays untouched
            Value::Object(entries.into_iter().collect())     // rebuild ordered map in sorted order
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sort_value).collect()),
        other => other,
    }
}
// let value = if self.sort_keys { sort_value(value) } else { value }; // BEFORE the is_json_on() fork
```

### ascii braille 2×4 bitmask (ASCI-V2-01)
```rust
// Source: Unicode Braille Patterns block (U+2800 base); D9-03 little-endian dot→bit mapping
// dots:  1 4      bits: 0 3
//        2 5            1 4
//        3 6            2 5
//        7 8            6 7
const DOT_BITS: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7]; // (dot1..dot8) → bit index; lock with a unit test
// For a 2×4 cell, set bit DOT_BITS[i] when sub-pixel i passes the 50% luma threshold (post --invert):
let glyph = char::from_u32(0x2800 + mask as u32).unwrap(); // mask: u8 of the 8 dots
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `qr` terminal-only (EcLevel::M fixed) | `--save` PNG/SVG + selectable EcLevel | Phase 9 (was VIS-V2-01, now in scope) | Re-enables qrcode features dropped in v1; first qr filesystem write. |
| `lolcat` one-shot recolor | `--animate` bounded alt-screen loop | Phase 9 | Reuses matrix's RAII; the static path becomes the degrade target. |
| `ascii`/`matrix` monochrome | truecolor (ascii) / preset color (matrix) | Phase 9 | New colored paths, all on the single `is_color_on()` gate. |

**Deprecated/outdated:** none relevant. (`uuid` v6/v8, sixel/kitty graphics, arbitrary-hex matrix color are explicitly VIS-V3, OUT OF SCOPE.)

## Runtime State Inventory

> Phase 9 is per-command code + per-command tests. It is NOT a rename/refactor/migration. No stored data, live-service config, OS-registered state, secrets, or build artifacts carry the renamed-string risk. The ONE new persistent artifact is the `qr --save` output FILE — a user-specified path, not box-managed state.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore, no keys, no IDs touched. | None. |
| Live service config | None. | None. |
| OS-registered state | None — `lolcat --animate`/`matrix` use the alternate screen (auto-restored by `RawGuard`); nothing registered. | None. |
| Secrets/env vars | None new. `passgen` reads `OsRng` (unchanged). Animate gates read `CLICOLOR_FORCE`/`NO_COLOR` (read-only). | None. |
| Build artifacts / installed packages | `Cargo.toml` gains `chrono-tz`; `uuid`/`qrcode` feature edits → a `Cargo.lock` update + recompile. The qrcode `image` feature unifies with the present `image 0.25.10` (one build, verified). | `cargo build` regenerates lock; no stale egg-info/binary equivalent. |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (MSVC) | build | ✓ | cargo 1.90.0 / rustc 1.90.0 | — |
| `chrono-tz` 0.10.x | epoch `--tz` | ✓ (crates.io, compatible) | 0.10.4 | none needed |
| `image` png encoder | qr `--save` PNG | ✓ (already in manifest) | 0.25.10 (`png`) | — |
| crossterm | lolcat `--animate` | ✓ (already in manifest) | 0.29 | — |
| A real PS7 TTY | LOL-V2-01 human-verify (SC3) | human-gated | — | none — the gate IS a human in PS7 |

**Missing dependencies with no fallback:** none. **Missing with fallback:** none. The LOL-V2-01 human-verify is not a tool dependency — it is the required human gate.

## Validation Architecture

> Nyquist validation is ENABLED (`config.json` → `workflow.nyquist_validation: true`). This section becomes VALIDATION.md.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + `assert_cmd 2.2` / `trycmd 1.2` / `insta 1.48` / `assert_fs 1.1` / `predicates 3.1` / `tempfile 3.27` (dev-deps, all present) |
| Config file | `Cargo.toml` (no separate test config) |
| Unit-test command | `cargo test --bin box` (binary-only crate — NOT `--lib`) |
| Integration command | `cargo test --test <name>` (e.g. `--test qr`, `--test epoch`) |
| Full suite + lint | `cargo test` then `cargo clippy --all-targets -- -D warnings` (both must be green — the phase-gate bar) |

Every Phase-9 command already has BOTH a unit-test seam (`#[cfg(test)] mod tests` in `src/commands/*/mod.rs`) AND an integration file in `tests/` (`uuid.rs`, `epoch.rs`, `color.rs`, `json.rs`, `passgen.rs`, `qr.rs`, `ascii.rs`, `matrix.rs`, `lolcat.rs`, plus `cli.rs::display_only_omit_json`). New tests EXTEND these — no Wave-0 framework bootstrap needed.

### Phase Requirements → Test Map
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
| LOL-V2-01 | **Automatable subset:** `rgb_at(phase,freq)` channel bounds + freq/seed effect; piped/`--json` does NOT enter raw mode (no `0x1B`, static, byte-identical); non-hanging smoke (piped input exits cleanly). **Human-gated:** smooth animation, clean Ctrl+C/q/Esc restore, visible `--freq`/`--seed` change — PS7 only. | unit + integration + **HUMAN-VERIFY** | `cargo test --bin box rgb_at` / `cargo test --test lolcat` + **manual PS7 checkpoint** | ✅ lolcat.rs |

### What is automatable vs. requires the human-verify gate
- **Fully automatable (8 of 9 reqs + most of LOL):** every pure seam (format/parse/sort/entropy/bitmask/EC), every byte-identity / `0x1B`-scan (SC4), exit-code assertions (conflict→2, bad-tz→1, bad-ext→1), the `qr --save` file write (assert non-empty + correct magic bytes via `assert_fs`), and lolcat's piped-degrades-to-static + non-hanging smoke.
- **Requires the PS7 human-verify gate (LOL-V2-01 / SC3 only):** smooth ~20-FPS animation, clean terminal restore on Ctrl+C/q/Esc with no stuck raw mode, and the *visible* effect of `--freq`/`--seed` on the gradient. Automated tests can prove the loop NEVER runs off-TTY and the math changes with seed; they cannot prove the on-screen animation is smooth and restores cleanly. Plan 09-03 carries this gate (UI hint: yes).

### Sampling Rate
- **Per task commit:** `cargo test --bin box <changed-seam>` (the fast pure-fn subset).
- **Per wave / plan merge:** the affected `cargo test --test <cmd>` integration files.
- **Phase gate:** full `cargo test` green + `cargo clippy --all-targets -- -D warnings` clean, THEN the LOL-V2-01 PS7 human-verify, before `/gsd:verify-work`.

### Wave 0 Gaps
None — existing test infrastructure (harness + dev-deps + per-command unit+integration files + `fixtures/`) covers all 9 requirements. New tests are additive to the files listed above. (One reminder for the executor: `assert_fs`/`tempfile` are the right tools for the `qr --save` file-write assertion; `cli.rs::display_only_omit_json` is the SC4 template for ascii/matrix/lolcat piped purity.)

## Security Domain

> `config.json` has no `security_enforcement` key (absent → treat as enabled). This is a local single-binary CLI with no auth, no session, no network-in, no access-control surface. The applicable ASVS slice is narrow: input validation and the one CSPRNG/secret-handling path.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no auth surface) |
| V3 Session Management | no | — |
| V4 Access Control | no | — (filesystem write is user-scoped to a user-given `--save` path) |
| V5 Input Validation | **yes** | `hsl(...)`/RGB/CSS-name parse → `bail!` exit 1 on malformed (no panic, no index overflow — extend `parse_color`'s T-02-06 discipline); `Tz::from_str` validates IANA names; QR capacity overflow `?`-propagates (T-05-QR-DoS); `--save` extension whitelist (`.png`/`.svg` only) + `.with_context` on the write. |
| V6 Cryptography | **yes (unchanged)** | passgen RNG stays `OsRng` + unbiased `choose` (T-V6) — `--no-similar`/`--separator` change only the charset/join, NEVER the draw. Entropy is theoretical (display only), not a security control. |
| V7 Error Handling | yes | All new failure paths exit 1/2 cleanly, never panic (FOUND-05); animate loop kept panic-free so `RawGuard` Drop is the restore backstop under `panic="abort"`. |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Terminal-escape injection re-emitted by `lolcat --animate` | Tampering | Keep the unconditional `strip_ansi_escapes::strip_str` on the input before recolor (T-04L-01) on BOTH the animated and static paths. |
| Raw-mode escape written to a pipe (`--animate` off-TTY) | DoS / corruption | The MANDATORY `std::io::stdout().is_terminal()` AND-gate (D9-04) — never enter raw mode off-TTY even when `CLICOLOR_FORCE` forces color. |
| QR capacity-overflow on oversized `--save` input | DoS | `with_error_correction_level` returns `Err` → clean exit 1 (existing T-05-QR-DoS, preserved). |
| `--save` to an unwritable / traversal path | Tampering | User-supplied path is honored as-is (single-user local CLI, accepted scope) but every write is `.with_context`-wrapped → clean exit 1, never a panic; extension whitelist limits format surface. |
| Secret disclosure via passgen entropy on stdout | Information disclosure | Entropy → STDERR (human) / `entropy_bits` JSON field; secret stays the sole stdout content (D-14). |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `chrono-tz 0.10.4` + `chrono 0.4.45` render `%Z` as the zone abbreviation (e.g. `Asia/Tokyo` → `JST`) | Pitfall 2 / Code Examples | LOW — if `%Z` is blank, the `tz` line/field shows only the numeric offset (`+0900`); cosmetic, caught by the recommended unit test. Mitigation: assert a known zone's `%Z` in a unit test (cheap, locks it). |

> All other claims are VERIFIED against cached crate source (uuid v7/fmt, qrcode features/render modules) or CITED to crates.io/docs.rs (chrono-tz deps + `Tz: FromStr`). The seam verifications are VERIFIED against the real project source at the cited line numbers.

## Open Questions

1. **`qr --save` + `--json` simultaneous behavior**
   - What we know: `--save` is the action (writes the file); `--json` is the output mode (metadata-only doc, D-14). CONTEXT.md says JSON gains `saved_path` on `--save`.
   - What's unclear: whether `box qr "x" --save out.png --json` writes the file AND emits the metadata doc (most consistent), or treats `--json` as suppressing the write.
   - Recommendation: write the file AND emit the doc with `saved_path` set — `--save` and `--json` are orthogonal (action vs mode). Lock with an integration test. Planner to confirm in 09-02.

2. **`lolcat --animate --duration` default value**
   - What we know: `--duration 0` = run-until-keypress; default is "a few seconds — lolcat parity" (`Instant`-based).
   - What's unclear: the exact default seconds.
   - Recommendation: pick a small default (e.g. 3–5s) as executor discretion; document it in `--help`. Not a blocker.

## Sources

### Primary (HIGH confidence)
- Project source read in full: `src/commands/{uuid,epoch,color,json,passgen,lolcat,matrix,qr,ascii}/mod.rs`, `src/core/{output,input}.rs`, `Cargo.toml` — every seam verified at the cited line.
- Cached crate source: `uuid-1.23.3/src/{v7.rs,fmt.rs,lib.rs}` (now_v7 gated on `v7`; format methods `const fn`, no feature); `qrcode-0.14.1/{Cargo.toml.orig,src/render/{mod,image,svg}.rs}` (features `image`/`svg`/`pic`; render APIs).
- `cargo 1.90.0` / `rustc 1.90.0`; `cargo search chrono-tz` → 0.10.4 latest.
- Milestone blueprints: `.planning/research/{STACK,PITFALLS,ARCHITECTURE}.md` (HIGH), `.planning/STATE.md` Accumulated Context (D-1..D-38), `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md` Phase 9, `.planning/phases/09-.../09-CONTEXT.md`.

### Secondary (MEDIUM confidence)
- `crates.io/api/v1/crates/chrono-tz/0.10.4/dependencies` — chrono `^0.4.25`, phf `^0.12`, optional serde/arbitrary/uncased.
- `docs.rs/chrono-tz/0.10.4/chrono_tz/enum.Tz.html` — `Tz: FromStr`; `Tz: chrono::TimeZone` (used via `with_timezone`); 597 variants.

### Tertiary (LOW confidence)
- The exact `%Z` → abbreviation rendering of the chrono×chrono-tz pairing (A1) — confirm via unit test at implementation.

## Metadata

**Confidence breakdown:**
- Seam verification (all 9 reqs buildable): HIGH — read every source at the cited line; no BLOCKER.
- Standard stack / dependency mechanics: HIGH — uuid + qrcode verified against cached source; chrono-tz compatibility verified against crates.io deps API.
- Pitfalls (lolcat animate, qr save, ascii gate): HIGH — grounded in PITFALLS.md + the real module state (e.g. ascii imports neither owo nor is_color_on; read_input stdin-TTY-vs-stdout-TTY subtlety).
- The single `%Z` format detail: MEDIUM — A1, confirm with a unit test.

**Research date:** 2026-06-28
**Valid until:** ~2026-07-28 (stable stack; chrono-tz/qrcode/uuid are mature). Re-verify only if a major chrono/chrono-tz bump lands.
