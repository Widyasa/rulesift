# RS-CFG-004: Broad allowed-tools in a skill or command

| Severity | Category | Applies to |
|---|---|---|
| low | agent-config | skill, command |

## What it detects

`allowed-tools` grants unrestricted `Bash` (or every tool), so the skill or command can run any shell command without a prompt. List specific commands, e.g. `Bash(npm test)`.

## Why it matters

`allowed-tools` lets a skill or command use tools without asking. Broad grants (all Bash, all MCP tools, write access everywhere) turn a skill that is copied from the internet into a pre-approved code runner.

## Example

Flagged:

```text
---
name: example
description: Demo.
allowed-tools: Read Bash
---

```

Not flagged:

```text
---
name: example
description: Demo.
allowed-tools: Read Bash(npm test)
---

```

## How to fix

Narrow `allowed-tools` to the specific commands the skill needs, or drop the field so the agent asks each time.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://agentskills.io/specification>
- <https://code.claude.com/docs/en/skills>

## Affected tools

Agent Skills, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline
