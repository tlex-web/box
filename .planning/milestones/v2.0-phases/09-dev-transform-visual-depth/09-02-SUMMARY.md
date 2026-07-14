---
phase: 09-dev-transform-visual-depth
plan: 02
subsystem: cli-commands
tags: [matrix, qr, ascii, qrcode, image, braille, truecolor, valueenum, ec-level, svg, png]

# Dependency graph
requires:
  - phase: 07-spine-rollout
    provides: "the frozen per-command Serialize output structs (qr metadata) + is_color_on()/is_json_on() spine the new fields/flags ride on"
  - phase: 09-01-dev-transform-depth
    provides: "owns Cargo.toml in Wave 1 so this plan's qrcode feature re-enable lands conflict-free"
provides:
  - "matrix --color (7 presets → head/trail RGB), --speed (slow/normal/fast → poll + fall range), --charset (preset name OR literal string) — all color is_color_on()-gated (SC4)"
  - "qr --save <file> PNG/SVG (extension whitelist, .with_context writes, bail! on unknown) + --error-correction L|M|Q|H (default M) feeding both terminal render and file; saved_path JSON field; --save/--json orthogonal"
  - "ascii truecolor default-on (is_color_on()-gated, mono-RAMP degrade) + hand-rolled 2×4 braille engine (unit-locked bit-order) + --invert at the single luma seam"
affects: [09-03-lolcat-animate, 11-meta-commands-completions]

# Tech tracking
tech-stack:
  added: ["qrcode [\"image\",\"svg\"] features re-enabled (unifies with present image 0.25.10; svg zero-dep)"]
  patterns:
    - "preset flags as pub clap ValueEnum (matches hash::Algo) with pure resolvers so preset→RGB / EC / charset unit-test WITHOUT a terminal"
    - "is_color_on() is the SOLE color gate on new colored paths (matrix presets, ascii truecolor) → piped/NO_COLOR byte-identical-minus-color (SC4)"
    - "--save (action) and --json (output mode) are orthogonal: file write happens regardless of the JSON fork; saved_path rides the frozen QrOutput additively"
    - "ValueEnum uppercase-canonical + lowercase alias (#[value(name=\"H\", alias=\"h\")]) for conventionally-uppercase EC levels"

key-files:
  created: []
  modified:
    - "Cargo.toml (qrcode [\"image\",\"svg\"] re-enabled) + Cargo.lock"
    - "src/commands/matrix/mod.rs"
    - "src/commands/qr/mod.rs"
    - "src/commands/ascii/mod.rs"
    - "tests/{matrix,qr,ascii}.rs"

key-decisions:
  - "matrix --color mapped via a color_axes(bool,bool,bool) channel mask → head_rgb (on=255/off=180) + trail_rgb (on=level/off=0); Green reproduces the v1 (180,255,180)/(0,g,0) exactly"
  - "matrix SC4 test scans for the truecolor SGR introducer (ESC[38;2;), NOT any 0x1B: matrix legitimately writes cursor-control ANSI (alt-screen/MoveTo) to a captured pipe — that is pre-existing display behavior, absent under real file redirect; only the NEW color is gated and asserted"
  - "qr Ec ValueEnum uses uppercase canonical values (L|M|Q|H, the plan's surface) with lowercase aliases so both --error-correction H and h parse"
  - "ascii braille bit-order is the Unicode identity DOT_BITS=[0..8] (bit i = dot i+1) paired with an explicit BRAILLE_DOTS position table; both unit-locked so the mapping can't drift"
  - "ascii --invert lives in one pure apply_invert(luma,bool) seam applied before ramp index AND braille threshold, orthogonal to color (which uses the true RGB)"

patterns-established:
  - "Pure terminal-free preset resolvers per behavior (color_axes/head_rgb/trail_rgb, speed_params, resolve_charset, Ec::to_level/label, apply_invert, braille_glyph) — every new behavior unit-tests without spawning the binary"
  - "New colored path = new .truecolor site reached ONLY inside an is_color_on() branch; plain branch byte-identical minus the escape (Pitfall 4)"

requirements-completed: [MTRX-V2-01, QR-V2-01, ASCI-V2-01]

# Metrics
duration: 20min
completed: 2026-07-14
---

# Phase 9 Plan 02: Visual Depth (matrix / qr / ascii) Summary

