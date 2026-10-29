//! Every built-in rule and lock check has a complete `docs/rules/<ID>.md`,
//! and SARIF `helpUri` points at it.

use rulesift_core::report::sarif::help_uri;
use std::path::PathBuf;

const SECTIONS: &[&str] = &[
    "## What it detects",
    "## Why it matters",
    "## Example",
    "## How to fix",
    "## References",
    "## Affected tools",
];

fn docs() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/rules")
}

fn check(id: &str, title: &str, severity: &str, index: &str) {
    let path = docs().join(format!("{id}.md"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{id}: missing {}", path.display()));
    assert!(
        text.starts_with(&format!("# {id}: {title}\n")),
        "{id}: first line must be `# {id}: {title}`"
    );
    assert!(
        text.contains(&format!("| {severity} |")),
        "{id}: severity {severity} not stated"
    );
    let mut last = 0;
    for s in SECTIONS {
        let at = text
            .find(&format!("\n{s}\n"))
            .unwrap_or_else(|| panic!("{id}: missing section `{s}`"));
        assert!(at > last, "{id}: section `{s}` out of order");
        let body = text[at + s.len() + 2..].split("\n## ").next().unwrap();
        assert!(!body.trim().is_empty(), "{id}: section `{s}` is empty");
        last = at;
    }
    assert!(
        text.contains("\n- <http"),
        "{id}: References needs at least one link"
    );
    assert!(
        help_uri(id).ends_with(&format!("/docs/rules/{id}.md")),
        "{id}: helpUri does not point at docs/rules"
    );
    assert!(
        index.contains(&format!("[{id}]({id}.md)")),
        "{id}: not in docs/rules/README.md"
    );
    // docs never contain raw hidden characters
    assert!(
        !text.chars().any(|c| matches!(c as u32, 0xE0000..=0xE007F | 0x200B..=0x200F | 0x202A..=0x202E | 0x2066..=0x2069 | 0xFE00..=0xFE0F)),
        "{id}: render hidden characters as <U+XXXX>"
    );
}

#[test]
fn every_rule_has_complete_docs() {
    let index = std::fs::read_to_string(docs().join("README.md")).unwrap();
    let rules = rulesift_core::rules::loader::builtin().unwrap();
    assert!(rules.len() >= 39);
    for r in &rules {
        check(&r.def.id, &r.def.title, &r.severity.to_string(), &index);
    }
    for (id, title, sev) in [
        (
            "RS-LOCK-001",
            "Locked agent file or MCP server changed",
            "high",
        ),
        ("RS-LOCK-002", "New agent file not in lock", "medium"),
        ("RS-LOCK-003", "Locked agent file missing", "low"),
    ] {
        check(id, title, sev, &index);
    }
    let n = std::fs::read_dir(docs())
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "md")
        })
        .count();
    assert_eq!(n, rules.len() + 3 + 1, "stale doc files in docs/rules");
}
