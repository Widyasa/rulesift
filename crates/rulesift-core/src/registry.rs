//! Static, data-driven tool registry (PRD §5.1).
//!
//! Every supported path is one [`RegistryRow`]. Supporting a new tool means
//! adding a row (and fixtures); discovery, parsing, and rules read the table
//! and contain no per-tool path logic.

use crate::model::{string_enum, FileKind, Tool};
use crate::{CoreError, Result};
use globset::{Glob, GlobMatcher};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::OnceLock;

string_enum! {
    /// Shared parser family used for a row.
    pub enum ParserFamily {
        /// Markdown.
        Md => "md",
        /// YAML frontmatter + Markdown.
        MdFm => "md+fm",
        /// Agent Skills `SKILL.md` (frontmatter + Markdown + skill validation).
        Skill => "skill",
        /// Plain text (scripts and other files inside a skill directory).
        Text => "text",
        /// MCP servers in JSON (`mcpServers` / `servers` / `context_servers`).
        McpJson => "mcp-json",
        /// MCP servers in TOML (`[mcp_servers.*]`).
        McpToml => "mcp-toml",
        /// MCP servers in YAML (Continue block files).
        McpYaml => "mcp-yaml",
        /// Hook definitions in JSON.
        HooksJson => "hooks-json",
        /// Settings JSON (permissions, trust).
        SettingsJson => "settings-json",
        /// Plugin / marketplace manifest JSON.
        PluginJson => "plugin-json",
        /// Not implemented yet (v0.2 rows).
        Planned => "planned",
    }
}

string_enum! {
    /// Always-loaded predicate id (PRD §5.1, used by scoring §9).
    pub enum AlwaysLoaded {
        /// Always loaded into the agent context.
        Always => "always",
        /// Loaded on demand only.
        Never => "never",
        /// Always loaded only when the file sits at the scan root.
        RootOnly => "root-only",
        /// Decided by the tool's frontmatter dialect (e.g. `alwaysApply: true`).
        Frontmatter => "frontmatter",
    }
}

string_enum! {
    /// Release in which a row is enabled.
    pub enum Release {
        /// Enabled in the MVP.
        Mvp => "mvp",
        /// Present but disabled until v0.2.
        V0_2 => "v0.2",
    }
}

string_enum! {
    /// Frontmatter dialect of `md+fm` rows (validated in `formats::dialects`).
    pub enum Dialect {
        /// Agent Skills `SKILL.md`.
        AgentSkill => "agent-skill",
        /// Cursor `.mdc` rules (`description`, `globs`, `alwaysApply`).
        CursorRule => "cursor-rule",
        /// Cursor commands.
        CursorCommand => "cursor-command",
        /// Continue rules (`globs`, `alwaysApply`).
        ContinueRule => "continue-rule",
        /// Copilot path instructions (`applyTo`).
        CopilotInstructions => "copilot-instructions",
        /// Copilot prompt files.
        CopilotPrompt => "copilot-prompt",
        /// Copilot custom agents (`tools`, `mcp-servers`).
        CopilotAgent => "copilot-agent",
        /// Kiro steering (`inclusion`).
        KiroSteering => "kiro-steering",
        /// Windsurf / Devin rules (`trigger`).
        WindsurfRule => "windsurf-rule",
        /// Claude Code sub-agents (`name`, `description`, `tools`, ...).
        ClaudeAgent => "claude-agent",
        /// Claude Code commands (`allowed-tools`, ...).
        ClaudeCommand => "claude-command",
    }
}

/// One supported path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegistryRow {
    /// Stable row id.
    pub id: &'static str,
    /// Tool.
    pub tool: Tool,
    /// Primary kind reported for the file.
    pub kind: FileKind,
    /// Path globs relative to the scan root (`/` separated, `*` does not cross `/`).
    pub globs: &'static [&'static str],
    /// Parser families applied to the file, in order.
    pub parsers: &'static [ParserFamily],
    /// Frontmatter dialect, if any.
    pub dialect: Option<Dialect>,
    /// Always-loaded predicate.
    pub always: AlwaysLoaded,
    /// Release.
    pub release: Release,
    /// Official documentation URL.
    pub doc_url: &'static str,
    /// Only matched for paths given explicitly (user-global configs).
    pub global_only: bool,
    /// The file only counts if this sibling exists in the parent of the
    /// file's directory (e.g. plugin `hooks/hooks.json` next to `.claude-plugin/`).
    pub requires_sibling: Option<&'static str>,
}

const fn row(
    id: &'static str,
    tool: Tool,
    kind: FileKind,
    globs: &'static [&'static str],
    parsers: &'static [ParserFamily],
    always: AlwaysLoaded,
    doc_url: &'static str,
) -> RegistryRow {
    RegistryRow {
        id,
        tool,
        kind,
        globs,
        parsers,
        dialect: None,
        always,
        release: Release::Mvp,
        doc_url,
        global_only: false,
        requires_sibling: None,
    }
}

const fn with_dialect(mut r: RegistryRow, d: Dialect) -> RegistryRow {
    r.dialect = Some(d);
    r
}

const fn v0_2(mut r: RegistryRow) -> RegistryRow {
    r.release = Release::V0_2;
    r
}

const fn global(mut r: RegistryRow) -> RegistryRow {
    r.global_only = true;
    r
}

const fn sibling(mut r: RegistryRow, s: &'static str) -> RegistryRow {
    r.requires_sibling = Some(s);
    r
}

use AlwaysLoaded as A;
use FileKind as K;
use ParserFamily as P;
use Tool as T;

const SKILLS_DOC: &str = "https://agentskills.io/specification";
const CLAUDE_DIR_DOC: &str = "https://code.claude.com/docs/en/claude-directory";
const COPILOT_INSTR_DOC: &str =
    "https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions";

