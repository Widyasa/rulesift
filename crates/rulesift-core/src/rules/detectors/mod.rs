//! Built-in Rust detectors referenced by name from YAML rules (PRD §8).

pub mod base64;
pub mod entropy;
pub mod secrets;
pub mod unicode;

/// Names of the built-in detectors YAML rules may reference.
pub const KNOWN_DETECTORS: &[&str] = &[
    "skill_frontmatter",
    "skill_name",
    "skill_dir_name",
    "skill_description",
    "skill_fields",
    "dialect_frontmatter",
    "config_format",
    "unicode_tags",
    "bidi",
    "zero_width",
    "invisible_format",
    "base64_payload",
    "hardcoded_secret",
];

/// One detector match inside a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// Byte offset of the match start in the scanned text.
    pub start: usize,
    /// Byte offset after the match.
    pub end: usize,
    /// English message.
    pub message: String,
    /// Evidence (hidden characters rendered as `<U+XXXX>`, ≤ 200 chars).
    pub evidence: String,
    /// Decoded hidden text, if any.
    pub decoded: Option<String>,
}

/// Truncate to at most `max` characters, appending `…` when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// A located detector or matcher result in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loc {
    /// Byte span in the file.
    pub span: crate::model::Span,
    /// 1-based line.
    pub line: usize,
    /// 1-based column.
    pub column: usize,
    /// 1-based end line.
    pub end_line: usize,
    /// Evidence (rendered, ≤ 200 chars).
    pub evidence: String,
    /// Decoded hidden text, if any.
    pub decoded: Option<String>,
    /// Specific English message (overrides the rule message).
    pub message: Option<String>,
    /// Produced by `not` (no real location); dropped when other matches exist.
    pub synthetic: bool,
}

impl Loc {
    /// Location of a whole line.
    pub fn at_line(li: &crate::text::LineIndex, line: usize, evidence: String) -> Self {
        let start = li.line_start(line);
        Loc {
            span: crate::model::Span { start, end: start },
            line,
            column: 1,
            end_line: line,
            evidence,
            decoded: None,
            message: None,
            synthetic: false,
        }
    }

    /// Location from a text [`Hit`].
    pub fn from_hit(li: &crate::text::LineIndex, h: Hit) -> Self {
        let (line, column) = li.position(h.start);
        let (end_line, _) = li.position(h.end.saturating_sub(1).max(h.start));
        Loc {
            span: crate::model::Span {
                start: h.start,
                end: h.end,
            },
            line,
            column,
            end_line,
            evidence: h.evidence,
            decoded: h.decoded,
            message: Some(h.message),
            synthetic: false,
        }
    }
}

fn issue_locs(
    li: &crate::text::LineIndex,
    text: &str,
    issues: &[crate::formats::FormatIssue],
) -> Vec<Loc> {
    issues
        .iter()
        .map(|i| {
            let ev = text.lines().nth(i.line.saturating_sub(1)).unwrap_or("");
            let mut l = Loc::at_line(li, i.line, truncate(&unicode::render(ev.trim()), 200));
            l.message = Some(i.message.clone());
            l
        })
        .collect()
}

/// Run a named detector over an analyzed file.
pub fn run(name: &str, a: &crate::analyze::Analyzed) -> Vec<Loc> {
    let text = a.doc.text.as_str();
    let li = crate::text::LineIndex::new(text);
    let skill = |id: &str| -> Vec<Loc> {
        a.skill_issues
            .iter()
            .filter(|i| i.rule_id == id)
            .map(|i| {
                let mut l = Loc::at_line(&li, i.line, truncate(&unicode::render(&i.evidence), 200));
                l.message = Some(i.message.clone());
                l
            })
            .collect()
    };
    let hits = |f: fn(&str) -> Vec<Hit>| -> Vec<Loc> {
        f(text).into_iter().map(|h| Loc::from_hit(&li, h)).collect()
    };
    match name {
        "skill_frontmatter" => skill("RS-FMT-001"),
        "skill_name" => skill("RS-FMT-002"),
        "skill_dir_name" => skill("RS-FMT-003"),
        "skill_description" => skill("RS-FMT-004"),
        "skill_fields" => skill("RS-FMT-005"),
        "dialect_frontmatter" => issue_locs(&li, text, &a.dialect_issues),
        "config_format" => issue_locs(&li, text, &a.format_issues),
        "unicode_tags" => hits(unicode::unicode_tags),
        "bidi" => hits(unicode::bidi),
        "zero_width" => hits(unicode::zero_width),
        "invisible_format" => hits(unicode::invisible_format),
        "base64_payload" => base64::base64_payload(a),
        "hardcoded_secret" => secrets::hardcoded_secret(a),
        _ => Vec::new(),
    }
}
