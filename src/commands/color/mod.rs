//! The `color` command: convert a color between hex and RGB formats and show
//! every representation plus a truecolor swatch (COLR-01). The first command to
//! reuse the Phase-1 `core::output` color gate — the swatch is gated on
//! [`is_color_on`] so it degrades to a plain `██████████` line when piped,
//! byte-identical minus ANSI (D-10). There is exactly ONE color path here: no
//! owo-colors global-override call, and no background-color swatch built from
//! raw SGR escapes (D-10 rejects that — it strips to a blank line).
//!
//! Flow (Pattern 1 — thin orchestrator over pure parse/convert helpers):
//! `run()` acquires the input string via [`crate::core::input::read_input`]
//! (arg → piped stdin → no-arg interactive TTY → exit 2), `parse_color`s it to
//! an `(r,g,b)` triple (auto-detecting hex vs RGB, D-13), computes HSL, prints
//! the aligned D-10 block, then the gated swatch. All parsing returns `Result`;
//! malformed hex / out-of-range RGB error to stderr (exit 1) with no panic and
//! no array-index overflow (T-02-06).

use anyhow::{bail, Context};
use clap::Args;
use owo_colors::OwoColorize;

use crate::commands::RunCommand;
use crate::core::output::is_color_on;

/// `box color [INPUT]` — convert between hex and RGB (COLR-01).
///
/// `INPUT` is auto-detected (D-13): hex (`#3b82f6`, `3b82f6`, short `#abc`) or
/// RGB (`"59,130,246"` / `"59 130 246"`). Omit it to read from piped stdin.
#[derive(Debug, Args)]
pub struct ColorArgs {
    /// A hex (`#3b82f6` / `3b82f6` / `#abc`) or RGB (`59,130,246`) color; omit to read stdin.
    pub input: Option<String>,
}

impl RunCommand for ColorArgs {
    fn run(self) -> anyhow::Result<()> {
        // arg → piped stdin → exit-2 on a no-arg interactive TTY (D-04 branch 3).
        // color is the first live caller of the String reader `read_input`.
        let raw = crate::core::input::read_input(self.input)?;
        let (r, g, b) = parse_color(raw.trim())?;
        let (h, s, l) = rgb_to_hsl(r, g, b);

        // Aligned label block (D-10). Labels are padded to a fixed width so the
        // `:` columns line up; the snapshot locks this exact layout.
        println!("  Hex   : #{r:02X}{g:02X}{b:02X}");
        println!("  RGB   : rgb({r}, {g}, {b})");
        println!("  Tuple : {r} {g} {b}");
        println!("  HSL   : hsl({h}, {s}%, {l}%)");
        println!();

        // The ONLY color path: gate the truecolor swatch on the single Phase-1
        // decision. Piped/NO_COLOR → the plain glyphs survive, byte-identical
        // minus ANSI (D-10). No global-override call, no raw-SGR background fill.
        let swatch = "██████████";
        if is_color_on() {
            println!("  {}", swatch.truecolor(r, g, b));
        } else {
            println!("  {swatch}");
        }
        Ok(())
    }
}

/// Parse a color string to an `(r, g, b)` triple, auto-detecting hex vs RGB
/// (D-13). Hex: a leading `#` or an all-hex-digit string of length 3 or 6 —
/// short forms (`#abc` / `abc`) expand each nibble by doubling (Pitfall 8:
/// `#abc` → `(170, 187, 204)`). RGB: a comma- or whitespace-separated triple of
/// `u8` components; out-of-range components error (no panic, T-02-06). Anything
/// else `bail!`s with a hint.
fn parse_color(s: &str) -> anyhow::Result<(u8, u8, u8)> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        bail!("empty color input; expected a hex (#3b82f6 / #abc) or RGB (59,130,246) value");
    }

    // RGB form: contains a separator (comma or whitespace).
    if trimmed.contains(',') || trimmed.split_whitespace().count() > 1 {
        return parse_rgb(trimmed);
    }

    // Otherwise treat as hex. A leading '#' is optional; the remainder must be
    // all hex digits of length 3 or 6.
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.chars().all(|c| c.is_ascii_hexdigit()) && (hex.len() == 3 || hex.len() == 6) {
        return parse_hex(hex);
    }

    bail!(
        "unrecognized color '{s}'; expected hex (#3b82f6 / 3b82f6 / #abc) \
         or RGB (59,130,246 / '59 130 246')"
    );
}

/// Parse the hex digits (no leading `#`), already validated to be all-hex of
/// length 3 or 6. Short form doubles each nibble (`abc` → `aabbcc`).
fn parse_hex(hex: &str) -> anyhow::Result<(u8, u8, u8)> {
    let full = if hex.len() == 3 {
        // Double each nibble: "abc" -> "aabbcc" (Pitfall 8 / D-13).
        hex.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        hex.to_string()
    };
    // `full` is exactly 6 validated hex digits here, so these slices and parses
    // cannot fail — but use `?`/context anyway so there is no `.unwrap()` on a
    // user-derived value (defense in depth, T-02-06).
    let r = u8::from_str_radix(&full[0..2], 16).context("invalid red channel")?;
    let g = u8::from_str_radix(&full[2..4], 16).context("invalid green channel")?;
    let b = u8::from_str_radix(&full[4..6], 16).context("invalid blue channel")?;
    Ok((r, g, b))
}

