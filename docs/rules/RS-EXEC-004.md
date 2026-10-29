# RS-EXEC-004: Destructive command

| Severity | Category | Applies to |
|---|---|---|
| high | dangerous-shell | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

The command can wipe the home directory, the whole filesystem, or a disk. Agents may run it without understanding the impact; scope it to a project path.

## Why it matters

Commands such as `rm -rf ~`, `rm -rf /`, `mkfs`, `dd` onto a disk device, `chmod -R 777 /`, or a fork bomb destroy data or the machine. An agent following instructions does not know which deletions are safe.

## Example

Flagged:

```text
rm -rf ~
sudo rm -rf / --no-preserve-root
dd if=/dev/zero of=/dev/sda
```

Not flagged:

```text
rm -rf build
rm -rf ./node_modules
rm -rf /tmp/rulesift-test
```

## How to fix

Scope deletions to build output inside the project (for example `rm -rf ./target`) or remove the step.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md.

## References

- <https://genai.owasp.org/2025/12/09/owasp-top-10-for-agentic-applications-the-benchmark-for-agentic-security-in-the-age-of-autonomous-ai/>

## Affected tools

Agent Skills, AGENTS.md, Claude Code, Cursor, GitHub Copilot / VS Code, Windsurf / Devin Desktop, Cline, Gemini CLI, Roo Code, Continue, Zed, JetBrains Junie, Kiro, OpenAI Codex, Amazon Q Developer
