//! MCP server normalizer, JSON family (PRD §5 F1).
//!
//! Sources: `mcpServers` (Claude Code `.mcp.json`, Cursor, Claude Desktop,
//! Roo, Kiro, Junie, Gemini CLI, Amazon Q, Continue JSON), `servers`
//! (VS Code `.vscode/mcp.json`), and `context_servers` (Zed settings).
//! Trust fields (`trust`, `alwaysAllow`, `autoApprove`) map to
//! `auto_trust` / `auto_approve`.

use super::FormatIssue;
use crate::parse::json::{self, escape_token, JsonDoc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Transport of an MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Transport {
    /// Local process over stdio.
    Stdio,
    /// Streamable HTTP.
    Http,
    /// Server-sent events.
    Sse,
}

/// One normalized MCP server definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServer {
    /// Server name (map key or `name` field).
    pub name: String,
    /// Transport.
    pub transport: Transport,
    /// Executable for stdio servers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables (values as written, may be `${VAR}` references).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// URL for remote servers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// HTTP headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Server is trusted without confirmation (e.g. Gemini `trust: true`).
    #[serde(default)]
    pub auto_trust: bool,
    /// Tools approved without confirmation (`alwaysAllow`, `autoApprove`); `*` = all.
    #[serde(default)]
    pub auto_approve: Vec<String>,
    /// Whether the server is disabled in the config.
    #[serde(default)]
    pub disabled: bool,
    /// JSON pointer (or TOML/YAML path) of the server entry.
    pub pointer: String,
    /// 1-based line of the server entry.
    pub line: usize,
    /// 1-based lines of individual fields (`command`, `args`, `env`, `url`, ...).
    #[serde(default)]
    pub field_lines: BTreeMap<String, usize>,
}

/// Result of normalizing one MCP config file.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// Servers sorted by name.
    pub servers: Vec<McpServer>,
    /// RS-FMT-007 issues.
    pub issues: Vec<FormatIssue>,
}

/// Keys that hold the server map, in lookup order.
pub const SERVER_KEYS: &[&str] = &["mcpServers", "servers", "context_servers"];

fn string_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn string_list(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Array(a)) => a.iter().filter_map(string_of).collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn string_map(v: Option<&Value>) -> BTreeMap<String, String> {
    match v {
        Some(Value::Object(m)) => m
            .iter()
            .map(|(k, v)| (k.clone(), string_of(v).unwrap_or_else(|| v.to_string())))
            .collect(),
        _ => BTreeMap::new(),
    }
}

