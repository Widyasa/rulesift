# RS-LOCK-004: RuleSift policy weakened

| Severity | Category | Applies to |
|---|---|---|
| high | lock | policy |

## What it detects

`rulesift diff` (the GitHub Action's pull request mode) or MCP `diff_refs` found a change to RuleSift's own policy that can hide a file or a finding:

- a `rulesift.toml` (anywhere in the repository) that is added, changed, or deleted so that it adds `exclude` globs, limits `include`, `paths`, `kinds`, or `tools`, adds `[rules] disable` ids, lowers a severity with `severity_override`, drops `extra_dirs`, adds `[[suppress]]` entries, raises `[score]` thresholds or lowers weights, loosens `[policy] fail_on`, or raises `min_severity`;
- a custom rule file in the base config's `[rules] extra_dirs` that is changed or deleted;
- a `rulesift.lock` that is deleted, or that drops entries for files, skills, or MCP servers that still exist.

The check itself always uses the **base** revision's `rulesift.toml` and `rulesift.lock`, so the change cannot switch off the gate that checks it. Like the other lock findings, RS-LOCK-004 makes `diff` exit 1 regardless of `--fail-on`.

## Why it matters

The author of a pull request controls its files, including the policy file. One added `rulesift.toml` with `exclude = [".claude/**"]` or `[rules] disable = ["RS-EXEC-001"]` would otherwise turn the gate off for that pull request and every later one.

## Example

```text
$ rulesift diff --base origin/main .
  ! rulesift.toml  rulesift.toml is added and weakens the RuleSift policy: scan.exclude adds `.claude/**`; the base revision's policy was used (RS-LOCK-004)
```

## How to fix

If the weaker policy is intended, have a maintainer review the change and merge it; from then on the new policy applies. Protect `rulesift.toml`, `rulesift.lock`, and custom rule directories with CODEOWNERS (see docs/ci.md).

## References

- <https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners>

## Affected tools

Every tool in docs/supported-tools.md (the policy covers all discovered agent files).
