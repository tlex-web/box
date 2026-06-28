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

/// The `box color --json` document (D-17 LOCKED NESTED shape, COLR-V2-01 additive):
/// `{hex, rgb:{r,g,b}, hsl:{h,s,l}, name, nearest}` (D-01 scalar → flat object with
/// nested sub-objects). SPINE-04 (the human block tees to the clipboard via
/// `out_line`).
///
/// **Hex case is LOCKED LOWERCASE** here (`#rrggbb` via `{:02x}`) so the
/// `json_purity` test is deterministic — note the HUMAN render at the
/// `Hex` line uses UPPERCASE `#{:02X}`; only the JSON `hex` field is lowercased.
/// The swatch is display-only and is NOT serialized.
///
/// `name` (COLR-V2-01) is the EXACT CSS keyword for the resolved RGB or `null`;
/// `nearest` is the redmean-closest CSS keyword. BOTH are always-present (a stable
/// schema): an exact color has `name == nearest`, an arbitrary color has
/// `name: null` + a non-empty `nearest`.
#[derive(serde::Serialize)]
struct ColorOutput {
    hex: String,
    rgb: Rgb,
    hsl: Hsl,
    name: Option<String>,
    nearest: String,
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
        // CSS-name resolution (COLR-V2-01): `exact` is the precise keyword (or
        // None); `nearest` is the redmean-closest keyword (always some). Both feed
        // the JSON and the human Name row from the SAME helpers (no-drift).
        let exact = exact_name(r, g, b);
        let nearest = nearest_name(r, g, b);

