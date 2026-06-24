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
//! Monochrome v1 (D-03): plain ASCII to stdout, NO color path. The module imports
//! neither `owo_colors` nor `is_color_on` — colored ASCII is VIS-V2-01, and the
//! ramp emit is the clean seam where v2 color would attach.
//!
//! `image::open` detects the format from the FILE EXTENSION, not the content
//! (Pitfall 2), so the path's extension must match the actual format (PNG/JPEG).
//! A missing or undecodable file is mapped to a clean exit-1 anyhow error via
//! `.with_context(...)?` — never a panic (FOUND-05 / T-04A-02).

use anyhow::Context;
use clap::Args;
use image::GenericImageView;

use crate::commands::RunCommand;

/// The dark→light ASCII ramp (D-02). `b" .:-=+*#%@"` runs from a space (darkest,
/// luma 0) to `@` (lightest, luma 255). The exact glyph set is discretion
/// (RESEARCH OQ-1); a 10-step ramp gives a smooth tonal range. Indexed by
/// [`luma_to_char`].
const RAMP: &[u8] = b" .:-=+*#%@";

/// `box ascii <PATH>` — render a PNG or JPEG image as ASCII art (ASCI-01).
///
/// The image is decoded, resized to the current terminal width (80 columns when
/// piped), and each pixel's brightness is mapped to a character on a dark→light
/// ramp. Monochrome — no color (colored ASCII art is a future enhancement).
#[derive(Debug, Args)]
pub struct AsciiArgs {
    /// Path to a PNG or JPEG image. The extension must match the actual format
    /// (the decoder selects by extension, not by inspecting the file contents).
    pub path: String,
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

        // resize_exact ignores aspect (we computed `rows` to correct it), then
        // to_luma8 collapses to one brightness byte per pixel (row-major).
        let gray = img
            .resize_exact(cols, rows, image::imageops::FilterType::Triangle)
            .to_luma8();
        let buf = gray.as_raw();

        for y in 0..rows {
            let mut line = String::with_capacity(cols as usize);
            for x in 0..cols {
                // Index in `usize` (WR-02): `y * cols + x` in `u32` can overflow
                // for a large `rows`×`cols` render (debug-build panic, release
                // silent wrap). `usize` is wide enough for any real buffer.
                let idx = y as usize * cols as usize + x as usize;
                let luma = buf[idx];
                line.push(luma_to_char(luma, RAMP) as char);
            }
            println!("{line}");
        }

        Ok(())
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