**Three additive visual-depth flags each on their frozen command struct — matrix `--color`/`--speed`/`--charset` named presets (all `is_color_on()`-gated), qr `--save` PNG/SVG (the one new filesystem write) + `--error-correction L|M|Q|H` feeding both terminal and file, and ascii truecolor-default + a hand-rolled 2×4 braille engine + `--invert` — plus the single manifest edit re-enabling qrcode `["image","svg"]`.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-07-14T11:31:26Z
- **Completed:** 2026-07-14T11:51:48Z
- **Tasks:** 3
- **Files modified:** 8 (3 source modules, Cargo.toml + Cargo.lock, 3 integration tests)

## Accomplishments
- **matrix (MTRX-V2-01):** `--color` (Green [default]/Red/Blue/Cyan/Magenta/Yellow/White) resolves via a pure `color_axes`→`head_rgb`/`trail_rgb` mask; `--speed` (Slow/Normal [default]/Fast) → `(poll_ms, speed_min, speed_max)` threaded through `Drop_::new_random`/`step`; `--charset` (katakana [default]/ascii/binary/digits OR any literal string) → the glyph table. Both `.truecolor` sites are now `is_color_on()`-gated (the color is the SOLE new gated path; the RawGuard/poll/quit RAII loop is untouched — SC4).
- **qr (QR-V2-01):** `render_qr` gains an `ec: EcLevel` param feeding BOTH the terminal render and the `--save` file; `--error-correction` is a `pub Ec` ValueEnum (uppercase `L|M|Q|H` canonical + lowercase alias, default M); `--save <file>` writes PNG (`image::Luma<u8>` render) or SVG (`svg::Color` render) by lowercased extension, `bail!`s exit 1 on any other/missing extension, `.with_context`-wraps every write, suppresses the terminal glyph block, and confirms on stderr. `QrOutput` reflects the chosen `error_correction` and adds `saved_path` (`skip_serializing_if`); `--save`+`--json` write the file AND emit the metadata doc (Open-Q1 orthogonal).
- **ascii (ASCI-V2-01):** the module now imports `owo_colors` + `is_color_on` for the first time (Pitfall 4); truecolor is default-on and gated SOLELY on `is_color_on()` (per-cell `.to_rgb8()` on the ramp glyph), degrading to the mono `RAMP` off-TTY/`NO_COLOR`; `--braille` swaps the ramp for a hand-rolled 2×4 engine (`char = U+2800 + mask`, unit-locked `DOT_BITS`/`BRAILLE_DOTS`, averaged truecolor per glyph); `--invert` = `255 - luma` at the single pure `apply_invert` seam before ramp/braille selection.
- **Manifest:** qrcode `default-features = false, features = ["image", "svg"]` — the only manifest edit; `image` unifies with the present `image 0.25.10`, `svg` is zero-dep. No hand-rolled rasterizer (RESEARCH-locked).

## Task Commits

Each task was committed atomically:

1. **Task 1: matrix --color / --speed / --charset presets** - `bcb1284` (feat)
2. **Task 2: qr --save (PNG/SVG) + --error-correction** - `12ee898` (feat)
3. **Task 3: ascii truecolor + --braille (2×4) + --invert** - `7faa152` (feat)

_Tasks 2 and 3 are `tdd="true"`: the pure seams (`save_qr`/`render_qr`, `braille_glyph`/`apply_invert`) and their unit tests were authored together and verified RED→GREEN via `cargo test --bin box <seam>`, then committed atomically once green (Rust inline `#[cfg(test)]` tests share the source file, so no separate non-compiling test commit is introduced — the 09-01 precedent)._

## Files Created/Modified
- `Cargo.toml` / `Cargo.lock` - qrcode `["image","svg"]` features re-enabled (drop `pic`)
- `src/commands/matrix/mod.rs` - `MatrixColor`/`Speed` ValueEnums, `--charset` String arg, `color_axes`/`head_rgb`/`trail_rgb`/`speed_params`/`resolve_charset` pure resolvers, gated `.truecolor` sites, speed-range threaded through the drop model
- `src/commands/qr/mod.rs` - `Ec` ValueEnum + `--save: Option<PathBuf>`, `render_qr(input, ec)`, `save_qr` (PNG/SVG whitelist), `QrOutput.saved_path`/chosen-level `error_correction`, glyph suppression + stderr confirmation under `--save`
- `src/commands/ascii/mod.rs` - `owo_colors`/`is_color_on` imports, `--braille`/`--invert` args, `render_ramp`/`render_braille` split, `apply_invert`/`braille_glyph` + `DOT_BITS`/`BRAILLE_DOTS`/`BRAILLE_THRESHOLD`
- `tests/matrix.rs` - SC4 `--color red` no-truecolor-escape assertion
- `tests/qr.rs` - `assert_fs` PNG/SVG file-write, bad-extension exit-1, `--save --json` orthogonality, EC-in-JSON
- `tests/ascii.rs` - braille glyph render, `--invert` (+ compose with braille), piped-no-ANSI (SC4), forced-color truecolor

