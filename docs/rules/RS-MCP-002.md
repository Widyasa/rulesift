# RS-MCP-002: Literal secret in MCP env or headers

| Severity | Category | Applies to |
|---|---|---|
| high | mcp | mcp-config, plugin |

## What it detects

An MCP server's `env` or `headers` contains a literal secret instead of a `${VAR}` reference. The value is committed with the config; rotate it.

## Why it matters

A literal API key or token in a committed MCP config leaks to everyone with read access to the repo, and to the git history forever.

## Example

Flagged:

```text
{"mcpServers": {"x": {"command": "x-mcp", "env": {"API_TOKEN": "EXAMPLEq8Zt3Lw9Xv2Rk7"}}}}
```

Not flagged:

```text
{"mcpServers": {"x": {"command": "x-mcp", "env": {"API_TOKEN": "${API_TOKEN}", "LOG_LEVEL": "debug"}}}}
```

## How to fix

Use environment variable references (`${API_KEY}`) or the client's secret store, and rotate the leaked value.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://code.claude.com/docs/en/mcp>

## Affected tools

Claude Code, Cursor, GitHub Copilot / VS Code, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
