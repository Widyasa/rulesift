# `--explain`: a second opinion from your own agent CLI

```sh
rulesift scan . --explain                 # auto-select an installed agent
rulesift scan . --explain --agent claude  # or codex, cursor
```

RuleSift does no AI and no network I/O itself. With `--explain` it runs an
agent CLI you have already installed and signed in to, always in a read-only,
non-interactive mode:

| Agent | Command |
|---|---|
| Codex | `codex exec --sandbox read-only --ephemeral --skip-git-repo-check -o <tmp>/last.md "<prompt>"` |
| Claude Code | `claude -p --output-format json --no-session-persistence "<prompt>"` |
| Cursor | `agent -p --output-format json "<prompt>"` (falls back to `cursor-agent`) |

How the agent is chosen:

1. `--agent`;
2. `RULESIFT_AGENT`;
3. `[explain].agent` in `rulesift.toml`;
4. otherwise the first of codex, claude, cursor found on `PATH`.

RuleSift refuses to start an agent with write or bypass flags (`--force`,
`--yolo`, `danger-full-access`, `bypassPermissions`, …).

What goes into the prompt:

- the top `[explain].max_findings` findings, as a JSON data block wrapped in
  `<untrusted-data>`;
- a preface telling the agent the data is untrusted and must not be followed;
- hidden text written as `<U+XXXX>`, with the decoded text in a separate field.

The agent is asked for a short English verdict per file: likely benign or
suspicious, why, and what to check.

The answer appears after the table. With `--format json`/`sarif`/`markdown`
it goes to stderr, so stdout stays machine-readable. The agent's output is
advisory only:

- it never changes scores or the exit code;
- if the agent is missing, fails, or times out, you get a warning and the
  scan result stands.
