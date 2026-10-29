//! Domain model shared by every module: file kinds, tools, documents,
//! segments, findings, and reports.

use serde::{Deserialize, Serialize};

/// Declares a string-backed enum with `as_str`, `ALL`, `Display`, and `FromStr`.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident { $( $(#[$vmeta:meta])* $variant:ident => $s:literal ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ::serde::Serialize, ::serde::Deserialize)]
        pub enum $name {
            $( $(#[$vmeta])* #[serde(rename = $s)] $variant ),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            /// Stable string id used in config, CLI flags, and outputs.
            pub fn as_str(self) -> &'static str {
                match self { $( $name::$variant => $s ),+ }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let t = s.trim();
                $( if t.eq_ignore_ascii_case($s) { return Ok($name::$variant); } )+
                Err(format!(
                    "unknown {} {:?} (expected one of: {})",
                    stringify!($name).to_ascii_lowercase(),
                    s,
                    $name::ALL.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                ))
            }
        }
    };
}
pub(crate) use string_enum;

string_enum! {
    /// Kind of agent file (PRD §5 F1). Used by rules' `applies_to`.
    pub enum FileKind {
        /// Agent Skills directory with `SKILL.md`.
        Skill => "skill",
        /// Instructions/rules loaded into the agent context.
        Instructions => "instructions",
        /// On-demand prompt/command/workflow.
        Command => "command",
        /// Agent or sub-agent definition.
        Agent => "agent",
        /// MCP server definitions.
        McpConfig => "mcp-config",
        /// Lifecycle hook commands.
        Hooks => "hooks",
        /// Permission/trust settings.
        Settings => "settings",
        /// Plugin or marketplace manifest.
        Plugin => "plugin",
    }
}

string_enum! {
    /// Coding agent (or cross-tool standard) a file belongs to.
    pub enum Tool {
        /// Claude Code.
        ClaudeCode => "claude-code",
        /// Cursor.
        Cursor => "cursor",
        /// GitHub Copilot / VS Code.
        Copilot => "copilot",
        /// Windsurf / Devin Desktop.
        Windsurf => "windsurf",
        /// Cline.
        Cline => "cline",
        /// Gemini CLI.
        GeminiCli => "gemini-cli",
        /// Roo Code.
        RooCode => "roo-code",
        /// Continue.
        Continue => "continue",
        /// Zed.
        Zed => "zed",
        /// JetBrains Junie.
        Junie => "junie",
        /// Kiro.
        Kiro => "kiro",
        /// OpenAI Codex.
        Codex => "codex",
        /// Amazon Q Developer.
        AmazonQ => "amazon-q",
        /// AGENTS.md (cross-tool).
        AgentsMd => "agents-md",
        /// Agent Skills (cross-tool).
        AgentSkills => "agent-skills",
    }
}

string_enum! {
    /// Where a segment of text comes from (PRD §5 F2).
    pub enum Scope {
        /// YAML frontmatter value.
        Frontmatter => "frontmatter",
        /// Plain prose.
        Text => "text",
        /// Markdown heading.
        Heading => "heading",
        /// Fenced or indented code block.
        CodeBlock => "code_block",
        /// Inline code span.
        InlineCode => "inline_code",
        /// HTML comment body.
        HtmlComment => "html_comment",
        /// Link destination URL.
        LinkUrl => "link_url",
        /// Image source URL.
        ImageUrl => "image_url",
        /// JSON/TOML/YAML string value (with pointer).
        JsonString => "json_string",
        /// Derived text decoded from hidden content.
        Decoded => "decoded",
    }
}

string_enum! {
    /// Finding severity. Ordered `Low < Medium < High < Critical`.
    pub enum Severity {
        /// Low.
        Low => "low",
        /// Medium.
        Medium => "medium",
        /// High.
        High => "high",
        /// Critical.
        Critical => "critical",
    }
}

string_enum! {
    /// Risk tier of a file or repo. Ordered `Ok < Info < Review < Block`.
    pub enum Tier {
        /// Score 0.
        Ok => "ok",
        /// Score 1–39 (default thresholds).
        Info => "info",
        /// Score 40–69.
        Review => "review",
        /// Score ≥ 70.
        Block => "block",
    }
}