/// The registry. Order matters: the first matching row wins, so specific and
/// disabled (v0.2) rows come before broad ones.
pub static ROWS: &[RegistryRow] = &[
    // --- Agent Skills -----------------------------------------------------
    with_dialect(
        row(
            "agent-skills",
            T::AgentSkills,
            K::Skill,
            &["**/SKILL.md"],
            &[P::Skill],
            A::Never,
            SKILLS_DOC,
        ),
        Dialect::AgentSkill,
    ),
    // --- v0.2 rows (disabled; listed first so they shadow broad rows) -----
    v0_2(row(
        "amazon-q-default",
        T::AmazonQ,
        K::McpConfig,
        &[".amazonq/default.json"],
        &[P::Planned],
        A::Never,
        "https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/mcp-ide.html",
    )),
    v0_2(row(
        "roo-modes",
        T::RooCode,
        K::Agent,
        &[".roomodes"],
        &[P::Planned],
        A::Never,
        "https://docs.roocode.com/features/custom-modes/",
    )),
    v0_2(row(
        "gemini-commands",
        T::GeminiCli,
        K::Command,
        &[".gemini/commands/**/*.toml"],
        &[P::Planned],
        A::Never,
        "https://geminicli.com/docs/cli/custom-commands/",
    )),
    v0_2(row(
        "gemini-extensions",
        T::GeminiCli,
        K::Plugin,
        &[".gemini/extensions/*/gemini-extension.json"],
        &[P::Planned],
        A::Never,
        "https://geminicli.com/docs/cli/custom-commands/",
    )),
    v0_2(row(
        "cline-hooks",
        T::Cline,
        K::Hooks,
        &[".clinerules/hooks/**", ".cline/hooks/**"],
        &[P::Planned],
        A::Always,
        "https://docs.cline.bot/getting-started/config",
    )),
    v0_2(row(
        "aider-conf",
        T::AgentsMd,
        K::Instructions,
        &[".aider.conf.yml"],
        &[P::Planned],
        A::Never,
        "https://aider.chat/docs/config/aider_conf.html",
    )),
    // --- Claude Code --------------------------------------------------------
    row(
        "claude-dir-md",
        T::ClaudeCode,
        K::Instructions,
        &[".claude/CLAUDE.md"],
        &[P::Md],
        A::Always,
        CLAUDE_DIR_DOC,
    ),
    with_dialect(
        row(
            "claude-commands",
            T::ClaudeCode,
            K::Command,
            &[".claude/commands/**/*.md"],
            &[P::MdFm],
            A::Never,
            "https://code.claude.com/docs/en/skills",
        ),
        Dialect::ClaudeCommand,
    ),
    with_dialect(
        row(
            "claude-agents",
            T::ClaudeCode,
            K::Agent,
            &[".claude/agents/*.md"],
            &[P::MdFm],
            A::Never,
            "https://code.claude.com/docs/en/sub-agents",
        ),
        Dialect::ClaudeAgent,
    ),
    row(
        "claude-mcp",
        T::ClaudeCode,
        K::McpConfig,
        &[".mcp.json"],
        &[P::McpJson],
        A::Never,
        "https://code.claude.com/docs/en/mcp",
    ),
    row(
        "claude-settings",
        T::ClaudeCode,
        K::Settings,
        &[".claude/settings.json", ".claude/settings.local.json"],
        &[P::SettingsJson, P::HooksJson],
        A::Always,
        "https://code.claude.com/docs/en/hooks",
    ),
    row(
        "claude-plugin",
        T::ClaudeCode,
        K::Plugin,
        &[
            "**/.claude-plugin/plugin.json",
            "**/.claude-plugin/marketplace.json",
        ],
        &[P::PluginJson],
        A::Never,
        "https://code.claude.com/docs/en/plugins/manifest-reference",
    ),
    sibling(
        row(
            "claude-plugin-hooks",
            T::ClaudeCode,
            K::Hooks,
            &["**/hooks/hooks.json"],
            &[P::HooksJson],
            A::Always,
            "https://code.claude.com/docs/en/plugins/manifest-reference",
        ),
        ".claude-plugin",
    ),
    global(row(
        "claude-desktop-global",
        T::ClaudeCode,
        K::McpConfig,
        &["claude_desktop_config.json"],
        &[P::McpJson],
        A::Never,
        "https://modelcontextprotocol.io/docs/develop/connect-local-servers",
    )),
    // --- Cursor -------------------------------------------------------------
    with_dialect(
        row(
            "cursor-rules",
            T::Cursor,
            K::Instructions,
            &[".cursor/rules/**/*.mdc", ".cursor/rules/**/*.md"],
            &[P::MdFm],
            A::Frontmatter,
            "https://cursor.com/docs/rules",
        ),
        Dialect::CursorRule,
    ),
    row(
        "cursor-rules-legacy",
        T::Cursor,
        K::Instructions,
        &[".cursorrules"],
        &[P::Md],
        A::Always,
        "https://cursor.com/docs/rules",
    ),
    row(
        "cursor-mcp",
        T::Cursor,
        K::McpConfig,
        &[".cursor/mcp.json"],
        &[P::McpJson],
        A::Never,
        "https://cursor.com/docs/rules",
    ),
    row(
        "cursor-hooks",
        T::Cursor,
        K::Hooks,
        &[".cursor/hooks.json"],
        &[P::HooksJson],
        A::Always,
        "https://cursor.com/docs/hooks",
    ),
    with_dialect(
        row(
            "cursor-commands",
            T::Cursor,
            K::Command,
            &[".cursor/commands/*.md"],
            &[P::MdFm],
            A::Never,
            "https://cursor.com/docs/reference/plugins",
        ),
        Dialect::CursorCommand,
    ),
    // --- GitHub Copilot / VS Code ------------------------------------------
    row(
        "copilot-instructions",
        T::Copilot,
        K::Instructions,
        &[".github/copilot-instructions.md"],
        &[P::Md],
        A::Always,
        COPILOT_INSTR_DOC,
    ),
    with_dialect(
        row(
            "copilot-path-instructions",
            T::Copilot,
            K::Instructions,
            &[".github/instructions/**/*.instructions.md"],
            &[P::MdFm],
            A::Frontmatter,
            COPILOT_INSTR_DOC,
        ),
        Dialect::CopilotInstructions,
    ),
    with_dialect(
        row(
            "copilot-prompts",
            T::Copilot,
            K::Command,
            &[".github/prompts/*.prompt.md"],
            &[P::MdFm],
            A::Never,
            "https://docs.github.com/en/copilot/concepts/prompting-and-context/prompt-files",
        ),
        Dialect::CopilotPrompt,
    ),
    with_dialect(
        row(
            "copilot-agents",
            T::Copilot,
            K::Agent,
            &[".github/agents/*.agent.md", ".github/agents/*.md"],
            &[P::MdFm],
            A::Never,
            "https://docs.github.com/en/copilot/reference/custom-agents-configuration",
        ),
        Dialect::CopilotAgent,
    ),
    row(
        "vscode-mcp",
        T::Copilot,
        K::McpConfig,
        &[".vscode/mcp.json"],
        &[P::McpJson],
        A::Never,
        "https://code.visualstudio.com/docs/copilot/reference/mcp-configuration",
    ),
    // --- Windsurf / Devin Desktop ------------------------------------------
    with_dialect(
        row(
            "windsurf-rules",
            T::Windsurf,
            K::Instructions,
            &[".windsurf/rules/*.md", ".devin/rules/*.md"],
            &[P::MdFm],
            A::Frontmatter,
            "https://docs.windsurf.com/windsurf/cascade/memories",
        ),
        Dialect::WindsurfRule,
    ),
    row(
        "windsurf-rules-legacy",
        T::Windsurf,
        K::Instructions,
        &[".windsurfrules"],
        &[P::Md],
        A::Always,
        "https://docs.windsurf.com/windsurf/cascade/memories",
    ),
    row(
        "windsurf-workflows",
        T::Windsurf,
        K::Command,
        &[".windsurf/workflows/*.md", ".devin/workflows/*.md"],
        &[P::Md],
        A::Never,
        "https://docs.windsurf.com/windsurf/cascade/workflows",
    ),
    row(
        "windsurf-hooks",
        T::Windsurf,
        K::Hooks,
        &[".windsurf/hooks.json", ".devin/hooks.json"],
        &[P::HooksJson],
        A::Always,
        "https://docs.windsurf.com/windsurf/cascade/hooks",
    ),
    global(row(
        "windsurf-mcp-global",
        T::Windsurf,
        K::McpConfig,
        &["mcp_config.json"],
        &[P::McpJson],
        A::Never,
        "https://docs.windsurf.com/windsurf/cascade/mcp",
    )),
    // --- Cline ----------------------------------------------------------------
    row(
        "cline-workflows",
        T::Cline,
        K::Command,
        &[".clinerules/workflows/**", ".cline/workflows/**"],
        &[P::Md],
        A::Never,
        "https://docs.cline.bot/getting-started/config",
    ),
    row(
        "cline-rules",
        T::Cline,
        K::Instructions,
        &[".clinerules", ".clinerules/**", ".cline/rules/**"],
        &[P::Md],
        A::Always,
        "https://docs.cline.bot/customization/cline-rules",
    ),
    global(row(
        "cline-mcp-global",
        T::Cline,
        K::McpConfig,
        &["cline_mcp_settings.json"],
        &[P::McpJson],
        A::Never,
        "https://docs.cline.bot/mcp/configuring-mcp-servers",
    )),
    // --- Gemini CLI -----------------------------------------------------------
    row(
        "gemini-settings",
        T::GeminiCli,
        K::McpConfig,
        &[".gemini/settings.json"],
        &[P::McpJson],
        A::Never,
        "https://geminicli.com/docs/reference/configuration/",
    ),
    // --- Roo Code ---------------------------------------------------------------
    row(
        "roo-rules",
        T::RooCode,
        K::Instructions,
        &[".roo/rules/**", ".roorules"],
        &[P::Md],
        A::Always,
        "https://docs.roocode.com/features/custom-instructions",
    ),
    row(
        "roo-mode-rules",
        T::RooCode,
        K::Instructions,
        &[".roo/rules-*/**", ".roorules-*"],
        &[P::Md],
        A::Never,
        "https://docs.roocode.com/features/custom-instructions",
    ),
    row(
        "roo-mcp",
        T::RooCode,
        K::McpConfig,
        &[".roo/mcp.json"],
        &[P::McpJson],
        A::Never,
        "https://docs.roocode.com/features/mcp/using-mcp-in-roo",
    ),
    // --- Continue ---------------------------------------------------------------
    with_dialect(
        row(
            "continue-rules",
            T::Continue,
            K::Instructions,
            &[".continue/rules/*.md"],
            &[P::MdFm],
            A::Frontmatter,
            "https://docs.continue.dev/customize/deep-dives/rules",
        ),
        Dialect::ContinueRule,
    ),
    row(
        "continue-mcp-yaml",
        T::Continue,
        K::McpConfig,
        &[".continue/mcpServers/*.yaml", ".continue/mcpServers/*.yml"],
        &[P::McpYaml],
        A::Never,
        "https://docs.continue.dev/customize/deep-dives/mcp",
    ),
    row(
        "continue-mcp-json",
        T::Continue,
        K::McpConfig,
        &[".continue/mcpServers/*.json"],
        &[P::McpJson],
        A::Never,
        "https://docs.continue.dev/customize/deep-dives/mcp",
    ),
    // --- Zed ----------------------------------------------------------------------
    row(
        "zed-rules",
        T::Zed,
        K::Instructions,
        &[".rules"],
        &[P::Md],
        A::Always,
        "https://zed.dev/docs/ai/instructions",
    ),
    row(
        "zed-settings",
        T::Zed,
        K::McpConfig,
        &[".zed/settings.json"],
        &[P::McpJson],
        A::Never,
        "https://zed.dev/docs/ai/mcp",
    ),
    // --- JetBrains Junie ------------------------------------------------------
    row(
        "junie-guidelines",
        T::Junie,
        K::Instructions,
        &[
            ".junie/AGENTS.md",
            ".junie/guidelines.md",
            ".junie/guidelines/**",
            ".junie/playbook.md",
            ".junie/rules/*.md",
        ],
        &[P::Md],
        A::Always,
        "https://junie.jetbrains.com/docs/guidelines-and-memory.html",
    ),
    row(
        "junie-mcp",
        T::Junie,
        K::McpConfig,
        &[".junie/mcp/mcp.json"],
        &[P::McpJson],
        A::Never,
        "https://junie.jetbrains.com/docs/junie-cli-mcp-configuration.html",
    ),
    // --- Kiro -------------------------------------------------------------------
    with_dialect(
        row(
            "kiro-steering",
            T::Kiro,
            K::Instructions,
            &[".kiro/steering/*.md"],
            &[P::MdFm],
            A::Frontmatter,
            "https://kiro.dev/docs/steering/",
        ),
        Dialect::KiroSteering,
    ),
    row(
        "kiro-mcp",
        T::Kiro,
        K::McpConfig,
        &[".kiro/settings/mcp.json"],
        &[P::McpJson],
        A::Never,
        "https://kiro.dev/docs/mcp/",
    ),
    row(
        "kiro-hooks",
        T::Kiro,
        K::Hooks,
        &[".kiro/hooks/*.json"],
        &[P::HooksJson],
        A::Always,
        "https://kiro.dev/docs/hooks/",
    ),
    // --- OpenAI Codex -----------------------------------------------------------
    row(
        "codex-config",
        T::Codex,
        K::McpConfig,
        &[".codex/config.toml"],
        &[P::McpToml],
        A::Never,
        "https://developers.openai.com/codex/config-basic",
    ),
    // --- Amazon Q Developer -----------------------------------------------------
    row(
        "amazon-q-rules",
        T::AmazonQ,
        K::Instructions,
        &[".amazonq/rules/**/*.md"],
        &[P::Md],
        A::Always,
        "https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/context-project-rules.html",
    ),
    row(
        "amazon-q-mcp",
        T::AmazonQ,
        K::McpConfig,
        &[".amazonq/mcp.json"],
        &[P::McpJson],
        A::Never,
        "https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/mcp-ide.html",
    ),
    // --- Broad cross-directory rows (last) -------------------------------------
    row(
        "claude-md",
        T::ClaudeCode,
        K::Instructions,
        &["**/CLAUDE.md", "**/CLAUDE.local.md"],
        &[P::Md],
        A::RootOnly,
        CLAUDE_DIR_DOC,
    ),
    row(
        "gemini-md",
        T::GeminiCli,
        K::Instructions,
        &["**/GEMINI.md"],
        &[P::Md],
        A::RootOnly,
        "https://geminicli.com/docs/cli/gemini-md/",
    ),
    row(
        "agents-md",
        T::AgentsMd,
        K::Instructions,
        &["**/AGENTS.md"],
        &[P::Md],
        A::RootOnly,
        "https://agents.md/",
    ),
];

