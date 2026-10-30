//! `rulesift.toml` configuration (PRD §7.1).
//!
//! Unknown keys are errors (`deny_unknown_fields`) and every `[[suppress]]`
//! entry must carry a non-empty `reason`.

use crate::model::{string_enum, FileKind, Severity, Tier, Tool};
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// File name searched for when walking up from the scan path.
pub const CONFIG_FILE_NAME: &str = "rulesift.toml";
/// Environment variable naming an explicit config file.
pub const CONFIG_ENV: &str = "RULESIFT_CONFIG";

string_enum! {
    /// Tier at which `scan` exits with status 1.
    pub enum FailOn {
        /// Fail if any file is `block`.
        Block => "block",
        /// Fail if any file is `review` or worse.
        Review => "review",
        /// Fail if any file has a finding.
        Info => "info",
        /// Never fail on findings.
        Never => "never",
    }
}

impl FailOn {
    /// Whether a file at `tier` trips this policy.
    pub fn trips(self, tier: Tier) -> bool {
        match self {
            FailOn::Never => false,
            FailOn::Block => tier >= Tier::Block,
            FailOn::Review => tier >= Tier::Review,
            FailOn::Info => tier >= Tier::Info,
        }
    }
}

string_enum! {
    /// Agent CLI used by `--explain`.
    pub enum AgentChoice {
        /// Pick the first available agent.
        Auto => "auto",
        /// OpenAI Codex CLI.
        Codex => "codex",
        /// Claude Code CLI.
        Claude => "claude",
        /// Cursor agent CLI.
        Cursor => "cursor",
    }
}

/// Full configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// `[scan]`.
    pub scan: ScanConfig,
    /// `[policy]`.
    pub policy: PolicyConfig,
    /// `[score]`.
    pub score: ScoreConfig,
    /// `[rules]`.
    pub rules: RulesConfig,
    /// `[[suppress]]`.
    pub suppress: Vec<Suppression>,
    /// `[explain]`.
    pub explain: ExplainConfig,
}

/// `[scan]` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ScanConfig {
    /// Paths to scan when none are given on the command line.
    pub paths: Vec<String>,
    /// Include globs (empty = everything the registry recognizes).
    pub include: Vec<String>,
    /// Exclude globs, relative to the scan root.
    pub exclude: Vec<String>,
    /// Kinds to scan.
    pub kinds: Vec<FileKind>,
    /// Tools to scan (empty = all).
    pub tools: Vec<Tool>,
    /// Files larger than this are skipped with a warning.
    pub max_file_size_kib: u64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            paths: vec![".".into()],
            include: Vec::new(),
            exclude: vec!["node_modules/**".into(), "target/**".into()],
            kinds: FileKind::ALL.to_vec(),
            tools: Vec::new(),
            max_file_size_kib: 2048,
        }
    }
}

/// `[policy]` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PolicyConfig {
    /// Exit 1 when a file reaches this tier.
    pub fail_on: FailOn,
    /// Hide findings below this severity in output.
    pub min_severity: Severity,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            fail_on: FailOn::Block,
            min_severity: Severity::Low,
        }
    }
}

/// `[score]` section (PRD §9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ScoreConfig {
    /// Weight of a critical finding.
    pub critical: u32,
    /// Weight of a high finding.
    pub high: u32,
    /// Weight of a medium finding.
    pub medium: u32,
    /// Weight of a low finding.
    pub low: u32,
    /// Multiplier for always-loaded files.
    pub always_loaded_multiplier: f64,
    /// Bonus for the toxic combination.
    pub toxic_combo_bonus: u32,
    /// Score at which a file is `block`.
    pub block_at: u32,
    /// Score at which a file is `review`.
    pub review_at: u32,
}

impl Default for ScoreConfig {
    fn default() -> Self {
        Self {
            critical: 70,
            high: 25,
            medium: 10,
            low: 3,
            always_loaded_multiplier: 1.5,
            toxic_combo_bonus: 20,
            block_at: 70,
            review_at: 40,
        }
    }
}