/// Default score threshold for [`Tier::Block`].
pub const DEFAULT_BLOCK_AT: u32 = 70;
/// Default score threshold for [`Tier::Review`].
pub const DEFAULT_REVIEW_AT: u32 = 40;

impl Tier {
    /// Tier for `score` with the given thresholds.
    pub fn from_score_with(score: u32, review_at: u32, block_at: u32) -> Tier {
        if score >= block_at {
            Tier::Block
        } else if score >= review_at {
            Tier::Review
        } else if score >= 1 {
            Tier::Info
        } else {
            Tier::Ok
        }
    }

    /// Tier for `score` with the default thresholds (70 / 40).
    pub fn from_score(score: u32) -> Tier {
        Self::from_score_with(score, DEFAULT_REVIEW_AT, DEFAULT_BLOCK_AT)
    }

    /// Upper-case label shown in tables (`BLOCK`, `REVIEW`, `INFO`, `OK`).
    pub fn label(self) -> &'static str {
        match self {
            Tier::Block => "BLOCK",
            Tier::Review => "REVIEW",
            Tier::Info => "INFO",
            Tier::Ok => "OK",
        }
    }
}

/// Byte range `[start, end)` into a document's text.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub struct Span {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

/// One piece of text from a document with its location and scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    /// Scope of the segment.
    pub scope: Scope,
    /// Text content (decoded text for `decoded` segments).
    pub text: String,
    /// Byte span in the source (for `decoded`, the span of the hidden source).
    pub span: Span,
    /// 1-based start line.
    pub line: usize,
    /// 1-based start column (in characters).
    pub column: usize,
    /// 1-based end line.
    pub end_line: usize,
    /// Code-block info string language, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// JSON pointer for `json_string` / structured values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer: Option<String>,
    /// For `decoded` segments: what was decoded (`unicode_tags`, `base64`, `details`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

impl Segment {
    /// New segment without language, pointer, or origin.
    pub fn new(
        scope: Scope,
        text: impl Into<String>,
        span: Span,
        line: usize,
        column: usize,
        end_line: usize,
    ) -> Self {
        Self {
            scope,
            text: text.into(),
            span,
            line,
            column,
            end_line,
            lang: None,
            pointer: None,
            origin: None,
        }
    }
}

/// A parsed agent file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Path relative to the scan root, `/`-separated.
    pub path: String,
    /// Tool the file belongs to.
    pub tool: Tool,
    /// File kind.
    pub kind: FileKind,
    /// Whether the file is always loaded into the agent context (PRD §5.1).
    pub always_loaded: bool,
    /// Full text (lossily decoded if not UTF-8).
    pub text: String,
    /// Segments, sorted by span.
    pub segments: Vec<Segment>,
}

/// One rule match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Rule id, e.g. `RS-EXEC-001`.
    pub rule_id: String,
    /// Effective severity.
    pub severity: Severity,
    /// English rule title.
    pub title: String,
    /// English message.
    pub message: String,
    /// File path relative to the scan root.
    pub path: String,
    /// 1-based line.
    pub line: usize,
    /// 1-based column.
    pub column: usize,
    /// 1-based end line.
    pub end_line: usize,
    /// Evidence snippet (≤ 200 chars, hidden characters as `<U+XXXX>`).
    pub evidence: String,
    /// Decoded hidden text, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoded: Option<String>,
    /// Stable fingerprint (`sha256:<hex>`).
    pub fingerprint: String,
    /// Whether the finding is suppressed (config or baseline).
    #[serde(default)]
    pub suppressed: bool,
}

/// Per-file score breakdown (PRD §9).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Breakdown {
    /// Sum of finding weights.
    pub weights: u32,
    /// Always-loaded multiplier (1.0 or configured value).
    pub multiplier: f64,
    /// Toxic-combination bonus.
    pub bonus: u32,
}

