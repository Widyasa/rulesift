//! Argument definitions and dispatch.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// Offline, zero-cost linter and PR gate for AI agent configuration files.
#[derive(Debug, Parser)]
#[command(name = "rulesift", version, about, long_about = None)]
pub struct Cli {
    /// Global options.
    #[command(flatten)]
    pub global: GlobalArgs,

    /// Print the built-in tool registry (debug).
    #[arg(long, hide = true)]
    pub list_tools: bool,

    /// Command to run.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Options shared by all commands.
#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    /// Config file (default: nearest rulesift.toml walking up from the first path; env RULESIFT_CONFIG).
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
    /// Disable colors (also honors NO_COLOR).
    #[arg(long, global = true)]
    pub no_color: bool,
    /// Print less output (hides OK files, discovery warnings, and unknown rule id warnings).
    #[arg(short, long, global = true)]
    pub quiet: bool,
    /// Print more output (currently no extra output: warnings show by default).
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scan agent files and report findings, scores, and tiers.
    Scan(Box<crate::scan::ScanArgs>),
    /// Show agent capability changes between two revisions and scan changed files.
    Diff(Box<crate::diff::DiffArgs>),
    /// Write a commented rulesift.toml, a CI workflow, or print a pre-commit snippet.
    Init(crate::init::InitArgs),
    /// Start the read-only MCP server over stdio (for your coding agent).
    Mcp,
    /// Write or update rulesift.lock for all agent files.
    Lock(crate::lock::LockArgs),
    /// Verify agent files against rulesift.lock (exit 1 on any difference).
    Verify(crate::lock::VerifyArgs),
    /// Show hidden characters, decoded text, base64 payloads, and HTML comments in a file.
    Reveal(crate::reveal::RevealArgs),
    /// List, show, or test rules.
    Rules(crate::rules::RulesArgs),
}

/// Run the parsed command line.
pub fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    if cli.list_tools {
        print!("{}", rulesift_core::registry::render_table());
        return Ok(ExitCode::SUCCESS);
    }
    match cli.command {
        Some(Command::Scan(a)) => crate::scan::run(&cli.global, &a),
        Some(Command::Diff(a)) => crate::diff::run(&cli.global, &a),
        Some(Command::Init(a)) => crate::init::run(&cli.global, &a),
        Some(Command::Mcp) => crate::mcp::run(&cli.global),
        Some(Command::Lock(a)) => crate::lock::run_lock(&cli.global, &a),
        Some(Command::Verify(a)) => crate::lock::run_verify(&cli.global, &a),
        Some(Command::Reveal(a)) => crate::reveal::run(&cli.global, &a),
        Some(Command::Rules(a)) => crate::rules::run(&cli.global, &a),
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
