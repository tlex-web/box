# Phase 9: Dev-Transform & Visual Depth - Context

**Gathered:** 2026-06-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Add the deferred **depth flags** to nine commands across two clusters — **dev-transform** (`uuid`, `epoch`, `color`, `json`, `passgen`) and **visual** (`lolcat`, `matrix`, `qr`, `ascii`). Each flag is local to one command and **additive** to that command's already-frozen `--json` output struct (Phase 7), so each new field slots into BOTH the human render and the JSON path for free (no-drift by construction). New colored paths ride on the frozen spine and stay gated on `is_color_on()`. Requirements: **UUID-V2-01, EPOC-V2-01, COLR-V2-01, JSON-V2-01, PASS-V2-01, LOL-V2-01, MTRX-V2-01, QR-V2-01, ASCI-V2-01** (9).

**In scope (per ROADMAP Phase 9 + REQUIREMENTS):**
- `uuid` — `--v7` time-ordered output + format flags `--upper` / `--no-hyphens` / `--braces` / `--urn` (UUID-V2-01).
- `epoch` — relative time ("3 hours ago") + `--tz <zone>` via chrono-tz (EPOC-V2-01).
- `color` — CSS named-color lookup both directions + HSL input (COLR-V2-01).
- `json` — `--sort-keys` (opt-in; `preserve_order` stays default) (JSON-V2-01).
- `passgen` — entropy-bits estimate + `--no-similar` (drops `il1Lo0O`) + `--separator` (PASS-V2-01).
- `lolcat` — `--animate` / `--freq` / `--seed`; **PS7 HUMAN-VERIFY gate** (LOL-V2-01).
- `matrix` — `--color` / `--speed` / `--charset` (MTRX-V2-01).
- `qr` — `--save <file>` (PNG / SVG) + `--error-correction L|M|Q|H` (QR-V2-01).
- `ascii` — truecolor output + `--braille` (2×4 dot density) + `--invert` (ASCI-V2-01).

**Plan sketch (ROADMAP):** 09-01 dev transforms (uuid/epoch/color/json/passgen) · 09-02 visuals (matrix/qr/ascii) · 09-03 **HUMAN-VERIFY (PS7)** lolcat `--animate`.

**Out of scope (later phases / deferred):**
- Fun & system depth (Phase 10), `config`/`completions` meta-commands (Phase 11).
- Any change to the frozen `--json`/`--clip`/config spine — Phase 9 only ADDS fields/flags onto it.
- `json --sort-keys` as a DEFAULT (breaks the v1 `preserve_order` contract — opt-in only, REQUIREMENTS Out-of-Scope).
- Advanced animation modes / sixel-kitty graphics / arbitrary-hex matrix color (VIS-V3, deferred below).
- No destructive flags this phase (no adversarial code-review gate); the one gate here is the LOL-V2-01 **PS7 human-verify**.
</domain>

<decisions>
## Implementation Decisions

> Four gray areas were researched with project-aware advisor tables (standard calibration tier) and decided. **All four resolved to the recommended option.** The five non-discussed commands' depth (uuid / json / passgen / matrix / qr) are pre-stated under Claude's Discretion with defaults so the planner has everything in one place. The frozen-spine locks carried from Phases 6–8 are recorded last.