/// Build a server from an entry object. Shared by the TOML and YAML
/// normalizers, which convert their values to JSON first.
pub fn server_from_entry(
    name: &str,
    entry: &Map<String, Value>,
    pointer: &str,
    line_of: &dyn Fn(&str) -> usize,
    issues: &mut Vec<FormatIssue>,
) -> Option<McpServer> {
    // Zed (older form): "command": { "path": ..., "args": [...], "env": {...} }
    let (command, mut args, mut env) = match entry.get("command") {
        Some(Value::Object(c)) => (
            c.get("path").and_then(string_of),
            string_list(c.get("args")),
            string_map(c.get("env")),
        ),
        Some(v) => (string_of(v), Vec::new(), BTreeMap::new()),
        None => (None, Vec::new(), BTreeMap::new()),
    };
    if args.is_empty() {
        args = string_list(entry.get("args"));
    }
    if env.is_empty() {
        env = string_map(entry.get("env"));
    }
    let http_url = entry.get("httpUrl").and_then(string_of);
    let url = http_url
        .clone()
        .or_else(|| entry.get("url").and_then(string_of))
        .or_else(|| entry.get("serverUrl").and_then(string_of));
    let declared = entry
        .get("type")
        .or_else(|| entry.get("transport"))
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let transport = match declared.as_deref() {
        Some("sse") => Transport::Sse,
        Some("http")
        | Some("streamable-http")
        | Some("streamablehttp")
        | Some("streamable_http") => Transport::Http,
        Some("stdio") => Transport::Stdio,
        _ if http_url.is_some() => Transport::Http,
        _ if url.is_some() && command.is_none() => Transport::Http,
        _ => Transport::Stdio,
    };
    let headers = string_map(
        entry
            .get("headers")
            .or_else(|| entry.get("requestInit").and_then(|r| r.get("headers"))),
    );
    let auto_trust = entry.get("trust").and_then(Value::as_bool).unwrap_or(false);
    let mut auto_approve = string_list(entry.get("alwaysAllow"));
    auto_approve.extend(string_list(entry.get("autoApprove")));
    auto_approve.sort();
    auto_approve.dedup();
    let disabled = entry
        .get("disabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || entry.get("enabled").and_then(Value::as_bool) == Some(false);

    if command.is_none() && url.is_none() {
        issues.push(FormatIssue::new(
            line_of(pointer),
            format!("MCP server {name:?} has neither `command` nor `url`."),
        ));
        return None;
    }
    let mut field_lines = BTreeMap::new();
    for f in [
        "command",
        "args",
        "env",
        "url",
        "httpUrl",
        "serverUrl",
        "headers",
        "trust",
        "alwaysAllow",
        "autoApprove",
        "type",
    ] {
        if entry.contains_key(f) {
            field_lines.insert(
                f.to_string(),
                line_of(&format!("{pointer}/{}", escape_token(f))),
            );
        }
    }
    Some(McpServer {
        name: name.to_string(),
        transport,
        command,
        args,
        env,
        url,
        headers,
        auto_trust,
        auto_approve,
        disabled,
        pointer: pointer.to_string(),
        line: line_of(pointer),
        field_lines,
    })
}

/// Normalize a parsed JSON-like value (also used for TOML/YAML converted to JSON).
pub fn from_value(root: &Value, line_of: &dyn Fn(&str) -> usize) -> McpConfig {
    let mut cfg = McpConfig::default();
    let obj = match root {
        Value::Object(o) => o,
        _ => {
            cfg.issues
                .push(FormatIssue::new(1, "MCP config must be a JSON object."));
            return cfg;
        }
    };
    for key in SERVER_KEYS {
        let Some(v) = obj.get(*key) else { continue };
        let base = format!("/{key}");
        match v {
            Value::Object(servers) => {
                for (name, entry) in servers {
                    let ptr = format!("{base}/{}", escape_token(name));
                    match entry {
                        Value::Object(e) => {
                            if let Some(s) =
                                server_from_entry(name, e, &ptr, line_of, &mut cfg.issues)
                            {
                                cfg.servers.push(s);
                            }
                        }
                        _ => cfg.issues.push(FormatIssue::new(
                            line_of(&ptr),
                            format!("MCP server {name:?} must be an object."),
                        )),
                    }
                }
            }
            // Continue-style list of servers with a `name` field.
            Value::Array(list) => {
                for (i, entry) in list.iter().enumerate() {
                    let ptr = format!("{base}/{i}");
                    match entry {
                        Value::Object(e) => {
                            let name = e.get("name").and_then(Value::as_str).map(str::to_string);
                            match name {
                                Some(n) => {
                                    if let Some(s) =
                                        server_from_entry(&n, e, &ptr, line_of, &mut cfg.issues)
                                    {
                                        cfg.servers.push(s);
                                    }
                                }
                                None => cfg.issues.push(FormatIssue::new(
                                    line_of(&ptr),
                                    format!("MCP server entry {i} is missing `name`."),
                                )),
                            }
                        }
                        _ => cfg.issues.push(FormatIssue::new(
                            line_of(&ptr),
                            format!("MCP server entry {i} must be an object."),
                        )),
                    }
                }
            }
            _ => cfg.issues.push(FormatIssue::new(
                line_of(&base),
                format!("`{key}` must be an object."),
            )),
        }
    }
    cfg.servers
        .sort_by(|a, b| (a.name.as_str(), a.line).cmp(&(b.name.as_str(), b.line)));
    cfg.issues.sort();
    cfg
}

/// Normalize a JSON (or JSONC) MCP config file.
pub fn normalize_json(text: &str) -> McpConfig {
    match json::parse(text) {
        Ok(doc) => from_doc(&doc),
        Err(e) => McpConfig {
            servers: Vec::new(),
            issues: vec![FormatIssue::new(e.line, e.to_string())],
        },
    }
}

/// Normalize an already parsed JSON document.
pub fn from_doc(doc: &JsonDoc) -> McpConfig {
    from_value(&doc.value, &|p| doc.line_of(p))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::PathBuf;

    pub(crate) fn fixture(rel: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/mcp")
            .join(rel);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    fn snap(rel: &str) -> String {
        serde_json::to_string_pretty(&normalize_json(&fixture(rel))).unwrap()
    }

    macro_rules! fixture_snapshot {
        ($name:ident, $rel:literal) => {
            #[test]
            fn $name() {
                insta::assert_snapshot!(snap($rel));
            }
        };
    }

    fixture_snapshot!(claude_code, "claude-code/.mcp.json");
    fixture_snapshot!(claude_desktop, "claude-desktop/claude_desktop_config.json");
    fixture_snapshot!(cursor, "cursor/mcp.json");
    fixture_snapshot!(roo_code, "roo-code/mcp.json");
    fixture_snapshot!(kiro, "kiro/mcp.json");
    fixture_snapshot!(junie, "junie/mcp.json");
    fixture_snapshot!(gemini_cli, "gemini-cli/settings.json");
    fixture_snapshot!(amazon_q, "amazon-q/mcp.json");
    fixture_snapshot!(continue_json, "continue/docs.json");
    fixture_snapshot!(vscode, "copilot/mcp.json");
    fixture_snapshot!(zed, "zed/settings.json");

    #[test]
    fn trust_fields_map() {
        let g = normalize_json(&fixture("gemini-cli/settings.json"));
        assert!(g.servers.iter().any(|s| s.auto_trust));
        let r = normalize_json(&fixture("roo-code/mcp.json"));
        assert!(r.servers.iter().any(|s| !s.auto_approve.is_empty()));
        let k = normalize_json(&fixture("kiro/mcp.json"));
        assert!(k.servers.iter().any(|s| !s.auto_approve.is_empty()));
    }

    #[test]
    fn invalid_json_yields_issue() {
        let c = normalize_json(&fixture("invalid/broken.json"));
        assert!(c.servers.is_empty());
        assert_eq!(c.issues.len(), 1);
        assert!(c.issues[0].message.starts_with("invalid JSON"));
        assert_eq!(c.issues[0].line, 5);
    }

    #[test]
    fn missing_command_yields_issue() {
        let c = normalize_json(&fixture("invalid/missing-command.json"));
        assert_eq!(c.servers.len(), 1);
        assert_eq!(
            c.issues,
            vec![FormatIssue::new(
                7,
                "MCP server \"broken\" has neither `command` nor `url`."
            )]
        );
    }

    #[test]
    fn non_object_root() {
        let c = normalize_json("[1]");
        assert_eq!(c.issues.len(), 1);
    }
}
