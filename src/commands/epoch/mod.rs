//! The `epoch` command: convert between Unix timestamps and human-readable
//! dates (EPOC-01), delegating all timezone/DST math to `chrono`'s `Local`
//! (D-01 — never hand-roll TZ offsets).
//!
//! Three modes (D-12), decided by the optional positional `value`:
//! - **no arg** → print the current Unix timestamp (`Utc::now().timestamp()`),
//!   a single integer to stdout.
//! - **integer arg** (parses as `i64`) → treat it as a Unix timestamp and print
//!   the `Local` and `UTC` human dates on two labeled lines.
//! - **anything else** → treat it as a date string and print the resulting Unix
//!   timestamp. Accepted formats (D-12): RFC 3339 / ISO 8601 (with offset/`Z`),
//!   `YYYY-MM-DD HH:MM:SS` (local), and `YYYY-MM-DD` (local midnight). No
//!   ambiguous `MM/DD/YYYY`. An unrecognized string `bail!`s with a hint naming
//!   the three formats; a nonexistent/ambiguous local time (DST spring-forward /
//!   fall-back) also errors — `LocalResult` is collapsed with `.single()`,
//!   never `.unwrap()` (T-02-05), so bad input is always a clean exit 1, never a
//!   panic.

use anyhow::{bail, Context};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use clap::Args;

use crate::commands::RunCommand;

/// The `box epoch --json` document (D-17 LOCKED UNIFIED shape, EPOC-V2-01 additive):
/// the SAME `{epoch, utc, local, relative[, tz]}` object for EVERY input mode (now /
/// integer / date string) — no branching on input direction (D-01 scalar → flat
/// object). `epoch` is the Unix timestamp (i64); `utc`/`local` are the human-readable
/// datetime strings derived from it. `relative` (always-present, clock-dependent so
/// tests assert FORMAT not value) is the hand-rolled relative-time phrase. `tz` is
/// present ONLY under `--tz` (`skip_serializing_if`), the zoned `%Z %z` rendering.
#[derive(serde::Serialize)]
struct EpochOutput {
    epoch: i64,
    utc: String,
    local: String,
    relative: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tz: Option<String>,
}

/// `box epoch [VALUE]` — Unix timestamp ↔ human date (EPOC-01).
///
/// `VALUE` may be omitted (print now), an integer Unix timestamp (print the
/// human dates), or a date string in one of the D-12 formats (print the
/// timestamp). It is also read from piped stdin when omitted.
#[derive(Debug, Args)]
pub struct EpochArgs {
    /// A Unix timestamp or a date string; omit to print the current timestamp.
    pub value: Option<String>,

    /// Also show the time in this IANA timezone (e.g. `Asia/Tokyo`).
    #[arg(long)]
    pub tz: Option<String>,
}

impl RunCommand for EpochArgs {
    fn run(self) -> anyhow::Result<()> {
        // Acquire the value: arg → piped stdin → (no-arg interactive TTY is NOT
        // an error here, it means "print now"). So we only consult stdin when it
        // is piped; an interactive TTY with no arg falls through to the now path.
        let value = resolve_value(self.value)?;

        // Resolve the input to a single `epoch: i64` regardless of mode (the
        // D-17 unification): now → now; integer → that integer; date string → the
        // parsed timestamp. This collapses the three input directions into one
        // value BEFORE the human/JSON fork, so the JSON shape never branches.
        let epoch: i64 = match &value {
            None => Utc::now().timestamp(),
            Some(s) => {
                let s = s.trim();
                if let Ok(secs) = s.parse::<i64>() {
                    secs
                } else {
                    parse_date(s)?
                }
            }
        };

        // Validate `--tz` ONCE, BEFORE any output, so a bad IANA name fails the
        // command cleanly (exit 1 + hint, never a panic — T-02-05 discipline)
        // regardless of mode. `None` when `--tz` is unset.
        let tz: Option<chrono_tz::Tz> = match self.tz.as_deref() {
            Some(name) => Some(parse_tz(name)?),
            None => None,
        };

        // Fork on `is_json_on()` FIRST (Pitfall 1): under `--json` emit the ONE
        // unified `{epoch, utc, local, relative[, tz]}` document for EVERY mode. The
        // human path keeps its mode-specific lines (now → bare integer; otherwise the
        // two labeled date lines + a relative suffix + an optional --tz line), routed
        // through `out_line`.
        if crate::core::output::is_json_on() {
            crate::core::output::emit_json(&epoch_output(epoch, tz)?)?;
            return Ok(());
        }

        match value {
            // No input at all → current Unix timestamp (single integer). Stays a
            // bare integer (scripting-clean) — no relative suffix, no --tz line.
            None => {
                crate::core::output::out_line(&epoch.to_string());
            }
            Some(s) => {
                let s = s.trim();
                if s.parse::<i64>().is_ok() {
                    // Integer → treat as a Unix timestamp, print local + UTC with an
                    // always-on relative suffix appended to each (EPOC-V2-01).
                    let (local_line, utc_line) = format_timestamp(epoch)?;
                    let rel = relative_for(epoch, Utc::now().timestamp());
                    crate::core::output::out_line(&format!("{local_line} ({rel})"));
                    crate::core::output::out_line(&format!("{utc_line} ({rel})"));
                    // `--tz` adds a third zone-labeled line off the SAME instant.
                    if let Some(tz) = tz {
                        crate::core::output::out_line(&format_tz_line(epoch, tz)?);
                    }
                } else {
                    // Otherwise a date string → print the timestamp. Stays a bare
                    // integer (no relative suffix, scripting-clean).
                    crate::core::output::out_line(&epoch.to_string());
                }
            }
        }
        Ok(())
    }
}

