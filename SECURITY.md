# Security policy

## Reporting a vulnerability

Please report security issues **privately**, either through
[GitHub private vulnerability reporting](https://github.com/Widyasa/rulesift/security/advisories/new)
or by email to the maintainer listed on the GitHub profile. Do not open a
public issue.

Include:

- the RuleSift version (`rulesift --version`);
- the command you ran;
- a minimal, harmless reproduction file.

You should get an answer within 7 days. A fix or mitigation plan should
follow within 30 days for confirmed issues.

## What counts as a vulnerability

- RuleSift executing, fetching, or writing anything while scanning.
  `scan`, `diff`, `reveal`, `verify`, and the MCP tools are read-only and
  offline.
- Path traversal out of the working directory in `rulesift mcp`.
- `--explain` launching an agent in a write or permission-bypass mode, or
  passing hidden characters to it unescaped.
- Crashes or hangs on crafted input (denial of service in CI).
- Raw hidden characters leaking into the PR comment, SARIF, or terminal
  output, where they should appear as `<U+XXXX>`.

## What does not

A malicious file that RuleSift fails to flag (a false negative) is a
detection gap, not a vulnerability. RuleSift is a linter and change gate, not
a safety certificate. Please open a normal issue with a harmless sample, or a
private report if the technique is not public yet.

## Supported versions

Only the latest release gets security fixes.
