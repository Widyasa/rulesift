//! Cursor agent CLI: `agent -p --output-format json "<prompt>"` (fallback
//! binary `cursor-agent`), field `result`. Never `--force`/`--yolo`
//! (re-checked 29 Oct 2026 at https://cursor.com/docs/cli/headless).

use super::AgentAdapter;
use std::path::Path;
use std::process::Command;

/// Cursor adapter.
pub struct Cursor;

impl AgentAdapter for Cursor {
    fn name(&self) -> &'static str {
        "cursor"
    }
    fn binaries(&self) -> &'static [&'static str] {
        &["agent", "cursor-agent"]
    }
    fn build_command(&self, bin: &Path, prompt: &str, _tmp: &Path) -> Command {
        let mut c = Command::new(bin);
        c.args(["-p", "--output-format", "json"]).arg(prompt);
        c
    }
    fn parse_output(&self, stdout: &str, _tmp: &Path) -> anyhow::Result<String> {
        super::claude::parse_result_json("cursor", stdout)
    }
}
