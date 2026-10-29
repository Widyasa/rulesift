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
