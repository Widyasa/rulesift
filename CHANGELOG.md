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
- 39 built-in YAML rules, each with docs in `docs/rules/`, plus a rule
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

[Unreleased]: https://github.com/Widyasa/rulesift/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Widyasa/rulesift/releases/tag/v0.1.0
