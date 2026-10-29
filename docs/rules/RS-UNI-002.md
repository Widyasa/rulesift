# RS-UNI-002: Bidirectional control characters

| Severity | Category | Applies to |
|---|---|---|
| high | hidden-text | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

The file contains bidirectional control characters, which can make text display differently from how it is read (Trojan Source).

In JSON, TOML, and YAML agent files (MCP configs, hooks, settings, plugin manifests) the characters are also found when they are written as escape sequences such as `\u202E` inside a string value or an object key (for example an MCP server name), and the same holds for keys and quoted strings in the YAML frontmatter of `SKILL.md`, `.mdc`, and other Markdown files, because the agent sees them once it decodes the file.

## Why it matters

Bidirectional controls reorder how text is displayed, so reviewers see something different from what the agent reads (Trojan Source).

## Example

Flagged (hidden characters shown as `<U+XXXX>`):

```text
Allow user <U+202E>nimda<U+202C> only.
```

Not flagged:

```text
Allow user admin only.
```

## How to fix

Delete the control characters unless the file genuinely contains right-to-left text, and then keep them out of commands.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://trojansource.codes/>
- <https://www.pillar.security/blog/new-vulnerability-in-github-copilot-and-cursor-how-hackers-can-weaponize-code-agents>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