        // Fork on `is_json_on()` FIRST (Pitfall 1): under `--json` emit the nested
        // `{hex, rgb, hsl, name, nearest}` document (hex LOWERCASE-locked) and omit
        // the swatch (display-only). The values already computed above feed both
        // paths, so the JSON can never drift from the printed block.
        if crate::core::output::is_json_on() {
            let doc = ColorOutput {
                hex: format!("#{r:02x}{g:02x}{b:02x}"),
                rgb: Rgb { r, g, b },
                hsl: Hsl { h, s, l },
                name: exact.map(str::to_string),
                nearest: nearest.to_string(),
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
        // Name row (COLR-V2-01): an EXACT match prints the bare keyword; an
        // approximate prints the redmean-closest keyword prefixed with `~`.
        match exact {
            Some(name) => crate::core::output::out_line(&format!("  Name  : {name}")),
            None => crate::core::output::out_line(&format!("  Name  : ~{nearest}")),
        }
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

    // HSL functional form (COLR-V2-01) — MUST be checked BEFORE the RGB separator
    // branch: a bare `hsl(210, 100%, 50%)` contains commas and would otherwise
    // mis-hit `parse_rgb`. Case-insensitive `hsl(` prefix, char-boundary-safe.
    if trimmed
        .get(..4)
        .is_some_and(|p| p.eq_ignore_ascii_case("hsl("))
    {
        return parse_hsl(trimmed);
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

    // CSS named color (COLR-V2-01) — AFTER the hex check (no CSS name is an all-hex
    // string of length 3/6, so there is no collision) and BEFORE the final bail!.
    // Case-insensitive.
    if let Some(rgb) = name_to_rgb(trimmed) {
        return Ok(rgb);
    }

    bail!(
        "unrecognized color '{s}'; expected hex (#3b82f6 / 3b82f6 / #abc), \
         RGB (59,130,246 / '59 130 246'), HSL (hsl(210, 100%, 50%)), \
         or a CSS color name (e.g. cornflowerblue)"
    );
}

/// Parse the CSS functional HSL form `hsl(H, S%, L%)` (also the modern space form
/// `hsl(H S% L%)`). `H` is degrees 0–360, `S`/`L` are percentages 0–100 (the `%`
/// is tolerated and optional). Out-of-range / non-numeric components error cleanly
/// (exit 1, no panic — V5). The `hsl(` prefix was already matched by the caller.
fn parse_hsl(s: &str) -> anyhow::Result<(u8, u8, u8)> {
    let inner = s
        .get(4..)
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or_else(|| anyhow::anyhow!("malformed hsl(...): expected 'hsl(H, S%, L%)', got '{s}'"))?;

    let parts: Vec<&str> = if inner.contains(',') {
        inner
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect()
    } else {
        inner.split_whitespace().collect()
    };
    if parts.len() != 3 {
        bail!(
            "hsl(...) needs exactly 3 components (H, S%, L%), got {}: '{s}'",
            parts.len()
        );
    }

    let h: f64 = parts[0]
        .trim()
        .parse()
        .with_context(|| format!("hue '{}' is not a number", parts[0]))?;
    let sp: f64 = parts[1]
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse()
        .with_context(|| format!("saturation '{}' is not a number", parts[1]))?;
    let lp: f64 = parts[2]
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse()
        .with_context(|| format!("lightness '{}' is not a number", parts[2]))?;

    if !(0.0..=360.0).contains(&h) {
        bail!("hue {h} out of range (0-360)");
    }
    if !(0.0..=100.0).contains(&sp) {
        bail!("saturation {sp} out of range (0-100)");
    }
    if !(0.0..=100.0).contains(&lp) {
        bail!("lightness {lp} out of range (0-100)");
    }
    Ok(hsl_to_rgb(h, sp, lp))
}

/// Look up a CSS color name (case-insensitive) in [`CSS_NAMES`], returning its
/// RGB triple. Pure so the table is unit-testable against the anchors.
fn name_to_rgb(name: &str) -> Option<(u8, u8, u8)> {
    let key = name.to_ascii_lowercase();
    CSS_NAMES
        .iter()
        .find(|(n, _)| *n == key)
        .map(|(_, rgb)| *rgb)
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

/// Convert HSL back to RGB — the inverse of [`rgb_to_hsl`], the standard closed
/// form. `h` is degrees (any value, wrapped to 0–360), `s`/`l` are percentages
/// 0–100. Pure + crate-free so the round-trip (rgb→hsl→rgb within ±1) is
/// unit-testable. Used by [`parse_hsl`].
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0);
    let s = (s / 100.0).clamp(0.0, 1.0);
    let l = (l / 100.0).clamp(0.0, 1.0);

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x), // 5 and the h == 360 → hp == 0 edge already wrapped
    };
    let m = l - c / 2.0;
    let to_u8 = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

/// The exact CSS keyword for an RGB triple, or `None` (COLR-V2-01). Returns the
/// FIRST table entry that matches exactly (so a shared RGB like aqua/cyan resolves
/// deterministically, and agrees with [`nearest_name`] for an exact color).
fn exact_name(r: u8, g: u8, b: u8) -> Option<&'static str> {
    CSS_NAMES
        .iter()
        .find(|(_, rgb)| *rgb == (r, g, b))
        .map(|(n, _)| *n)
}

/// The redmean-closest CSS keyword for an RGB triple (COLR-V2-01). Uses the
/// hand-rolled weighted-RGB "redmean" distance (NOT plain Euclidean, NOT
/// CIEDE2000) — a cheap perceptual approximation. Keeps the FIRST entry on a tie
/// (so an exact color, distance 0, returns the same keyword as [`exact_name`]).
fn nearest_name(r: u8, g: u8, b: u8) -> &'static str {
    let mut best = CSS_NAMES[0].0;
    let mut best_d = f64::MAX;
    for (name, (nr, ng, nb)) in CSS_NAMES {
        let d = redmean_sq(r, g, b, *nr, *ng, *nb);
        if d < best_d {
            best_d = d;
            best = name;
        }
    }
    best
}

