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
//! border, and `EcLevel::M` (15% recovery — the qrcode default) FIXED for v1 (a
//! configurable EC level / `--save`-to-image is VIS-V2-01, out of scope).
//!
//! Monochrome — there is NO color path here (D-03); `qr` never touches
//! `is_color_on()`/owo-colors. Input is acquired via
//! [`crate::core::input::read_input`] (positional arg / `-` stdin / no-arg
//! interactive TTY → `BoxError::MissingInput` exit 2, D-03).
//!
//! Flow (Pattern 1 — thin orchestrator over a pure helper): `run()` acquires the
//! input string, hands it to the pure [`render_qr`] seam (encode → `Dense1x2` →
//! owned `String`), and `println!`s the result. Oversized input that exceeds the
//! QR capacity is propagated by the `?` on `with_error_correction_level` → clean
//! exit 1, never a panic (FOUND-05 / T-05-QR-DoS).
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

use clap::Args;
use qrcode::render::unicode::Dense1x2;
use qrcode::{EcLevel, QrCode};

use crate::commands::RunCommand;

/// `box qr [INPUT]` — render a QR code for text or a URL in the terminal (QR-01).
///
/// `INPUT` is encoded as a QR at error-correction level M and rendered with
/// Unicode half-block glyphs (no ANSI). Omit it to read from piped stdin
/// (`echo "https://example.com" | box qr`). Redirecting stdout
/// (`box qr "x" > qr.txt`) yields clean half-block text with no escape sequences.
#[derive(Debug, Args)]
pub struct QrArgs {
    /// Text or URL to encode; omit to read from piped stdin.
    pub input: Option<String>,
}

impl RunCommand for QrArgs {
    fn run(self) -> anyhow::Result<()> {
        // arg → piped stdin → exit-2 on a no-arg interactive TTY (D-03).
        let input = crate::core::input::read_input(self.input)?;
        let rendered = render_qr(&input)?;
        // data → stdout; no color path (D-03).
        println!("{rendered}");
        Ok(())
    }
}

/// Encode `input` as a QR code (EcLevel::M) and render it to an owned `String`
/// of pure Unicode half-block glyphs (`▀▄█`/space) with a full quiet zone — no
/// ANSI (D-01/D-02). The unit-test seam for the slice.
///
/// Oversized input exceeding the QR capacity is returned as an `Err` via the `?`
/// on `with_error_correction_level` (clean exit 1, never a panic — T-05-QR-DoS).
fn render_qr(input: &str) -> anyhow::Result<String> {
    // M = the qrcode default EC level (15% recovery), FIXED for v1 (D-02). The `?`
    // propagates a capacity-overflow on oversized input → clean exit 1, never a
    // panic (FOUND-05 / T-05-QR-DoS).
    let code = QrCode::with_error_correction_level(input.as_bytes(), EcLevel::M)?;
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
        let out = render_qr("https://example.com").expect("render should succeed");
        assert!(!out.is_empty(), "rendered QR must be non-empty");
    }

    /// `render_qr` output carries NO ANSI escape (the D-01 invariant — the whole
    /// reason for rejecting qr2term). Same assertion shape as json.rs:69.
    #[test]
    fn render_qr_contains_no_ansi() {
        let out = render_qr("https://example.com").expect("render should succeed");
        assert!(
            !out.contains('\u{1b}'),
            "rendered QR must contain no ANSI escape: {out:?}"
        );
    }

    /// `render_qr` is deterministic — two calls with the same input render byte-
    /// identically (no RNG / no clock in the encode→render path).
    #[test]
    fn render_qr_is_deterministic() {
        let a = render_qr("https://example.com").expect("render should succeed");
        let b = render_qr("https://example.com").expect("render should succeed");
        assert_eq!(a, b, "render_qr must be deterministic for the same input");
    }
}
