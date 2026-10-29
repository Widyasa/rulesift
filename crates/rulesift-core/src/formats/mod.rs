//! Normalizers: per-tool file formats → shared models (`McpServer`, `Hook`,
//! settings, tool grants). Rules only ever see the normalized models.

pub mod definitions;
pub mod grants;
pub mod hooks;
pub mod mcp;
pub mod mcp_toml;
pub mod mcp_yaml;
pub mod plugin;
pub mod settings;

use serde::{Deserialize, Serialize};

/// A format problem reported as RS-FMT-007 (invalid JSON/TOML/YAML or a
/// missing required field).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FormatIssue {
    /// 1-based line.
    pub line: usize,
    /// English message.
    pub message: String,
}

impl FormatIssue {
    /// New issue.
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}
