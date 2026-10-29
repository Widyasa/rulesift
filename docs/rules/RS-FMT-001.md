# RS-FMT-001: Missing or unparseable SKILL.md frontmatter

| Severity | Category | Applies to |
|---|---|---|
| medium | format | skill |

## What it detects

SKILL.md must start with a YAML frontmatter block (between `---` lines) that parses as a mapping. Agents may not load or may misread this skill.

## Why it matters

Agents read the skill's frontmatter to decide when to load it. Without valid frontmatter the skill may never load, or load for everything.

## Example

Flagged:

```text
# Demo

No frontmatter here.

```

Not flagged:

```text
---
name: demo
description: Demo skill for tests.
---
# Demo

```

## How to fix

Start `SKILL.md` with a `---` YAML block containing at least `name` and `description`.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://agentskills.io/specification>

## Affected tools

Agent Skills