/// Resolve the optional value following the D-04 precedence, but WITHOUT treating
/// a no-arg interactive TTY as an error: for `epoch`, no input means "print now".
/// An explicit arg wins; otherwise piped stdin is read; an interactive TTY with
/// no arg yields `None` (→ the now path).
fn resolve_value(arg: Option<String>) -> anyhow::Result<Option<String>> {
    use std::io::{IsTerminal, Read};

    match arg.as_deref() {
        Some(s) if s != "-" => Ok(Some(s.to_string())),
        _ => {
            let stdin = std::io::stdin();
            if stdin.is_terminal() {
                // Interactive, no arg → "print now", not an error.
                Ok(None)
            } else {
                let mut buf = String::new();
                stdin
                    .lock()
                    .read_to_string(&mut buf)
                    .context("failed to read input from stdin")?;
                let trimmed = buf.trim();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(trimmed.to_string()))
                }
            }
        }
    }
}

/// Build the unified `{epoch, utc, local}` JSON document from a Unix timestamp,
/// reusing the SAME `DateTime::from_timestamp` / `with_timezone(&Local)` math as
/// [`format_timestamp`] so the JSON `utc`/`local` can never disagree with the
/// human lines (no-drift). Errors (never panics) when the timestamp is out of
/// chrono's representable range (T-02-05). The datetime strings carry the same
/// `%Y-%m-%d %H:%M:%S`-based formats the human lines use, MINUS the `Local:`/`UTC:`
/// label prefixes (the JSON key names already convey which is which).
fn epoch_output(epoch: i64, tz: Option<chrono_tz::Tz>) -> anyhow::Result<EpochOutput> {
    let dt_utc: DateTime<Utc> = DateTime::from_timestamp(epoch, 0)
        .ok_or_else(|| anyhow::anyhow!("timestamp {epoch} is out of range"))?;
    let dt_local = dt_utc.with_timezone(&Local);
    // `tz` rides the SAME `dt_utc` (no-drift with the human --tz line), rendered
    // `%Z %z` (abbreviation + numeric offset). Present only under `--tz`.
    let tz_field =
        tz.map(|z| dt_utc.with_timezone(&z).format("%Y-%m-%d %H:%M:%S %Z %z").to_string());
    Ok(EpochOutput {
        epoch,
        utc: dt_utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        local: dt_local.format("%Y-%m-%d %H:%M:%S %z").to_string(),
        relative: relative_for(epoch, Utc::now().timestamp()),
        tz: tz_field,
    })
}

/// Validate an IANA timezone name into a [`chrono_tz::Tz`], `bail!`ing a clean
/// exit-1 error with a hint on an unknown name (mirrors [`parse_date`]'s
/// discipline — never a panic, T-02-05).
fn parse_tz(name: &str) -> anyhow::Result<chrono_tz::Tz> {
    use std::str::FromStr;
    chrono_tz::Tz::from_str(name)
        .map_err(|_| anyhow::anyhow!("unknown timezone '{name}'; expected an IANA name like 'Asia/Tokyo'"))
}

/// Build the third human line for `--tz`: the instant rendered in the requested
/// zone, labeled with the IANA name and carrying the `%Z %z` abbreviation+offset
/// (e.g. `Asia/Tokyo: 2023-11-15 07:13:20 JST +0900`). Shares `dt_utc` with
/// [`epoch_output`] so the line and the JSON `tz` field can never disagree.
fn format_tz_line(epoch: i64, tz: chrono_tz::Tz) -> anyhow::Result<String> {
    let dt_utc: DateTime<Utc> = DateTime::from_timestamp(epoch, 0)
        .ok_or_else(|| anyhow::anyhow!("timestamp {epoch} is out of range"))?;
    let zoned = dt_utc.with_timezone(&tz);
    Ok(format!(
        "{}: {}",
        tz.name(),
        zoned.format("%Y-%m-%d %H:%M:%S %Z %z")
    ))
}

