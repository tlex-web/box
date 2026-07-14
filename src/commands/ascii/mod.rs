//! The `ascii` command: render an image as terminal ASCII art (ASCI-01). The
//! third Phase-4 slice and the one that establishes the `image` crate — the
//! single sanctioned image-decoding hand-roll exception (D-01). artem is REJECTED
//! (it drags clap/colored/terminal_size/log/env_logger/once_cell/ureq and bypasses
//! our `terminal_width()` gate); the rendering engine is hand-rolled on `image`.
//!
//! Pipeline (D-02): `image::open(path)` → `resize_exact(cols, rows, Triangle)` →
//! `to_luma8()` → map each luma byte through a dark→light ASCII ramp. The column
//! count is [`crate::core::output::terminal_width`] (which falls back to 80 when
//! stdout is piped), so the render fits the live terminal — this INTENTIONALLY
//! diverges from cowsay's fixed width (cowsay locks 40 cols for reproducible
//! pipe-vs-TTY output; ascii is a visual render that should fill the terminal,
//! D-02). The row count corrects the ~2:1 terminal cell aspect via `/2` so the
//! image is not vertically stretched.
//!
//! Color + density depth (ASCI-V2-01):
//! - **truecolor DEFAULT-ON, gated SOLELY on [`is_color_on`]** (the lolcat
//!   template). Each cell keeps its luma for the ramp index but ALSO samples
//!   `.to_rgb8()`, emitting `.truecolor(r,g,b)` on the ramp glyph ONLY inside the
//!   `is_color_on()` branch; piped / `NO_COLOR` / `--json` degrades to the mono
//!   `RAMP` with no ANSI (SC4 byte-identical-minus-color). There is NO `--color`
//!   flag. The module now imports both `owo_colors` and `is_color_on` for the
//!   first time (Pitfall 4).
//! - **`--braille`** replaces the ramp with a hand-rolled 2×4 Unicode-braille
//!   engine ([`braille_glyph`], `char = U+2800 + bitmask`): the image is resized to
//!   `2*cols × 4*rows` and each output cell's 8 sub-pixels set their braille bit via
//!   the unit-locked [`DOT_BITS`] mapping past a fixed [`BRAILLE_THRESHOLD`]. Color
//!   in braille mode averages the 8 sub-pixels into ONE gated `.truecolor()` per
//!   glyph (braille and color stay orthogonal). No crate (the `image`-only
//!   hand-roll exception).
//! - **`--invert`** applies `255 - luma` at the single luma seam ([`apply_invert`])
//!   BEFORE the ramp index / braille per-dot threshold — orthogonal to both color
//!   (which uses the true RGB) and braille.
//!
//! `image::open` detects the format from the FILE EXTENSION, not the content
//! (Pitfall 2), so the path's extension must match the actual format (PNG/JPEG).
//! A missing or undecodable file is mapped to a clean exit-1 anyhow error via
//! `.with_context(...)?` — never a panic (FOUND-05 / T-04A-02).

// # Spine omission (SC4)
// `ascii` is a DISPLAY-ONLY command: it INTENTIONALLY does not honor the global
// `--json`/`--clip` flags (roadmap SC4). The flags parse (global on `Cli`) but
// `run()` never calls `is_json_on()` / `emit_json` — `box ascii <img> --json`
// renders the ASCII art and emits NO JSON document to stdout (the render is a
// visual). Asserted by `tests/cli.rs::display_only_omit_json`.

use std::fmt::Write as _;

use anyhow::Context;
use clap::Args;
use image::GenericImageView;
use owo_colors::OwoColorize;

use crate::commands::RunCommand;
use crate::core::output::is_color_on;

/// The dark→light ASCII ramp (D-02). `b" .:-=+*#%@"` runs from a space (darkest,
/// luma 0) to `@` (lightest, luma 255). The exact glyph set is discretion
/// (RESEARCH OQ-1); a 10-step ramp gives a smooth tonal range. Indexed by
/// [`luma_to_char`].
const RAMP: &[u8] = b" .:-=+*#%@";

/// The per-dot luma threshold for the `--braille` engine (ASCI-V2-01): a sub-pixel
/// whose (post-`--invert`) luma is `>=` this value sets its braille dot. A fixed
/// 50% (`128`) cut kept as a swappable `const` — an adaptive Otsu / per-cell-mean
/// threshold is the deferred upgrade.
const BRAILLE_THRESHOLD: u8 = 128;

