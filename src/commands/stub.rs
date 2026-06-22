//! Shared handler for the 22 registered-but-unbuilt commands.
//!
//! Task 2 wires every stub dispatch arm in `main()` to this helper. It returns
//! the typed [`crate::core::errors::BoxError::NotImplemented`] error, which
//! `main()` prints to stderr and maps to exit code 1 (D-06).

/// Build the `NotImplemented` error for an unbuilt command.
pub fn not_implemented(cmd: &'static str) -> anyhow::Error {
    crate::core::errors::BoxError::NotImplemented { cmd }.into()
}
