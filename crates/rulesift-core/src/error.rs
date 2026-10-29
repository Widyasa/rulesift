//! Error type of the core library.

use std::path::PathBuf;

/// Result alias using [`CoreError`].
pub type Result<T, E = CoreError> = std::result::Result<T, E>;

/// Every error the core library can return. Messages are English and name the
/// offending file, key, or rule so they can be shown to users verbatim.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// Reading or writing a file failed.
    #[error("cannot read {path}: {source}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// `rulesift.toml` is invalid.
    #[error("invalid config {path}: {message}")]
    Config {
        /// Config file path.
        path: PathBuf,
        /// English description naming the bad key.
        message: String,
    },
    /// A rule file or rule definition is invalid.
    #[error("invalid rule {rule_id} in {file}: {message}")]
    Rule {
        /// Rule file path (or `builtin:<name>` for embedded rules).
        file: String,
        /// Rule id, or `<unknown>` if it could not be read.
        rule_id: String,
        /// English description.
        message: String,
    },
    /// A scanned file could not be parsed.
    #[error("cannot parse {path}: {message}")]
    Parse {
        /// File path.
        path: String,
        /// English description.
        message: String,
    },
    /// A glob pattern is invalid.
    #[error("invalid glob {pattern:?}: {message}")]
    Glob {
        /// The pattern.
        pattern: String,
        /// English description.
        message: String,
    },
    /// A git operation failed.
    #[error("git error: {0}")]
    Git(String),
    /// Lockfile problem.
    #[error("invalid lockfile {path}: {message}")]
    Lock {
        /// Lockfile path.
        path: PathBuf,
        /// English description.
        message: String,
    },
    /// Any other invalid input (bad value, path escape, ...).
    #[error("{0}")]
    Invalid(String),
}

impl CoreError {
    /// Build an [`CoreError::Io`] error.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