### D9-01 — `color`: hybrid naming + CSS-functional HSL input (COLR-V2-01)
- **HSL input syntax = CSS functional `hsl(H, S%, L%)`** (also accept the modern space form `hsl(H S% L%)`). Route it by an **`hsl(` prefix check placed BEFORE** the comma/whitespace RGB branch in `parse_color` — the decisive no-collision choice, since a bare `210,100,50` already auto-detects as RGB. Add a hand-rolled **`hsl_to_rgb()`** inverse mirroring the existing `rgb_to_hsl()`. `H` 0–360, `S`/`L` 0–100 (percent). Name→color resolves via the table → rgb.
- **Named-color list = hand-rolled `const` table** of the ~148 **CSS Color Module Level 4** names (incl. `rebeccapurple`), as `&[(&str, (u8,u8,u8))]`. **No crate** (matches the humansize/qr2term/artem hand-roll ethos; a crate would add a dep, duplicate the existing hex/RGB parsers, and still lacks the reverse rgb→name lookup). Guard with a unit test against anchors (`black`, `white`, `rebeccapurple`, `cornflowerblue`).
- **hex→name policy = HYBRID (honest + useful).** `name` = the exact CSS keyword for the resolved RGB, or `null`; `nearest` = the closest keyword via a hand-rolled **weighted-RGB "redmean"** distance (~10 lines — NOT plain Euclidean RGB, which looks wrong; NOT full CIEDE2000/Lab, which is overkill — that's the deferred accuracy upgrade). The human block marks the approximate as `~name` (vs an exact `name`).
- **JSON (additive to the LOCKED `{hex, rgb:{r,g,b}, hsl:{h,s,l}}`, D-17):** add `name` (`string|null`) and `nearest` (`string`), **both always-present** (a stable schema so a PS7 script's `$c.name`/`$c.nearest` always resolves; reinforces no-drift since both deterministically drive the human block AND `emit_json`).

### D9-02 — `epoch`: always-on relative time + additive `--tz` (EPOC-V2-01)
- **Relative time = ALWAYS-ON, confined to the integer→date human path.** Append `(3 hours ago)` / `(in 2 days)` / `(just now)` to the two `Local:` / `UTC:` lines by default. The **`now` and date-string modes keep emitting bare integers** (scripting-clean — relative must NOT leak into the pipeable integer outputs).
- **Humanizer = hand-rolled ~30-line `relative_for(epoch, now)`** — a threshold ladder (just now / N min / N hr / N days / weeks / months / years) with a sign branch for future `in N …`. **No crate** (`timeago` lacks future + "just now" and drags `isolang`; `chrono-humanize` is stale and unfunable). ONE helper feeds both the human suffix and the JSON `relative` field (structural no-drift, same pattern as `format_timestamp`/`epoch_output`).
- **`--tz <zone>` = ADD a third labeled line** (`Local:` / `UTC:` / `<zone>:`) — Local/UTC stay the anchors (additive, NOT a replace). Validate the IANA name via `chrono_tz::Tz::from_str` → `bail!` clean **exit 1** with a hint (`unknown timezone '…'; expected an IANA name like 'Asia/Tokyo'`), never a panic (mirrors the existing `parse_date` discipline). The zoned datetime shares `DateTime::from_timestamp` with the other lines (no-drift).
- **JSON (additive to the LOCKED `{epoch, utc, local}`, D-20):** `relative: String` **always-present** (derived from `epoch`; like `local` it is clock-dependent → tests assert its FORMAT, not its value). `tz` field present **only under `--tz`** via `#[serde(skip_serializing_if = "Option::is_none")]`, value rendered `%Z %z` (self-describing, e.g. `"2023-11-15 07:13:20 JST +0900"`). Flat object (D-01 scalar rule — no nested `{zone,datetime}`).
- **Dep:** add `chrono-tz` to `Cargo.toml` (committed in D-1, not yet present) with its default tz-database feature.

### D9-03 — `ascii`: truecolor default-on + hand-rolled braille + invert (ASCI-V2-01)
- **truecolor = DEFAULT-ON, gated SOLELY on `is_color_on()`.** Truecolor in a TTY, **auto-degrade to the existing mono `RAMP`** when piped / redirected / `NO_COLOR` / the display-only `--json` path. **No `--color` flag** (matches the lolcat/color precedent and every analog — artem/chafa/viu all default-on + degrade; SC4 byte-identity falls out of the gate for free). Sample per-cell RGB via `.to_rgb8()` (keep luma for the ramp index); emit `.truecolor(r,g,b)` on the ramp glyph (mirrors lolcat's gated `.truecolor`).
- **`--braille` = hand-rolled 2×4 Unicode-braille engine** that **REPLACES the dark→light ramp** (an alternative glyph engine, not an overlay). `char = U+2800 + bitmask`, little-endian dot→bit mapping (dots 1-2-3 = bits 0-1-2 left column, 4-5-6 = bits 3-4-5 right column, 7/8 = bits 6/7 bottom row). Resize to `2*cols × 4*rows`; per-dot **fixed 50% luma threshold** kept as a swappable `const` (Otsu/per-cell-mean is the deferred upgrade). Color composes by **averaging the 8 sub-pixels into one RGB → a single gated `.truecolor()` per braille glyph** (braille and color stay orthogonal). Lock the bit-order with a unit test. **No crate** (drawille-style rejected per the D-01 `image`-only hand-roll exception).
- **`--invert` = `255 - luma`** applied at the **single luma seam** BEFORE the ramp index / braille per-dot threshold. Orthogonal to both color (display uses the true RGB; luma only selects the glyph/dot) and braille. NOT a `RAMP`-string reversal (which wouldn't generalize to braille/color).
- Stays **display-only** (SC4 — no `--json` document emitted); the new color paths are all gated on `is_color_on()`.

### D9-04 — `lolcat --animate`: bounded alt-screen, persist final frame (LOL-V2-01, HUMAN-VERIFY)
- **Loop model = BOUNDED alternate-screen + persist-final-frame.** Reuse `matrix`'s teardown **verbatim**: `RawGuard` RAII armed the instant `enable_raw_mode()` succeeds (BEFORE the fallible `EnterAlternateScreen`/`Hide` `execute!`), `event::poll(50ms)` that is BOTH the ~20-FPS frame timer AND the input gate, a single `stdout.flush()` per frame (never per-char — the STATE.md pitfall), and the `KeyEventKind::Press`-only `is_quit` filter (q / Esc / Ctrl+C-as-KeyEvent; Windows fires Press AND Release). Run until a **`--duration` deadline** (`Instant`-based, default a few seconds — lolcat parity) OR a quit key; then `LeaveAlternateScreen` and **reprint ONE final static frame to the normal buffer** so the colored text PERSISTS (recovers lolcat's "leave the final frame" while keeping matrix's clean wipe for the animation itself). **`--duration 0` = run until keypress** (folds the "infinite" option in without a second code path). This avoids the authentic in-place `MoveUp` reprint, whose line-wrap miscount is a known lolcat bug (busyloop/lolcat#116, kitty#2813) and would be fragile across PS7 window widths → risk the human-verify gate.
- **Animation mechanism:** advance ONE global **phase offset per frame**; parameterize **`rgb_at(phase, freq)`** (replaces the hard-coded `0.1`). `--seed` = the **initial phase offset** (reproducible / different start hue). The per-frame phase step is a tuned "flow speed" const. **`--freq`/`--seed` ALSO govern the static one-pass render** — ONE gradient path (the D-11 single-color-path discipline); existing `rgb_at` unit tests extend trivially.
- **Degradation (SC3/SC4):** enter the loop ONLY when **`std::io::stdout().is_terminal() && is_color_on()`** AND not under `--json`/`--clip`; otherwise dispatch to the **EXISTING static one-pass renderer** (which already strips color when `is_color_on()` is false → piped output byte-identical-minus-ANSI, D-14). The `is_terminal()` AND-gate is **mandatory** — `is_color_on()` alone can be forced true on a pipe (`CLICOLOR_FORCE`) → a raw-mode-on-a-pipe hazard SC3 forbids. Never emit a raw-mode escape off-TTY.
- **Human-verify (PS7):** smooth animated rainbow, clean exit on Ctrl+C/q/Esc with no stuck raw mode, `--freq`/`--seed` visibly change the gradient; degrades to a static render when piped/`--json`. RawGuard + the release `panic = "abort"` alt-screen backstop carry from matrix. Cleared by a human in PS7, not by automated test alone.

### Claude's Discretion (planner/executor latitude — sensible defaults pre-stated)
> Not discussed; recorded so the planner has the full phase in one place. Each is additive to the command's frozen `--json` struct and (where colored) gated on `is_color_on()`.

- **`uuid` v7 + format flags (UUID-V2-01):** add the `uuid` crate's **`v7` feature**; `--v7` → `Uuid::now_v7()` (else `new_v4`); the `version` JSON field becomes `"v4"`/`"v7"`. Format flags: `--upper` (exists), `--no-hyphens` (simple/`Hyphenated`-off form), `--braces` (`{…}`), `--urn` (`urn:uuid:…`). **Composition default:** `--braces` and `--urn` are the two wrapping forms — make them **`conflicts_with` each other** (fail loud, clap usage exit 2) rather than last-wins; `--upper`/`--no-hyphens` are modifiers that compose with any form. Extend the pure `format_one` to take the format options and feed BOTH paths. **JSON-field policy default:** the `uuid` field carries the SAME formatted value the human line prints (the format IS the value the user asked for; `format_one` stays single-source → no drift). Planner may emit canonical-in-JSON if a downstream-stability argument surfaces; default = formatted-everywhere.
- **`json --sort-keys` (JSON-V2-01):** opt-in `--sort-keys`; **recursively** sort object keys (nested objects too) of the parsed `Value`; arrays keep order. **`preserve_order` STAYS the default — never sorted implicitly** (Out-of-Scope lock). Mechanism: a recursive `Value`-rewrite that rebuilds each `Map` in sorted-key order BEFORE the existing pretty / `--compact` / `colorize` / `emit_json` paths, so the sort feeds all four outputs identically (no-drift). `--json --sort-keys` emits a sorted document too.
- **`passgen` entropy + `--no-similar` + `--separator` (PASS-V2-01):** entropy = **theoretical pool-based bits** — char mode `length * log2(pool_size)`, passphrase mode `words * log2(7776)` (~12.92 bits/word). **Display to STDERR** for the human path (keeps the secret-only-on-stdout D-14 contract + JSON purity) and add a top-level **`entropy_bits`** to the `{results,count}` JSON (per-config, not per-row). **`--no-similar`** drops the visually-ambiguous set `il1Lo0O` (and `0`/`O`, `1`/`l`/`I`) from the char charset and recomputes `pool_size` for entropy. **`--separator <str>`** overrides the hardcoded `.` passphrase join (default stays `.`; the existing paste-safe rationale holds). RNG stays OsRng + unbiased `choose` — untouched (T-V6).
- **`matrix --color`/`--speed`/`--charset` (MTRX-V2-01):** **`--color`** = a small **named-preset `ValueEnum`** (green [default] / red / blue / cyan / magenta / yellow / white) mapping to the head/fade RGB — NOT arbitrary hex (cleaner for a screensaver, no hex-parse surface; matches the `Algo`/`SortMode`/`Case` ValueEnum style). **`--speed`** = discrete **levels** (slow / normal [default] / fast) mapping to the poll interval / `SPEED_MIN..MAX` (friendlier than a raw FPS number). **`--charset`** = named presets (katakana [default] / ascii / binary / digits) OR a literal custom string accepted by the same flag (a known preset name resolves to its table, else the string's chars become the glyph set). All color gated on `is_color_on()`; the `RawGuard`/loop/quit logic untouched. Stays display-only (SC4).
- **`qr --save` + EC (QR-V2-01):** **`--save <file>`** infers format from the extension — `.png` (raster) and `.svg` (text). Re-enable the qrcode features dropped via `default-features = false` (`image` and/or `svg`), OR rasterize the QR matrix through the already-present `image` crate — planner picks the lighter binary path. **`--error-correction L|M|Q|H`** → `EcLevel::{L,M,Q,H}` (default stays **M**), applied to BOTH the terminal render and `--save`. PNG module scale / quiet-zone = sensible default (~8 px/module, 4-module quiet zone). On `--save`, write the file + a stderr confirmation and suppress the terminal glyph block (avoid noise); the JSON metadata's `error_correction` reflects the chosen level + add a `saved_path` when `--save`.

### Carried forward — LOCKED upstream, NOT re-discussed (recorded for the planner)
- **Frozen spine** (Phases 6/7 — see `<canonical_refs>`): `is_json_on()` fork happens FIRST; every new field feeds ONE `#[derive(Serialize)]` struct → both the human render and `emit_json` (no-drift); progress (indicatif) → stderr, suppressed under `--json`; piped/`--json` output byte-identical-minus-ANSI; locked JSON shapes are **additive-only** (`color {hex,rgb,hsl}`, `epoch {epoch,utc,local}`, `uuid {results:[{uuid,version}],count}`, `qr` metadata).
- **`is_color_on()` is the SOLE color gate (SC4):** `matrix --color`, `ascii` truecolor, and animated `lolcat` all stay gated → piped/`--json` output remains byte-identical minus ANSI. No second color stack, no owo-colors global override.
- **Display-only commands (matrix, lolcat, ascii, pomodoro, clip) parse but IGNORE `--json`/`--clip`** (D-21 — they never call `is_json_on`/`emit_json`).
- **Field policies (D-3/D-4):** bare `u64` for large JSON numbers; `to_string_lossy()` for non-UTF-8 paths, never `to_str().unwrap()`.
- **Terminal-loop pitfalls (D-10 / matrix):** `RawGuard` armed BEFORE the fallible alt-screen setup; single-flush-per-frame; `KeyEventKind::Press`-only quit; release `panic = "abort"` → the alternate screen is the teardown backstop; no `.unwrap()` on terminal ops.
- **`preserve_order` stays the `json` default;** `--sort-keys` is opt-in and never implicit.
- **No `cli.rs`/`main.rs` spine plumbing changes** — global `--json`/`--clip` + `init_output`/`flush_clip` already wired; Phase 9 is per-command args + per-command logic + per-command tests.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & locked contracts
- `.planning/ROADMAP.md` § "Phase 9: Dev-Transform & Visual Depth" — goal, 4 success criteria, and the 3-plan sketch (09-01 dev / 09-02 visuals / 09-03 lolcat HUMAN-VERIFY).
- `.planning/REQUIREMENTS.md` § "Dev-transform depth" + "Visual depth" — the 9 Phase-9 requirements (UUID/EPOC/COLR/JSON/PASS-V2-01 + LOL/MTRX/QR/ASCI-V2-01) + Milestone Decision **D-1** (committed deps: `chrono-tz`, `uuid` v7 feature) + the **Out-of-Scope** table (no `json --sort-keys` default, no NDJSON, no per-command bespoke JSON schemas) + **Future Requirements** (VIS-V3 advanced animation / sixel).
- `.planning/STATE.md` § "Accumulated Context" — locked v2 decisions **D-1..D-38**, the critical spine pitfalls, and the terminal-loop RAII pitfall.
- `.planning/PROJECT.md` — milestone scope, "deepening not rewrite" framing, v1 Key Decisions.

### The frozen spine (READ FIRST — every new field rides on these)
- `.planning/phases/06-scriptable-core-foundation/06-CONTEXT.md` — the authoritative spine contract: D-01 JSON root rule, D-03 field-naming, D-07/D-08 `--clip`, D-09/D-10 error/exit contract, D-3/D-4 field policies.
- `.planning/phases/07-spine-rollout/07-CONTEXT.md` — the per-command `{results,count}` projections + the **D-17 frozen schemas** this phase extends: `color {hex,rgb:{r,g,b},hsl:{h,s,l}}` (D-19), `epoch {epoch,utc,local}` (D-20), `uuid {results:[{uuid,version}],count}`, `qr` metadata (D-14), `json` D-16 identity passthrough; SC4 display-only omission (D-21).
- `.planning/phases/08-filesystem-depth/08-CONTEXT.md` — the advisor-table + Claude's-Discretion pattern this phase mirrors (no destructive flags here).

### Source files this phase touches (per command, with the seam)
- `src/commands/uuid/mod.rs` — extend the pure `format_one(u, upper)` to a format-options renderer; `v7` via `Uuid::now_v7()`; `version` field "v4"/"v7". Frozen `{results:[{uuid,version}],count}`.
- `src/commands/epoch/mod.rs` — extend the shared-math helpers `epoch_output()` (JSON) + `format_timestamp()` (human); add `relative_for(epoch, now)` (hand-roll) + `--tz` via `chrono_tz::Tz::from_str`; additive `relative`/`tz` JSON fields.
- `src/commands/color/mod.rs` — `parse_color` gains an `hsl(` branch BEFORE the RGB branch + `parse_hsl`/`hsl_to_rgb` (mirror `rgb_to_hsl` at L199); a `const` CSS-name table + exact/redmean-nearest lookups; additive `name`/`nearest` JSON.
- `src/commands/json/mod.rs` — a recursive `Value` key-sort applied before the pretty/compact/colorize/emit_json fork; `preserve_order` default untouched.
- `src/commands/passgen/mod.rs` — entropy estimate (stderr + `entropy_bits` JSON) + `--no-similar` charset prune + `--separator` (replaces the hardcoded `.` at the `phrase.join(".")` site); RNG untouched.
- `src/commands/lolcat/mod.rs` — `rgb_at(phase, freq)` parameterized; `--animate`/`--freq`/`--seed`; TTY+color gate before the loop; existing static render is the degrade path.
- `src/commands/matrix/mod.rs` — `--color` (preset ValueEnum → HEAD/FADE RGB), `--speed` (level → poll/SPEED consts), `--charset` (preset table or custom string → `katakana_glyphs` replacement); `RawGuard`/`is_quit`/loop untouched.
- `src/commands/qr/mod.rs` — `--save` (extension → PNG/SVG) + `--error-correction` (→ `EcLevel`, default M) in/around the pure `render_qr` seam; re-enable qrcode `image`/`svg` features or use the present `image` crate; additive `saved_path` JSON.
- `src/commands/ascii/mod.rs` — truecolor via `.to_rgb8()` + `is_color_on()`-gated `.truecolor()` on the ramp glyph; `--braille` 2×4 engine (resize `2c×4r`, `U+2800 + bitmask`) replacing `luma_to_char`; `--invert` `255 - luma` at the luma seam.

### THE copy-me patterns (reuse VERBATIM — do not re-derive)
- `src/commands/matrix/mod.rs` — `RawGuard` RAII, `EnterAlternateScreen`/`Hide`, `event::poll(50ms)` as frame-timer-AND-input-gate, single per-frame `flush()`, `KeyEventKind::Press`-only `is_quit` (q/Esc/Ctrl+C). **The template for `lolcat --animate`.**
- `src/commands/lolcat/mod.rs` — the pure `rgb_at(phase)` sine gradient + `is_color_on()`-gated `.truecolor()` per Unicode scalar + unconditional input ANSI strip. **The template for `ascii` truecolor.**

### Shared infra (reuse VERBATIM — do not re-implement)
- `src/core/output.rs` — `emit_json`, `out_line`, `is_json_on`, `is_color_on`, `clip_feed`, `terminal_width`, `human_size`. (Add nothing to the spine; consume it.)
- `src/core/input.rs` — `read_input` (arg → piped stdin → no-arg interactive TTY → exit 2; used by color/json/lolcat/qr).
- `Cargo.toml` — current deps; **add `chrono-tz`**, **add the `uuid` `v7` feature**, **re-enable qrcode `image`/`svg`** (or use the present `image` crate). `owo-colors`/`crossterm`/`image`/`serde_json (preserve_order)`/`unicode-width` already present.

### Research blueprints (HIGH-confidence)
- `.planning/research/ARCHITECTURE.md` — the New-vs-Modified file ledger + per-command change lists.
- `.planning/research/PITFALLS.md` — `--json` contamination, color/progress leakage, terminal raw-mode-on-a-pipe, single-flush-per-frame.
- `.planning/research/STACK.md` — dependency versions/rationale (`chrono-tz`, `uuid v7`, qrcode features).
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Every Phase-9 command already has its `--json` output struct** (Phase 7) — new depth fields are ADDITIONS to existing `#[derive(Serialize)]` structs feeding both renders, so no-drift is free.
- **`matrix`'s `RawGuard` + `is_quit` + `event::poll(50ms)`-as-timer + single-flush-per-frame** — the proven terminal-animation pattern; `lolcat --animate` copies it verbatim (the decisive reason the bounded-alt-screen model beats the line-wrap-fragile in-place reprint).
- **`lolcat`'s pure `rgb_at(phase)` + `is_color_on()`-gated `.truecolor()`** — the per-scalar coloring template `ascii` truecolor mirrors; parameterizing it to `rgb_at(phase, freq)` serves BOTH the animated and static lolcat renders (one color path).
- **`color`'s `rgb_to_hsl()` (L199)** — the new `hsl_to_rgb()` inverse mirrors it; `parse_color`'s auto-detect ordering (L125–147) is where the `hsl(` branch slots in BEFORE the RGB branch.
- **`epoch`'s shared `DateTime::from_timestamp`/`with_timezone(&Local)` math** in `epoch_output()`/`format_timestamp()` — `--tz` and `relative` extend the SAME helpers so JSON can't drift from the human lines.
- **`ascii`'s `image` pipeline** (`resize_exact` + `to_luma8`) — keep `to_rgb8()` for color, resize to `2c×4r` for braille; `luma_to_char` is the single luma seam `--invert` flips.
- **`qr`'s pure `render_qr` seam + `qrcode` `EcLevel`** — `--error-correction` swaps the level; `--save` branches off the matrix to PNG/SVG.
- **`uuid`'s pure `format_one` / `passgen`'s pure `build_charset` + `phrase.join(".")`** — extend in place; both already feed the no-drift `{results,count}` struct.

### Established Patterns
- **`is_json_on()` fork FIRST**, then human writes below it; new flags keep all human chrome (and progress) below the fork.
- **`is_color_on()` is the SOLE color gate** — new colored paths (matrix preset, ascii truecolor, animated lolcat) attach in that one gated slot → byte-identical-minus-ANSI when piped/`--json`.
- **Single `#[derive(Serialize)]` struct feeds both human + `emit_json`** (no-drift) — additive fields, never a parallel render.
- **Terminal-loop RAII template** (D-10): guard armed before fallible setup; single-flush-per-frame; Press-only quit; panic=abort alt-screen backstop.
- **Preset flags as `pub` clap `ValueEnum`** (matches `hash::Algo`/`tree::SortMode`/`bulk_rename::Case`) — `matrix --color`/`--speed` and the uuid wrapping forms follow this style; satisfies `private_interfaces`.

### Integration Points
- **New deps:** `chrono-tz` (epoch `--tz`), the `uuid` `v7` feature, and re-enabled `qrcode` `image`/`svg` features (or rasterize via the present `image` crate). **No new crate** for color names (hand-roll table), relative time (hand-roll), braille (hand-roll bitmask), json sort (existing serde_json), or lolcat animation (crossterm already present).
- **No `cli.rs`/`main.rs` changes** — global spine already wired; Phase 9 is per-command args + logic + tests.
- **`box` is binary-only** — unit tests run via `cargo test --bin box`, integration tests in `tests/`. Build target `x86_64-pc-windows-msvc` + `crt-static`.
</code_context>

<specifics>
## Specific Ideas

- **The headline is the `lolcat --animate` PS7 human-verify (09-03).** The bounded-alt-screen-**persist-final-frame** model is the decisive UX choice: it reuses matrix's proven RAII teardown verbatim (sidestepping the `MoveUp` line-wrap corruption class that makes the authentic in-place reprint fragile to verify across PS7 window widths) AND persists the rainbow (the whole point of a colorizer). `--duration 0` = run-until-keypress folds the "infinite" variant in with no second code path.
- **`color` naming is hybrid + honest.** Exact `name` when the resolved RGB lands on a CSS keyword, else a clearly-marked `~nearest` via hand-rolled **redmean** distance (pastel-grade utility without lying about arbitrary colors). The `hsl(...)` prefix-before-the-RGB-branch is the decisive no-collision parse choice — a bare HSL triple would otherwise silently mis-parse as RGB.
- **`epoch` relative is always-on but CONFINED** to the integer→date human path so the `now`/date-string (bare-integer) outputs stay pipe-clean; `--tz` is purely additive (3rd line + `skip_serializing_if` JSON field) so the locked `{epoch,utc,local}` shape is preserved.
- **`ascii` truecolor-default mirrors lolcat/color + every analog** (artem/chafa/viu default-on + auto-degrade); `--braille` is a full ramp-REPLACEMENT glyph engine with color orthogonal via one averaged RGB per 2×4 cell; `--invert` is one byte-flip at the luma seam.
- **All four discussed areas resolved to the research-recommended option** (standard calibration tier). The non-discussed depth (uuid / json / passgen / matrix / qr) is pre-stated under Claude's Discretion with defaults so the planner has the whole phase in one place.
- **Pre-planning doc check:** no doc-amendment action item surfaced — ROADMAP Phase-9 SC + REQUIREMENTS wording are consistent with these decisions (the deps are already committed in D-1; the `json --sort-keys` opt-in and the `is_color_on()`-gated color paths are already the locked contracts).
</specifics>

<deferred>
## Deferred Ideas

- **`color` perceptual nearest-color (CIEDE2000/Lab)** — redmean is the chosen v2 hand-roll; full Lab/CIEDE2000 is the accuracy upgrade if naming quality is ever questioned. Would justify the `palette` crate only if color-science grows.
- **`ascii` adaptive braille threshold (Otsu / per-cell mean)** — ship the fixed 50% per-dot threshold first (kept as a swappable `const`); revisit only if photo quality is a complaint. Sixel / kitty-graphics output is **VIS-V3** (deferred beyond v2).
- **`lolcat` / `matrix` advanced animation modes** (e.g. lolcat vertical-scroll variant, matrix arbitrary-hex `--color`) — **VIS-V3**. Matrix ships named-preset colors first; arbitrary hex is the later upgrade.
- **`qr` additional output formats** beyond PNG/SVG (e.g. PDF/EPS) — out of scope; PNG + SVG cover the `--save` requirement.
- **`uuid --urn`/`--braces` combinatorics beyond the two wrapping forms** — the four flags cover UUID-V2-01; further RFC-9562 niceties (e.g. v6/v8) are out of scope.

None of these are in Phase 9 scope — captured so they are not lost.
</deferred>

---

*Phase: 9-Dev-Transform & Visual Depth*
*Context gathered: 2026-06-28*
