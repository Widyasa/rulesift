//! Discovery and classification (PRD §5 F1).
//!
//! Walks the given paths with the `ignore` crate (respecting `.gitignore`,
//! including hidden directories), classifies files through the tool registry,
//! groups skill directories, and skips binaries, oversize files, and symlinks
//! that escape the root, recording an English warning for each skip.

use crate::model::{FileKind, Tool};
use crate::registry::{self, AlwaysLoaded, Origin, ParserFamily, RegistryRow, Release};
use crate::{CoreError, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Synthetic registry row for non-`SKILL.md` files inside a skill directory
/// (scripts, references). They are scanned as plain text.
pub static SKILL_FILE_ROW: RegistryRow = RegistryRow {
    id: "agent-skills-file",
    tool: Tool::AgentSkills,
    kind: FileKind::Skill,
    globs: &["<inside a SKILL.md directory>"],
    parsers: &[ParserFamily::Text],
    dialect: None,
    always: AlwaysLoaded::Never,
    release: Release::Mvp,
    doc_url: "https://agentskills.io/specification",
    global_only: false,
    requires_sibling: None,
};

/// Options controlling discovery.
#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// Include globs (empty = all classified files).
    pub include: Vec<String>,
    /// Exclude globs.
    pub exclude: Vec<String>,
    /// Kinds to keep (empty = all).
    pub kinds: Vec<FileKind>,
    /// Tools to keep (empty = all).
    pub tools: Vec<Tool>,
    /// Skip files larger than this many KiB.
    pub max_file_size_kib: u64,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
            kinds: Vec::new(),
            tools: Vec::new(),
            max_file_size_kib: 2048,
        }
    }
}

impl DiscoveryOptions {
    /// Build options from the `[scan]` config section.
    pub fn from_config(c: &crate::config::ScanConfig) -> Self {
        Self {
            include: c.include.clone(),
            exclude: c.exclude.clone(),
            kinds: c.kinds.clone(),
            tools: c.tools.clone(),
            max_file_size_kib: c.max_file_size_kib,
        }
    }
}

/// A discovered and classified agent file.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredFile {
    /// Display path (relative to the working directory as given, `/` separated).
    pub path: String,
    /// Path used for registry matching (relative to the scanned directory).
    pub match_path: String,
    /// Filesystem path to read.
    pub fs_path: PathBuf,
    /// Matched registry row.
    pub row: &'static RegistryRow,
    /// Skill directory (display path) if the file belongs to a skill.
    pub skill_dir: Option<String>,
    /// File content (lossy UTF-8).
    pub text: String,
    /// Whether the content was not valid UTF-8.
    pub lossy: bool,
}

impl DiscoveredFile {
    /// Tool of the file.
    pub fn tool(&self) -> Tool {
        self.row.tool
    }
    /// Kind of the file.
    pub fn kind(&self) -> FileKind {
        self.row.kind
    }
}

/// A skipped file or other discovery problem.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct Warning {
    /// Display path.
    pub path: String,
    /// English message.
    pub message: String,
}

/// Discovery result.
#[derive(Debug, Clone, Default)]
pub struct Discovery {
    /// Files sorted by display path.
    pub files: Vec<DiscoveredFile>,
    /// Warnings sorted by path.
    pub warnings: Vec<Warning>,
}

fn build_set(patterns: &[String]) -> Result<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let g = GlobBuilder::new(p)
            .literal_separator(true)
            .build()
            .map_err(|e| CoreError::Glob {
                pattern: p.clone(),
                message: e.to_string(),
            })?;
        b.add(g);
    }
    b.build().map_err(|e| CoreError::Glob {
        pattern: patterns.join(", "),
        message: e.to_string(),
    })
}

/// True if `rel` or any of its ancestor directories matches `set`.
fn matches_path_or_parent(set: &GlobSet, rel: &str) -> bool {
    if set.is_match(rel) {
        return true;
    }
    let mut cur = rel;
    while let Some(i) = cur.rfind('/') {
        cur = &cur[..i];
        if set.is_match(cur) || set.is_match(format!("{cur}/")) {
            return true;
        }
    }
    false
}

