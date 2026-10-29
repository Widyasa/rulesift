//! `docs/supported-tools.md` must match the registry, and the positive
//! compatibility fixture must produce a finding for every MVP row.

use rulesift_core::registry::{self, Release};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn supported_tools_doc_is_fresh() {
    let path = root().join("docs/supported-tools.md");
    let want = registry::render_markdown();
    if std::env::var_os("RULESIFT_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &want).unwrap();
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        have == want,
        "docs/supported-tools.md is stale; run `RULESIFT_BLESS=1 cargo test -p rulesift-core --test registry_docs`"
    );
    for r in registry::ROWS {
        assert!(have.contains(r.doc_url), "{} missing", r.id);
    }
}

#[test]
fn every_mvp_row_has_a_positive_finding() {
    let fixture = root().join("tests/fixtures/repos/all-tools-positive");
    let d = rulesift_core::discovery::discover(std::slice::from_ref(&fixture), &Default::default())
        .unwrap();
    let rules = rulesift_core::rules::loader::RuleSet {
        rules: rulesift_core::rules::loader::builtin().unwrap(),
        warnings: vec![],
    };
    let opts = rulesift_core::scan::ScanOptions {
        paths: vec![fixture],
        ..Default::default()
    };
    let out = rulesift_core::scan::scan(&rules, &opts).unwrap();
    let mut hit = BTreeSet::new();
    for f in &d.files {
        let r = out.report.files.iter().find(|r| r.path == f.path).unwrap();
        if !r.findings.is_empty() {
            hit.insert(f.row.id);
        }
    }
    let missing: Vec<_> = registry::ROWS
        .iter()
        .filter(|r| r.release == Release::Mvp && !r.global_only)
        .map(|r| r.id)
        .filter(|id| !hit.contains(id))
        .collect();
    assert!(
        missing.is_empty(),
        "MVP rows without a finding: {missing:?}"
    );
}
