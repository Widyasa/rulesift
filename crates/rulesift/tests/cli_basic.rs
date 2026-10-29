use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_prints_package_version() {
    Command::cargo_bin("rulesift")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(format!(
            "rulesift {}",
            env!("CARGO_PKG_VERSION")
        )));
}

#[test]
fn help_is_english() {
    Command::cargo_bin("rulesift")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: rulesift"))
        .stdout(predicate::str::contains("Print version"));
}

#[test]
fn hidden_list_tools_prints_registry() {
    Command::cargo_bin("rulesift")
        .unwrap()
        .arg("--list-tools")
        .assert()
        .success()
        .stdout(predicate::str::contains(".kiro/steering/*.md"))
        .stdout(predicate::str::contains("v0.2"));
    Command::cargo_bin("rulesift")
        .unwrap()
        .arg("--help")
        .assert()
        .stdout(predicate::str::contains("list-tools").not());
}
