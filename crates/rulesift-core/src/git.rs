//! Git access via `gix`: changed files between two revisions (or a revision
//! and the working tree) and file contents at each side.

use crate::discovery::{Listing, Reader};
use crate::error::{CoreError, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// How a file changed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    /// Only in head.
    Added,
    /// In both, different content.
    Modified,
    /// Only in base.
    Deleted,
    /// Identical content moved from `from` (only exact renames are
    /// detected; a moved and edited file is a deletion plus an addition).
    Renamed {
        /// Old path.
        from: String,
    },
}

/// One changed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    /// Repo-relative path (the new path; the old path for deletions).
    pub path: String,
    /// Kind of change.
    pub kind: ChangeKind,
    /// Content at base (none when added).
    pub old: Option<Vec<u8>>,
    /// Content at head (none when deleted).
    pub new: Option<Vec<u8>>,
}

/// An opened repository.
pub struct GitRepo {
    repo: gix::Repository,
    workdir: PathBuf,
}

fn git_err(e: impl std::fmt::Display) -> CoreError {
    CoreError::Git(e.to_string())
}

impl GitRepo {
    /// Open the repository containing `path`.
    pub fn discover(path: &Path) -> Result<Self> {
        let repo = gix::discover(path).map_err(git_err)?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| CoreError::Git("bare repositories are not supported".into()))?
            .to_path_buf();
        Ok(Self { repo, workdir })
    }

    /// Working tree root.
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Blob ids of every file in the tree of `rev`, by repo-relative path.
    pub fn tree_files(&self, rev: &str) -> Result<BTreeMap<String, gix::ObjectId>> {
        let tree = self
            .repo
            .rev_parse_single(rev)
            .map_err(|e| CoreError::Git(format!("cannot resolve {rev:?}: {e}")))?
            .object()
            .map_err(git_err)?
            .peel_to_tree()
            .map_err(git_err)?;
        let mut rec = gix::traverse::tree::Recorder::default();
        tree.traverse().breadthfirst(&mut rec).map_err(git_err)?;
        Ok(rec
            .records
            .into_iter()
            .filter(|e| e.mode.is_blob())
            .map(|e| (e.filepath.to_string(), e.oid))
            .collect())
    }

    /// Read a blob.
    pub fn blob(&self, id: gix::ObjectId) -> Result<Vec<u8>> {
        Ok(self.repo.find_object(id).map_err(git_err)?.detach().data)
    }

    /// Content of `path` at `rev`, if it exists there.
    pub fn read_at(&self, rev: &str, path: &str) -> Result<Option<Vec<u8>>> {
        match self.tree_files(rev)?.get(path) {
            Some(id) => Ok(Some(self.blob(*id)?)),
            None => Ok(None),
        }
    }

    /// The tree of `rev` as a discovery [`Listing`] plus a [`Reader`] for
    /// its blobs, so diff mode classifies a revision exactly like `scan`
    /// classifies a checkout (#132/#133). Symlinks (mode 120000) are
    /// resolved by their link text: inside the repository they stand for
    /// their target file, outside it they are reported as escaping.
    pub fn listing(&self, rev: &str) -> Result<(Listing, GitTree<'_>)> {
        let tree = self
            .repo
            .rev_parse_single(rev)
            .map_err(|e| CoreError::Git(format!("cannot resolve {rev:?}: {e}")))?
            .object()
            .map_err(git_err)?
            .peel_to_tree()
            .map_err(git_err)?;
        let mut rec = gix::traverse::tree::Recorder::default();
        tree.traverse().breadthfirst(&mut rec).map_err(git_err)?;
        let mut blobs: BTreeMap<String, gix::ObjectId> = BTreeMap::new();
        let mut links: BTreeMap<String, String> = BTreeMap::new();
        let mut dirs: BTreeSet<String> = BTreeSet::new();
        let mut submodules: Vec<String> = Vec::new();
        for e in rec.records {
            let p = e.filepath.to_string();
            if e.mode.is_commit() {
                submodules.push(p);
            } else if e.mode.is_tree() {
                dirs.insert(p);
            } else if e.mode.is_link() {
                let target = self.blob(e.oid)?;
                links.insert(p, String::from_utf8_lossy(&target).into_owned());
            } else if e.mode.is_blob() {
                blobs.insert(p, e.oid);
            }
        }
        let root = self.workdir.clone();
        let canon = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        let mut listing = Listing::default();
        for p in submodules {
            dirs.insert(p.clone());
            listing.submodules.push((p.clone(), root.join(&p)));
        }
        let mut map = blobs.clone();
        for p in blobs.keys() {
            listing.files.insert(p.clone(), root.join(p));
        }
        for (p, target) in &links {
            let mut cur = (p.clone(), target.clone());
            let mut resolved = None;
            for _ in 0..40 {
                match resolve_link(&cur.0, &cur.1, &canon) {
                    Link::Inside(t) => {
                        if let Some(id) = blobs.get(&t) {
                            resolved = Some(Ok(*id));
                        } else if let Some(next) = links.get(&t) {
                            cur = (t, next.clone());
                            continue;
                        } else if dirs.contains(&t) {
                            // a directory inside the root is scanned where it is
                            resolved = Some(Err(true));
                        }
                    }
                    Link::Outside(abs) => {
                        let is_dir = std::fs::metadata(&abs).ok().map(|m| m.is_dir());
                        listing.escaped.push((p.clone(), root.join(p), is_dir));
                        resolved = Some(Err(true));
                    }
                }
                break;
            }
            match resolved {
                Some(Ok(id)) => {
                    map.insert(p.clone(), id);
                    listing.files.insert(p.clone(), root.join(p));
                }
                Some(Err(_)) => {}
                None => listing.broken.push(p.clone()),
            }
        }
        let mut all_dirs = dirs;
        for p in map.keys() {
            let mut cur = p.as_str();
            while let Some((d, _)) = cur.rsplit_once('/') {
                all_dirs.insert(d.to_string());
                cur = d;
            }
        }
        Ok((
            listing,
            GitTree {
                repo: self,
                root,
                blobs: map,
                dirs: all_dirs,
            },
        ))
    }

    fn worktree_files(&self, keep: &dyn Fn(&str) -> bool) -> BTreeMap<String, PathBuf> {
        let mut out = BTreeMap::new();
        let walker = ignore::WalkBuilder::new(&self.workdir)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .git_global(false)
            .require_git(false)
            .parents(false)
            .follow_links(false)
            .filter_entry(|e| e.file_name() != ".git")
            .build();
        for e in walker.flatten() {
            if !e.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let Ok(rel) = e.path().strip_prefix(&self.workdir) else {
                continue;
            };
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if keep(&rel) {
                out.insert(rel, e.path().to_path_buf());
            }
        }
        out
    }

    /// Files changed between `base` and `head` (`None` = working tree),
    /// limited to paths for which `keep` returns true. Exact renames
    /// (identical content) are reported as [`ChangeKind::Renamed`].
    pub fn changed(
        &self,
        base: &str,
        head: Option<&str>,
        keep: &dyn Fn(&str) -> bool,
    ) -> Result<Vec<ChangedFile>> {
        let old: BTreeMap<String, gix::ObjectId> = self
            .tree_files(base)?
            .into_iter()
            .filter(|(p, _)| keep(p))
            .collect();
        let mut old_bytes: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (p, id) in &old {
            old_bytes.insert(p.clone(), self.blob(*id)?);
        }
        let mut new_bytes: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        match head {
            Some(rev) => {
                for (p, id) in self.tree_files(rev)? {
                    if keep(&p) {
                        new_bytes.insert(p, self.blob(id)?);
                    }
                }
            }
            None => {
                let mut files = self.worktree_files(keep);
                // tracked files that are ignored still count
                for p in old.keys() {
                    let fs = self.workdir.join(p);
                    if fs.is_file() {
                        files.entry(p.clone()).or_insert(fs);
                    }
                }
                for (p, fs) in files {
                    let b = std::fs::read(&fs).map_err(|e| CoreError::io(&fs, e))?;
                    new_bytes.insert(p, b);
                }
            }
        }
        let mut added = Vec::new();
        let mut deleted = Vec::new();
        let mut out = Vec::new();
        for (p, nb) in &new_bytes {
            match old_bytes.get(p) {
                Some(ob) if ob == nb => {}
                Some(ob) => out.push(ChangedFile {
                    path: p.clone(),
                    kind: ChangeKind::Modified,
                    old: Some(ob.clone()),
                    new: Some(nb.clone()),
                }),
                None => added.push(p.clone()),
            }
        }
        for p in old_bytes.keys() {
            if !new_bytes.contains_key(p) {
                deleted.push(p.clone());
            }
        }
        for a in added {
            let nb = new_bytes[&a].clone();
            if let Some(i) = deleted.iter().position(|d| old_bytes[d] == nb) {
                let from = deleted.remove(i);
                out.push(ChangedFile {
                    path: a,
                    kind: ChangeKind::Renamed { from },
                    old: Some(nb.clone()),
                    new: Some(nb),
                });
            } else {
                out.push(ChangedFile {
                    path: a,
                    kind: ChangeKind::Added,
                    old: None,
                    new: Some(nb),
                });
            }
        }
        for d in deleted {
            out.push(ChangedFile {
                old: old_bytes.get(&d).cloned(),
                path: d,
                kind: ChangeKind::Deleted,
                new: None,
            });
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }
}

