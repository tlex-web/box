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
#[command(
    name = "box",
    version,
    about = "A Rust CLI toolbox",
    arg_required_else_help = true
)]
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
    Flatten(crate::commands::flatten::FlattenArgs),
    /// Generate a random UUID
    Uuid(crate::commands::uuid::UuidArgs),
    /// Encode or decode base64 text
    Base64(crate::commands::base64::Base64Args),
    /// Convert between Unix timestamps and human-readable dates
    Epoch(crate::commands::epoch::EpochArgs),
    /// Convert colors between hex and RGB formats
    Color(crate::commands::color::ColorArgs),
    /// Generate secure passwords and passphrases
    Passgen(crate::commands::passgen::PassgenArgs),
    /// Wrap text in an ASCII-art speech bubble
    Cowsay(crate::commands::cowsay::CowsayArgs),
    /// Print a random fortune or quote
    Fortune(crate::commands::fortune::FortuneArgs),
    /// Ask the magic 8-ball a question
    #[command(name = "8ball")]
    EightBall(crate::commands::eight_ball::EightBallArgs),
    /// Deliver a random programmer roast
    Roast(crate::commands::roast::RoastArgs),
    /// Compute and verify file hashes
    Hash(crate::commands::hash::HashArgs),
    /// Print a directory tree with file sizes
    Tree(crate::commands::tree::TreeArgs),
    /// Analyze disk usage for a directory
    Du(crate::commands::du::DuArgs),
    /// Find duplicate files by content
    Dupes(crate::commands::dupes::DupesArgs),
    /// Bulk-rename files with a regex and dry-run preview
    #[command(name = "bulk-rename")]
    BulkRename(crate::commands::bulk_rename::BulkRenameArgs),
    /// Rainbow-colorize piped text
    Lolcat(crate::commands::lolcat::LolcatArgs),
    /// Run the Matrix digital-rain screensaver effect
    ///
    /// A full-terminal green halfwidth-katakana "digital rain" at ~20 FPS. Press
    /// Ctrl+C, q, or Esc to exit — the cursor and terminal are restored cleanly
    /// with no leftover artifacts.
    ///
    /// FONT NOTE: the rain uses halfwidth katakana (U+FF66–U+FF9D), which need a
    /// CJK-capable font. On the bare default PowerShell 7 font (Cascadia Mono)
    /// the glyphs show as tofu boxes — this is a known cosmetic font limitation,
    /// not a bug. For authentic glyphs, switch to a CJK-capable font such as
    /// Cascadia Next JP.
    Matrix(crate::commands::matrix::MatrixArgs),
    /// Render an image as ASCII art
    Ascii(crate::commands::ascii::AsciiArgs),
    /// Pretty-print and validate JSON
    Json(crate::commands::json::JsonArgs),
    /// Render a QR code for text or a URL in the terminal
    Qr(crate::commands::qr::QrArgs),
    /// Read from or write to the system clipboard
    ///
    /// With no flag, `box clip` copies piped stdin to the Windows clipboard
    /// (`"text" | box clip`); with `--paste` it reads the clipboard to stdout.
    /// On copy a single trailing newline is stripped (the implicit CRLF
    /// PowerShell appends when piping a string) so no spurious blank line lands
    /// on the clipboard; on paste the text is emitted byte-exact.
    Clip(crate::commands::clip::ClipArgs),
    /// Run a focus timer with Windows toast notifications
    ///
    /// `box pomodoro` runs a blocking in-place `MM:SS` countdown: 25 minutes by
    /// default, `--break` for 5, `--long-break` for 15, or a positional `[MINUTES]`
    /// to override. Press Ctrl+C, q, or Esc to cancel — "cancelled" prints to
    /// stderr, the command exits 1, and NO toast fires. On completion a Windows
    /// toast notification appears and the command exits 0; the cursor and terminal
    /// are restored on every exit path.
    Pomodoro(crate::commands::pomodoro::PomodoroArgs),
    /// Fetch the weather for a location
    Weather,
}
