//! `rulesift lock` / `rulesift verify` (PRD §6.4, §7.2).

use crate::cli::GlobalArgs;
use crate::common;
use clap::Args;
use rulesift_core::discovery::{discover, DiscoveryOptions};
use rulesift_core::lock::{self, Lockfile, LOCK_FILE};
use rulesift_core::model::{LockStatus, LockViolation};
use rulesift_core::scan::analyze_file;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Arguments of `lock`.
#[derive(Debug, Clone, Args)]
pub struct LockArgs {
    /// Directory to lock (default `.`); `rulesift.lock` is written there.
    pub path: Option<PathBuf>,
    /// Verify instead of writing (same as `rulesift verify`).
    #[arg(long)]
    pub check: bool,
}

/// Arguments of `verify`.
#[derive(Debug, Clone, Args)]
pub struct VerifyArgs {
    /// Directory containing `rulesift.lock` (default `.`).
    pub path: Option<PathBuf>,
}

/// Compute the current lock for `root` with the config's discovery options.
pub fn current(g: &GlobalArgs, root: &Path) -> anyhow::Result<Lockfile> {
    if !root.is_dir() {
        anyhow::bail!("{} is not a directory", root.display());
    }
    let (cfg, _) = common::load_config(g, root)?;
    current_with(&DiscoveryOptions::from_config(&cfg.scan), root)
}

/// Compute the current lock for `root` with the given discovery options.
pub fn current_with(opts: &DiscoveryOptions, root: &Path) -> anyhow::Result<Lockfile> {
    let d = discover(&[root.to_path_buf()], opts)?;
    let analyzed: Vec<_> = d.files.iter().map(analyze_file).collect();
    Ok(lock::compute(root, &d.files, &analyzed)?)
}

/// Verify the lock under `root`: `None` if no lockfile exists.
pub fn check(g: &GlobalArgs, root: &Path) -> anyhow::Result<Option<Vec<LockViolation>>> {
    let Some(locked) = lock::read(&root.join(LOCK_FILE))? else {
        return Ok(None);
    };
    Ok(Some(lock::verify(&locked, &current(g, root)?)))
}

/// Lock state of `root` for a report: `present: false` without a lockfile.
pub fn status(g: &GlobalArgs, root: &Path) -> anyhow::Result<LockStatus> {
    Ok(match check(g, root)? {
        None => LockStatus::default(),
        Some(violations) => LockStatus {
            present: true,
            violations,
        },
    })
}

/// Lock state for `scan` (#106): checked when the scan covers exactly one
/// directory, `None` (not checked) for files or several paths.
pub fn scan_status(g: &GlobalArgs, paths: &[PathBuf]) -> anyhow::Result<Option<LockStatus>> {
    match paths {
        [p] if p.is_dir() => Ok(Some(status(g, p)?)),
        _ => Ok(None),
    }
}

fn verify_cmd(g: &GlobalArgs, root: &Path) -> anyhow::Result<ExitCode> {
    match check(g, root)? {
        None => anyhow::bail!(
            "no {LOCK_FILE} in {}; run `rulesift lock` first",
            root.display()
        ),
        Some(v) if v.is_empty() => {
            if !g.quiet {
                println!("Lock verified: {LOCK_FILE} matches all agent files.");
            }
            Ok(ExitCode::SUCCESS)
        }
        Some(v) => {
            for x in &v {
                let sev = match x.rule_id.as_str() {
                    "RS-LOCK-001" => "high",
                    "RS-LOCK-002" => "medium",
                    _ => "low",
                };
                println!("{sev:<8}  {}  {}", x.rule_id, x.message);
            }
            println!(
                "\nLock verification failed: {} difference{}  ·  exit code 1",
                v.len(),
                if v.len() == 1 { "" } else { "s" }
            );
            Ok(ExitCode::from(1))
        }
    }
}

/// Run `lock`.
pub fn run_lock(g: &GlobalArgs, a: &LockArgs) -> anyhow::Result<ExitCode> {
    let root = a.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if a.check {
        return verify_cmd(g, &root);
    }
    let l = current(g, &root)?;
    let path = root.join(LOCK_FILE);
    lock::write(&path, &l)?;
    if !g.quiet {
        println!(
            "Wrote {}: {} files, {} skills, {} MCP servers.",
            path.display(),
            l.file.len(),
            l.skill.len(),
            l.mcp_server.len()
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// Run `verify`.
pub fn run_verify(g: &GlobalArgs, a: &VerifyArgs) -> anyhow::Result<ExitCode> {
    verify_cmd(g, &a.path.clone().unwrap_or_else(|| PathBuf::from(".")))
}
