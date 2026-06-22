//! The `flatten` anchor command: recursively copy every file from a source tree
//! into one flat output directory, originals untouched, collisions renamed by
//! encoding the source path — with a `--dry-run` preview that writes nothing.
//!
//! Orchestration (filled in Task 2) lives in [`FlattenArgs::run`]; the pure
//! collision-encoding lives in [`rename`] so its Windows edge cases are unit
//! tested in isolation.

pub mod rename;

use std::path::PathBuf;

use clap::Args;

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