/// Compiled matchers for [`ROWS`].
struct Compiled {
    rows: Vec<(&'static RegistryRow, Vec<GlobMatcher>)>,
}

fn compile_glob(pattern: &str) -> Result<GlobMatcher> {
    globset::GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|g: Glob| g.compile_matcher())
        .map_err(|e| CoreError::Glob {
            pattern: pattern.to_string(),
            message: e.to_string(),
        })
}

fn compiled() -> &'static Compiled {
    static C: OnceLock<Compiled> = OnceLock::new();
    C.get_or_init(|| Compiled {
        rows: ROWS
            .iter()
            .map(|r| {
                let globs = r
                    .globs
                    .iter()
                    // INVARIANT: every built-in glob compiles (asserted by `all_globs_compile`).
                    .filter_map(|g| compile_glob(g).ok())
                    .collect();
                (r, globs)
            })
            .collect(),
    })
}

/// Check that every row's globs compile.
pub fn validate() -> Result<()> {
    for r in ROWS {
        for g in r.globs {
            compile_glob(g)?;
        }
    }
    Ok(())
}

/// How a path was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Found while walking a directory.
    Walked,
    /// Passed explicitly by the user (global-only rows also match).
    Explicit,
}

/// Find the row for a path relative to the scan root (`/` separated).
///
/// Returns the matched row even when it is a v0.2 row; callers check
/// [`RegistryRow::release`]. Sibling requirements are checked by discovery.
pub fn match_path(rel: &str, origin: Origin) -> Option<&'static RegistryRow> {
    let rel = rel.trim_start_matches("./");
    compiled().rows.iter().find_map(|(r, globs)| {
        if r.global_only && origin != Origin::Explicit {
            return None;
        }
        globs.iter().any(|g| g.is_match(rel)).then_some(*r)
    })
}

