//! Per-tool frontmatter dialects (RS-FMT-006) and the always-loaded
//! predicate (PRD §5.1, used by scoring §9).

use super::FormatIssue;
use crate::parse::frontmatter::Frontmatter;
use crate::registry::{AlwaysLoaded, Dialect, RegistryRow};
use serde_json::Value;

/// Expected type of a frontmatter field.
#[derive(Debug, Clone, Copy)]
enum Ty {
    Bool,
    Str,
    /// String or list of strings.
    StrOrList,
    /// One of the given strings.
    OneOf(&'static [&'static str]),
}

/// Field rules of a dialect: (key, type, required).
type Fields = &'static [(&'static str, Ty, bool)];

const CLAUDE_PERMISSION_MODES: &[&str] = &[
    "default",
    "acceptEdits",
    "plan",
    "bypassPermissions",
    "dontAsk",
    "auto",
];

fn fields(d: Dialect) -> Fields {
    use Ty::*;
    match d {
        Dialect::CursorRule => &[
            ("alwaysApply", Bool, false),
            ("globs", StrOrList, false),
            ("description", Str, false),
        ],
        Dialect::ContinueRule => &[
            ("alwaysApply", Bool, false),
            ("globs", StrOrList, false),
            ("name", Str, false),
            ("description", Str, false),
            ("regex", StrOrList, false),
        ],
        Dialect::CopilotInstructions => &[
            ("applyTo", Str, false),
            ("excludeAgent", Str, false),
            ("description", Str, false),
        ],
        Dialect::CopilotPrompt => &[
            ("description", Str, false),
            ("agent", Str, false),
            ("mode", Str, false),
            ("model", Str, false),
            ("tools", StrOrList, false),
        ],
        Dialect::CopilotAgent => &[
            ("name", Str, false),
            ("description", Str, true),
            ("tools", StrOrList, false),
            ("model", Str, false),
        ],
        Dialect::KiroSteering => &[
            (
                "inclusion",
                OneOf(&["always", "fileMatch", "manual", "auto"]),
                false,
            ),
            ("fileMatchPattern", StrOrList, false),
        ],
        Dialect::WindsurfRule => &[
            (
                "trigger",
                OneOf(&["always_on", "manual", "model_decision", "glob"]),
                false,
            ),
            ("globs", StrOrList, false),
            ("description", Str, false),
        ],
        Dialect::ClaudeAgent => &[
            ("name", Str, true),
            ("description", Str, true),
            ("tools", StrOrList, false),
            ("disallowedTools", StrOrList, false),
            ("model", Str, false),
            ("permissionMode", OneOf(CLAUDE_PERMISSION_MODES), false),
        ],
        Dialect::ClaudeCommand => &[
            ("allowed-tools", StrOrList, false),
            ("description", Str, false),
            ("model", Str, false),
            ("disable-model-invocation", Bool, false),
        ],
        Dialect::CursorCommand | Dialect::AgentSkill => &[],
    }
}

fn check(v: &Value, ty: Ty) -> Result<(), String> {
    match (ty, v) {
        (Ty::Bool, Value::Bool(_)) => Ok(()),
        (Ty::Bool, _) => Err("must be true or false".into()),
        (Ty::Str, Value::String(_)) => Ok(()),
        (Ty::Str, _) => Err("must be a string".into()),
        (Ty::StrOrList, Value::String(_)) => Ok(()),
        (Ty::StrOrList, Value::Array(a)) if a.iter().all(Value::is_string) => Ok(()),
        (Ty::StrOrList, _) => Err("must be a string or a list of strings".into()),
        (Ty::OneOf(opts), Value::String(s)) if opts.contains(&s.as_str()) => Ok(()),
        (Ty::OneOf(opts), _) => Err(format!("must be one of: {}", opts.join(", "))),
    }
}

