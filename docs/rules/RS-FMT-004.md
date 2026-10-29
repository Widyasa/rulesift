# RS-FMT-004: Missing, empty, or over-long skill description

| Severity | Category | Applies to |
|---|---|---|
| low | format | skill |

## What it detects

The skill `description` must be present, non-empty, and at most 1024 characters; agents use it to decide when to load the skill.

## Why it matters

The description is what the agent reads to decide when to use the skill. An empty one makes the skill useless; an over-long one (over 1024 chars) is a place to hide instructions.

## Example

Flagged:

```text
---
name: demo
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

Write a short description (one or two sentences, at most 1024 characters) of what the skill does and when to use it.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://agentskills.io/specification>

## Affected tools

Agent Skills
