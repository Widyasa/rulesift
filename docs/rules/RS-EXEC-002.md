# RS-EXEC-002: Decode-and-execute

| Severity | Category | Applies to |
|---|---|---|
| critical | dangerous-shell | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

Instructions decode hidden data and execute it, so nobody can review what actually runs. Replace it with the plain command.

## Why it matters

Decoding a blob and executing it hides the real command from reviewers and from scanners that read text. Legitimate setup steps have no reason to be encoded.

## Example

Flagged:

```text
echo ZWNobyBoaQ== | base64 -d | sh
powershell -enc ZQBjAGgAbwAgAGgAaQAAAAAA
```

Not flagged:

```text
base64 -d notes.b64 > notes.txt
```

## How to fix

Replace the encoded payload with the plain command so it can be reviewed.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://snyk.io/blog/toxicskills-malicious-ai-agent-skills-clawhub/>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