/// Scan result for one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileReport {
    /// Path relative to the scan root.
    pub path: String,
    /// Tool.
    pub tool: Tool,
    /// Kind.
    pub kind: FileKind,
    /// Score 0–100.
    pub score: u32,
    /// Tier.
    pub tier: Tier,
    /// Score breakdown.
    pub breakdown: Breakdown,
    /// Findings, sorted by line then rule id.
    pub findings: Vec<Finding>,
}

/// Name and version of the producing tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Always `rulesift`.
    pub name: String,
    /// Version.
    pub version: String,
}

impl Default for ToolInfo {
    fn default() -> Self {
        Self {
            name: "rulesift".into(),
            version: crate::VERSION.into(),
        }
    }
}

/// Report summary counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Summary {
    /// Files scanned.
    pub files_scanned: usize,
    /// Unsuppressed findings.
    pub findings: usize,
    /// Files in `block`.
    pub block: usize,
    /// Files in `review`.
    pub review: usize,
    /// Files in `info`.
    pub info: usize,
    /// Files in `ok`.
    pub ok: usize,
    /// Max file score.
    pub repo_score: u32,
    /// Tier of the repo score.
    pub repo_tier: Tier,
}

// The enum is generated by `string_enum!`, which cannot place `#[default]`
// on a single variant, so the impl is written by hand.
#[allow(clippy::derivable_impls)]
impl Default for Tier {
    fn default() -> Self {
        Tier::Ok
    }
}

/// Full scan report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// Always 1.
    pub schema_version: u32,
    /// Producing tool.
    pub tool: ToolInfo,
    /// Scan root as given by the user.
    pub root: String,
    /// RFC 3339 timestamp, if set by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    /// Summary.
    pub summary: Summary,
    /// Files, sorted by path.
    pub files: Vec<FileReport>,
    /// Capability changes (diff mode; empty for `scan`).
    #[serde(default)]
    pub changes: Vec<Change>,
    /// Lockfile status.
    #[serde(default)]
    pub lock: LockStatus,
}

/// Direction of a capability change (PRD §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ChangeOp {
    /// `+` something new.
    #[serde(rename = "+")]
    Added,
    /// `-` something removed.
    #[serde(rename = "-")]
    Removed,
    /// `~` something changed.
    #[serde(rename = "~")]
    Changed,
    /// `!` a lock violation.
    #[serde(rename = "!")]
    Lock,
}

impl ChangeOp {
    /// The one-character symbol.
    pub fn symbol(self) -> &'static str {
        match self {
            ChangeOp::Added => "+",
            ChangeOp::Removed => "-",
            ChangeOp::Changed => "~",
            ChangeOp::Lock => "!",
        }
    }
}

/// One capability change between two revisions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Change {
    /// Agent file path.
    pub path: String,
    /// `+`, `-`, `~`, or `!`.
    pub op: ChangeOp,
    /// What changed: `file`, `mcp_server`, `hook`, `permission`,
    /// `allowed_tools`, `always_apply`, `instructions`, or `lock`.
    pub what: String,
    /// Name of the item (server name, hook event, permission rule), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// English detail.
    pub detail: String,
}

/// Lockfile state for a report.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LockStatus {
    /// Whether `rulesift.lock` exists.
    pub present: bool,
    /// Lock violations (RS-LOCK-00x).
    #[serde(default)]
    pub violations: Vec<LockViolation>,
}

/// One lockfile mismatch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LockViolation {
    /// `RS-LOCK-001` changed, `-002` new and unlocked, `-003` missing.
    pub rule_id: String,
    /// File or skill path.
    pub path: String,
    /// MCP server name, for server entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// English message.
    pub message: String,
}