fn to_slash(p: &Path) -> String {
    let s: Vec<String> = p
        .components()
        .filter_map(|c| match c {
            std::path::Component::CurDir => None,
            other => Some(other.as_os_str().to_string_lossy().into_owned()),
        })
        .collect();
    let joined = s.join("/");
    // `RootDir` renders as `/`, so joining yields `//`; keep a single slash.
    if joined.starts_with("//") {
        format!("/{}", joined.trim_start_matches('/'))
    } else if p.has_root() && !joined.starts_with('/') && cfg!(unix) {
        format!("/{}", joined)
    } else {
        joined
    }
}

fn display_join(base: &str, rel: &str) -> String {
    if base.is_empty() || base == "." {
        rel.to_string()
    } else {
        format!("{}/{}", base.trim_end_matches('/'), rel)
    }
}

/// Discover agent files under `paths` (directories are walked; files are
/// classified directly, including global-only names).
pub fn discover(paths: &[PathBuf], opts: &DiscoveryOptions) -> Result<Discovery> {
    let include = build_set(&opts.include)?;
    let exclude = build_set(&opts.exclude)?;
    let mut out = Discovery::default();
    let mut seen = BTreeSet::new();
    let max_bytes = opts.max_file_size_kib.saturating_mul(1024);

    for given in paths {
        let meta = std::fs::metadata(given).map_err(|e| CoreError::io(given, e))?;
        if meta.is_file() {
            let disp = to_slash(given);
            match registry::match_explicit(given) {
                Some(row) if row.release == Release::Mvp => {
                    let name = given
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let skill_dir = (row.kind == FileKind::Skill)
                        .then(|| given.parent().map(to_slash).unwrap_or_default());
                    let mp = match_path_for_explicit(given, row).unwrap_or(name);
                    push_file(
                        &mut out, &mut seen, opts, max_bytes, given, disp, mp, row, skill_dir,
                    );
                }
                Some(row) => out.warnings.push(Warning {
                    path: disp,
                    message: format!("skipped: {} support is planned for v0.2", row.id),
                }),
                None => out.warnings.push(Warning {
                    path: disp,
                    message: "skipped: not a recognized agent file".into(),
                }),
            }
            continue;
        }
        walk_dir(
            given, opts, &include, &exclude, max_bytes, &mut out, &mut seen,
        )?;
    }
    out.files.sort_by(|a, b| a.path.cmp(&b.path));
    out.warnings.sort();
    out.warnings.dedup();
    Ok(out)
}

fn match_path_for_explicit(p: &Path, row: &RegistryRow) -> Option<String> {
    let parts: Vec<String> = p
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    (0..parts.len())
        .map(|i| parts[i..].join("/"))
        .find(|s| registry::match_path(s, Origin::Explicit).map(|r| r.id) == Some(row.id))
}

#[allow(clippy::too_many_arguments)] // internal helper; grouping the args would only add noise
fn push_file(
    out: &mut Discovery,
    seen: &mut BTreeSet<PathBuf>,
    opts: &DiscoveryOptions,
    max_bytes: u64,
    fs_path: &Path,
    path: String,
    match_path: String,
    row: &'static RegistryRow,
    skill_dir: Option<String>,
) {
    if !opts.kinds.is_empty() && !opts.kinds.contains(&row.kind) {
        return;
    }
    if !opts.tools.is_empty() && !opts.tools.contains(&row.tool) {
        return;
    }
    let key = std::fs::canonicalize(fs_path).unwrap_or_else(|_| fs_path.to_path_buf());
    if !seen.insert(key) {
        return;
    }
    let size = std::fs::metadata(fs_path).map(|m| m.len()).unwrap_or(0);
    if size > max_bytes {
        out.warnings.push(Warning {
            path,
            message: format!(
                "skipped: file is {} KiB, larger than max_file_size_kib = {}",
                size.div_ceil(1024),
                opts.max_file_size_kib
            ),
        });
        return;
    }
    let bytes = match std::fs::read(fs_path) {
        Ok(b) => b,
        Err(e) => {
            out.warnings.push(Warning {
                path,
                message: format!("skipped: cannot read file: {e}"),
            });
            return;
        }
    };
    if is_binary(&bytes) {
        out.warnings.push(Warning {
            path,
            message: "skipped: binary file".into(),
        });
        return;
    }
    let (text, lossy) = match String::from_utf8(bytes) {
        Ok(s) => (s, false),
        Err(e) => (String::from_utf8_lossy(e.as_bytes()).into_owned(), true),
    };
    if lossy {
        out.warnings.push(Warning {
            path: path.clone(),
            message: "file is not valid UTF-8; scanned lossily".into(),
        });
    }
    out.files.push(DiscoveredFile {
        path,
        match_path,
        fs_path: fs_path.to_path_buf(),
        row,
        skill_dir,
        text,
        lossy,
    });
}

