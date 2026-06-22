//! Shared stdin/arg input reader for `box` commands (D-04/D-05/D-06).
//!
//! Every stdin-consuming Phase-2 command (base64, cowsay, epoch, color) and the
//! later file commands (hash, json, clip, lolcat) acquire their input through
//! one of two readers here, so the arg-vs-stdin-vs-TTY precedence and the
//! no-block guard live in exactly one place.
//!
//! Precedence (D-04), evaluated by [`resolve`] / [`resolve_bytes`]:
//! 1. positional `arg` is `Some(s)` and `s != "-"` → use it directly; stdin is
//!    never touched.
//! 2. `arg` is `None` or `Some("-")` **and** stdin is **not** an interactive TTY
//!    (i.e. piped/redirected) → read stdin to EOF.
//! 3. `arg` is `None`/`"-"` **and** stdin **is** an interactive TTY → return
//!    [`BoxError::MissingInput`] (→ exit 2 in `main()`) instead of blocking on a
//!    read that would hang the terminal.
//!
//! [`read_input`] returns a UTF-8 `String` (cowsay/epoch/color). [`read_input_bytes`]
//! returns a `Vec<u8>` via `read_to_end` with **no** UTF-8 validation, so binary
//! input (base64) is byte-exact even for non-UTF-8 bytes (D-05/D-06).
//!
//! The TTY decision is threaded through the inner `resolve*` helpers as an
//! `is_tty` bool and the stdin source as a `Read`, so branch 3 (and the
//! piped-bytes path) are unit-testable without a real terminal (RESEARCH:540-543).
//! A future `--file PATH` layer (D-06, Phase 3) slots in ahead of the stdin
//! branch inside these helpers without reshaping the public signatures.

use std::io::{IsTerminal, Read};

use anyhow::Context;

use crate::core::errors::BoxError;

/// Read UTF-8 text input for a command, following the D-04 precedence.
///
/// Use for text-oriented commands (cowsay, epoch, color). Reads piped stdin as a
/// UTF-8 `String`; an interactive TTY with no argument yields
/// [`BoxError::MissingInput`] (exit 2) rather than blocking.
//
// Still awaits its first live caller (the Wave-2 text commands
// cowsay/epoch/color); the byte sibling `read_input_bytes` went live with base64
// (Plan 02-02), so the forward-compat allow on the *byte* path came off. This
// String reader keeps a scoped allow until cowsay/epoch/color consume it.
#[allow(dead_code)]
pub fn read_input(arg: Option<String>) -> anyhow::Result<String> {
    let stdin = std::io::stdin();
    resolve(arg, stdin.is_terminal(), stdin.lock())
}

/// Read binary-exact input for a command, following the D-04 precedence.
///
/// Use for byte-oriented commands (base64). Reads piped stdin via `read_to_end`
/// into a `Vec<u8>` with no UTF-8 validation, so arbitrary bytes round-trip
/// unchanged (D-05/D-06). An interactive TTY with no argument yields
/// [`BoxError::MissingInput`] (exit 2) rather than blocking.
//
// Live as of Plan 02-02: `base64` is the first caller, so the Phase-1-style
// forward-compat `#[allow(dead_code)]` has been removed here (and on
// `resolve_bytes` + `BoxError::MissingInput`), restoring the strict dead-code
// gate on the byte path (mirrors STATE.md [01-03] allow-then-remove).
pub fn read_input_bytes(arg: Option<String>) -> anyhow::Result<Vec<u8>> {
    let stdin = std::io::stdin();
    resolve_bytes(arg, stdin.is_terminal(), stdin.lock())
}

/// Inner resolver for [`read_input`] — `is_tty` and the reader are injected so the
/// three branches are unit-testable without a real terminal.
// Reachable only from the (still caller-less) public `read_input` in the bin
// build, and from unit tests; the scoped allow stays paired with `read_input`
// until cowsay/epoch/color make the String path live.
#[allow(dead_code)]
fn resolve<R: Read>(arg: Option<String>, is_tty: bool, mut reader: R) -> anyhow::Result<String> {
    match arg.as_deref() {
        // Branch 1: an explicit argument that is not the stdin sentinel "-".
        Some(s) if s != "-" => Ok(s.to_string()),
        // Branch 3: no usable arg AND interactive TTY → do not block (D-04).
        // Returned as the typed variant via `.into()` (not a plain anyhow macro),
        // so `main()` can downcast to BoxError::MissingInput and map it to exit 2
        // (RESEARCH Pitfall 2). A type-erased anyhow error would lose the variant.
        _ if is_tty => Err(BoxError::MissingInput.into()),
        // Branch 2: no usable arg AND piped → read stdin to EOF as UTF-8.
        _ => {
            let mut buf = String::new();
            reader
                .read_to_string(&mut buf)
                .context("failed to read input from stdin")?;
            Ok(buf)
        }
    }
}

/// Inner resolver for [`read_input_bytes`] — mirrors [`resolve`] but reads bytes
/// via `read_to_end` (no UTF-8 validation) so binary input is byte-exact.
// Live via `read_input_bytes` (base64, Plan 02-02); allow removed.
fn resolve_bytes<R: Read>(
    arg: Option<String>,
    is_tty: bool,
    mut reader: R,
) -> anyhow::Result<Vec<u8>> {
    match arg.as_deref() {
        Some(s) if s != "-" => Ok(s.as_bytes().to_vec()),
        _ if is_tty => Err(BoxError::MissingInput.into()),
        _ => {
            let mut buf = Vec::new();
            reader
                .read_to_end(&mut buf)
                .context("failed to read input from stdin")?;
            Ok(buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Branch 1: an explicit arg is returned verbatim and stdin is never read
    /// (we pass an empty reader; if it were touched the result would be empty).
    #[test]
    fn arg_branch_returns_arg_verbatim() {
        let out = resolve(Some("hello".to_string()), false, &b""[..]).unwrap();
        assert_eq!(out, "hello");
    }

    /// Branch 3: no arg on an interactive TTY returns an error that downcasts to
    /// `BoxError::MissingInput` — proving the exit-2 path in `main()` is reachable
    /// and that we returned the typed variant (a type-erased anyhow error created
    /// by a plain macro would not downcast).
    #[test]
    fn missing_input_on_tty_downcasts_to_box_error() {
        let err = resolve(None, true, &b""[..]).unwrap_err();
        assert!(
            matches!(err.downcast_ref::<BoxError>(), Some(BoxError::MissingInput)),
            "expected BoxError::MissingInput, got: {err:?}"
        );
    }

    /// Branch 2 (bytes): piped input is read byte-exactly via `read_to_end`,
    /// including a non-UTF-8 byte (0xFF) — proving we use `read_to_end`, not
    /// `read_to_string` (which would reject/mangle the byte).
    #[test]
    fn piped_bytes_are_byte_exact_including_non_utf8() {
        let input: &[u8] = &[0x00, 0xFF, b'a', 0x80, b'\n'];
        let out = resolve_bytes(None, false, input).unwrap();
        assert_eq!(out, input);
    }

    /// The "-" sentinel with no real arg behaves like branch 2 (read stdin) when
    /// piped — confirming "-" is treated as "read from stdin", not as literal text.
    #[test]
    fn dash_sentinel_reads_piped_stdin() {
        let out = resolve(Some("-".to_string()), false, &b"piped"[..]).unwrap();
        assert_eq!(out, "piped");
    }
}
