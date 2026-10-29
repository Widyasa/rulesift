//! YAML frontmatter: split from the Markdown body and parse with
//! `serde-saphyr` (never `serde_yaml`).

use crate::model::{Scope, Segment, Span};
use crate::text::LineIndex;
use serde_json::Value;
use std::collections::BTreeMap;

/// Parsed (or attempted) frontmatter block.
#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter {
    /// Raw YAML between the delimiters.
    pub raw: String,
    /// Byte offset of `raw` in the file.
    pub raw_start: usize,
    /// 1-based line of the opening `---`.
    pub start_line: usize,
    /// 1-based line of the closing delimiter (or last line if unclosed).
    pub end_line: usize,
    /// Byte offset where the Markdown body starts.
    pub body_start: usize,
    /// Whether a closing delimiter was found.
    pub closed: bool,
    /// Parsed value (`Ok`) or an English error with its 1-based file line.
    pub value: Result<Value, (String, usize)>,
    /// 1-based file line of each top-level key.
    pub key_lines: BTreeMap<String, usize>,
}

/// Lenient `key: value` reading for dialects whose frontmatter is not strict
/// YAML (Cursor writes `globs: **/*.ts, src/**` unquoted). Every non-blank,
/// non-comment line must be a top-level `key: value`; values are strings
/// except `true`/`false` and the empty value (`null`). Returns `None` when
/// any line does not fit, so real YAML errors still surface.
pub fn lenient(raw: &str) -> Option<Value> {
    let mut m = serde_json::Map::new();
    for line in raw.lines() {
        let t = line.trim_end();
        if t.trim().is_empty() || t.trim_start().starts_with('#') {
            continue;
        }
        let (k, v) = t.split_once(':')?;
        let key_ok = !k.is_empty()
            && k.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && k.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !key_ok {
            return None;
        }
        let v = v.trim();
        let val = match v {
            "" => Value::Null,
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => {
                let unq = v
                    .strip_prefix('"')
                    .and_then(|x| x.strip_suffix('"'))
                    .or_else(|| v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
                    .unwrap_or(v);
                Value::String(unq.to_string())
            }
        };
        m.insert(k.to_string(), val);
    }
    Some(Value::Object(m))
}

impl Frontmatter {
    /// Top-level mapping, if the frontmatter parsed to one.
    pub fn mapping(&self) -> Option<&serde_json::Map<String, Value>> {
        self.value.as_ref().ok().and_then(Value::as_object)
    }

    /// Value of a top-level key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.mapping().and_then(|m| m.get(key))
    }

    /// File line of a top-level key (falls back to the opening delimiter).
    pub fn line_of(&self, key: &str) -> usize {
        self.key_lines.get(key).copied().unwrap_or(self.start_line)
    }

    /// `frontmatter` segments: one per top-level key (`pointer` `/key`), with
    /// strings as-is and other values as compact JSON.
    pub fn segments(&self, full: &str) -> Vec<Segment> {
        let li = LineIndex::new(full);
        let mut out = Vec::new();
        if let Some(m) = self.mapping() {
            for (k, v) in m {
                let line = self.line_of(k);
                let text = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let start = li.line_start(line);
                let end = li.line_start(line + 1).max(start);
                let mut seg =
                    Segment::new(Scope::Frontmatter, text, Span { start, end }, line, 1, line);
                seg.pointer = Some(format!("/{}", k.replace('~', "~0").replace('/', "~1")));
                out.push(seg);
            }
        }
        out.sort_by_key(|s| (s.line, s.pointer.clone()));
        out
    }
}

fn is_delim(line: &str, closing: bool) -> bool {
    let t = line.trim_end_matches(['\r', '\n']).trim_end();
    t == "---" || (closing && t == "...")
}

