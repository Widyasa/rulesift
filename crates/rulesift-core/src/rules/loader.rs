//! Load, validate, and compile rules: built-ins embedded at compile time plus
//! user rule directories (PRD §4 F4).

use crate::error::{CoreError, Result};
use crate::model::{FileKind, Scope, Severity};
use crate::rules::detectors::KNOWN_DETECTORS;
use crate::rules::schema::{FrontmatterDef, MatchDef, RuleDef};
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex::{Regex, RegexBuilder};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

macro_rules! builtin_files {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_str!(concat!("../../../../rules/builtin/", $name)))),*]
    };
}

/// Built-in rule files (name, YAML source). A test checks that every file
/// in `rules/builtin/` is listed here.
pub const BUILTIN: &[(&str, &str)] = builtin_files!(
    "exec.yaml",
    "fmt.yaml",
    "hid.yaml",
    "inj.yaml",
    "sec.yaml",
    "uni.yaml",
);

/// Max regex pattern length in bytes.
pub const MAX_PATTERN_LEN: usize = 4096;
/// Compiled regex program size limit (bytes).
pub const REGEX_SIZE_LIMIT: usize = 1 << 20;
/// Lazy DFA cache size limit (bytes).
pub const REGEX_DFA_LIMIT: usize = 2 << 20;
/// Max phrases per `phrases` matcher.
pub const MAX_PHRASES: usize = 2000;
/// Max rule file size (bytes).
pub const MAX_RULE_FILE: u64 = 1 << 20;
/// Max matcher nesting depth.
pub const MAX_DEPTH: usize = 16;

/// Where a rule came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleSource {
    /// Embedded built-in file.
    Builtin(&'static str),
    /// User rule file.
    User(PathBuf),
}

impl RuleSource {
    /// Display name (`builtin:<file>` or the path).
    pub fn display(&self) -> String {
        match self {
            RuleSource::Builtin(n) => format!("builtin:{n}"),
            RuleSource::User(p) => p.display().to_string().replace('\\', "/"),
        }
    }
}

/// Compiled frontmatter condition.
#[derive(Debug, Clone)]
pub enum FmCond {
    /// Equals a value.
    Equals(serde_json::Value),
    /// String or list element matches.
    Matches(Regex),
    /// Present or absent.
    Present(bool),
}

/// Compiled matcher node.
#[derive(Debug, Clone)]
pub enum Matcher {
    /// Regex.
    Regex(Regex),
    /// Phrases (ASCII case-insensitive, leftmost-longest).
    Phrases(AhoCorasick, Vec<String>),
    /// JSON pointer glob segments plus optional value regex.
    JsonPath(Vec<String>, Option<Regex>),
    /// Frontmatter field condition.
    Frontmatter(String, FmCond),
    /// Named detector.
    Detector(String),
    /// Any child.
    Any(Vec<Node>),
    /// All children.
    All(Vec<Node>),
    /// Negation.
    Not(Box<Node>),
}

/// Matcher plus its effective scope set (`None` = all scopes).
#[derive(Debug, Clone)]
pub struct Node {
    /// Matcher.
    pub matcher: Matcher,
    /// Effective scopes.
    pub scope: Option<BTreeSet<Scope>>,
}

/// A validated, compiled rule.
#[derive(Debug, Clone)]
pub struct Rule {
    /// Raw definition.
    pub def: RuleDef,
    /// Effective severity (after overrides).
    pub severity: Severity,
    /// Compiled matcher.
    pub node: Node,
    /// Origin.
    pub source: RuleSource,
}

impl Rule {
    /// Rule id.
    pub fn id(&self) -> &str {
        &self.def.id
    }
    /// Does the rule apply to files of `kind`?
    pub fn applies_to(&self, kind: FileKind) -> bool {
        self.def
            .applies_to
            .as_ref()
            .is_none_or(|k| k.contains(&kind))
    }
}

/// Loader options.
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Include the built-in rules.
    pub builtin: bool,
    /// Extra rule directories (`*.yaml` / `*.yml`, non-recursive).
    pub extra_dirs: Vec<PathBuf>,
    /// Rule ids to disable.
    pub disable: Vec<String>,
    /// Severity overrides.
    pub severity_override: BTreeMap<String, Severity>,
}