impl ScoreConfig {
    /// Weight for a severity.
    pub fn weight(&self, s: Severity) -> u32 {
        match s {
            Severity::Critical => self.critical,
            Severity::High => self.high,
            Severity::Medium => self.medium,
            Severity::Low => self.low,
        }
    }
}

/// `[rules]` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct RulesConfig {
    /// Extra rule directories (relative to the config file).
    pub extra_dirs: Vec<String>,
    /// Rule ids to disable.
    pub disable: Vec<String>,
    /// Per-rule severity overrides.
    pub severity_override: BTreeMap<String, Severity>,
}

/// One `[[suppress]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suppression {
    /// Rule id to suppress.
    pub rule: String,
    /// Optional path glob (relative to the scan root); all files if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Why this is acceptable (required).
    pub reason: String,
}

/// `[explain]` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ExplainConfig {
    /// Which agent to use.
    pub agent: AgentChoice,
    /// Timeout per agent call.
    pub timeout_secs: u64,
    /// Max findings passed to the agent.
    pub max_findings: usize,
}

impl Default for ExplainConfig {
    fn default() -> Self {
        Self {
            agent: AgentChoice::Auto,
            timeout_secs: 120,
            max_findings: 10,
        }
    }
}

impl Config {
    /// Parse config text. `path` is only used in error messages.
    pub fn parse(text: &str, path: &Path) -> Result<Config> {
        let cfg: Config = toml::from_str(text).map_err(|e| CoreError::Config {
            path: path.to_path_buf(),
            message: describe_toml_error(&e),
        })?;
        cfg.validate(path)?;
        Ok(cfg)
    }

    /// Read and parse a config file.
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
        Self::parse(&text, path)
    }

    fn validate(&self, path: &Path) -> Result<()> {
        let err = |message: String| CoreError::Config {
            path: path.to_path_buf(),
            message,
        };
        for (i, s) in self.suppress.iter().enumerate() {
            if s.reason.trim().is_empty() {
                return Err(err(format!(
                    "suppress[{i}] (rule {:?}): key `reason` must not be empty",
                    s.rule
                )));
            }
            if s.rule.trim().is_empty() {
                return Err(err(format!("suppress[{i}]: key `rule` must not be empty")));
            }
        }
        let sc = &self.score;
        if sc.review_at == 0 || sc.review_at > sc.block_at {
            return Err(err(format!(
                "score: key `review_at` ({}) must be between 1 and `block_at` ({})",
                sc.review_at, sc.block_at
            )));
        }
        if !(sc.always_loaded_multiplier.is_finite() && sc.always_loaded_multiplier >= 1.0) {
            return Err(err(
                "score: key `always_loaded_multiplier` must be a number >= 1.0".into(),
            ));
        }
        if self.explain.timeout_secs == 0 {
            return Err(err(
                "explain: key `timeout_secs` must be greater than 0".into()
            ));
        }
        Ok(())
    }
}

