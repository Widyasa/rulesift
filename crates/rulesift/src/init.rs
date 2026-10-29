//! `rulesift init` (PRD §6.6): config, CI workflow, pre-commit snippet.

use crate::cli::GlobalArgs;
use anyhow::{bail, Context};
use clap::{Args, ValueEnum};
use rulesift_core::VERSION;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// CI providers for `--ci`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Ci {
    /// GitHub Actions (`.github/workflows/rulesift.yml`).
    Github,
}

/// Arguments of `init`.
#[derive(Debug, Clone, Args)]
pub struct InitArgs {
    /// Directory to initialize (default `.`).
    pub dir: Option<PathBuf>,
    /// Overwrite existing files.
    #[arg(long)]
    pub force: bool,
    /// Write a CI workflow instead of the config.
    #[arg(long, value_enum)]
    pub ci: Option<Ci>,
    /// Print the `.pre-commit-config.yaml` snippet.
    #[arg(long)]
    pub pre_commit: bool,
    /// Print this repository's generated `.pre-commit-hooks.yaml` (maintainers).
    #[arg(long, hide = true)]
    pub print_hooks: bool,
}

/// Commented default `rulesift.toml` (PRD §7.1).
pub fn config_template() -> String {
    r#"# rulesift.toml — RuleSift configuration.
# Docs: https://github.com/Widyasa/rulesift/blob/master/docs/configuration.md
# Unknown keys are an error.

[scan]
# Files or directories to scan when no PATHS are given.
paths = ["."]
# Globs (relative to the scanned directory) to skip.
exclude = ["node_modules/**", "target/**"]
# File kinds to scan: skill, instructions, command, agent, mcp-config, hooks, settings, plugin.
kinds = ["skill", "instructions", "command", "agent", "mcp-config", "hooks", "settings", "plugin"]
# Tools to scan; empty = all (e.g. ["claude-code", "cursor", "gemini-cli"]).
tools = []
max_file_size_kib = 2048

[policy]
# Exit 1 when any file reaches this tier: block | review | info | never.
fail_on = "block"
# Hide findings below this severity (scores still use all findings).
min_severity = "low"

[score]
critical = 70
high = 25
medium = 10
low = 3
always_loaded_multiplier = 1.5
toxic_combo_bonus = 20
block_at = 70
review_at = 40

[rules]
# Extra directories with YAML rules.
extra_dirs = []
# Rule ids to disable, e.g. ["RS-EXF-003"].
disable = []

[rules.severity_override]
# "RS-SEC-003" = "low"

# Scoped suppressions; each needs a reason.
# [[suppress]]
# rule = "RS-EXEC-001"
# path = "skills/installer/SKILL.md"
# reason = "Documented installer; script is vendored and pinned."

[explain]
# Optional AI explanation with your own agent CLI: auto | codex | claude | cursor.
agent = "auto"
timeout_secs = 120
max_findings = 10
"#
    .to_string()
}

/// GitHub workflow written by `init --ci github` (PRD §12.1).
pub fn github_workflow() -> String {
    format!(
        r#"name: RuleSift
on: [pull_request, push]
permissions: {{ contents: read, security-events: write, pull-requests: write }}
jobs:
  rulesift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with: {{ fetch-depth: 0 }}
      - uses: Widyasa/rulesift@v{VERSION}
"#
    )
}

/// `.pre-commit-config.yaml` snippet printed by `init --pre-commit`.
pub fn pre_commit_snippet() -> String {
    format!(
        r#"# Add to .pre-commit-config.yaml
repos:
  - repo: https://github.com/Widyasa/rulesift
    rev: v{VERSION}
    hooks:
      - id: rulesift
"#
    )
}

/// The repository's `.pre-commit-hooks.yaml` (PRD §12.2), with the `files`
/// regex generated from the tool registry.
pub fn pre_commit_hooks() -> String {
    let re = rulesift_core::registry::precommit_files_regex().replace('\'', "''");
    format!(
        r#"# Generated from the RuleSift tool registry; do not edit by hand.
# Regenerate with: RULESIFT_BLESS=1 cargo test -p rulesift --test init_cmd
- id: rulesift
  name: RuleSift
  description: Lint AI agent configuration files (skills, rules, MCP configs, hooks, settings).
  language: rust
  entry: rulesift scan --fail-on block
  pass_filenames: true
  files: '{re}'
# Same hook using a rulesift binary already on PATH (cargo install, brew,
# npm, pip, or a release download).
- id: rulesift-system
  name: RuleSift (installed binary)
  description: Lint AI agent configuration files with the rulesift binary on PATH.
  language: system
  entry: rulesift scan --fail-on block
  pass_filenames: true
  files: '{re}'
"#
    )
}

fn write_new(path: &Path, text: &str, force: bool, quiet: bool) -> anyhow::Result<()> {
    if path.exists() && !force {
        bail!(
            "{} already exists; use --force to overwrite",
            path.display()
        );
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))?;
    if !quiet {
        println!("Wrote {}", path.display());
    }
    Ok(())
}

/// Run `init`.
pub fn run(g: &GlobalArgs, a: &InitArgs) -> anyhow::Result<ExitCode> {
    let dir = a.dir.clone().unwrap_or_else(|| PathBuf::from("."));
    if a.print_hooks {
        print!("{}", pre_commit_hooks());
        return Ok(ExitCode::SUCCESS);
    }
    if a.pre_commit {
        print!("{}", pre_commit_snippet());
    }
    if let Some(Ci::Github) = a.ci {
        write_new(
            &dir.join(".github/workflows/rulesift.yml"),
            &github_workflow(),
            a.force,
            g.quiet,
        )?;
    }
    if !a.pre_commit && a.ci.is_none() {
        write_new(
            &dir.join("rulesift.toml"),
            &config_template(),
            a.force,
            g.quiet,
        )?;
    }
    Ok(ExitCode::SUCCESS)
}
