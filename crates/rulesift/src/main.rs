//! `rulesift` command-line interface.

#![forbid(unsafe_code)]

use clap::Parser;

/// Offline, zero-cost linter and PR gate for AI agent configuration files.
#[derive(Debug, Parser)]
#[command(name = "rulesift", version, about, long_about = None)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