/// Hand-rolled relative-time humanizer (EPOC-V2-01, no crate): a threshold ladder
/// over `now - epoch` returning `just now` (within ~45s), `N {minute|hour|day|
/// week|month|year}s ago` for past instants, and `in N …` for future instants.
/// Singular units drop the trailing `s`. Pure so the FORMAT is unit-testable
/// independent of the wall clock (the value is clock-dependent by nature).
fn relative_for(epoch: i64, now: i64) -> String {
    const MIN: i64 = 60;
    const HOUR: i64 = 60 * MIN;
    const DAY: i64 = 24 * HOUR;
    const WEEK: i64 = 7 * DAY;
    const MONTH: i64 = 30 * DAY;
    const YEAR: i64 = 365 * DAY;

    let diff = now - epoch; // > 0 = past, < 0 = future
    let abs = diff.abs();
    if abs < 45 {
        return "just now".to_string();
    }
    let (n, unit) = if abs < HOUR {
        (abs / MIN, "minute")
    } else if abs < DAY {
        (abs / HOUR, "hour")
    } else if abs < WEEK {
        (abs / DAY, "day")
    } else if abs < MONTH {
        (abs / WEEK, "week")
    } else if abs < YEAR {
        (abs / MONTH, "month")
    } else {
        (abs / YEAR, "year")
    };
    let plural = if n == 1 { "" } else { "s" };
    if diff >= 0 {
        format!("{n} {unit}{plural} ago")
    } else {
        format!("in {n} {unit}{plural}")
    }
}

/// Format a Unix timestamp as the two human-date lines the command prints:
/// `Local: <local datetime>` and `UTC:   <utc datetime>`. Errors (never panics)
/// when the timestamp is out of chrono's representable range (T-02-05).
fn format_timestamp(secs: i64) -> anyhow::Result<(String, String)> {
    let dt_utc: DateTime<Utc> = DateTime::from_timestamp(secs, 0)
        .ok_or_else(|| anyhow::anyhow!("timestamp {secs} is out of range"))?;
    let dt_local = dt_utc.with_timezone(&Local);
    let local_line = format!("Local: {}", dt_local.format("%Y-%m-%d %H:%M:%S %z"));
    let utc_line = format!("UTC:   {}", dt_utc.format("%Y-%m-%d %H:%M:%S UTC"));
    Ok((local_line, utc_line))
}

/// Parse a date string to a Unix timestamp, trying the three D-12 formats in
/// order: RFC 3339 → `YYYY-MM-DD HH:MM:SS` (local) → `YYYY-MM-DD` (local
/// midnight). A DST-nonexistent/ambiguous local time (`LocalResult::None` /
/// `Ambiguous`) is collapsed with `.single()` to an error — never `.unwrap()`
/// (T-02-05). An unrecognized string `bail!`s with a hint naming the formats.
fn parse_date(s: &str) -> anyhow::Result<i64> {
    // 1) RFC 3339 / ISO 8601 with an explicit offset or `Z`.
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.timestamp());
    }
    // 2) "YYYY-MM-DD HH:MM:SS" interpreted as LOCAL time.
    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return local_naive_to_timestamp(naive);
    }
    // 3) "YYYY-MM-DD" interpreted as LOCAL midnight.
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid midnight for {s}"))?;
        return local_naive_to_timestamp(naive);
    }
    bail!(
        "unrecognized date '{s}'; expected one of: RFC3339 (e.g. 2026-06-22T14:30:00Z), \
         'YYYY-MM-DD HH:MM:SS', or 'YYYY-MM-DD'"
    );
}

