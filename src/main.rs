//! `box` entry point: ANSI bootstrap, argument parse, dispatch, and the strict
//! 0/1/2 exit-code mapping every command inherits. No business logic lives here.
//!
//! Exit-code policy:
//! - `--help` / `--version` → clap prints to stdout, exit 0.
//! - bare `box` (no subcommand) → help to **stderr**, exit 2 (D-08 override;
//!   clap's `arg_required_else_help` default is exit 0 — see Pitfall 2 / OQ-1).
//! - bad command / bad args → clap's own exit 2 to stderr (D-07).
//! - command runs OK → exit 0.
//! - command returns `Err` → `error: …` to stderr, exit 1.
//!
//! The 2-vs-1 boundary is "USAGE error vs RUNTIME/DATA error", NOT "any bad
//! user input" (WR-01 carve-out, D-06):
//! - exit 2 = USAGE: the invocation itself is wrong — a missing required
//!   input/no-arg interactive TTY (`BoxError::MissingInput`), an unknown flag or
//!   bad option value (clap), or a `--verify` length matching no algorithm
//!   (`BoxError::UnsupportedHashLength`). These are mistakes in HOW the command
//!   was called.
//! - exit 1 = RUNTIME/DATA: the invocation was well-formed but the work failed
//!   on the *content* — e.g. `box json` given syntactically invalid JSON exits 1
//!   (the parse error reports the 1-based line/column; pinned by
//!   `tests/json.rs::invalid_json_exits_1_with_line_and_column`). Malformed JSON
//!   is bad *data* the command processed and rejected, not a usage mistake, so
//!   it is deliberately exit 1 — NOT exit 2. Do not "align" it with the usage
//!   variants above; that divergence is intentional (D-06 / JSON-01).

mod cli;
mod commands;
mod core;

use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::Parser;

use crate::cli::{Cli, Commands};
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

    // Spine wiring in the LOAD-BEARING order (Pitfall 7), all folded into a single
    // `result` so config-load errors AND dispatch errors flow through the SAME
    // exit-code downcast below (D-10 — a malformed config must still print the
    // `error:` line, so we never `return ExitCode::from(2)` inline here):
    //   1. init_config FIRST — its Err short-circuits the `and_then` chain and is
    //      routed to exit 2 by the BoxError::Config downcast (config never reaches
    //      `config()` on a malformed file, so dispatch never runs).
    //   2. init_color (unconditional TTY/NO_COLOR decision).
    //   3. init_output LAST so its json||clip force-off overwrites color's decision.
    //   4. dispatch the subcommand.
    //   5. flush_clip ONLY on the Ok dispatch branch (chained via `.and_then`, so a
    //      failed command never clobbers the clipboard — D-08, Pitfall 6).
    let result = crate::core::config::init_config()
        .and_then(|()| {
            crate::core::output::init_color(cli.no_color);
            crate::core::output::init_output(cli.json, cli.clip);
            match cli.command {
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
                Commands::Lolcat(args) => args.run(),
                Commands::Matrix(args) => args.run(),
                Commands::Ascii(args) => args.run(),
                Commands::Json(args) => args.run(),
                Commands::Qr(args) => args.run(),
                Commands::Clip(args) => args.run(),
                Commands::Pomodoro(args) => args.run(),
                Commands::Weather(args) => args.run(),
                Commands::Config(args) => args.run(),
            }
        })
        .and_then(|()| crate::core::output::flush_clip());

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Lowercase `error:` prefix (matches clap's style), stderr only so
            // stdout stays clean for pipes (FOUND-03, D-06).
            eprintln!("error: {e:#}");
            // Four typed variants are *usage*/config errors and must surface as
            // exit 2 — same single-owner mapping pattern as the clap parse-error
            // path above:
            //   - `MissingInput`: no arg + interactive TTY (D-04 branch 3).
            //   - `UnsupportedHashLength`: a `box hash --verify <hex>` whose length
            //     matches no algorithm (D-04). A mismatched-but-valid `--verify` is
            //     a plain `anyhow` error → exit 1, NOT this variant (Pitfall 1).
            //   - `UnknownFigure`: a `box cowsay --figure <name>` naming no built-in
            //     figure (COW-V2-01 / D-02) — a fixed-roster usage mistake.
            //   - `Config`: a malformed/unknown-key `%APPDATA%\box\config.toml`
            //     aborts BEFORE dispatch (D-10). A MISSING file is never this error
            //     (it silently falls back to Config::default()).
            // All other errors keep exit 1. Downcast on the typed variants so a
            // plain `anyhow::bail!` elsewhere is unaffected.
            match e.downcast_ref::<crate::core::errors::BoxError>() {
                Some(
                    crate::core::errors::BoxError::MissingInput
                    | crate::core::errors::BoxError::UnsupportedHashLength { .. }
                    | crate::core::errors::BoxError::UnknownFigure { .. }
                    | crate::core::errors::BoxError::Config { .. }
                    // `MissingLocation`: bare `box weather` with no positional AND no
                    // `[weather] location` config (D-12) — a usage mistake, exit 2.
                    | crate::core::errors::BoxError::MissingLocation
                    // `ConfigUsage`: `box config get/set <unknown-key>` or a bad
                    // `set` value (CFG-01 / D-04) — a usage mistake, exit 2. The
                    // `get` unset-no-default → exit 1 signal is NOT a BoxError (it
                    // `std::process::exit(1)`s directly), so it never reaches here.
                    | crate::core::errors::BoxError::ConfigUsage { .. },
                ) => ExitCode::from(2),
                _ => ExitCode::from(1),
            }
        }
    }
}
