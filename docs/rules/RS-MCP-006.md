# RS-MCP-006: MCP server auto-trusted or tools auto-approved

| Severity | Category | Applies to |
|---|---|---|
| medium | mcp | mcp-config, plugin |

## What it detects

The server is trusted without confirmation (`trust: true`) or its tools are approved automatically (`alwaysAllow`, `autoApprove`), so tool calls run without the user seeing them.

## Why it matters

`trust: true`, `alwaysAllow`, or `autoApprove` let the server's tools run without confirmation, so one poisoned tool description is enough to act.

## Example

Flagged:

```text
{"mcpServers": {"x": {"command": "x-mcp", "alwaysAllow": ["run_query"]}}}
```

Not flagged:

```text
{"mcpServers": {"x": {"command": "x-mcp"}}}
```

## How to fix

Remove auto-approval from committed config; let each user decide locally.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://invariantlabs.ai/blog/mcp-security-notification-tool-poisoning-attacks>
- <https://geminicli.com/docs/reference/configuration/>

## Affected tools

Claude Code, Cursor, GitHub Copilot / VS Code, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
