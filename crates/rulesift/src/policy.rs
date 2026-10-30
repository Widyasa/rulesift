//! Which policy gates a change in diff mode (#160).
//!
//! In a pull request the head revision is written by its author, so the
//! policy that gates it (`rulesift.toml`, `rulesift.lock`, custom rule
//! directories) comes from the **base** revision. Changes to those inputs
//! are reported as lock violations: RS-LOCK-004 (fails) when they weaken
//! the policy, RS-LOCK-005 (review only) otherwise.

use crate::cli::GlobalArgs;
use crate::scan::{prepare_from, prepare_with, OutputArgs, Prepared};
use anyhow::Context;
use rulesift_core::config::{weakenings, Config, CONFIG_FILE_NAME};
use rulesift_core::discovery::DiscoveryOptions;
use rulesift_core::git::{ChangeKind, GitRepo};
use rulesift_core::lock::{self, Lockfile, LOCK_FILE};
use rulesift_core::model::{LockStatus, LockViolation, Severity};
use std::path::{Path, PathBuf};

/// RS-LOCK-005 is a review note: it does not fail the run.
pub const REVIEW_ONLY: &str = "RS-LOCK-005";

/// Prepared options plus the lock state for a diff.
pub struct DiffPolicy {
    /// Options and rules from the base revision's config.
    pub prepared: Prepared,
    /// Lock violations and policy changes, if any were checked.
    pub lock: Option<LockStatus>,
}

