use assert_cmd::Command;

fn bin() -> Command {
    let mut c = Command::cargo_bin("rulesift").unwrap();
    // never pick up a config from the developer's tree
    c.env("RULESIFT_CONFIG", "")
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    c
}

fn stdout(c: &mut Command) -> (Option<i32>, String, String) {
    let o = c.output().unwrap();
    (
        o.status.code(),
        String::from_utf8(o.stdout).unwrap(),
        String::from_utf8(o.stderr).unwrap(),
    )
}

#[test]
fn list_contains_builtins() {
    let (code, out, _) = stdout(bin().args(["rules", "list"]));
    assert_eq!(code, Some(0));
    assert!(out.starts_with("ID"));
    assert!(out.contains("RS-UNI-001  critical  Hidden Unicode Tag characters"));
    let ids: Vec<&str> = out
        .lines()
        .skip(1)
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[test]
fn list_with_user_dir() {
    let (_, out, _) = stdout(bin().args([
        "rules",
        "list",
        "--rules",
        "../../tests/fixtures/user-rules/good",
    ]));
    assert!(out.contains("ACME-001"));
}

#[test]
fn show_snapshot() {
    let (code, out, _) = stdout(bin().args(["rules", "show", "RS-UNI-001"]));
    assert_eq!(code, Some(0));
    insta::assert_snapshot!("show_rs_uni_001", out);
}

#[test]
fn show_unknown_id() {
    let (code, _, err) = stdout(bin().args(["rules", "show", "RS-NOPE-999"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("unknown rule id"));
}

#[test]
fn test_good_bad_failing() {
    let (code, out, _) =
        stdout(bin().args(["rules", "test", "../../tests/fixtures/user-rules/good"]));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ok    ACME-001 (2 examples)"));
    assert!(out.contains("1 rules, 2 examples, 0 failed"));
    let (code, _, err) =
        stdout(bin().args(["rules", "test", "../../tests/fixtures/user-rules/bad"]));
    assert_eq!(code, Some(2));
    assert!(
        err.contains("ACME-002") && err.contains("bad.yaml") && err.contains("invalid regex"),
        "{err}"
    );
    let (code, out, _) =
        stdout(bin().args(["rules", "test", "../../tests/fixtures/user-rules/failing"]));
    assert_eq!(code, Some(1));
    assert!(
        out.contains(
            "FAIL  ACME-003 should_match \"this text is fine\": expected a match, got none"
        ),
        "{out}"
    );
    let (code, _, _) = stdout(bin().args(["rules", "test", "../../tests/fixtures/nope"]));
    assert_eq!(code, Some(2));
}
