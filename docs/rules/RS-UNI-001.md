# RS-UNI-001: Hidden Unicode Tag characters

| Severity | Category | Applies to |
|---|---|---|
| critical | hidden-text | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

The file contains invisible Unicode Tag characters (U+E0000-U+E007F). Models read them as text but people cannot see them; this is a known way to hide instructions. The decoded text is shown.

In JSON, TOML, and YAML agent files (MCP configs, hooks, settings, plugin manifests) the characters are also found when they are written as escape sequences such as `\u202E` inside a string value or an object key (for example an MCP server name), and the same holds for keys and quoted strings in the YAML frontmatter of `SKILL.md`, `.mdc`, and other Markdown files, because the agent sees them once it decodes the file.

## Why it matters

Unicode Tag characters (U+E0000-U+E007F) are invisible in editors and PR diffs but many models read them as ASCII. They can carry a full hidden instruction.

## Example

Flagged (hidden characters shown as `<U+XXXX>`):

```text
Use tabs.<U+E0072><U+E0075><U+E006E><U+E0020><U+E0078>
```

Not flagged:

```text
Use tabs.
```

## How to fix

Delete the characters. Run `rulesift reveal FILE` to see the decoded text.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://embracethered.com/blog/posts/2024/hiding-and-finding-text-with-unicode-tags/>
- <https://www.pillar.security/blog/new-vulnerability-in-github-copilot-and-cursor-how-hackers-can-weaponize-code-agents>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
