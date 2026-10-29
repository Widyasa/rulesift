# RS-LOCK-001: Locked agent file or MCP server changed

| Severity | Category | Applies to |
|---|---|---|
| high | lock | lock |

## What it detects

`rulesift verify` (or `lock --check`, or `diff` against the working tree) found a file, skill directory, or MCP server entry whose hash differs from `rulesift.lock`.

## Why it matters

A file or MCP server you reviewed and locked has changed. MCPoison (CVE-2025-54136) worked by swapping an already-approved MCP config; a lock makes that change visible.

## Example

```text
$ rulesift verify
RS-LOCK-001  .mcp.json#docs  MCP server docs changed since it was locked.
```

## How to fix

Review the change (`rulesift diff`), then run `rulesift lock` and commit `rulesift.lock`.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://research.checkpoint.com/2025/cursor-vulnerability-mcpoison/>

## Affected tools

Every tool in docs/supported-tools.md (the lock covers all discovered agent files).
