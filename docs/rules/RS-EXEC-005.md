# RS-EXEC-005: Persistence mechanism

| Severity | Category | Applies to |
|---|---|---|
| high | dangerous-shell | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

The command installs something that keeps running after the session (cron job, service, launch agent, shell rc file, git hook, Windows Run key or scheduled task). Agents should not change machine startup behavior.

## Why it matters

Persistence (cron entries, shell rc files, launch agents, systemd units, git hooks, Windows Run keys, scheduled tasks, startup folders) keeps code running after the agent session ends, outside any review.

## Example

Flagged:

```text
(crontab -l; echo '@reboot ~/.x/run') | crontab -
echo 'export X=1' >> ~/.bashrc
schtasks /create /tn x /tr x.exe
```

Not flagged:

```text
crontab -l
systemctl status nginx
cat ~/.bashrc
a symlink report.html -> ~/.bashrc must not be followed
```

## How to fix

Do not ask an agent to install background jobs or modify shell startup files. Document the step for humans if it is really needed.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://genai.owasp.org/2025/12/09/owasp-top-10-for-agentic-applications-the-benchmark-for-agentic-security-in-the-age-of-autonomous-ai/>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
