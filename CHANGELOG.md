# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - Unreleased

First public release. The release date is set by the maintainer when the tag is pushed
(see docs/release-checklist.md).

### Added

- `rulesift scan`: discovery of agent files for 13 coding agents plus
  AGENTS.md and Agent Skills, through one data-driven registry
  (docs/supported-tools.md).
- Parsers for Markdown, frontmatter, MCP configs (JSON/YAML/TOML), hooks,
  settings, and plugin manifests. A normalizer decodes Unicode Tags, base64
  payloads, and HTML comments.
- 40 built-in YAML rules, each with docs in `docs/rules/`, plus a rule
  engine and user rules (`--rules`, `rulesift rules test`).
- Risk scores and tiers (block/review/info/ok) with an always-loaded
  multiplier and a toxic-combination bonus.
- Output formats: table, JSON (schema `schemas/report.v1.json`), SARIF 2.1.0,
  and a Markdown PR comment.
- `rulesift reveal` for hidden characters and decoded text.
- Suppressions with reasons, and baselines (`--write-baseline`, `--baseline`).
- `rulesift diff` for agent capability changes between revisions, and
  `rulesift.lock` with `lock` / `verify`.
- `rulesift mcp`, a read-only MCP server with six tools.
- `scan --explain`, an optional second opinion from your own Codex, Claude
  Code, or Cursor CLI in read-only mode.
- `rulesift init`: config, GitHub workflow, pre-commit snippet, agent
  instruction block, and MCP config snippets.
- A composite GitHub Action and pre-commit hooks.
- A false-positive check on pinned public repos (docs/false-positives.md).
- Packaging: cargo-dist (shell, PowerShell, npm, Homebrew) and maturin
  (PyPI wheel).

### Fixed

