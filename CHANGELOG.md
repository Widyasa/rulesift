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

[Unreleased]: https://github.com/Widyasa/rulesift/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Widyasa/rulesift/releases/tag/v0.1.0
