//! The `uuid` command: print one or more random RFC-4122 version-4 UUIDs
//! (UUID-01). The lowest-risk Phase-2 slice — no input, no network, just a
//! CSPRNG-backed generator behind the shared [`RunCommand`] trait.
//!
//! Flow (Pattern 1 — thin orchestrator over a pure generator):
//! `run()` loops `count` times, asks the `uuid` crate for a fresh v4 value
//! (`Uuid::new_v4()`, whose `Display` is RFC-4122 lowercase-hyphenated), applies
//! the `--upper` case transform, and `println!`s one UUID per line to stdout.
//! Nothing is read from stdin (uuid takes no input), so `core::input` is unused.

use clap::Args;

use crate::commands::RunCommand;

/// `box uuid [-n N] [--upper]` — generate random v4 UUIDs (UUID-01).
#[derive(Debug, Args)]
pub struct UuidArgs {
    /// Number of UUIDs to generate (one per line).
    #[arg(short = 'n', long = "count", default_value_t = 1)]
    pub count: u8,
    /// Print the UUIDs in uppercase.
    #[arg(long)]
    pub upper: bool,
}

impl RunCommand for UuidArgs {
    fn run(self) -> anyhow::Result<()> {
        for _ in 0..self.count {
            // `Uuid::new_v4()` draws from a CSPRNG (the `v4` feature pulls
            // getrandom); its `Display` impl is RFC-4122 lowercase-hyphenated.
            println!("{}", format_one(uuid::Uuid::new_v4(), self.upper));
        }
        Ok(())
    }
}

/// Render one UUID as the command prints it: lowercase by default (the crate's
/// `Display`), uppercased when `--upper`. Pure + terminal-free so the case
/// transform is unit-testable without spawning the binary.
fn format_one(u: uuid::Uuid, upper: bool) -> String {
    let s = u.to_string();
    if upper {
        s.to_uppercase()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A nil UUID renders lowercase-hyphenated by default and uppercase under
    /// `--upper` — locking the case transform independent of randomness.
    #[test]
    fn format_one_applies_case() {
        let u = uuid::Uuid::nil();
        assert_eq!(format_one(u, false), "00000000-0000-0000-0000-000000000000");
        assert_eq!(format_one(u, true), "00000000-0000-0000-0000-000000000000");
    }

    /// A generated v4 UUID is hyphenated, length 36, and its uppercase form is
    /// the exact uppercase of its default (lowercase) form.
    #[test]
    fn v4_default_is_lower_upper_is_upper() {
        let u = uuid::Uuid::new_v4();
        let lower = format_one(u, false);
        let upper = format_one(u, true);
        assert_eq!(lower.len(), 36);
        assert_eq!(lower, lower.to_lowercase());
        assert_eq!(upper, lower.to_uppercase());
        // Version nibble is `4` (RFC-4122 v4); position 14 in the canonical form.
        assert_eq!(lower.as_bytes()[14], b'4');
    }
}
