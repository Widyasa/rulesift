//! Hooks normalizer (PRD §5 F1): every supported hooks format becomes one
//! list of [`Hook`] `{ tool, event, matcher, command }`.
//!
//! Per-tool layouts are data ([`HookShape`] JSON pointer patterns), not code
//! branches. Docs: Claude Code <https://code.claude.com/docs/en/hooks>,
//! Cursor <https://cursor.com/docs/hooks>, Windsurf
//! <https://docs.windsurf.com/windsurf/cascade/hooks>, Kiro
//! <https://kiro.dev/docs/hooks/>.

use super::FormatIssue;
use crate::model::Tool;
use crate::parse::json::{self, escape_token, JsonDoc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One normalized hook command.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hook {
    /// Tool.
    pub tool: Tool,
    /// Lifecycle event (e.g. `PostToolUse`, `afterFileEdit`, `PostFileSave`).
    pub event: String,
    /// Matcher, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// Shell command.
    pub command: String,
    /// JSON pointer of the command value.
    pub pointer: String,
    /// 1-based line of the command.
    pub line: usize,
}

/// Where the event name comes from.
#[derive(Debug, Clone, Copy)]
pub enum EventFrom {
    /// The `{event}` segment of the entry pattern.
    Key,
    /// A field of the entry, walking `up` levels first.
    Field {
        /// Levels to walk up from the entry.
        up: usize,
        /// Field name.
        name: &'static str,
    },
}

/// Data description of one hooks layout.
#[derive(Debug, Clone, Copy)]
pub struct HookShape {
    /// Pointer pattern of a hook entry object; `{event}` captures an object
    /// key as the event, `*` matches any key or index.
    pub entry: &'static str,
    /// Fields of the entry that hold commands (each becomes a hook).
    pub commands: &'static [&'static str],
    /// Matcher field: (levels up from the entry, field name).
    pub matcher: Option<(usize, &'static str)>,
    /// Event source.
    pub event: EventFrom,
    /// Optional `type`-like field and the value that marks a command hook
    /// (entries with another value are skipped; a missing field counts as a command).
    pub type_field: Option<(&'static str, &'static str)>,
}

/// Claude Code settings and plugin `hooks/hooks.json`.
pub const CLAUDE: HookShape = HookShape {
    entry: "/hooks/{event}/*/hooks/*",
    commands: &["command"],
    matcher: Some((2, "matcher")),
    event: EventFrom::Key,
    type_field: Some(("type", "command")),
};
/// Cursor `.cursor/hooks.json`.
pub const CURSOR: HookShape = HookShape {
    entry: "/hooks/{event}/*",
    commands: &["command"],
    matcher: Some((0, "matcher")),
    event: EventFrom::Key,
    type_field: Some(("type", "command")),
};
/// Windsurf / Devin `hooks.json` (`powershell` is the Windows variant).
pub const WINDSURF: HookShape = HookShape {
    entry: "/hooks/{event}/*",
    commands: &["command", "powershell"],
    matcher: None,
    event: EventFrom::Key,
    type_field: None,
};
/// Kiro `.kiro/hooks/*.json`.
pub const KIRO: HookShape = HookShape {
    entry: "/hooks/*/action",
    commands: &["command"],
    matcher: Some((1, "matcher")),
    event: EventFrom::Field {
        up: 1,
        name: "trigger",
    },
    type_field: Some(("type", "command")),
};

/// Shape for a tool's hooks files.
pub fn shape_for(tool: Tool) -> Option<HookShape> {
    match tool {
        Tool::ClaudeCode => Some(CLAUDE),
        Tool::Cursor => Some(CURSOR),
        Tool::Windsurf => Some(WINDSURF),
        Tool::Kiro => Some(KIRO),
        _ => None,
    }
}

/// Normalized hooks of one file.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HooksFile {
    /// Hooks sorted by line.
    pub hooks: Vec<Hook>,
    /// RS-FMT-007 issues.
    pub issues: Vec<FormatIssue>,
}

/// Matched entry: pointer tokens and captured event.
struct Entry<'v> {
    tokens: Vec<String>,
    event: Option<String>,
    value: &'v Value,
}

fn walk<'v>(
    v: &'v Value,
    pat: &[&str],
    tokens: Vec<String>,
    event: Option<String>,
    out: &mut Vec<Entry<'v>>,
) {
    let Some((head, rest)) = pat.split_first() else {
        out.push(Entry {
            tokens,
            event,
            value: v,
        });
        return;
    };
    let mut step = |k: String, child: &'v Value, ev: Option<String>| {
        let mut t = tokens.clone();
        t.push(k);
        walk(child, rest, t, ev, out);
    };
    match (*head, v) {
        ("{event}", Value::Object(m)) => {
            for (k, c) in m {
                step(k.clone(), c, Some(k.clone()));
            }
        }
        ("*", Value::Object(m)) => {
            for (k, c) in m {
                step(k.clone(), c, event.clone());
            }
        }
        ("*", Value::Array(a)) | ("{event}", Value::Array(a)) => {
            for (i, c) in a.iter().enumerate() {
                step(i.to_string(), c, event.clone());
            }
        }
        (lit, Value::Object(m)) => {
            if let Some(c) = m.get(lit) {
                step(lit.to_string(), c, event.clone());
            }
        }
        _ => {}
    }
}

fn ptr(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|t| format!("/{}", escape_token(t)))
        .collect()
}

