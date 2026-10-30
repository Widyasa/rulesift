//! `rulesift diff --base <REF> [--head <REF>] [PATHS...]` (PRD §6.3).

use crate::cli::GlobalArgs;
use crate::scan::{apply_min_severity, emit, exit_code, now_rfc3339, OutputArgs, RenderCtx};
use clap::Args;
use rulesift_core::diff::{diff, DiffOptions};
use rulesift_core::git::GitRepo;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Arguments of `diff`.
#[derive(Debug, Clone, Args)]
pub struct DiffArgs {
    /// Base revision (branch, tag, or commit), e.g. `origin/master`.
    #[arg(long, value_name = "REF")]
    pub base: String,
    /// Head revision (default: the working tree).
    #[arg(long, value_name = "REF")]
    pub head: Option<String>,
    /// Limit to these files or directories.
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub out: OutputArgs,
}

fn repo_prefix(workdir: &Path, p: &Path) -> anyhow::Result<String> {
    let abs = std::fs::canonicalize(p)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default().join(p));
    let wd = std::fs::canonicalize(workdir)?;
    let rel = abs.strip_prefix(&wd).map_err(|_| {
        anyhow::anyhow!("{} is outside the repository {}", p.display(), wd.display())
    })?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

/// Run `diff`.
pub fn run(g: &GlobalArgs, a: &DiffArgs) -> anyhow::Result<ExitCode> {
    let start = a
        .paths
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = if start.is_file() {
        start
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        start.clone()
    };
    let repo = GitRepo::discover(&dir)?;
    let workdir = repo.workdir().to_path_buf();
    // scan options and rules as for `scan`, but the policy (config, lock)
    // comes from the base revision: a change cannot weaken the gate that
    // checks it (#160)
    let dir_rel = repo_prefix(&workdir, &dir)?;
    let crate::policy::DiffPolicy {
        prepared: mut p,
        lock,
    } = crate::policy::prepare_diff(g, &repo, &dir, &dir_rel, &a.base, a.head.as_deref(), &a.out)?;
    p.opts.root = ".".into();
    let mut prefixes = Vec::new();
    for x in &a.paths {
        prefixes.push(repo_prefix(&workdir, x)?);
    }
    let out = diff(
        &repo,
        &p.rules,
        &DiffOptions {
            base: a.base.clone(),
            head: a.head.clone(),
            prefixes,
        },
        &p.opts,
        lock,
    )?;
    let mut report = out.report;
    // discovery warnings the change introduced, as `scan` prints them
    if !g.quiet {
        for w in &report.warnings {
            eprintln!("warning: {}: {}", w.path, w.message);
        }
    }
    report.generated_at = Some(now_rfc3339());
    apply_min_severity(&mut report, p.min_severity);
    let code = exit_code(&report, p.fail_on)
        .max(u8::from(crate::policy::lock_fails(report.lock.as_ref())));
    emit(
        g,
        &a.out,
        &report,
        &RenderCtx {
            code,
            rules: &p.rules.rules,
        },
    )?;
    Ok(ExitCode::from(code))
}
