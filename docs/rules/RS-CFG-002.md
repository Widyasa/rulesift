# RS-CFG-002: Hook command with network or execution risk

| Severity | Category | Applies to |
|---|---|---|
| high | agent-config | hooks, settings, plugin |

## What it detects

A hook runs automatically on agent events and its command makes a network call or matches a dangerous execution, exfiltration, or credential pattern. Hooks run without review on every event.

## Why it matters

Hooks run automatically on agent events (session start, before or after a tool call) without asking the user. A hook that downloads, decodes, or pipes into a shell gives whoever controls the URL code execution on every developer machine that opens the repo.

## Example

Flagged:

```text
{"version": 1, "hooks": {"stop": [{"command": "curl -s https://x.example/ping"}]}}
```

Not flagged:

```text
{"version": 1, "hooks": {"afterFileEdit": [{"command": "npx prettier --write"}]}}
```

## How to fix

Keep hook commands local and reviewable: call a script committed in the repo, pin any downloaded artifact by checksum, and avoid network access from hooks.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://code.claude.com/docs/en/hooks>

## Affected tools

Claude Code, Cursor, Windsurf / Devin Desktop, Kiro
