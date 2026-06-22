//! Command modules and the shared dispatch trait.
//!
//! Every command's argument struct implements [`RunCommand`]; `main()` matches
//! the parsed [`crate::cli::Commands`] variant and calls `.run()`. Static
//! dispatch — no `Box<dyn>`.

pub mod base64;
pub mod color;
pub mod cowsay;
pub mod du;
pub mod dupes;
pub mod eight_ball;
pub mod epoch;
pub mod flatten;
pub mod fortune;
pub mod hash;
pub mod passgen;
pub mod roast;
pub mod stub;
pub mod tree;
pub mod uuid;

/// Implemented by each command's argument struct. `main()` calls `.run()` after
/// parsing; the command owns all of its business logic and returns errors via
/// `anyhow` for `main()` to map to an exit code.
pub trait RunCommand {
    fn run(self) -> anyhow::Result<()>;
}
