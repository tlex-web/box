---
phase: 04-terminal-visuals
plan: 03
subsystem: ui
tags: [ascii, image, png, jpeg, luma, ramp, terminal-width, clap, ascii-art]

# Dependency graph
requires:
  - phase: 04-terminal-visuals (04-01 json, 04-02 lolcat)
    provides: the clap command-registry wiring pattern (cli.rs enum variant → main.rs dispatch → commands/mod.rs) + the pure-helper-with-#[cfg(test)]-seam shape
  - phase: 02 (cowsay)
    provides: the render + path-arg precedent (cowsay's fixed-width is the reproducibility analog ascii intentionally DIVERGES from per D-02)
  - phase: 03 (hash, tree)
    provides: src/core/output::terminal_width() (80 fallback when piped, reused as the cols source) + the positional path-arg field shape + assert_cmd checked-in-fixture test pattern
  - phase: 01 (foundation)
    provides: the RunCommand trait, the 0/1/2 exit mapping in main.rs, anyhow .with_context boundary (FOUND-05)
provides:
  - "box ascii: a live command rendering a PNG or JPEG as terminal ASCII art fitted to the current terminal width (80 cols when piped); monochrome v1"
  - "pure luma_to_char(luma, ramp) dark→light mapper + compute_rows(cols, src_w, src_h) aspect-corrected (/2) row formula — unit-tested, crate-free"
  - "the image crate (0.25.10, png+jpeg trim) established as the single sanctioned image-decoding hand-roll exception (D-01); the decode→resize→luma→ramp pipeline"
  - "the ramp-emit seam where VIS-V2-01 colored ASCII would attach (clean monochrome v1 boundary, D-03)"
affects: [matrix (04-04), any future image-consuming command, VIS-V2-01 colored-ascii]

# Tech tracking
tech-stack:
  added: [image 0.25.10 (default-features=false, features=["png","jpeg"]; transitive png/zune-jpeg/moxcms/bytemuck/zune-core/byteorder-lite)]
  patterns:
    - "image pipeline: image::open(path) [extension-based, Pitfall 2] → resize_exact(cols, rows, Triangle) → to_luma8() → as_raw() row-major luma bytes → luma_to_char ramp loop → println! per row"
    - "cols = core::output::terminal_width() (80 piped) — a visual render fills the terminal, INTENTIONALLY diverging from cowsay's fixed width (D-02); rows = (cols*src_h/src_w/2).max(1) corrects the ~2:1 cell aspect"
    - "pure crate-free helpers (luma_to_char, compute_rows) behind an in-module #[cfg(test)] seam, run via cargo test --bin box (NOT --lib)"
    - "decode error mapped via .with_context(...)? → clean exit-1, never a panic (FOUND-05 / T-04A-02); zero-dimension image guarded with bail! before the rows divide"

key-files:
  created:
    - src/commands/ascii/mod.rs
    - tests/ascii.rs
    - tests/cmd/ascii.in/tiny.png
    - tests/cmd/ascii.in/tiny.jpg
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/cli.rs
    - src/main.rs
    - src/commands/mod.rs

key-decisions:
  - "[04-03] image added with default-features=false, features=[\"png\",\"jpeg\"] — the trimmed decoder set (RESEARCH A2 discretion) covers exactly the two formats ASCI-01 needs; verified to resolve the full open→resize_exact→to_luma8 path (fixture generator + cargo test + clippy build). artem REJECTED (D-01): its unconditional deps drag clap/colored/terminal_size/log/env_logger/once_cell/ureq and bypass terminal_width()"
  - "[04-03] ascii uses core::output::terminal_width() (80 piped) for cols, INTENTIONALLY diverging from cowsay's fixed 40-col reproducibility lock — a visual ASCII render should fill the terminal (D-02). rows = (cols*src_h/src_w/2).max(1): the /2 corrects the ~2:1 terminal-cell aspect; .max(1) prevents a zero-height render for an extreme aspect ratio"
  - "[04-03] monochrome v1 (D-03): the module imports neither owo_colors nor is_color_on; the dark→light ramp b\" .:-=+*#%@\" emit is the clean seam where VIS-V2-01 colored ASCII attaches"
  - "[04-03] ascii rendering is NOT snapshotable (output depends on terminal_width, 80 when piped) — the integration tests pin the CLI contract (PNG/JPEG render exit-0 non-empty valid-UTF-8, missing-file exit-1 no-panic) instead of an exact transcript; the luma_to_char monotonicity/bounds + compute_rows aspect/clamp invariants live in #[cfg(test)] units"
  - "[04-03] tiny 8x8 grayscale diagonal-gradient fixtures (140 B PNG / 340 B JPEG) generated once via a throwaway image-crate program; extension matches format (Pitfall 2: image::open detects by extension, not content), committed as binary"

patterns-established:
  - "Pattern: image decode pipeline — image::open → resize_exact(Triangle) → to_luma8 → as_raw → pure ramp mapper, with the decode error mapped to a clean exit-1 via .with_context"
  - "Pattern: terminal_width()-fitted visual render (cols source) vs cowsay's fixed-width reproducibility lock — the two are deliberately different and documented as such (D-02)"

requirements-completed: [ASCI-01]

# Metrics
duration: 4min
completed: 2026-06-24
---

# Phase 4 Plan 03: ASCII Art (`box ascii`) Summary

**`box ascii ./photo.png|.jpg` renders an image as terminal-width-fitted ASCII art via a hand-rolled image-crate pipeline (decode → resize_exact → to_luma8 → dark→light ramp); monochrome v1, missing/bad file exits 1 with no panic**

## Performance

- **Duration:** 4 min
- **Started:** 2026-06-24T12:45:10Z
- **Completed:** 2026-06-24T12:49:34Z
- **Tasks:** 2 (TDD RED → GREEN)
- **Files modified:** 9 (4 created, 5 modified)

## Accomplishments
- `box ascii` is live — the `not_implemented("ascii")` arm is gone; a PNG and a JPEG both render to width-fitted ASCII art (full dark→light ramp, verified visually at 80 cols when piped)
- Established the `image` crate (0.25.10, png+jpeg trim) as the single sanctioned image-decoding hand-roll exception (D-01); artem rejected
- Pure, unit-tested `luma_to_char` (monotonic, bounds-safe) + `compute_rows` (aspect-corrected `/2`, clamped `≥1`) helpers behind a `#[cfg(test)]` seam
- A missing/bad image errors cleanly (exit 1, no `panicked` in stderr) — the decode error is mapped via `.with_context(...)?`, never unwrapped (FOUND-05 / T-04A-02)
- Full suite stays green: 111 unit + all integration (no regression in the prior 106 unit tests); clippy `-D warnings` + fmt clean

## Task Commits

Each task was committed atomically (TDD RED → GREEN):

1. **Task 1: Wave-0 failing tests + tiny PNG/JPEG fixtures** - `977fec3` (test) — RED gate
2. **Task 2: Implement ascii slice — AsciiArgs + decode/resize/ramp pipeline + pure helpers** - `7c2e8eb` (feat) — GREEN gate

**Plan metadata:** (this commit) `docs(04-03): complete ascii plan`

_No REFACTOR commit needed — the GREEN implementation was clean (clippy + fmt passed without restructuring; the only fmt reflow was applied before the GREEN commit)._

## Files Created/Modified
- `src/commands/ascii/mod.rs` (created) - `AsciiArgs` + `RunCommand` impl: `image::open` → `resize_exact(Triangle)` → `to_luma8` → `luma_to_char` ramp loop; pure `luma_to_char` + `compute_rows` + 5 `#[cfg(test)]` unit tests; zero-dimension guard
- `tests/ascii.rs` (created) - ASCI-01 integration: PNG render, JPEG render (both exit-0 non-empty valid-UTF-8), missing-file → exit-1 no-panic
- `tests/cmd/ascii.in/tiny.png` (created) - 140-byte 8x8 grayscale gradient PNG fixture
- `tests/cmd/ascii.in/tiny.jpg` (created) - 340-byte 8x8 grayscale gradient JPEG fixture
- `Cargo.toml` (modified) - added `image = { version = "0.25.10", default-features = false, features = ["png", "jpeg"] }` with the D-01/D-02 citation
- `Cargo.lock` (modified) - locked image + its transitive decoder deps (binary-crate reproducible-build contract)
- `src/cli.rs` (modified) - swapped the `Ascii` unit variant for `Ascii(crate::commands::ascii::AsciiArgs)`
- `src/main.rs` (modified) - replaced the `not_implemented("ascii")` arm with `Commands::Ascii(args) => args.run()`
- `src/commands/mod.rs` (modified) - added `pub mod ascii;` (alpha first)

## Decisions Made
- **image feature trim (`["png","jpeg"]`, `default-features = false`):** covers exactly the two formats ASCI-01 needs (RESEARCH A2 discretion); verified to resolve the full decode/resize/luma path during fixture generation and the build. artem rejected per D-01.
- **`terminal_width()` for cols (80 piped), diverging from cowsay's fixed width:** a visual render should fill the terminal (D-02). `rows = (cols*src_h/src_w/2).max(1)` — the `/2` corrects the ~2:1 cell aspect; `.max(1)` prevents a zero-height render.
- **Ramp `b" .:-=+*#%@"`, FilterType::Triangle:** discretion (RESEARCH OQ-1); a 10-step dark→light ramp + linear filter give a smooth tonal range at low cost.
- **Tests pin the CLI contract, not a snapshot:** the render depends on `terminal_width()` (80 when piped), so an exact transcript would be fragile — integration tests assert exit-code + non-empty + valid-UTF-8 + no-panic; the `luma_to_char`/`compute_rows` invariants are unit-tested.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Reflowed two long `assert!` lines to satisfy `cargo fmt --check`**
- **Found during:** Task 2 (the fmt gate of the GREEN verify step)
- **Issue:** Two `assert!` calls in `src/commands/ascii/mod.rs` and `tests/ascii.rs` exceeded rustfmt's line width, so `cargo fmt --check` failed (a blocking gate in the plan's verify command).
- **Fix:** Ran `cargo fmt` to auto-reflow the two multi-line `assert!` calls (no logic change — behavior-preserving formatting only).
- **Files modified:** src/commands/ascii/mod.rs, tests/ascii.rs
- **Verification:** `cargo fmt --check` exits 0; full suite re-run still green.
- **Committed in:** `7c2e8eb` (Task 2 GREEN commit — applied before staging)

