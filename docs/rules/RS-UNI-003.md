# RS-UNI-003: Zero-width characters inside words or runs

| Severity | Category | Applies to |
|---|---|---|
| medium | hidden-text | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

Zero-width characters split words invisibly and can hide keywords from reviewers and filters. A byte order mark at the start of the file is allowed.

## Why it matters

Zero-width characters inside words split keywords so filters miss them, or hide data between visible letters.

## Example

Flagged (hidden characters shown as `<U+XXXX>`):

```text
Never ig<U+200B>nore lint errors.
```

Not flagged:

```text
<U+FEFF>Never ignore lint errors.
```

## How to fix

Delete the zero-width characters (`rulesift reveal` shows where they are).

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://www.pillar.security/blog/new-vulnerability-in-github-copilot-and-cursor-how-hackers-can-weaponize-code-agents>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
