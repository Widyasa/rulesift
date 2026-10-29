//! Built-in Rust detectors referenced by name from YAML rules (PRD §8).

pub mod base64;
pub mod entropy;
pub mod exfil_url;
pub mod mcp;
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
    "exfil_url",
    "mcp_shell",
    "mcp_literal_secret",
    "mcp_unpinned",
    "mcp_plain_http",
    "mcp_auto_trust",
    "hook_risk",
    "unscannable",
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
        "unicode_tags" => with_escaped(a, hits(unicode::unicode_tags), unicode::unicode_tags),
        "bidi" => with_escaped(a, hits(unicode::bidi), unicode::bidi),
        "zero_width" => with_escaped(a, hits(unicode::zero_width), unicode::zero_width),
        "invisible_format" => with_escaped(
            a,
            hits(unicode::invisible_format),
            unicode::invisible_format,
        ),
        "base64_payload" => base64::base64_payload(a),
        "hardcoded_secret" => secrets::hardcoded_secret(a),
        "exfil_url" => exfil_url::exfil_url(a),
        "mcp_shell" => mcp::mcp_shell(a),
        "mcp_literal_secret" => mcp::mcp_literal_secret(a),
        "mcp_unpinned" => mcp::mcp_unpinned(a),
        "mcp_plain_http" => mcp::mcp_plain_http(a),
        "mcp_auto_trust" => mcp::mcp_auto_trust(a),
        "hook_risk" => mcp::hook_risk(a),
        "unscannable" => unscannable(&li, a),
        _ => Vec::new(),
    }
}

/// RS-FMT-008: the file could not be read or analysed, or contains NUL bytes
/// (tools that treat it as binary skip it while agents may still read it).
fn unscannable(li: &crate::text::LineIndex, a: &crate::analyze::Analyzed) -> Vec<Loc> {
    if let Some(reason) = &a.unscannable {
        let mut l = Loc::at_line(li, 1, reason.clone());
        l.message = Some(format!(
            "RuleSift could not scan this agent file: {reason}. An agent may still load it, so it is treated as unsafe. Fix the file, raise `max_file_size_kib`, or suppress RS-FMT-008 for this path with a reason."
        ));
        return vec![l];
    }
    let text = a.doc.text.as_str();
    let Some(pos) = text.find('\0') else {
        return Vec::new();
    };
    let (line, _) = li.position(pos);
    let ev = text.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let mut l = Loc::at_line(li, line, truncate(&unicode::render(ev.trim()), 200));
    l.message = Some(
        "The file contains NUL bytes. Scanners and editors may treat it as binary and skip it while agents still read it. It was scanned as text; remove the NUL bytes."
            .into(),
    );
    vec![l]
}

/// Add hits for hidden characters that only exist after decoding a string
/// value, e.g. a JSON `\u202E` escape in `.mcp.json` (#104), or an object
/// key or frontmatter string (#125). Raw characters are already found in the
/// file text; only strings whose decoded text is not a verbatim slice of the
/// file are checked again, and a hit is kept only if its text does not occur
/// verbatim in the file.
fn with_escaped(
    a: &crate::analyze::Analyzed,
    mut locs: Vec<Loc>,
    f: fn(&str) -> Vec<Hit>,
) -> Vec<Loc> {
    let text = a.doc.text.as_str();
    for seg in a
        .doc
        .segments
        .iter()
        .filter(|s| s.scope == crate::model::Scope::JsonString)
        .chain(a.escaped_strings.iter())
    {
        let verbatim = text
            .get(seg.span.start..seg.span.end)
            .is_some_and(|raw| raw.contains(seg.text.as_str()));
        if verbatim {
            continue;
        }
        for h in f(&seg.text) {
            if seg
                .text
                .get(h.start..h.end)
                .is_some_and(|s| !s.is_empty() && text.contains(s))
            {
                continue; // written raw: already reported from the file text
            }
            locs.push(Loc {
                span: seg.span,
                line: seg.line,
                column: seg.column,
                end_line: seg.end_line,
                evidence: h.evidence,
                decoded: h.decoded,
                message: Some(format!(
                    "{} They are written as escape sequences in a quoted string or key, so they are invisible in the file but present once the agent decodes it.",
                    h.message
                )),
                synthetic: false,
            });
        }
    }
    locs
}
