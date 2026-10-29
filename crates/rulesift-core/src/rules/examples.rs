//! Run a rule's inline `examples` (PRD §6.5).

use crate::analyze::analyze;
use crate::model::FileKind;
use crate::registry::{match_path, Origin};
use crate::rules::detectors::{truncate, unicode};
use crate::rules::engine::run_rules;
use crate::rules::loader::Rule;

/// Result of one example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleResult {
    /// Rule id.
    pub rule_id: String,
    /// `true` for `should_match`, `false` for `should_not_match`.
    pub should_match: bool,
    /// Example text (rendered, ≤ 80 chars).
    pub text: String,
    /// Path the example was scanned as.
    pub path: String,
    /// Passed?
    pub passed: bool,
    /// English explanation when failed.
    pub problem: Option<String>,
}

/// Default path for examples of a rule, from its first `applies_to` kind.
pub fn default_path(kinds: Option<&[FileKind]>) -> &'static str {
    match kinds.and_then(|k| k.first()) {
        Some(FileKind::Skill) => ".claude/skills/example/SKILL.md",
        Some(FileKind::Command) => ".claude/commands/example.md",
        Some(FileKind::Agent) => ".claude/agents/example.md",
        Some(FileKind::McpConfig) => ".mcp.json",
        Some(FileKind::Hooks) => ".cursor/hooks.json",
        Some(FileKind::Settings) => ".claude/settings.json",
        Some(FileKind::Plugin) => ".claude-plugin/plugin.json",
        Some(FileKind::Instructions) | None => "AGENTS.md",
    }
}

/// Run all examples of `rule`. Rules without examples return nothing.
pub fn run(rule: &Rule) -> Vec<ExampleResult> {
    let Some(ex) = &rule.def.examples else {
        return Vec::new();
    };
    let path = ex
        .path
        .clone()
        .unwrap_or_else(|| default_path(rule.def.applies_to.as_deref()).to_string());
    let row = match_path(&path, Origin::Explicit);
    let skill_dir = path
        .strip_suffix("/SKILL.md")
        .map(|d| d.rsplit('/').next().unwrap_or(d).to_string());
    let cases = ex
        .should_match
        .iter()
        .map(|t| (true, t))
        .chain(ex.should_not_match.iter().map(|t| (false, t)));
    cases
        .map(|(want, text)| {
            let shown = truncate(&unicode::render(text), 80);
            let (passed, problem) = match row {
                None => (
                    false,
                    Some(format!("example path {path:?} is not a known agent file")),
                ),
                Some(row) if !rule.applies_to(row.kind) => (
                    false,
                    Some(format!(
                        "example path {path:?} is a {} file, which the rule does not apply to",
                        row.kind
                    )),
                ),
                Some(row) => {
                    let a = analyze(&path, &path, row, skill_dir.as_deref(), text);
                    let got = !run_rules([rule], &a).is_empty();
                    if got == want {
                        (true, None)
                    } else if want {
                        (false, Some("expected a match, got none".to_string()))
                    } else {
                        (false, Some("expected no match, got one".to_string()))
                    }
                }
            };
            ExampleResult {
                rule_id: rule.def.id.clone(),
                should_match: want,
                text: shown,
                path: path.clone(),
                passed,
                problem,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::loader::builtin;

    #[test]
    fn default_paths_match_their_kind() {
        for k in [
            FileKind::Skill,
            FileKind::Instructions,
            FileKind::Command,
            FileKind::Agent,
            FileKind::McpConfig,
            FileKind::Hooks,
            FileKind::Settings,
            FileKind::Plugin,
        ] {
            let p = default_path(Some(&[k]));
            assert_eq!(
                match_path(p, Origin::Explicit).map(|r| r.kind),
                Some(k),
                "{p}"
            );
        }
    }

    #[test]
    fn every_builtin_example_passes() {
        let rules = builtin().unwrap();
        let mut n = 0;
        for r in &rules {
            for res in run(r) {
                n += 1;
                assert!(
                    res.passed,
                    "{} {:?}: {:?}",
                    res.rule_id, res.text, res.problem
                );
            }
        }
        assert!(n >= 2 * rules.len(), "every built-in rule has examples");
        for r in &rules {
            let ex = r.def.examples.as_ref().unwrap();
            assert!(
                !ex.should_match.is_empty() && !ex.should_not_match.is_empty(),
                "{}",
                r.id()
            );
        }
    }
}
