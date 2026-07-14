//! The `qr` command: render a scannable QR code for arbitrary text/URL using
//! pure Unicode half-block glyphs (QR-01).
//!
//! Engine = `qrcode` 0.14.1 + its built-in [`qrcode::render::unicode::Dense1x2`]
//! renderer, NOT `qr2term` (D-01). `qr2term` ALWAYS emits ANSI color (every cell
//! is `"▄".white().on_black()` with no TTY gate) and encodes polarity in the
//! *background* color, which would write raw escape sequences to a redirected
//! file and break the locked glyph-is-truth + byte-identical-minus-ANSI-when-
//! piped invariant. `Dense1x2` renders pure half-blocks (`▀▄█`/space) with **no
//! ANSI** and returns an owned `String` via `.build()` (its `Pixel::Image =
//! String`), so the output routes through the standard `data → stdout` path and
//! `box qr "x" > qr.txt` yields clean half-block text.
//!
//! Render config (D-02): `.quiet_zone(true)` for the full ISO-18004 4-module
//! border. The error-correction level is `--error-correction L|M|Q|H` (QR-V2-01),
//! default `M` (15% recovery — the qrcode default), applied to BOTH the terminal
//! render and any `--save` file so they never diverge.
//!
//! Monochrome — there is NO color path here (D-03); `qr` never touches
//! `is_color_on()`/owo-colors. Input is acquired via
//! [`crate::core::input::read_input`] (positional arg / `-` stdin / no-arg
//! interactive TTY → `BoxError::MissingInput` exit 2, D-03).
//!
//! ## `--save` PNG/SVG (QR-V2-01 — the one new filesystem write)
//! `--save <file>` writes the QR to an image file instead of printing the glyph
//! block: `.png` (raster, via the qrcode `image` feature — unifies with the
//! present `image 0.25.10`) or `.svg` (vector, via the zero-dep qrcode `svg`
//! feature). The format is inferred from the lowercased extension; any other
//! extension is a clean `bail!` (exit 1) with a `.png`/`.svg` hint, and every
//! write is `.with_context`-wrapped — never a panic (FOUND-05 / T-09-02-SAVE).
//! Under `--save` the terminal glyph block is suppressed and a one-line
//! confirmation is written to stderr. `--save` (the action) and `--json` (the
//! output mode) are ORTHOGONAL: under both, the file is written AND the metadata
//! document carries `saved_path` (Open-Q1).
//!
//! Flow (Pattern 1 — thin orchestrator over a pure helper): `run()` acquires the
//! input string, optionally writes the `--save` file via [`save_qr`], then either
//! emits the metadata document (`--json`) or hands the input to the pure
//! [`render_qr`] seam (encode → `Dense1x2` → owned `String`) and `println!`s it.
//! Oversized input that exceeds the QR capacity is propagated by the `?` on
//! `with_error_correction_level` → clean exit 1, never a panic (FOUND-05 /
//! T-05-QR-DoS).
//!
//! ## Dark-on-light polarity (Pitfall QR-1 — human-verify flip instructions)
//!
//! The #1 reason a terminal QR fails to scan is inverted polarity on a dark
//! theme: if the "dark" modules render as the terminal *background* color,
//! contrast is lost against a dark PowerShell 7 background. `Dense1x2`'s default
//! `default_color()` polarity is verified during the QR-01 phone-scan
//! human-verify checkpoint. If a phone camera will NOT lock or the QR looks
//! inverted (light modules on a dark field), flip the polarity by inserting
//! `.dark_color(Dense1x2::Dark).light_color(Dense1x2::Light)` into the
//! [`render_qr`] chain (between `.quiet_zone(true)` and `.build()`), rebuild, and
//! re-scan. Apply this flip ONLY if the default polarity fails the scan (D-02).

use std::path::Path;
use std::path::PathBuf;

use anyhow::{bail, Context};
use clap::{Args, ValueEnum};
use qrcode::render::unicode::Dense1x2;
use qrcode::{EcLevel, QrCode};

use crate::commands::RunCommand;