/// Split frontmatter from `text`. Returns `None` when the file does not start
/// with a `---` line (a UTF-8 BOM is allowed before it).
pub fn split(text: &str) -> Option<Frontmatter> {
    let bom = if text.starts_with('\u{FEFF}') { 3 } else { 0 };
    let rest = &text[bom..];
    let first_end = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
    if !is_delim(&rest[..first_end], false) {
        return None;
    }
    let raw_start = bom + first_end;
    let mut pos = raw_start;
    let mut line_no = 2;
    let mut close: Option<(usize, usize, usize)> = None; // (raw_end, body_start, line)
    while pos < text.len() {
        let end = text[pos..]
            .find('\n')
            .map(|i| pos + i + 1)
            .unwrap_or(text.len());
        if is_delim(&text[pos..end], true) {
            close = Some((pos, end, line_no));
            break;
        }
        pos = end;
        line_no += 1;
    }
    let (raw_end, body_start, end_line, closed) = match close {
        Some((r, b, l)) => (r, b, l, true),
        None => (
            text.len(),
            text.len(),
            line_no.saturating_sub(1).max(1),
            false,
        ),
    };
    let raw = text[raw_start..raw_end].to_string();
    let mut key_lines = BTreeMap::new();
    for (i, l) in raw.lines().enumerate() {
        if l.starts_with([' ', '\t', '-', '#']) || l.is_empty() {
            continue;
        }
        if let Some((k, _)) = l.split_once(':') {
            let k = k.trim().trim_matches(['"', '\'']);
            if !k.is_empty() {
                key_lines.entry(k.to_string()).or_insert(i + 2);
            }
        }
    }
    let value = if !closed {
        Err(("frontmatter is not closed with a `---` line".to_string(), 1))
    } else if raw.trim().is_empty() {
        Ok(Value::Object(Default::default()))
    } else {
        serde_saphyr::from_str::<Value>(&raw).map_err(|e| {
            let line = e.location().map(|l| l.line() as usize + 1).unwrap_or(1);
            let msg = e.to_string();
            let first = msg.lines().next().unwrap_or("invalid YAML").trim();
            let first = first.strip_prefix("error: ").unwrap_or(first);
            (format!("invalid YAML: {first}"), line)
        })
    };
    Some(Frontmatter {
        raw,
        raw_start,
        start_line: 1,
        end_line,
        body_start,
        closed,
        value,
        key_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_parses() {
        let t = "---\nname: pdf\ndescription: \"Reads PDFs\"\nmetadata:\n  a: 1\n---\n# Body\n";
        let fm = split(t).unwrap();
        assert!(fm.closed);
        assert_eq!(fm.get("name").unwrap(), "pdf");
        assert_eq!(fm.line_of("description"), 3);
        assert_eq!(fm.line_of("metadata"), 4);
        assert_eq!(fm.end_line, 6);
        assert_eq!(&t[fm.body_start..], "# Body\n");
        let segs = fm.segments(t);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].pointer.as_deref(), Some("/name"));
        assert_eq!(segs[0].line, 2);
    }

    #[test]
    fn no_frontmatter() {
        assert!(split("# Title\n---\n").is_none());
        assert!(split("").is_none());
    }

    #[test]
    fn crlf_and_bom() {
        let t = "\u{FEFF}---\r\nname: x\r\n---\r\nbody";
        let fm = split(t).unwrap();
        assert_eq!(fm.get("name").unwrap(), "x");
        assert_eq!(&t[fm.body_start..], "body");
    }

    #[test]
    fn invalid_yaml_reports_file_line() {
        let t = "---\nname: x\ntools: [a, b\n---\n";
        let fm = split(t).unwrap();
        let (msg, line) = fm.value.unwrap_err();
        assert!(msg.starts_with("invalid YAML"), "{msg}");
        assert!((2..=4).contains(&line), "{line}");
    }

    #[test]
    fn unclosed() {
        let fm = split("---\nname: x\n").unwrap();
        assert!(!fm.closed);
        assert!(fm.value.is_err());
    }
}

#[cfg(test)]
mod lenient_tests {
    use super::lenient;
    use serde_json::json;

    #[test]
    fn cursor_style_globs() {
        let v = lenient("description: Rust rules\nglobs: **/*.rs, src/**\nalwaysApply: false\n")
            .unwrap();
        assert_eq!(
            v,
            json!({"description": "Rust rules", "globs": "**/*.rs, src/**", "alwaysApply": false})
        );
    }

    #[test]
    fn rejects_non_key_lines() {
        assert!(lenient("globs: **/*\n  - nested\n").is_none());
        assert!(lenient("just text\n").is_none());
    }
}
