//! `rulesift reveal <FILE> [--strip]` (PRD §6.2).

use crate::cli::GlobalArgs;
use anyhow::Context;
use clap::Args;
use rulesift_core::reveal::{reveal, RevealedText};
use rulesift_core::rules::detectors::unicode;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

/// Arguments of `reveal`.
#[derive(Debug, Args)]
pub struct RevealArgs {
    /// File to inspect.
    pub file: PathBuf,
    /// Print the file to stdout with Unicode Tag, bidi, and zero-width characters removed (never edits the file).
    #[arg(long)]
    pub strip: bool,
}

/// Render the reveal report as plain text.
pub fn render(path: &str, text: &str) -> String {
    let r = reveal(text);
    let mut out = String::new();
    let _ = writeln!(out, "RuleSift reveal: {path}");
    if r.is_empty() {
        let _ = writeln!(
            out,
            "\nNo hidden characters, encoded payloads, or HTML comments found."
        );
        return out;
    }
    if !r.hidden.is_empty() {
        let _ = writeln!(out, "\nHidden characters ({} runs):", r.hidden.len());
        for h in &r.hidden {
            let more = if h.count > h.code_points.len() {
                " …"
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "  {:>7}  {:<18} x{:<4} {}{}",
                format!("{}:{}", h.line, h.column),
                h.category,
                h.count,
                h.code_points.join(" "),
                more
            );
        }
    }
    let section = |out: &mut String, title: &str, items: &[RevealedText]| {
        if items.is_empty() {
            return;
        }
        let _ = writeln!(out, "\n{title}:");
        for i in items {
            let _ = writeln!(
                out,
                "  {:>7}  {:?}",
                format!("{}:{}", i.line, i.column),
                i.text
            );
        }
    };
    section(&mut out, "Decoded Unicode Tag text", &r.tag_text);
    section(&mut out, "Decoded base64 payloads", &r.base64);
    section(&mut out, "HTML comments", &r.comments);
    out
}

/// Run `reveal`.
pub fn run(_g: &GlobalArgs, a: &RevealArgs) -> anyhow::Result<ExitCode> {
    let bytes =
        std::fs::read(&a.file).with_context(|| format!("cannot read {}", a.file.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let mut stdout = std::io::stdout().lock();
    if a.strip {
        stdout.write_all(unicode::strip(&text).as_bytes())?;
    } else {
        let path = a.file.to_string_lossy().replace('\\', "/");
        stdout.write_all(render(&path, &text).as_bytes())?;
    }
    Ok(ExitCode::SUCCESS)
}
