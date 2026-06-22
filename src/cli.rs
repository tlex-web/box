//! Command registry. The single source of truth for every `box` subcommand.
//!
//! Each [`Commands`] variant's `///` doc comment becomes its `box --help`
//! one-liner and its `box <cmd> --help` long help (D-05). Identifiers that
//! cannot be Rust idents (`8ball`, `bulk-rename`) keep their CLI name via
//! `#[command(name = "...")]`.
//!
//! This phase registers all 23 commands; only `flatten` becomes functional, in
//! plan 03 — which swaps the unit `Flatten` variant for one carrying the real
//! `FlattenArgs`. Until then every variant is routed to the stub handler.

use clap::{Parser, Subcommand};

/// Top-level CLI parser.
#[derive(Parser)]
#[command(name = "box", version, about = "A Rust CLI toolbox", arg_required_else_help = true)]
pub struct Cli {
    /// Disable colored output (also honors NO_COLOR and non-TTY stdout)
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Every registered subcommand. All 23 are listed in `box --help`; only the
/// built ones do real work — the rest dispatch to the NotImplemented stub.
#[derive(Subcommand)]
pub enum Commands {
    /// Flatten a folder tree into a single output directory
    Flatten,
    /// Generate a random UUID
    Uuid,
    /// Encode or decode base64 text
    Base64,
    /// Convert between Unix timestamps and human-readable dates
    Epoch,
    /// Convert colors between hex and RGB formats
    Color,
    /// Generate secure passwords and passphrases
    Passgen,
    /// Wrap text in an ASCII-art speech bubble
    Cowsay,
    /// Print a random fortune or quote
    Fortune,
    /// Ask the magic 8-ball a question
    #[command(name = "8ball")]
    EightBall,
    /// Deliver a random programmer roast
    Roast,
    /// Compute and verify file hashes
    Hash,
    /// Print a directory tree with file sizes
    Tree,
    /// Analyze disk usage for a directory
    Du,
    /// Find duplicate files by content
    Dupes,
    /// Bulk-rename files with a regex and dry-run preview
    #[command(name = "bulk-rename")]
    BulkRename,
    /// Rainbow-colorize piped text
    Lolcat,
    /// Run the Matrix digital-rain screensaver effect
    Matrix,
    /// Render an image as ASCII art
    Ascii,
    /// Pretty-print and validate JSON
    Json,
    /// Render a QR code for text or a URL in the terminal
    Qr,
    /// Read from or write to the system clipboard
    Clip,
    /// Run a focus timer with Windows toast notifications
    Pomodoro,
    /// Fetch the weather for a location
    Weather,
}
