//! Stable fingerprints and `[[suppress]]` entries (PRD §7.1).

use crate::config::Suppression;
use crate::error::{CoreError, Result};
use crate::model::Finding;
use globset::{Glob, GlobMatcher};
use sha2::{Digest, Sha256};

/// Normalize evidence for fingerprinting: collapse whitespace and trim, so
/// re-indenting or moving a line keeps the fingerprint.
pub fn normalize_evidence(e: &str) -> String {
    e.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Fingerprint = `sha256:` + hex(sha256(rule id NUL path NUL evidence)).
/// The line number is deliberately not included.
pub fn fingerprint(rule_id: &str, path: &str, evidence: &str) -> String {
    let mut h = Sha256::new();
    h.update(rule_id.as_bytes());
    h.update([0]);
    h.update(path.replace('\\', "/").as_bytes());
    h.update([0]);
    h.update(normalize_evidence(evidence).as_bytes());
    let d = h.finalize();
    let mut s = String::with_capacity(71);
    s.push_str("sha256:");
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Fill in `fingerprint` for every finding.
pub fn fingerprint_all(findings: &mut [Finding]) {
    for f in findings {
        f.fingerprint = fingerprint(&f.rule_id, &f.path, &f.evidence);
    }
}

/// Compiled suppressions.
#[derive(Debug, Clone, Default)]
pub struct Suppressor {
    entries: Vec<(String, Option<GlobMatcher>)>,
}

impl Suppressor {
    /// Compile `[[suppress]]` entries.
    pub fn new(entries: &[Suppression]) -> Result<Self> {
        let mut out = Vec::new();
        for s in entries {
            let m = match &s.path {
                Some(p) => Some(
                    Glob::new(p)
                        .map_err(|e| CoreError::Glob {
                            pattern: p.clone(),
                            message: e.to_string(),
                        })?
                        .compile_matcher(),
                ),
                None => None,
            };
            out.push((s.rule.clone(), m));
        }
        Ok(Self { entries: out })
    }

    /// Does an entry suppress `rule_id` at `path`?
    pub fn matches(&self, rule_id: &str, path: &str) -> bool {
        self.entries
            .iter()
            .any(|(r, m)| (r == rule_id || r == "*") && m.as_ref().is_none_or(|g| g.is_match(path)))
    }

    /// Mark matching findings as suppressed.
    pub fn apply(&self, findings: &mut [Finding]) {
        for f in findings {
            if self.matches(&f.rule_id, &f.path) {
                f.suppressed = true;
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::Severity;

    pub(crate) fn finding(id: &str, path: &str, line: usize, ev: &str) -> Finding {
        Finding {
            rule_id: id.into(),
            severity: Severity::High,
            title: String::new(),
            message: String::new(),
            path: path.into(),
            line,
            column: 1,
            end_line: line,
            evidence: ev.into(),
            decoded: None,
            fingerprint: String::new(),
            suppressed: false,
        }
    }

    #[test]
    fn fingerprint_ignores_line_and_whitespace() {
        let a = fingerprint("RS-EXEC-001", "AGENTS.md", "curl x | sh");
        let b = fingerprint("RS-EXEC-001", "AGENTS.md", "  curl   x | sh ");
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:") && a.len() == 71);
        assert_ne!(a, fingerprint("RS-EXEC-002", "AGENTS.md", "curl x | sh"));
        assert_ne!(a, fingerprint("RS-EXEC-001", "CLAUDE.md", "curl x | sh"));
        let mut v = vec![
            finding("RS-EXEC-001", "AGENTS.md", 3, "curl x | sh"),
            finding("RS-EXEC-001", "AGENTS.md", 40, "curl x | sh"),
        ];
        fingerprint_all(&mut v);
        assert_eq!(v[0].fingerprint, v[1].fingerprint);
    }

    #[test]
    fn suppressions_by_rule_and_glob() {
        let s = Suppressor::new(&[
            Suppression {
                rule: "RS-EXEC-001".into(),
                path: Some("skills/installer/**".into()),
                reason: "vendored".into(),
            },
            Suppression {
                rule: "RS-EXF-003".into(),
                path: None,
                reason: "docs".into(),
            },
        ])
        .unwrap();
        let mut v = vec![
            finding("RS-EXEC-001", "skills/installer/SKILL.md", 1, "a"),
            finding("RS-EXEC-001", "AGENTS.md", 1, "a"),
            finding("RS-EXF-003", "any/where.md", 1, "a"),
        ];
        s.apply(&mut v);
        assert_eq!(
            v.iter().map(|f| f.suppressed).collect::<Vec<_>>(),
            vec![true, false, true]
        );
        assert!(Suppressor::new(&[Suppression {
            rule: "X".into(),
            path: Some("a/[".into()),
            reason: "r".into()
        }])
        .is_err());
    }
}
