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

/// The four wrapping forms a UUID can be printed in (UUID-V2-01). The default is
/// [`Form::Plain`] (the 36-char hyphenated canonical form). All four crate methods
/// (`.hyphenated()`/`.simple()`/`.braced()`/`.urn()`) are `const fn` and need no
/// feature flag. `--upper` composes ON TOP of the chosen form (a post-pass).
#[derive(Clone, Copy)]
enum Form {
    /// Canonical 36-char hyphenated form (default).
    Plain,
    /// `--no-hyphens` — 32 hex digits, no separators.
    Simple,
    /// `--braces` — the hyphenated form wrapped in `{…}`.
    Braces,
    /// `--urn` — the hyphenated form prefixed with `urn:uuid:`.
    Urn,
}

/// One row of `box uuid --json` output (D-03 field names: `uuid`, `version`).
/// `version` is `"v4"` by default or `"v7"` under `--v7` (UUID-V2-01). The SAME
/// [`format_one`] renderer fills `uuid` for both the human and JSON paths, so the
/// two can never disagree (no-drift, Pattern 2).
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

/// `box uuid [-n N] [--upper] [--v7] [--no-hyphens] [--braces|--urn]` — generate
/// random UUIDs (UUID-01 / UUID-V2-01).
#[derive(Debug, Args)]
pub struct UuidArgs {
    /// Number of UUIDs to generate (one per line).
    #[arg(short = 'n', long = "count", default_value_t = 1)]
    pub count: u8,
    /// Print the UUIDs in uppercase.
    #[arg(long)]
    pub upper: bool,
    /// Generate version-7 (time-ordered) UUIDs instead of v4.
    #[arg(long)]
    pub v7: bool,
    /// Print without hyphens (32 hex digits).
    #[arg(long = "no-hyphens")]
    pub no_hyphens: bool,
    /// Wrap each UUID in braces: `{…}`.
    #[arg(long, conflicts_with = "urn")]
    pub braces: bool,
    /// Prefix each UUID with the `urn:uuid:` scheme.
    #[arg(long)]
    pub urn: bool,
}

impl RunCommand for UuidArgs {
    fn run(self) -> anyhow::Result<()> {
        // Select the single wrapping form from the args (clap already rejected
        // `--braces --urn` together with exit 2). `--braces`/`--urn` take
        // precedence over `--no-hyphens` (the crate has no "simple braced"/"simple
        // urn" form); `--upper` is orthogonal and composes with whichever wins.
        let form = if self.braces {
            Form::Braces
        } else if self.urn {
            Form::Urn
        } else if self.no_hyphens {
            Form::Simple
        } else {
            Form::Plain
        };
        let version = if self.v7 { "v7" } else { "v4" };

        // Build the rows ONCE — the pure `format_one` renderer (used by the
        // human path too) feeds both branches, so the JSON `uuid` can never
        // differ from the printed line (no-drift, Pattern 2). Both `new_v4()` and
        // `now_v7()` draw from a CSPRNG (the `v4`/`v7` features pull getrandom).
        let rows: Vec<UuidRow> = (0..self.count)
            .map(|_| {
                let u = if self.v7 {
                    uuid::Uuid::now_v7()
                } else {
                    uuid::Uuid::new_v4()
                };
                UuidRow {
                    uuid: format_one(u, form, self.upper),
                    version,
                }
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

/// Render one UUID as the command prints it: the chosen wrapping [`Form`] first
/// (Plain/Simple/Braces/Urn — all `const fn`, no feature), THEN `--upper` as a
/// composable post-pass (uppercases the whole result). Pure + terminal-free so
/// every form + the case transform are unit-testable without spawning the binary.
fn format_one(u: uuid::Uuid, form: Form, upper: bool) -> String {
    let s = match form {
        Form::Plain => u.hyphenated().to_string(),
        Form::Simple => u.simple().to_string(),
        Form::Braces => u.braced().to_string(),
        Form::Urn => u.urn().to_string(),
    };
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
        assert_eq!(
            format_one(u, Form::Plain, false),
            "00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(
            format_one(u, Form::Plain, true),
            "00000000-0000-0000-0000-000000000000"
        );
    }

    /// A generated v4 UUID is hyphenated, length 36, and its uppercase form is
    /// the exact uppercase of its default (lowercase) form.
    #[test]
    fn v4_default_is_lower_upper_is_upper() {
        let u = uuid::Uuid::new_v4();
        let lower = format_one(u, Form::Plain, false);
        let upper = format_one(u, Form::Plain, true);
        assert_eq!(lower.len(), 36);
        assert_eq!(lower, lower.to_lowercase());
        assert_eq!(upper, lower.to_uppercase());
        // Version nibble is `4` (RFC-4122 v4); position 14 in the canonical form.
        assert_eq!(lower.as_bytes()[14], b'4');
    }

    /// Each wrapping [`Form`] has the expected shape, and `--upper` composes with
    /// every one (uppercasing the whole rendered result). Locked against a fixed
    /// nil UUID so the shapes are independent of randomness.
    #[test]
    fn format_one_each_form_shape_and_upper_composes() {
        let u = uuid::Uuid::nil();
        // Plain: 36-char hyphenated.
        let plain = format_one(u, Form::Plain, false);
        assert_eq!(plain.len(), 36);
        assert!(plain.contains('-'));
        // Simple: 32 hex digits, no hyphens.
        let simple = format_one(u, Form::Simple, false);
        assert_eq!(simple.len(), 32);
        assert!(!simple.contains('-'));
        // Braces: the 36-char form wrapped in `{…}` (38 chars total).
        let braces = format_one(u, Form::Braces, false);
        assert!(braces.starts_with('{') && braces.ends_with('}'));
        assert_eq!(braces.len(), 38);
        // Urn: the `urn:uuid:` prefix + the 36-char form.
        let urn = format_one(u, Form::Urn, false);
        assert!(urn.starts_with("urn:uuid:"));
        assert_eq!(urn.len(), "urn:uuid:".len() + 36);

        // `--upper` uppercases the rendered result of every form. Use a random
        // UUID so there are hex letters to uppercase.
        let r = uuid::Uuid::new_v4();
        for form in [Form::Plain, Form::Simple, Form::Braces, Form::Urn] {
            let lower = format_one(r, form, false);
            let upper = format_one(r, form, true);
            assert_eq!(upper, lower.to_uppercase(), "--upper must compose");
        }
    }

    /// A `--v7` UUID carries the version-7 nibble (`'7'` at canonical position 14)
    /// and `now_v7()` is reachable (the `v7` feature is enabled).
    #[test]
    fn v7_has_version_nibble_seven() {
        let u = uuid::Uuid::now_v7();
        let s = format_one(u, Form::Plain, false);
        // Canonical position 14 is the version nibble (8-4-`V`...).
        assert_eq!(s.as_bytes()[14], b'7', "v7 version nibble must be '7': {s}");
    }
}