/// How `head` weakens the policy of `base` (#160): every setting that can
/// hide a file or a finding, lower a severity or a tier, or loosen the exit
/// policy. Empty if `head` is at least as strict. `severity_of` gives a
/// rule's built-in severity.
pub fn weakenings(
    base: &Config,
    head: &Config,
    severity_of: &dyn Fn(&str) -> Option<Severity>,
) -> Vec<String> {
    let mut out = Vec::new();
    let added = |b: &[String], h: &[String]| -> Vec<String> {
        h.iter().filter(|x| !b.contains(x)).cloned().collect()
    };
    let (bs, hs) = (&base.scan, &head.scan);
    let list = |v: &[String]| {
        v.iter()
            .map(|x| format!("`{x}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let a = added(&bs.exclude, &hs.exclude);
    if !a.is_empty() {
        out.push(format!("scan.exclude adds {}", list(&a)));
    }
    if hs.include != bs.include && !hs.include.is_empty() {
        out.push(format!(
            "scan.include limits the scan to {}",
            list(&hs.include)
        ));
    }
    if hs.paths != bs.paths {
        out.push(format!("scan.paths changes to {}", list(&hs.paths)));
    }
    let narrowed = |b: Vec<String>, h: Vec<String>| -> Vec<String> {
        if h.is_empty() {
            return Vec::new();
        }
        if b.is_empty() {
            return h;
        }
        b.into_iter().filter(|x| !h.contains(x)).collect()
    };
    let kinds = |v: &[crate::model::FileKind]| v.iter().map(|k| k.to_string()).collect::<Vec<_>>();
    let all_kinds = kinds(crate::model::FileKind::ALL);
    let bk = if bs.kinds.is_empty() {
        all_kinds.clone()
    } else {
        kinds(&bs.kinds)
    };
    let hk = if hs.kinds.is_empty() {
        all_kinds
    } else {
        kinds(&hs.kinds)
    };
    let dropped: Vec<String> = bk.into_iter().filter(|k| !hk.contains(k)).collect();
    if !dropped.is_empty() {
        out.push(format!("scan.kinds no longer scans {}", list(&dropped)));
    }
    let tools = |v: &[crate::model::Tool]| v.iter().map(|k| k.to_string()).collect::<Vec<_>>();
    let t = narrowed(tools(&bs.tools), tools(&hs.tools));
    if !t.is_empty() {
        out.push(if bs.tools.is_empty() {
            format!("scan.tools limits the scan to {}", list(&tools(&hs.tools)))
        } else {
            format!("scan.tools no longer scans {}", list(&t))
        });
    }
    let a = added(&base.rules.disable, &head.rules.disable);
    if !a.is_empty() {
        out.push(format!("rules.disable adds {}", list(&a)));
    }
    for (id, sev) in &head.rules.severity_override {
        let before = base
            .rules
            .severity_override
            .get(id)
            .copied()
            .or_else(|| severity_of(id));
        if before.is_none_or(|b| *sev < b) {
            out.push(format!("rules.severity_override lowers `{id}` to {sev}"));
        }
    }
    let gone: Vec<String> = base
        .rules
        .extra_dirs
        .iter()
        .filter(|d| !head.rules.extra_dirs.contains(d))
        .cloned()
        .collect();
    if !gone.is_empty() {
        out.push(format!("rules.extra_dirs drops {}", list(&gone)));
    }
    let new_supp: Vec<String> = head
        .suppress
        .iter()
        .filter(|s| !base.suppress.contains(s))
        .map(|s| {
            format!(
                "{} on {}",
                s.rule,
                s.path.as_deref().unwrap_or("every file")
            )
        })
        .collect();
    if !new_supp.is_empty() {
        out.push(format!("suppress adds {}", new_supp.join(", ")));
    }
    let (b, h) = (&base.score, &head.score);
    for (name, bv, hv) in [
        ("block_at", b.block_at, h.block_at),
        ("review_at", b.review_at, h.review_at),
    ] {
        if hv > bv {
            out.push(format!("score.{name} raises {bv} to {hv}"));
        }
    }
    for (name, bv, hv) in [
        ("critical", b.critical, h.critical),
        ("high", b.high, h.high),
        ("medium", b.medium, h.medium),
        ("low", b.low, h.low),
        (
            "toxic_combo_bonus",
            b.toxic_combo_bonus,
            h.toxic_combo_bonus,
        ),
    ] {
        if hv < bv {
            out.push(format!("score.{name} lowers {bv} to {hv}"));
        }
    }
    if h.always_loaded_multiplier < b.always_loaded_multiplier {
        out.push(format!(
            "score.always_loaded_multiplier lowers {} to {}",
            b.always_loaded_multiplier, h.always_loaded_multiplier
        ));
    }
    let rank = |f: FailOn| match f {
        FailOn::Info => 0,
        FailOn::Review => 1,
        FailOn::Block => 2,
        FailOn::Never => 3,
    };
    if rank(head.policy.fail_on) > rank(base.policy.fail_on) {
        out.push(format!(
            "policy.fail_on loosens {} to {}",
            base.policy.fail_on, head.policy.fail_on
        ));
    }
    if head.policy.min_severity > base.policy.min_severity {
        out.push(format!(
            "policy.min_severity hides findings below {}",
            head.policy.min_severity
        ));
    }
    out
}

fn describe_toml_error(e: &toml::de::Error) -> String {
    // `toml` names the key, e.g. "unknown field `foo`, expected one of ...".
    e.message().trim().to_string()
}

/// Where the effective config came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    /// `--config <FILE>`.
    Flag(PathBuf),
    /// `RULESIFT_CONFIG`.
    Env(PathBuf),
    /// Found by walking up from the first scan path.
    Discovered(PathBuf),
    /// No file; defaults.
    Default,
}

impl ConfigSource {
    /// The config file path, if any.
    pub fn path(&self) -> Option<&Path> {
        match self {
            ConfigSource::Flag(p) | ConfigSource::Env(p) | ConfigSource::Discovered(p) => Some(p),
            ConfigSource::Default => None,
        }
    }
}

impl fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.path() {
            Some(p) => write!(f, "{}", p.display()),
            None => f.write_str("defaults"),
        }
    }
}

