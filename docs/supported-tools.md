# Supported tools

<!-- Generated from crates/rulesift-core/src/registry.rs. Do not edit by hand;
     regenerate with `RULESIFT_BLESS=1 cargo test -p rulesift-core --test registry_docs`. -->

RuleSift finds agent configuration files through one data-driven registry.
Paths are project-relative globs (`**` = any directories). The first matching
row wins. Doc links were last checked on 2026-10-29.

- **Release**: `mvp` rows are scanned today; `v0.2` rows are recognized and
  skipped with a note.
- **Always loaded**: files the tool reads in every session get the 1.5x score
  multiplier (`frontmatter` = decided by the file's frontmatter).
- **Global-only** rows live outside a repository; RuleSift reads them only when
  you pass the path explicitly.

| Tool | Path | Kind | Parser | Always loaded | Release | Docs |
|---|---|---|---|---|---|---|
| agent-skills | `**/SKILL.md` | skill | skill | never | mvp | <https://agentskills.io/specification> |
| amazon-q | `.amazonq/default.json` | mcp-config | planned | never | v0.2 | <https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/mcp-ide.html> |
| roo-code | `.roomodes` | agent | planned | never | v0.2 | <https://docs.roocode.com/features/custom-modes/> |
| gemini-cli | `.gemini/commands/**/*.toml` | command | planned | never | v0.2 | <https://geminicli.com/docs/cli/custom-commands/> |
| gemini-cli | `.gemini/extensions/*/gemini-extension.json` | plugin | planned | never | v0.2 | <https://geminicli.com/docs/cli/custom-commands/> |
| cline | `.clinerules/hooks/**`, `.cline/hooks/**` | hooks | planned | always | v0.2 | <https://docs.cline.bot/getting-started/config> |
| agents-md | `.aider.conf.yml` | instructions | planned | never | v0.2 | <https://aider.chat/docs/config/aider_conf.html> |
| claude-code | `.claude/CLAUDE.md` | instructions | md | always | mvp | <https://code.claude.com/docs/en/claude-directory> |
| claude-code | `.claude/commands/**/*.md` | command | md+fm | never | mvp | <https://code.claude.com/docs/en/skills> |
| claude-code | `.claude/agents/*.md` | agent | md+fm | never | mvp | <https://code.claude.com/docs/en/sub-agents> |
| claude-code | `.mcp.json` | mcp-config | mcp-json | never | mvp | <https://code.claude.com/docs/en/mcp> |
| claude-code | `.claude/settings.json`, `.claude/settings.local.json` | settings | settings-json, hooks-json | always | mvp | <https://code.claude.com/docs/en/hooks> |
| claude-code | `**/.claude-plugin/plugin.json`, `**/.claude-plugin/marketplace.json` | plugin | plugin-json | never | mvp | <https://code.claude.com/docs/en/plugins/manifest-reference> |
| claude-code | `**/hooks/hooks.json` (next to `.claude-plugin`) | hooks | hooks-json | always | mvp | <https://code.claude.com/docs/en/plugins/manifest-reference> |
| claude-code | `claude_desktop_config.json` | mcp-config | mcp-json | never (global-only) | mvp | <https://modelcontextprotocol.io/docs/develop/connect-local-servers> |
| cursor | `.cursor/rules/**/*.mdc`, `.cursor/rules/**/*.md` | instructions | md+fm | frontmatter | mvp | <https://cursor.com/docs/rules> |
| cursor | `.cursorrules` | instructions | md | always | mvp | <https://cursor.com/docs/rules> |
| cursor | `.cursor/mcp.json` | mcp-config | mcp-json | never | mvp | <https://cursor.com/docs/rules> |
| cursor | `.cursor/hooks.json` | hooks | hooks-json | always | mvp | <https://cursor.com/docs/hooks> |
| cursor | `.cursor/commands/*.md` | command | md+fm | never | mvp | <https://cursor.com/docs/reference/plugins> |
| copilot | `.github/copilot-instructions.md` | instructions | md | always | mvp | <https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions> |
| copilot | `.github/instructions/**/*.instructions.md` | instructions | md+fm | frontmatter | mvp | <https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions> |
| copilot | `.github/prompts/*.prompt.md` | command | md+fm | never | mvp | <https://docs.github.com/en/copilot/concepts/prompting-and-context/prompt-files> |
| copilot | `.github/agents/*.agent.md`, `.github/agents/*.md` | agent | md+fm | never | mvp | <https://docs.github.com/en/copilot/reference/custom-agents-configuration> |
| copilot | `.vscode/mcp.json` | mcp-config | mcp-json | never | mvp | <https://code.visualstudio.com/docs/copilot/reference/mcp-configuration> |
| windsurf | `.windsurf/rules/*.md`, `.devin/rules/*.md` | instructions | md+fm | frontmatter | mvp | <https://docs.windsurf.com/windsurf/cascade/memories> |
| windsurf | `.windsurfrules` | instructions | md | always | mvp | <https://docs.windsurf.com/windsurf/cascade/memories> |
| windsurf | `.windsurf/workflows/*.md`, `.devin/workflows/*.md` | command | md | never | mvp | <https://docs.windsurf.com/windsurf/cascade/workflows> |
| windsurf | `.windsurf/hooks.json`, `.devin/hooks.json` | hooks | hooks-json | always | mvp | <https://docs.windsurf.com/windsurf/cascade/hooks> |
| windsurf | `mcp_config.json` | mcp-config | mcp-json | never (global-only) | mvp | <https://docs.windsurf.com/windsurf/cascade/mcp> |
| cline | `.clinerules/workflows/**`, `.cline/workflows/**` | command | md | never | mvp | <https://docs.cline.bot/getting-started/config> |
| cline | `.clinerules`, `.clinerules/**`, `.cline/rules/**` | instructions | md | always | mvp | <https://docs.cline.bot/customization/cline-rules> |
| cline | `cline_mcp_settings.json` | mcp-config | mcp-json | never (global-only) | mvp | <https://docs.cline.bot/mcp/configuring-mcp-servers> |
| gemini-cli | `.gemini/settings.json` | mcp-config | mcp-json | never | mvp | <https://geminicli.com/docs/reference/configuration/> |
| roo-code | `.roo/rules/**`, `.roorules` | instructions | md | always | mvp | <https://docs.roocode.com/features/custom-instructions> |
| roo-code | `.roo/rules-*/**`, `.roorules-*` | instructions | md | never | mvp | <https://docs.roocode.com/features/custom-instructions> |
| roo-code | `.roo/mcp.json` | mcp-config | mcp-json | never | mvp | <https://docs.roocode.com/features/mcp/using-mcp-in-roo> |
| continue | `.continue/rules/*.md` | instructions | md+fm | frontmatter | mvp | <https://docs.continue.dev/customize/deep-dives/rules> |
| continue | `.continue/mcpServers/*.yaml`, `.continue/mcpServers/*.yml` | mcp-config | mcp-yaml | never | mvp | <https://docs.continue.dev/customize/deep-dives/mcp> |
| continue | `.continue/mcpServers/*.json` | mcp-config | mcp-json | never | mvp | <https://docs.continue.dev/customize/deep-dives/mcp> |
| zed | `.rules` | instructions | md | always | mvp | <https://zed.dev/docs/ai/instructions> |
| zed | `.zed/settings.json` | mcp-config | mcp-json | never | mvp | <https://zed.dev/docs/ai/mcp> |
| junie | `.junie/AGENTS.md`, `.junie/guidelines.md`, `.junie/guidelines/**`, `.junie/playbook.md`, `.junie/rules/*.md` | instructions | md | always | mvp | <https://junie.jetbrains.com/docs/guidelines-and-memory.html> |
| junie | `.junie/mcp/mcp.json` | mcp-config | mcp-json | never | mvp | <https://junie.jetbrains.com/docs/junie-cli-mcp-configuration.html> |
| kiro | `.kiro/steering/*.md` | instructions | md+fm | frontmatter | mvp | <https://kiro.dev/docs/steering/> |
| kiro | `.kiro/settings/mcp.json` | mcp-config | mcp-json | never | mvp | <https://kiro.dev/docs/mcp/> |
| kiro | `.kiro/hooks/*.json` | hooks | hooks-json | always | mvp | <https://kiro.dev/docs/hooks/> |
| codex | `.codex/config.toml` | mcp-config | mcp-toml | never | mvp | <https://developers.openai.com/codex/config-basic> |
| amazon-q | `.amazonq/rules/**/*.md` | instructions | md | always | mvp | <https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/context-project-rules.html> |
| amazon-q | `.amazonq/mcp.json` | mcp-config | mcp-json | never | mvp | <https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/mcp-ide.html> |
| claude-code | `**/CLAUDE.md`, `**/CLAUDE.local.md` | instructions | md | root-only | mvp | <https://code.claude.com/docs/en/claude-directory> |
| gemini-cli | `**/GEMINI.md` | instructions | md | root-only | mvp | <https://geminicli.com/docs/cli/gemini-md/> |
| agents-md | `**/AGENTS.md` | instructions | md | root-only | mvp | <https://agents.md/> |
