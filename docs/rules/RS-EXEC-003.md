# RS-EXEC-003: Password-protected archive extracted then executed

| Severity | Category | Applies to |
|---|---|---|
| high | dangerous-shell | skill, instructions, command, agent, hooks, settings, plugin, mcp-config |

## What it detects

A password-protected archive is extracted and its content executed. Passwords keep scanners from looking inside; this is a common malware delivery trick.

## Why it matters

Password-protected archives cannot be inspected by antivirus or code review. Extracting one and running its content is a known malware delivery pattern used in malicious skills.

## Example

Flagged:

```text
unzip -P infected tool.zip && chmod +x tool && ./tool
```

Not flagged:

```text
unzip tool.zip && ./tool --help
7z x -pdocs manual.7z
```

## How to fix

Ship the tool unpacked in the repo or download a signed release and verify it. Never execute content from an encrypted archive.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://snyk.io/blog/toxicskills-malicious-ai-agent-skills-clawhub/>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
