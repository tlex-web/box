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

    /// A `box cowsay --figure <name>` whose name matches no built-in figure.
    /// `main()` downcasts this variant and maps it to exit code 2 (a usage error) —
    /// the invocation named a figure that does not exist, joining
    /// [`BoxError::MissingInput`] / [`BoxError::UnsupportedHashLength`] in the
    /// `main()` downcast. The message lists the available figures so the user can
    /// correct the call. There is NO external `.cow` file lookup (D-02), so an
    /// unknown name is ALWAYS a usage mistake, never a missing-file runtime error.
    ///
    /// Constructed in `commands::cowsay` and downcast-mapped to exit 2 in `main()`.
    /// Live as of Plan 10-01 (COW-V2-01).
    #[error("unknown figure '{name}'; available figures: {available}")]
    UnknownFigure { name: String, available: String },

    /// A corrupt/unparseable `%APPDATA%\box\config.toml` (or an unknown key under
    /// `deny_unknown_fields`). Aborts BEFORE the operation runs → exit 2 (D-10),
    /// joining [`BoxError::MissingInput`] / [`BoxError::UnsupportedHashLength`] in
    /// the `main()` downcast. A MISSING file is **not** this error — it falls back
    /// to `Config::default()` silently (`core::config::load` matches `NotFound`).
    /// Constructed in `core::config::load` on a `toml::from_str` failure.
    #[error("config error in {path}: {message}")]
    Config { path: String, message: String },

    /// A bare `box weather` with NO positional location AND no `[weather] location`
    /// in config (D-12). `main()` downcasts this variant and maps it to exit code 2
    /// (a usage error), joining [`BoxError::MissingInput`] /
    /// [`BoxError::UnsupportedHashLength`] / [`BoxError::UnknownFigure`] /
    /// [`BoxError::Config`] — the invocation named no place to fetch weather for, so
    /// it is a mistake in HOW the command was called, not a runtime/data failure. The
    /// message points the user at either a positional or the `weather.location`
    /// config key.
    ///
    /// Constructed in `commands::weather::run` and downcast-mapped to exit 2 in
    /// `main()`. Live as of Plan 10-05 (WTHR-V2-01 / D-12).
    #[error("no location: pass a location or set weather.location in config")]
    MissingLocation,

    /// A `box config get/set <key>` USAGE error — a key outside the closed
    /// [`crate::core::config::SETTABLE_KEYS`] registry (D-04), so it can never be
    /// read or written. `main()` downcasts this variant and maps it to exit code 2
    /// (a usage error), joining [`BoxError::Config`] / [`BoxError::MissingLocation`]
    /// in the downcast set. Distinct from [`BoxError::Config`] (a bad-VALUE / bad-
    /// TOML round-trip failure): both exit 2, but this one names the offending key
    /// and lists the settable keys so the user can correct the call. The `message`
    /// carries the full "did you mean" text. NOTHING is written when it fires
    /// (rejected before any atomic write — D-03/D-04).
    ///
    /// Constructed in `core::config::set_value` (unknown-key branch) and
    /// `commands::config` (get unknown-key branch), downcast-mapped to exit 2 in
    /// `main()`. Live as of Plan 11-01 (CFG-01).
    ///
    /// Forward-compat `#[allow(dead_code)]`: constructed via the (not-yet-reachable)
    /// `set_value` unknown-key path in Task 1; the `config` command (Task 2) makes it
    /// reachable and wires the exit-2 downcast, at which point the allow is removed
    /// (allow-then-remove, mirroring `MissingInput`).
    #[allow(dead_code)]
    #[error("{message}")]
    ConfigUsage { message: String },
}