/// The Unicode-braille bit for each dot, indexed by dot number − 1 (dot1..dot8)
/// (ASCI-V2-01). Unicode braille defines bit `i` = dot `i+1`, so this is the
/// identity `[0,1,2,3,4,5,6,7]`; it is kept explicit and unit-locked so the
/// mapping cannot silently drift. Paired with [`BRAILLE_DOTS`], which gives each
/// dot's `(col, row)` position inside the 2×4 cell.
const DOT_BITS: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

/// The `(sub-col, sub-row)` position of each braille dot inside a 2×4 cell, indexed
/// by dot number − 1 (dot1..dot8). The Unicode braille cell layout is:
/// ```text
/// dot1 dot4      (0,0) (1,0)
/// dot2 dot5  =>  (0,1) (1,1)
/// dot3 dot6      (0,2) (1,2)
/// dot7 dot8      (0,3) (1,3)
/// ```
const BRAILLE_DOTS: [(u32, u32); 8] = [
    (0, 0), // dot1
    (0, 1), // dot2
    (0, 2), // dot3
    (1, 0), // dot4
    (1, 1), // dot5
    (1, 2), // dot6
    (0, 3), // dot7
    (1, 3), // dot8
];

/// `box ascii <PATH>` — render a PNG or JPEG image as terminal art (ASCI-01 /
/// ASCI-V2-01).
///
/// The image is decoded, resized to the current terminal width (80 columns when
/// piped), and each pixel's brightness is mapped to a character on a dark→light
/// ramp (or a 2×4 braille cell with `--braille`). Truecolor is emitted in a
/// terminal and auto-degrades to monochrome when piped / `NO_COLOR` / `--json`.
#[derive(Debug, Args)]
pub struct AsciiArgs {
    /// Path to a PNG or JPEG image. The extension must match the actual format
    /// (the decoder selects by extension, not by inspecting the file contents).
    pub path: String,

    /// Render with 2×4 Unicode braille dots instead of the dark→light ASCII ramp
    /// (higher spatial density). Color, when enabled, is one averaged truecolor
    /// per braille glyph.
    #[arg(long)]
    pub braille: bool,

    /// Invert brightness (`255 - luma`) before glyph selection — a light subject on
    /// a dark background renders as dark glyphs and vice versa. Composes with
    /// `--braille` and with color.
    #[arg(long)]
    pub invert: bool,
}

impl RunCommand for AsciiArgs {
    fn run(self) -> anyhow::Result<()> {
        // Decode by extension (Pitfall 2). A missing/undecodable file → clean
        // exit-1 anyhow error, never a panic (FOUND-05 / T-04A-02).
        let img = image::open(&self.path)
            .with_context(|| format!("failed to read image {}", self.path))?;

        let (src_w, src_h) = img.dimensions();
        // Defensive: a 0-dimension image would divide-by-zero in compute_rows /
        // produce an empty render. Surface as a clean error rather than panic.
        if src_w == 0 || src_h == 0 {
            anyhow::bail!("image {} has zero width or height", self.path);
        }

        // Columns = terminal width (80 when piped, D-02). Diverges from cowsay's
        // fixed width on purpose — a visual render should fill the terminal.
        let cols = crate::core::output::terminal_width() as u32;
        let rows = compute_rows(cols, src_w, src_h);

        if self.braille {
            render_braille(&img, cols, rows, self.invert);
        } else {
            render_ramp(&img, cols, rows, self.invert);
        }

        Ok(())
    }
}

/// Render `img` through the dark→light ASCII ramp at `cols`×`rows` (ASCI-01 /
/// ASCI-V2-01). Each cell keeps its luma for the ramp index and, when
/// [`is_color_on`] is true, ALSO samples the cell RGB and wraps the glyph in a
/// gated `.truecolor()`; the plain branch is byte-identical minus the escape (SC4).
/// `--invert` flips the luma at the single [`apply_invert`] seam before indexing.
fn render_ramp(img: &image::DynamicImage, cols: u32, rows: u32, invert: bool) {
    // resize_exact ignores aspect (we computed `rows` to correct it), then
    // to_luma8 collapses to one brightness byte per pixel (row-major); to_rgb8
    // keeps the per-cell color for the gated truecolor path.
    let resized = img.resize_exact(cols, rows, image::imageops::FilterType::Triangle);
    let gray = resized.to_luma8();
    let rgb = resized.to_rgb8();
    let color_on = is_color_on();

    for y in 0..rows {
        let mut line = String::with_capacity(cols as usize);
        for x in 0..cols {
            let luma = apply_invert(gray.get_pixel(x, y).0[0], invert);
            let glyph = luma_to_char(luma, RAMP) as char;
            if color_on {
                let [r, g, b] = rgb.get_pixel(x, y).0;
                // The SOLE color gate (Pitfall 4): only reached when color is on,
                // so the plain branch below stays byte-identical minus the escape.
                let _ = write!(line, "{}", glyph.truecolor(r, g, b));
            } else {
                line.push(glyph);
            }
        }
        println!("{line}");
    }
}

