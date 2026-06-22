//! Typed errors for `box`. `main()` maps specific variants to exit codes.

use thiserror::Error;

/// Errors raised by `box` commands that `main()` translates into exit codes.
#[derive(Debug, Error)]
pub enum BoxError {
    /// A registered-but-unbuilt command was invoked. Maps to exit code 1 with
    /// the message printed to stderr (D-06).
    #[error("'{cmd}' is not yet implemented — coming in a future release")]
    NotImplemented { cmd: &'static str },
}
