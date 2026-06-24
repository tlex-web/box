//! Typed errors for `box`. `main()` maps specific variants to exit codes.

use thiserror::Error;

/// Errors raised by `box` commands that `main()` translates into exit codes.
#[derive(Debug, Error)]
pub enum BoxError {
    /// No input was provided: a stdin-consuming command was run with no
    /// positional argument on an interactive TTY (D-04 branch 3). `main()`
    /// downcasts this variant and maps it to exit code 2 (a usage error), never
    /// blocking on a `read_to_string` that would hang the terminal.
    ///
    /// Constructed in `core::input` and downcast-mapped to exit 2 in `main()`.
    /// Live as of Plan 02-02: `base64` is the first command that reaches
    /// `read_input_bytes` with no arg on an interactive TTY, so the variant now
    /// has a live constructor and the forward-compat `#[allow(dead_code)]` has
    /// been removed (allow-then-remove, mirroring STATE.md [01-03]).
    #[error("no input: pass an argument or pipe data")]
    MissingInput,

    /// A `box hash --verify <hex>` value whose length matches no supported
    /// algorithm (only 32→md5, 64→sha256, 128→sha512 auto-detect). `main()`
    /// downcasts this variant and maps it to exit code 2 (a usage error), same
    /// as [`BoxError::MissingInput`] — a mismatched-but-well-formed `--verify`
    /// stays a plain exit-1 error, NOT this variant (D-04, RESEARCH Pitfall 1).
    ///
    /// Constructed in `commands::hash` and downcast-mapped to exit 2 in `main()`.
    /// Live as of Plan 03-01.
    #[error("unsupported --verify hash length: {len} (expected 32/64/128 hex)")]
    UnsupportedHashLength { len: usize },
}
