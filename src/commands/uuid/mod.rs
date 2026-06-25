//! The `uuid` command: print one or more random RFC-4122 version-4 UUIDs
//! (UUID-01). The lowest-risk Phase-2 slice — no input, no network, just a
//! CSPRNG-backed generator behind the shared [`RunCommand`] trait.
//!
//! Flow (Pattern 1 — thin orchestrator over a pure generator):
//! `run()` builds a `Vec<UuidRow>` (one fresh v4 value per `count`, the pure
//! [`format_one`] renderer applying the `--upper` case transform feeding BOTH
//! output paths — no drift), then forks on `is_json_on()` FIRST (Pitfall 1):
//! under `--json` it emits ONE `{results,count}` document via
//! [`crate::core::output::emit_json`]; otherwise it prints one UUID per line via
//! [`crate::core::output::out_line`] (which tees to the clipboard under `--clip`).
//! Nothing is read from stdin (uuid takes no input), so `core::input` is unused.
//!
//! uuid is the FIRST consumer of the wave-1 spine (06-02): the
//! `UuidRow`/`UuidOutput` serde structs + `is_json_on()` fork + `out_line`
//! routing here are the frozen pilot template Phase 7 copies across 21 commands.

use clap::Args;

use crate::commands::RunCommand;

/// One row of `box uuid --json` output (D-03 field names: `uuid`, `version`).
/// `version` is the locked literal `"v4"` (every UUID this command emits is v4).
/// The SAME [`format_one`] renderer fills `uuid` for both the human and JSON
/// paths, so the two can never disagree (no-drift, Pattern 2).
#[derive(serde::Serialize)]
struct UuidRow {
    uuid: String,
    version: &'static str,
}

/// The `box uuid --json` document (D-01/D-02): a `results` array wrapped in an
/// object with a `count`, ALWAYS wrapped even for N=1 (never a bare array, so the
/// shape is Phase-8-compatible). Locked field names: `results`, `count`.
#[derive(serde::Serialize)]
struct UuidOutput {
    results: Vec<UuidRow>,
    count: usize,
}

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
        // Build the rows ONCE — the pure `format_one` renderer (used by the
        // human path too) feeds both branches, so the JSON `uuid` can never
        // differ from the printed line (no-drift, Pattern 2). `Uuid::new_v4()`
        // draws from a CSPRNG (the `v4` feature pulls getrandom); its `Display`
        // impl is RFC-4122 lowercase-hyphenated.
        let rows: Vec<UuidRow> = (0..self.count)
            .map(|_| UuidRow {
                uuid: format_one(uuid::Uuid::new_v4(), self.upper),
                version: "v4",
            })
            .collect();

        // Fork on `is_json_on()` FIRST (Pitfall 1): the ONLY stdout write
        // reachable under `--json` must be `emit_json` — no stray human chrome.
        if crate::core::output::is_json_on() {
            let doc = UuidOutput {
                count: rows.len(),
                results: rows,
            };
            crate::core::output::emit_json(&doc)?;
        } else {
            // Human path: one UUID per line via `out_line` (NOT `println!`), so
            // `--clip` tees every line into the clipboard buffer (D-07).
            for r in &rows {
                crate::core::output::out_line(&r.uuid);
            }
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