---

**Total deviations:** 1 auto-fixed (1 blocking — a formatting gate)
**Impact on plan:** The only deviation was a behavior-preserving rustfmt reflow required to pass the plan's own `cargo fmt --check` gate. No scope creep, no logic change.

## Issues Encountered
- **No PIL/ImageMagick on the host for fixture generation:** generated the two tiny fixtures with a throwaway `image`-crate program in the scratchpad (depends on `image` 0.25.10 with the same `["png","jpeg"]` trim), then committed the resulting binaries. This also served as an early proof that the trimmed feature set resolves the save/encode path before the manifest change landed.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- ASCI-01 complete; 3/4 Phase-4 plans done (json, lolcat, ascii). One Phase-4 stub remains: `matrix` (04-04, MTRX-01).
- The `image` crate is now in the manifest; matrix needs no new crate (crossterm/owo-colors already present — see the standing STATE.md note).
- The decode→resize→luma→ramp pipeline + the `terminal_width()`-fitted-render pattern are available for any future image-consuming command; the ramp-emit seam is the documented attach point for VIS-V2-01 colored ASCII.
- No blockers.

## Self-Check: PASSED

All created files present (`src/commands/ascii/mod.rs`, `tests/ascii.rs`, `tests/cmd/ascii.in/tiny.png`, `tests/cmd/ascii.in/tiny.jpg`, this SUMMARY); both task commits present (`977fec3` test/RED, `7c2e8eb` feat/GREEN).

---
*Phase: 04-terminal-visuals*
*Completed: 2026-06-24*