/// Render `img` through the 2×4 Unicode-braille engine at `cols`×`rows`
/// (ASCI-V2-01). The image is resized to `2*cols × 4*rows`; each output cell's 8
/// sub-pixels set their braille bit ([`braille_glyph`]) past [`BRAILLE_THRESHOLD`]
/// (post-`--invert`), and color — when [`is_color_on`] — is ONE truecolor averaged
/// over the cell's 8 sub-pixels (braille and color stay orthogonal).
fn render_braille(img: &image::DynamicImage, cols: u32, rows: u32, invert: bool) {
    // Each output cell is 2 sub-columns × 4 sub-rows.
    let sub_w = cols * 2;
    let sub_h = rows * 4;
    let resized = img.resize_exact(sub_w, sub_h, image::imageops::FilterType::Triangle);
    let gray = resized.to_luma8();
    let rgb = resized.to_rgb8();
    let color_on = is_color_on();

    for cy in 0..rows {
        let mut line = String::with_capacity(cols as usize);
        for cx in 0..cols {
            let mut dots_on = [false; 8];
            let (mut rsum, mut gsum, mut bsum) = (0u32, 0u32, 0u32);
            for (i, &(dx, dy)) in BRAILLE_DOTS.iter().enumerate() {
                let sx = cx * 2 + dx;
                let sy = cy * 4 + dy;
                let luma = apply_invert(gray.get_pixel(sx, sy).0[0], invert);
                dots_on[i] = luma >= BRAILLE_THRESHOLD;
                let [r, g, b] = rgb.get_pixel(sx, sy).0;
                rsum += r as u32;
                gsum += g as u32;
                bsum += b as u32;
            }
            let glyph = braille_glyph(dots_on);
            if color_on {
                // One averaged truecolor per glyph (8 sub-pixels), gated (SC4).
                let (r, g, b) = ((rsum / 8) as u8, (gsum / 8) as u8, (bsum / 8) as u8);
                let _ = write!(line, "{}", glyph.truecolor(r, g, b));
            } else {
                line.push(glyph);
            }
        }
        println!("{line}");
    }
}

/// Map a luma byte (0..=255) to a ramp glyph. Monotonic: luma 0 → `ramp[0]`
/// (darkest of a dark→light ramp), luma 255 → `ramp[last]` (lightest). The
/// `* (len-1) / 255` scaling never indexes out of bounds for any byte value.
///
/// Pure and crate-free so it is unit-testable without a terminal.
fn luma_to_char(luma: u8, ramp: &[u8]) -> u8 {
    let idx = (luma as usize * (ramp.len() - 1)) / 255;
    ramp[idx]
}

/// The single `--invert` luma seam (ASCI-V2-01): `255 - luma` when `invert`, else
/// `luma` unchanged. Applied once per (sub-)pixel BEFORE the ramp index / braille
/// per-dot threshold, so inverting composes with both the ramp and braille engines
/// without touching the true RGB used for color. Pure → unit-testable.
fn apply_invert(luma: u8, invert: bool) -> u8 {
    if invert {
        255 - luma
    } else {
        luma
    }
}

/// Build the Unicode-braille glyph for one 2×4 cell from its 8 sub-pixel "on"
/// flags in dot order (dot1..dot8) (ASCI-V2-01). Dot `i+1` sets bit `DOT_BITS[i]`;
/// the glyph is `U+2800 + mask`. Pure and terminal-free so the bit-order is
/// unit-lockable independent of any image.
fn braille_glyph(dots_on: [bool; 8]) -> char {
    let mut mask: u8 = 0;
    for (i, &on) in dots_on.iter().enumerate() {
        if on {
            mask |= 1 << DOT_BITS[i];
        }
    }
    // 0x2800..=0x28FF is always a valid scalar; the fallback keeps this total.
    char::from_u32(0x2800 + mask as u32).unwrap_or('\u{2800}')
}

