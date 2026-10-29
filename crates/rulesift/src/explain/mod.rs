//! `scan --explain`: optional explanation by the user's own agent CLI
//! (PRD §11.2). RuleSift never calls a model API itself; it runs a locally
//! installed agent in read-only/print mode with a prompt that marks all
//! scanned content as untrusted data.

pub mod claude;
pub mod codex;
pub mod cursor;
pub mod prompt;

use rulesift_core::config::{AgentChoice, ExplainConfig};
use rulesift_core::model::Report;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Flags that must never be passed to an agent (write or bypass modes).
pub const FORBIDDEN_FLAGS: &[&str] = &[
    "--force",
    "--yolo",
    "danger-full-access",
    "workspace-write",
    "bypassPermissions",
    "--dangerously-skip-permissions",
    "--allow-dangerously-skip-permissions",
    "--full-auto",
    "acceptEdits",
];

/// One agent CLI.
pub trait AgentAdapter {
    /// Short name (`codex`, `claude`, `cursor`).
    fn name(&self) -> &'static str;
    /// Binary names to look for on `PATH`, in order.
    fn binaries(&self) -> &'static [&'static str];
    /// Build the command. `tmp` is a private scratch directory.
    fn build_command(&self, bin: &Path, prompt: &str, tmp: &Path) -> Command;
    /// Extract the explanation from the agent's stdout (and `tmp`).
    fn parse_output(&self, stdout: &str, tmp: &Path) -> anyhow::Result<String>;
}

/// All adapters in `auto` order.
pub fn adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(codex::Codex),
        Box::new(claude::Claude),
        Box::new(cursor::Cursor),
    ]
}

fn adapter_for(choice: AgentChoice) -> Option<Box<dyn AgentAdapter>> {
    match choice {
        AgentChoice::Codex => Some(Box::new(codex::Codex)),
        AgentChoice::Claude => Some(Box::new(claude::Claude)),
        AgentChoice::Cursor => Some(Box::new(cursor::Cursor)),
        AgentChoice::Auto => None,
    }
}

fn find_binary(a: &dyn AgentAdapter, path_env: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    a.binaries()
        .iter()
        .find_map(|b| which::which_in(b, path_env, &cwd).ok())
}

/// Pick the adapter: `--agent` flag → `RULESIFT_AGENT` → config → first of
/// codex, claude, cursor found on `PATH`.
pub fn select(
    flag: Option<AgentChoice>,
    env: Option<&str>,
    cfg: AgentChoice,
    path_env: Option<&std::ffi::OsStr>,
) -> anyhow::Result<(Box<dyn AgentAdapter>, PathBuf)> {
    let env_choice = match env.filter(|s| !s.is_empty()) {
        Some(s) => Some(
            s.parse::<AgentChoice>()
                .map_err(|e| anyhow::anyhow!("RULESIFT_AGENT: {e}"))?,
        ),
        None => None,
    };
    let choice = flag
        .filter(|c| *c != AgentChoice::Auto)
        .or(env_choice.filter(|c| *c != AgentChoice::Auto))
        .unwrap_or(cfg);
    if let Some(a) = adapter_for(choice) {
        let bin = find_binary(a.as_ref(), path_env).ok_or_else(|| {
            anyhow::anyhow!(
                "agent `{}` not found on PATH (looked for {})",
                a.name(),
                a.binaries().join(", ")
            )
        })?;
        return Ok((a, bin));
    }
    for a in adapters() {
        if let Some(bin) = find_binary(a.as_ref(), path_env) {
            return Ok((a, bin));
        }
    }
    anyhow::bail!("no agent CLI found on PATH (looked for codex, claude, agent/cursor-agent); install one or drop --explain")
}

/// Run a command with a timeout; returns stdout.
pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> anyhow::Result<String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let out_t = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = std::io::Read::read_to_string(&mut stdout, &mut s);
        s
    });
    let err_t = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        if let Some(st) = child.try_wait()? {
            break st;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("agent timed out after {} s", timeout.as_secs());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let out = out_t.join().unwrap_or_default();
    let err = err_t.join().unwrap_or_default();
    if !status.success() {
        let tail: String = err
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!("agent exited with {status}: {tail}");
    }
    Ok(out)
}

/// Explanation result.
pub struct Explanation {
    /// Adapter name.
    pub agent: &'static str,
    /// English text from the agent.
    pub text: String,
}

