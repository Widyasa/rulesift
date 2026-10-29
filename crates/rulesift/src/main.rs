//! `rulesift` command-line interface.

#![forbid(unsafe_code)]

mod cli;
mod common;
mod diff;
mod explain;
mod init;
mod lock;
mod mcp;
mod reveal;
mod rules;
mod scan;
mod table;

use clap::Parser;
use std::process::ExitCode;

/// The error and its causes, joined with `: `, skipping a cause whose text
/// is already part of the message (`cannot read X: <os error>` wraps the OS
/// error as its source, #120).
fn message(e: &anyhow::Error) -> String {
    let mut out = String::new();
    for cause in e.chain() {
        let s = cause.to_string();
        if out.contains(&s) {
            continue;
        }
        if !out.is_empty() {
            out.push_str(": ");
        }
        out.push_str(&s);
    }
    out
}

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    match cli::run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {}", message(&e));
            ExitCode::from(2)
        }
    }
}
