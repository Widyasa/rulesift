//! JSON with a JSON-pointer → position map (PRD §5 F2).
//!
//! The value is parsed with `serde_json`; a small scanner then walks the same
//! text and records the byte span and line/column of every value. Comments
//! (`//`, `/* */`) and trailing commas are accepted (JSONC, used by VS Code
//! and Zed settings) by blanking them out before parsing, which keeps every
//! offset intact.

use crate::model::{Scope, Segment, Span};
use crate::text::LineIndex;
use serde_json::Value;
use std::collections::BTreeMap;

/// Position of one JSON value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonPos {
    /// Byte offset of the value start.
    pub start: usize,
    /// Byte offset after the value end.
    pub end: usize,
    /// 1-based line of the value start.
    pub line: usize,
    /// 1-based character column of the value start.
    pub column: usize,
    /// 1-based line of the object key that owns the value, if any.
    pub key_line: Option<usize>,
}

/// Parse error with position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    /// English message.
    pub message: String,
    /// 1-based line.
    pub line: usize,
    /// 1-based column.
    pub column: usize,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid JSON at line {} column {}: {}",
            self.line, self.column, self.message
        )
    }
}

/// Parsed JSON document.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonDoc {
    /// The value.
    pub value: Value,
    /// Positions by JSON pointer (`""` is the root).
    pub positions: BTreeMap<String, JsonPos>,
}

/// Escape one JSON pointer token.
pub fn escape_token(t: &str) -> String {
    t.replace('~', "~0").replace('/', "~1")
}

impl JsonDoc {
    /// Position of a pointer.
    pub fn pos(&self, ptr: &str) -> Option<&JsonPos> {
        self.positions.get(ptr)
    }

    /// Line of a pointer, falling back to the nearest ancestor, then line 1.
    pub fn line_of(&self, ptr: &str) -> usize {
        let mut p = ptr.to_string();
        loop {
            if let Some(pos) = self.positions.get(&p) {
                return pos.key_line.unwrap_or(pos.line);
            }
            match p.rfind('/') {
                Some(i) => p.truncate(i),
                None => return 1,
            }
        }
    }

    /// One `json_string` segment per string value (decoded text, pointer set).
    pub fn segments(&self, text: &str) -> Vec<Segment> {
        let li = LineIndex::new(text);
        let mut out = Vec::new();
        for (ptr, pos) in &self.positions {
            if let Some(Value::String(s)) = self.value.pointer(ptr) {
                let (end_line, _) = li.position(pos.end.saturating_sub(1).max(pos.start));
                let mut seg = Segment::new(
                    Scope::JsonString,
                    s.clone(),
                    Span {
                        start: pos.start,
                        end: pos.end,
                    },
                    pos.line,
                    pos.column,
                    end_line,
                );
                seg.pointer = Some(ptr.clone());
                out.push(seg);
            }
        }
        out.sort_by_key(|s| s.span.start);
        out
    }
}

/// Replace `//` and `/* */` comments and trailing commas with spaces
/// (newlines kept), leaving strings untouched.
pub fn strip_jsonc(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    out[i] = b' ';
                    i += 1;
                }
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let mut j = i;
                while j < b.len() && !(b[j] == b'*' && b.get(j + 1) == Some(&b'/')) {
                    if b[j] != b'\n' {
                        out[j] = b' ';
                    }
                    j += 1;
                }
                let stop = (j + 2).min(b.len());
                out[j.min(stop)..stop].fill(b' ');
                i = j + 2;
                continue;
            }
            b',' => {
                // trailing comma: next non-space (in the already-stripped output) is } or ]
                let mut j = i + 1;
                while j < b.len() {
                    if b[j].is_ascii_whitespace() {
                        j += 1;
                    } else if b[j] == b'/'
                        && (b.get(j + 1) == Some(&b'/') || b.get(j + 1) == Some(&b'*'))
                    {
                        // skip comment
                        if b[j + 1] == b'/' {
                            while j < b.len() && b[j] != b'\n' {
                                j += 1;
                            }
                        } else {
                            j += 2;
                            while j < b.len() && !(b[j] == b'*' && b.get(j + 1) == Some(&b'/')) {
                                j += 1;
                            }
                            j += 2;
                        }
                    } else {
                        break;
                    }
                }
                if matches!(b.get(j), Some(b'}') | Some(b']')) {
                    out[i] = b' ';
                }
            }
            _ => {}
        }
        i += 1;
    }
    // INVARIANT: only ASCII bytes outside strings were replaced by ASCII spaces,
    // so the buffer is still valid UTF-8.
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Parse JSON (or JSONC) text.
pub fn parse(text: &str) -> Result<JsonDoc, JsonError> {
    let clean = strip_jsonc(text);
    let value: Value = serde_json::from_str(&clean).map_err(|e| JsonError {
        message: strip_serde_pos(&e.to_string()),
        line: e.line().max(1),
        column: e.column().max(1),
    })?;
    let li = LineIndex::new(&clean);
    let mut sc = Scanner {
        b: clean.as_bytes(),
        i: 0,
        li: &li,
        out: BTreeMap::new(),
    };
    sc.ws();
    sc.value(String::new(), None);
    Ok(JsonDoc {
        value,
        positions: sc.out,
    })
}

