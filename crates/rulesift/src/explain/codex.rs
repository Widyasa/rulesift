//! OpenAI Codex CLI: `codex exec --sandbox read-only -o <tmp>/last.md "<prompt>"`
//! (flags re-checked 29 Oct 2026 at https://developers.openai.com/codex/noninteractive).

use super::AgentAdapter;
use std::path::Path;
use std::process::Command;

/// File the final message is written to.
pub const LAST_MESSAGE: &str = "last.md";

/// Codex adapter.
pub struct Codex;

impl AgentAdapter for Codex {
    fn name(&self) -> &'static str {
        "codex"
    }
    fn binaries(&self) -> &'static [&'static str] {
        &["codex"]
    }
    fn build_command(&self, bin: &Path, prompt: &str, tmp: &Path) -> Command {
        let mut c = Command::new(bin);
        c.args([
            "exec",
            "--sandbox",
            "read-only",
            "--ephemeral",
            "--skip-git-repo-check",
            "-o",
        ])
        .arg(tmp.join(LAST_MESSAGE))
        .arg(prompt);
        c
    }
    fn parse_output(&self, stdout: &str, tmp: &Path) -> anyhow::Result<String> {
        match std::fs::read_to_string(tmp.join(LAST_MESSAGE)) {
            Ok(s) if !s.trim().is_empty() => Ok(s),
            _ if !stdout.trim().is_empty() => Ok(stdout.to_string()),
            _ => anyhow::bail!("codex returned no message"),
        }
    }
}