/// Compute the row count for a `cols`-wide render of a `src_w` × `src_h` image.
///
/// `rows = (cols * src_h / src_w / 2).max(1)`. The `/2` corrects the ~2:1
/// terminal-cell aspect (a character cell is roughly twice as tall as it is wide),
/// so a square image renders as a square block rather than a tall one. Clamped to
/// `>= 1` so even an extreme aspect ratio (very wide source) yields at least one
/// row — never a zero-height render. `src_w` is assumed non-zero (the caller
/// guards a zero-dimension image before calling).
///
/// `cols`/`src_w`/`src_h` come from an untrusted, user-supplied image, so the
/// `cols * src_h` product is computed in `u64` to avoid a `u32` overflow (WR-02:
/// a crafted-but-decodable large image could push `cols * src_h` past `u32::MAX`,
/// which panics in a debug/test build and silently wraps in release). The result
/// is clamped to `u16::MAX` rows — far more than any real terminal — so the
/// downstream `resize_exact`/buffer math also stays in a sane range.
///
/// Pure and crate-free so it is unit-testable without a terminal.
fn compute_rows(cols: u32, src_w: u32, src_h: u32) -> u32 {
    let rows = (cols as u64 * src_h as u64 / src_w as u64 / 2).max(1);
    rows.min(u16::MAX as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `luma_to_char` maps the byte extremes to the ramp ends: 0 → first glyph
    /// (darkest), 255 → last glyph (lightest). This is the dark→light contract.
    #[test]
    fn luma_to_char_maps_extremes() {
        assert_eq!(
            luma_to_char(0, RAMP),
            RAMP[0],
            "luma 0 must map to the darkest (first) ramp glyph"
        );
        assert_eq!(
            luma_to_char(255, RAMP),
            RAMP[RAMP.len() - 1],
            "luma 255 must map to the lightest (last) ramp glyph"
        );
    }

    /// `luma_to_char` is monotonic non-decreasing across the full byte range and
    /// never indexes out of bounds (no panic for any luma 0..=255).
    #[test]
    fn luma_to_char_is_monotonic_and_in_bounds() {
        let mut prev = luma_to_char(0, RAMP);
        let first_idx = RAMP.iter().position(|&c| c == prev).unwrap();
        let mut prev_idx = first_idx;
        for luma in 0u16..=255 {
            let c = luma_to_char(luma as u8, RAMP);
            let idx = RAMP
                .iter()
                .position(|&r| r == c)
                .expect("glyph must come from the ramp (in bounds)");
            assert!(
                idx >= prev_idx,
                "ramp index must be non-decreasing: luma {luma} gave idx {idx} < {prev_idx}"
            );
            prev = c;
            prev_idx = idx;
        }
        let _ = prev;
    }

    /// ASCI-V2-01 — `apply_invert` flips the luma ONLY when the flag is set, and
    /// the flip is exactly `255 - luma` (the locked seam formula).
    #[test]
    fn apply_invert_is_255_minus_luma() {
        assert_eq!(apply_invert(0, false), 0, "no-op when invert is off");
        assert_eq!(apply_invert(200, false), 200);
        assert_eq!(apply_invert(0, true), 255, "0 inverts to 255");
        assert_eq!(apply_invert(255, true), 0, "255 inverts to 0");
        assert_eq!(apply_invert(200, true), 55, "200 inverts to 55");
    }

    /// ASCI-V2-01 — inverting a dark value at the seam selects a LIGHTER ramp glyph
    /// (the seam is applied BEFORE `luma_to_char`), and the extremes swap.
    #[test]
    fn invert_selects_the_complementary_ramp_glyph() {
        // Extremes swap: dark→lightest glyph, light→darkest glyph.
        assert_eq!(luma_to_char(apply_invert(0, true), RAMP), RAMP[RAMP.len() - 1]);
        assert_eq!(luma_to_char(apply_invert(255, true), RAMP), RAMP[0]);
        // A generic dark value maps to a strictly lighter glyph once inverted.
        let luma = 40u8;
        let normal = luma_to_char(apply_invert(luma, false), RAMP);
        let inverted = luma_to_char(apply_invert(luma, true), RAMP);
        let n_idx = RAMP.iter().position(|&c| c == normal).unwrap();
        let i_idx = RAMP.iter().position(|&c| c == inverted).unwrap();
        assert!(
            i_idx > n_idx,
            "inverting a dark value must select a lighter ramp glyph: {n_idx} !< {i_idx}"
        );
    }

    /// ASCI-V2-01 — LOCK the braille bit-order: each single dot maps to its
    /// documented `U+28xx` glyph, and the all-off / all-on cells are U+2800 /
    /// U+28FF. Guards the `DOT_BITS` mapping (Unicode bit `i` = dot `i+1`).
    #[test]
    fn braille_bit_order_is_locked() {
        assert_eq!(DOT_BITS, [0, 1, 2, 3, 4, 5, 6, 7], "bit i must be dot i+1");
        // No dots → blank braille cell.
        assert_eq!(braille_glyph([false; 8]), '\u{2800}');
        // All dots → the full 8-dot cell.
        assert_eq!(braille_glyph([true; 8]), '\u{28FF}');
        // Each single dot lights exactly its bit: dot(i+1) → U+2800 + (1 << i).
        for i in 0..8 {
            let mut d = [false; 8];
            d[i] = true;
            let expected = char::from_u32(0x2800 + (1u32 << i)).unwrap();
            assert_eq!(
                braille_glyph(d),
                expected,
                "dot {} must set bit {i} (glyph U+{:04X})",
                i + 1,
                0x2800 + (1u32 << i)
            );
        }
        // Spot-check the documented examples: dot1 → U+2801, dot7 → U+2840,
        // dot8 → U+2880.
        let mut d1 = [false; 8];
        d1[0] = true;
        assert_eq!(braille_glyph(d1), '\u{2801}');
        let mut d7 = [false; 8];
        d7[6] = true;
        assert_eq!(braille_glyph(d7), '\u{2840}');
        let mut d8 = [false; 8];
        d8[7] = true;
        assert_eq!(braille_glyph(d8), '\u{2880}');
    }

    /// The braille positions cover all 8 distinct sub-pixels of the 2×4 cell
    /// (2 columns × 4 rows), so no sub-pixel is double-sampled or missed.
    #[test]
    fn braille_dot_positions_tile_the_cell() {
        let mut seen = std::collections::HashSet::new();
        for &(dx, dy) in BRAILLE_DOTS.iter() {
            assert!(dx < 2 && dy < 4, "({dx},{dy}) must lie in the 2×4 cell");
            assert!(seen.insert((dx, dy)), "({dx},{dy}) sampled twice");
        }
        assert_eq!(seen.len(), 8, "all 8 sub-pixels must be covered");
    }

    /// A square source at 80 columns renders 40 rows (`80 * h / w / 2` with
    /// `w == h` → `80 / 2 = 40`) — the `/2` aspect correction.
    #[test]
    fn compute_rows_square_source_halves() {
        assert_eq!(compute_rows(80, 100, 100), 40);
        assert_eq!(compute_rows(80, 8, 8), 40);
    }

    /// A wide source (w > h) yields fewer rows than a square one at the same
    /// width; a tall source (h > w) yields more — the formula tracks aspect.
    #[test]
    fn compute_rows_tracks_aspect() {
        let square = compute_rows(80, 100, 100); // 40
        let wide = compute_rows(80, 200, 100); // 80*100/200/2 = 20
        let tall = compute_rows(80, 100, 200); // 80*200/100/2 = 80
        assert_eq!(wide, 20, "wide source → fewer rows");
        assert_eq!(tall, 80, "tall source → more rows");
        assert!(
            wide < square && square < tall,
            "aspect ordering: {wide} < {square} < {tall}"
        );
    }

    /// An extreme aspect ratio that would round the row count to 0 is clamped to
    /// 1 — never a zero-height render (no empty output, no degenerate loop).
    #[test]
    fn compute_rows_clamps_to_at_least_one() {
        // A very wide, short source: 80 * 1 / 1000 / 2 = 0 → clamp to 1.
        assert_eq!(compute_rows(80, 1000, 1), 1);
        // Degenerate cols=1 still yields >= 1.
        assert_eq!(compute_rows(1, 1000, 1), 1);
    }

    /// WR-02 — adversarial-but-decodable large dimensions must NOT overflow.
    /// `cols * src_h` here is `65535 * 100000 = 6_553_500_000`, which exceeds
    /// `u32::MAX` (4_294_967_295). The old plain-`u32` math panicked here in a
    /// debug/test build (overflow checks on); the `u64` intermediate computes it
    /// cleanly. The result is also clamped to `u16::MAX` rows so the downstream
    /// render math stays in a sane range — no panic for any decodable image.
    #[test]
    fn compute_rows_no_overflow_on_large_dimensions() {
        // cols and src_h both large: the u32 product would wrap/panic.
        let rows = compute_rows(u16::MAX as u32, 1, 100_000);
        assert!(
            rows <= u16::MAX as u32,
            "rows must be clamped to a sane (<= u16::MAX) value, got {rows}"
        );
        assert!(rows >= 1, "rows must still be at least 1, got {rows}");
        // The most extreme decodable case: cols, src_h, src_w all at u32::MAX.
        // u32 arithmetic would overflow on the very first multiply; u64 does not.
        let extreme = compute_rows(u32::MAX, u32::MAX, u32::MAX);
        assert!(
            extreme >= 1 && extreme <= u16::MAX as u32,
            "extreme dimensions must clamp without panicking, got {extreme}"
        );
    }
}