/// Loaded rules, sorted by id.
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    /// Enabled rules.
    pub rules: Vec<Rule>,
    /// English warnings (unknown ids in disable/override lists).
    pub warnings: Vec<String>,
}

impl RuleSet {
    /// Find a rule by id.
    pub fn get(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.def.id == id)
    }
}

fn err(src: &RuleSource, id: &str, message: impl Into<String>) -> CoreError {
    CoreError::Rule {
        file: src.display(),
        rule_id: id.to_string(),
        message: message.into(),
    }
}

/// Is `id` well-formed: uppercase ASCII letters/digits in `-`-separated
/// groups, at least two groups, ≤ 64 chars.
pub fn valid_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('-').collect();
    id.len() <= 64
        && parts.len() >= 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        })
        && parts[0].starts_with(|c: char| c.is_ascii_uppercase())
}

/// Compile a regex with RuleSift's size limits.
pub fn compile_regex(pattern: &str) -> std::result::Result<Regex, String> {
    if pattern.len() > MAX_PATTERN_LEN {
        return Err(format!("regex is longer than {MAX_PATTERN_LEN} bytes"));
    }
    RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_DFA_LIMIT)
        .build()
        .map_err(|e| format!("invalid regex {pattern:?}: {e}"))
}

fn compile_node(
    m: &MatchDef,
    inherited: Option<&BTreeSet<Scope>>,
    depth: usize,
) -> std::result::Result<Node, String> {
    if depth > MAX_DEPTH {
        return Err(format!("matcher nesting is deeper than {MAX_DEPTH}"));
    }
    let scope: Option<BTreeSet<Scope>> = match &m.scope {
        Some(s) if s.is_empty() => return Err("`scope` must not be empty".into()),
        Some(s) => Some(s.iter().copied().collect()),
        None => inherited.cloned(),
    };
    let keys: Vec<&str> = [
        ("regex", m.regex.is_some()),
        ("phrases", m.phrases.is_some()),
        ("json_path", m.json_path.is_some()),
        ("frontmatter", m.frontmatter.is_some()),
        ("detector", m.detector.is_some()),
        ("any", m.any.is_some()),
        ("all", m.all.is_some()),
        ("not", m.not.is_some()),
    ]
    .iter()
    .filter(|(_, set)| *set)
    .map(|(k, _)| *k)
    .collect();
    if keys.len() != 1 {
        return Err(if keys.is_empty() {
            "matcher needs one of regex, phrases, json_path, frontmatter, detector, any, all, not"
                .into()
        } else {
            format!("matcher has more than one of: {}", keys.join(", "))
        });
    }
    let children = |v: &[MatchDef], name: &str| -> std::result::Result<Vec<Node>, String> {
        if v.is_empty() {
            return Err(format!("`{name}` must not be empty"));
        }
        v.iter()
            .map(|c| compile_node(c, scope.as_ref(), depth + 1))
            .collect()
    };
    let matcher = if let Some(p) = &m.regex {
        Matcher::Regex(compile_regex(p)?)
    } else if let Some(ph) = &m.phrases {
        if ph.is_empty() || ph.len() > MAX_PHRASES {
            return Err(format!("`phrases` must have 1 to {MAX_PHRASES} entries"));
        }
        if ph.iter().any(|p| p.trim().is_empty() || p.len() > 512) {
            return Err("phrases must be non-empty and at most 512 bytes".into());
        }
        let ac = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::LeftmostLongest)
            .build(ph)
            .map_err(|e| format!("cannot build phrase matcher: {e}"))?;
        Matcher::Phrases(ac, ph.clone())
    } else if let Some(j) = &m.json_path {
        if !j.pointer.starts_with('/') {
            return Err(format!(
                "json_path pointer {:?} must start with '/'",
                j.pointer
            ));
        }
        let segs = j.pointer[1..].split('/').map(str::to_string).collect();
        let value = j.value.as_deref().map(compile_regex).transpose()?;
        Matcher::JsonPath(segs, value)
    } else if let Some(f) = &m.frontmatter {
        Matcher::Frontmatter(f.field.clone(), compile_fm(f)?)
    } else if let Some(d) = &m.detector {
        if !KNOWN_DETECTORS.contains(&d.as_str()) {
            return Err(format!(
                "unknown detector {d:?} (known: {})",
                KNOWN_DETECTORS.join(", ")
            ));
        }
        Matcher::Detector(d.clone())
    } else if let Some(v) = &m.any {
        Matcher::Any(children(v, "any")?)
    } else if let Some(v) = &m.all {
        Matcher::All(children(v, "all")?)
    } else if let Some(n) = &m.not {
        Matcher::Not(Box::new(compile_node(n, scope.as_ref(), depth + 1)?))
    } else {
        // INVARIANT: exactly one key is set (checked above).
        unreachable!("matcher key checked above")
    };
    Ok(Node { matcher, scope })
}