/// Find the row of agent files a directory at `rel` (relative to the scan
/// root, `/` separated) could hold, for directories the scan cannot enter
/// (symlinks outside the root, #100/#122). Generic over the registry:
///
/// - a glob rooted at a fixed location (`.cursor/rules/**/*.mdc`,
///   `.claude/settings.json`) matches when `rel` is one of its directories
///   or a directory its `**` reaches;
/// - a glob starting with `**/` (`**/.claude-plugin/plugin.json`) matches
///   when a trailing part of `rel` names one of its directories, so a dir
///   that only might contain a nested `AGENTS.md` does not count;
/// - skills (`**/SKILL.md`, which can sit anywhere) match any directory at
///   or below a `skills` directory (`.claude/skills`, `.claude/skills/x`,
///   `skills/x/scripts`);
/// - a tool directory at any depth (`pkg/.claude`, `.agents`) matches what
///   it holds at the top level, and counts as a skill location itself.
///
/// `sibling_ok(dir, name)` reports whether `name` exists in `dir` (scan-root
/// relative) for rows with [`RegistryRow::requires_sibling`].
pub fn match_dir(
    rel: &str,
    sibling_ok: &dyn Fn(&str, &str) -> bool,
) -> Option<&'static RegistryRow> {
    let rel = rel.trim_start_matches("./").trim_end_matches('/');
    let parts: Vec<&str> = rel.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let usable = |r: &RegistryRow| r.release == Release::Mvp && !r.global_only;
    let generic = ROWS.iter().filter(|r| usable(r)).find(|r| {
        r.globs.iter().any(|g| {
            let comps: Vec<&str> = g.split('/').collect();
            let fits = |dir: &[&str], glob: &[&str], start: usize| -> bool {
                match dir_fits_glob(dir, glob) {
                    DirFit::No => false,
                    DirFit::Yes => true,
                    // `dir` is exactly the file's directory: check siblings
                    DirFit::FileDir => r.requires_sibling.is_none_or(|sib| {
                        sibling_ok(&parts[..start + dir.len() - 1].join("/"), sib)
                    }),
                }
            };
            match comps.split_first() {
                Some((&"**", rest)) => (0..parts.len()).any(|k| fits(&parts[k..], rest, k)),
                _ => fits(&parts, &comps, 0),
            }
        })
    });
    let skill_row = || ROWS.iter().find(|r| usable(r) && r.kind == FileKind::Skill);
    if let Some(r) = generic {
        return Some(r);
    }
    if parts.contains(&"skills") {
        return skill_row();
    }
    // A tool directory at any depth (`pkg/.claude`, `a/b/.cursor/rules`,
    // `.agents`): match what it would hold at the top level, and count the
    // tool directory itself as a skill location (`<dir>/skills/<name>/SKILL.md`
    // is discovered at any depth) (#134).
    for k in 0..parts.len() {
        if !is_tool_dir(parts[k]) {
            continue;
        }
        if k > 0 {
            let prefix = parts[..k].join("/");
            let nested = |d: &str, s: &str| {
                sibling_ok(
                    &if d.is_empty() {
                        prefix.clone()
                    } else {
                        format!("{prefix}/{d}")
                    },
                    s,
                )
            };
            if let Some(r) = match_dir(&parts[k..].join("/"), &nested) {
                return Some(r);
            }
        }
        if k + 1 == parts.len() {
            return skill_row();
        }
    }
    None
}