## Decisions Made
- **matrix `--color` channel-mask model** — `color_axes(color) -> (bool,bool,bool)` selects the active R/G/B channels; `head_rgb` sets active=255/inactive=180 (the near-white tint) and `trail_rgb` sets active=fade-level/inactive=0. Green reproduces the v1 look byte-for-byte, and all seven presets fall out of one pure mask (unit-locked).
- **matrix SC4 assertion scope** — the plan's acceptance criteria said "a redirected run carries no `0x1B`", but matrix is a full-screen animation that legitimately writes cursor-control ANSI (alternate screen, `MoveTo`) whenever crossterm believes the output supports ANSI (as a captured `assert_cmd` pipe does). That cursor control is PRE-EXISTING (matrix wrote it before this plan) and orthogonal to color, and is absent under a real file redirect (crossterm's Windows backend no-ops on a file handle). SC4 for the NEW colored path is precisely that the *color* stays gated — the test therefore scans for the truecolor SGR introducer (`ESC[38;2;`) and asserts its absence under `--color red`, which is the meaningful, honest byte-level proof. This was reached after the initial "no 0x1B" test failed on the alt-screen escape.
- **qr `Ec` uppercase-canonical + lowercase alias** — clap ValueEnum lowercases variants by default (`l/m/q/h` only). EC levels are conventionally uppercase and the plan surface is `L|M|Q|H`, so each variant is `#[value(name = "X", alias = "x")]` — both `--error-correction H` and `h` parse.
- **ascii braille identity bit-order, explicitly locked** — Unicode braille defines bit `i` = dot `i+1`, so `DOT_BITS` is `[0..8]`, paired with an explicit `BRAILLE_DOTS` `(col,row)` position table for the 2×4 cell. Both are unit-locked (single-dot → `U+28xx`, all-on → `U+28FF`, positions tile the cell) so a future edit cannot silently transpose the layout.

## Deviations from Plan

None - plan executed exactly as written.

The one nuance worth recording is NOT a scope deviation but a plan-assumption correction: Task 1's action text says the matrix `.truecolor` sites are "EXISTING gated" calls and to "keep them gated on `is_color_on()`". In the actual source they were **un**gated (matrix ran the animation only in a TTY and colored unconditionally). Per the plan's own `must_haves` truth ("all color stays is_color_on()-gated") and the phase's SOLE-color-gate contract, the calls were made gated (importing `is_color_on`) — i.e. the plan's stated end-state was implemented, matching the truth rather than the incidental "keep" wording. The `matrix_pomodoro_have_no_spine_calls` guard bans only `emit_json`/`is_json_on` (not `is_color_on`), so the new import is compliant.

## Issues Encountered
- **`--error-correction H` initially rejected (exit 2):** clap's default ValueEnum lowercasing accepted only `h`. Resolved by adding uppercase-canonical value names with lowercase aliases (see Decisions). Caught by the `ec_level_reflected_in_json` integration test on first run; fixed within Task 2 before commit.
- **matrix SC4 "no 0x1B" test failed on the alt-screen escape:** the cargo-test process had a console attached, so crossterm wrote `ESC[?1049h`/`MoveTo` cursor control to the captured pipe. Diagnosed as pre-existing, color-orthogonal display behavior and re-scoped the SC4 assertion to the truecolor SGR sequence (see Decisions) — the glyphs themselves were already plain, proving the `--color` gate worked. Fixed within Task 1 before commit.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The three visual-depth requirements (MTRX/QR/ASCI-V2-01) are complete, tested, and clippy-clean. Plan gate satisfied: full `cargo test` green (206 unit + 216 integration = 422 passing, 0 failed) and `cargo clippy --all-targets -- -D warnings` clean.
- Plan 09-03 (lolcat `--animate`, PS7 human-verify) is independent of these modules and unblocked.
- IMPORTANT (worktree mode): STATE.md/ROADMAP.md were intentionally NOT modified — the orchestrator owns those writes after this worktree merges.

## Self-Check

- Created files: `.planning/phases/09-dev-transform-visual-depth/09-02-SUMMARY.md` (this file).
- Commits verified present below: `bcb1284`, `12ee898`, `7faa152`.

---
*Phase: 09-dev-transform-visual-depth*
*Completed: 2026-07-14*
