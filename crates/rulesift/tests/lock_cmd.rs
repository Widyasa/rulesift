use assert_cmd::Command;
use std::fs;
use std::path::Path;

fn bin(dir: &Path) -> Command {
    let mut c = Command::cargo_bin("rulesift").unwrap();
    c.env("RULESIFT_CONFIG", "").current_dir(dir);
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
fn lock_then_verify_then_tamper() {
    let t = tempfile::tempdir().unwrap();
    let r = t.path();
    fs::write(r.join("AGENTS.md"), "# Guide\n").unwrap();
    fs::write(
        r.join(".mcp.json"),
        r#"{"mcpServers":{"a":{"command":"uvx","args":["a==1.0"]}}}"#,
    )
    .unwrap();

    let (code, _, err) = run(bin(r).arg("verify"));
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("rulesift lock"));

    let (code, out, _) = run(bin(r).arg("lock"));
    assert_eq!(code, Some(0));
    assert!(out.contains("2 files, 0 skills, 1 MCP servers"), "{out}");
    let first = fs::read_to_string(r.join("rulesift.lock")).unwrap();
    // idempotent
    run(bin(r).arg("lock"));
    assert_eq!(fs::read_to_string(r.join("rulesift.lock")).unwrap(), first);

    let (code, out, _) = run(bin(r).arg("verify"));
    assert_eq!(code, Some(0), "{out}");
    let (code, _, _) = run(bin(r).args(["lock", "--check"]));
    assert_eq!(code, Some(0));

    fs::write(r.join("AGENTS.md"), "# Guide\u{200B}\n").unwrap();
    fs::write(r.join("CLAUDE.md"), "new\n").unwrap();
    let (code, out, _) = run(bin(r).arg("verify"));
    assert_eq!(code, Some(1));
    assert!(
        out.contains("RS-LOCK-001") && out.contains("RS-LOCK-002"),
        "{out}"
    );
    let (code, _, _) = run(bin(r).args(["lock", "--check"]));
    assert_eq!(code, Some(1));
}

fn json_lock(r: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value, String) {
    let (code, out, err) =
        run(bin(r)
            .arg("scan")
            .args(args)
            .args(["--format", "json", "--fail-on", "never"]));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap_or_else(|e| panic!("{e}: {out}"));
    (code, v["lock"].clone(), err)
}

/// #106: `scan` reports the real lock state instead of `present: false`.
#[test]
fn scan_reports_lock_state() {
    let t = tempfile::tempdir().unwrap();
    let r = t.path();
    fs::write(r.join("AGENTS.md"), "# Guide\n").unwrap();

    // no lockfile: checked, not present
    let (code, lock, _) = json_lock(r, &["."]);
    assert_eq!(code, Some(0));
    assert_eq!(
        lock,
        serde_json::json!({"present": false, "violations": []})
    );

    run(bin(r).arg("lock"));
    let (_, lock, err) = json_lock(r, &[]);
    assert_eq!(lock, serde_json::json!({"present": true, "violations": []}));
    assert!(!err.contains("rulesift.lock"), "{err}");

    // stale lock: violations reported, exit code unchanged
    fs::write(r.join("AGENTS.md"), "# Guide\nx\n").unwrap();
    let (code, lock, err) = json_lock(r, &["."]);
    assert_eq!(code, Some(0));
    assert_eq!(lock["present"], true);
    assert_eq!(lock["violations"][0]["rule_id"], "RS-LOCK-001", "{lock}");
    assert!(
        err.contains("warning: rulesift.lock: 1 difference"),
        "{err}"
    );
    let (code, _, _) = run(bin(r).arg("verify"));
    assert_eq!(code, Some(1));

    // a single file is not a lock check: null, not `present: false`
    let (_, lock, _) = json_lock(r, &["AGENTS.md"]);
    assert!(lock.is_null(), "{lock}");
}
