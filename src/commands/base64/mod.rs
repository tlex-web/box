//! The `base64` command: encode or decode base64 text (B64-01). The first live
//! consumer of `core::input::read_input_bytes`, and the end-to-end proof that
//! the Plan-01 exit-2 wiring works (no arg + interactive TTY → exit 2).
//!
//! Flow (Pattern 1 — thin orchestrator over the engine API):
//! `run()` acquires input bytes via [`crate::core::input::read_input_bytes`]
//! (arg → piped stdin → TTY-no-block per D-04), picks the engine
//! (`URL_SAFE_NO_PAD` when `--url-safe`, else `STANDARD`), then:
//! - **encode** (default): `engine.encode(&bytes)` → one line, no wrapping
//!   (B64-01), via `println!`.
//! - **decode** (`--decode`): interpret the bytes as text, `.trim()` trailing
//!   whitespace/CRLF (Pitfall 3 — piped input carries a newline), decode, and
//!   write the raw bytes with `stdout().write_all` so binary output is
//!   byte-exact (NOT through a lossy `String` — T-02-04). A malformed-input
//!   decode error propagates as `anyhow::Error` → exit 1 (FOUND-05, T-02-03);
//!   user input is never `.unwrap()`ed, so there is no panic.

use std::io::Write;

use anyhow::Context;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use clap::Args;

use crate::commands::RunCommand;

/// `box base64 [--decode] [--url-safe] [INPUT]` — encode or decode base64
/// (B64-01). Reads INPUT from the argument or piped stdin (D-04).
#[derive(Debug, Args)]
pub struct Base64Args {
    /// Decode the input instead of encoding it.
    #[arg(long)]
    pub decode: bool,
    /// Use the URL-safe alphabet (`-`/`_`) with no `=` padding.
    #[arg(long = "url-safe")]
    pub url_safe: bool,
    /// Text/bytes to process; omit to read from piped stdin (`-` also means stdin).
    pub input: Option<String>,
}

impl RunCommand for Base64Args {
    fn run(self) -> anyhow::Result<()> {
        // Byte-exact input (binary safe): arg → piped stdin → exit-2 on a no-arg
        // interactive TTY (D-04 branch 3). This is the first live caller of
        // read_input_bytes / the first path that constructs BoxError::MissingInput.
        let bytes = crate::core::input::read_input_bytes(self.input)?;

        if self.decode {
            let decoded = decode(&bytes, self.url_safe)?;
            // Write raw bytes straight to stdout — never through a String — so
            // arbitrary (incl. non-UTF-8) decoded bytes are byte-exact (T-02-04).
            std::io::stdout()
                .write_all(&decoded)
                .context("failed to write decoded output")?;
        } else {
            // Encode never wraps lines (B64-01); one line to stdout.
            println!("{}", encode(&bytes, self.url_safe));
        }
        Ok(())
    }
}

/// Pick the engine for the requested alphabet. `&'static` so callers borrow it.
fn engine(url_safe: bool) -> &'static base64::engine::GeneralPurpose {
    if url_safe {
        &URL_SAFE_NO_PAD
    } else {
        &STANDARD
    }
}

/// Encode bytes to a single base64 string (no line wrapping).
fn encode(bytes: &[u8], url_safe: bool) -> String {
    engine(url_safe).encode(bytes)
}

/// Decode base64 text to raw bytes. The input bytes are interpreted as UTF-8
/// text and trimmed of trailing whitespace/CRLF before decoding (Pitfall 3),
/// so a piped trailing newline does not produce a spurious `InvalidByte` error.
/// A genuinely malformed alphabet surfaces as an `anyhow::Error` (→ exit 1).
fn decode(bytes: &[u8], url_safe: bool) -> anyhow::Result<Vec<u8>> {
    // Lossy is fine here: any non-UTF-8 byte is already not a valid base64
    // alphabet character, so the decode below rejects it cleanly either way.
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    engine(url_safe)
        .decode(trimmed)
        .context("invalid base64 input")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `decode(encode(x)) == x` for a payload including `0x00` and `0xFF`, for
    /// BOTH the standard and url-safe alphabets (byte-exact round-trip, T-02-04).
    #[test]
    fn round_trip_includes_zero_and_ff_both_engines() {
        let payload: Vec<u8> = vec![0x00, 0xFF, b'a', 0x80, 0x7F, 0x00, b'z'];
        for url_safe in [false, true] {
            let enc = encode(&payload, url_safe);
            let dec = decode(enc.as_bytes(), url_safe).unwrap();
            assert_eq!(dec, payload, "round-trip failed (url_safe={url_safe})");
        }
    }

    /// A trailing newline on the encoded input is trimmed before decode, so the
    /// classic "echo adds EOL" footgun (Pitfall 3) does not break the round-trip.
    #[test]
    fn decode_tolerates_trailing_newline() {
        let enc = format!("{}\n", encode(b"hello", false));
        let dec = decode(enc.as_bytes(), false).unwrap();
        assert_eq!(dec, b"hello");
    }

    /// url-safe (NO_PAD) output contains no `+`, `/`, or `=` for a payload that
    /// would force `+`/`/` and padding under STANDARD.
    #[test]
    fn url_safe_uses_url_alphabet_no_pad() {
        let payload: &[u8] = &[0xFB, 0xFF, 0xBF];
        let std_enc = encode(payload, false);
        let url_enc = encode(payload, true);
        assert!(
            std_enc.contains('+') || std_enc.contains('/'),
            "payload should force +// in STANDARD (got {std_enc})"
        );
        assert!(!url_enc.contains('+') && !url_enc.contains('/'));
        assert!(!url_enc.contains('='));
    }

    /// Malformed alphabet input is an `Err` (→ exit 1), not a panic (T-02-03).
    #[test]
    fn decode_garbage_is_err_not_panic() {
        assert!(decode(b"!!!not-base64!!!", false).is_err());
    }
}
