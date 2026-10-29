//! MCP normalizer for Continue YAML block files (`.continue/mcpServers/*.yaml`):
//! top-level `name`, `version`, `schema`, and a `mcpServers` list.
//!
//! Docs: <https://docs.continue.dev/customize/deep-dives/mcp>.

use super::mcp::{from_value, McpConfig};
use super::mcp_toml::lookup;
use super::FormatIssue;
use serde_json::Value;
use std::collections::BTreeMap;

/// Map pointers to lines for block-style YAML: top-level keys, list items
/// under them, and keys inside those items.
pub(crate) fn line_map(text: &str) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    let mut top: Option<String> = None;
    let mut idx: Option<usize> = None;
    let mut item_indent = 0usize;
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let trimmed = raw.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw.len() - trimmed.len();
        if indent == 0 && !trimmed.starts_with('-') {
            if let Some((k, _)) = trimmed.split_once(':') {
                let k = k.trim().trim_matches(['"', '\'']).to_string();
                map.entry(format!("/{k}")).or_insert(line);
                top = Some(k);
                idx = None;
            }
            continue;
        }
        let Some(t) = &top else { continue };
        if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| (trimmed == "-").then_some(""))
        {
            idx = Some(idx.map_or(0, |n| n + 1));
            item_indent = indent + 2;
            let base = format!("/{t}/{}", idx.unwrap_or(0));
            map.entry(base.clone()).or_insert(line);
            if let Some((k, _)) = rest.split_once(':') {
                map.entry(format!("{base}/{}", k.trim())).or_insert(line);
            }
            continue;
        }
        if let (Some(n), true) = (idx, indent == item_indent) {
            if let Some((k, _)) = trimmed.split_once(':') {
                map.entry(format!("/{t}/{n}/{}", k.trim())).or_insert(line);
            }
        }
    }
    map
}

/// Normalize a Continue MCP YAML block.
pub fn normalize_yaml(text: &str) -> McpConfig {
    let value: Value = match serde_saphyr::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            let line = e.location().map(|l| l.line() as usize).unwrap_or(1).max(1);
            let msg = e.to_string();
            let first = msg.lines().next().unwrap_or("invalid YAML").trim();
            let first = first.strip_prefix("error: ").unwrap_or(first);
            return McpConfig {
                issues: vec![FormatIssue::new(line, format!("invalid YAML: {first}"))],
                ..Default::default()
            };
        }
    };
    let lines = line_map(text);
    let mut cfg = from_value(&value, &|p| lookup(&lines, p));
    if value.is_object() && value.get("mcpServers").is_none() {
        cfg.issues.push(FormatIssue::new(
            1,
            "Continue MCP block is missing `mcpServers`.",
        ));
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::mcp::tests::fixture;
    use crate::formats::mcp::{normalize_json, McpServer};
    use crate::formats::mcp_toml::normalize_toml;

    /// Position-free view used to compare the three syntaxes.
    fn shape(c: &McpConfig) -> Vec<McpServer> {
        c.servers
            .iter()
            .cloned()
            .map(|mut s| {
                s.pointer = String::new();
                s.line = 0;
                s.field_lines.clear();
                s
            })
            .collect()
    }

    #[test]
    fn continue_snapshot() {
        let c = normalize_yaml(&fixture("continue/db.yaml"));
        insta::assert_snapshot!(serde_json::to_string_pretty(&c).unwrap());
    }

    #[test]
    fn toml_yaml_json_normalize_identically() {
        let j = normalize_json(&fixture("equivalent/servers.json"));
        let t = normalize_toml(&fixture("equivalent/config.toml"));
        let y = normalize_yaml(&fixture("equivalent/servers.yaml"));
        assert!(j.issues.is_empty() && t.issues.is_empty() && y.issues.is_empty());
        assert_eq!(shape(&j), shape(&t));
        assert_eq!(shape(&j), shape(&y));
        insta::assert_snapshot!(serde_json::to_string_pretty(&shape(&j)).unwrap());
    }

    #[test]
    fn lines_are_tracked() {
        let y = normalize_yaml(&fixture("equivalent/servers.yaml"));
        let docs = y.servers.iter().find(|s| s.name == "docs").unwrap();
        assert_eq!(docs.line, 5);
        assert_eq!(docs.field_lines["command"], 6);
        let t = normalize_toml(&fixture("equivalent/config.toml"));
        let docs = t.servers.iter().find(|s| s.name == "docs").unwrap();
        assert_eq!(docs.line, 1);
        assert_eq!(docs.field_lines["args"], 3);
    }

    #[test]
    fn invalid_yaml() {
        let c = normalize_yaml("mcpServers:\n  - name: [x\n");
        assert_eq!(c.issues.len(), 1);
        assert!(c.issues[0].message.starts_with("invalid YAML"));
        let c = normalize_yaml("name: x\nversion: 1\n");
        assert_eq!(
            c.issues[0].message,
            "Continue MCP block is missing `mcpServers`."
        );
    }
}
