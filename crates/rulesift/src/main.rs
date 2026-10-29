//! `rulesift` command-line interface.

#![forbid(unsafe_code)]

mod cli;
mod common;
mod diff;
mod lock;
mod reveal;
mod rules;
mod scan;
mod table;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    match cli::run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}
