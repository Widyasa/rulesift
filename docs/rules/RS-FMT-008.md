# RS-FMT-008: Agent file could not be scanned

| Severity | Category | Applies to |
|---|---|---|
| critical | format | skill, instructions, command, agent, mcp-config, hooks, settings, plugin |

## What it detects

A file at an agent-file location that RuleSift could not fully scan:

- it is larger than `max_file_size_kib` (default 2048 KiB);
- it cannot be read (for example permission denied);
- it is a symlink, or a directory symlink at an agent-file location, that
  points outside the scan root. A directory counts when agent files could
  live in it: any directory on a registry path (`.claude`, `.cursor/rules`,
  `.cursor/rules/shared`, `.github/instructions`, a plugin's
  `.claude-plugin`, …) and any directory at or below a `skills` directory
  (`.claude/skills`, `.claude/skills/<name>`, `skills/<name>/scripts`).
  Tool directories count at any depth (`.agents`, `pkg/.claude`,
  `a/b/.cursor`), since skills below them are discovered. Inside a skill
  directory every file or directory symlink counts (`scripts/setup.sh`
  pointing outside the repo hides a script the skill runs).
  Other directory symlinks (`vendor`, `node_modules`) are skipped with a
  warning;
- it contains NUL bytes. The file is still scanned as text, so other rules
  fire on its content too.

Files inside a skill directory count as agent files here when they are text
an agent may run or read: a script or text extension (`.sh`, `.py`, `.js`,
`.md`, `.json`, …), a `#!` shebang, or content that is almost all printable
(so `bash setup.png` cannot hide a script). Real binary assets there, such as
images, fonts, and archives, are skipped with a warning.

UTF-16 files (with a BOM, or plain ASCII text saved as UTF-16) are decoded
and scanned normally; they do not trigger this rule.

## Why it matters

A scanner that silently skips files it cannot read is trivial to bypass: add
one NUL byte, pad the file past the size limit, or point a symlink outside
the repo, and a malicious `AGENTS.md` passes the gate while the agent still
loads it. RuleSift fails closed instead. The finding is critical, so with the
default policy (`fail_on = "block"`) the scan exits 1.

## Example

Flagged (`<U+0000>` is a NUL byte):

```text
# Project

Use tabs.
<!-- <U+0000> -->
```

Not flagged:

```text
# Project

Use tabs.
```

## How to fix

- Remove NUL bytes and save the file as UTF-8.
- Split or shrink very large agent files, or raise `[scan].max_file_size_kib`
  in `rulesift.toml`.
- Make the file readable by the user running RuleSift (for example in CI).
- Replace symlinks that leave the repository with the real file, or scan the
  directory that contains the target.

To accept a finding on purpose, add a `[[suppress]]` entry with a `reason` to `rulesift.toml`, or record it in a baseline (`rulesift scan --baseline`); see docs/configuration.md. `--disable RS-FMT-008` turns the check off entirely (not recommended).

## References

- <https://agents.md/>
- <https://github.com/Widyasa/rulesift/issues/100>

## Affected tools

All supported tools.