/// Parse an RGB triple from comma- or whitespace-separated components. Each must
/// be a `u8` (0-255); an out-of-range or non-numeric component errors cleanly.
fn parse_rgb(s: &str) -> anyhow::Result<(u8, u8, u8)> {
    let parts: Vec<&str> = if s.contains(',') {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect()
    } else {
        s.split_whitespace().collect()
    };
    if parts.len() != 3 {
        bail!(
            "RGB needs exactly 3 components (got {}): '{s}'",
            parts.len()
        );
    }
    let r: u8 = parts[0]
        .parse()
        .with_context(|| format!("red component '{}' out of range (0-255)", parts[0]))?;
    let g: u8 = parts[1]
        .parse()
        .with_context(|| format!("green component '{}' out of range (0-255)", parts[1]))?;
    let b: u8 = parts[2]
        .parse()
        .with_context(|| format!("blue component '{}' out of range (0-255)", parts[2]))?;
    Ok((r, g, b))
}

/// Convert RGB (0-255) to HSL — h in 0-359, s and l in 0-100, integer-rounded
/// for display (the closed-form from RESEARCH; verified against CSS Color
/// Module Level 3). Pure + crate-free so it is unit-testable.
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let (rf, gf, bf) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    let d = max - min;
    let (h, s) = if d == 0.0 {
        (0.0, 0.0)
    } else {
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let h = if max == rf {
            ((gf - bf) / d).rem_euclid(6.0)
        } else if max == gf {
            (bf - rf) / d + 2.0
        } else {
            (rf - gf) / d + 4.0
        };
        (h * 60.0, s)
    };
    (
        (h.round() as u16) % 360,
        (s * 100.0).round() as u8,
        (l * 100.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `#3b82f6`, `3b82f6` (no hash), and the comma/space RGB forms all parse to
    /// the same `(59, 130, 246)` triple (D-13 bidirectional, auto-detected).
    #[test]
    fn parse_color_hex_and_rgb_agree() {
        let expected = (59u8, 130u8, 246u8);
        assert_eq!(parse_color("#3b82f6").unwrap(), expected);
        assert_eq!(parse_color("3b82f6").unwrap(), expected);
        assert_eq!(parse_color("59,130,246").unwrap(), expected);
        assert_eq!(parse_color("59 130 246").unwrap(), expected);
        // Mixed whitespace/comma spacing is tolerated.
        assert_eq!(parse_color("59, 130, 246").unwrap(), expected);
    }

    /// Short hex expands by nibble-doubling: `#abc` → `(170, 187, 204)`
    /// (Pitfall 8). Both `#abc` and bare `abc` work.
    #[test]
    fn parse_color_short_hex_expands() {
        assert_eq!(parse_color("#abc").unwrap(), (170, 187, 204));
        assert_eq!(parse_color("abc").unwrap(), (170, 187, 204));
    }

    /// Out-of-range RGB and malformed hex are `Err` (→ exit 1), never a panic
    /// or an index overflow (T-02-06).
    #[test]
    fn parse_color_rejects_bad_input() {
        assert!(parse_color("300,0,0").is_err(), "300 > 255 must error");
        assert!(parse_color("#zz").is_err(), "non-hex digits must error");
        assert!(parse_color("#12").is_err(), "len-2 hex must error");
        assert!(parse_color("1,2").is_err(), "2-component RGB must error");
        assert!(
            parse_color("1,2,3,4").is_err(),
            "4-component RGB must error"
        );
        assert!(parse_color("").is_err(), "empty must error");
    }

    /// `rgb_to_hsl(59,130,246)` is within ±1 of the reference `(217, 91, 60)`.
    #[test]
    fn rgb_to_hsl_reference_value() {
        let (h, s, l) = rgb_to_hsl(59, 130, 246);
        assert!((h as i32 - 217).abs() <= 1, "h={h} expected ~217");
        assert!((s as i32 - 91).abs() <= 1, "s={s} expected ~91");
        assert!((l as i32 - 60).abs() <= 1, "l={l} expected ~60");
    }

    /// Greyscale is fully desaturated (s = 0) and the hue degenerates to 0.
    #[test]
    fn rgb_to_hsl_greyscale_is_desaturated() {
        let (h, s, l) = rgb_to_hsl(128, 128, 128);
        assert_eq!(h, 0);
        assert_eq!(s, 0);
        assert!((l as i32 - 50).abs() <= 1, "mid-grey l={l} expected ~50");
    }
}
