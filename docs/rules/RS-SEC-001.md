# RS-SEC-001: Hardcoded secret

| Severity | Category | Applies to |
|---|---|---|
| high | secrets | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

A credential (AWS key id, GitHub or Slack token, private key, or a high-entropy API key) is written in an agent file. Anyone or any agent reading the file gets it. Rotate it and use an environment variable reference instead.

## Why it matters

A real-looking secret (cloud key, token, private key) committed in agent config is exposed to every reader and stays in git history.

## Example

Flagged:

```text
Use sk-EXAMPLE7f3Kq9Zr2LmW8xYt4 for the API.
api_key = "EXAMPLEq8Zt3Lw9Xv2Rk7"
```

Not flagged:

```text
Use ${OPENAI_API_KEY} for the API.
api_key = "<your-key>"
```

## How to fix

Remove it, rotate it, and load it from the environment or a secret manager.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://docs.github.com/en/code-security/secret-scanning/introduction/about-secret-scanning>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
