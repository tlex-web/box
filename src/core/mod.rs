//! Shared infrastructure for all `box` commands.
//!
//! `errors` holds the typed `BoxError`; `output` owns color init + reusable
//! print/glyph/summary helpers; `fs` owns UNC-safe path normalization, the
//! hidden-entry predicate, and timestamp-preserving copy; `input` owns the
//! shared stdin/arg reader with the TTY no-block guard (D-04/D-05).

pub mod config;
pub mod errors;
pub mod fs;
pub mod input;
pub mod output;
