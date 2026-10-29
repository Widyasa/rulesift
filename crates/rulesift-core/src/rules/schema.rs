//! YAML rule schema (PRD §8). Raw definitions as written in rule files;
//! [`crate::rules::loader`] validates and compiles them.

use crate::model::{FileKind, Scope, Severity};
use serde::{Deserialize, Serialize};

/// One rule as written in YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDef {
    /// Rule id, e.g. `RS-EXEC-001` (built-in) or `ACME-001` (user).
    pub id: String,
    /// Short English title.
    pub title: String,
    /// Default severity.
    pub severity: Severity,
    /// Category, e.g. `dangerous-shell`.
    #[serde(default)]
    pub category: Option<String>,
    /// File kinds the rule applies to; all kinds if absent.
    #[serde(default)]
    pub applies_to: Option<Vec<FileKind>>,
    /// Matcher tree.
    #[serde(rename = "match")]
    pub matcher: MatchDef,
    /// English message shown with each finding.
    pub message: String,
    /// Inline examples run by `rulesift rules test`.
    #[serde(default)]
    pub examples: Option<Examples>,
    /// Reference URLs.
    #[serde(default)]
    pub references: Vec<String>,
}

/// Inline examples.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Examples {
    /// Path the example text is scanned as (default chosen from `applies_to`).
    #[serde(default)]
    pub path: Option<String>,
    /// Texts that must produce at least one finding of this rule.
    #[serde(default)]
    pub should_match: Vec<String>,
    /// Texts that must not produce a finding of this rule.
    #[serde(default)]
    pub should_not_match: Vec<String>,
}

/// One matcher node: exactly one of the matcher or combinator keys, plus an
/// optional `scope` filter inherited by child nodes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchDef {
    /// Regular expression (Rust `regex` syntax, linear time).
    #[serde(default)]
    pub regex: Option<String>,
    /// Case-insensitive phrase list.
    #[serde(default)]
    pub phrases: Option<Vec<String>>,
    /// JSON pointer glob with optional value regex.
    #[serde(default)]
    pub json_path: Option<JsonPathDef>,
    /// Frontmatter field condition.
    #[serde(default)]
    pub frontmatter: Option<FrontmatterDef>,
    /// Name of a built-in Rust detector.
    #[serde(default)]
    pub detector: Option<String>,
    /// Matches if any child matches.
    #[serde(default)]
    pub any: Option<Vec<MatchDef>>,
    /// Matches if all children match (in the same file).
    #[serde(default)]
    pub all: Option<Vec<MatchDef>>,
    /// Matches if the child does not match (in the same file).
    #[serde(default)]
    pub not: Option<Box<MatchDef>>,
    /// Segment scopes to scan; inherited by children; all scopes if absent.
    #[serde(default)]
    pub scope: Option<Vec<Scope>>,
}

/// `json_path` matcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonPathDef {
    /// JSON pointer glob: `*` matches one segment, `**` any number.
    pub pointer: String,
    /// Optional regex on the value (strings as-is, other scalars as JSON).
    #[serde(default)]
    pub value: Option<String>,
}

/// `frontmatter` matcher: a top-level field plus one condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontmatterDef {
    /// Field name.
    pub field: String,
    /// Field equals this value.
    #[serde(default)]
    pub equals: Option<serde_json::Value>,
    /// Field (or any list element) matches this regex.
    #[serde(default)]
    pub matches: Option<String>,
    /// Field is present (`true`) or absent (`false`).
    #[serde(default)]
    pub present: Option<bool>,
}