/// Paths tracked in the index of the git work tree that contains `dir`:
/// (canonical work tree, repo-relative paths with `true` for submodules).
/// `None` outside a git work tree or if the index cannot be read.
pub fn tracked(dir: &Path) -> Option<(PathBuf, Vec<(String, bool)>)> {
    let repo = gix::discover(dir).ok()?;
    let wd = repo.workdir()?.to_path_buf();
    let wd = std::fs::canonicalize(&wd).unwrap_or(wd);
    let index = repo.index_or_empty().ok()?;
    let out = index
        .entries()
        .iter()
        .filter(|e| e.mode != gix::index::entry::Mode::DIR)
        .map(|e| {
            (
                e.path(&index).to_string(),
                e.mode == gix::index::entry::Mode::COMMIT,
            )
        })
        .collect();
    Some((wd, out))
}

/// Where a symlink in a git tree points.
enum Link {
    /// A repo-relative path inside the repository.
    Inside(String),
    /// An absolute path outside it.
    Outside(PathBuf),
}

/// Resolve the link text `target` of the symlink at repo-relative `link`
/// lexically (`..` pops a directory). `canon_root` is the work tree.
fn resolve_link(link: &str, target: &str, canon_root: &Path) -> Link {
    let t = Path::new(target);
    if t.is_absolute() {
        return match t.strip_prefix(canon_root) {
            Ok(r) => Link::Inside(
                r.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/"),
            ),
            Err(_) => Link::Outside(t.to_path_buf()),
        };
    }
    let mut parts: Vec<String> = link.split('/').map(str::to_string).collect();
    parts.pop();
    let mut escaped_by = 0usize;
    for c in target.split(['/', '\\']) {
        match c {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    escaped_by += 1;
                }
            }
            other => parts.push(other.to_string()),
        }
    }
    if escaped_by > 0 {
        let mut abs = canon_root.to_path_buf();
        for _ in 0..escaped_by {
            abs.pop();
        }
        for p in &parts {
            abs.push(p);
        }
        Link::Outside(abs)
    } else {
        Link::Inside(parts.join("/"))
    }
}