/// `box qr [INPUT]` — render a QR code for text or a URL in the terminal (QR-01),
/// or save it to a file (QR-V2-01).
///
/// `INPUT` is encoded as a QR at the `--error-correction` level (default M) and
/// rendered with Unicode half-block glyphs (no ANSI). Omit it to read from piped
/// stdin (`echo "https://example.com" | box qr`). Redirecting stdout
/// (`box qr "x" > qr.txt`) yields clean half-block text with no escape sequences.
/// `--save out.png` / `--save out.svg` writes an image file instead of printing.
#[derive(Debug, Args)]
pub struct QrArgs {
    /// Text or URL to encode; omit to read from piped stdin.
    pub input: Option<String>,

    /// Error-correction level (QR-V2-01): L (~7%) / M (~15%, default) / Q (~25%) /
    /// H (~30%). Applied to BOTH the terminal render and any `--save` file.
    #[arg(long, value_enum, default_value_t = Ec::M)]
    pub error_correction: Ec,

    /// Save the QR to a file instead of printing it (QR-V2-01). The format is
    /// inferred from the extension: `.png` (raster) or `.svg` (vector); any other
    /// extension is an error. Suppresses the terminal glyph block and writes a
    /// one-line confirmation to stderr.
    #[arg(long)]
    pub save: Option<PathBuf>,
}

/// The `--error-correction` levels (QR-V2-01), a `pub` clap `ValueEnum` mirroring
/// the `hash::Algo` style. Maps to the `qrcode` [`EcLevel`]; default stays `M`. The
/// canonical value spellings are the conventional UPPERCASE `L|M|Q|H` (the plan's
/// surface), each with a lowercase alias so `--error-correction h` also parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Ec {
    /// ~7% recovery.
    #[value(name = "L", alias = "l")]
    L,
    /// ~15% recovery (the default).
    #[value(name = "M", alias = "m")]
    M,
    /// ~25% recovery.
    #[value(name = "Q", alias = "q")]
    Q,
    /// ~30% recovery.
    #[value(name = "H", alias = "h")]
    H,
}

impl Ec {
    /// The `qrcode` [`EcLevel`] for this preset.
    fn to_level(self) -> EcLevel {
        match self {
            Ec::L => EcLevel::L,
            Ec::M => EcLevel::M,
            Ec::Q => EcLevel::Q,
            Ec::H => EcLevel::H,
        }
    }

    /// The single-letter label reflected in the `--json` `error_correction` field.
    fn label(self) -> &'static str {
        match self {
            Ec::L => "L",
            Ec::M => "M",
            Ec::Q => "Q",
            Ec::H => "H",
        }
    }
}

/// The `--json` document for `qr` (D-14): encode METADATA, never the glyphs. The
/// rendered ▀▄█ half-block block is a *visual*, not a scriptable value, so the
/// machine document carries only the source `text`, the chosen error-correction
/// level, and (under `--save`) the written path. Scalar → flat object (D-01), no
/// `results`/`count` wrapper.
#[derive(serde::Serialize)]
struct QrOutput {
    /// The source text/URL that was encoded (the useful, scriptable payload).
    text: String,
    /// The QR error-correction level actually used (`"L"`/`"M"`/`"Q"`/`"H"`).
    error_correction: String,
    /// The path written under `--save` (absent otherwise). `--save` and `--json`
    /// are orthogonal (action vs mode): under both, the file is written AND this
    /// field is populated (Open-Q1).
    #[serde(skip_serializing_if = "Option::is_none")]
    saved_path: Option<String>,
}

impl RunCommand for QrArgs {
    fn run(self) -> anyhow::Result<()> {
        // arg → piped stdin → exit-2 on a no-arg interactive TTY (D-03).
        let input = crate::core::input::read_input(self.input)?;
        let ec = self.error_correction.to_level();

        // --save is the ACTION, orthogonal to the --json/human output MODE
        // (Open-Q1): if a path was given, write the file FIRST (regardless of
        // --json), confirm on stderr, and remember it for `saved_path`. The encode
        // here also `?`-propagates a capacity overflow on oversized input → clean
        // exit 1, never a panic (T-05-QR-DoS / T-09-02-QRDOS).
        let saved_path = match &self.save {
            Some(path) => {
                save_qr(&input, ec, path)?;
                eprintln!("Saved QR to {}", path.display());
                Some(path.display().to_string())
            }
            None => None,
        };

        // Fork on --json FIRST (Pitfall 1). Under --json emit METADATA only — do
        // NOT render the glyphs (D-14); `emit_json` also tees the whole document
        // to the clipboard under `--json --clip` (so we do NOT also clip_feed).
        if crate::core::output::is_json_on() {
            let doc = QrOutput {
                text: input.clone(),
                error_correction: self.error_correction.label().to_string(),
                saved_path,
            };
            return crate::core::output::emit_json(&doc);
        }

        // Human path: render the glyph block and print it (data → stdout; no color
        // path, D-03) — UNLESS --save was given, in which case the file is the
        // output and the glyph block is suppressed (avoid noise; Pitfall 3). Keep
        // `println!` here — routing the glyphs through `out_line` would copy the ▀▄
        // block to the clipboard (garbage as text).
        if self.save.is_none() {
            let rendered = render_qr(&input, ec)?;
            println!("{rendered}");
        }
        // D-15: under --clip copy the SOURCE TEXT (the encoded URL/text), NOT the
        // glyphs. `clip_feed` tees `input` without printing; a no-op when --clip
        // is off. This is the one deliberate break from the copy-all rule.
        crate::core::output::clip_feed(&input);
        Ok(())
    }
}

