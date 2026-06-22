//! Typed errors for `box`. `main()` maps specific variants to exit codes.

use thiserror::Error;

/// Errors raised by `box` commands that `main()` translates into exit codes.
#[derive(Debug, Error)]
pub enum BoxError {
    /// A registered-but-unbuilt command was invoked. Maps to exit code 1 with
    /// the message printed to stderr (D-06).
    #[error("'{cmd}' is not yet implemented — coming in a future release")]
    NotImplemented { cmd: &'static str },

    /// No input was provided: a stdin-consuming command was run with no
    /// positional argument on an interactive TTY (D-04 branch 3). `main()`
    /// downcasts this variant and maps it to exit code 2 (a usage error), never
    /// blocking on a `read_to_string` that would hang the terminal.
    ///
    /// `main()` reads this variant (downcast in the `Err` arm) but the only
    /// *constructor* is `core::input`, whose public readers have no live caller
    /// until the Wave-2 command plans (base64/cowsay/epoch/color). Until then the
    /// non-test bin build sees the construction site as dead. Mirrors the Phase-1
    /// forward-compat pattern (STATE.md [01-03]): the allow comes off once the
    /// first command becomes a live caller of `core::input`.
    #[allow(dead_code)]
    #[error("no input: pass an argument or pipe data")]
    MissingInput,
}
