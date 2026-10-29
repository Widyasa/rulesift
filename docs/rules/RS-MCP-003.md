# RS-MCP-003: Unpinned package runner

| Severity | Category | Applies to |
|---|---|---|
| high | mcp | mcp-config, plugin |

## What it detects

The MCP server is fetched by a package runner without a pinned version (`npx -y pkg`, `@latest`, `uvx pkg`, `docker … :latest`). Each start can run a different, possibly compromised release. Pin an exact version or digest.

## Why it matters

`npx -y pkg`, `uvx pkg`, or `pipx run pkg` without a version runs whatever release is newest on each start. A compromised or typo-squatted release then runs on every developer machine.

## Example

Flagged:

```text
{"mcpServers": {"x": {"command": "npx", "args": ["-y", "example-mcp"]}}}
```

Not flagged:

```text
{"mcpServers": {"x": {"command": "npx", "args": ["-y", "example-mcp@1.2.3"]}}}
```

## How to fix

Pin an exact version (`pkg@1.2.3`, `pkg==1.2.3`) or a digest, and update it through reviewed PRs.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://code.claude.com/docs/en/mcp>
- <https://research.checkpoint.com/2025/cursor-vulnerability-mcpoison/>

## Affected tools

Claude Code, Cursor, GitHub Copilot / VS Code, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
