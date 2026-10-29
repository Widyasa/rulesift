//! Claude Code settings (`.claude/settings*.json`): permissions,
//! `defaultMode`, and MCP enablement as [`Grant`]s.
//!
//! Docs: <https://code.claude.com/docs/en/settings>.

use super::grants::{Grant, GrantKind};
use super::FormatIssue;
use crate::parse::json::{self, JsonDoc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Normalized settings of one file.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SettingsFile {
    /// Grants sorted by line.
    pub grants: Vec<Grant>,
    /// RS-FMT-007 issues.
    pub issues: Vec<FormatIssue>,
}

/// Extract grants from parsed settings JSON.
pub fn from_doc(doc: &JsonDoc) -> SettingsFile {
    let mut out = SettingsFile::default();
    let Some(root) = doc.value.as_object() else {
        out.issues
            .push(FormatIssue::new(1, "settings file must be a JSON object."));
        return out;
    };
    let mut push = |kind, value: String, ptr: String| {
        out.grants.push(Grant {
            kind,
            value,
            line: doc.line_of(&ptr),
            pointer: ptr,
        });
    };
    if let Some(p) = root.get("permissions") {
        match p {
            Value::Object(perm) => {
                for (key, kind) in [
                    ("allow", GrantKind::Allow),
                    ("ask", GrantKind::Ask),
                    ("deny", GrantKind::Deny),
                ] {
                    if let Some(Value::Array(list)) = perm.get(key) {
                        for (i, v) in list.iter().enumerate() {
                            if let Some(s) = v.as_str() {
                                push(kind, s.to_string(), format!("/permissions/{key}/{i}"));
                            }
                        }
                    }
                }
                if let Some(m) = perm.get("defaultMode").and_then(Value::as_str) {
                    push(
                        GrantKind::DefaultMode,
                        m.to_string(),
                        "/permissions/defaultMode".into(),
                    );
                }
            }
            _ => out.issues.push(FormatIssue::new(
                doc.line_of("/permissions"),
                "`permissions` must be an object.",
            )),
        }
    }
    if let Some(m) = root.get("defaultMode").and_then(Value::as_str) {
        push(GrantKind::DefaultMode, m.to_string(), "/defaultMode".into());
    }
    if root
        .get("enableAllProjectMcpServers")
        .and_then(Value::as_bool)
        == Some(true)
    {
        push(
            GrantKind::EnableAllProjectMcpServers,
            "true".into(),
            "/enableAllProjectMcpServers".into(),
        );
    }
    if let Some(Value::Array(list)) = root.get("enabledMcpjsonServers") {
        for (i, v) in list.iter().enumerate() {
            if let Some(s) = v.as_str() {
                push(
                    GrantKind::EnabledMcpServer,
                    s.to_string(),
                    format!("/enabledMcpjsonServers/{i}"),
                );
            }
        }
    }
    out.grants
        .sort_by(|a, b| (a.line, &a.pointer).cmp(&(b.line, &b.pointer)));
    out
}

/// Parse and normalize a settings file.
pub fn normalize(text: &str) -> SettingsFile {
    match json::parse(text) {
        Ok(doc) => from_doc(&doc),
        Err(e) => SettingsFile {
            grants: Vec::new(),
            issues: vec![FormatIssue::new(e.line, e.to_string())],
        },
    }
}
