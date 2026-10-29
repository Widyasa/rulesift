# RS-FMT-006: Invalid frontmatter for the tool's dialect

| Severity | Category | Applies to |
|---|---|---|
| low | format | instructions, command, agent |

## What it detects

A frontmatter field has the wrong type or value for this tool (for example Cursor `alwaysApply` must be a boolean, Kiro `inclusion` must be always, fileMatch, manual, or auto). The tool may ignore or misapply the file.

## Why it matters

Each tool reads specific frontmatter keys (`alwaysApply`, `globs`, `applyTo`, `inclusion`, …). Wrong types or unknown values silently change when a rule loads, for example making it load everywhere.

## Example

Flagged:

```text
---
description: Demo
alwaysApply: "yes"
---
Use tabs.

```

Not flagged:

```text
---
description: Demo
alwaysApply: true
---
Use tabs.

```

## How to fix

Fix the field per the tool's docs (see docs/supported-tools.md for links).

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://cursor.com/docs/rules>
- <https://kiro.dev/docs/steering/>

## Affected tools

AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, Amazon Q Developer
