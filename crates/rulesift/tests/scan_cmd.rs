use assert_cmd::Command;
use std::path::PathBuf;

fn repo(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/repos")
        .join(name)
}

fn bin(dir: &str) -> Command {
    let mut c = Command::cargo_bin("rulesift").unwrap();
    c.env("RULESIFT_CONFIG", "")
        .env_remove("NO_COLOR")
        .current_dir(repo(dir));
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
fn all_tools_table_snapshot() {
    let (code, out, _) = run(bin("all-tools").args(["scan", ".", "--no-color"]));
    assert_eq!(code, Some(0));
    insta::assert_snapshot!("scan_all_tools_table", out);
}

#[test]
fn risky_table_snapshot() {
    let (code, out, _) = run(bin("risky").args(["scan", ".", "--no-color"]));
    assert_eq!(code, Some(1));
    insta::assert_snapshot!("scan_risky_table", out);
}

#[test]
fn quiet_hides_ok_files() {
    let (_, out, _) = run(bin("risky").args(["-q", "scan", ".", "--no-color"]));
    assert!(!out.contains("CLAUDE.md"));
    assert!(out.contains("AGENTS.md"));
}

#[test]
fn fail_on_thresholds() {
    // whole repo has a BLOCK file
    for (fail_on, want) in [("block", 1), ("review", 1), ("info", 1), ("never", 0)] {
        let (code, _, _) = run(bin("risky").args(["scan", ".", "--fail-on", fail_on]));
        assert_eq!(code, Some(want), "fail-on {fail_on}");
    }
    // .cursor alone has only an INFO file
    for (fail_on, want) in [("block", 0), ("review", 0), ("info", 1), ("never", 0)] {
        let (code, _, _) = run(bin("risky").args(["scan", ".cursor", "--fail-on", fail_on]));
        assert_eq!(code, Some(want), ".cursor fail-on {fail_on}");
    }
}

#[test]
fn bad_fail_on_is_usage_error() {
    let (code, _, err) = run(bin("risky").args(["scan", ".", "--fail-on", "sometimes"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("sometimes"));
}

#[test]
fn config_error_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("bad.toml");
    std::fs::write(&cfg, "[policy]\nfail_on = \"sometimes\"\n").unwrap();
    let (code, _, err) = run(bin("risky")
        .args(["--config"])
        .arg(&cfg)
        .args(["scan", "."]));
    assert_eq!(code, Some(2));
    assert!(err.starts_with("error:"), "{err}");
}

#[test]
fn missing_path_exits_2() {
    let (code, _, err) = run(bin("risky").args(["scan", "does-not-exist"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("does-not-exist"));
}

#[test]
fn min_severity_hides_findings() {
    let (code, out, _) =
        run(bin("risky").args(["scan", ".", "--no-color", "--min-severity", "critical"]));
    assert_eq!(code, Some(1));
    assert!(out.contains("RS-EXEC-001"));
    assert!(!out.contains("RS-INJ-001"));
}

#[test]
fn write_baseline_then_suppress() {
    let dir = tempfile::tempdir().unwrap();
    let b = dir.path().join("baseline.json");
    let (code, _, _) = run(bin("risky")
        .args(["scan", "."])
        .arg("--write-baseline")
        .arg(&b));
    assert_eq!(code, Some(0));
    let text = std::fs::read_to_string(&b).unwrap();
    assert!(text.contains("RS-EXEC-001"));
    let (code, out, _) = run(bin("risky")
        .args(["scan", ".", "--no-color", "--baseline"])
        .arg(&b));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("suppressed"));
}

#[test]
fn kinds_and_disable_filters() {
    let (_, out, _) =
        run(bin("risky").args(["scan", ".", "--no-color", "--disable", "RS-EXEC-001"]));
    assert!(!out.contains("RS-EXEC-001"));
    let (code, _, err) = run(bin("risky").args(["scan", ".", "--kinds", "nonsense"]));
    assert_eq!(code, Some(2), "{err}");
}

#[test]
fn json_stdout_and_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("report.json");
    let (code, out, _) = run(bin("risky")
        .args(["scan", ".", "--format", "json", "--json"])
        .arg(&file));
    assert_eq!(code, Some(1));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["summary"]["block"], 1);
    assert!(v["generated_at"].as_str().unwrap().ends_with('Z'));
    let f: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(f, v);
}

#[test]
fn sarif_file_has_relative_uris() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("out.sarif");
    let (code, _, _) = run(bin("risky")
        .args(["scan", ".", "--no-color", "--sarif"])
        .arg(&file));
    assert_eq!(code, Some(1));
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let uris: Vec<&str> = v["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert_eq!(uris, vec![".cursor/rules/style.mdc", "AGENTS.md"]);
}

#[test]
fn markdown_stdout_and_all_files_in_one_scan() {
    let dir = tempfile::tempdir().unwrap();
    let (j, sa, m) = (
        dir.path().join("r.json"),
        dir.path().join("r.sarif"),
        dir.path().join("r.md"),
    );
    let (code, out, _) = run(bin("risky")
        .args(["scan", ".", "--format", "markdown", "--json"])
        .arg(&j)
        .arg("--sarif")
        .arg(&sa)
        .arg("--markdown")
        .arg(&m));
    assert_eq!(code, Some(1));
    assert!(out.starts_with("<!-- rulesift:pr-comment -->"));
    assert_eq!(std::fs::read_to_string(&m).unwrap(), out);
    assert!(j.exists() && sa.exists());
}
