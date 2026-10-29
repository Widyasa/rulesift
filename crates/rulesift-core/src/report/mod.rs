//! Report serializers (PRD §10): JSON, SARIF, Markdown.

pub mod json;
pub mod markdown;
pub mod sarif;

use crate::model::Report;

/// Sort every collection in a report into its canonical order: files by
/// path; findings by line, column, rule id; changes and lock violations by
/// path then the rest.
pub fn canonicalize(r: &mut Report) {
    r.files.sort_by(|a, b| a.path.cmp(&b.path));
    for f in &mut r.files {
        f.findings.sort_by(|a, b| {
            (a.line, a.column, &a.rule_id, &a.fingerprint).cmp(&(
                b.line,
                b.column,
                &b.rule_id,
                &b.fingerprint,
            ))
        });
    }
    r.changes.sort();
    r.lock.violations.sort();
    r.warnings.sort();
    r.warnings.dedup();
}