/// Whether `name` is a tool's configuration directory: the first component
/// of a scanned registry glob (`.claude`, `.cursor`, `.github`, ...) or
/// `.agents` (the cross-tool skills location, `.agents/skills`).
pub fn is_tool_dir(name: &str) -> bool {
    name == ".agents"
        || ROWS
            .iter()
            .filter(|r| r.release == Release::Mvp && !r.global_only)
            .flat_map(|r| r.globs.iter())
            .filter_map(|g| g.trim_start_matches("**/").split_once('/'))
            .any(|(first, _)| first.starts_with('.') && first == name)
}

/// How a directory path relates to a file glob's components.
enum DirFit {
    /// The glob cannot match anything below the directory.
    No,
    /// The glob can match a file below the directory.
    Yes,
    /// The directory is the glob's file directory itself.
    FileDir,
}

fn dir_fits_glob(dir: &[&str], glob: &[&str]) -> DirFit {
    for (i, d) in dir.iter().enumerate() {
        let Some(g) = glob.get(i) else {
            return DirFit::No;
        };
        if *g == "**" {
            return DirFit::Yes;
        }
        // the last glob component names the file, not a directory
        if i + 1 == glob.len() {
            return DirFit::No;
        }
        let ok = compile_glob(g).map(|m| m.is_match(d)).unwrap_or(false);
        if !ok {
            return DirFit::No;
        }
    }
    if dir.len() + 1 == glob.len() {
        DirFit::FileDir
    } else {
        DirFit::Yes
    }
}

/// Match an explicitly passed file: try the full relative path, then every
/// suffix of its components (so `/home/u/.codex/config.toml` matches
/// `.codex/config.toml`), longest first.
pub fn match_explicit(path: &Path) -> Option<&'static RegistryRow> {
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    (0..parts.len()).find_map(|i| match_path(&parts[i..].join("/"), Origin::Explicit))
}

/// Rows enabled in this release.
pub fn mvp_rows() -> impl Iterator<Item = &'static RegistryRow> {
    ROWS.iter().filter(|r| r.release == Release::Mvp)
}

/// Date the registry's doc links were last re-checked (PRD §5.1).
pub const DOCS_CHECKED: &str = "2026-10-29";

