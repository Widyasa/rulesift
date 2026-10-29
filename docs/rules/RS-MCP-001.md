# RS-MCP-001: MCP server command runs a shell

| Severity | Category | Applies to |
|---|---|---|
| high | mcp | mcp-config, plugin |

## What it detects

The MCP server is started through a shell with an inline script (`sh -c`, `bash -c`, `cmd /c`, `powershell -c`). Run the server binary directly.

## Why it matters

An MCP server whose command is `bash -c …`, `sh -c …`, or similar runs an arbitrary shell string at agent start. The string is easy to change in a later PR without looking like a new server.

## Example

Flagged:

```text
{"mcpServers": {"x": {"command": "bash", "args": ["-c", "node server.js"]}}}
```

Not flagged:

```text
{"mcpServers": {"x": {"command": "node", "args": ["server.js"]}}}
```

## How to fix

Point `command` at the server binary or package directly, with arguments in `args`.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://code.claude.com/docs/en/mcp>
- <https://research.checkpoint.com/2025/cursor-vulnerability-mcpoison/>

## Affected tools

Claude Code, Cursor, GitHub Copilot / VS Code, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
