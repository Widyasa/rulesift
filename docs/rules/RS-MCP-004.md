# RS-MCP-004: Remote MCP server over plain HTTP

| Severity | Category | Applies to |
|---|---|---|
| medium | mcp | mcp-config, plugin |

## What it detects

A remote MCP server is reached over `http://` (not localhost). Traffic, tool results, and credentials can be read or changed in transit.

## Why it matters

A remote MCP server over plain `http://` can be read and modified by anyone on the network path, including the tool descriptions the model trusts.

## Example

Flagged:

```text
{"mcpServers": {"x": {"url": "http://mcp.x.example/sse"}}}
```

Not flagged:

```text
{"mcpServers": {"x": {"url": "https://mcp.x.example/sse"}, "y": {"url": "http://localhost:3000/mcp"}}}
```

## How to fix

Use `https://` (localhost is allowed).

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://code.claude.com/docs/en/mcp>

## Affected tools

Claude Code, Cursor, GitHub Copilot / VS Code, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