fn strip_serde_pos(s: &str) -> String {
    match s.rfind(" at line ") {
        Some(i) => s[..i].to_string(),
        None => s.to_string(),
    }
}

struct Scanner<'a> {
    b: &'a [u8],
    i: usize,
    li: &'a LineIndex<'a>,
    out: BTreeMap<String, JsonPos>,
}

impl Scanner<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn string_end(&self, start: usize) -> usize {
        let mut j = start + 1;
        while j < self.b.len() {
            match self.b[j] {
                b'\\' => j += 2,
                b'"' => return j + 1,
                _ => j += 1,
            }
        }
        self.b.len()
    }

    fn record(&mut self, ptr: String, start: usize, end: usize, key_line: Option<usize>) {
        let (line, column) = self.li.position(start);
        self.out.insert(
            ptr,
            JsonPos {
                start,
                end,
                line,
                column,
                key_line,
            },
        );
    }

    fn value(&mut self, ptr: String, key_line: Option<usize>) {
        self.ws();
        let start = self.i;
        match self.b.get(self.i) {
            Some(b'{') => {
                self.i += 1;
                loop {
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b'}') => {
                            self.i += 1;
                            break;
                        }
                        Some(b',') => {
                            self.i += 1;
                            continue;
                        }
                        Some(b'"') => {
                            let ks = self.i;
                            let ke = self.string_end(ks);
                            let raw = std::str::from_utf8(&self.b[ks..ke]).unwrap_or("\"\"");
                            let key: String = serde_json::from_str(raw).unwrap_or_default();
                            let (kl, _) = self.li.position(ks);
                            self.i = ke;
                            self.ws();
                            if self.b.get(self.i) == Some(&b':') {
                                self.i += 1;
                            }
                            let child = format!("{ptr}/{}", escape_token(&key));
                            self.value(child, Some(kl));
                        }
                        _ => break,
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut idx = 0usize;
                loop {
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b']') => {
                            self.i += 1;
                            break;
                        }
                        Some(b',') => {
                            self.i += 1;
                            continue;
                        }
                        Some(_) => {
                            self.value(format!("{ptr}/{idx}"), None);
                            idx += 1;
                        }
                        None => break,
                    }
                }
            }
            Some(b'"') => self.i = self.string_end(start),
            Some(_) => {
                while self.i < self.b.len()
                    && !matches!(self.b[self.i], b',' | b'}' | b']')
                    && !self.b[self.i].is_ascii_whitespace()
                {
                    self.i += 1;
                }
            }
            None => return,
        }
        let end = self.i;
        self.record(ptr, start, end, key_line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_keys_and_arrays() {
        let t = "{\n  \"mcpServers\": {\n    \"helper\": {\n      \"command\": \"npx\",\n      \"args\": [\"-y\", \"pkg\"]\n    }\n  }\n}\n";
        let d = parse(t).unwrap();
        let p = d.pos("/mcpServers/helper/command").unwrap();
        assert_eq!((p.line, p.column), (4, 18));
        assert_eq!(&t[p.start..p.end], "\"npx\"");
        let a = d.pos("/mcpServers/helper/args/1").unwrap();
        assert_eq!((a.line, a.column), (5, 22));
        assert_eq!(d.pos("/mcpServers/helper").unwrap().key_line, Some(3));
        assert_eq!(d.line_of("/mcpServers/helper/missing"), 3);
        assert_eq!(d.pos("").unwrap().line, 1);
    }

    #[test]
    fn escaped_strings_and_keys() {
        let t = r#"{"a\"b": "x\"y\\", "c/d": {"e~f": "\u00e9"}, "n": 1}"#;
        let d = parse(t).unwrap();
        assert!(d.pos("/a\"b").is_some());
        let p = d.pos("/c~1d/e~0f").unwrap();
        assert_eq!(&t[p.start..p.end], "\"\\u00e9\"");
        let n = d.pos("/n").unwrap();
        assert_eq!(&t[n.start..n.end], "1");
        let segs = d.segments(t);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].text, "x\"y\\");
        assert_eq!(segs[1].text, "é");
        assert_eq!(segs[1].pointer.as_deref(), Some("/c~1d/e~0f"));
    }

    #[test]
    fn crlf_files() {
        let t = "{\r\n  \"a\": {\r\n    \"b\": \"c\"\r\n  },\r\n  \"d\": [1, \"e\"]\r\n}\r\n";
        let d = parse(t).unwrap();
        assert_eq!(
            (d.pos("/a/b").unwrap().line, d.pos("/a/b").unwrap().column),
            (3, 10)
        );
        assert_eq!(d.pos("/d/1").unwrap().line, 5);
    }

    #[test]
    fn jsonc_comments_and_trailing_commas() {
        let t = "{\n  // comment \"x\"\n  \"servers\": { /* c */ \"a\": \"b\", },\n  \"u\": \"http://x.example/y\",\n}\n";
        let d = parse(t).unwrap();
        assert_eq!(d.value["servers"]["a"], "b");
        assert_eq!(d.pos("/servers/a").unwrap().line, 3);
        assert_eq!(d.value["u"], "http://x.example/y");
    }

    #[test]
    fn invalid_json_has_position() {
        let e = parse("{\n  \"a\": [1, 2\n}\n").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.to_string().starts_with("invalid JSON at line 3"));
    }
}
