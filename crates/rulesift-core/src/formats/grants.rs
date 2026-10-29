//! Tool grants: the normalized view of what settings, agent definitions,
//! commands, and skills allow an agent to do.

use serde::{Deserialize, Serialize};

/// What a grant represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantKind {
    /// Permission rule that allows a tool (`permissions.allow`).
    Allow,
    /// Permission rule that asks first (`permissions.ask`).
    Ask,
    /// Permission rule that denies a tool (`permissions.deny`).
    Deny,
    /// Default permission mode (`defaultMode`).
    DefaultMode,
    /// Agent permission mode (`permissionMode`).
    PermissionMode,
    /// `enableAllProjectMcpServers: true`.
    EnableAllProjectMcpServers,
    /// Project MCP servers enabled by name (`enabledMcpjsonServers`).
    EnabledMcpServer,
    /// Agent tool list entry (`tools`).
    Tool,
    /// Tool the agent may not use (`disallowedTools`).
    DisallowedTool,
    /// Skill/command `allowed-tools` entry.
    AllowedTool,
    /// MCP server made available to an agent (`mcpServers` / `mcp-servers`).
    McpServer,
    /// Codex approval policy.
    ApprovalPolicy,
    /// Codex sandbox mode.
    SandboxMode,
}

/// One grant with its location.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Grant {
    /// Kind.
    pub kind: GrantKind,
    /// Value (tool pattern, mode, server name, ...).
    pub value: String,
    /// 1-based line.
    pub line: usize,
    /// Source key path (JSON pointer or `/frontmatter-key`).
    pub pointer: String,
}

/// Split a tool list written as a string. Claude uses commas
/// (`Read, Grep, Bash(git:*)`); Agent Skills `allowed-tools` uses spaces
/// (`Read Edit Bash(git:*)`). Parentheses group (`Bash(npm run test:*)`).
pub fn split_tools(s: &str) -> Vec<String> {
    let comma = s.contains(',');
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth <= 0 => out.push(std::mem::take(&mut cur)),
            c if c.is_whitespace() && !comma && depth <= 0 => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Tool names from a JSON value that is a list or a delimited string.
pub fn tools_of(v: &serde_json::Value) -> Vec<String> {
    match v {
        serde_json::Value::String(s) => split_tools(s),
        serde_json::Value::Array(a) => a
            .iter()
            .filter_map(|x| match x {
                serde_json::Value::String(s) => Some(s.trim().to_string()),
                _ => None,
            })
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_tool_lists() {
        assert_eq!(
            split_tools("Read, Grep, Bash(git add:*)"),
            vec!["Read", "Grep", "Bash(git add:*)"]
        );
        assert_eq!(
            split_tools("Read Edit Bash(npm run test:*)"),
            vec!["Read", "Edit", "Bash(npm run test:*)"]
        );
        assert_eq!(split_tools("  "), Vec::<String>::new());
        assert_eq!(
            tools_of(&serde_json::json!(["read", "search", 3])),
            vec!["read", "search"]
        );
    }
}