/// Explain the report with the selected agent. Returns `Ok(None)` when there
/// is nothing to explain.
pub fn explain(
    report: &Report,
    cfg: &ExplainConfig,
    flag: Option<AgentChoice>,
) -> anyhow::Result<Option<Explanation>> {
    let Some(prompt) = prompt::build(report, cfg.max_findings) else {
        return Ok(None);
    };
    let env = std::env::var("RULESIFT_AGENT").ok();
    let path_env = std::env::var_os("PATH");
    let (adapter, bin) = select(flag, env.as_deref(), cfg.agent, path_env.as_deref())?;
    let tmp = tempfile::tempdir()?;
    let cmd = adapter.build_command(&bin, &prompt, tmp.path());
    // defense in depth: never launch an agent in a write or bypass mode
    for a in cmd.get_args() {
        let a = a.to_string_lossy();
        if a != prompt.as_str() && FORBIDDEN_FLAGS.iter().any(|f| a.contains(f)) {
            anyhow::bail!("refusing to run {} with {a}", adapter.name());
        }
    }
    let stdout = run_with_timeout(cmd, Duration::from_secs(cfg.timeout_secs.max(1)))?;
    let text = adapter.parse_output(&stdout, tmp.path())?;
    Ok(Some(Explanation {
        agent: adapter.name(),
        text: text.trim().to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_of(c: &Command) -> Vec<String> {
        std::iter::once(c.get_program().to_string_lossy().into_owned())
            .chain(c.get_args().map(|a| a.to_string_lossy().into_owned()))
            .collect()
    }

    #[test]
    fn built_commands_snapshot_and_are_read_only() {
        let tmp = Path::new("/tmp/rulesift-explain");
        let mut out = String::new();
        for a in adapters() {
            let bin = PathBuf::from(a.binaries()[0]);
            let c = a.build_command(&bin, "PROMPT", tmp);
            let args = args_of(&c);
            for f in FORBIDDEN_FLAGS {
                assert!(
                    !args.iter().any(|x| x.contains(f)),
                    "{} passes {f}",
                    a.name()
                );
            }
            out.push_str(&format!("{}: {:?}\n", a.name(), args));
        }
        insta::assert_snapshot!(out);
    }

    #[test]
    fn parse_outputs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(codex::LAST_MESSAGE), "codex says hi\n").unwrap();
        assert_eq!(
            codex::Codex.parse_output("ignored", tmp.path()).unwrap(),
            "codex says hi\n"
        );
        // codex without the file falls back to stdout
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(
            codex::Codex
                .parse_output("from stdout", empty.path())
                .unwrap(),
            "from stdout"
        );
        let j =
            r#"{"type":"result","subtype":"success","result":"claude verdict","is_error":false}"#;
        assert_eq!(
            claude::Claude.parse_output(j, tmp.path()).unwrap(),
            "claude verdict"
        );
        assert!(claude::Claude
            .parse_output(r#"{"is_error":true,"result":"boom"}"#, tmp.path())
            .is_err());
        assert!(claude::Claude.parse_output("not json", tmp.path()).is_err());
        let j = r#"{"type":"result","result":"cursor verdict"}"#;
        assert_eq!(
            cursor::Cursor.parse_output(j, tmp.path()).unwrap(),
            "cursor verdict"
        );
    }

    #[test]
    fn selection_order() {
        let d = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for b in ["claude", "cursor-agent"] {
                let p = d.path().join(b);
                std::fs::write(&p, "#!/bin/sh\n").unwrap();
                std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            let path = Some(d.path().as_os_str());
            // auto: codex missing → claude
            assert_eq!(
                select(None, None, AgentChoice::Auto, path)
                    .unwrap()
                    .0
                    .name(),
                "claude"
            );
            // config
            assert_eq!(
                select(None, None, AgentChoice::Cursor, path)
                    .unwrap()
                    .0
                    .name(),
                "cursor"
            );
            // env beats config
            assert_eq!(
                select(None, Some("claude"), AgentChoice::Cursor, path)
                    .unwrap()
                    .0
                    .name(),
                "claude"
            );
            // flag beats env
            assert_eq!(
                select(
                    Some(AgentChoice::Cursor),
                    Some("claude"),
                    AgentChoice::Auto,
                    path
                )
                .unwrap()
                .0
                .name(),
                "cursor"
            );
            // explicit but missing
            let e = select(Some(AgentChoice::Codex), None, AgentChoice::Auto, path)
                .err()
                .unwrap();
            assert!(e.to_string().contains("codex"));
            assert!(select(None, Some("gpt"), AgentChoice::Auto, path).is_err());
        }
        let empty = tempfile::tempdir().unwrap();
        assert!(select(
            None,
            None,
            AgentChoice::Auto,
            Some(empty.path().as_os_str())
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_the_agent() {
        let mut c = Command::new("sh");
        c.args(["-c", "sleep 5"]);
        let t = Instant::now();
        let e = run_with_timeout(c, Duration::from_secs(1)).unwrap_err();
        assert!(e.to_string().contains("timed out"));
        assert!(t.elapsed() < Duration::from_secs(4));
    }

    /// Real agents, only with `RULESIFT_E2E_AGENT=1`.
    #[test]
    #[ignore]
    fn e2e_real_agent() {
        if std::env::var("RULESIFT_E2E_AGENT").as_deref() != Ok("1") {
            return;
        }
        let r = prompt::tests::sample_report();
        let cfg = ExplainConfig::default();
        let e = explain(&r, &cfg, None).unwrap().unwrap();
        assert!(!e.text.is_empty());
    }
}