/// A file is binary if its first 8 KiB contain a NUL byte.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|&b| b == 0)
}

fn walk_dir(
    root: &Path,
    opts: &DiscoveryOptions,
    include: &GlobSet,
    exclude: &GlobSet,
    max_bytes: u64,
    out: &mut Discovery,
    seen: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let base = to_slash(root);
    let canon_root = std::fs::canonicalize(root).map_err(|e| CoreError::io(root, e))?;
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(false)
        .require_git(false)
        .ignore(true)
        .parents(true)
        .follow_links(false)
        .sort_by_file_path(|a, b| a.cmp(b))
        .filter_entry(|e| e.file_name() != ".git")
        .build();

    // When a relative sub-directory such as `.cursor` is scanned, its files
    // match registry patterns only with the directory prefix restored.
    let prefix: Option<String> = (root.is_relative()
        && root.components().count() > 0
        && root.components().all(|c| {
            matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        }))
    .then(|| {
        to_slash(root)
            .trim_start_matches("./")
            .trim_end_matches('/')
            .to_string()
    })
    .filter(|p| !p.is_empty() && p != ".");
    let effective = |rel: &str| -> (String, bool) {
        if registry::match_path(rel, Origin::Walked).is_some() {
            return (rel.to_string(), false);
        }
        if let Some(p) = &prefix {
            // try the innermost directories first: `a/.claude` → `.claude/x`, `a/.claude/x`
            let parts: Vec<&str> = p.split('/').collect();
            for i in (0..parts.len()).rev() {
                let full = format!("{}/{rel}", parts[i..].join("/"));
                if registry::match_path(&full, Origin::Walked).is_some() {
                    return (full, true);
                }
            }
        }
        (rel.to_string(), false)
    };

    // First pass: collect candidate files (relative paths) and skill dirs.
    let mut entries: BTreeMap<String, PathBuf> = BTreeMap::new();
    for item in walker {
        let entry = match item {
            Ok(e) => e,
            Err(e) => {
                out.warnings.push(Warning {
                    path: base.clone(),
                    message: format!("walk error: {e}"),
                });
                continue;
            }
        };
        let ft = match entry.file_type() {
            Some(ft) => ft,
            None => continue,
        };
        let p = entry.path();
        let rel = match p.strip_prefix(root) {
            Ok(r) => to_slash(r),
            Err(_) => continue,
        };
        if rel.is_empty() {
            continue;
        }
        if ft.is_symlink() {
            match std::fs::canonicalize(p) {
                Ok(target) if target.starts_with(&canon_root) => {
                    if target.is_file() {
                        entries.insert(rel, p.to_path_buf());
                    }
                }
                Ok(_) => out.warnings.push(Warning {
                    path: display_join(&base, &rel),
                    message: "skipped: symlink points outside the scan root".into(),
                }),
                Err(_) => out.warnings.push(Warning {
                    path: display_join(&base, &rel),
                    message: "skipped: broken symlink".into(),
                }),
            }
            continue;
        }
        if ft.is_file() {
            entries.insert(rel, p.to_path_buf());
        }
    }

    let skill_dirs: Vec<String> = entries
        .keys()
        .filter(|r| {
            r.ends_with("SKILL.md") && (r.as_str() == "SKILL.md" || r.ends_with("/SKILL.md"))
        })
        .filter(|r| {
            registry::match_path(&effective(r).0, Origin::Walked).map(|x| x.kind)
                == Some(FileKind::Skill)
        })
        .map(|r| {
            r.rsplit_once('/')
                .map(|(d, _)| d.to_string())
                .unwrap_or_default()
        })
        .collect();

    for (rel, fs_path) in &entries {
        if matches_path_or_parent(exclude, rel) {
            continue;
        }
        if !include.is_empty() && !include.is_match(rel) {
            continue;
        }
        let skill_dir = skill_dirs
            .iter()
            .filter(|d| d.is_empty() || rel.starts_with(&format!("{d}/")))
            .max_by_key(|d| d.len());
        let (mrel, prefixed) = effective(rel);
        let row = match registry::match_path(&mrel, Origin::Walked) {
            Some(r) if r.kind == FileKind::Skill => Some(r),
            _ if skill_dir.is_some() => Some(&SKILL_FILE_ROW),
            Some(r) => Some(r),
            None => None,
        };
        let row = match row {
            Some(r) => r,
            None => continue,
        };
        if row.release != Release::Mvp {
            out.warnings.push(Warning {
                path: display_join(&base, rel),
                message: format!("skipped: {} support is planned for v0.2", row.id),
            });
            continue;
        }
        if let Some(sib) = row.requires_sibling {
            // e.g. `hooks/hooks.json` needs `.claude-plugin/` in the parent of `hooks/`.
            let dir = Path::new(&mrel)
                .parent()
                .and_then(Path::parent)
                .unwrap_or(Path::new(""));
            let dir = if prefixed {
                dir.to_path_buf()
            } else {
                root.join(dir)
            };
            if !dir.join(sib).exists() {
                continue;
            }
        }
        let skill_disp =
            skill_dir.map(|d| display_join(&base, d).trim_end_matches('/').to_string());
        push_file(
            out,
            seen,
            opts,
            max_bytes,
            fs_path,
            display_join(&base, rel),
            mrel,
            row,
            skill_disp,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn all_tools() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/repos/all-tools")
    }

    fn listing(d: &Discovery, strip: &str) -> String {
        let mut s = String::new();
        for f in &d.files {
            let p = f
                .path
                .strip_prefix(strip)
                .unwrap_or(&f.path)
                .trim_start_matches('/');
            let _ = writeln!(
                s,
                "{p}  tool={} kind={} row={}{}",
                f.tool(),
                f.kind(),
                f.row.id,
                f.skill_dir
                    .as_ref()
                    .map(|d| format!(
                        " skill={}",
                        d.strip_prefix(strip).unwrap_or(d).trim_start_matches('/')
                    ))
                    .unwrap_or_default()
            );
        }
        for w in &d.warnings {
            let p = w
                .path
                .strip_prefix(strip)
                .unwrap_or(&w.path)
                .trim_start_matches('/');
            let _ = writeln!(s, "warning: {p}: {}", w.message);
        }
        s
    }

    #[test]
    fn all_tools_classification_snapshot() {
        let root = all_tools();
        let d = discover(std::slice::from_ref(&root), &DiscoveryOptions::default()).unwrap();
        let strip = to_slash(&root);
        insta::assert_snapshot!(listing(&d, &strip));
    }

    #[test]
    fn all_tools_covers_every_mvp_row() {
        let d = discover(&[all_tools()], &DiscoveryOptions::default()).unwrap();
        for r in registry::mvp_rows().filter(|r| !r.global_only) {
            assert!(
                d.files.iter().any(|f| f.row.id == r.id),
                "no fixture for row {}",
                r.id
            );
        }
    }

    fn write(root: &Path, rel: &str, content: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn size_limit_skips_with_warning() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "AGENTS.md", &vec![b'a'; 3 * 1024]);
        write(t.path(), "GEMINI.md", b"small");
        let opts = DiscoveryOptions {
            max_file_size_kib: 2,
            ..Default::default()
        };
        let d = discover(&[t.path().to_path_buf()], &opts).unwrap();
        assert_eq!(d.files.len(), 1);
        assert!(d.files[0].path.ends_with("GEMINI.md"));
        assert_eq!(d.warnings.len(), 1);
        assert!(d.warnings[0]
            .message
            .contains("larger than max_file_size_kib = 2"));
    }

    #[test]
    fn binary_files_skipped() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "AGENTS.md", b"text\0with nul");
        let d = discover(&[t.path().to_path_buf()], &DiscoveryOptions::default()).unwrap();
        assert!(d.files.is_empty());
        assert_eq!(d.warnings[0].message, "skipped: binary file");
    }

    #[test]
    fn non_utf8_is_lossy_with_warning() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "AGENTS.md", b"caf\xe9 rules");
        let d = discover(&[t.path().to_path_buf()], &DiscoveryOptions::default()).unwrap();
        assert_eq!(d.files.len(), 1);
        assert!(d.files[0].lossy);
        assert!(d.files[0].text.contains('\u{FFFD}'));
        assert!(d.warnings[0].message.contains("not valid UTF-8"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_not_followed() {
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), "secret.md", b"outside");
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join(".cursor/rules")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("secret.md"), t.path().join("AGENTS.md"))
            .unwrap();
        std::os::unix::fs::symlink(outside.path(), t.path().join(".cursor/rules/linked")).unwrap();
        write(t.path(), "CLAUDE.md", b"inside");
        std::os::unix::fs::symlink(t.path().join("CLAUDE.md"), t.path().join("GEMINI.md")).unwrap();
        let d = discover(&[t.path().to_path_buf()], &DiscoveryOptions::default()).unwrap();
        let names: Vec<_> = d.files.iter().map(|f| f.match_path.as_str()).collect();
        assert_eq!(
            names,
            vec!["CLAUDE.md"],
            "inside symlink to an already-seen file is deduplicated"
        );
        assert!(d
            .warnings
            .iter()
            .any(|w| w.path.ends_with("AGENTS.md") && w.message.contains("outside the scan root")));
        assert!(d
            .warnings
            .iter()
            .any(|w| w.path.ends_with("linked") && w.message.contains("outside the scan root")));
    }

    #[test]
    fn exclude_and_include_globs() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "AGENTS.md", b"a");
        write(t.path(), "vendor/lib/AGENTS.md", b"b");
        write(t.path(), "node_modules/pkg/AGENTS.md", b"c");
        write(t.path(), ".cursor/rules/a.mdc", b"d");
        let opts = DiscoveryOptions {
            exclude: vec!["vendor/**".into(), "node_modules".into()],
            ..Default::default()
        };
        let d = discover(&[t.path().to_path_buf()], &opts).unwrap();
        let names: Vec<_> = d.files.iter().map(|f| f.match_path.as_str()).collect();
        assert_eq!(names, vec![".cursor/rules/a.mdc", "AGENTS.md"]);
        let opts = DiscoveryOptions {
            include: vec![".cursor/**".into()],
            ..Default::default()
        };
        let d = discover(&[t.path().to_path_buf()], &opts).unwrap();
        assert_eq!(d.files.len(), 1);
        assert!(matches!(
            build_set(&["[".into()]),
            Err(CoreError::Glob { .. })
        ));
    }

    #[test]
    fn kind_and_tool_filters() {
        let d = discover(
            &[all_tools()],
            &DiscoveryOptions {
                tools: vec![Tool::Kiro],
                ..Default::default()
            },
        )
        .unwrap();
        assert!(d.files.iter().all(|f| f.tool() == Tool::Kiro));
        assert_eq!(d.files.len(), 3);
        let d = discover(
            &[all_tools()],
            &DiscoveryOptions {
                kinds: vec![FileKind::Hooks],
                ..Default::default()
            },
        )
        .unwrap();
        assert!(d.files.iter().all(|f| f.kind() == FileKind::Hooks));
        assert!(d.files.len() >= 5);
    }

    #[test]
    fn gitignore_respected_and_hidden_included() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), ".gitignore", b"ignored/\n");
        write(t.path(), "ignored/AGENTS.md", b"x");
        write(t.path(), ".kiro/steering/a.md", b"y");
        let d = discover(&[t.path().to_path_buf()], &DiscoveryOptions::default()).unwrap();
        let names: Vec<_> = d.files.iter().map(|f| f.match_path.as_str()).collect();
        assert_eq!(names, vec![".kiro/steering/a.md"]);
    }

    #[test]
    fn explicit_global_file_is_classified() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "windsurf/mcp_config.json", b"{}");
        write(t.path(), "notes.txt", b"x");
        let d = discover(
            &[
                t.path().join("windsurf/mcp_config.json"),
                t.path().join("notes.txt"),
            ],
            &DiscoveryOptions::default(),
        )
        .unwrap();
        assert_eq!(d.files.len(), 1);
        assert_eq!(d.files[0].tool(), Tool::Windsurf);
        assert!(d.warnings[0]
            .message
            .contains("not a recognized agent file"));
        // walking the same directory does not pick it up
        let d = discover(&[t.path().to_path_buf()], &DiscoveryOptions::default()).unwrap();
        assert!(d.files.is_empty());
    }

    #[test]
    fn plugin_hooks_need_claude_plugin_sibling() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "hooks/hooks.json", b"{}");
        let d = discover(&[t.path().to_path_buf()], &DiscoveryOptions::default()).unwrap();
        assert!(d.files.is_empty());
        write(t.path(), ".claude-plugin/plugin.json", b"{}");
        let d = discover(&[t.path().to_path_buf()], &DiscoveryOptions::default()).unwrap();
        assert_eq!(d.files.len(), 2);
    }
}
