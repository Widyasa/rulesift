//! Built-in Rust detectors referenced by name from YAML rules (PRD §8).

pub mod unicode;

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