impl Report {
    /// Recompute `summary` from `files` with the given thresholds.
    pub fn summarize(files: &[FileReport]) -> Summary {
        let mut s = Summary {
            files_scanned: files.len(),
            ..Summary::default()
        };
        for f in files {
            s.findings += f.findings.iter().filter(|x| !x.suppressed).count();
            match f.tier {
                Tier::Block => s.block += 1,
                Tier::Review => s.review += 1,
                Tier::Info => s.info += 1,
                Tier::Ok => s.ok += 1,
            }
            if f.score > s.repo_score {
                s.repo_score = f.score;
                s.repo_tier = f.tier;
            }
        }
        if s.repo_score == 0 {
            s.repo_tier = Tier::Ok;
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_thresholds() {
        assert_eq!(Tier::from_score(0), Tier::Ok);
        assert_eq!(Tier::from_score(1), Tier::Info);
        assert_eq!(Tier::from_score(39), Tier::Info);
        assert_eq!(Tier::from_score(40), Tier::Review);
        assert_eq!(Tier::from_score(69), Tier::Review);
        assert_eq!(Tier::from_score(70), Tier::Block);
        assert_eq!(Tier::from_score(100), Tier::Block);
        assert_eq!(Tier::from_score_with(50, 30, 50), Tier::Block);
        assert_eq!(Tier::from_score_with(30, 30, 50), Tier::Review);
    }

    #[test]
    fn tier_labels_and_order() {
        assert_eq!(Tier::Block.label(), "BLOCK");
        assert_eq!(Tier::Review.label(), "REVIEW");
        assert_eq!(Tier::Info.label(), "INFO");
        assert_eq!(Tier::Ok.label(), "OK");
        assert!(Tier::Ok < Tier::Info && Tier::Info < Tier::Review && Tier::Review < Tier::Block);
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
        let mut v = vec![
            Severity::High,
            Severity::Low,
            Severity::Critical,
            Severity::Medium,
        ];
        v.sort();
        assert_eq!(
            v,
            vec![
                Severity::Low,
                Severity::Medium,
                Severity::High,
                Severity::Critical
            ]
        );
    }

    #[test]
    fn enums_roundtrip_strings_and_serde() {
        for k in FileKind::ALL {
            assert_eq!(k.as_str().parse::<FileKind>().unwrap(), *k);
            let j = serde_json::to_string(k).unwrap();
            assert_eq!(j, format!("\"{}\"", k.as_str()));
            assert_eq!(serde_json::from_str::<FileKind>(&j).unwrap(), *k);
        }
        for t in Tool::ALL {
            assert_eq!(t.as_str().parse::<Tool>().unwrap(), *t);
        }
        assert_eq!(FileKind::ALL.len(), 8);
        assert_eq!(Tool::ALL.len(), 15);
        assert_eq!(Scope::CodeBlock.as_str(), "code_block");
        assert!("nope"
            .parse::<Tier>()
            .unwrap_err()
            .contains("expected one of"));
    }

    #[test]
    fn report_serde_roundtrip() {
        let f = Finding {
            rule_id: "RS-UNI-001".into(),
            severity: Severity::Critical,
            title: "Hidden Unicode Tag characters".into(),
            message: "m".into(),
            path: "AGENTS.md".into(),
            line: 1,
            column: 1,
            end_line: 1,
            evidence: "<U+E0061>".into(),
            decoded: Some("a".into()),
            fingerprint: "sha256:00".into(),
            suppressed: false,
        };
        let fr = FileReport {
            path: "AGENTS.md".into(),
            tool: Tool::AgentsMd,
            kind: FileKind::Instructions,
            score: 100,
            tier: Tier::Block,
            breakdown: Breakdown {
                weights: 70,
                multiplier: 1.5,
                bonus: 0,
            },
            findings: vec![f],
        };
        let files = vec![fr];
        let r = Report {
            schema_version: 1,
            tool: ToolInfo::default(),
            root: ".".into(),
            generated_at: None,
            summary: Report::summarize(&files),
            files,
            changes: vec![],
            lock: LockStatus::default(),
        };
        assert_eq!(r.summary.block, 1);
        assert_eq!(r.summary.repo_tier, Tier::Block);
        let s = serde_json::to_string(&r).unwrap();
        let back: Report = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
        let seg = Segment::new(Scope::Text, "hi", Span { start: 0, end: 2 }, 1, 1, 1);
        let d = Document {
            path: "a".into(),
            tool: Tool::Cursor,
            kind: FileKind::Instructions,
            always_loaded: false,
            text: "hi".into(),
            segments: vec![seg],
        };
        let back: Document = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(back, d);
    }
}
