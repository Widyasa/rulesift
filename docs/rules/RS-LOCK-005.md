# RS-LOCK-005: RuleSift policy or lock changed

| Severity | Category | Applies to |
|---|---|---|
| medium | lock | policy |

## What it detects

`rulesift diff` (the GitHub Action's pull request mode) or MCP `diff_refs` found a change to RuleSift's policy inputs that does not weaken them:

- a `rulesift.toml` that is added, changed, or deleted without weakening the policy (for example a stricter `[score] block_at`), or that cannot be parsed;
- a `rulesift.lock` that approves changes made in the same pull request (`rulesift lock` was run on the changed files), or that is added or otherwise changed;
- a custom rule file added to the base config's `[rules] extra_dirs`.

RS-LOCK-005 is a review note: it is listed in the report and the PR comment, but does not change the exit code. The check uses the **base** revision's policy and lock.

## Why it matters

A lock is an approval. A pull request that changes a locked skill and regenerates `rulesift.lock` approves its own change; a reviewer should see that.

## Example

```text
$ rulesift diff --base origin/main .
  ! rulesift.lock  rulesift.lock approves 1 change(s) made in this change (AGENTS.md); review them (RS-LOCK-005)
```

## How to fix

Review the listed changes. Protect `rulesift.toml` and `rulesift.lock` with CODEOWNERS so that a maintainer has to approve them (see docs/ci.md).

## References

- <https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners>

## Affected tools

Every tool in docs/supported-tools.md (the policy covers all discovered agent files).
