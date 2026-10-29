//! Entropy and placeholder helpers for secret detection (RS-SEC-001,
//! RS-MCP-002).

/// Shannon entropy in bits per character.
pub fn shannon(s: &str) -> f64 {
    let n = s.chars().count();
    if n == 0 {
        return 0.0;
    }
    let mut counts: std::collections::BTreeMap<char, usize> = Default::default();
    for c in s.chars() {
        *counts.entry(c).or_default() += 1;
    }
    counts
        .values()
        .map(|&k| {
            let p = k as f64 / n as f64;
            -p * p.log2()
        })
        .sum()
}

/// Obvious placeholders and variable references, which are not secrets:
/// `${OPENAI_API_KEY}`, `$TOKEN`, `<your-key>`, `{{ key }}`, `%KEY%`,
/// `your-api-key-here`, `xxxx`, `****`, `changeme`, ...
pub fn is_placeholder(s: &str) -> bool {
    let t = s.trim().trim_matches(['"', '\'']);
    let l = t.to_ascii_lowercase();
    t.is_empty()
        || t.starts_with('$')
        || t.starts_with('<')
        || t.starts_with("{{")
        || (t.starts_with('%') && t.ends_with('%'))
        || l.starts_with("env:")
        || l.starts_with("process.env")
        || l.starts_with("os.environ")
        || l.contains("your")
        || l.contains("xxxx")
        || t.contains("****")
        || t.contains("...")
        || l.contains("changeme")
        || l.contains("placeholder")
        || l.contains("replace_me")
        || l.contains("replace-me")
        || l.contains("redacted")
        || l.contains("insert")
        || t.chars().collect::<std::collections::BTreeSet<_>>().len() <= 3
}

/// Does `s` look like a random secret: long enough, mixed letters and
/// digits, and high entropy?
pub fn looks_random(s: &str) -> bool {
    let n = s.chars().count();
    n >= 16
        && s.chars().any(|c| c.is_ascii_alphabetic())
        && s.chars().any(|c| c.is_ascii_digit())
        && shannon(s) >= 3.0
        && !is_placeholder(s)
}

/// Redact a secret for evidence: keep the first 4 characters.
pub fn redact(s: &str) -> String {
    let head: String = s.chars().take(4).collect();
    format!("{head}…[redacted {} chars]", s.chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_and_placeholders() {
        assert!(shannon("aaaa") < 0.01);
        assert!(shannon("a1B2c3D4e5F6g7H8") > 3.5);
        for p in [
            "${OPENAI_API_KEY}",
            "$TOKEN",
            "<your-key>",
            "{{ api_key }}",
            "%API_KEY%",
            "your-api-key-here",
            "sk-xxxxxxxxxxxxxxxxxxxx",
            "********",
            "changeme",
        ] {
            assert!(is_placeholder(p), "{p}");
            assert!(!looks_random(p), "{p}");
        }
        assert!(looks_random("EXAMPLE7f3Kq9Zr2LmW8xYt4"));
        assert!(!looks_random("aaaaaaaaaaaaaaaaaaaa1"));
        assert_eq!(redact("EXAMPLEKEY123"), "EXAM…[redacted 13 chars]");
    }
}