/// Normalize parsed hooks JSON. `require_hooks` reports a missing `hooks`
/// key (true for dedicated hooks files, false for Claude settings).
pub fn from_doc(tool: Tool, doc: &JsonDoc, require_hooks: bool) -> HooksFile {
    let mut out = HooksFile::default();
    let Some(shape) = shape_for(tool) else {
        return out;
    };
    let root = &doc.value;
    if !root.is_object() {
        out.issues
            .push(FormatIssue::new(1, "hooks file must be a JSON object."));
        return out;
    }
    match root.get("hooks") {
        None if require_hooks => {
            out.issues.push(FormatIssue::new(
                1,
                "hooks file is missing the `hooks` key.",
            ));
            return out;
        }
        None => return out,
        Some(h) if !(h.is_object() || h.is_array()) => {
            out.issues.push(FormatIssue::new(
                doc.line_of("/hooks"),
                "`hooks` must be an object or a list.",
            ));
            return out;
        }
        _ => {}
    }
    let pat: Vec<&str> = shape.entry.trim_start_matches('/').split('/').collect();
    let mut entries = Vec::new();
    walk(root, &pat, Vec::new(), None, &mut entries);
    for e in entries {
        let Value::Object(obj) = e.value else {
            out.issues.push(FormatIssue::new(
                doc.line_of(&ptr(&e.tokens)),
                "hook entry must be an object.",
            ));
            continue;
        };
        if let Some((field, want)) = shape.type_field {
            if let Some(t) = obj.get(field).and_then(Value::as_str) {
                if t != want {
                    continue;
                }
            }
        }
        let ancestor = |up: usize| -> Option<&Value> {
            let n = e.tokens.len().checked_sub(up)?;
            root.pointer(&ptr(&e.tokens[..n]))
        };
        let event = match shape.event {
            EventFrom::Key => e.event.clone(),
            EventFrom::Field { up, name } => ancestor(up)
                .and_then(|a| a.get(name))
                .and_then(Value::as_str)
                .map(str::to_string),
        }
        .unwrap_or_else(|| "unknown".to_string());
        let matcher = shape
            .matcher
            .and_then(|(up, name)| ancestor(up).and_then(|a| a.get(name)))
            .and_then(Value::as_str)
            .map(str::to_string);
        let mut found = false;
        for cf in shape.commands {
            if let Some(c) = obj.get(*cf) {
                let mut t = e.tokens.clone();
                t.push((*cf).to_string());
                let p = ptr(&t);
                match c.as_str() {
                    Some(cmd) => {
                        found = true;
                        out.hooks.push(Hook {
                            tool,
                            event: event.clone(),
                            matcher: matcher.clone(),
                            command: cmd.to_string(),
                            line: doc.line_of(&p),
                            pointer: p,
                        });
                    }
                    None => out.issues.push(FormatIssue::new(
                        doc.line_of(&p),
                        format!("hook `{cf}` must be a string."),
                    )),
                }
            }
        }
        if !found
            && !out
                .issues
                .iter()
                .any(|i| i.line == doc.line_of(&ptr(&e.tokens)))
        {
            out.issues.push(FormatIssue::new(
                doc.line_of(&ptr(&e.tokens)),
                format!("hook for event {event:?} has no `command`."),
            ));
        }
    }
    out.hooks
        .sort_by(|a, b| (a.line, &a.pointer).cmp(&(b.line, &b.pointer)));
    out.issues.sort();
    out
}

/// Parse and normalize a hooks JSON file.
pub fn normalize(tool: Tool, text: &str, require_hooks: bool) -> HooksFile {
    match json::parse(text) {
        Ok(doc) => from_doc(tool, &doc, require_hooks),
        Err(e) => HooksFile {
            hooks: Vec::new(),
            issues: vec![FormatIssue::new(e.line, e.to_string())],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(rel: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/hooks")
            .join(rel);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    fn snap(tool: Tool, rel: &str, req: bool) -> String {
        serde_json::to_string_pretty(&normalize(tool, &fixture(rel), req)).unwrap()
    }

    #[test]
    fn claude_settings() {
        insta::assert_snapshot!(snap(Tool::ClaudeCode, "claude-code/settings.json", false));
    }

    #[test]
    fn claude_plugin() {
        insta::assert_snapshot!(snap(Tool::ClaudeCode, "claude-code/hooks.json", true));
    }

    #[test]
    fn cursor() {
        insta::assert_snapshot!(snap(Tool::Cursor, "cursor/hooks.json", true));
    }

    #[test]
    fn windsurf() {
        insta::assert_snapshot!(snap(Tool::Windsurf, "windsurf/hooks.json", true));
    }

    #[test]
    fn kiro() {
        insta::assert_snapshot!(snap(Tool::Kiro, "kiro/lint.json", true));
    }

    #[test]
    fn invalid_files() {
        let h = normalize(Tool::Cursor, &fixture("invalid/broken.json"), true);
        assert_eq!(h.issues.len(), 1);
        assert!(h.issues[0].message.starts_with("invalid JSON"));
        let h = normalize(Tool::Cursor, "{\"version\": 1}", true);
        assert_eq!(
            h.issues[0].message,
            "hooks file is missing the `hooks` key."
        );
        let h = normalize(Tool::ClaudeCode, "{\"permissions\": {}}", false);
        assert!(h.issues.is_empty() && h.hooks.is_empty());
        let h = normalize(
            Tool::Windsurf,
            "{\"hooks\": {\"pre_read_code\": [{\"show_output\": true}]}}",
            true,
        );
        assert_eq!(h.issues.len(), 1);
        assert!(h.issues[0].message.contains("has no `command`"));
    }
}
