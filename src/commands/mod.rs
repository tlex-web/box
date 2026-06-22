//! Command modules and the shared dispatch trait.
//!
//! Every command's argument struct implements [`RunCommand`]; `main()` matches
//! the parsed [`crate::cli::Commands`] variant and calls `.run()`. Static
//! dispatch — no `Box<dyn>`.

pub mod flatten;
pub mod stub;

/// Implemented by each command's argument struct. `main()` calls `.run()` after
/// parsing; the command owns all of its business logic and returns errors via
/// `anyhow` for `main()` to map to an exit code.
pub trait RunCommand {
    fn run(self) -> anyhow::Result<()>;
}