/// [`Reader`] over the blobs of a git revision (see [`GitRepo::listing`]).
pub struct GitTree<'a> {
    repo: &'a GitRepo,
    root: PathBuf,
    blobs: BTreeMap<String, gix::ObjectId>,
    dirs: BTreeSet<String>,
}

impl GitTree<'_> {
    fn rel(&self, path: &Path) -> Option<String> {
        let r = path.strip_prefix(&self.root).ok()?;
        Some(
            r.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/"),
        )
    }
}

impl Reader for GitTree<'_> {
    fn len(&self, path: &Path) -> std::io::Result<u64> {
        self.read(path).map(|b| b.len() as u64)
    }
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let id = self
            .rel(path)
            .and_then(|r| self.blobs.get(&r).copied())
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
        self.repo
            .blob(id)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
    fn exists(&self, path: &Path) -> bool {
        self.rel(path)
            .is_some_and(|r| r.is_empty() || self.blobs.contains_key(&r) || self.dirs.contains(&r))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::process::Command;

    /// Run git in `dir` with a fixed identity; panics on failure.
    pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
        let o = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("git");
        assert!(
            o.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }

    pub(crate) fn commit_all(dir: &Path, msg: &str) -> String {
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "--allow-empty", "-m", msg]);
        git(dir, &["rev-parse", "HEAD"])
    }

    fn setup() -> (tempfile::TempDir, String) {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        git(r, &["init", "-q"]);
        std::fs::write(r.join("AGENTS.md"), "a\n").unwrap();
        std::fs::write(r.join("CLAUDE.md"), "c\n").unwrap();
        std::fs::write(r.join("GEMINI.md"), "g\n").unwrap();
        std::fs::create_dir_all(r.join(".cursor/rules")).unwrap();
        std::fs::write(r.join(".cursor/rules/old.mdc"), "rule body\n").unwrap();
        std::fs::write(r.join("README.md"), "readme\n").unwrap();
        let base = commit_all(r, "base");
        (t, base)
    }

    fn summary(v: &[ChangedFile]) -> Vec<String> {
        v.iter()
            .map(|c| match &c.kind {
                ChangeKind::Renamed { from } => format!("R {from} -> {}", c.path),
                k => format!(
                    "{} {}",
                    match k {
                        ChangeKind::Added => "A",
                        ChangeKind::Modified => "M",
                        ChangeKind::Deleted => "D",
                        _ => "?",
                    },
                    c.path
                ),
            })
            .collect()
    }

    fn agent(p: &str) -> bool {
        p != "README.md"
    }

    fn mutate(r: &Path) {
        std::fs::write(r.join("AGENTS.md"), "a changed\n").unwrap();
        std::fs::remove_file(r.join("CLAUDE.md")).unwrap();
        std::fs::write(r.join(".windsurfrules"), "new\n").unwrap();
        std::fs::rename(
            r.join(".cursor/rules/old.mdc"),
            r.join(".cursor/rules/new.mdc"),
        )
        .unwrap();
        std::fs::write(r.join("README.md"), "readme changed\n").unwrap();
    }

    const WANT: [&str; 4] = [
        "R .cursor/rules/old.mdc -> .cursor/rules/new.mdc",
        "A .windsurfrules",
        "M AGENTS.md",
        "D CLAUDE.md",
    ];

    #[test]
    fn symlink_targets_resolve_lexically() {
        let root = Path::new("/r/repo");
        let inside = |l: &str, t: &str| match resolve_link(l, t, root) {
            Link::Inside(p) => Some(p),
            Link::Outside(_) => None,
        };
        assert_eq!(inside("a/b/l", "../c/f").as_deref(), Some("a/c/f"));
        assert_eq!(inside("l", "./x/../y").as_deref(), Some("y"));
        assert_eq!(inside("l", "/r/repo/z").as_deref(), Some("z"));
        assert!(inside("a/l", "../../ext").is_none());
        assert!(inside("l", "/etc/passwd").is_none());
        match resolve_link("a/l", "../../ext/x", root) {
            Link::Outside(p) => assert_eq!(p, Path::new("/r/ext/x")),
            Link::Inside(p) => panic!("{p}"),
        }
    }

    #[test]
    fn working_tree_changes() {
        let (t, base) = setup();
        mutate(t.path());
        let g = GitRepo::discover(t.path()).unwrap();
        let ch = g.changed(&base, None, &agent).unwrap();
        assert_eq!(summary(&ch), WANT);
        let m = ch.iter().find(|c| c.path == "AGENTS.md").unwrap();
        assert_eq!(m.old.as_deref(), Some(&b"a\n"[..]));
        assert_eq!(m.new.as_deref(), Some(&b"a changed\n"[..]));
        let d = ch.iter().find(|c| c.path == "CLAUDE.md").unwrap();
        assert!(d.new.is_none() && d.old.is_some());
    }

    #[test]
    fn commit_to_commit_changes() {
        let (t, base) = setup();
        mutate(t.path());
        let head = commit_all(t.path(), "head");
        // later working tree edits must not leak into a rev-to-rev diff
        std::fs::write(t.path().join("GEMINI.md"), "dirty\n").unwrap();
        let g = GitRepo::discover(t.path()).unwrap();
        assert_eq!(
            summary(&g.changed(&base, Some(&head), &agent).unwrap()),
            WANT
        );
        assert_eq!(
            summary(&g.changed("HEAD~1", Some("HEAD"), &agent).unwrap()),
            WANT
        );
        assert_eq!(
            g.read_at(&base, "CLAUDE.md").unwrap().as_deref(),
            Some(&b"c\n"[..])
        );
        assert_eq!(g.read_at(&head, "CLAUDE.md").unwrap(), None);
    }

    #[test]
    fn no_changes_and_bad_rev() {
        let (t, base) = setup();
        let g = GitRepo::discover(t.path()).unwrap();
        assert!(g.changed(&base, None, &agent).unwrap().is_empty());
        let e = g.changed("no-such-ref", None, &agent).unwrap_err();
        assert!(e.to_string().contains("no-such-ref"));
        let nd = tempfile::tempdir().unwrap();
        assert!(GitRepo::discover(nd.path()).is_err());
    }
}
