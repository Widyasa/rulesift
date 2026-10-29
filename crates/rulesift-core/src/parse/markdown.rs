//! Markdown → segments via `comrak` with source positions (PRD §5 F2).
//!
//! Produces `text`, `heading`, `code_block` (with language), `inline_code`,
//! `html_comment`, `link_url`, and `image_url` segments. Multi-line segment
//! text keeps the source line breaks, so the line of any match inside a
//! segment is `segment.line + newlines before the match`.

use crate::model::{Scope, Segment, Span};
use crate::text::LineIndex;
use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use comrak::{parse_document, Arena, Options};

fn options() -> Options<'static> {
    let mut o = Options::default();
    o.extension.table = true;
    o.extension.strikethrough = true;
    o.extension.tasklist = true;
    o.extension.autolink = true;
    o.extension.footnotes = true;
    o
}

/// Parse a whole Markdown document.
pub fn parse(src: &str) -> Vec<Segment> {
    parse_at(src, 0)
}

/// Parse `full[start..]` as Markdown (e.g. the body after frontmatter) and
/// report positions relative to `full`.
pub fn parse_at(full: &str, start: usize) -> Vec<Segment> {
    let start = start.min(full.len());
    let body = &full[start..];
    let arena = Arena::new();
    let root = parse_document(&arena, body, &options());
    let body_index = LineIndex::new(body);
    let full_index = LineIndex::new(full);
    let mut cx = Cx {
        body,
        base: start,
        bi: body_index,
        fi: full_index,
        out: Vec::new(),
    };
    for node in root.children() {
        cx.block(node);
    }
    let mut out = cx.out;
    out.sort_by(|a, b| {
        (a.span.start, a.scope.as_str(), a.span.end).cmp(&(
            b.span.start,
            b.scope.as_str(),
            b.span.end,
        ))
    });
    out
}

struct Cx<'s> {
    body: &'s str,
    base: usize,
    bi: LineIndex<'s>,
    fi: LineIndex<'s>,
    out: Vec<Segment>,
}

impl<'s> Cx<'s> {
    /// Byte span (in body) of a sourcepos.
    fn span_of(&self, sp: Sourcepos) -> (usize, usize) {
        let s = self.bi.offset(sp.start.line, sp.start.column);
        let e = self.bi.offset(sp.end.line, sp.end.column);
        // End column points at the last byte of the node.
        let e = self.bi.ceil_boundary(e + 1).max(s);
        (s, e.min(self.body.len()))
    }

    fn push(
        &mut self,
        scope: Scope,
        text: String,
        body_span: (usize, usize),
        first_line_offset: Option<usize>,
    ) -> usize {
        let span = Span {
            start: self.base + body_span.0,
            end: self.base + body_span.1,
        };
        let anchor = self.base + first_line_offset.unwrap_or(body_span.0);
        let (line, column) = self.fi.position(anchor);
        let (end_line, _) = self.fi.position(span.end.saturating_sub(1).max(span.start));
        self.out.push(Segment::new(
            scope,
            text,
            span,
            line,
            column,
            end_line.max(line),
        ));
        self.out.len() - 1
    }

