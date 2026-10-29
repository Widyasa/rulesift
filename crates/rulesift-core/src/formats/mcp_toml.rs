//! MCP normalizer for OpenAI Codex `.codex/config.toml` (`[mcp_servers.*]`),
//! plus the `approval_policy` / `sandbox_mode` settings.
//!
//! Docs: <https://developers.openai.com/codex/config-basic>,
//! <https://developers.openai.com/codex/mcp>.

use super::mcp::{from_value, McpConfig, Setting};
use super::FormatIssue;
use std::collections::BTreeMap;

/// Top-level Codex settings recorded as [`Setting`]s.
pub const CODEX_SETTINGS: &[&str] = &["approval_policy", "sandbox_mode"];

/// Map TOML paths (as JSON pointers) to 1-based lines by scanning table
/// headers and `key =` lines.
fn line_map(text: &str) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    let mut table: Vec<String> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let l = raw.trim();
        if l.starts_with('#') || l.is_empty() {
            continue;
        }
        if l.starts_with('[') {
            let inner = l.trim_start_matches('[').split(']').next().unwrap_or("");
            table = split_dotted(inner);
            map.entry(pointer(&table)).or_insert(line);
            continue;
        }
        if let Some((k, _)) = l.split_once('=') {
            let mut p = table.clone();
            p.extend(split_dotted(k));
            map.entry(pointer(&p)).or_insert(line);
            // dotted keys define their parent tables too
            for n in table.len() + 1..p.len() {
                map.entry(pointer(&p[..n])).or_insert(line);
            }
        }
    }
    map
}

fn split_dotted(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"') | (None, '\'') => quote = Some(c),
            (None, '.') => out.push(std::mem::take(&mut cur).trim().to_string()),
            (None, c) => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

fn pointer(parts: &[String]) -> String {
    parts
        .iter()
        .map(|p| format!("/{}", crate::parse::json::escape_token(p)))
        .collect()
}

/// Line lookup with ancestor fallback.
pub(crate) fn lookup(map: &BTreeMap<String, usize>, ptr: &str) -> usize {
    let mut p = ptr.to_string();
    loop {
        if let Some(l) = map.get(&p) {
            return *l;
        }
        match p.rfind('/') {
            Some(i) => p.truncate(i),
            None => return 1,
        }
    }
}

/// Normalize a Codex `config.toml`.
pub fn normalize_toml(text: &str) -> McpConfig {
    let value: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            let line = e
                .span()
                .map(|s| crate::text::line_of(text, s.start))
                .unwrap_or(1);
            return McpConfig {
                issues: vec![FormatIssue::new(
                    line,
                    format!("invalid TOML: {}", e.message().trim()),
                )],
                ..Default::default()
            };
        }
    };
    let json = match serde_json::to_value(&value) {
        Ok(j) => j,
        Err(e) => {
            return McpConfig {
                issues: vec![FormatIssue::new(1, format!("unsupported TOML value: {e}"))],
                ..Default::default()
            }
        }
    };
    let lines = line_map(text);
    let mut cfg = from_value(&json, &|p| lookup(&lines, p));
    for key in CODEX_SETTINGS {
        if let Some(v) = json.get(*key) {
            cfg.settings.push(Setting {
                key: (*key).to_string(),
                value: v
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string()),
                line: lookup(&lines, &format!("/{key}")),
            });
        }
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::mcp::tests::fixture;

    #[test]
    fn codex_snapshot() {
        let c = normalize_toml(&fixture("codex/config.toml"));
        insta::assert_snapshot!(serde_json::to_string_pretty(&c).unwrap());
    }

    #[test]
    fn invalid_toml() {
        let c = normalize_toml("[mcp_servers.x]\ncommand = \n");
        assert_eq!(c.issues.len(), 1);
        assert!(c.issues[0].message.starts_with("invalid TOML"));
        assert_eq!(c.issues[0].line, 2);
        let c = normalize_toml(&fixture("invalid/missing-command.toml"));
        assert_eq!(c.issues.len(), 1);
        assert!(c.issues[0].message.contains("neither `command` nor `url`"));
        assert_eq!(c.issues[0].line, 1);
    }

    #[test]
    fn dotted_keys() {
        assert_eq!(
            split_dotted("mcp_servers.\"my.server\""),
            vec!["mcp_servers", "my.server"]
        );
    }
}
