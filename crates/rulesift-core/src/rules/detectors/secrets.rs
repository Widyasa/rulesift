//! `hardcoded_secret` detector (RS-SEC-001): provider token formats,
//! private key blocks, and generic `sk-` / `api_key = "…"` values with an
//! entropy check. Evidence is always redacted.

use super::entropy::{is_placeholder, looks_random, redact};
use super::Loc;
use crate::analyze::Analyzed;
use crate::model::Span;
use crate::text::LineIndex;
use regex::Regex;
use std::sync::OnceLock;

/// (label, regex, needs entropy check). Capture group 1, if present, is the
/// secret value.
const PATTERNS: &[(&str, &str, bool)] = &[
    (
        "AWS access key id",
        r"\b((?:AKIA|ASIA|AGPA|AIDA|AROA|ANPA|ANVA|AIPA)[0-9A-Z]{16})\b",
        false,
    ),
    (
        "GitHub token",
        r"\b((?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22,})\b",
        false,
    ),
    ("Slack token", r"\b(xox[abposr]-[A-Za-z0-9-]{10,})\b", false),
    (
        "private key block",
        r"(-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP |ENCRYPTED )?PRIVATE KEY(?: BLOCK)?-----)",
        false,
    ),
    (
        "API key",
        r"\b((?:sk|rk)[-_](?:proj-|ant-(?:api\d+-)?|live_|test_|svcacct-)?[A-Za-z0-9_-]{20,})",
        true,
    ),
    (
        "secret value",
        r#"(?i)\b(?:api[_-]?key|apikey|secret(?:[_-]?key)?|access[_-]?token|auth[_-]?token|token|password|passwd|client[_-]?secret|private[_-]?key)\b["']?\s*[:=]\s*["']([^"'\s]{16,})["']"#,
        true,
    ),
];

fn compiled() -> &'static [(&'static str, Regex, bool)] {
    static RE: OnceLock<Vec<(&'static str, Regex, bool)>> = OnceLock::new();
    RE.get_or_init(|| {
        PATTERNS
            .iter()
            .filter_map(|(l, p, e)| Regex::new(p).ok().map(|r| (*l, r, *e)))
            .collect()
    })
}

/// Secrets in `text`: (start, end, label, value).
pub fn find(text: &str) -> Vec<(usize, usize, &'static str, String)> {
    let mut out: Vec<(usize, usize, &'static str, String)> = Vec::new();
    for (label, re, entropy) in compiled() {
        for c in re.captures_iter(text) {
            let Some(m) = c.get(1).or_else(|| c.get(0)) else {
                continue;
            };
            let v = m.as_str();
            if is_placeholder(v) || (*entropy && !looks_random(v)) {
                continue;
            }
            if out
                .iter()
                .any(|(s, e, _, _)| m.start() < *e && *s < m.end())
            {
                continue;
            }
            out.push((m.start(), m.end(), *label, v.to_string()));
        }
    }
    out.sort_by_key(|x| x.0);
    out
}

/// Run the detector over a file (raw text; decoded segments too).
pub fn hardcoded_secret(a: &Analyzed) -> Vec<Loc> {
    let text = a.doc.text.as_str();
    let li = LineIndex::new(text);
    let mut out = Vec::new();
    for (s, e, label, v) in find(text) {
        let (line, column) = li.position(s);
        let shown = if label == "private key block" {
            v.clone()
        } else {
            redact(&v)
        };
        out.push(Loc {
            span: Span { start: s, end: e },
            line,
            column,
            end_line: li.position(e.saturating_sub(1).max(s)).0,
            evidence: format!("{label}: {shown}"),
            decoded: None,
            message: Some(format!(
                "Hardcoded {label} ({shown}). Move it to a secret store or an environment variable reference."
            )),
            synthetic: false,
        });
    }
    for seg in a
        .doc
        .segments
        .iter()
        .filter(|s| s.scope == crate::model::Scope::Decoded)
    {
        for (_, _, label, v) in find(&seg.text) {
            out.push(Loc {
                span: seg.span,
                line: seg.line,
                column: seg.column,
                end_line: seg.end_line,
                evidence: format!("{label} in hidden text: {}", redact(&v)),
                decoded: Some(format!("{label}: {}", redact(&v))),
                message: Some(format!(
                    "Hidden text contains a hardcoded {label} ({}).",
                    redact(&v)
                )),
                synthetic: false,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fake values are assembled at runtime so the source never contains a
    // token-shaped string.
    fn fake(prefix: &str, body: &str) -> String {
        format!("{prefix}{body}")
    }

    #[test]
    fn finds_formats_and_skips_placeholders() {
        let gh = fake("gh", "p_EXAMPLE0a1B2c3D4e5F6g7H8i9J0kLmNoPq1");
        assert_eq!(gh.len(), 40);
        let aws = fake("AKIA", "IOSFODNN7EXAMPLE");
        let sk = fake("sk", "-EXAMPLE7f3Kq9Zr2LmW8xYt4");
        let t = format!("a {gh}\nb {aws}\nc {sk}\napi_key = \"EXAMPLEq8Zt3Lw9Xv2Rk7\"\n");
        let f = find(&t);
        let labels: Vec<&str> = f.iter().map(|x| x.2).collect();
        assert_eq!(
            labels,
            vec![
                "GitHub token",
                "AWS access key id",
                "API key",
                "secret value"
            ]
        );
        let neg = "api_key = \"${OPENAI_API_KEY}\"\ntoken: \"<your-key-goes-here>\"\nsk-xxxxxxxxxxxxxxxxxxxxxxxx\npassword = \"aaaaaaaaaaaaaaaaaaaa\"\n";
        assert!(find(neg).is_empty(), "{:?}", find(neg));
    }

    #[test]
    fn evidence_is_redacted() {
        let row =
            crate::registry::match_path("AGENTS.md", crate::registry::Origin::Explicit).unwrap();
        let sk = fake("sk", "-EXAMPLE7f3Kq9Zr2LmW8xYt4");
        let a = crate::analyze::analyze(
            "AGENTS.md",
            "AGENTS.md",
            row,
            None,
            &format!("Use {sk} here.\n"),
        );
        let l = hardcoded_secret(&a);
        assert_eq!(l.len(), 1);
        assert!(!l[0].evidence.contains(&sk) && !l[0].message.as_ref().unwrap().contains(&sk));
        assert_eq!(l[0].evidence, "API key: sk-E…[redacted 27 chars]");
    }
}
