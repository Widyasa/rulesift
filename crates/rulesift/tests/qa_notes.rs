//! Regression tests for the smaller QA notes (#120).

use assert_cmd::Command;
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/repos/risky")
}

fn run(args: &[&str]) -> (Option<i32>, String) {
    let o = Command::cargo_bin("rulesift")
        .unwrap()
        .env("RULESIFT_CONFIG", "")
        .current_dir(fixture())
        .args(args)
        .output()
        .unwrap();
    (o.status.code(), String::from_utf8(o.stderr).unwrap())
}

#[test]
fn missing_config_or_baseline_prints_the_os_error_once() {
    let dir = tempfile::tempdir().unwrap();
    for flag in ["--config", "--baseline"] {
        let missing = dir.path().join("missing.file");
        let (code, err) = run(&["scan", ".", flag, missing.to_str().unwrap()]);
        assert_eq!(code, Some(2), "{err}");
        assert!(err.contains("cannot read"), "{err}");
        let os = err.matches("No such file or directory").count()
            + err.matches("cannot find the file").count();
        assert_eq!(os, 1, "{flag}: {err}");
    }
}

#[test]
fn unknown_disable_ids_warn_unless_quiet() {
    let (code, err) = run(&[
        "scan",
        ".",
        "--fail-on",
        "never",
        "--disable",
        "RS-NOPE-001",
    ]);
    assert_eq!(code, Some(0));
    assert!(
        err.contains("warning: disabled rule RS-NOPE-001 does not exist"),
        "{err}"
    );
    let (_, err) = run(&[
        "scan",
        ".",
        "-q",
        "--fail-on",
        "never",
        "--disable",
        "RS-NOPE-001",
    ]);
    assert!(!err.contains("RS-NOPE-001"), "{err}");
    // known ids stay silent
    let (_, err) = run(&["scan", ".", "--fail-on", "never", "--disable", "RS-INJ-001"]);
    assert!(!err.contains("does not exist"), "{err}");
}
