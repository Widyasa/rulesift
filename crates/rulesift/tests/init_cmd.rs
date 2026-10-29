use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};

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

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn config_template_snapshot_parses_and_force() {
    let t = tempfile::tempdir().unwrap();
    let (code, out, _) = run(bin(t.path()).arg("init"));
    assert_eq!(code, Some(0));
    assert!(out.contains("rulesift.toml"));
    let text = fs::read_to_string(t.path().join("rulesift.toml")).unwrap();
    insta::assert_snapshot!("init_config", text);
    // the template is a valid config equal to the defaults
    let cfg = rulesift_core::config::Config::load(&t.path().join("rulesift.toml")).unwrap();
    assert_eq!(cfg, rulesift_core::config::Config::default());
    // refuses to overwrite without --force
    fs::write(t.path().join("rulesift.toml"), "# mine\n").unwrap();
    let (code, _, err) = run(bin(t.path()).arg("init"));
    assert_eq!(code, Some(2));
    assert!(err.contains("--force"), "{err}");
    assert_eq!(
        fs::read_to_string(t.path().join("rulesift.toml")).unwrap(),
        "# mine\n"
    );
    let (code, _, _) = run(bin(t.path()).args(["init", "--force"]));
    assert_eq!(code, Some(0));
    assert_eq!(
        fs::read_to_string(t.path().join("rulesift.toml")).unwrap(),
        text
    );
}

#[test]
fn ci_github_workflow_snapshot() {
    let t = tempfile::tempdir().unwrap();
    let (code, _, _) = run(bin(t.path()).args(["init", "--ci", "github"]));
    assert_eq!(code, Some(0));
    let p = t.path().join(".github/workflows/rulesift.yml");
    let text = fs::read_to_string(&p).unwrap();
    insta::assert_snapshot!(
        "init_ci_github",
        text.replace(rulesift_core::VERSION, "[version]")
    );
    assert!(!t.path().join("rulesift.toml").exists());
    let (code, _, _) = run(bin(t.path()).args(["init", "--ci", "github"]));
    assert_eq!(code, Some(2));
    let (code, _, _) = run(bin(t.path()).args(["init", "--ci", "github", "--force"]));
    assert_eq!(code, Some(0));
}

#[test]
fn pre_commit_snippet_snapshot() {
    let t = tempfile::tempdir().unwrap();
    let (code, out, _) = run(bin(t.path()).args(["init", "--pre-commit"]));
    assert_eq!(code, Some(0));
    insta::assert_snapshot!(
        "init_pre_commit",
        out.replace(rulesift_core::VERSION, "[version]")
    );
    assert!(!t.path().join("rulesift.toml").exists());
}

#[test]
fn pre_commit_hooks_file_is_generated_from_registry() {
    let (_, generated, _) = run(bin(&root()).args(["init", "--print-hooks"]));
    let path = root().join(".pre-commit-hooks.yaml");
    if std::env::var_os("RULESIFT_BLESS").is_some() {
        fs::write(&path, &generated).unwrap();
    }
    let committed = fs::read_to_string(&path).unwrap();
    assert_eq!(
        committed, generated,
        ".pre-commit-hooks.yaml is stale; regenerate with RULESIFT_BLESS=1"
    );
    let v: serde_json::Value = serde_saphyr::from_str(&committed).unwrap();
    let hook = &v[0];
    assert_eq!(hook["id"], "rulesift");
    assert_eq!(hook["language"], "rust");
    assert_eq!(hook["entry"], "rulesift scan --fail-on block");
    assert_eq!(hook["pass_filenames"], true);
    assert_eq!(v[1]["id"], "rulesift-system");
    assert_eq!(v[1]["files"], hook["files"]);
    let re = regex::Regex::new(hook["files"].as_str().unwrap()).unwrap();
    assert_eq!(
        hook["files"],
        rulesift_core::registry::precommit_files_regex()
    );
    // every MVP path in the compatibility fixture matches
    let fixture = root().join("tests/fixtures/repos/all-tools");
    let d = rulesift_core::discovery::discover(&[fixture], &Default::default()).unwrap();
    for f in &d.files {
        assert!(re.is_match(&f.match_path), "{}", f.match_path);
    }
}