/// RS-FMT-006 issues for a file of `dialect`. `fm` is `None` when the file
/// has no frontmatter.
pub fn validate(dialect: Dialect, fm: Option<&Frontmatter>) -> Vec<FormatIssue> {
    let mut out = Vec::new();
    let rules = fields(dialect);
    let Some(fm) = fm else {
        for (k, _, req) in rules {
            if *req {
                out.push(FormatIssue::new(
                    1,
                    format!(
                        "{} frontmatter is missing the required `{k}` field.",
                        dialect
                    ),
                ));
            }
        }
        return out;
    };
    let map = match &fm.value {
        Err((msg, line)) => {
            out.push(FormatIssue::new(
                *line,
                format!("{} frontmatter cannot be parsed: {msg}.", dialect),
            ));
            return out;
        }
        Ok(Value::Object(m)) => m,
        Ok(_) => {
            out.push(FormatIssue::new(
                fm.start_line,
                format!("{} frontmatter must be a mapping of fields.", dialect),
            ));
            return out;
        }
    };
    for (k, ty, req) in rules {
        match map.get(*k) {
            Some(v) => {
                if let Err(e) = check(v, *ty) {
                    out.push(FormatIssue::new(
                        fm.line_of(k),
                        format!("{} frontmatter field `{k}` {e}.", dialect),
                    ));
                }
            }
            None if *req => out.push(FormatIssue::new(
                fm.start_line,
                format!(
                    "{} frontmatter is missing the required `{k}` field.",
                    dialect
                ),
            )),
            None => {}
        }
    }
    if dialect == Dialect::KiroSteering
        && map.get("inclusion").and_then(Value::as_str) == Some("fileMatch")
        && !map.contains_key("fileMatchPattern")
    {
        out.push(FormatIssue::new(
            fm.line_of("inclusion"),
            "kiro-steering `inclusion: fileMatch` requires `fileMatchPattern`.",
        ));
    }
    out.sort();
    out
}

fn get<'a>(fm: Option<&'a Frontmatter>, key: &str) -> Option<&'a Value> {
    fm.and_then(|f| f.get(key))
}

