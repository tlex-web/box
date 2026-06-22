//! `box` entry point: ANSI bootstrap, argument parse, dispatch, and the strict
//! 0/1/2 exit-code mapping every command inherits. No business logic lives here.
//!
//! Exit-code policy:
//! - `--help` / `--version` → clap prints to stdout, exit 0.
//! - bare `box` (no subcommand) → help to **stderr**, exit 2 (D-08 override;
//!   clap's `arg_required_else_help` default is exit 0 — see Pitfall 2 / OQ-1).
//! - bad command / bad args → clap's own exit 2 to stderr (D-07).
//! - command runs OK → exit 0.
//! - command returns `Err` (incl. NotImplemented) → `error: …` to stderr, exit 1.

mod cli;
mod commands;
mod core;

use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::commands::stub::not_implemented;
use crate::commands::RunCommand;

fn main() -> ExitCode {
    // FIRST — set ENABLE_VIRTUAL_TERMINAL_PROCESSING before any output so ANSI
    // escapes render in PowerShell child processes. Ignore failure (non-Windows
    // or already-enabled).
    let _ = enable_ansi_support::enable_ansi_support();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // --help / --version: clap's normal stdout output, exit 0.
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                let _ = e.print();
                return ExitCode::SUCCESS;
            }
            // Bare `box`: clap would print help to stdout and exit 0. D-08
            // requires exit 2; route the help text to stderr ("messages →
            // stderr", OQ-1) and force exit 2.
            if matches!(
                e.kind(),
                ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            ) {
                eprint!("{e}");
                return ExitCode::from(2);
            }
            // Genuine parse errors (bad command, bad/missing args): preserve
            // clap's exit 2 to stderr (D-07). `e.exit()` never returns.
            e.exit();
        }
    };

    // Decide color once, before dispatch, from --no-color ∧ NO_COLOR ∧ TTY
    // (FOUND-04, D-10). Installs a global owo-colors override so all decorations
    // no-op when piped — plain output is byte-identical minus ANSI.
    crate::core::output::init_color(cli.no_color);

    let result = match cli.command {
        Commands::Flatten(args) => args.run(),
        Commands::Uuid => Err(not_implemented("uuid")),
        Commands::Base64 => Err(not_implemented("base64")),
        Commands::Epoch => Err(not_implemented("epoch")),
        Commands::Color => Err(not_implemented("color")),
        Commands::Passgen => Err(not_implemented("passgen")),
        Commands::Cowsay => Err(not_implemented("cowsay")),
        Commands::Fortune => Err(not_implemented("fortune")),
        Commands::EightBall => Err(not_implemented("8ball")),
        Commands::Roast => Err(not_implemented("roast")),
        Commands::Hash => Err(not_implemented("hash")),
        Commands::Tree => Err(not_implemented("tree")),
        Commands::Du => Err(not_implemented("du")),
        Commands::Dupes => Err(not_implemented("dupes")),
        Commands::BulkRename => Err(not_implemented("bulk-rename")),
        Commands::Lolcat => Err(not_implemented("lolcat")),
        Commands::Matrix => Err(not_implemented("matrix")),
        Commands::Ascii => Err(not_implemented("ascii")),
        Commands::Json => Err(not_implemented("json")),
        Commands::Qr => Err(not_implemented("qr")),
        Commands::Clip => Err(not_implemented("clip")),
        Commands::Pomodoro => Err(not_implemented("pomodoro")),
        Commands::Weather => Err(not_implemented("weather")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Lowercase `error:` prefix (matches clap's style), stderr only so
            // stdout stays clean for pipes (FOUND-03, D-06).
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}
