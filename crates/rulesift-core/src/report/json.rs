//! JSON report (`schema_version: 1`, PRD §10.1). The schema is committed at
//! `schemas/report.v1.json`.

use crate::model::Report;

/// The committed JSON Schema for [`to_json`] output.
pub const SCHEMA: &str = include_str!("../../../../schemas/report.v1.json");

/// Serialize a report as pretty JSON (canonical order, trailing newline).
pub fn to_json(report: &Report) -> String {
    let mut r = report.clone();
    super::canonicalize(&mut r);
    let mut s = serde_json::to_string_pretty(&r).expect("report serializes");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Change, ChangeOp, FileKind, LockStatus, LockViolation, Severity, Tier, Tool,
    };
    use crate::rules::loader::{load, LoadOptions};
    use crate::scan::{scan, ScanOptions};
    use std::path::PathBuf;

    fn validator() -> jsonschema::Validator {
        let schema: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
        jsonschema::draft202012::new(&schema).unwrap()
    }

    fn check(v: &serde_json::Value) {
        let val = validator();
        let errs: Vec<String> = val
            .iter_errors(v)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .collect();
        assert!(errs.is_empty(), "schema errors: {errs:#?}");
    }

    fn repo(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/repos")
            .join(name)
    }

    fn scan_repo(name: &str) -> Report {
        let rules = load(&LoadOptions {
            builtin: true,
            ..Default::default()
        })
        .unwrap();
        let mut out = scan(
            &rules,
            &ScanOptions {
                paths: vec![repo(name)],
                root: ".".into(),
                ..ScanOptions::default()
            },
        )
        .unwrap()
        .report;
        out.generated_at = Some("2026-10-29T00:00:00Z".into());
        // make paths relative to the fixture, as the CLI does when run there
        let prefix = format!("{}/", repo(name).to_string_lossy());
        for f in &mut out.files {
            f.path = f.path.trim_start_matches(&prefix).to_string();
            for x in &mut f.findings {
                x.path = x.path.trim_start_matches(&prefix).to_string();
            }
        }
        out
    }

    #[test]
    fn schema_enums_match_rust_enums() {
        let s: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
        let list = |k: &str| -> Vec<String> {
            s["$defs"][k]["enum"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect()
        };
        let tools: Vec<String> = Tool::ALL.iter().map(|t| t.to_string()).collect();
        assert_eq!(list("tool"), tools);
        let kinds: Vec<String> = FileKind::ALL.iter().map(|t| t.to_string()).collect();
        assert_eq!(list("kind"), kinds);
        let sev: Vec<String> = Severity::ALL.iter().map(|t| t.to_string()).collect();
        assert_eq!(list("severity"), sev);
        let tiers: Vec<String> = Tier::ALL.iter().map(|t| t.as_str().to_string()).collect();
        assert_eq!(list("tier"), tiers);
    }

    #[test]
    fn risky_and_all_tools_validate() {
        for name in ["risky", "all-tools"] {
            let r = scan_repo(name);
            let v: serde_json::Value = serde_json::from_str(&to_json(&r)).unwrap();
            check(&v);
            for f in v["files"].as_array().unwrap() {
                assert!(f["tool"].is_string() && f["kind"].is_string());
            }
        }
    }

    #[test]
    fn changes_and_lock_validate() {
        let mut r = scan_repo("risky");
        r.changes.push(Change {
            path: ".mcp.json".into(),
            op: ChangeOp::Added,
            what: "mcp_server".into(),
            name: Some("helper".into()),
            detail: "new MCP server `helper` runs `npx helper`".into(),
        });
        r.lock = LockStatus {
            present: true,
            violations: vec![LockViolation {
                rule_id: "RS-LOCK-001".into(),
                path: "AGENTS.md".into(),
                name: None,
                message: "AGENTS.md changed since it was locked.".into(),
            }],
        };
        let text = to_json(&r);
        check(&serde_json::from_str(&text).unwrap());
        assert!(text.contains("\"op\": \"+\""));
        let back: Report = serde_json::from_str(&text).unwrap();
        assert_eq!(back.changes.len(), 1);
    }

    #[test]
    fn schema_rejects_bad_report() {
        let r = scan_repo("risky");
        let mut v: serde_json::Value = serde_json::from_str(&to_json(&r)).unwrap();
        v["files"][0]["tier"] = "fatal".into();
        assert!(!validator().is_valid(&v));
        let mut v: serde_json::Value = serde_json::from_str(&to_json(&r)).unwrap();
        v["surprise"] = 1.into();
        assert!(!validator().is_valid(&v));
    }

    #[test]
    fn risky_snapshot() {
        let r = scan_repo("risky");
        let mut v: serde_json::Value = serde_json::from_str(&to_json(&r)).unwrap();
        v["generated_at"] = "[generated_at]".into();
        v["tool"]["version"] = "[version]".into();
        insta::assert_snapshot!(serde_json::to_string_pretty(&v).unwrap());
    }
}
