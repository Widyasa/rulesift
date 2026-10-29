//! `rulesift scan [PATHS...]` (PRD §6.1).

use crate::cli::GlobalArgs;
use crate::common;
use anyhow::{bail, Context};
use clap::{Args, ValueEnum};
use rulesift_core::baseline::Baseline;
use rulesift_core::config::FailOn;
use rulesift_core::discovery::DiscoveryOptions;
use rulesift_core::model::{FileKind, Report, Severity, Tool};
use rulesift_core::scan::{scan, ScanOptions, ScanOutput};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Human-readable table.
    Table,
    /// JSON report (schema_version 1).
    Json,
    /// SARIF 2.1.0.
    Sarif,
    /// Markdown (PR comment).
    Markdown,
}

fn parse_enum<T: std::str::FromStr<Err = String>>(s: &str) -> Result<T, String> {
    s.parse::<T>()
}

/// Arguments of `scan` (also reused by `diff`).
#[derive(Debug, Clone, Args)]
pub struct ScanArgs {
    /// Files or directories to scan (default: `[scan].paths` or `.`).
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub out: OutputArgs,
}

/// Output and policy options shared by `scan` and `diff`.
#[derive(Debug, Clone, Args)]
pub struct OutputArgs {
    /// stdout format.
    #[arg(long, value_enum, default_value_t = Format::Table)]
    pub format: Format,
    /// Also write the JSON report to FILE.
    #[arg(long, value_name = "FILE")]
    pub json: Option<PathBuf>,
    /// Also write SARIF to FILE.
    #[arg(long, value_name = "FILE")]
    pub sarif: Option<PathBuf>,
    /// Also write the Markdown PR comment to FILE.
    #[arg(long, value_name = "FILE")]
    pub markdown: Option<PathBuf>,
    /// Exit 1 if any file reaches this tier (default: `[policy].fail_on` or block).
    #[arg(long, value_name = "TIER", value_parser = parse_enum::<FailOn>)]
    pub fail_on: Option<FailOn>,
    /// Hide findings below this severity (scoring still uses all).
    #[arg(long, value_name = "SEV", value_parser = parse_enum::<Severity>)]
    pub min_severity: Option<Severity>,
    /// Limit to file kinds (comma-separated).
    #[arg(long, value_delimiter = ',', value_parser = parse_enum::<FileKind>)]
    pub kinds: Vec<FileKind>,
    /// Limit to tools (comma-separated, e.g. cursor,gemini-cli).
    #[arg(long, value_delimiter = ',', value_parser = parse_enum::<Tool>)]
    pub tools: Vec<Tool>,
    /// Only scan files matching GLOB (repeatable).
    #[arg(long, value_name = "GLOB")]
    pub include: Vec<String>,
    /// Skip files matching GLOB (repeatable).
    #[arg(long, value_name = "GLOB")]
    pub exclude: Vec<String>,
    /// Extra rule directory (repeatable).
    #[arg(long = "rules", value_name = "DIR")]
    pub rules: Vec<PathBuf>,
    /// Disable a rule (repeatable).
    #[arg(long, value_name = "ID")]
    pub disable: Vec<String>,
    /// Suppress findings whose fingerprints are in FILE.
    #[arg(long, value_name = "FILE")]
    pub baseline: Option<PathBuf>,
    /// Write current fingerprints to FILE and exit 0.
    #[arg(long, value_name = "FILE")]
    pub write_baseline: Option<PathBuf>,
}

/// A prepared scan: options, rules, and policy.
pub struct Prepared {
    /// Core options.
    pub opts: ScanOptions,
    /// Rules.
    pub rules: rulesift_core::rules::loader::RuleSet,
    /// Fail threshold.
    pub fail_on: FailOn,
    /// Minimum severity shown.
    pub min_severity: Severity,
}

