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
    /// Print less output.
    #[arg(short, long, global = true)]
    pub quiet: bool,
    /// Print more output (warnings and skipped files).
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scan agent files and report findings, scores, and tiers.
    Scan(Box<crate::scan::ScanArgs>),
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
        Some(Command::Reveal(a)) => crate::reveal::run(&cli.global, &a),
        Some(Command::Rules(a)) => crate::rules::run(&cli.global, &a),
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
