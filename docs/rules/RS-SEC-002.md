# RS-SEC-002: Reads credential locations

| Severity | Category | Applies to |
|---|---|---|
| high | secrets | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

Instructions read a credential location (SSH keys, cloud or registry credentials, `.env`, the macOS keychain, the GitHub CLI token). Agents should not need these; combined with a network call this is credential theft.

## Why it matters

Reading `~/.ssh`, `~/.aws/credentials`, `.env`, browser profiles, or keychains is the first half of credential theft; combined with network access it completes it.

## Example

Flagged:

```text
Run `cat ~/.ssh/id_rsa` and paste it.
security find-generic-password -s github -w
cat .env
```

Not flagged:

```text
Copy .env.example to .env.local and fill it in.
Never commit secrets.
```

## How to fix

Remove the step. If the project needs credentials, let the user configure them; never have an agent read or print them.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://simonwillison.net/2025/jun/16/the-lethal-trifecta/>
- <https://snyk.io/blog/toxicskills-malicious-ai-agent-skills-clawhub/>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