/// Find the config: `--config` wins, then `RULESIFT_CONFIG` (passed in by the
/// caller so core never reads the environment), then the nearest
/// `rulesift.toml` walking up from `start`.
pub fn resolve(flag: Option<&Path>, env: Option<&Path>, start: &Path) -> ConfigSource {
    if let Some(p) = flag {
        return ConfigSource::Flag(p.to_path_buf());
    }
    if let Some(p) = env.filter(|p| !p.as_os_str().is_empty()) {
        return ConfigSource::Env(p.to_path_buf());
    }
    let start = if start.is_file() {
        start.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        start.to_path_buf()
    };
    let abs = std::fs::canonicalize(&start).unwrap_or(start);
    let mut dir: Option<&Path> = Some(abs.as_path());
    while let Some(d) = dir {
        let candidate = d.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return ConfigSource::Discovered(candidate);
        }
        dir = d.parent();
    }
    ConfigSource::Default
}

/// Resolve and load. Returns defaults when no file is found.
pub fn load_resolved(
    flag: Option<&Path>,
    env: Option<&Path>,
    start: &Path,
) -> Result<(Config, ConfigSource)> {
    let src = resolve(flag, env, start);
    let cfg = match src.path() {
        Some(p) => Config::load(p)?,
        None => Config::default(),
    };
    Ok((cfg, src))
}