/// Whether the operator chose the config (`--config`, `RULESIFT_CONFIG`;
/// an empty `RULESIFT_CONFIG` chooses the defaults).
fn explicit(g: &GlobalArgs) -> bool {
    g.config.is_some() || std::env::var_os(rulesift_core::config::CONFIG_ENV).is_some()
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// `a/b` → `["a/b", "a", ""]`.
fn chain(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = dir.trim_matches('/').to_string();
    loop {
        out.push(cur.clone());
        if cur.is_empty() {
            break;
        }
        cur = cur
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_default();
    }
    out
}

/// Normalize `a/./b/../c` lexically.
fn normalize(p: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in p.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

fn violation(rule: &str, path: &str, message: String) -> LockViolation {
    LockViolation {
        rule_id: rule.into(),
        path: path.into(),
        name: None,
        message,
    }
}

fn parse_cfg(bytes: &[u8], path: &str) -> anyhow::Result<Config> {
    let text = String::from_utf8_lossy(bytes);
    Ok(Config::parse(&text, Path::new(path))?)
}

/// A changed policy input: path, base bytes, head bytes.
type Entry = (String, Option<Vec<u8>>, Option<Vec<u8>>);

/// Resolve the policy for `rulesift diff` / MCP `diff_refs`: `dir` is the
/// scanned directory (`dir_rel` relative to the work tree).
pub fn prepare_diff(
    g: &GlobalArgs,
    repo: &GitRepo,
    dir: &Path,
    dir_rel: &str,
    base: &str,
    head: Option<&str>,
    o: &OutputArgs,
) -> anyhow::Result<DiffPolicy> {
    let workdir = repo.workdir().to_path_buf();
    // 1. the base revision's config (unless the operator chose one)
    let mut cfg_rel_dir: Option<String> = None;
    let (prepared, cfg) = if explicit(g) {
        let p = prepare_from(g, dir, std::slice::from_ref(&dir.to_path_buf()), o)?;
        let (cfg, _) = crate::common::load_config(g, dir)?;
        (p, cfg)
    } else {
        let mut found = None;
        for d in chain(dir_rel) {
            let path = join(&d, CONFIG_FILE_NAME);
            if let Some(bytes) = repo.read_at(base, &path)? {
                let cfg = parse_cfg(&bytes, &path)
                    .with_context(|| format!("invalid {path} in the base revision {base}"))?;
                found = Some((d, cfg));
                break;
            }
        }
        let (cfg, cfg_base, cfg_dir) = match found {
            Some((d, cfg)) => {
                let b = workdir.join(&d);
                let canon = std::fs::canonicalize(&b).ok();
                cfg_rel_dir = Some(d);
                (cfg, b, canon)
            }
            None => (Config::default(), PathBuf::new(), None),
        };
        let p = prepare_with(
            g,
            cfg.clone(),
            cfg_base,
            cfg_dir,
            std::slice::from_ref(&dir.to_path_buf()),
            o,
        )?;
        (p, cfg)
    };

    // 2. changes to policy inputs in this change
    let builtin = rulesift_core::rules::loader::builtin().unwrap_or_default();
    let severity_of = |id: &str| -> Option<Severity> {
        builtin.iter().find(|r| r.def.id == id).map(|r| r.severity)
    };
    let extra: Vec<String> = cfg_rel_dir
        .as_ref()
        .map(|d| {
            cfg.rules
                .extra_dirs
                .iter()
                .map(|x| normalize(&join(d, x)))
                .collect()
        })
        .unwrap_or_default();
    let in_extra = |p: &str| {
        extra
            .iter()
            .any(|x| x.is_empty() || p == x || p.starts_with(&format!("{x}/")))
    };
    let name_of = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    let keep = |p: &str| {
        let n = name_of(p);
        n == CONFIG_FILE_NAME || n == LOCK_FILE || in_extra(p)
    };
    let mut entries: Vec<Entry> = Vec::new();
    for c in repo.changed(base, head, &keep)? {
        match c.kind {
            ChangeKind::Renamed { from } => {
                entries.push((from, c.old.clone(), None));
                entries.push((c.path, None, c.new));
            }
            _ => entries.push((c.path, c.old, c.new)),
        }
    }
    let mut violations: Vec<LockViolation> = Vec::new();
    let mut root_lock_changed = false;
    for (path, old, new) in &entries {
        let n = name_of(path);
        if n == CONFIG_FILE_NAME {
            let old_cfg = old
                .as_ref()
                .and_then(|b| parse_cfg(b, path).ok())
                .unwrap_or_default();
            let new_cfg = match new {
                None => Config::default(),
                Some(b) => match parse_cfg(b, path) {
                    Ok(c) => c,
                    Err(e) => {
                        violations.push(violation(
                            REVIEW_ONLY,
                            path,
                            format!("{path} is changed and cannot be parsed ({e}); the base revision's policy was used"),
                        ));
                        continue;
                    }
                },
            };
            let weak = weakenings(&old_cfg, &new_cfg, &severity_of);
            let verb = match (old, new) {
                (None, _) => "added",
                (_, None) => "deleted",
                _ => "changed",
            };
            if weak.is_empty() {
                violations.push(violation(
                    REVIEW_ONLY,
                    path,
                    format!("{path} is {verb} without weakening the RuleSift policy; the base revision's policy was used"),
                ));
            } else {
                violations.push(violation(
                    "RS-LOCK-004",
                    path,
                    format!(
                        "{path} is {verb} and weakens the RuleSift policy: {}; the base revision's policy was used",
                        weak.join("; ")
                    ),
                ));
            }
        } else if path == LOCK_FILE && head.is_none() {
            root_lock_changed = true;
        } else if n == LOCK_FILE {
            violations.push(violation(
                REVIEW_ONLY,
                path,
                format!("{path} is changed; lock approvals are reviewed with the base revision's lock in working-tree mode"),
            ));
        } else if old.is_none() {
            violations.push(violation(
                REVIEW_ONLY,
                path,
                format!("custom rule file {path} is added"),
            ));
        } else {
            violations.push(violation(
                "RS-LOCK-004",
                path,
                format!(
                    "custom rule file {path} is {} (custom rules can be weakened this way); review it",
                    if new.is_none() { "deleted" } else { "changed" }
                ),
            ));
        }
    }

    // 3. the lock: the base revision's lock approves; a lock changed in
    //    this change only adds review notes
    let lock_status = if head.is_none() {
        let base_lock: Option<Lockfile> = match repo.read_at(base, LOCK_FILE)? {
            Some(b) => Some(
                lock::parse(&String::from_utf8_lossy(&b), Path::new(LOCK_FILE))
                    .with_context(|| format!("invalid {LOCK_FILE} in the base revision {base}"))?,
            ),
            None => None,
        };
        let head_lock = lock::read(&workdir.join(LOCK_FILE))?;
        let opts = DiscoveryOptions::from_config(&cfg.scan);
        let current = || crate::lock::current_with(&opts, &workdir);
        match (&base_lock, &head_lock) {
            (None, None) => LockStatus::default(),
            (None, Some(h)) => {
                let mut v = lock::verify(h, &current()?);
                if root_lock_changed {
                    v.push(violation(
                        REVIEW_ONLY,
                        LOCK_FILE,
                        format!("{LOCK_FILE} is added in this change"),
                    ));
                }
                LockStatus {
                    present: true,
                    violations: v,
                }
            }
            (Some(b), h) => {
                let cur = current()?;
                let base_v = lock::verify(b, &cur);
                let mut v: Vec<LockViolation> = Vec::new();
                match h {
                    None => {
                        v.extend(base_v);
                        v.push(violation(
                            "RS-LOCK-004",
                            LOCK_FILE,
                            format!("{LOCK_FILE} is deleted in this change; the base revision's lock was used"),
                        ));
                    }
                    Some(h) if h == b => v.extend(base_v),
                    Some(h) => {
                        let head_v = lock::verify(h, &cur);
                        let (fail, approved): (Vec<_>, Vec<_>) =
                            base_v.into_iter().partition(|x| head_v.contains(x));
                        v.extend(fail);
                        if !approved.is_empty() {
                            let list: Vec<String> = approved
                                .iter()
                                .map(|x| match &x.name {
                                    Some(n) => format!("{}#{n}", x.path),
                                    None => x.path.clone(),
                                })
                                .collect();
                            v.push(violation(
                                REVIEW_ONLY,
                                LOCK_FILE,
                                format!(
                                    "{LOCK_FILE} approves {} change(s) made in this change ({}); review them",
                                    list.len(),
                                    list.join(", ")
                                ),
                            ));
                        }
                        let dropped = dropped_entries(b, h, &cur);
                        if !dropped.is_empty() {
                            v.push(violation(
                                "RS-LOCK-004",
                                LOCK_FILE,
                                format!(
                                    "{LOCK_FILE} drops the entries for {}, which still exist; they would no longer be checked",
                                    dropped.join(", ")
                                ),
                            ));
                        } else if approved.is_empty() {
                            v.push(violation(
                                REVIEW_ONLY,
                                LOCK_FILE,
                                format!("{LOCK_FILE} is changed in this change"),
                            ));
                        }
                    }
                }
                LockStatus {
                    present: true,
                    violations: v,
                }
            }
        }
    } else {
        LockStatus {
            present: repo.read_at(head.unwrap_or(base), LOCK_FILE)?.is_some(),
            violations: Vec::new(),
        }
    };
    let mut lock_status = lock_status;
    lock_status.violations.extend(violations);
    lock_status.violations.sort();
    lock_status.violations.dedup();
    let lock = if head.is_none() || !lock_status.violations.is_empty() {
        Some(lock_status)
    } else {
        None
    };
    Ok(DiffPolicy { prepared, lock })
}

/// Entries of `base` missing from `head` whose subject still exists.
fn dropped_entries(base: &Lockfile, head: &Lockfile, current: &Lockfile) -> Vec<String> {
    let mut out = Vec::new();
    for e in &base.file {
        if !head.file.iter().any(|x| x.path == e.path)
            && current.file.iter().any(|x| x.path == e.path)
        {
            out.push(e.path.clone());
        }
    }
    for e in &base.skill {
        if !head.skill.iter().any(|x| x.path == e.path)
            && current.skill.iter().any(|x| x.path == e.path)
        {
            out.push(e.path.clone());
        }
    }
    for e in &base.mcp_server {
        let same = |x: &lock::McpEntry| x.source == e.source && x.name == e.name;
        if !head.mcp_server.iter().any(same) && current.mcp_server.iter().any(same) {
            out.push(format!("{}#{}", e.source, e.name));
        }
    }
    out
}

/// Whether lock violations fail the run (RS-LOCK-005 is review only).
pub fn lock_fails(lock: Option<&LockStatus>) -> bool {
    lock.is_some_and(|l| l.violations.iter().any(|v| v.rule_id != REVIEW_ONLY))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_walks_up() {
        assert_eq!(chain("a/b"), vec!["a/b", "a", ""]);
        assert_eq!(chain(""), vec![""]);
        assert_eq!(normalize("a/./b/../c"), "a/c");
    }
}
