# RS-FMT-005: Unknown skill frontmatter field or over-long compatibility

| Severity | Category | Applies to |
|---|---|---|
| low | format | skill |

## What it detects

Allowed SKILL.md frontmatter fields are name, description, license, compatibility, metadata, and allowed-tools (the Agent Skills spec), and `compatibility` is at most 500 characters.

Skills in Claude Code locations (`.claude/skills/` at any level, or a plugin's `skills/` directory) may also use the fields Claude Code documents: `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`, `hooks`, `paths`, and `shell`. Elsewhere (for example `.cursor/skills/` or `.agents/skills/`) those fields are reported as Claude Code extensions.

## Why it matters

A field that no agent defines has no effect, so text in it is easy to overlook in review. Claude Code extensions in a skill meant for other agents do nothing there, and Agent Skills uploads (claude.ai, the Skills API) reject any field outside the spec. Over-long `compatibility` values break the spec.

## Example

Flagged:

```text
---
name: demo
description: Demo skill for tests.
colour: red
---

```

Not flagged:

```text
---
name: demo
description: Demo skill for tests.
license: MIT
---

```

## How to fix

Remove unknown fields or move them under `metadata`; keep `compatibility` short.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://agentskills.io/specification>
- <https://code.claude.com/docs/en/skills>

## Affected tools

Agent Skills