impl FromStr for Config {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self> {
        Config::parse(s, Path::new(CONFIG_FILE_NAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/config")
            .join(name)
    }

    #[test]
    fn defaults_match_prd() {
        let c = Config::default();
        assert_eq!(c.scan.paths, vec!["."]);
        assert_eq!(c.scan.max_file_size_kib, 2048);
        assert_eq!(c.scan.kinds.len(), 8);
        assert_eq!(c.policy.fail_on, FailOn::Block);
        assert_eq!(c.score.critical, 70);
        assert_eq!(c.score.always_loaded_multiplier, 1.5);
        assert_eq!(c.explain.timeout_secs, 120);
        assert_eq!(c, "".parse::<Config>().unwrap());
    }

    #[test]
    fn full_prd_example_parses() {
        let c = Config::load(&fixture("valid-full.toml")).unwrap();
        assert_eq!(c.scan.exclude.len(), 3);
        assert_eq!(
            c.scan.tools,
            vec![Tool::ClaudeCode, Tool::Cursor, Tool::GeminiCli]
        );
        assert_eq!(c.rules.disable, vec!["RS-EXF-003"]);
        assert_eq!(c.rules.severity_override["RS-SEC-003"], Severity::Low);
        assert_eq!(c.suppress.len(), 1);
        assert_eq!(
            c.suppress[0].path.as_deref(),
            Some("skills/installer/SKILL.md")
        );
        assert_eq!(c.explain.agent, AgentChoice::Auto);
    }

    #[test]
    fn minimal_parses_with_defaults() {
        let c = Config::load(&fixture("valid-minimal.toml")).unwrap();
        assert_eq!(c.policy.fail_on, FailOn::Review);
        assert_eq!(c.score, ScoreConfig::default());
    }

    fn err_of(name: &str) -> String {
        Config::load(&fixture(name)).unwrap_err().to_string()
    }

    #[test]
    fn unknown_key_is_named() {
        let e = err_of("invalid-unknown-key.toml");
        assert!(e.contains("unknown field `max_size`"), "{e}");
        assert!(e.contains("invalid-unknown-key.toml"), "{e}");
    }

    #[test]
    fn unknown_section_is_named() {
        let e = err_of("invalid-unknown-section.toml");
        assert!(e.contains("unknown field `telemetry`"), "{e}");
    }

    #[test]
    fn suppress_requires_reason() {
        let e = err_of("invalid-suppress-no-reason.toml");
        assert!(e.contains("missing field `reason`"), "{e}");
        let e = err_of("invalid-suppress-empty-reason.toml");
        assert!(e.contains("`reason` must not be empty"), "{e}");
    }

    #[test]
    fn bad_enum_value_is_reported() {
        let e = err_of("invalid-fail-on.toml");
        assert!(
            e.contains("fail_on") || e.contains("unknown variant"),
            "{e}"
        );
        assert!(e.contains("sometimes"), "{e}");
    }

    #[test]
    fn bad_thresholds() {
        let e = err_of("invalid-thresholds.toml");
        assert!(e.contains("`review_at`"), "{e}");
    }

    #[test]
    fn resolution_order() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join(CONFIG_FILE_NAME),
            "[policy]\nfail_on = \"info\"\n",
        )
        .unwrap();
        let sub = root.join("a/b");
        std::fs::create_dir_all(&sub).unwrap();
        // walk up
        let (c, src) = load_resolved(None, None, &sub).unwrap();
        assert!(matches!(src, ConfigSource::Discovered(_)));
        assert_eq!(c.policy.fail_on, FailOn::Info);
        // env beats walk-up
        let other = root.join("other.toml");
        std::fs::write(&other, "[policy]\nfail_on = \"never\"\n").unwrap();
        let (c, src) = load_resolved(None, Some(&other), &sub).unwrap();
        assert_eq!(src, ConfigSource::Env(other.clone()));
        assert_eq!(c.policy.fail_on, FailOn::Never);
        // flag beats env
        let flag = root.join("flag.toml");
        std::fs::write(&flag, "[policy]\nfail_on = \"review\"\n").unwrap();
        let (c, src) = load_resolved(Some(&flag), Some(&other), &sub).unwrap();
        assert_eq!(src, ConfigSource::Flag(flag));
        assert_eq!(c.policy.fail_on, FailOn::Review);
    }

    #[test]
    fn no_config_means_defaults() {
        let dir = tempfile::tempdir().unwrap();
        // A temp dir might sit under a directory with a rulesift.toml; only
        // assert that resolution does not error.
        let (c, _) = load_resolved(None, None, dir.path()).unwrap();
        assert_eq!(c.scan.max_file_size_kib, 2048);
    }

    #[test]
    fn fail_on_trips() {
        assert!(FailOn::Block.trips(Tier::Block));
        assert!(!FailOn::Block.trips(Tier::Review));
        assert!(FailOn::Review.trips(Tier::Review));
        assert!(FailOn::Info.trips(Tier::Info));
        assert!(!FailOn::Info.trips(Tier::Ok));
        assert!(!FailOn::Never.trips(Tier::Block));
    }
}
