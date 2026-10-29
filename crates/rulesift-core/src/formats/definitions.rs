//! `md+fm` definitions: agents and commands, plus skill `allowed-tools`.
//! Extracts tool grants from frontmatter (`tools`, `allowed-tools`,
//! `permissionMode`, `mcpServers` / `mcp-servers`, `disallowedTools`).
//!
//! Docs: Claude sub-agents <https://code.claude.com/docs/en/sub-agents>,
//! Claude commands/skills <https://code.claude.com/docs/en/skills>, Copilot
//! custom agents <https://docs.github.com/en/copilot/reference/custom-agents-configuration>,
//! Copilot prompt files <https://docs.github.com/en/copilot/concepts/prompting-and-context/prompt-files>.

use super::grants::{tools_of, Grant, GrantKind};
use crate::parse::frontmatter::Frontmatter;
use serde_json::Value;

/// Frontmatter keys and the grant they produce.
pub const GRANT_KEYS: &[(&str, GrantKind)] = &[
    ("tools", GrantKind::Tool),
    ("allowed-tools", GrantKind::AllowedTool),
    ("allowed_tools", GrantKind::AllowedTool),
    ("allowedTools", GrantKind::AllowedTool),
    ("disallowedTools", GrantKind::DisallowedTool),
    ("disallowed-tools", GrantKind::DisallowedTool),
];

/// Extract grants from a definition's frontmatter.
pub fn grants(fm: &Frontmatter) -> Vec<Grant> {
    let mut out = Vec::new();
    let Some(map) = fm.mapping() else { return out };
    for (key, kind) in GRANT_KEYS {
        if let Some(v) = map.get(*key) {
            for t in tools_of(v) {
                out.push(Grant {
                    kind: *kind,
                    value: t,
                    line: fm.line_of(key),
                    pointer: format!("/{key}"),
                });
            }
        }
    }
    if let Some(m) = map.get("permissionMode").and_then(Value::as_str) {
        out.push(Grant {
            kind: GrantKind::PermissionMode,
            value: m.to_string(),
            line: fm.line_of("permissionMode"),
            pointer: "/permissionMode".into(),
        });
    }
    for key in ["mcpServers", "mcp-servers"] {
        let names: Vec<String> = match map.get(key) {
            Some(Value::Object(m)) => m.keys().cloned().collect(),
            Some(v @ (Value::Array(_) | Value::String(_))) => tools_of(v),
            _ => Vec::new(),
        };
        for n in names {
            out.push(Grant {
                kind: GrantKind::McpServer,
                value: n,
                line: fm.line_of(key),
                pointer: format!("/{key}"),
            });
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{plugin, settings};
    use crate::parse::frontmatter;
    use std::path::PathBuf;

    fn fixture(rel: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/definitions")
            .join(rel);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    fn fm_grants(rel: &str) -> String {
        let t = fixture(rel);
        let fm = frontmatter::split(&t).unwrap();
        serde_json::to_string_pretty(&grants(&fm)).unwrap()
    }

    #[test]
    fn claude_agent_grants() {
        insta::assert_snapshot!(fm_grants("claude-agent.md"));
    }

    #[test]
    fn claude_command_grants() {
        insta::assert_snapshot!(fm_grants("claude-command.md"));
    }

    #[test]
    fn copilot_agent_grants() {
        insta::assert_snapshot!(fm_grants("copilot.agent.md"));
    }

    #[test]
    fn copilot_prompt_grants() {
        insta::assert_snapshot!(fm_grants("release.prompt.md"));
    }

    #[test]
    fn skill_allowed_tools() {
        insta::assert_snapshot!(fm_grants("skill.md"));
    }

    #[test]
    fn claude_settings_grants() {
        insta::assert_snapshot!(serde_json::to_string_pretty(&settings::normalize(&fixture(
            "settings.json"
        )))
        .unwrap());
    }

    #[test]
    fn plugin_manifest() {
        insta::assert_snapshot!(serde_json::to_string_pretty(&plugin::normalize(
            &fixture("plugin.json"),
            false
        ))
        .unwrap());
    }

    #[test]
    fn marketplace_manifest() {
        insta::assert_snapshot!(serde_json::to_string_pretty(&plugin::normalize(
            &fixture("marketplace.json"),
            true
        ))
        .unwrap());
    }

    #[test]
    fn invalid_json_is_fmt_007() {
        let s = settings::normalize("{\"permissions\": {\"allow\": [}\n");
        assert_eq!(s.issues.len(), 1);
        assert!(s.issues[0].message.starts_with("invalid JSON"));
        let p = plugin::normalize("{ \"name\": ", false);
        assert!(p.issues[0].message.starts_with("invalid JSON"));
        let p = plugin::normalize("{\"version\": \"1\"}", false);
        assert_eq!(
            p.issues[0].message,
            "plugin manifest is missing the required `name` field."
        );
        let m = plugin::normalize("{\"name\": \"m\"}", true);
        assert_eq!(m.issues.len(), 2);
        let s = settings::normalize("{\"permissions\": []}");
        assert_eq!(s.issues[0].message, "`permissions` must be an object.");
    }
}
