//! Shared infrastructure for all `box` commands.
//!
//! `errors` holds the typed `BoxError`; `output` owns color init + reusable
//! print/glyph/summary helpers; `fs` owns UNC-safe path normalization, the
//! hidden-entry predicate, and timestamp-preserving copy.

pub mod errors;
pub mod output;
