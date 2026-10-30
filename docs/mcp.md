# Using RuleSift from your coding agent (MCP)

`rulesift mcp` starts a read-only MCP server over stdio. Your agent can then
scan files itself, for example before it adds a skill or edits an MCP config.

Print config snippets for Cursor, Claude Code, and Codex:

```sh
rulesift init --print-mcp-config
```

RuleSift prints these snippets but never writes them for you:

- **Cursor:** `.cursor/mcp.json`
- **Claude Code:** `.mcp.json`, or `claude mcp add --scope project rulesift -- rulesift mcp`
- **Codex:** `~/.codex/config.toml`, or `codex mcp add rulesift -- rulesift mcp`

Tools (all read-only and offline; each carries `readOnlyHint: true` and `openWorldHint: false`):

| Tool | Input | Output |
|---|---|---|
| `scan_path` | `{ path = ".", kinds?, min_severity? }` | JSON report (at most 50 findings, full summary) |
| `diff_refs` | `{ base, head?, path? }` | JSON report with capability `changes` |
| `reveal_file` | `{ path }` | Hidden characters, decoded text, HTML comments |
| `explain_rule` | `{ id }` | The rule's documentation (no AI involved) |
| `list_rules` | `{}` | id, severity, title |
| `verify_lock` | `{ path? }` | Lock status and violations |

The server resolves paths inside its working directory and rejects anything
that escapes with `..`.

`scan_path` uses the `rulesift.toml` of the server's working directory (or
above it), never one inside the scanned path, so a skill you are vetting
cannot bring its own policy. `diff_refs` uses the base revision's
`rulesift.toml` and `rulesift.lock`, like `rulesift diff`, and reports
changes to them as RS-LOCK-004/005 (see docs/configuration.md, #160).

To tell your agent to use RuleSift, add a short marked block to its
instruction file:

```sh
rulesift init --agent agents    # or claude, cursor (repeatable; idempotent)
```
