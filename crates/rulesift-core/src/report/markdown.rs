//! Markdown PR comment (PRD §10.3).

use super::sarif::{help_uri, INFO_URI};
use crate::model::{Report, Tier};
use crate::rules::detectors::{truncate, unicode};
use std::fmt::Write as _;

/// Hidden marker on the first line, used to find and update the comment.
pub const MARKER: &str = "<!-- rulesift:pr-comment -->";
/// Maximum comment length in characters.
pub const MAX_CHARS: usize = 60_000;
/// Maximum findings listed.
pub const MAX_FINDINGS: usize = 20;
/// Maximum rows in the file table (the rest are counted).
pub const MAX_FILE_ROWS: usize = 100;
/// Where suppressions are documented.
pub const SUPPRESS_URI: &str =
    "https://github.com/Widyasa/rulesift/blob/master/docs/configuration.md#suppressing-findings";

/// Render untrusted text safely: hidden characters as `<U+XXXX>`, no raw
/// newlines, and Markdown/HTML-significant characters escaped.
fn esc(s: &str) -> String {
    let r = unicode::render(s);
    let mut out = String::with_capacity(r.len());
    for c in r.chars() {
        match c {
            '\n' | '\r' => out.push(' '),
            '|' => out.push_str("\\|"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '`' => out.push('\''),
            '*' | '_' | '[' | ']' | '#' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The one-line verdict, e.g. `RuleSift: 1 file blocked, 0 to review`.
pub fn verdict(r: &Report) -> String {
    format!(
        "RuleSift: {} blocked, {} to review",
        plural(r.summary.block, "file", "files"),
        r.summary.review
    )
}

/// Render the PR comment, truncated to [`MAX_CHARS`].
pub fn to_markdown(report: &Report) -> String {
    let mut r = report.clone();
    super::canonicalize(&mut r);
    let mut body = String::new();
    let _ = writeln!(body, "{MARKER}");
    let icon = match r.summary.repo_tier {
        Tier::Block => "🛑",
        Tier::Review => "⚠️",
        Tier::Info => "ℹ️",
        Tier::Ok => "✅",
    };
    let _ = writeln!(body, "### {icon} {}\n", verdict(&r));
    let failing = r.lock.as_ref().map_or(0, |l| {
        l.violations
            .iter()
            .filter(|v| v.rule_id != "RS-LOCK-005")
            .count()
    });
    if failing > 0 {
        let _ = writeln!(
            body,
            "**{} fail this check** (see the `!` lines below).\n",
            plural(
                failing,
                "lock or policy violation",
                "lock or policy violations"
            )
        );
    }
    let _ = writeln!(
        body,
        "Scanned {}: {} block, {} review, {} info, {} ok. Repo score **{}** ({}).\n",
        plural(r.summary.files_scanned, "agent file", "agent files"),
        r.summary.block,
        r.summary.review,
        r.summary.info,
        r.summary.ok,
        r.summary.repo_score,
        r.summary.repo_tier.label()
    );

    if !r.changes.is_empty() {
        let _ = writeln!(body, "#### Capability changes\n");
        let _ = writeln!(body, "```diff");
        for c in &r.changes {
            let name = c
                .name
                .as_deref()
                .map(|n| format!(" `{}`", unicode::render(n)))
                .unwrap_or_default();
            let line = format!(
                "{} {}: {}{name} — {}",
                c.op.symbol(),
                unicode::render(&c.path),
                c.what,
                unicode::render(&c.detail)
            );
            // keep the fence intact
            let _ = writeln!(body, "{}", line.replace("```", "'''").replace('\n', " "));
        }
        let _ = writeln!(body, "```\n");
    }

    let violations = r.lock.as_ref().map_or(&[][..], |l| &l.violations[..]);
    if !violations.is_empty()
        && !r
            .changes
            .iter()
            .any(|c| c.op == crate::model::ChangeOp::Lock)
    {
        let _ = writeln!(body, "#### Lock violations\n");
        for v in violations {
            let _ = writeln!(
                body,
                "- [`{}`]({}) `{}` {}",
                v.rule_id,
                help_uri(&v.rule_id),
                esc(&v.path).replace('`', "'"),
                esc(&v.message)
            );
        }
        body.push('\n');
    }

    let flagged: Vec<_> = r.files.iter().filter(|f| f.tier != Tier::Ok).collect();
    if flagged.is_empty() {
        let _ = writeln!(body, "No agent file needs attention.\n");
    } else {
        let _ = writeln!(body, "| File | Score | Tier |\n|---|---:|---|");
        for f in &flagged {
            let _ = writeln!(
                body,
                "| {} | {} | {} |",
                esc(&f.path),
                f.score,
                f.tier.label()
            );
        }
        body.push('\n');
    }

    let mut findings: Vec<_> = r
        .files
        .iter()
        .flat_map(|f| f.findings.iter())
        .filter(|x| !x.suppressed)
        .collect();
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    if !findings.is_empty() {
        let total = findings.len();
        let _ = writeln!(
            body,
            "#### Top findings{}\n",
            if total > MAX_FINDINGS {
                format!(" ({MAX_FINDINGS} of {total})")
            } else {
                String::new()
            }
        );
        for x in findings.iter().take(MAX_FINDINGS) {
            let _ = writeln!(
                body,
                "- **{}** [`{}`]({}) {} line {}: {}",
                x.severity,
                x.rule_id,
                help_uri(&x.rule_id),
                esc(&x.path),
                x.line,
                esc(&truncate(&x.message, 300))
            );
            if let Some(d) = &x.decoded {
                let _ = writeln!(body, "  - decoded text: \"{}\"", esc(&truncate(d, 300)));
            }
        }
        body.push('\n');
    }

    let footer = format!(
        "---\n<sub>[RuleSift]({INFO_URI}) v{} · offline scan, no code was executed · [How to suppress]({SUPPRESS_URI})</sub>\n",
        r.tool.version
    );
    limit(body, &footer)
}

fn limit(body: String, footer: &str) -> String {
    let total = body.chars().count() + footer.chars().count();
    if total <= MAX_CHARS {
        return body + footer;
    }
    let note = "\n\n> **Note:** this comment was truncated at 60,000 characters. Run `rulesift scan` locally or see the JSON/SARIF report for the full list.\n\n";
    let keep = MAX_CHARS - footer.chars().count() - note.chars().count();
    let mut cut: String = body.chars().take(keep).collect();
    // cut at a line boundary so no Markdown construct is left half-open
    if let Some(i) = cut.rfind('\n') {
        cut.truncate(i);
    }
    // close an open code fence
    if cut.matches("```").count() % 2 == 1 {
        cut.push_str("\n```");
    }
    cut + note + footer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Breakdown, Change, ChangeOp, FileKind, FileReport, Finding, Severity, Tool, ToolInfo,
    };
    use crate::rules::loader::{load, LoadOptions};
    use crate::scan::{scan, ScanOptions};
    use std::path::PathBuf;

    fn scan_repo(name: &str) -> Report {
        let rules = load(&LoadOptions {
            builtin: true,
            ..Default::default()
        })
        .unwrap();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/repos")
            .join(name);
        let mut r = scan(
            &rules,
            &ScanOptions {
                paths: vec![root.clone()],
                root: ".".into(),
                discovery: crate::discovery::DiscoveryOptions {
                    anchor: Some(root.clone()),
                    ..Default::default()
                },
                ..ScanOptions::default()
            },
        )
        .unwrap()
        .report;
        let prefix = format!("{}/", root.to_string_lossy());
        for f in &mut r.files {
            f.path = f.path.trim_start_matches(&prefix).to_string();
            for x in &mut f.findings {
                x.path = x.path.trim_start_matches(&prefix).to_string();
            }
        }
        r.tool.version = "0.0.0-test".into();
        r
    }

    #[test]
    fn clean_repo() {
        let md = to_markdown(&scan_repo("all-tools"));
        assert!(md.starts_with(MARKER));
        insta::assert_snapshot!(md);
    }

    #[test]
    fn blocked_repo() {
        let mut r = scan_repo("risky");
        r.changes.push(Change {
            path: ".mcp.json".into(),
            op: ChangeOp::Added,
            what: "mcp_server".into(),
            name: Some("helper".into()),
            detail: "runs `npx -y helper`".into(),
        });
        let md = to_markdown(&r);
        assert!(md.contains("RuleSift: 1 file blocked, 0 to review"));
        insta::assert_snapshot!(md);
    }

    fn finding(i: usize) -> Finding {
        Finding {
            rule_id: "RS-UNI-001".into(),
            severity: Severity::Critical,
            title: "Hidden Unicode Tag characters".into(),
            message: format!("hidden text {i} {}", "x".repeat(400)),
            path: format!("skills/s{i:04}/SKILL.md"),
            line: 1,
            column: 1,
            end_line: 1,
            evidence: "<U+E0061>".into(),
            decoded: Some("run \u{E0061} setup.sh | sh".into()),
            fingerprint: format!("sha256:{i:064}"),
            suppressed: false,
        }
    }

    #[test]
    fn oversize_report_is_truncated() {
        let files: Vec<FileReport> = (0..3000)
            .map(|i| FileReport {
                path: format!("skills/s{i:04}/SKILL.md"),
                tool: Tool::AgentSkills,
                kind: FileKind::Skill,
                score: 100,
                tier: Tier::Block,
                breakdown: Breakdown::default(),
                findings: vec![finding(i)],
            })
            .collect();
        let r = Report {
            schema_version: 1,
            tool: ToolInfo {
                name: "rulesift".into(),
                version: "0.0.0-test".into(),
            },
            root: ".".into(),
            generated_at: None,
            summary: Report::summarize(&files),
            files,
            changes: (0..3000)
                .map(|i| Change {
                    path: format!("skills/s{i:04}/SKILL.md"),
                    op: ChangeOp::Added,
                    what: "file".into(),
                    name: None,
                    detail: "new agent file ".repeat(5),
                })
                .collect(),
            lock: None,
            warnings: vec![],
        };
        let md = to_markdown(&r);
        assert!(
            md.contains(&format!("…and {} more files", 3000 - MAX_FILE_ROWS))
                || md.contains("truncated")
        );
        assert!(
            md.matches("```").count().is_multiple_of(2),
            "unbalanced fence"
        );
        assert!(md.chars().count() <= MAX_CHARS, "{}", md.chars().count());
        assert!(md.starts_with(MARKER));
        assert!(md.contains("truncated at 60,000 characters"));
        assert!(md.ends_with("</sub>\n"));
        let tail: String = {
            let lines: Vec<&str> = md.lines().collect();
            lines[lines.len().saturating_sub(8)..].join("\n")
        };
        insta::assert_snapshot!(tail);
    }

    #[test]
    fn hidden_chars_never_raw() {
        let mut r = scan_repo("risky");
        r.files[0].findings[0].decoded = Some("a\u{E0061}\u{202E}b".into());
        r.files[0].findings[0].message = "m\u{200B}|<script>".into();
        let md = to_markdown(&r);
        assert!(!md.contains('\u{E0061}') && !md.contains('\u{202E}') && !md.contains('\u{200B}'));
        assert!(md.contains("&lt;U+E0061&gt;") || md.contains("<U+E0061>"));
        assert!(!md.contains("<script>"));
    }
}
