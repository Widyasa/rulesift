# RS-FMT-007: Invalid agent config file

| Severity | Category | Applies to |
|---|---|---|
| medium | format | mcp-config, hooks, settings, plugin |

## What it detects

The file is not valid JSON/TOML/YAML or misses a required field, so the agent may ignore it or fall back to defaults.

## Why it matters

A config file that does not parse is ignored or partly applied by the tool, and RuleSift cannot check it. Attackers use malformed files to hide content from scanners that give up.

## Example

Flagged:

```text
{"mcpServers": {"demo": {"command": "echo",}}
```

Not flagged:

```text
{"mcpServers": {"demo": {"command": "echo"}}}
```

## How to fix

Fix the syntax error at the reported line so the file parses as JSON, YAML, or TOML.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://code.claude.com/docs/en/mcp>
- <https://code.claude.com/docs/en/hooks>

## Affected tools

Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