    fn block<'a>(&mut self, node: &'a AstNode<'a>) {
        let ast = node.data.borrow();
        let sp = ast.sourcepos;
        match &ast.value {
            NodeValue::CodeBlock(cb) => {
                let span = self.span_of(sp);
                // Fenced content starts on the line after the fence.
                let content_line = if cb.fenced {
                    sp.start.line + 1
                } else {
                    sp.start.line
                };
                let first = self.bi.line_start(content_line.min(self.bi.line_count()));
                let first = if cb.fenced { first } else { span.0 };
                let lang = cb
                    .info
                    .split_whitespace()
                    .next()
                    .map(|s| s.to_ascii_lowercase());
                let i = self.push(Scope::CodeBlock, cb.literal.clone(), span, Some(first));
                self.out[i].lang = lang.filter(|l| !l.is_empty());
            }
            NodeValue::HtmlBlock(_) => {
                let span = self.span_of(sp);
                self.comments_in(span);
                // Raw HTML is often hidden by renderers but read by models:
                // expose the non-comment parts as text.
                self.html_text_in(span);
            }
            NodeValue::Heading(_) => {
                drop(ast);
                self.inline_container(node, Scope::Heading);
            }
            NodeValue::Paragraph | NodeValue::TableCell => {
                drop(ast);
                self.inline_container(node, Scope::Text);
            }
            NodeValue::FrontMatter(_) => {}
            _ => {
                drop(ast);
                for c in node.children() {
                    self.block(c);
                }
            }
        }
    }

    /// Collect the inline text of a paragraph/heading/cell into one segment
    /// and emit inline code, links, images, and inline HTML comments.
    fn inline_container<'a>(&mut self, node: &'a AstNode<'a>, scope: Scope) {
        let sp = node.data.borrow().sourcepos;
        let mut text = String::new();
        let mut first: Option<usize> = None;
        for c in node.children() {
            self.inline(c, &mut text, &mut first);
        }
        let span = self.span_of(sp);
        if !text.trim().is_empty() {
            self.push(scope, text, span, first.or(Some(span.0)));
        }
    }

    fn inline<'a>(&mut self, node: &'a AstNode<'a>, text: &mut String, first: &mut Option<usize>) {
        let ast = node.data.borrow();
        let sp = ast.sourcepos;
        match &ast.value {
            NodeValue::Text(t) => {
                if first.is_none() && sp.start.line > 0 {
                    *first = Some(self.bi.offset(sp.start.line, sp.start.column));
                }
                text.push_str(t);
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => text.push('\n'),
            NodeValue::Code(c) => {
                let span = self.span_of(sp);
                // Keep line structure for multi-line inline code.
                for _ in 0..c.literal.matches('\n').count() {
                    text.push('\n');
                }
                self.push(Scope::InlineCode, c.literal.clone(), span, None);
            }
            NodeValue::HtmlInline(h) => {
                let span = self.span_of(sp);
                if h.trim_start().starts_with("<!--") {
                    self.comments_in(span);
                } else {
                    if first.is_none() {
                        *first = Some(span.0);
                    }
                    text.push_str(h);
                }
            }
            NodeValue::Link(l) | NodeValue::Image(l) => {
                let scope = if matches!(ast.value, NodeValue::Image(_)) {
                    Scope::ImageUrl
                } else {
                    Scope::LinkUrl
                };
                let span = self.span_of(sp);
                let url = l.url.clone();
                drop(ast);
                if !url.is_empty() {
                    self.push(scope, url, span, None);
                }
                for c in node.children() {
                    self.inline(c, text, first);
                }
            }
            _ => {
                drop(ast);
                for c in node.children() {
                    self.inline(c, text, first);
                }
            }
        }
    }

    /// Emit a `text` segment for each non-comment chunk of a raw HTML block.
    fn html_text_in(&mut self, span: (usize, usize)) {
        let src = &self.body[span.0..span.1];
        let mut chunks: Vec<(usize, usize)> = Vec::new();
        let mut pos = 0;
        while let Some(i) = src[pos..].find("<!--") {
            chunks.push((pos, pos + i));
            let body_start = pos + i + 4;
            pos = match src[body_start..].find("-->") {
                Some(j) => body_start + j + 3,
                None => src.len(),
            };
        }
        chunks.push((pos, src.len()));
        for (s, e) in chunks {
            let chunk = &src[s..e];
            let t = chunk.trim();
            if t.is_empty() {
                continue;
            }
            let lead = chunk.len() - chunk.trim_start().len();
            let start = span.0 + s + lead;
            self.push(
                Scope::Text,
                t.to_string(),
                (start, start + t.len()),
                Some(start),
            );
        }
    }

    /// Emit an `html_comment` segment for every `<!-- ... -->` in the body span.
    fn comments_in(&mut self, span: (usize, usize)) {
        let src = &self.body[span.0..span.1];
        let mut pos = 0;
        while let Some(i) = src[pos..].find("<!--") {
            let open = pos + i;
            let body_start = open + 4;
            let (body_end, close_end) = match src[body_start..].find("-->") {
                Some(j) => (body_start + j, body_start + j + 3),
                None => (src.len(), src.len()),
            };
            let inner = src[body_start..body_end].to_string();
            if !inner.trim().is_empty() {
                let s = span.0 + open;
                let e = span.0 + close_end;
                // Anchor the segment at the first character of the comment body.
                let lead = inner.len() - inner.trim_start().len();
                let anchor = span.0 + body_start + lead;
                let trimmed = inner.trim().to_string();
                self.push(Scope::HtmlComment, trimmed, (s, e), Some(anchor));
            }
            pos = close_end;
            if pos >= src.len() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn fixture(name: &str) -> String {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/markdown")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    pub(crate) fn render(segs: &[Segment]) -> String {
        let mut s = String::new();
        for g in segs {
            let _ = writeln!(
                s,
                "{}:{}-{} {}{}{} {:?}",
                g.line,
                g.column,
                g.end_line,
                g.scope,
                g.lang
                    .as_ref()
                    .map(|l| format!("[{l}]"))
                    .unwrap_or_default(),
                g.pointer
                    .as_ref()
                    .map(|l| format!("[{l}]"))
                    .unwrap_or_default(),
                g.text
            );
        }
        s
    }

    #[test]
    fn plain_instructions_snapshot() {
        insta::assert_snapshot!(render(&parse(&fixture("plain-instructions.md"))));
    }

    #[test]
    fn skill_body_snapshot() {
        let src = fixture("skill-body.md");
        insta::assert_snapshot!(render(&parse(&src)));
    }

    #[test]
    fn comments_and_images_snapshot() {
        insta::assert_snapshot!(render(&parse(&fixture("comments-images.md"))));
    }

    /// Every segment's line must contain (the first line of) its text in the source,
    /// except for URLs/comments where the anchor is the node start.
    #[test]
    fn line_numbers_match_source() {
        for name in [
            "plain-instructions.md",
            "skill-body.md",
            "comments-images.md",
        ] {
            let src = fixture(name);
            let lines: Vec<&str> = src.lines().collect();
            for g in parse(&src) {
                let first = g
                    .text
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("")
                    .trim();
                let probe: String = first
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(12)
                    .collect();
                let src_line = lines[g.line - 1];
                assert!(
                    src_line.contains(probe.trim()),
                    "{name}: {} segment {:?} not on line {} ({:?})",
                    g.scope,
                    g.text,
                    g.line,
                    src_line
                );
                assert!(g.span.end <= src.len() && g.span.start <= g.span.end);
            }
        }
    }

    #[test]
    fn explicit_positions() {
        let src = "# Title\n\nSome *text* with `code` and\na [link](https://x.example/a).\n\n```sh\necho hi\n```\n<!-- hidden note -->\n";
        let segs = parse(src);
        let find = |s: Scope| segs.iter().find(|g| g.scope == s).unwrap();
        assert_eq!(find(Scope::Heading).line, 1);
        let t = find(Scope::Text);
        assert_eq!((t.line, t.column), (3, 1));
        assert_eq!(t.text, "Some text with  and\na link.");
        let c = find(Scope::InlineCode);
        assert_eq!((c.line, c.text.as_str()), (3, "code"));
        let l = find(Scope::LinkUrl);
        assert_eq!((l.line, l.text.as_str()), (4, "https://x.example/a"));
        let cb = find(Scope::CodeBlock);
        assert_eq!(
            (cb.line, cb.lang.as_deref(), cb.text.as_str()),
            (7, Some("sh"), "echo hi\n")
        );
        let h = find(Scope::HtmlComment);
        assert_eq!((h.line, h.text.as_str()), (9, "hidden note"));
        assert_eq!(&src[h.span.start..h.span.end], "<!-- hidden note -->");
    }

    #[test]
    fn parse_at_offsets_lines() {
        let full = "---\nname: x\n---\nHello\n";
        let segs = parse_at(full, 16);
        assert_eq!(segs[0].line, 4);
        assert_eq!(segs[0].span.start, 16);
    }

    #[test]
    fn unclosed_comment_runs_to_end() {
        let segs = parse("text\n\n<!-- open comment\nstill hidden\n");
        let h = segs.iter().find(|g| g.scope == Scope::HtmlComment).unwrap();
        assert_eq!(h.text, "open comment\nstill hidden");
        assert_eq!(h.line, 3);
    }
}