fn compile_fm(f: &FrontmatterDef) -> std::result::Result<FmCond, String> {
    let n = [f.equals.is_some(), f.matches.is_some(), f.present.is_some()]
        .iter()
        .filter(|b| **b)
        .count();
    if n != 1 {
        return Err(format!(
            "frontmatter matcher for {:?} needs exactly one of equals, matches, present",
            f.field
        ));
    }
    Ok(if let Some(v) = &f.equals {
        FmCond::Equals(v.clone())
    } else if let Some(r) = &f.matches {
        FmCond::Matches(compile_regex(r)?)
    } else {
        FmCond::Present(f.present.unwrap_or(true))
    })
}

/// Parse one rule file (a single rule mapping or a list of rules).
pub fn parse_file(src: &RuleSource, yaml: &str) -> Result<Vec<RuleDef>> {
    let value: serde_json::Value = serde_saphyr::from_str(yaml)
        .map_err(|e| err(src, "<unknown>", format!("invalid YAML: {e}")))?;
    let items = match value {
        serde_json::Value::Array(v) => v,
        v @ serde_json::Value::Object(_) => vec![v],
        serde_json::Value::Null => Vec::new(),
        _ => {
            return Err(err(
                src,
                "<unknown>",
                "a rule file must contain a rule mapping or a list of rules",
            ))
        }
    };
    items
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let id = v
                .get("id")
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| format!("<rule #{}>", i + 1));
            serde_json::from_value::<RuleDef>(v).map_err(|e| err(src, &id, e.to_string()))
        })
        .collect()
}

/// Validate and compile one definition.
pub fn compile(src: &RuleSource, def: RuleDef) -> Result<Rule> {
    let id = def.id.clone();
    if !valid_id(&id) {
        return Err(err(
            src,
            &id,
            "rule id must be uppercase letters/digits in hyphen-separated groups, e.g. ACME-001",
        ));
    }
    match src {
        RuleSource::Builtin(_) if !id.starts_with("RS-") => {
            return Err(err(src, &id, "built-in rule ids must start with RS-"))
        }
        RuleSource::User(_) if id.starts_with("RS-") => return Err(err(
            src,
            &id,
            "user rule ids must not start with RS- (reserved for built-ins); use e.g. X- or ACME-",
        )),
        _ => {}
    }
    if def.title.trim().is_empty() || def.message.trim().is_empty() {
        return Err(err(src, &id, "`title` and `message` must not be empty"));
    }
    if def.applies_to.as_ref().is_some_and(|a| a.is_empty()) {
        return Err(err(
            src,
            &id,
            "`applies_to` must not be empty (omit it for all kinds)",
        ));
    }
    let node = compile_node(&def.matcher, None, 0).map_err(|m| err(src, &id, m))?;
    if let Some(ex) = &def.examples {
        if ex.should_match.is_empty() && ex.should_not_match.is_empty() {
            return Err(err(
                src,
                &id,
                "`examples` must list should_match or should_not_match",
            ));
        }
    }
    Ok(Rule {
        severity: def.severity,
        def,
        node,
        source: src.clone(),
    })
}

