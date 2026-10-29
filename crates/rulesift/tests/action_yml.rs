//! Checks on the composite GitHub Action (`action.yml`, PRD §12.1).

use serde_json::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn action() -> Value {
    let text = std::fs::read_to_string(root().join("action.yml")).unwrap();
    serde_saphyr::from_str(&text).unwrap()
}

fn steps(a: &Value) -> &Vec<Value> {
    a["runs"]["steps"].as_array().unwrap()
}

#[test]
fn inputs_and_outputs_match_prd() {
    let a = action();
    assert_eq!(a["runs"]["using"], "composite");
    for k in [
        "version", "path", "mode", "base", "fail-on", "sarif", "comment", "args", "install",
    ] {
        assert!(a["inputs"].get(k).is_some(), "missing input {k}");
    }
    assert_eq!(a["inputs"]["path"]["default"], ".");
    assert_eq!(a["inputs"]["fail-on"]["default"], "block");
    assert_eq!(a["inputs"]["sarif"]["default"], "true");
    assert_eq!(a["inputs"]["comment"]["default"], "true");
    assert_eq!(a["inputs"]["install"]["default"], "release");
    assert!(a["outputs"].get("exit-code").is_some());
}

#[test]
fn third_party_actions_are_pinned_to_a_major() {
    let a = action();
    let uses: Vec<&str> = steps(&a)
        .iter()
        .filter_map(|s| s["uses"].as_str())
        .collect();
    assert!(uses.contains(&"github/codeql-action/upload-sarif@v4"));
    assert!(uses.contains(&"actions/github-script@v9"));
    for u in uses {
        let (_, v) = u.split_once('@').unwrap();
        assert!(
            v.starts_with('v') && v[1..].chars().all(|c| c.is_ascii_digit()),
            "{u}"
        );
    }
}

#[test]
fn run_scripts_never_interpolate_inputs() {
    // inputs reach shell scripts only through env vars (no script injection)
    for s in steps(&action()) {
        if let Some(run) = s["run"].as_str() {
            assert!(!run.contains("${{"), "expression inside run script: {run}");
        }
    }
}

#[test]
fn comment_step_finds_the_marker() {
    let a = action();
    let script = steps(&a)
        .iter()
        .find(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/github-script"))
        })
        .unwrap()["with"]["script"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(script.contains("<!-- rulesift:pr-comment -->"));
    // #120: never take over someone else's comment that starts with the marker
    assert!(
        script.contains("c.user.login === 'github-actions[bot]'"),
        "{script}"
    );
    assert!(script.contains("updateComment") && script.contains("createComment"));
}

#[cfg(unix)]
#[test]
fn run_step_executes_and_records_exit_code() {
    let a = action();
    let run = steps(&a).iter().find(|s| s["id"] == "run").unwrap()["run"]
        .as_str()
        .unwrap()
        .to_string();
    let t = tempfile::tempdir().unwrap();
    let bin = assert_cmd::cargo::cargo_bin("rulesift");
    let path = format!(
        "{}:{}",
        bin.parent().unwrap().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let fixture = root().join("tests/fixtures/repos/risky");
    let o = std::process::Command::new("bash")
        .arg("-c")
        .arg(&run)
        .current_dir(&fixture)
        .env("PATH", path)
        .env("RUNNER_TEMP", t.path())
        .env("GITHUB_OUTPUT", t.path().join("out"))
        .env("GITHUB_STEP_SUMMARY", t.path().join("summary"))
        .env("INPUT_PATH", ".")
        .env("INPUT_MODE", "")
        .env("INPUT_BASE", "")
        .env("INPUT_FAIL_ON", "block")
        .env("INPUT_ARGS", "--no-color")
        .env("EVENT_NAME", "push")
        .env("PR_BASE_SHA", "")
        .env("RULESIFT_CONFIG", "")
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let out = std::fs::read_to_string(t.path().join("out")).unwrap();
    assert!(out.contains("exit-code=1"), "{out}");
    let p = |k: &str| -> PathBuf {
        let line = out.lines().find(|l| l.starts_with(k)).unwrap();
        PathBuf::from(&line[k.len() + 1..])
    };
    assert!(Path::new(&p("sarif-file")).is_file());
    assert!(std::fs::read_to_string(p("markdown-file"))
        .unwrap()
        .starts_with("<!-- rulesift:pr-comment -->"));
    assert!(std::fs::read_to_string(t.path().join("summary"))
        .unwrap()
        .contains("RuleSift:"));
}
