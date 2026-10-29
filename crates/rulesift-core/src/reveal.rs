//! `reveal`: list hidden characters, decoded text, base64 payloads, and
//! HTML comments of a file (PRD §6.2). Shared by the CLI and MCP server.

use crate::model::Scope;
use crate::normalize;
use crate::parse::markdown;
use crate::rules::detectors::unicode::{
    self, is_bidi, is_tag, is_variation_selector, is_zero_width,
};
use crate::text::LineIndex;
use serde::{Deserialize, Serialize};

/// A run of hidden characters of one category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenRun {
    /// 1-based line.
    pub line: usize,
    /// 1-based column.
    pub column: usize,
    /// Category: `unicode-tag`, `bidi`, `zero-width`, `variation-selector`, `invisible-format`.
    pub category: String,
    /// Number of characters.
    pub count: usize,
    /// Code points as `U+XXXX` (first 16).
    pub code_points: Vec<String>,
}

/// A decoded or extracted text item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevealedText {
    /// 1-based line.
    pub line: usize,
    /// 1-based column.
    pub column: usize,
    /// Text with hidden characters rendered as `<U+XXXX>`.
    pub text: String,
}

/// Everything `reveal` shows for one file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Reveal {
    /// Hidden character runs.
    pub hidden: Vec<HiddenRun>,
    /// Decoded Unicode Tag text.
    pub tag_text: Vec<RevealedText>,
    /// Decoded base64 payloads.
    pub base64: Vec<RevealedText>,
    /// HTML comment bodies.
    pub comments: Vec<RevealedText>,
}

impl Reveal {
    /// True if nothing hidden was found.
    pub fn is_empty(&self) -> bool {
        self.hidden.is_empty()
            && self.tag_text.is_empty()
            && self.base64.is_empty()
            && self.comments.is_empty()
    }
}

fn category(c: char) -> Option<&'static str> {
    if is_tag(c) {
        Some("unicode-tag")
    } else if is_bidi(c) {
        Some("bidi")
    } else if is_zero_width(c) {
        Some("zero-width")
    } else if is_variation_selector(c) {
        Some("variation-selector")
    } else if unicode::is_other_invisible(c) {
        Some("invisible-format")
    } else {
        None
    }
}

/// Analyze `text`. HTML comments are taken from a Markdown parse (only
/// meaningful for Markdown files; harmless for others).
pub fn reveal(text: &str) -> Reveal {
    let li = LineIndex::new(text);
    let mut r = Reveal::default();
    let mut cur: Option<(usize, &'static str, Vec<char>)> = None;
    let flush = |cur: &mut Option<(usize, &'static str, Vec<char>)>, r: &mut Reveal| {
        if let Some((start, cat, chars)) = cur.take() {
            let (line, column) = li.position(start);
            r.hidden.push(HiddenRun {
                line,
                column,
                category: cat.to_string(),
                count: chars.len(),
                code_points: chars
                    .iter()
                    .take(16)
                    .map(|c| format!("U+{:04X}", *c as u32))
                    .collect(),
            });
        }
    };
    for (i, c) in text.char_indices() {
        // A BOM at offset 0 is normal.
        let cat = if i == 0 && c == '\u{FEFF}' {
            None
        } else {
            category(c)
        };
        match (cat, &mut cur) {
            (Some(k), Some((_, ck, chars))) if *ck == k => chars.push(c),
            (Some(k), _) => {
                flush(&mut cur, &mut r);
                cur = Some((i, k, vec![c]));
            }
            (None, _) => flush(&mut cur, &mut r),
        }
    }
    flush(&mut cur, &mut r);

    let md = markdown::parse(text);
    for s in normalize::derive(text, &md) {
        let item = RevealedText {
            line: s.line,
            column: s.column,
            text: unicode::render(&s.text),
        };
        match s.origin.as_deref() {
            Some("unicode_tags") => r.tag_text.push(item),
            Some("base64") | Some("base64_nested") => r.base64.push(item),
            _ => {}
        }
    }
    for s in md.iter().filter(|s| s.scope == Scope::HtmlComment) {
        r.comments.push(RevealedText {
            line: s.line,
            column: s.column,
            text: unicode::render(&s.text),
        });
    }
    r
}
