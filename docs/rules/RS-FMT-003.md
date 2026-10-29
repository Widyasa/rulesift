# RS-FMT-003: Skill name does not match its directory

| Severity | Category | Applies to |
|---|---|---|
| low | format | skill |

## What it detects

The skill `name` should equal the name of the directory that contains SKILL.md, so agents and users see the same identifier.

## Why it matters

The spec requires the `name` field to match the skill's directory. A mismatch is confusing and can be used to make a skill look like a different one.

## Example

Flagged:

```text
---
name: other
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

Rename the directory or the `name` field so they match.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://agentskills.io/specification>

## Affected tools

Agent Skills