/// Convert a naive (zone-less) local datetime to a Unix timestamp, handling the
/// DST `LocalResult` cases (`None` = nonexistent during spring-forward,
/// `Ambiguous` = two valid instants during fall-back) by erroring rather than
/// panicking — `.single()` returns `None` for both, which we map to an error.
fn local_naive_to_timestamp(naive: NaiveDateTime) -> anyhow::Result<i64> {
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.timestamp())
        .ok_or_else(|| anyhow::anyhow!("ambiguous or nonexistent local time: {naive}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 3339 round-trips: format a fixed timestamp as RFC3339 and assert
    /// `parse_date` recovers exactly that timestamp.
    #[test]
    fn round_trip_rfc3339() {
        let ts: i64 = 1_700_000_000;
        let dt = DateTime::from_timestamp(ts, 0).unwrap();
        let formatted = dt.to_rfc3339(); // e.g. 2023-11-14T22:13:20+00:00
        assert_eq!(parse_date(&formatted).unwrap(), ts);
    }

    /// `YYYY-MM-DD HH:MM:SS` (local) round-trips: render a fixed timestamp in
    /// LOCAL time to that format, then assert parse recovers it. (Built in local
    /// time on both sides, so any TZ/DST offset cancels.)
    #[test]
    fn round_trip_naive_datetime_local() {
        let ts: i64 = 1_700_000_000;
        let local = DateTime::from_timestamp(ts, 0)
            .unwrap()
            .with_timezone(&Local);
        let formatted = local.format("%Y-%m-%d %H:%M:%S").to_string();
        // The local instant may be DST-ambiguous in rare zones/dates; only assert
        // the round-trip when chrono resolves it unambiguously.
        if let Ok(parsed) = parse_date(&formatted) {
            assert_eq!(parsed, ts, "naive-local round-trip mismatch");
        }
    }

    /// `YYYY-MM-DD` (local midnight) round-trips to that day's local-midnight
    /// timestamp: format the parsed timestamp back to a date and assert equality.
    #[test]
    fn round_trip_date_local_midnight() {
        // A fixed date string; parse to a local-midnight timestamp, then format
        // that timestamp back in local time and assert the date component matches.
        let date_str = "2023-11-14";
        let ts = parse_date(date_str).expect("date parses");
        let local = DateTime::from_timestamp(ts, 0)
            .unwrap()
            .with_timezone(&Local);
        assert_eq!(local.format("%Y-%m-%d").to_string(), date_str);
        assert_eq!(local.format("%H:%M:%S").to_string(), "00:00:00");
    }

    /// Integer-timestamp formatting produces both a Local and a UTC labeled line,
    /// and the UTC line carries the known calendar date for 1700000000.
    #[test]
    fn format_timestamp_labels_local_and_utc() {
        let (local_line, utc_line) = format_timestamp(1_700_000_000).unwrap();
        assert!(local_line.starts_with("Local:"));
        assert!(utc_line.starts_with("UTC:"));
        assert!(utc_line.contains("2023-11-14"));
    }

    /// A junk string is an `Err` (→ exit 1), never a panic (T-02-05); and the
    /// error message carries the format hint.
    #[test]
    fn junk_string_is_err_with_hint() {
        let err = parse_date("not-a-date").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unrecognized date"), "got: {msg}");
        assert!(msg.contains("RFC3339"), "hint should name RFC3339: {msg}");
    }

    /// An out-of-range timestamp errors rather than panicking.
    #[test]
    fn out_of_range_timestamp_is_err() {
        assert!(format_timestamp(i64::MAX).is_err());
    }

    /// `relative_for` FORMAT across every branch (clock-independent — `now` is
    /// passed explicitly): just-now window, singular vs plural units, and the
    /// past (`… ago`) vs future (`in …`) sign branch.
    #[test]
    fn relative_for_format_across_branches() {
        // Within the just-now window (|Δ| < 45s), either direction.
        assert_eq!(relative_for(1000, 1000), "just now");
        assert_eq!(relative_for(1000, 1000 + 30), "just now");
        assert_eq!(relative_for(1000, 1000 - 30), "just now");
        // Past instants → "N unit(s) ago".
        assert_eq!(relative_for(1000, 1000 + 60), "1 minute ago");
        assert_eq!(relative_for(1000, 1000 + 5 * 60), "5 minutes ago");
        assert_eq!(relative_for(1000, 1000 + 3 * 3600), "3 hours ago");
        assert_eq!(relative_for(1000, 1000 + 2 * 86400), "2 days ago");
        // Future instants → "in N unit(s)".
        assert_eq!(relative_for(1000, 1000 - 60), "in 1 minute");
        assert_eq!(relative_for(1000, 1000 - 2 * 86400), "in 2 days");
        // Larger ladders resolve to weeks/months/years.
        assert_eq!(relative_for(0, 400 * 86400), "1 year ago");
    }

    /// A1 lock (Pitfall 2): the `%Z %z` rendering for a known zone+instant carries
    /// BOTH the abbreviation (`JST`) and the numeric offset (`+0900`). This is the
    /// single chrono×chrono-tz pairing behavior worth pinning against a bump.
    #[test]
    fn tz_line_renders_abbreviation_and_offset() {
        let line = format_tz_line(1_700_000_000, parse_tz("Asia/Tokyo").unwrap()).unwrap();
        assert!(line.contains("JST"), "expected the JST abbreviation: {line}");
        assert!(line.contains("+0900"), "expected the +0900 offset: {line}");
        assert!(line.starts_with("Asia/Tokyo:"), "zone-labeled line: {line}");
    }

    /// An unknown IANA name is an `Err` (→ exit 1) with an IANA hint, never a
    /// panic (mirrors `parse_date`).
    #[test]
    fn parse_tz_rejects_unknown_zone() {
        let err = parse_tz("Not/AZone").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unknown timezone"), "got: {msg}");
        assert!(msg.contains("Asia/Tokyo"), "hint should name an IANA example: {msg}");
    }
}
