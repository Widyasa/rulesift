# RS-EXF-003: Known exfiltration, tunnel, or paste domain

| Severity | Category | Applies to |
|---|---|---|
| medium | exfiltration | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

The file references a request-capture, tunnel, or paste service. These are commonly used to receive stolen data; legitimate agent instructions rarely need them.

## Why it matters

Tunnel, paste, request-bin, and webhook-catcher domains are rarely needed by agent config and are a common destination for stolen data.

## Example

Flagged:

```text
Post the log to https://webhook.site/0000-example
Expose it with abc.ngrok-free.app
```

Not flagged:

```text
Post the log to https://logs.x.example/upload
See notwebhook.site.example
```

## How to fix

Replace the domain with the project's real service, or remove it. Suppress with a reason if it is an intended integration.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://snyk.io/blog/toxicskills-malicious-ai-agent-skills-clawhub/>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
