//! Runs every rule fixture under `tests/fixtures/rules/<ID>/{match,no-match}/`.
//!
//! The path below `match/` or `no-match/` is the path the file is scanned
//! as (so `match/.kiro/steering/a.md` is a Kiro steering file). Files ending
//! in `.escaped` have `\u{XXXX}` escapes decoded first, so fixtures with
//! invisible characters stay reviewable.

use crate::analyze::analyze;
use crate::registry::{match_path, Origin};
use crate::rules::engine::run_rules;
use crate::rules::loader::{load, LoadOptions};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/rules")
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut v: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    v.sort();
    for p in v {
        if p.is_dir() {
            files(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// Decode `\u{XXXX}` escapes.
pub(crate) fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("\\u{") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 3..];
        match after.find('}').and_then(|j| {
            u32::from_str_radix(&after[..j], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| (c, j))
        }) {
            Some((c, j)) => {
                out.push(c);
                rest = &after[j + 1..];
            }
            None => {
                out.push_str("\\u{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[test]
fn every_rule_fixture() {
    let set = load(&LoadOptions {
        builtin: true,
        ..Default::default()
    })
    .unwrap();
    let mut failures = Vec::new();
    for rule in &set.rules {
        let dir = root().join(rule.id());
        for (sub, want) in [("match", true), ("no-match", false)] {
            let base = dir.join(sub);
            let mut list = Vec::new();
            files(&base, &mut list);
            if list.is_empty() {
                failures.push(format!("{}: no {sub} fixture", rule.id()));
            }
            for f in list {
                let rel = f
                    .strip_prefix(&base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let raw = std::fs::read_to_string(&f).unwrap();
                let (rel, text) = match rel.strip_suffix(".escaped") {
                    Some(r) => (r.to_string(), unescape(&raw)),
                    None => (rel, raw),
                };
                let Some(row) = match_path(&rel, Origin::Explicit) else {
                    failures.push(format!("{}: {rel} is not an agent file", rule.id()));
                    continue;
                };
                let skill_dir = rel
                    .strip_suffix("/SKILL.md")
                    .map(|d| d.rsplit('/').next().unwrap_or(d).to_string());
                let a = analyze(&rel, &rel, row, skill_dir.as_deref(), &text);
                let got = run_rules([rule], &a);
                if got.is_empty() == want {
                    failures.push(format!(
                        "{} {sub}/{rel}: expected {}, got {:?}",
                        rule.id(),
                        if want { "a finding" } else { "none" },
                        got.iter()
                            .map(|x| (x.line, &x.evidence))
                            .collect::<Vec<_>>()
                    ));
                }
            }
        }
    }
    // every fixture directory belongs to a loaded rule
    for e in std::fs::read_dir(root()).unwrap().filter_map(|e| e.ok()) {
        let id = e.file_name().to_string_lossy().into_owned();
        if set.get(&id).is_none() {
            failures.push(format!("fixture dir {id} has no loaded rule"));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn unescape_works() {
    assert_eq!(unescape("a\\u{200B}b\\u{zz}"), "a\u{200B}b\\u{zz}");
}