/// Encode `input` at `ec` and write it to `path`, inferring the format from the
/// lowercased extension: `.png` → raster via the qrcode `image` feature, `.svg` →
/// vector via the qrcode `svg` feature (QR-V2-01). Any other (or missing)
/// extension is a clean `bail!` exit 1 with a `.png`/`.svg` hint — never a panic;
/// every write is `.with_context`-wrapped (FOUND-05 / T-09-02-SAVE). The encode's
/// `?` propagates a capacity overflow on oversized input → clean exit 1
/// (T-09-02-QRDOS).
fn save_qr(input: &str, ec: EcLevel, path: &Path) -> anyhow::Result<()> {
    let code = QrCode::with_error_correction_level(input.as_bytes(), ec)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("png") => {
            // image::Luma<u8>'s `Pixel::Image = ImageBuffer`; default module size
            // is 8×8 with a 4-module quiet zone. `.save` infers PNG from the path.
            let img = code.render::<image::Luma<u8>>().quiet_zone(true).build();
            img.save(path)
                .with_context(|| format!("writing {}", path.display()))?;
        }
        Some("svg") => {
            let svg: String = code
                .render::<qrcode::render::svg::Color>()
                .quiet_zone(true)
                .build();
            std::fs::write(path, svg).with_context(|| format!("writing {}", path.display()))?;
        }
        Some(other) => bail!("unsupported --save extension '.{other}'; use .png or .svg"),
        None => bail!("--save path '{}' has no extension; use .png or .svg", path.display()),
    }
    Ok(())
}

