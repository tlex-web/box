//! The `flatten` anchor command (RED stub — Task 2 implementation follows).

pub mod rename;

use std::path::PathBuf;

use clap::Args;

use crate::commands::RunCommand;

/// `box flatten <src> <out> [--dry-run]` — flatten a folder tree into one
/// directory (FLAT-01..04).
#[derive(Debug, Args)]
pub struct FlattenArgs {
    /// Source directory tree to flatten.
    pub src: PathBuf,
    /// Output directory to copy every file into (created if missing).
    pub out: PathBuf,
    /// Preview the plan without writing anything.
    #[arg(long)]
    pub dry_run: bool,
}

impl RunCommand for FlattenArgs {
    fn run(self) -> anyhow::Result<()> {
        unimplemented!("RED: flatten orchestration not yet implemented")
    }
}
