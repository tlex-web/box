//! Shared infrastructure for all `box` commands.
//!
//! `errors` holds the typed `BoxError`. Plan 02 adds `output` (color init +
//! print helpers) and `fs` (walkdir wrapper, safe_copy, collision rename).

pub mod errors;
