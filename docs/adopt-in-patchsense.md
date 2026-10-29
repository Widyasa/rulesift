# Adopting RuleSift in Widyasa/patchsense

This is a ready-to-use plan. Nothing has been opened in the patchsense
repository; Widya decides whether and when to adopt it.

## Current state

The scan below was run on 29 Oct 2026 against patchsense `6b755ae55482`
(`master`), using RuleSift built from this repository:

```text
$ rulesift scan .
RuleSift v0.0.1 · scanned 2 agent files
Result: 0 block, 0 review, 0 info  ·  exit code 0
```

RuleSift only looks at the files listed in
[supported-tools.md](supported-tools.md). Today that is `AGENTS.md` and
`CLAUDE.md`. The rest of the repo (Rust, Go, PHP, Python code, `templates/`,
`tests/`) is ignored, so patchsense needs no `rulesift.toml` to start.

## Steps (after RuleSift v0.1.0 is released)

1. **Workflow.** Add `.github/workflows/rulesift.yml` with this exact content:

   ```yaml
   name: RuleSift
   on:
     pull_request:
     push:
       branches: [master]
   permissions:
     contents: read
     security-events: write
     pull-requests: write
   concurrency:
     group: rulesift-${{ github.ref }}
     cancel-in-progress: true
   jobs:
     rulesift:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v7
           with:
             fetch-depth: 0
         - uses: Widyasa/rulesift@v0.1.0
           with:
             fail-on: block
   ```

   On pull requests this runs `rulesift diff` against the PR base, uploads
   SARIF to Code Scanning, and keeps one PR comment up to date. On pushes to
   `master` it runs a full `rulesift scan`. It needs no secrets beyond the
   automatic `GITHUB_TOKEN`.

2. **Lock (recommended).** Lock the reviewed agent files so silent changes
   fail CI:

   ```sh
   rulesift lock          # writes rulesift.lock (AGENTS.md, CLAUDE.md)
   git add rulesift.lock
   ```

   From then on, any PR that edits `AGENTS.md` or `CLAUDE.md` gets an
   RS-LOCK-001 finding until someone runs `rulesift lock` again in that PR.
   That step is the explicit "I reviewed this" moment.

3. **Optional extras:**
   - Add `rulesift init --agent agents` and `rulesift init --agent claude`.
     They insert a short marked block that tells coding agents to run RuleSift
     before they edit agent config. The command is idempotent.
   - Add a pre-commit hook: `rulesift init --pre-commit` prints the snippet.

4. **Code Scanning.** Make sure it is enabled for the repository
   (Settings → Code security). It is free for public repos.

## Before the release

To try it now, without a release, copy this repo's dogfooding workflow
(`.github/workflows/rulesift.yml`). It uses `install: source` and needs a
Rust toolchain step. Point it at `Widyasa/rulesift@master` instead of `./`.
