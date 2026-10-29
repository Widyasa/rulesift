//! `exfil_url` detector (RS-EXF-001): Markdown images and links (and raw
//! HTML `<img src>`) whose URL carries templated or variable data, the
//! pattern used to leak data through auto-loaded images.

use super::{truncate, unicode, Loc};
use crate::analyze::Analyzed;
use crate::model::{Scope, Span};
use crate::text::LineIndex;
use regex::Regex;
use std::sync::OnceLock;

fn re(cell: &'static OnceLock<Regex>, p: &str) -> &'static Regex {
    // INVARIANT: all patterns in this module are constants that compile.
    cell.get_or_init(|| Regex::new(p).unwrap_or_else(|_| unreachable!()))
}

fn template_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(
        &R,
        r"\{\{?[^}]*\}\}?|\$\{[^}]*\}|\$\(|\$[A-Za-z_][A-Za-z0-9_]*|%7[Bb].*?%7[Dd]|<[^>]+>|%3[Cc]|\[[A-Z_]{3,}\]|=(?:DATA|SECRET|SECRETS|TOKEN|PROMPT|CONVERSATION|CHAT|HISTORY|INPUT|CONTEXT|ENV|KEY|PASSWORD|INFO|INSERT_[A-Z_]+|REPLACE_[A-Z_]+)(?:&|$)",
    )
}

fn img_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r#"(?i)<img\b[^>]*?\bsrc\s*=\s*["']([^"']+)["']"#)
}

/// Why `url` looks like a data-leaking URL, if it does.
pub fn dynamic_part(url: &str) -> Option<String> {
    let u = url.trim();
    let lower = u.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .or_else(|| lower.strip_prefix("//"))?;
    // Only the part after the host is templated in practice.
    let host_len = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let tail_start = u.len() - rest.len() + host_len;
    let tail = &u[tail_start..];
    template_re().find(tail).map(|m| m.as_str().to_string())
}

/// Run the detector.
pub fn exfil_url(a: &Analyzed) -> Vec<Loc> {
    let text = a.doc.text.as_str();
    let li = LineIndex::new(text);
    let mut out = Vec::new();
    let mut push = |span: Span, line: usize, column: usize, url: &str, part: String| {
        let shown = truncate(&unicode::render(url), 200);
        out.push(Loc {
            span,
            line,
            column,
            end_line: line,
            evidence: shown.clone(),
            decoded: None,
            message: Some(format!(
                "URL carries templated or variable data ({}) and would send it to the server when loaded: {}",
                truncate(&unicode::render(&part), 40),
                truncate(&shown, 120)
            )),
            synthetic: false,
        });
    };
    for s in &a.doc.segments {
        match s.scope {
            Scope::ImageUrl | Scope::LinkUrl => {
                if let Some(p) = dynamic_part(&s.text) {
                    push(s.span, s.line, s.column, &s.text, p);
                }
            }
            Scope::Text | Scope::HtmlComment => {
                let base = text
                    .get(s.span.start..s.span.end)
                    .and_then(|sl| sl.find(s.text.as_str()))
                    .map(|o| s.span.start + o);
                for c in img_re().captures_iter(&s.text) {
                    let Some(m) = c.get(1) else { continue };
                    if let Some(p) = dynamic_part(m.as_str()) {
                        let (line, column) = match base {
                            Some(b) => li.position(b + m.start()),
                            None => (s.line, s.column),
                        };
                        push(s.span, line, column, m.as_str(), p);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_urls() {
        for u in [
            "https://x.example/p.png?d={{conversation}}",
            "https://x.example/c?q=$SECRET",
            "https://x.example/i.png?data=${DATA}",
            "https://x.example/log/%7Bsecret%7D.png",
            "https://x.example/p?d=<chat history>",
            "https://x.example/p?d=DATA",
            "//x.example/p?d=[SUMMARY]",
        ] {
            assert!(dynamic_part(u).is_some(), "{u}");
        }
        for u in [
            "https://img.shields.io/badge/build-passing-green.svg",
            "https://x.example/logo.png?v=2",
            "https://x.example/docs#section",
            "./images/local.png",
            "https://x.example/a?q=data",
        ] {
            assert!(dynamic_part(u).is_none(), "{u}");
        }
    }
}