/// Render `docs/supported-tools.md` from the registry.
pub fn render_markdown() -> String {
    let mut out = format!(
        "# Supported tools

<!-- Generated from crates/rulesift-core/src/registry.rs. Do not edit by hand;
     regenerate with `RULESIFT_BLESS=1 cargo test -p rulesift-core --test registry_docs`. -->

RuleSift finds agent configuration files through one data-driven registry.
Paths are project-relative globs (`**` = any directories). The first matching
row wins. Doc links were last checked on {DOCS_CHECKED}.

- **Release**: `mvp` rows are scanned today; `v0.2` rows are recognized and
  skipped with a note.
- **Always loaded**: files the tool reads in every session get the 1.5x score
  multiplier (`frontmatter` = decided by the file's frontmatter).
- **Global-only** rows live outside a repository; RuleSift reads them only when
  you pass the path explicitly.

| Tool | Path | Kind | Parser | Always loaded | Release | Docs |
|---|---|---|---|---|---|---|
"
    );
    for r in ROWS {
        let paths = r
            .globs
            .iter()
            .map(|g| format!("`{g}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let parsers = r
            .parsers
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let mut always = r.always.as_str().to_string();
        if r.global_only {
            always.push_str(" (global-only)");
        }
        let mut paths = paths;
        if let Some(s) = r.requires_sibling {
            paths.push_str(&format!(" (next to `{s}`)"));
        }
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | <{}> |\n",
            r.tool, paths, r.kind, parsers, always, r.release, r.doc_url
        ));
    }
    out
}

/// Render the registry as a plain-text table (used by `--list-tools` and snapshots).
pub fn render_table() -> String {
    let mut rows: Vec<[String; 7]> = vec![[
        "TOOL".into(),
        "KIND".into(),
        "PATH".into(),
        "PARSER".into(),
        "ALWAYS".into(),
        "RELEASE".into(),
        "DOC".into(),
    ]];
    for r in ROWS {
        let parsers = r
            .parsers
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let mut always = r.always.as_str().to_string();
        if r.global_only {
            always.push_str(" (global-only)");
        }
        rows.push([
            r.tool.to_string(),
            r.kind.to_string(),
            r.globs.join(" "),
            parsers,
            always,
            r.release.to_string(),
            r.doc_url.to_string(),
        ]);
    }
    let mut widths = [0usize; 7];
    for r in &rows {
        for (i, c) in r.iter().enumerate() {
            widths[i] = widths[i].max(c.chars().count());
        }
    }
    let mut out = String::new();
    for r in &rows {
        let mut line = String::new();
        for (i, c) in r.iter().enumerate() {
            if i == 6 {
                line.push_str(c);
            } else {
                let _ = write!(line, "{:<w$}  ", c, w = widths[i]);
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn match_dir_covers_agent_locations_generically() {
        let none = |_: &str, _: &str| false;
        let id = |rel: &str| match_dir(rel, &none).map(|r| r.id);
        // skills, at any depth below a `skills` dir (#122)
        for rel in [
            ".claude/skills",
            ".claude/skills/evil",
            ".claude/skills/x/scripts",
            ".agents/skills/x",
            ".cursor/skills",
            "skills/x",
            "pkg/.claude/skills/x",
        ] {
            assert_eq!(id(rel), Some("agent-skills"), "{rel}");
        }
        // fixed locations and their parents
        for rel in [
            ".claude",
            ".cursor",
            ".cursor/rules",
            ".cursor/rules/linked",
            ".github",
            ".github/instructions",
            ".windsurf/rules",
            ".continue/mcpServers",
            ".kiro/steering",
            ".roo/rules-code",
            ".clinerules",
            ".codex",
            "a/.claude-plugin",
            // nested tool dirs and `.agents` (#134)
            ".agents",
            "pkg/.claude",
            "a/b/.cursor",
            "pkg/.cursor/rules",
            "pkg/.github/instructions",
        ] {
            assert!(id(rel).is_some(), "{rel}");
        }
        assert_eq!(id(".agents"), Some("agent-skills"));
        // ordinary directories do not count
        for rel in [
            "src",
            "vendor/lib",
            "node_modules",
            "docs/rules",
            ".vscode/extensions",
            "hooks",
            "pkg/.claude/foo",
            "pkg/docs",
        ] {
            assert_eq!(id(rel), None, "{rel}");
        }
        // plugin hooks need the `.claude-plugin` sibling
        let yes = |d: &str, s: &str| d == "plug" && s == ".claude-plugin";
        assert!(match_dir("plug/hooks", &yes).is_some());
        assert!(match_dir("other/hooks", &yes).is_none());
    }

    #[test]
    fn all_globs_compile() {
        validate().unwrap();
        for r in ROWS {
            assert!(!r.globs.is_empty(), "{} has no globs", r.id);
            assert!(r.doc_url.starts_with("https://"), "{} doc url", r.id);
        }
    }

    #[test]
    fn row_ids_unique() {
        let mut ids: Vec<_> = ROWS.iter().map(|r| r.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    /// One example path per §5.1 MVP row: (path, tool, kind, row id).
    pub(crate) const PRD_MVP_PATHS: &[(&str, Tool, FileKind)] = &[
        (
            ".claude/skills/pdf-tools/SKILL.md",
            T::AgentSkills,
            K::Skill,
        ),
        (".agents/skills/x/SKILL.md", T::AgentSkills, K::Skill),
        (".cursor/skills/x/SKILL.md", T::AgentSkills, K::Skill),
        (".cline/skills/x/SKILL.md", T::AgentSkills, K::Skill),
        (".windsurf/skills/x/SKILL.md", T::AgentSkills, K::Skill),
        ("AGENTS.md", T::AgentsMd, K::Instructions),
        ("pkg/sub/AGENTS.md", T::AgentsMd, K::Instructions),
        ("CLAUDE.md", T::ClaudeCode, K::Instructions),
        ("CLAUDE.local.md", T::ClaudeCode, K::Instructions),
        (".claude/CLAUDE.md", T::ClaudeCode, K::Instructions),
        (".claude/commands/review.md", T::ClaudeCode, K::Command),
        (".claude/commands/git/commit.md", T::ClaudeCode, K::Command),
        (".claude/agents/reviewer.md", T::ClaudeCode, K::Agent),
        (".mcp.json", T::ClaudeCode, K::McpConfig),
        (".claude/settings.json", T::ClaudeCode, K::Settings),
        (".claude/settings.local.json", T::ClaudeCode, K::Settings),
        (".claude-plugin/plugin.json", T::ClaudeCode, K::Plugin),
        (".claude-plugin/marketplace.json", T::ClaudeCode, K::Plugin),
        ("hooks/hooks.json", T::ClaudeCode, K::Hooks),
        (".cursor/rules/web.mdc", T::Cursor, K::Instructions),
        (".cursor/rules/sub/style.md", T::Cursor, K::Instructions),
        (".cursorrules", T::Cursor, K::Instructions),
        (".cursor/mcp.json", T::Cursor, K::McpConfig),
        (".cursor/hooks.json", T::Cursor, K::Hooks),
        (".cursor/commands/fix.md", T::Cursor, K::Command),
        (
            ".github/copilot-instructions.md",
            T::Copilot,
            K::Instructions,
        ),
        (
            ".github/instructions/rust.instructions.md",
            T::Copilot,
            K::Instructions,
        ),
        (".github/prompts/release.prompt.md", T::Copilot, K::Command),
        (".github/agents/planner.agent.md", T::Copilot, K::Agent),
        (".github/agents/helper.md", T::Copilot, K::Agent),
        (".vscode/mcp.json", T::Copilot, K::McpConfig),
        (".windsurf/rules/style.md", T::Windsurf, K::Instructions),
        (".devin/rules/style.md", T::Windsurf, K::Instructions),
        (".windsurfrules", T::Windsurf, K::Instructions),
        (".windsurf/workflows/deploy.md", T::Windsurf, K::Command),
        (".devin/workflows/deploy.md", T::Windsurf, K::Command),
        (".windsurf/hooks.json", T::Windsurf, K::Hooks),
        (".devin/hooks.json", T::Windsurf, K::Hooks),
        (".clinerules", T::Cline, K::Instructions),
        (".clinerules/style.md", T::Cline, K::Instructions),
        (".cline/rules/style.md", T::Cline, K::Instructions),
        (".clinerules/workflows/release.md", T::Cline, K::Command),
        (".cline/workflows/release.md", T::Cline, K::Command),
        ("GEMINI.md", T::GeminiCli, K::Instructions),
        ("docs/GEMINI.md", T::GeminiCli, K::Instructions),
        (".gemini/settings.json", T::GeminiCli, K::McpConfig),
        (".roo/rules/style.md", T::RooCode, K::Instructions),
        (".roo/rules-code/style.md", T::RooCode, K::Instructions),
        (".roorules", T::RooCode, K::Instructions),
        (".roorules-code", T::RooCode, K::Instructions),
        (".roo/mcp.json", T::RooCode, K::McpConfig),
        (".continue/rules/style.md", T::Continue, K::Instructions),
        (".continue/mcpServers/db.yaml", T::Continue, K::McpConfig),
        (".continue/mcpServers/db.json", T::Continue, K::McpConfig),
        (".rules", T::Zed, K::Instructions),
        (".zed/settings.json", T::Zed, K::McpConfig),
        (".junie/AGENTS.md", T::Junie, K::Instructions),
        (".junie/guidelines.md", T::Junie, K::Instructions),
        (".junie/guidelines/style.md", T::Junie, K::Instructions),
        (".junie/playbook.md", T::Junie, K::Instructions),
        (".junie/rules/style.md", T::Junie, K::Instructions),
        (".junie/mcp/mcp.json", T::Junie, K::McpConfig),
        (".kiro/steering/product.md", T::Kiro, K::Instructions),
        (".kiro/settings/mcp.json", T::Kiro, K::McpConfig),
        (".kiro/hooks/lint.json", T::Kiro, K::Hooks),
        (".codex/config.toml", T::Codex, K::McpConfig),
        (".amazonq/rules/style.md", T::AmazonQ, K::Instructions),
        (
            ".amazonq/rules/nested/style.md",
            T::AmazonQ,
            K::Instructions,
        ),
        (".amazonq/mcp.json", T::AmazonQ, K::McpConfig),
    ];

    #[test]
    fn every_prd_mvp_path_is_classified() {
        for (path, tool, kind) in PRD_MVP_PATHS {
            let r =
                match_path(path, Origin::Walked).unwrap_or_else(|| panic!("{path} not matched"));
            assert_eq!(r.release, Release::Mvp, "{path} -> {}", r.id);
            assert_eq!((r.tool, r.kind), (*tool, *kind), "{path} -> {}", r.id);
        }
    }

    #[test]
    fn every_mvp_row_is_exercised() {
        for r in mvp_rows().filter(|r| !r.global_only) {
            assert!(
                PRD_MVP_PATHS
                    .iter()
                    .any(|(p, ..)| match_path(p, Origin::Walked).map(|x| x.id) == Some(r.id)),
                "row {} has no example path",
                r.id
            );
        }
    }

    #[test]
    fn v0_2_rows_are_present_but_disabled() {
        for p in [
            ".amazonq/default.json",
            ".roomodes",
            ".gemini/commands/deploy.toml",
            ".gemini/extensions/foo/gemini-extension.json",
            ".clinerules/hooks/pre.sh",
            ".cline/hooks/pre.sh",
            ".aider.conf.yml",
        ] {
            let r = match_path(p, Origin::Walked).unwrap_or_else(|| panic!("{p}"));
            assert_eq!(r.release, Release::V0_2, "{p}");
        }
    }

    #[test]
    fn unrelated_paths_do_not_match() {
        for p in [
            "README.md",
            "src/main.rs",
            ".github/workflows/ci.yml",
            "docs/agents.md",
            ".cursor/other.json",
        ] {
            assert!(match_path(p, Origin::Walked).is_none(), "{p}");
        }
    }

    #[test]
    fn global_only_rows_need_explicit_paths() {
        assert!(match_path("mcp_config.json", Origin::Walked).is_none());
        let r = match_explicit(Path::new("/home/u/.codeium/windsurf/mcp_config.json")).unwrap();
        assert_eq!(r.tool, T::Windsurf);
        let r = match_explicit(Path::new(
            "/home/u/.cline/data/settings/cline_mcp_settings.json",
        ))
        .unwrap();
        assert_eq!(r.tool, T::Cline);
        let r = match_explicit(Path::new(
            "/home/u/Library/Application Support/Claude/claude_desktop_config.json",
        ))
        .unwrap();
        assert_eq!(r.id, "claude-desktop-global");
        let r = match_explicit(Path::new("/home/u/.codex/config.toml")).unwrap();
        assert_eq!(r.tool, T::Codex);
    }

    #[test]
    fn registry_table_snapshot() {
        insta::assert_snapshot!(render_table());
    }
}

/// Translate one registry glob into an anchored regular expression
/// (Python `re` / Rust `regex` compatible subset): `**/` → any directories,
/// `**` → anything, `*` → within one path segment, `?` → one character,
/// `{a,b}` → alternatives, `[...]` kept.
pub fn glob_to_regex(glob: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    let mut in_brace = false;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '*' if chars.get(i + 1) == Some(&'*') => {
                if chars.get(i + 2) == Some(&'/') {
                    out.push_str("(?:.*/)?");
                    i += 3;
                } else {
                    out.push_str(".*");
                    i += 2;
                }
                continue;
            }
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            '{' => {
                in_brace = true;
                out.push_str("(?:");
            }
            '}' if in_brace => {
                in_brace = false;
                out.push(')');
            }
            ',' if in_brace => out.push('|'),
            '[' => {
                let end = chars[i..].iter().position(|&x| x == ']').map(|p| p + i);
                if let Some(end) = end {
                    let body: String = chars[i + 1..end].iter().collect();
                    let body = body
                        .strip_prefix('!')
                        .map(|b| format!("^{b}"))
                        .unwrap_or(body);
                    out.push('[');
                    out.push_str(&body);
                    out.push(']');
                    i = end + 1;
                    continue;
                }
                out.push_str("\\[");
            }
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '\\' | ']' | '}' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

/// The pre-commit `files` regex matching every project-level MVP path, plus
/// files bundled inside `skills/<name>/` directories.
pub fn precommit_files_regex() -> String {
    let mut parts: Vec<String> = ROWS
        .iter()
        .filter(|r| r.release == Release::Mvp && !r.global_only)
        .flat_map(|r| r.globs.iter().map(|g| glob_to_regex(g)))
        .collect();
    // files bundled with a skill (scripts, references) under a `skills/` dir
    parts.push("(?:.*/)?skills/[^/]+/.+".to_string());
    parts.sort();
    parts.dedup();
    format!("^(?:{})$", parts.join("|"))
}

#[cfg(test)]
mod precommit_tests {
    use super::*;

    #[test]
    fn glob_translation() {
        let re = |g: &str| regex::Regex::new(&format!("^{}$", glob_to_regex(g))).unwrap();
        assert!(re("**/SKILL.md").is_match("SKILL.md"));
        assert!(re("**/SKILL.md").is_match("a/b/SKILL.md"));
        assert!(!re("**/SKILL.md").is_match("xSKILL.md"));
        assert!(re(".cursor/rules/**/*.mdc").is_match(".cursor/rules/a.mdc"));
        assert!(re(".cursor/rules/**/*.mdc").is_match(".cursor/rules/x/y/a.mdc"));
        assert!(!re(".cursor/rules/*.mdc").is_match(".cursor/rules/x/a.mdc"));
        assert!(!re(".cursorrules").is_match("xcursorrules"));
        assert!(re("a.{json,yaml}").is_match("a.yaml"));
        assert!(re(".roo/rules-[a-z]*/x").is_match(".roo/rules-code/x"));
    }

    #[test]
    fn regex_agrees_with_globs_on_all_tools_fixture() {
        let re = regex::Regex::new(&precommit_files_regex()).unwrap();
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/repos/all-tools");
        let d = crate::discovery::discover(&[root], &Default::default()).unwrap();
        assert!(d.files.len() > 40);
        for f in &d.files {
            // files inside skill dirs are found through their SKILL.md, not a glob
            if f.row.id == "agent-skills-file" && !f.match_path.contains("skills/") {
                continue;
            }
            assert!(
                re.is_match(&f.match_path),
                "{} ({}) not matched",
                f.match_path,
                f.row.id
            );
        }
        for neg in ["README.md", "src/main.rs", "docs/guide.md", "Cargo.toml"] {
            assert!(!re.is_match(neg), "{neg} matched");
        }
    }

    #[test]
    fn every_mvp_glob_is_covered() {
        // each MVP glob's regex matches a path built from the glob itself
        let re = regex::Regex::new(&precommit_files_regex()).unwrap();
        for r in ROWS
            .iter()
            .filter(|r| r.release == Release::Mvp && !r.global_only)
        {
            for g in r.globs {
                let sample = g
                    .replace("**/", "")
                    .replace("**", "x")
                    .replace(['*', '?'], "x");
                let sample = if let (Some(a), Some(b)) = (sample.find('{'), sample.find('}')) {
                    let first = sample[a + 1..b].split(',').next().unwrap_or("").to_string();
                    format!("{}{}{}", &sample[..a], first, &sample[b + 1..])
                } else {
                    sample
                };
                if sample.contains('[') {
                    continue;
                }
                assert!(re.is_match(&sample), "{} glob {g} sample {sample}", r.id);
                assert!(
                    match_path(&sample, Origin::Walked).is_some() || r.requires_sibling.is_some(),
                    "{sample}"
                );
            }
        }
    }
}
