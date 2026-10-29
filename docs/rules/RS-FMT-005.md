# RS-FMT-005: Unknown skill frontmatter field or over-long compatibility

| Severity | Category | Applies to |
|---|---|---|
| low | format | skill |

## What it detects

Allowed SKILL.md frontmatter fields are name, description, license, compatibility, metadata, and allowed-tools; `compatibility` is at most 500 characters.

## Why it matters

Unknown fields are ignored by agents, so text in them is easy to overlook in review, and over-long `compatibility` values (over 500 chars) break the spec.

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

## Affected tools

Agent Skills
