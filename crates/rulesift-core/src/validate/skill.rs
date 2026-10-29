//! Agent Skills `SKILL.md` validation (RS-FMT-001..005).
//!
//! Spec: <https://agentskills.io/specification>. These checks are hard-coded
//! for now and move to YAML rules referencing detectors in W3-T3.

use crate::parse::frontmatter::{self, Frontmatter};
use serde_json::Value;

/// Frontmatter fields allowed by the Agent Skills spec.
pub const ALLOWED_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];
/// Max `name` length.
pub const MAX_NAME: usize = 64;
/// Max `description` length.
pub const MAX_DESCRIPTION: usize = 1024;
/// Max `compatibility` length.
pub const MAX_COMPATIBILITY: usize = 500;

/// A raw skill-format problem (rule id, line, message, evidence).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillIssue {
    /// Rule id (`RS-FMT-001` .. `RS-FMT-005`).
    pub rule_id: &'static str,
    /// 1-based line in the file.
    pub line: usize,
    /// English message.
    pub message: String,
    /// Evidence snippet.
    pub evidence: String,
}

/// Is `name` valid per the spec: 1–64 chars, lowercase ASCII letters, digits,
/// and hyphens, no leading/trailing/double hyphen.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

fn line_text(text: &str, line: usize) -> String {
    text.lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Check a `SKILL.md`. `dir_name` is the name of the directory containing it.
pub fn check(text: &str, dir_name: Option<&str>) -> Vec<SkillIssue> {
    let mut out = Vec::new();
    let fm: Frontmatter = match frontmatter::split(text) {
        Some(fm) => fm,
        None => {
            out.push(SkillIssue {
                rule_id: "RS-FMT-001",
                line: 1,
                message: "SKILL.md has no YAML frontmatter; add a `---` block with `name` and `description`.".into(),
                evidence: line_text(text, 1),
            });
            return out;
        }
    };
    let map = match &fm.value {
        Err((msg, line)) => {
            out.push(SkillIssue {
                rule_id: "RS-FMT-001",
                line: *line,
                message: format!("SKILL.md frontmatter cannot be parsed: {msg}."),
                evidence: line_text(text, *line),
            });
            return out;
        }
        Ok(Value::Object(m)) => m,
        Ok(_) => {
            out.push(SkillIssue {
                rule_id: "RS-FMT-001",
                line: 2,
                message: "SKILL.md frontmatter must be a YAML mapping of fields.".into(),
                evidence: line_text(text, 2),
            });
            return out;
        }
    };
    let ev = |key: &str| line_text(text, fm.line_of(key));

    // RS-FMT-002 / 003: name
    match map.get("name") {
        Some(Value::String(name)) if valid_name(name) => {
            if let Some(dir) = dir_name {
                if dir != name {
                    out.push(SkillIssue {
                        rule_id: "RS-FMT-003",
                        line: fm.line_of("name"),
                        message: format!("Skill name {name:?} does not match its directory {dir:?}."),
                        evidence: ev("name"),
                    });
                }
            }
        }
        Some(Value::String(name)) => out.push(SkillIssue {
            rule_id: "RS-FMT-002",
            line: fm.line_of("name"),
            message: format!(
                "Skill name {name:?} is invalid: use 1-64 lowercase letters, digits, and hyphens, with no leading, trailing, or double hyphen."
            ),
            evidence: ev("name"),
        }),
        Some(_) => out.push(SkillIssue {
            rule_id: "RS-FMT-002",
            line: fm.line_of("name"),
            message: "Skill `name` must be a string.".into(),
            evidence: ev("name"),
        }),
        None => out.push(SkillIssue {
            rule_id: "RS-FMT-002",
            line: fm.start_line,
            message: "Skill frontmatter is missing the required `name` field.".into(),
            evidence: line_text(text, fm.start_line),
        }),
    }

    // RS-FMT-004: description
    let desc_issue = match map.get("description") {
        None => Some("Skill frontmatter is missing the required `description` field.".to_string()),
        Some(Value::String(d)) if d.trim().is_empty() => {
            Some("Skill `description` is empty.".to_string())
        }
        Some(Value::String(d)) if d.chars().count() > MAX_DESCRIPTION => Some(format!(
            "Skill `description` is {} characters long; the limit is {MAX_DESCRIPTION}.",
            d.chars().count()
        )),
        Some(Value::String(_)) => None,
        Some(_) => Some("Skill `description` must be a string.".to_string()),
    };
    if let Some(message) = desc_issue {
        let line = if map.contains_key("description") {
            fm.line_of("description")
        } else {
            fm.start_line
        };
        out.push(SkillIssue {
            rule_id: "RS-FMT-004",
            line,
            message,
            evidence: line_text(text, line),
        });
    }

    // RS-FMT-005: unknown fields, compatibility length
    for key in map.keys() {
        if !ALLOWED_FIELDS.contains(&key.as_str()) {
            out.push(SkillIssue {
                rule_id: "RS-FMT-005",
                line: fm.line_of(key),
                message: format!(
                    "Unknown skill frontmatter field `{key}` (allowed: {}).",
                    ALLOWED_FIELDS.join(", ")
                ),
                evidence: ev(key),
            });
        }
    }
    if let Some(Value::String(c)) = map.get("compatibility") {
        if c.chars().count() > MAX_COMPATIBILITY {
            out.push(SkillIssue {
                rule_id: "RS-FMT-005",
                line: fm.line_of("compatibility"),
                message: format!(
                    "Skill `compatibility` is {} characters long; the limit is {MAX_COMPATIBILITY}.",
                    c.chars().count()
                ),
                evidence: ev("compatibility"),
            });
        }
    }
    out.sort_by(|a, b| (a.line, a.rule_id).cmp(&(b.line, b.rule_id)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn run(dir: &str) -> Vec<SkillIssue> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/skills")
            .join(dir)
            .join("SKILL.md");
        let text = std::fs::read_to_string(&p).unwrap();
        check(&text, Some(dir))
    }

    fn ids(v: &[SkillIssue]) -> Vec<(&str, usize)> {
        v.iter().map(|i| (i.rule_id, i.line)).collect()
    }

    #[test]
    fn valid_skill_has_no_findings() {
        assert!(run("valid").is_empty(), "{:?}", run("valid"));
    }

    #[test]
    fn fmt_001_missing_and_unparseable() {
        assert_eq!(ids(&run("invalid-no-frontmatter")), vec![("RS-FMT-001", 1)]);
        let r = run("invalid-yaml");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].rule_id, "RS-FMT-001");
        assert!(r[0].line >= 2 && r[0].line <= 4, "{:?}", r);
        assert!(r[0].message.contains("cannot be parsed"));
    }

    #[test]
    fn fmt_002_invalid_name() {
        assert_eq!(ids(&run("invalid-name")), vec![("RS-FMT-002", 2)]);
        for bad in ["", "-a", "a-", "a--b", "Abc", "a_b", &"a".repeat(65)] {
            assert!(!valid_name(bad), "{bad}");
        }
        for good in ["a", "pdf-tools", "x1-2", &"a".repeat(64)] {
            assert!(valid_name(good), "{good}");
        }
    }

    #[test]
    fn fmt_003_name_mismatch() {
        assert_eq!(ids(&run("invalid-name-mismatch")), vec![("RS-FMT-003", 2)]);
    }

    #[test]
    fn fmt_004_description() {
        assert_eq!(ids(&run("invalid-description")), vec![("RS-FMT-004", 3)]);
        let long = format!("---\nname: a\ndescription: {}\n---\n", "x".repeat(1025));
        assert_eq!(ids(&check(&long, Some("a"))), vec![("RS-FMT-004", 3)]);
        assert_eq!(
            ids(&check("---\nname: a\n---\n", Some("a"))),
            vec![("RS-FMT-004", 1)]
        );
    }

    #[test]
    fn fmt_005_unknown_field_and_compatibility() {
        assert_eq!(ids(&run("invalid-unknown-field")), vec![("RS-FMT-005", 4)]);
        let long = format!(
            "---\nname: a\ndescription: d\ncompatibility: {}\n---\n",
            "x".repeat(501)
        );
        assert_eq!(ids(&check(&long, Some("a"))), vec![("RS-FMT-005", 4)]);
    }
}
