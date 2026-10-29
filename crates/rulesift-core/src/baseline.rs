//! Baselines: `--write-baseline <FILE>` stores current fingerprints (sorted
//! JSON); `--baseline <FILE>` suppresses findings whose fingerprint is listed.

use crate::error::{CoreError, Result};
use crate::model::Finding;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// One baseline entry (rule and path kept for readability in reviews).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Fingerprint (`sha256:…`).
    pub fingerprint: String,
    /// Rule id.
    pub rule_id: String,
    /// Path.
    pub path: String,
}

/// Baseline file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    /// Format version (1).
    pub version: u32,
    /// Entries sorted by fingerprint, unique.
    pub findings: Vec<Entry>,
}

impl Baseline {
    /// Build from findings (fingerprints must be filled in).
    pub fn from_findings<'a>(findings: impl IntoIterator<Item = &'a Finding>) -> Self {
        let set: BTreeSet<Entry> = findings
            .into_iter()
            .map(|f| Entry {
                fingerprint: f.fingerprint.clone(),
                rule_id: f.rule_id.clone(),
                path: f.path.clone(),
            })
            .collect();
        let mut findings: Vec<Entry> = set.into_iter().collect();
        findings.dedup_by(|a, b| a.fingerprint == b.fingerprint);
        Self {
            version: 1,
            findings,
        }
    }

    /// Serialize (pretty JSON, trailing newline).
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).unwrap_or_default();
        s.push('\n');
        s
    }

    /// Parse.
    pub fn parse(text: &str, path: &Path) -> Result<Self> {
        let b: Baseline = serde_json::from_str(text)
            .map_err(|e| CoreError::Invalid(format!("invalid baseline {}: {e}", path.display())))?;
        if b.version != 1 {
            return Err(CoreError::Invalid(format!(
                "baseline {} has unsupported version {}",
                path.display(),
                b.version
            )));
        }
        Ok(b)
    }

    /// Read a baseline file.
    pub fn load(path: &Path) -> Result<Self> {
        let t = std::fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
        Self::parse(&t, path)
    }

    /// Write a baseline file.
    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.to_json()).map_err(|e| CoreError::io(path, e))
    }

    /// Mark findings whose fingerprint is in the baseline as suppressed.
    pub fn apply(&self, findings: &mut [Finding]) {
        let set: BTreeSet<&str> = self
            .findings
            .iter()
            .map(|e| e.fingerprint.as_str())
            .collect();
        for f in findings {
            if set.contains(f.fingerprint.as_str()) {
                f.suppressed = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suppress::fingerprint_all;
    use crate::suppress::tests::finding;

    #[test]
    fn roundtrip_sorted_and_applies_after_move() {
        let mut v = vec![
            finding("RS-SEC-002", "b.md", 9, "cat ~/.ssh/id_rsa"),
            finding("RS-EXEC-001", "a.md", 3, "curl x | sh"),
            finding("RS-EXEC-001", "a.md", 4, "curl x | sh"),
        ];
        fingerprint_all(&mut v);
        let b = Baseline::from_findings(&v);
        assert_eq!(b.findings.len(), 2);
        let mut sorted = b.findings.clone();
        sorted.sort();
        assert_eq!(b.findings, sorted);
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("baseline.json");
        b.write(&p).unwrap();
        let back = Baseline::load(&p).unwrap();
        assert_eq!(back, b);
        // the same finding moved to another line is still baselined
        let mut moved = vec![
            finding("RS-EXEC-001", "a.md", 120, "curl   x | sh"),
            finding("RS-EXEC-002", "a.md", 1, "new"),
        ];
        fingerprint_all(&mut moved);
        back.apply(&mut moved);
        assert!(moved[0].suppressed);
        assert!(!moved[1].suppressed);
        assert!(Baseline::parse("{\"version\": 2, \"findings\": []}", &p).is_err());
        assert!(Baseline::parse("{\"bogus\": 1}", &p).is_err());
    }
}
