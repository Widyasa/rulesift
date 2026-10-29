//! Claude Code: `claude -p --output-format json "<prompt>"`, field `result`
//! (flags re-checked 29 Oct 2026 at https://code.claude.com/docs/en/cli-reference).

use super::AgentAdapter;
use std::path::Path;
use std::process::Command;

/// Claude Code adapter.
pub struct Claude;

/// Parse `{"result": "...", "is_error": false}` print-mode JSON.
pub fn parse_result_json(agent: &str, stdout: &str) -> anyhow::Result<String> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| anyhow::anyhow!("{agent} returned invalid JSON: {e}"))?;
    if v["is_error"].as_bool() == Some(true) {
        anyhow::bail!(
            "{agent} reported an error: {}",
            v["result"].as_str().unwrap_or("")
        );
    }
    v["result"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("{agent} JSON has no `result` field"))
}

impl AgentAdapter for Claude {
    fn name(&self) -> &'static str {
        "claude"
    }
    fn binaries(&self) -> &'static [&'static str] {
        &["claude"]
    }
    fn build_command(&self, bin: &Path, prompt: &str, _tmp: &Path) -> Command {
        let mut c = Command::new(bin);
        c.args(["-p", "--output-format", "json", "--no-session-persistence"])
            .arg(prompt);
        c
    }
    fn parse_output(&self, stdout: &str, _tmp: &Path) -> anyhow::Result<String> {
        parse_result_json("claude", stdout)
    }
}
