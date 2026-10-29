# RS-LOCK-003: Locked agent file missing

| Severity | Category | Applies to |
|---|---|---|
| low | lock | lock |

## What it detects

`rulesift verify` found an entry in `rulesift.lock` whose file, skill directory, or MCP server no longer exists.

## Why it matters

A locked file is gone. Usually harmless, but removing a file can also remove a safety instruction.

## Example

```text
$ rulesift verify
RS-LOCK-003  .cursor/rules/old.mdc  is in rulesift.lock but no longer exists.
```

## How to fix

If the removal is intended, run `rulesift lock` and commit `rulesift.lock`.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://research.checkpoint.com/2025/cursor-vulnerability-mcpoison/>

## Affected tools

Every tool in docs/supported-tools.md (the lock covers all discovered agent files).
