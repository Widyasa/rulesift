# RS-FMT-002: Invalid skill name

| Severity | Category | Applies to |
|---|---|---|
| medium | format | skill |

## What it detects

The skill `name` must be 1-64 characters of lowercase letters, digits, and hyphens, with no leading, trailing, or double hyphen.

## Why it matters

The spec limits names to lowercase letters, digits, and hyphens (1-64 chars). Invalid names break loading and can impersonate other skills.

## Example

Flagged:

```text
---
name: Demo--Skill
description: Demo skill for tests.
---

```

Not flagged:

```text
---
name: demo
description: Demo skill for tests.
---

```

## How to fix

Rename the skill to match `^[a-z0-9]+(-[a-z0-9]+)*$`, at most 64 characters.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://agentskills.io/specification>

## Affected tools

Agent Skills