Release-gate QA findings (#99–#109):

- Finding fingerprints and absolute display paths are now relative to a
  stable root (config directory, git work tree, or scanned directory), so
  baselines match however and wherever a scan runs, and the test suite passes
  in any checkout (#99, #101).
- Agent files that could not be scanned were skipped silently (exit 0,
  "scanned 0 agent files"). They now fail closed: new critical rule
  RS-FMT-008 for files larger than `max_file_size_kib`, unreadable files,
  and symlinks (or agent-location directory symlinks) pointing outside the
  scan root. Files with NUL bytes are scanned as text and also flagged.
  UTF-16 files are decoded and scanned. Discovery warnings are printed
  without `-v` (`-q` hides them) and appear in JSON (`warnings`) and SARIF
  (tool execution notifications) (#100).
- RS-EXEC-001 no longer flags `curl … | python3 -c "…"`, `| python3 -m …`,
  `| node -e …`, `| perl -pe …`, or `| python3 script.py`, where the download
  is data, not the program (#102).
- `[scan].exclude` from `rulesift.toml` is matched relative to the config
  directory and also applies to files passed explicitly; with several paths
  the config is looked up from their common directory. The pre-commit hooks
  now scan the repository with its config (`pass_filenames: false`) instead
  of the changed file names, so excluded files never block a commit (#103).
- RS-UNI-001 to RS-UNI-004 also find hidden characters written as `\uXXXX`
  escapes in JSON, TOML, and YAML string values (#104).
- RS-FMT-005 accepts the documented Claude Code skill fields
  (`disable-model-invocation`, `argument-hint`, `context`, `hooks`, …) for
  skills under `.claude/skills/` or a plugin's `skills/`, and no longer claims
  that unknown fields are ignored (#105).
- `scan` reports the real `rulesift.lock` state and violations in the JSON
  `lock` object instead of always `present: false`; `lock` is `null` when the
  lock was not checked (files, several paths, `diff --head`) (#106).
- CONTRIBUTING lists the test prerequisites: Rust 1.89+, `git`, and `bash`
  (#107).
- The "How to suppress" link in Markdown reports and PR comments points to
  `docs/configuration.md#suppressing-findings` (#108).
- The `rulesift` and `rulesift-core` crates include `LICENSE-MIT` and
  `LICENSE-APACHE` (#109).
- MCP tools are annotated `readOnlyHint: true` / `openWorldHint: false`; the
  Action's PR-comment step only updates its own `github-actions[bot]` comment;
  a missing `--config` or `--baseline` file prints the OS error once; unknown
  rule ids in `--disable`, `[rules].disable`, or `severity_override` print a
  warning unless `-q` (#120).
- A directory symlink pointing outside the scan root fails closed with
  RS-FMT-008 wherever agent files could live, derived from the registry
  rather than a fixed probe list: this now covers symlinked skill
  directories (`.claude/skills`, `.claude/skills/<name>`, a skill's
  `scripts/`), `.claude`, `.github/instructions`, and the other tool
  directories (#122).
- Text files inside a skill directory (scripts, references) no longer pass
  silently when a NUL byte is added or they are padded past
  `max_file_size_kib`: they are scanned as text with RS-FMT-008, or reported
  as RS-FMT-008 when too large or unreadable. Only binary assets (judged by
  content, not just the extension) are skipped with a warning (#123).
- RS-EXEC-001 again flags downloads piped into an inline program that
  executes stdin (`python3 -c "exec(sys.stdin.read())"`, `node -e
  "eval(…readFileSync(0))"`, `perl -e 'eval …<STDIN>'`), scripts read from
  `/dev/stdin`, and `pip install -r /dev/stdin`, while data-only inline
  programs such as `python3 -c "…json.load(sys.stdin)…"` stay quiet (#124).
- RS-UNI-001 to RS-UNI-004 also find escaped hidden characters in object
  keys (MCP server names) of JSON, TOML, and YAML files and in frontmatter
  keys and strings; a character written raw is still reported once (#125).
- RS-CFG-002 checks hooks declared in the `hooks:` frontmatter of Claude
  Code skills and subagents, like hooks in `settings.json` (#126).

Release-gate QA findings on 73c04df (#132–#138):

- `rulesift diff`, the Action's default on pull requests, now classifies
  both revisions with the same discovery as `scan`: changed skill
  supporting files (scripts, references) are scanned, oversized,
  unreadable, and NUL-padded files get RS-FMT-008, and symlinks pointing
  outside the repository (mode 120000 in git) get RS-FMT-008 where `scan`
  gives it. `[scan]` include/exclude globs are matched the same way as in
  `scan`, and new discovery warnings are reported. A dangling symlink whose
  target lies outside the root also fails closed at an agent location
  instead of being skipped as broken (#132, #133).
- A directory symlink pointing outside the scan root at `.agents` or at a
  nested tool directory (`pkg/.claude`, `a/b/.cursor`) now fails closed with
  RS-FMT-008, since skills below them are discovered (#134).
- A file or directory symlink inside a skill directory that points outside
  the scan root (`scripts/setup.sh -> /outside/setup.sh`) fails closed with
  RS-FMT-008 instead of a warning (#135).
- Files inside a skill directory that are not text are skipped only when
  they are confidently a real binary format (magic bytes plus a structure
  check, or magic bytes matching the extension). A script padded with NUL
  bytes or glued behind a PNG header is scanned and reported as RS-FMT-008,
  and the printable text of real binaries is still checked (#136).
- RS-EXEC-001 also flags `| /bin/bash`, `| /usr/bin/env bash`, `|"bash"`,
  `| tee file | bash`, `eval "$(curl …)"`, `source <(curl …)`,
  `. <(curl …)`, `| deno run -`, `| bun run -`, and `| php`. An inline
  program that only mentions `exec` in a string literal
  (`print('exec' in d)`) is no longer flagged (#137).
- Skills and subagents whose frontmatter declares hooks get the
  always-loaded multiplier (1.5), like `settings.json`, and an RS-CFG-002
  finding raises its file to at least `review` in every location (#138).

Release-gate QA findings on 7edbf51 (#145–#147):

- `.gitignore`, `.ignore`, and `.git/info/exclude` no longer hide files
  tracked in git: `scan` and the working-tree `diff` (the Action's pull
  request mode) add every tracked file under the scanned path to the walk,
  so a PR cannot hide a changed agent file by also ignoring it (#145).
- A git submodule at an agent location (`.claude/skills/evil`, a skill's
  `scripts/`) fails closed with RS-FMT-008 in `scan` and in both `diff`
  modes, whether or not it is checked out; other submodules are skipped
  with a warning. `diff` now prints its discovery warnings like `scan`
  (#146).
- A binary in a skill directory larger than `max_file_size_kib` is no longer
  skipped on magic bytes and extension alone: it is read in full (up to
  64 MiB, above that RS-FMT-008) and must pass the structure check, so a
  PNG header glued onto a large script fails closed. The printable text
  checked in skipped binaries now drops NUL bytes as bash does, so a
  NUL-split command in a PNG text chunk is found (#147).
- docs: RS-EXEC-001 lists download-then-run (`curl … -o i.sh && bash i.sh`)
  under "Not flagged" and explains why (#149).

Release-gate QA findings on b256f0f (#155, #156):

- Ignore files no longer hide agent files that are not tracked in git
  either (a copied third-party skill with its own `.ignore` or
  `.gitignore`, a gitignored `.claude/settings.local.json`), inside a git
  repository or not: `scan`, the working-tree `diff`, and MCP `scan_path`
  find them with a second walk without ignore rules and scan them with a
  warning. Only `exclude` skips files (by default `node_modules/**` and
  `target/**`). An unreadable git index is reported with a warning instead
  of silently falling back (#155).
- A symlink at an agent location that resolves into a git submodule placed
  elsewhere (`.claude/skills/evil -> ../../vendor/evil`, `AGENTS.md ->
  vendor/evil/SKILL.md`) fails closed with RS-FMT-008 in `scan` and both
  `diff` modes, whether or not the submodule is checked out; a symlink into
  a submodule from elsewhere is skipped with a warning (#156).

Release-gate QA findings on cc716fb (#160, #161):

- Ignored `node_modules` directories at any depth (a monorepo's
  `packages/*/node_modules` after `npm ci`) are no longer walked, so
  third-party package docs that look like agent files (`SKILL.md`,
  `AGENTS.md`) no longer fail the scan; this was a side effect of #158.
  A `node_modules` inside an agent location (below `.claude` and other
  hidden directories, or in a skill directory) is still scanned, and so
  are files tracked in git. A symlink at an agent location into an
  excluded path or a dependency directory (installed or not) fails closed
  with RS-FMT-008 (#161).

[Unreleased]: https://github.com/Widyasa/rulesift/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Widyasa/rulesift/releases/tag/v0.1.0
