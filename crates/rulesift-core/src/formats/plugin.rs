//! Claude Code plugin and marketplace manifests
//! (`.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json`).
//!
//! Docs: <https://code.claude.com/docs/en/plugins/manifest-reference>.

use super::hooks::{self, HooksFile};
use super::mcp::{self, McpConfig};
use super::FormatIssue;
use crate::model::Tool;
use crate::parse::json::{self, JsonDoc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Manifest type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestKind {
    /// `plugin.json`.
    Plugin,
    /// `marketplace.json`.
    Marketplace,
}

/// A plugin listed in a marketplace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    /// Plugin name.
    pub name: String,
    /// Source (path or repository) as text.
    pub source: String,
    /// 1-based line.
    pub line: usize,
}

/// Normalized manifest.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginFile {
    /// Manifest type.
    pub kind: Option<ManifestKind>,
    /// `name`.
    pub name: Option<String>,
    /// Inline MCP servers (`mcpServers` object), normalized.
    pub mcp: McpConfig,
    /// Inline hooks (`hooks` object), normalized.
    pub hooks: HooksFile,
    /// Marketplace plugin entries.
    pub plugins: Vec<MarketplaceEntry>,
    /// RS-FMT-007 issues.
    pub issues: Vec<FormatIssue>,
}

/// Normalize a parsed manifest. `marketplace` selects the required fields.
pub fn from_doc(doc: &JsonDoc, marketplace: bool) -> PluginFile {
    let mut out = PluginFile {
        kind: Some(if marketplace {
            ManifestKind::Marketplace
        } else {
            ManifestKind::Plugin
        }),
        ..Default::default()
    };
    let Some(root) = doc.value.as_object() else {
        out.issues.push(FormatIssue::new(
            1,
            "plugin manifest must be a JSON object.",
        ));
        return out;
    };
    out.name = root.get("name").and_then(Value::as_str).map(str::to_string);
    let required: &[&str] = if marketplace {
        &["name", "owner", "plugins"]
    } else {
        &["name"]
    };
    for key in required {
        if !root.contains_key(*key) {
            out.issues.push(FormatIssue::new(
                1,
                format!("plugin manifest is missing the required `{key}` field."),
            ));
        }
    }
    if let Some(v @ Value::Object(_)) = root.get("mcpServers") {
        let wrapped = serde_json::json!({ "mcpServers": v });
        out.mcp = mcp::from_value(&wrapped, &|p| doc.line_of(p));
        out.issues.append(&mut out.mcp.issues.clone());
    }
    if root.get("hooks").is_some_and(Value::is_object) {
        out.hooks = hooks::from_doc(Tool::ClaudeCode, doc, false);
        out.issues.append(&mut out.hooks.issues.clone());
    }
    if marketplace {
        match root.get("plugins") {
            Some(Value::Array(list)) => {
                for (i, p) in list.iter().enumerate() {
                    let ptr = format!("/plugins/{i}");
                    match p.get("name").and_then(Value::as_str) {
                        Some(n) => out.plugins.push(MarketplaceEntry {
                            name: n.to_string(),
                            source: p
                                .get("source")
                                .map(|s| {
                                    s.as_str()
                                        .map(str::to_string)
                                        .unwrap_or_else(|| s.to_string())
                                })
                                .unwrap_or_default(),
                            line: doc.line_of(&ptr),
                        }),
                        None => out.issues.push(FormatIssue::new(
                            doc.line_of(&ptr),
                            format!("marketplace plugin {i} is missing `name`."),
                        )),
                    }
                }
            }
            Some(_) => out.issues.push(FormatIssue::new(
                doc.line_of("/plugins"),
                "`plugins` must be a list.",
            )),
            None => {}
        }
    }
    out.issues.sort();
    out.issues.dedup();
    out
}

/// Parse and normalize a manifest.
pub fn normalize(text: &str, marketplace: bool) -> PluginFile {
    match json::parse(text) {
        Ok(doc) => from_doc(&doc, marketplace),
        Err(e) => PluginFile {
            kind: Some(if marketplace {
                ManifestKind::Marketplace
            } else {
                ManifestKind::Plugin
            }),
            issues: vec![FormatIssue::new(e.line, e.to_string())],
            ..Default::default()
        },
    }
}