/// Squared redmean color distance between two RGB triples. The weights shift the
/// red/blue channel emphasis by the mean red, approximating perceptual distance
/// far better than plain Euclidean for ~10 lines of pure arithmetic.
fn redmean_sq(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> f64 {
    let rmean = (r1 as f64 + r2 as f64) / 2.0;
    let dr = r1 as f64 - r2 as f64;
    let dg = g1 as f64 - g2 as f64;
    let db = b1 as f64 - b2 as f64;
    (2.0 + rmean / 256.0) * dr * dr + 4.0 * dg * dg + (2.0 + (255.0 - rmean) / 256.0) * db * db
}

/// The ~148 CSS Color Module Level 4 named colors (incl. `rebeccapurple`), as
/// `(lowercase name, (r, g, b))`. Hand-rolled `const` table (no crate, per
/// CONTEXT.md D9-01). Aliases that share an RGB (aqua/cyan, fuchsia/magenta, the
/// gray/grey pairs) are all listed; lookups are deterministic via first-match.
const CSS_NAMES: &[(&str, (u8, u8, u8))] = &[
    ("aliceblue", (240, 248, 255)),
    ("antiquewhite", (250, 235, 215)),
    ("aqua", (0, 255, 255)),
    ("aquamarine", (127, 255, 212)),
    ("azure", (240, 255, 255)),
    ("beige", (245, 245, 220)),
    ("bisque", (255, 228, 196)),
    ("black", (0, 0, 0)),
    ("blanchedalmond", (255, 235, 205)),
    ("blue", (0, 0, 255)),
    ("blueviolet", (138, 43, 226)),
    ("brown", (165, 42, 42)),
    ("burlywood", (222, 184, 135)),
    ("cadetblue", (95, 158, 160)),
    ("chartreuse", (127, 255, 0)),
    ("chocolate", (210, 105, 30)),
    ("coral", (255, 127, 80)),
    ("cornflowerblue", (100, 149, 237)),
    ("cornsilk", (255, 248, 220)),
    ("crimson", (220, 20, 60)),
    ("cyan", (0, 255, 255)),
    ("darkblue", (0, 0, 139)),
    ("darkcyan", (0, 139, 139)),
    ("darkgoldenrod", (184, 134, 11)),
    ("darkgray", (169, 169, 169)),
    ("darkgreen", (0, 100, 0)),
    ("darkgrey", (169, 169, 169)),
    ("darkkhaki", (189, 183, 107)),
    ("darkmagenta", (139, 0, 139)),
    ("darkolivegreen", (85, 107, 47)),
    ("darkorange", (255, 140, 0)),
    ("darkorchid", (153, 50, 204)),
    ("darkred", (139, 0, 0)),
    ("darksalmon", (233, 150, 122)),
    ("darkseagreen", (143, 188, 143)),
    ("darkslateblue", (72, 61, 139)),
    ("darkslategray", (47, 79, 79)),
    ("darkslategrey", (47, 79, 79)),
    ("darkturquoise", (0, 206, 209)),
    ("darkviolet", (148, 0, 211)),
    ("deeppink", (255, 20, 147)),
    ("deepskyblue", (0, 191, 255)),
    ("dimgray", (105, 105, 105)),
    ("dimgrey", (105, 105, 105)),
    ("dodgerblue", (30, 144, 255)),
    ("firebrick", (178, 34, 34)),
    ("floralwhite", (255, 250, 240)),
    ("forestgreen", (34, 139, 34)),
    ("fuchsia", (255, 0, 255)),
    ("gainsboro", (220, 220, 220)),
    ("ghostwhite", (248, 248, 255)),
    ("gold", (255, 215, 0)),
    ("goldenrod", (218, 165, 32)),
    ("gray", (128, 128, 128)),
    ("green", (0, 128, 0)),
    ("greenyellow", (173, 255, 47)),
    ("grey", (128, 128, 128)),
    ("honeydew", (240, 255, 240)),
    ("hotpink", (255, 105, 180)),
    ("indianred", (205, 92, 92)),
    ("indigo", (75, 0, 130)),
    ("ivory", (255, 255, 240)),
    ("khaki", (240, 230, 140)),
    ("lavender", (230, 230, 250)),
    ("lavenderblush", (255, 240, 245)),
    ("lawngreen", (124, 252, 0)),
    ("lemonchiffon", (255, 250, 205)),
    ("lightblue", (173, 216, 230)),
    ("lightcoral", (240, 128, 128)),
    ("lightcyan", (224, 255, 255)),
    ("lightgoldenrodyellow", (250, 250, 210)),
    ("lightgray", (211, 211, 211)),
    ("lightgreen", (144, 238, 144)),
    ("lightgrey", (211, 211, 211)),
    ("lightpink", (255, 182, 193)),
    ("lightsalmon", (255, 160, 122)),
    ("lightseagreen", (32, 178, 170)),
    ("lightskyblue", (135, 206, 250)),
    ("lightslategray", (119, 136, 153)),
    ("lightslategrey", (119, 136, 153)),
    ("lightsteelblue", (176, 196, 222)),
    ("lightyellow", (255, 255, 224)),
    ("lime", (0, 255, 0)),
    ("limegreen", (50, 205, 50)),
    ("linen", (250, 240, 230)),
    ("magenta", (255, 0, 255)),
    ("maroon", (128, 0, 0)),
    ("mediumaquamarine", (102, 205, 170)),
    ("mediumblue", (0, 0, 205)),
    ("mediumorchid", (186, 85, 211)),
    ("mediumpurple", (147, 112, 219)),
    ("mediumseagreen", (60, 179, 113)),
    ("mediumslateblue", (123, 104, 238)),
    ("mediumspringgreen", (0, 250, 154)),
    ("mediumturquoise", (72, 209, 204)),
    ("mediumvioletred", (199, 21, 133)),
    ("midnightblue", (25, 25, 112)),
    ("mintcream", (245, 255, 250)),
    ("mistyrose", (255, 228, 225)),
    ("moccasin", (255, 228, 181)),
    ("navajowhite", (255, 222, 173)),
    ("navy", (0, 0, 128)),
    ("oldlace", (253, 245, 230)),
    ("olive", (128, 128, 0)),
    ("olivedrab", (107, 142, 35)),
    ("orange", (255, 165, 0)),
    ("orangered", (255, 69, 0)),
    ("orchid", (218, 112, 214)),
    ("palegoldenrod", (238, 232, 170)),
    ("palegreen", (152, 251, 152)),
    ("paleturquoise", (175, 238, 238)),
    ("palevioletred", (219, 112, 147)),
    ("papayawhip", (255, 239, 213)),
    ("peachpuff", (255, 218, 185)),
    ("peru", (205, 133, 63)),
    ("pink", (255, 192, 203)),
    ("plum", (221, 160, 221)),
    ("powderblue", (176, 224, 230)),
    ("purple", (128, 0, 128)),
    ("rebeccapurple", (102, 51, 153)),
    ("red", (255, 0, 0)),
    ("rosybrown", (188, 143, 143)),
    ("royalblue", (65, 105, 225)),
    ("saddlebrown", (139, 69, 19)),
    ("salmon", (250, 128, 114)),
    ("sandybrown", (244, 164, 96)),
    ("seagreen", (46, 139, 87)),
    ("seashell", (255, 245, 238)),
    ("sienna", (160, 82, 45)),
    ("silver", (192, 192, 192)),
    ("skyblue", (135, 206, 235)),
    ("slateblue", (106, 90, 205)),
    ("slategray", (112, 128, 144)),
    ("slategrey", (112, 128, 144)),
    ("snow", (255, 250, 250)),
    ("springgreen", (0, 255, 127)),
    ("steelblue", (70, 130, 180)),
    ("tan", (210, 180, 140)),
    ("teal", (0, 128, 128)),
    ("thistle", (216, 191, 216)),
    ("tomato", (255, 99, 71)),
    ("turquoise", (64, 224, 208)),
    ("violet", (238, 130, 238)),
    ("wheat", (245, 222, 179)),
    ("white", (255, 255, 255)),
    ("whitesmoke", (245, 245, 245)),
    ("yellow", (255, 255, 0)),
    ("yellowgreen", (154, 205, 50)),
];

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

    /// COLR-V2-01 — the four CSS anchors resolve to their known RGB (the table is
    /// correct and case-insensitive).
    #[test]
    fn css_anchors_map_to_known_rgb() {
        assert_eq!(name_to_rgb("black"), Some((0, 0, 0)));
        assert_eq!(name_to_rgb("white"), Some((255, 255, 255)));
        assert_eq!(name_to_rgb("rebeccapurple"), Some((102, 51, 153)));
        assert_eq!(name_to_rgb("cornflowerblue"), Some((100, 149, 237)));
        // Case-insensitive.
        assert_eq!(name_to_rgb("CornflowerBlue"), Some((100, 149, 237)));
        // Unknown name → None (drives the parse_color bail!).
        assert_eq!(name_to_rgb("notacolor"), None);
    }

    /// COLR-V2-01 — `hsl(...)` parses (comma and space forms agree) and a
    /// round-trip rgb→hsl→rgb stays within ±1 per channel.
    #[test]
    fn hsl_parses_and_round_trips() {
        // hsl(210, 100%, 50%) is a standard azure blue → (0, 128, 255).
        assert_eq!(parse_color("hsl(210, 100%, 50%)").unwrap(), (0, 128, 255));
        // The modern space-separated form parses identically.
        assert_eq!(parse_color("hsl(210 100% 50%)").unwrap(), (0, 128, 255));

        // Round-trip a battery of colors through rgb→hsl→rgb within ±1 per channel.
        for (r, g, b) in [
            (59u8, 130u8, 246u8),
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (128, 128, 128),
            (210, 105, 30),
        ] {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h as f64, s as f64, l as f64);
            assert!(
                (r as i32 - r2 as i32).abs() <= 1
                    && (g as i32 - g2 as i32).abs() <= 1
                    && (b as i32 - b2 as i32).abs() <= 1,
                "round-trip ({r},{g},{b}) → ({r2},{g2},{b2}) exceeded ±1"
            );
        }
    }

    /// COLR-V2-01 — `exact_name` returns the keyword for an exact RGB (or None),
    /// and `nearest_name` agrees with it on an exact color (distance 0) while
    /// returning a sensible non-empty keyword for an arbitrary color.
    #[test]
    fn exact_and_nearest_name_behavior() {
        // Exact keyword → name == nearest.
        assert_eq!(exact_name(255, 0, 0), Some("red"));
        assert_eq!(nearest_name(255, 0, 0), "red");
        assert_eq!(exact_name(102, 51, 153), Some("rebeccapurple"));
        assert_eq!(nearest_name(102, 51, 153), "rebeccapurple");

        // Arbitrary color → no exact name, but a non-empty nearest.
        assert_eq!(exact_name(59, 130, 246), None);
        assert!(!nearest_name(59, 130, 246).is_empty());
    }

    /// V5 — malformed `hsl(...)` input is an `Err` (→ exit 1), never a panic or an
    /// index overflow: missing close paren, wrong component count, out-of-range,
    /// non-numeric.
    #[test]
    fn malformed_hsl_is_err() {
        assert!(parse_color("hsl(210, 100%").is_err(), "missing close paren");
        assert!(parse_color("hsl(210, 100%)").is_err(), "too few components");
        assert!(parse_color("hsl(400, 100%, 50%)").is_err(), "hue out of range");
        assert!(parse_color("hsl(210, 200%, 50%)").is_err(), "sat out of range");
        assert!(parse_color("hsl(x, y%, z%)").is_err(), "non-numeric");
    }
}
