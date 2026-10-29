//! `scan --explain` with fake agent scripts on a temp PATH (no real agents).
#![cfg(unix)]

use assert_cmd::Command;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn risky() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/repos/risky")
}

fn script(dir: &Path, name: &str, body: &str) {
    let p = dir.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn bin(path: &Path) -> Command {
    let mut c = Command::cargo_bin("rulesift").unwrap();
    c.env("RULESIFT_CONFIG", "")
        .env_remove("RULESIFT_AGENT")
        .env("PATH", format!("{}:/usr/bin:/bin", path.display()))
        .current_dir(risky());
    c
}

fn run(c: &mut Command) -> (Option<i32>, String, String) {
    let o = c.output().unwrap();
    (
        o.status.code(),
        String::from_utf8(o.stdout).unwrap(),
        String::from_utf8(o.stderr).unwrap(),
    )
}

#[test]
fn codex_fake_writes_last_message() {
    let d = tempfile::tempdir().unwrap();
    // records its args, then writes the -o file
    let log = d.path().join("args.txt");
    script(
        d.path(),
        "codex",
        &format!(
            "printf '%s\\n' \"$@\" > '{}'\nwhile [ $# -gt 0 ]; do if [ \"$1\" = -o ]; then shift; echo '- AGENTS.md: suspicious, pipes curl to bash.' > \"$1\"; fi; shift; done",
            log.display()
        ),
    );
    let (code, out, err) = run(bin(d.path()).args(["scan", ".", "--no-color", "--explain"]));
    assert_eq!(code, Some(1), "{err}");
    assert!(out.contains("AI explanation (codex"), "{out}\n{err}");
    assert!(out.contains("suspicious, pipes curl to bash"));
    let args = std::fs::read_to_string(&log).unwrap();
    assert!(args.starts_with("exec\n--sandbox\nread-only\n"), "{args}");
    assert!(args.contains("UNTRUSTED DATA"));
}

#[test]
fn claude_fake_json_and_agent_flag() {
    let d = tempfile::tempdir().unwrap();
    script(d.path(), "codex", "exit 3");
    script(
        d.path(),
        "claude",
        "echo '{\"type\":\"result\",\"is_error\":false,\"result\":\"- AGENTS.md: suspicious.\"}'",
    );
    let (code, out, err) =
        run(bin(d.path()).args(["scan", ".", "--no-color", "--explain", "--agent", "claude"]));
    assert_eq!(code, Some(1), "{err}");
    assert!(
        out.contains("AI explanation (claude") && out.contains("AGENTS.md: suspicious."),
        "{out}"
    );
}

#[test]
fn env_selects_cursor_and_json_goes_to_stderr() {
    let d = tempfile::tempdir().unwrap();
    script(
        d.path(),
        "cursor-agent",
        "echo '{\"result\":\"cursor ok\"}'",
    );
    let (_, out, err) = run(bin(d.path()).env("RULESIFT_AGENT", "cursor").args([
        "scan",
        ".",
        "--format",
        "json",
        "--explain",
    ]));
    serde_json::from_str::<serde_json::Value>(&out).expect("stdout stays valid JSON");
    assert!(err.contains("cursor ok"), "{err}");
}

#[test]
fn failures_are_warnings_and_keep_exit_code() {
    let d = tempfile::tempdir().unwrap();
    script(d.path(), "claude", "echo oops >&2; exit 7");
    let (code, _, err) = run(bin(d.path()).args(["scan", ".", "--no-color", "--explain"]));
    assert_eq!(code, Some(1));
    assert!(err.contains("warning: --explain failed"), "{err}");
    // no agent at all
    let empty = tempfile::tempdir().unwrap();
    let (code, _, err) = run(bin(empty.path()).args(["scan", ".", "--explain"]));
    assert_eq!(code, Some(1));
    assert!(err.contains("no agent CLI found"), "{err}");
}
