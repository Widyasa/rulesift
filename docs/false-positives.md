# False-positive check

`scripts/fp-check.sh` clones popular public repos at pinned commits
(`scripts/fp-repos.txt`), scans them with built-in rules, and fails if any file
reaches `block`. The `fp-check` workflow runs it weekly and on demand and
uploads the JSON reports.

Run it locally:

```sh
cargo build --release -p rulesift
scripts/fp-check.sh target/release/rulesift fp-out
```

## Results (29 Oct 2026, RuleSift 0.0.1 + the W6-T3 tuning)

| Repo | Commit | Agent files | Files with findings | Findings | Accepted | Block | Review | Info |
|---|---|---|---|---|---|---|---|---|
| anthropics/skills | `ca45fc496a34` | 386 | 10 | 11 | 0 | 0 | 0 | 10 |
| openai/skills | `49f948faa925` | 746 | 4 | 4 | 3 | 0 | 0 | 4 |
| PatrickJS/awesome-cursorrules (Cursor) | `b044f956f021` | 257 | 4 | 5 | 0 | 0 | 0 | 4 |
| microsoft/vscode (Copilot, sparse) | `8c26947d4b26` | 108 | 13 | 16 | 1 | 0 | 0 | 13 |
| google-gemini/gemini-cli (Gemini, sparse) | `718e97c2448e` | 41 | 0 | 0 | 1 | 0 | 0 | 0 |
| kirodotdev/spirit-of-kiro (Kiro, sparse) | `ff0c8c22cb40` | 3 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Total** | | **1541** | **31 (2.0%)** | **36** | **5** | **0** | **0** | **31** |

The **Info** column counts files that have findings but stay below the
`review` score. No file reaches `review` or `block`.

How the corpus is set up:

- **Full commit SHAs** are in `scripts/fp-repos.txt`.
- **Sparse repos** check out only `.github/`, the tool directories, and root
  Markdown files, plus `AGENTS.md`/`GEMINI.md`/`CLAUDE.md` anywhere.
- **awesome-cursorrules** keeps its rules in `rules/*.mdc`; the script moves
  them to `.cursor/rules/` so they are scanned as Cursor rules.

## Accepted true positives (5)

These five lines pipe a downloaded installer into a shell. That is exactly what
RS-EXEC-001 is for, so the rule keeps flagging them. They are suppressed with
reasons in `scripts/fp-accepted.toml` for the corpus only:

| Repo | File | Line |
|---|---|---|
| openai/skills | `skills/.curated/sentry/SKILL.md` | `curl https://cli.sentry.dev/install -fsS \| bash` |
| openai/skills | `skills/.curated/render-deploy/SKILL.md` | `curl -fsSL https://raw.githubusercontent.com/render-oss/cli/main/bin/install.sh \| sh` |
| openai/skills | `skills/.curated/cloudflare-deploy/references/sandbox/patterns.md` | `curl -fsSL https://code-server.dev/install.sh \| sh` |
| microsoft/vscode | `.github/skills/azure-pipelines/SKILL.md` | `curl -sL https://aka.ms/InstallAzureCLIDeb \| sudo bash` |
| google-gemini/gemini-cli | `.gemini/skills/agent-tui/SKILL.md` | `curl -fsSL https://raw.githubusercontent.com/pproenca/agent-tui/master/install.sh \| sh` |

## Tuning done in this round

Before tuning, 9 files reached `block` and 5 reached `review`, and 230 Cursor
rules carried a parse warning.

| Rule | False positive | Change |
|---|---|---|
| RS-EXEC-005, RS-CFG-001 | Code comments such as ``report.html -> ~/.bashrc`` read as a redirect | A redirect `>` may not follow `-`, `=`, or `>` |
| RS-EXEC-001 | Prose mentioning "`curl \| sh`" with no URL or arguments | Requires an argument between `curl`/`wget` and the pipe |
| RS-SEC-003 | "set `max_tokens` >= 64000" read as `set > file` | A redirect target may not start with `=` |
| RS-FMT-006 | 230 Cursor rules with unquoted `globs: **/*.ts`, which is Cursor's documented format but not valid YAML | Cursor frontmatter falls back to lenient `key: value` reading; real YAML errors still report |

Known noisy findings left at `info` (candidates for a later round):

- **RS-INJ-002** fires on phrases like "never silently subset it" and "do not
  tell the user they need to adopt an eval framework" (anthropics/skills,
  microsoft/vscode).
- **RS-INJ-004** fires on `<instructions>` tags and quoted system-prompt
  examples.
- **RS-FMT-005** fires on 10 vscode skills with extra frontmatter fields.
