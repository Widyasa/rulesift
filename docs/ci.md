# CI integrations

## GitHub Action

```yaml
# .github/workflows/rulesift.yml (also written by `rulesift init --ci github`)
name: RuleSift
on: [pull_request, push]
permissions: { contents: read, security-events: write, pull-requests: write }
jobs:
  rulesift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with: { fetch-depth: 0 }
      - uses: Widyasa/rulesift@v0.1.0
```

What the action does:

1. **Install.** It installs the release binary pinned to the action's tag,
   using the cargo-dist installer from GitHub Releases. It does not use
   `cargo install`.
2. **Run.** It runs `rulesift diff` on pull requests and `rulesift scan`
   otherwise.
3. **Upload SARIF.** It uploads SARIF to GitHub Code Scanning with
   `github/codeql-action/upload-sarif`.
4. **Comment.** It creates or updates a single PR comment, found by its
   `<!-- rulesift:pr-comment -->` marker, using `actions/github-script` and
   the automatic `GITHUB_TOKEN`.
5. **Exit.** It exits with RuleSift's exit code after the uploads.

The `rulesift` binary itself never uses the network. You don't need any
secrets.

Inputs:

| Input | Default | Meaning |
|---|---|---|
| `version` | the action's tag | RuleSift version to install, e.g. `0.1.0` |
| `install` | `release` | `source` builds the action checkout with `cargo install --path` (for testing unreleased versions) |
| `path` | `.` | What to scan |
| `mode` | `diff` on `pull_request`, else `scan` | `scan` or `diff` |
| `base` | PR base SHA | Base revision for `diff` |
| `fail-on` | `block` | `block`, `review`, `info`, `never` |
| `sarif` | `true` | Upload SARIF (needs `security-events: write`) |
| `comment` | `true` | Upsert the PR comment (needs `pull-requests: write`) |
| `args` | | Extra flags for `rulesift` |

Outputs: `exit-code`, `sarif-file`, `markdown-file`.

The action also runs on PRs from forks. There the `GITHUB_TOKEN` is
read-only, so the comment step only logs a warning. The report is always in
the job summary.

## pre-commit

```yaml
# .pre-commit-config.yaml (print it with `rulesift init --pre-commit`)
repos:
  - repo: https://github.com/Widyasa/rulesift
    rev: v0.1.0
    hooks:
      - id: rulesift          # builds from source with cargo
      # - id: rulesift-system # uses a rulesift already on PATH
```

The hook only runs on agent files (its `files` regex is generated from the
registry).

## Other CI systems

RuleSift is one static binary. Install it with the shell or PowerShell
installer from a GitHub Release, or with `npx`, `pip`, or `cargo`. Then run:

```sh
rulesift scan . --sarif rulesift.sarif --markdown rulesift.md
rulesift diff --base "$BASE_SHA" --markdown rulesift.md   # in merge/pull requests
rulesift verify                                           # if you commit rulesift.lock
```

Exit codes:

| Code | Meaning |
|---|---|
| 0 | Pass |
| 1 | Policy failure: a tier threshold was reached or lock verification failed |
| 2 | Usage, config, or git error |
