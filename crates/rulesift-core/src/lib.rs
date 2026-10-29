//! RuleSift core library.
//!
//! Synchronous, side-effect free (never prints) building blocks for scanning
//! agent configuration files: model, config, registry, discovery, parsers,
//! rules, scoring, and reports.

#![forbid(unsafe_code)]

pub mod config;
pub mod discovery;
pub mod error;
pub mod formats;
pub mod model;
pub mod normalize;
pub mod parse;
pub mod registry;
pub mod reveal;
pub mod rules;
pub mod text;
pub mod validate;

pub use error::{CoreError, Result};

/// Crate version, as set in `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
