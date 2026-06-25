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

/// The `rgb` sub-object of `box color --json` (D-17 nested). Snake_case `r`/`g`/`b`
/// (u8) so a PS7 script reads `.rgb.r`.
#[derive(serde::Serialize)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

/// The `hsl` sub-object of `box color --json` (D-17 nested). `h` in 0-359 (u16),
/// `s`/`l` in 0-100 (u8) — matches the `rgb_to_hsl` return types.
#[derive(serde::Serialize)]
struct Hsl {
    h: u16,
    s: u8,
    l: u8,
}

/// The `box color --json` document (D-17 LOCKED NESTED shape):
/// `{hex, rgb:{r,g,b}, hsl:{h,s,l}}` (D-01 scalar → flat object with nested
/// sub-objects). SPINE-04 (the human block tees to the clipboard via `out_line`).
///
/// **Hex case is LOCKED LOWERCASE** here (`#rrggbb` via `{:02x}`) so the
/// `json_purity` test is deterministic — note the HUMAN render at the
/// `Hex` line uses UPPERCASE `#{:02X}`; only the JSON `hex` field is lowercased.
/// The swatch is display-only and is NOT serialized.
#[derive(serde::Serialize)]
struct ColorOutput {
    hex: String,
    rgb: Rgb,
    hsl: Hsl,
}

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

        // Fork on `is_json_on()` FIRST (Pitfall 1): under `--json` emit the nested
        // `{hex, rgb, hsl}` document (hex LOWERCASE-locked) and omit the swatch
        // (display-only). The `(r,g,b)`/`(h,s,l)` already computed above feed both
        // paths, so the JSON can never drift from the printed block.
        if crate::core::output::is_json_on() {
            let doc = ColorOutput {
                hex: format!("#{r:02x}{g:02x}{b:02x}"),
                rgb: Rgb { r, g, b },
                hsl: Hsl { h, s, l },
            };
            crate::core::output::emit_json(&doc)?;
            return Ok(());
        }

        // Aligned label block (D-10). Labels are padded to a fixed width so the
        // `:` columns line up; the snapshot locks this exact layout. Routed through
        // `out_line` so `--clip` tees the whole block (SPINE-04). The hex row keeps
        // its UPPERCASE human spelling (only the JSON `hex` field is lowercased).
        crate::core::output::out_line(&format!("  Hex   : #{r:02X}{g:02X}{b:02X}"));
        crate::core::output::out_line(&format!("  RGB   : rgb({r}, {g}, {b})"));
        crate::core::output::out_line(&format!("  Tuple : {r} {g} {b}"));
        crate::core::output::out_line(&format!("  HSL   : hsl({h}, {s}%, {l}%)"));
        crate::core::output::out_line("");

        // The ONLY color path: gate the truecolor swatch on the single Phase-1
        // decision. Piped/NO_COLOR → the plain glyphs survive, byte-identical
        // minus ANSI (D-10). No global-override call, no raw-SGR background fill.
        // The swatch is part of the human render → route through `out_line` so
        // `--clip` captures the full printed block (the styled variant is never
        // reached under `--clip`, which forces COLOR_ON=false).
        let swatch = "██████████";
        if is_color_on() {
            // Color is on only when NOT piped/--clip/--json; print the styled
            // swatch directly (the ANSI must NOT enter the clip buffer, but
            // is_color_on() is already false under --clip, so this branch is
            // clip-unreachable — keep the styled print here, plain via out_line).
            println!("  {}", swatch.truecolor(r, g, b));
        } else {
            crate::core::output::out_line(&format!("  {swatch}"));
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
