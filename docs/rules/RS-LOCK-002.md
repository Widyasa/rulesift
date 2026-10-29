# RS-LOCK-002: New agent file not in lock

| Severity | Category | Applies to |
|---|---|---|
| medium | lock | lock |

## What it detects

`rulesift verify` found an agent file, skill directory, or MCP server that is not in `rulesift.lock`.

## Why it matters

A new agent file appeared that was never reviewed and locked, for example a new skill or hook added in a PR.

## Example

```text
$ rulesift verify
RS-LOCK-002  .claude/skills/new-skill  New skill directory is not in rulesift.lock.
```

## How to fix

Review the new file, then run `rulesift lock` and commit `rulesift.lock`.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://research.checkpoint.com/2025/cursor-vulnerability-mcpoison/>

## Affected tools

Every tool in docs/supported-tools.md (the lock covers all discovered agent files).
