//! Every `rulesift …` command in README.md runs against the built binary,
//! and relative links in README.md and docs/ point at existing files (W6-T6).

use assert_cmd::Command;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Commands from ```bash blocks that start with `rulesift `, comments removed.
fn readme_commands() -> Vec<String> {
    let text = std::fs::read_to_string(root().join("README.md")).unwrap();
    let mut out = Vec::new();
    let mut in_bash = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_bash = line == "```bash";
            continue;
        }
        if in_bash && line.starts_with("rulesift ") {
            let cmd = line.split(" #").next().unwrap().trim();
            out.push(cmd.to_string());
        }
    }
    out
}

fn git(dir: &Path, args: &[&str]) {
    let ok = std::process::Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

#[test]
fn readme_commands_run() {
    let cmds = readme_commands();
    assert!(cmds.len() >= 8, "{cmds:?}");
    let d = tempfile::tempdir().unwrap();
    let p = d.path();
    std::fs::create_dir_all(p.join(".cursor/rules")).unwrap();
    std::fs::write(
        p.join(".cursor/rules/web.mdc"),
        "---\ndescription: Web\nalwaysApply: true\n---\n\nUse 2-space indentation.\n",
    )
    .unwrap();
    std::fs::write(p.join("AGENTS.md"), "# Project\n\nRun `cargo test`.\n").unwrap();
    git(p, &["init", "-q", "-b", "master"]);
    git(p, &["add", "-A"]);
    git(p, &["commit", "-q", "-m", "init"]);
    git(p, &["update-ref", "refs/remotes/origin/master", "HEAD"]);
    std::fs::write(p.join("CLAUDE.md"), "# Claude\n\nBe brief.\n").unwrap();
    for cmd in &cmds {
        let args: Vec<&str> = cmd.split_whitespace().skip(1).collect();
        let o = Command::cargo_bin("rulesift")
            .unwrap()
            .args(&args)
            .env("RULESIFT_CONFIG", "")
            .env("PATH", "/usr/bin:/bin") // no agent CLIs for --explain
            .current_dir(p)
            .output()
            .unwrap();
        let code = o.status.code();
        assert!(
            matches!(code, Some(0) | Some(1)),
            "`{cmd}` exited {code:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

#[test]
fn relative_links_resolve() {
    let mut files = vec![
        root().join("README.md"),
        root().join("CONTRIBUTING.md"),
        root().join("SECURITY.md"),
    ];
    for e in std::fs::read_dir(root().join("docs")).unwrap() {
        let path = e.unwrap().path();
        if path.extension().is_some_and(|x| x == "md") {
            files.push(path);
        }
    }
    let re = regex::Regex::new(r"\]\(([^)\s]+)\)").unwrap();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        for c in re.captures_iter(&text) {
            let target = c[1].split('#').next().unwrap();
            if target.is_empty() || target.contains("://") || target.starts_with("mailto:") {
                continue;
            }
            let p = f.parent().unwrap().join(target);
            assert!(p.exists(), "{}: broken link {target}", f.display());
        }
    }
}

/// #107: tests shell out to `git` and `bash`; CONTRIBUTING must say so.
#[test]
fn contributing_lists_test_prerequisites() {
    let text = std::fs::read_to_string(root().join("CONTRIBUTING.md")).unwrap();
    let section = text
        .split("## Prerequisites")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .expect("CONTRIBUTING.md has a Prerequisites section");
    for tool in ["`git`", "`bash`", "1.89"] {
        assert!(section.contains(tool), "Prerequisites must mention {tool}");
    }
}

/// GitHub's heading anchor: lowercase, spaces to `-`, punctuation dropped.
fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// #108: every `docs/<page>.md#anchor` link in the sources points at a
/// heading that exists.
#[test]
fn source_doc_anchors_resolve() {
    let re = regex::Regex::new(r"docs/([A-Za-z0-9_/.-]+\.md)#([A-Za-z0-9_-]+)").unwrap();
    let mut checked = 0;
    for dir in ["crates/rulesift-core/src", "crates/rulesift/src"] {
        for entry in walk(&root().join(dir)) {
            if entry.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&entry).unwrap();
            for c in re.captures_iter(&text) {
                let page = std::fs::read_to_string(root().join("docs").join(&c[1]))
                    .unwrap_or_else(|_| panic!("{}: docs/{} missing", entry.display(), &c[1]));
                let anchors: Vec<String> = page
                    .lines()
                    .filter_map(|l| l.strip_prefix('#'))
                    .map(|l| slug(l.trim_start_matches('#')))
                    .collect();
                assert!(
                    anchors.iter().any(|a| a == &c[2]),
                    "{}: docs/{}#{} has no matching heading",
                    entry.display(),
                    &c[1],
                    &c[2]
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0);
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}
