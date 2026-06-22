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
        Commands::Uuid(args) => args.run(),
        Commands::Base64(args) => args.run(),
        Commands::Epoch(args) => args.run(),
        Commands::Color(args) => args.run(),
        Commands::Passgen(args) => args.run(),
        Commands::Cowsay(args) => args.run(),
        Commands::Fortune(args) => args.run(),
        Commands::EightBall(args) => args.run(),
        Commands::Roast(args) => args.run(),
        Commands::Hash(args) => args.run(),
        Commands::Tree(args) => args.run(),
        Commands::Du(args) => args.run(),
        Commands::Dupes(args) => args.run(),
        Commands::BulkRename(args) => args.run(),
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
            // Two typed variants are *usage* errors and must surface as exit 2 —
            // same single-owner mapping pattern as the clap parse-error path above:
            //   - `MissingInput`: no arg + interactive TTY (D-04 branch 3).
            //   - `UnsupportedHashLength`: a `box hash --verify <hex>` whose length
            //     matches no algorithm (D-04). A mismatched-but-valid `--verify` is
            //     a plain `anyhow` error → exit 1, NOT this variant (Pitfall 1).
            // All other errors (incl. NotImplemented) keep exit 1. Downcast on the
            // typed variants so a plain `anyhow::bail!` elsewhere is unaffected.
            match e.downcast_ref::<crate::core::errors::BoxError>() {
                Some(
                    crate::core::errors::BoxError::MissingInput
                    | crate::core::errors::BoxError::UnsupportedHashLength { .. },
                ) => ExitCode::from(2),
                _ => ExitCode::from(1),
            }
        }
    }
}