/// Load all rule files of a user directory, sorted by file name.
pub fn load_dir(dir: &Path) -> Result<Vec<Rule>> {
    let rd = std::fs::read_dir(dir).map_err(|e| CoreError::io(dir, e))?;
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "yaml" || x == "yml"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let src = RuleSource::User(f.clone());
        let len = std::fs::metadata(&f)
            .map_err(|e| CoreError::io(&f, e))?
            .len();
        if len > MAX_RULE_FILE {
            return Err(err(&src, "<unknown>", "rule file is larger than 1 MiB"));
        }
        let text = std::fs::read_to_string(&f).map_err(|e| CoreError::io(&f, e))?;
        for def in parse_file(&src, &text)? {
            out.push(compile(&src, def)?);
        }
    }
    Ok(out)
}

/// Load the built-in rules only.
pub fn builtin() -> Result<Vec<Rule>> {
    let mut out = Vec::new();
    for (name, yaml) in BUILTIN {
        let src = RuleSource::Builtin(name);
        for def in parse_file(&src, yaml)? {
            out.push(compile(&src, def)?);
        }
    }
    Ok(out)
}

/// Load rules per `opts`: built-ins, user dirs, duplicate-id check,
/// disables, and severity overrides. Result is sorted by id.
pub fn load(opts: &LoadOptions) -> Result<RuleSet> {
    let mut rules = if opts.builtin { builtin()? } else { Vec::new() };
    for d in &opts.extra_dirs {
        rules.extend(load_dir(d)?);
    }
    rules.sort_by(|a, b| a.def.id.cmp(&b.def.id));
    for w in rules.windows(2) {
        if w[0].def.id == w[1].def.id {
            return Err(err(
                &w[1].source,
                &w[1].def.id,
                format!(
                    "duplicate rule id (also defined in {})",
                    w[0].source.display()
                ),
            ));
        }
    }
    let mut warnings = Vec::new();
    let ids: BTreeSet<String> = rules.iter().map(|r| r.def.id.clone()).collect();
    for d in &opts.disable {
        if !ids.contains(d) {
            warnings.push(format!("disabled rule {d} does not exist"));
        }
    }
    for (id, sev) in &opts.severity_override {
        match rules.iter_mut().find(|r| &r.def.id == id) {
            Some(r) => r.severity = *sev,
            None => warnings.push(format!("severity override for unknown rule {id}")),
        }
    }
    rules.retain(|r| !opts.disable.contains(&r.def.id));
    Ok(RuleSet { rules, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(yaml: &str) -> Result<Vec<Rule>> {
        let src = RuleSource::User(PathBuf::from("rules/acme.yaml"));
        parse_file(&src, yaml)?
            .into_iter()
            .map(|d| compile(&src, d))
            .collect()
    }

    fn msg(yaml: &str) -> String {
        user(yaml).unwrap_err().to_string()
    }

    const OK: &str =
        "id: ACME-001\ntitle: Demo\nseverity: high\nmatch:\n  regex: 'evil'\nmessage: Demo.\n";

    #[test]
    fn all_builtins_load() {
        let rules = builtin().unwrap();
        let ids: Vec<&str> = rules.iter().map(|r| r.id()).collect();
        for want in [
            "RS-FMT-001",
            "RS-FMT-002",
            "RS-FMT-003",
            "RS-FMT-004",
            "RS-FMT-005",
            "RS-FMT-006",
            "RS-FMT-007",
            "RS-UNI-001",
            "RS-UNI-002",
            "RS-UNI-003",
            "RS-UNI-004",
        ] {
            assert!(ids.contains(&want), "{want} missing");
        }
        let set = load(&LoadOptions {
            builtin: true,
            ..Default::default()
        })
        .unwrap();
        // YAML escapes decode to the hidden characters.
        let ex = set_examples(&rules, "RS-UNI-001");
        assert!(ex.chars().any(|c| c == '\u{E0072}'));
        let sorted: Vec<&str> = set.rules.iter().map(|r| r.id()).collect();
        let mut s2 = sorted.clone();
        s2.sort();
        assert_eq!(sorted, s2);
    }

    fn set_examples(rules: &[Rule], id: &str) -> String {
        let r = rules.iter().find(|r| r.id() == id).unwrap();
        r.def.examples.as_ref().unwrap().should_match[0].clone()
    }

    #[test]
    fn every_builtin_file_is_embedded() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rules/builtin");
        let mut on_disk: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".yaml"))
            .collect();
        on_disk.sort();
        let embedded: Vec<String> = BUILTIN.iter().map(|(n, _)| n.to_string()).collect();
        assert_eq!(on_disk, embedded);
    }

    #[test]
    fn user_rule_ok_and_list_form() {
        assert_eq!(user(OK).unwrap()[0].id(), "ACME-001");
        let two = format!(
            "- {}\n- {}",
            OK.replace('\n', "\n  "),
            OK.replace("ACME-001", "ACME-002").replace('\n', "\n  ")
        );
        assert_eq!(user(&two).unwrap().len(), 2);
    }

    #[test]
    fn errors_name_file_and_rule() {
        let m = msg(&OK.replace("ACME-001", "RS-EVIL-001"));
        assert!(
            m.contains("rules/acme.yaml")
                && m.contains("RS-EVIL-001")
                && m.contains("must not start with RS-"),
            "{m}"
        );
        let m = msg(&OK.replace("'evil'", "'(unclosed'"));
        assert!(m.contains("ACME-001") && m.contains("invalid regex"), "{m}");
        let m = msg(&format!("{OK}colour: red\n"));
        assert!(
            m.contains("ACME-001") && m.contains("unknown field `colour`"),
            "{m}"
        );
        let m = msg(&OK.replace("severity: high", "severity: severe"));
        assert!(m.contains("ACME-001") && m.contains("severe"), "{m}");
        let m = msg(&OK.replace("regex: 'evil'", "detector: nope"));
        assert!(m.contains("unknown detector \"nope\""), "{m}");
        let m = msg(&OK.replace("regex: 'evil'", "regex: 'a'\n  phrases: [b]"));
        assert!(m.contains("more than one of: regex, phrases"), "{m}");
        let m = msg("id: [unclosed\n");
        assert!(m.contains("invalid YAML") && m.contains("<unknown>"), "{m}");
        let m = msg(&OK.replace("id: ACME-001\n", ""));
        assert!(
            m.contains("<rule #1>") && m.contains("missing field `id`"),
            "{m}"
        );
        let m = msg(&OK.replace("ACME-001", "acme 1"));
        assert!(m.contains("rule id must be"), "{m}");
    }

    #[test]
    fn regex_size_limit() {
        let huge = "(\\w{1000}){1000}";
        let m = msg(&OK.replace("'evil'", &format!("'{huge}'")));
        assert!(m.contains("invalid regex"), "{m}");
        let long = "a".repeat(MAX_PATTERN_LEN + 1);
        let m = msg(&OK.replace("evil", &long));
        assert!(m.contains("longer than"), "{m}");
    }

    #[test]
    fn scope_inherited() {
        let y = OK.replace(
            "  regex: 'evil'\n",
            "  any:\n    - regex: a\n    - regex: b\n      scope: [code_block]\n  scope: [text]\n",
        );
        let r = &user(&y).unwrap()[0];
        let Matcher::Any(kids) = &r.node.matcher else {
            panic!()
        };
        assert_eq!(kids[0].scope, Some([Scope::Text].into()));
        assert_eq!(kids[1].scope, Some([Scope::CodeBlock].into()));
    }

    #[test]
    fn user_dirs_disable_override_duplicates() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.yaml"), OK).unwrap();
        std::fs::write(d.path().join("ignored.txt"), "x").unwrap();
        let mut opts = LoadOptions {
            builtin: true,
            extra_dirs: vec![d.path().to_path_buf()],
            disable: vec!["RS-UNI-004".into(), "RS-NOPE-001".into()],
            ..Default::default()
        };
        opts.severity_override
            .insert("ACME-001".into(), Severity::Low);
        let set = load(&opts).unwrap();
        assert!(set.get("RS-UNI-004").is_none());
        assert_eq!(set.get("ACME-001").unwrap().severity, Severity::Low);
        assert_eq!(
            set.warnings,
            vec!["disabled rule RS-NOPE-001 does not exist".to_string()]
        );
        std::fs::write(d.path().join("b.yml"), OK).unwrap();
        let m = load(&opts).unwrap_err().to_string();
        assert!(
            m.contains("duplicate rule id") && m.contains("ACME-001"),
            "{m}"
        );
    }
}