/// Always-loaded predicate (PRD §5.1). `match_path` is the registry-relative
/// path; `fm` the parsed frontmatter, if any.
pub fn always_loaded(row: &RegistryRow, match_path: &str, fm: Option<&Frontmatter>) -> bool {
    match row.always {
        AlwaysLoaded::Always => true,
        AlwaysLoaded::Never => false,
        AlwaysLoaded::RootOnly => !match_path.trim_start_matches("./").contains('/'),
        AlwaysLoaded::Frontmatter => match row.dialect {
            Some(Dialect::CursorRule) => get(fm, "alwaysApply") == Some(&Value::Bool(true)),
            Some(Dialect::ContinueRule) => match get(fm, "alwaysApply") {
                Some(Value::Bool(b)) => *b,
                _ => get(fm, "globs").is_none() && get(fm, "regex").is_none(),
            },
            Some(Dialect::KiroSteering) => {
                get(fm, "inclusion").is_none_or(|v| v.as_str() == Some("always"))
            }
            Some(Dialect::WindsurfRule) => {
                get(fm, "trigger").and_then(Value::as_str) == Some("always_on")
            }
            Some(Dialect::CopilotInstructions) => {
                matches!(
                    get(fm, "applyTo").and_then(Value::as_str).map(str::trim),
                    Some("**") | Some("**/*")
                )
            }
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::frontmatter;
    use crate::registry::{self, Origin};
    use std::path::PathBuf;

    fn fixture(rel: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/dialects")
            .join(rel);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    fn issues(d: Dialect, rel: &str) -> Vec<FormatIssue> {
        let t = fixture(rel);
        let fm = frontmatter::split(&t);
        validate(d, fm.as_ref())
    }

    const DIALECTS: &[(Dialect, &str)] = &[
        (Dialect::CursorRule, "cursor-rule"),
        (Dialect::ContinueRule, "continue-rule"),
        (Dialect::CopilotInstructions, "copilot-instructions"),
        (Dialect::CopilotPrompt, "copilot-prompt"),
        (Dialect::CopilotAgent, "copilot-agent"),
        (Dialect::KiroSteering, "kiro-steering"),
        (Dialect::WindsurfRule, "windsurf-rule"),
        (Dialect::ClaudeAgent, "claude-agent"),
        (Dialect::ClaudeCommand, "claude-command"),
    ];

    #[test]
    fn valid_fixture_per_dialect_has_no_issues() {
        for (d, dir) in DIALECTS {
            let i = issues(*d, &format!("{dir}/valid.md"));
            assert!(i.is_empty(), "{dir}: {i:?}");
        }
    }

    #[test]
    fn invalid_fixture_per_dialect_reports_line() {
        for (d, dir) in DIALECTS {
            let i = issues(*d, &format!("{dir}/invalid.md"));
            assert!(!i.is_empty(), "{dir}: no issue");
            let t = fixture(&format!("{dir}/invalid.md"));
            let expect = t
                .lines()
                .position(|l| l.contains("# bad"))
                .map(|p| p + 1)
                .unwrap_or(1);
            assert_eq!(i[0].line, expect, "{dir}: {i:?}");
        }
    }

    #[test]
    fn invalid_messages() {
        let i = issues(Dialect::CursorRule, "cursor-rule/invalid.md");
        assert_eq!(
            i[0].message,
            "cursor-rule frontmatter field `alwaysApply` must be true or false."
        );
        let i = validate(Dialect::ClaudeAgent, None);
        assert_eq!(i.len(), 2);
        let fm = frontmatter::split("---\ninclusion: fileMatch\n---\n").unwrap();
        assert_eq!(validate(Dialect::KiroSteering, Some(&fm)).len(), 1);
        let fm = frontmatter::split("---\na: [\n---\n").unwrap();
        assert!(validate(Dialect::CursorRule, Some(&fm))[0]
            .message
            .contains("cannot be parsed"));
    }

    fn al(path: &str, text: &str) -> bool {
        let row = registry::match_path(path, Origin::Walked).unwrap_or_else(|| panic!("{path}"));
        let fm = frontmatter::split(text);
        always_loaded(row, path, fm.as_ref())
    }

    #[test]
    fn always_loaded_for_every_registry_row() {
        // Expected value for each §5.1 example path when the file has no frontmatter.
        for (path, ..) in registry::tests::PRD_MVP_PATHS {
            let row = registry::match_path(path, Origin::Walked).unwrap();
            let expected = match row.id {
                "agents-md" | "claude-md" | "gemini-md" => !path.contains('/'),
                "claude-dir-md"
                | "claude-settings"
                | "claude-plugin-hooks"
                | "cursor-rules-legacy"
                | "cursor-hooks"
                | "copilot-instructions"
                | "windsurf-rules-legacy"
                | "windsurf-hooks"
                | "cline-rules"
                | "roo-rules"
                | "zed-rules"
                | "junie-guidelines"
                | "kiro-hooks"
                | "amazon-q-rules" => true,
                "kiro-steering" | "continue-rules" => true, // inclusion missing / no globs
                _ => false,
            };
            assert_eq!(
                always_loaded(row, path, None),
                expected,
                "{path} ({})",
                row.id
            );
        }
    }

    #[test]
    fn frontmatter_driven_always_loaded() {
        assert!(al(".cursor/rules/a.mdc", "---\nalwaysApply: true\n---\n"));
        assert!(!al(
            ".cursor/rules/a.mdc",
            "---\nalwaysApply: false\nglobs: \"*.ts\"\n---\n"
        ));
        assert!(al(".continue/rules/a.md", "---\nname: a\n---\n"));
        assert!(!al(".continue/rules/a.md", "---\nglobs: \"*.py\"\n---\n"));
        assert!(al(
            ".continue/rules/a.md",
            "---\nglobs: \"*.py\"\nalwaysApply: true\n---\n"
        ));
        assert!(al(".kiro/steering/a.md", "---\ninclusion: always\n---\n"));
        assert!(!al(".kiro/steering/a.md", "---\ninclusion: manual\n---\n"));
        assert!(al(".windsurf/rules/a.md", "---\ntrigger: always_on\n---\n"));
        assert!(!al(".windsurf/rules/a.md", "---\ntrigger: glob\n---\n"));
        assert!(al(
            ".github/instructions/a.instructions.md",
            "---\napplyTo: \"**\"\n---\n"
        ));
        assert!(!al(
            ".github/instructions/a.instructions.md",
            "---\napplyTo: \"**/*.rs\"\n---\n"
        ));
        assert!(al("AGENTS.md", "x"));
        assert!(!al("sub/AGENTS.md", "x"));
    }
}
