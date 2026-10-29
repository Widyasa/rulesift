//! `rulesift` command-line interface.

#![forbid(unsafe_code)]

use clap::Parser;
use std::process::ExitCode;

/// Offline, zero-cost linter and PR gate for AI agent configuration files.
#[derive(Debug, Parser)]
#[command(name = "rulesift", version, about, long_about = None)]
struct Cli {
    /// Print the built-in tool registry (debug).
    #[arg(long, hide = true)]
    list_tools: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.list_tools {
        print!("{}", rulesift_core::registry::render_table());
    }
    ExitCode::SUCCESS
}