/// Encode `input` at error-correction level `ec` and render it to an owned
/// `String` of pure Unicode half-block glyphs (`▀▄█`/space) with a full quiet zone
/// — no ANSI (D-01/D-02). The unit-test seam for the slice; `ec` feeds BOTH this
/// terminal render and the `--save` file so the two never diverge (QR-V2-01).
///
/// Oversized input exceeding the QR capacity is returned as an `Err` via the `?`
/// on `with_error_correction_level` (clean exit 1, never a panic — T-05-QR-DoS).
fn render_qr(input: &str, ec: EcLevel) -> anyhow::Result<String> {
    // `ec` is the chosen `--error-correction` level (default M — the qrcode
    // default, 15% recovery). The `?` propagates a capacity-overflow on oversized
    // input → clean exit 1, never a panic (FOUND-05 / T-05-QR-DoS).
    let code = QrCode::with_error_correction_level(input.as_bytes(), ec)?;
    // Dense1x2's `Pixel::Image = String`, so `.build()` returns an owned String of
    // pure half-block glyphs (`▀▄█`/space) with NO ANSI (D-01). `.quiet_zone(true)`
    // emits the full ISO-18004 4-module border (D-02). To force dark-on-light
    // polarity if a phone-scan fails, insert
    // `.dark_color(Dense1x2::Dark).light_color(Dense1x2::Light)` here (Pitfall QR-1).
    let rendered = code.render::<Dense1x2>().quiet_zone(true).build();
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `render_qr` returns a non-empty rendered String for a fixed input.
    #[test]
    fn render_qr_returns_non_empty_string() {
        let out = render_qr("https://example.com", EcLevel::M).expect("render should succeed");
        assert!(!out.is_empty(), "rendered QR must be non-empty");
    }

    /// `render_qr` output carries NO ANSI escape (the D-01 invariant — the whole
    /// reason for rejecting qr2term). Same assertion shape as json.rs:69.
    #[test]
    fn render_qr_contains_no_ansi() {
        let out = render_qr("https://example.com", EcLevel::M).expect("render should succeed");
        assert!(
            !out.contains('\u{1b}'),
            "rendered QR must contain no ANSI escape: {out:?}"
        );
    }

    /// `render_qr` is deterministic — two calls with the same input+level render
    /// byte-identically (no RNG / no clock in the encode→render path).
    #[test]
    fn render_qr_is_deterministic() {
        let a = render_qr("https://example.com", EcLevel::M).expect("render should succeed");
        let b = render_qr("https://example.com", EcLevel::M).expect("render should succeed");
        assert_eq!(a, b, "render_qr must be deterministic for the same input");
    }

    /// QR-V2-01 — `render_qr` HONORS the EcLevel: the same input at L vs H yields
    /// distinct renders (higher recovery packs more modules → a larger block), so
    /// the level is provably applied, not ignored.
    #[test]
    fn render_qr_honors_ec_level() {
        let input = "https://example.com";
        let l = render_qr(input, EcLevel::L).expect("render L");
        let m = render_qr(input, EcLevel::M).expect("render M");
        let h = render_qr(input, EcLevel::H).expect("render H");
        assert_ne!(l, h, "L and H must produce different renders for the same input");
        assert_ne!(l, m, "L and M must differ");
        assert!(
            h.len() >= l.len(),
            "higher EC (H) must not shrink the module count vs L"
        );
    }

    /// The `Ec` ValueEnum maps to the right `EcLevel` and reflects the right label.
    #[test]
    fn ec_maps_level_and_label() {
        assert_eq!(Ec::L.to_level(), EcLevel::L);
        assert_eq!(Ec::M.to_level(), EcLevel::M);
        assert_eq!(Ec::Q.to_level(), EcLevel::Q);
        assert_eq!(Ec::H.to_level(), EcLevel::H);
        assert_eq!(Ec::L.label(), "L");
        assert_eq!(Ec::M.label(), "M");
        assert_eq!(Ec::Q.label(), "Q");
        assert_eq!(Ec::H.label(), "H");
    }

    /// QR-V2-01 — `save_qr` writes a non-empty PNG whose first bytes are the PNG
    /// magic (`89 50 4E 47`). Uses a `tempfile` scratch dir (never a live path).
    #[test]
    fn save_qr_writes_png_magic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("q.png");
        save_qr("https://example.com", EcLevel::M, &path).expect("save png");
        let bytes = std::fs::read(&path).expect("read png");
        assert!(!bytes.is_empty(), "PNG must be non-empty");
        assert_eq!(
            &bytes[..4],
            &[0x89, b'P', b'N', b'G'],
            "PNG must start with the PNG magic bytes"
        );
    }

    /// QR-V2-01 — `save_qr` writes a non-empty SVG whose text begins with an
    /// `<svg`/`<?xml` marker.
    #[test]
    fn save_qr_writes_svg_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("q.svg");
        save_qr("https://example.com", EcLevel::M, &path).expect("save svg");
        let text = std::fs::read_to_string(&path).expect("read svg");
        assert!(!text.is_empty(), "SVG must be non-empty");
        let head = text.trim_start();
        assert!(
            head.starts_with("<svg") || head.starts_with("<?xml"),
            "SVG must begin with an <svg/<?xml marker: {:?}",
            &head[..head.len().min(32)]
        );
    }

    /// T-09-02-SAVE — an unsupported extension is a clean `bail!` (exit 1) with a
    /// `.png`/`.svg` hint, never a panic. The bad path is never written.
    #[test]
    fn save_qr_rejects_bad_extension() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("q.gif");
        let err = save_qr("hello", EcLevel::M, &path).expect_err("bad extension must error");
        let msg = err.to_string();
        assert!(msg.contains(".png") && msg.contains(".svg"), "hint must name .png/.svg: {msg}");
        assert!(!path.exists(), "the rejected path must not be written");
    }

    /// A path with no extension is rejected the same way (no panic).
    #[test]
    fn save_qr_rejects_missing_extension() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("noext");
        let err = save_qr("hello", EcLevel::M, &path).expect_err("no extension must error");
        assert!(err.to_string().contains(".png"), "hint must name .png/.svg");
    }
}
