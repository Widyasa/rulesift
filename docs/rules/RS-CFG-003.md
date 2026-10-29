# RS-CFG-003: Overly broad permissions

| Severity | Category | Applies to |
|---|---|---|
| medium | agent-config | settings, agent, mcp-config |

## What it detects

The configuration grants every tool or skips permission prompts (`Bash(*)`, `"*"`, `bypassPermissions`, `enableAllProjectMcpServers`, Codex `approval_policy = "never"` or `sandbox_mode = "danger-full-access"`). Grant only the tools the project needs.

## Why it matters

Wildcard permissions such as `Bash(*)`, `bypassPermissions`, `approval_policy = "never"`, or `danger-full-access` remove the user confirmation step. Any injected instruction can then run arbitrary commands.

## Example

Flagged:

```text
{"permissions": {"allow": ["Bash(*)"]}}
{"permissions": {"defaultMode": "bypassPermissions"}}
```

Not flagged:

```text
{"permissions": {"allow": ["Bash(npm test)", "Read"]}}
```

## How to fix

List the exact commands or tools the project needs (for example `Bash(npm run test:*)`) and keep the default approval mode. Put personal overrides in untracked local settings.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://code.claude.com/docs/en/hooks>
- <https://simonwillison.net/2025/jun/16/the-lethal-trifecta/>

## Affected tools

Claude Code, Cursor, GitHub Copilot / VS Code, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