/// Resolve config, rules, and options.
pub fn prepare(g: &GlobalArgs, paths: &[PathBuf], o: &OutputArgs) -> anyhow::Result<Prepared> {
    let start = paths.first().cloned().unwrap_or_else(|| PathBuf::from("."));
    let (cfg, base) = common::load_config(g, &start)?;
    let rules = common::build_rules(g, &cfg, &base, &o.rules, &o.disable)?;
    let paths: Vec<PathBuf> = if paths.is_empty() {
        let cwd = std::env::current_dir().unwrap_or_default();
        cfg.scan
            .paths
            .iter()
            .map(|p| {
                let full = base.join(p);
                // keep paths relative to the working directory when possible
                match full.strip_prefix(&cwd) {
                    Ok(r) if r.as_os_str().is_empty() || r == Path::new(".") => PathBuf::from("."),
                    Ok(r) => r.to_path_buf(),
                    Err(_) => full,
                }
            })
            .collect()
    } else {
        paths.to_vec()
    };
    for p in &paths {
        if !p.exists() {
            bail!("path {} does not exist", p.display());
        }
    }
    let mut d = DiscoveryOptions::from_config(&cfg.scan);
    d.include.extend(o.include.iter().cloned());
    d.exclude.extend(o.exclude.iter().cloned());
    if !o.kinds.is_empty() {
        d.kinds = o.kinds.clone();
    }
    if !o.tools.is_empty() {
        d.tools = o.tools.clone();
    }
    let baseline = match &o.baseline {
        Some(b) => Some(Baseline::load(b)?),
        None => None,
    };
    let root = if paths.len() == 1 {
        paths[0].to_string_lossy().replace('\\', "/")
    } else {
        ".".into()
    };
    Ok(Prepared {
        opts: ScanOptions {
            paths,
            root,
            discovery: d,
            score: cfg.score.clone(),
            suppress: cfg.suppress.clone(),
            baseline,
        },
        rules,
        fail_on: o.fail_on.unwrap_or(cfg.policy.fail_on),
        min_severity: o.min_severity.unwrap_or(cfg.policy.min_severity),
    })
}

/// Hide findings below `min` (after scoring) and recompute counts.
pub fn apply_min_severity(r: &mut Report, min: Severity) {
    for f in &mut r.files {
        f.findings.retain(|x| x.severity >= min);
    }
    let (rs, rt) = (r.summary.repo_score, r.summary.repo_tier);
    r.summary = Report::summarize(&r.files);
    r.summary.repo_score = rs;
    r.summary.repo_tier = rt;
}

/// Exit code for a report.
pub fn exit_code(r: &Report, fail_on: FailOn) -> u8 {
    u8::from(r.files.iter().any(|f| fail_on.trips(f.tier)))
}

/// What a renderer needs besides the report.
pub struct RenderCtx<'a> {
    /// Exit code shown in the footer.
    pub code: u8,
    /// Loaded rules (SARIF rule metadata).
    pub rules: &'a [rulesift_core::rules::loader::Rule],
}

/// Render `format` for a report.
pub fn render(
    format: Format,
    r: &Report,
    ctx: &RenderCtx<'_>,
    color: bool,
    quiet: bool,
) -> anyhow::Result<String> {
    match format {
        Format::Table => Ok(crate::table::render(r, ctx.code, color, quiet)),
        Format::Json => Ok(rulesift_core::report::json::to_json(r)),
        Format::Sarif => Ok(rulesift_core::report::sarif::to_sarif(r, ctx.rules)),
        Format::Markdown => Ok(rulesift_core::report::markdown::to_markdown(r)),
    }
}

fn write_file(path: &Path, text: &str) -> anyhow::Result<()> {
    std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
}

/// Write `--json/--sarif/--markdown` files and print stdout.
pub fn emit(g: &GlobalArgs, o: &OutputArgs, r: &Report, ctx: &RenderCtx<'_>) -> anyhow::Result<()> {
    for (path, fmt) in [
        (&o.json, Format::Json),
        (&o.sarif, Format::Sarif),
        (&o.markdown, Format::Markdown),
    ] {
        if let Some(p) = path {
            write_file(p, &render(fmt, r, ctx, false, false)?)?;
        }
    }
    let color = o.format == Format::Table && common::use_color(g);
    print!("{}", render(o.format, r, ctx, color, g.quiet)?);
    Ok(())
}

/// Run `scan`.
pub fn run(g: &GlobalArgs, a: &ScanArgs) -> anyhow::Result<ExitCode> {
    let p = prepare(g, &a.paths, &a.out)?;
    let ScanOutput {
        mut report,
        warnings,
        ..
    } = scan(&p.rules, &p.opts)?;
    if g.verbose {
        for w in &warnings {
            eprintln!("warning: {}: {}", w.path, w.message);
        }
    }
    if let Some(path) = &a.out.write_baseline {
        let b = Baseline::from_findings(report.files.iter().flat_map(|f| f.findings.iter()));
        b.write(path)?;
        if !g.quiet {
            eprintln!(
                "wrote {} fingerprints to {}",
                b.findings.len(),
                path.display()
            );
        }
        return Ok(ExitCode::SUCCESS);
    }
    report.generated_at = Some(now_rfc3339());
    apply_min_severity(&mut report, p.min_severity);
    let code = exit_code(&report, p.fail_on);
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

/// Current UTC time as RFC 3339 (seconds), without extra crates.
pub fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}
