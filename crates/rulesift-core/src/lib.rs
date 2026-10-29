//! RuleSift core library.
//!
//! Synchronous, side-effect free (never prints) building blocks for scanning
//! agent configuration files: model, config, registry, discovery, parsers,
//! rules, scoring, and reports.

#![forbid(unsafe_code)]

/// Crate version, as set in `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
