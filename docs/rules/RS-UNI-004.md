# RS-UNI-004: Variation selectors or invisible format characters in runs

| Severity | Category | Applies to |
|---|---|---|
| medium | hidden-text | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

Runs of variation selectors or other invisible format characters can carry hidden data inside otherwise normal text.

## Why it matters

Runs of variation selectors or invisible format characters can encode hidden data (for example inside an emoji) that reviewers cannot see.

## Example

Flagged (hidden characters shown as `<U+XXXX>`):

```text
Done.<U+FE00><U+FE01><U+FE02><U+FE03><U+FE04><U+FE05><U+FE06><U+FE07>
```

Not flagged:

```text
Done.
```

## How to fix

Delete the invisible characters.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://embracethered.com/blog/posts/2024/hiding-and-finding-text-with-unicode-tags/>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
