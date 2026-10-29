//! Terminal table output (PRD §10.4).

use owo_colors::OwoColorize;
use rulesift_core::model::{ChangeOp, Report, Severity, Tier};
use rulesift_core::rules::detectors::{truncate, unicode};
use std::fmt::Write as _;

fn sev(s: Severity, color: bool) -> String {
    let t = format!("{:<8}", s.to_string());
    if !color {
        return t;
    }
    match s {
        Severity::Critical => t.red().bold().to_string(),
        Severity::High => t.red().to_string(),
        Severity::Medium => t.yellow().to_string(),
        Severity::Low => t.dimmed().to_string(),
    }
}

fn tier(t: Tier, color: bool) -> String {
    let l = t.label();
    if !color {
        return l.to_string();
    }
    match t {
        Tier::Block => l.red().bold().to_string(),
        Tier::Review => l.yellow().bold().to_string(),
        Tier::Info => l.cyan().to_string(),
        Tier::Ok => l.green().to_string(),
    }
}

/// Render a report. `quiet` hides files without visible findings.
pub fn render(r: &Report, exit_code: u8, color: bool, quiet: bool) -> String {
    let mut out = String::new();
    let n = r.summary.files_scanned;
    let _ = writeln!(
        out,
        "RuleSift v{} · scanned {n} agent file{}\n",
        r.tool.version,
        if n == 1 { "" } else { "s" }
    );
    let width = r
        .files
        .iter()
        .map(|f| f.path.chars().count())
        .max()
        .unwrap_or(0)
        .clamp(20, 60);
    let id_w = r
        .files
        .iter()
        .flat_map(|f| f.findings.iter())
        .map(|f| f.rule_id.len())
        .max()
        .unwrap_or(10)
        .max(10);
    if !r.changes.is_empty() {
        let w = r
            .changes
            .iter()
            .map(|c| c.path.chars().count())
            .max()
            .unwrap_or(0)
            .min(40);
        let _ = writeln!(out, "Agent capability changes:");
        for c in &r.changes {
            let sym = c.op.symbol();
            let sym = if color {
                match c.op {
                    ChangeOp::Added => sym.green().to_string(),
                    ChangeOp::Removed => sym.red().to_string(),
                    ChangeOp::Changed => sym.yellow().to_string(),
                    ChangeOp::Lock => sym.red().bold().to_string(),
                }
            } else {
                sym.to_string()
            };
            let _ = writeln!(
                out,
                "  {sym} {:<w$}  {}",
                unicode::render(&c.path),
                unicode::render(&c.detail)
            );
        }
        out.push('\n');
    }
    for f in &r.files {
        let live: Vec<_> = f.findings.iter().filter(|x| !x.suppressed).collect();
        let hidden = f.findings.len() - live.len();
        if quiet && live.is_empty() {
            continue;
        }
        let path = unicode::render(&f.path);
        let _ = writeln!(
            out,
            "{path:<width$}  score {:<3}  {}",
            f.score,
            tier(f.tier, color)
        );
        let pad = " ".repeat(2 + 8 + 2 + id_w + 2);
        for x in live {
            let msg = truncate(&unicode::render(&x.message), 200);
            let _ = writeln!(
                out,
                "  {}  {:<id_w$}  line {}  {}",
                sev(x.severity, color),
                x.rule_id,
                x.line,
                msg
            );
            if !x.evidence.is_empty() && !msg.contains(x.evidence.as_str()) {
                let _ = writeln!(out, "{pad}evidence: {}", x.evidence);
            }
            if let Some(d) = &x.decoded {
                let _ = writeln!(out, "{pad}decoded text: {:?}", unicode::render(d));
            }
        }
        if hidden > 0 {
            let _ = writeln!(out, "  ({hidden} suppressed)");
        }
        if f.findings.is_empty() {
            continue;
        }
        out.push('\n');
    }
    if !out.ends_with("\n\n") {
        out.push('\n');
    }
    let s = &r.summary;
    let _ = writeln!(
        out,
        "Result: {} block, {} review, {} info  ·  exit code {exit_code}",
        s.block, s.review, s.info
    );
    out
}
