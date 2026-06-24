//! Integration tests for `box weather` (WTHR-01).
//!
//! Black-box against the compiled binary via `assert_cmd`. These cover the
//! cross-cutting CLI contract that does NOT require live network access:
//!   - a bad `--units` value is a clap usage error → exit 2 (`ValueEnum` rejects
//!     it; the command never runs);
//!   - an unreachable weather service → a graceful exit-1 error on stderr with an
//!     empty stdout (the offline path, D-13 / Pitfall WTHR-1).
//!
//! ## Determinism — never hit the live network
//! The offline test sets `BOX_WEATHER_BASE_URL=http://127.0.0.1:1`. Port 1 on
//! loopback is not bound, so the very first GET (the geocode call) fails with a
//! connection error → the `Err(_)` offline arm in `fetch` → "could not reach
//! weather service (offline?)" + exit 1. This is deterministic and offline: it
//! never resolves DNS and never contacts the real Open-Meteo API. The
//! authoritative happy-path coverage (geocode hit/no-match, metric/imperial unit
//! labels, the WMO map) lives in the in-module `#[cfg(test)]` fixture tests
//! (`cargo test --bin box weather`); a live fetch is an OPTIONAL phase-UAT
//! convenience, not part of the automated suite.

use assert_cmd::Command;
use predicates::prelude::*;

/// A bad `--units` value is a clap usage error: exit 2, message on stderr. clap's
/// `ValueEnum` rejects the value before the command runs (mirrors the bad-flag
/// exit-2 shape in `tests/cli.rs`). Exit 2 is reserved for usage errors; the
/// command's own runtime errors (offline / non-2xx / no-match) are exit 1.
#[test]
fn bad_units_exits_2() {
    Command::cargo_bin("box")
        .unwrap()
        .args(["weather", "London", "--units", "bogus"])
        .assert()
        .failure()
        .code(2);
}

/// An unreachable weather service → exit 1 with a graceful error on stderr and an
/// empty stdout (D-13). Deterministic + offline via the `BOX_WEATHER_BASE_URL`
/// seam pointing at an unbound loopback port (never touches the live API).
#[test]
fn offline_exits_1_clean() {
    Command::cargo_bin("box")
        .unwrap()
        .args(["weather", "London"])
        .env("BOX_WEATHER_BASE_URL", "http://127.0.0.1:1")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("offline").or(predicate::str::contains("service")));
}
