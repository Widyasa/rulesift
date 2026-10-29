//! `rulesift rules <list|show <ID>|test <DIR>>` (PRD §6.5).

use crate::cli::GlobalArgs;
use anyhow::{bail, Context};
use clap::{Args, Subcommand};
use rulesift_core::config;
use rulesift_core::rules::detectors::unicode;
use rulesift_core::rules::examples;
use rulesift_core::rules::loader::{self, LoadOptions, Matcher, Node, Rule, RuleSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Arguments of `rules`.
#[derive(Debug, Args)]
pub struct RulesArgs {
    /// Subcommand.
    #[command(subcommand)]
    pub cmd: RulesCmd,
}

/// `rules` subcommands.
#[derive(Debug, Subcommand)]
pub enum RulesCmd {
    /// List rules: id, severity, title.
    List {
        /// Extra rule directory (repeatable).
        #[arg(long = "rules", value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },
    /// Show the full documentation of one rule.
    Show {
        /// Rule id, e.g. RS-EXEC-001.
        id: String,
        /// Extra rule directory (repeatable).
        #[arg(long = "rules", value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },
    /// Validate the user rules in DIR and run their inline examples.
    Test {
        /// Directory with `*.yaml` / `*.yml` rule files.
        dir: PathBuf,
    },
}

/// Load built-in rules plus config and command-line rule directories,
/// applying config disables and severity overrides.
pub fn load_rules(g: &GlobalArgs, dirs: &[PathBuf], disable: &[String]) -> anyhow::Result<RuleSet> {
    let env = std::env::var_os("RULESIFT_CONFIG").map(PathBuf::from);
    let (cfg, src) = config::load_resolved(g.config.as_deref(), env.as_deref(), Path::new("."))?;
    let base = src
        .path()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut extra: Vec<PathBuf> = cfg.rules.extra_dirs.iter().map(|d| base.join(d)).collect();
    extra.extend(dirs.iter().cloned());
    let mut dis = cfg.rules.disable.clone();
    dis.extend(disable.iter().cloned());
    let set = loader::load(&LoadOptions {
        builtin: true,
        extra_dirs: extra,
        disable: dis,
        severity_override: cfg.rules.severity_override.clone(),
    })?;
    if g.verbose {
        for w in &set.warnings {
            eprintln!("warning: {w}");
        }
    }
    Ok(set)
}

/// `list` output.
pub fn render_list(set: &RuleSet) -> String {
    let w = set
        .rules
        .iter()
        .map(|r| r.id().len())
        .max()
        .unwrap_or(2)
        .max(2);
    let mut out = format!("{:<w$}  {:<8}  TITLE\n", "ID", "SEVERITY");
    for r in &set.rules {
        let _ = writeln!(
            out,
            "{:<w$}  {:<8}  {}",
            r.id(),
            r.severity.to_string(),
            r.def.title
        );
    }
    out
}

fn describe(
    n: &Node,
    parent: Option<&std::collections::BTreeSet<rulesift_core::model::Scope>>,
    depth: usize,
    out: &mut String,
) {
    let pad = "  ".repeat(depth + 1);
    let scope = n
        .scope
        .as_ref()
        .filter(|s| Some(*s) != parent)
        .map(|s| {
            let v: Vec<String> = s.iter().map(|x| x.to_string()).collect();
            format!(" [scope: {}]", v.join(", "))
        })
        .unwrap_or_default();
    let line = match &n.matcher {
        Matcher::Regex(r) => format!("regex `{}`", r.as_str()),
        Matcher::Phrases(_, p) => format!("phrases ({}): {}", p.len(), p.join(" | ")),
        Matcher::JsonPath(p, v) => format!(
            "json_path /{}{}",
            p.join("/"),
            v.as_ref()
                .map(|r| format!(" value {:?}", r.as_str()))
                .unwrap_or_default()
        ),
        Matcher::Frontmatter(f, c) => format!("frontmatter {f} {c:?}"),
        Matcher::Detector(d) => format!("detector {d}"),
        Matcher::Any(_) => "any of:".into(),
        Matcher::All(_) => "all of:".into(),
        Matcher::Not(_) => "not:".into(),
    };
    let _ = writeln!(out, "{pad}{}{scope}", unicode::render(&line));
    match &n.matcher {
        Matcher::Any(k) | Matcher::All(k) => k
            .iter()
            .for_each(|c| describe(c, n.scope.as_ref(), depth + 1, out)),
        Matcher::Not(c) => describe(c, n.scope.as_ref(), depth + 1, out),
        _ => {}
    }
}

fn wrap(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut line = String::new();
    for w in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + w.chars().count() > width {
            out.push_str(&line);
            out.push('\n');
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(w);
    }
    out.push_str(&line);
    out.push('\n');
    out
}

/// `show` output.
pub fn render_show(r: &Rule) -> String {
    let d = &r.def;
    let mut out = format!("{}: {}\n\n", d.id, d.title);
    let kinds = d
        .applies_to
        .as_ref()
        .map(|k| {
            k.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "all kinds".into());
    let _ = writeln!(out, "Severity:   {}", r.severity);
    let _ = writeln!(out, "Category:   {}", d.category.as_deref().unwrap_or("-"));
    let _ = writeln!(out, "Applies to: {kinds}");
    let _ = writeln!(out, "Source:     {}", r.source.display());
    out.push_str("Matcher:\n");
    describe(&r.node, None, 0, &mut out);
    out.push('\n');
    out.push_str(&wrap(&d.message, 78));
    if let Some(ex) = &d.examples {
        for (title, list) in [
            ("Matches", &ex.should_match),
            ("Does not match", &ex.should_not_match),
        ] {
            if !list.is_empty() {
                let _ = writeln!(out, "\n{title}:");
                for e in list {
                    let _ = writeln!(out, "  - {:?}", unicode::render(e));
                }
            }
        }
    }
    if !d.references.is_empty() {
        out.push_str("\nReferences:\n");
        for u in &d.references {
            let _ = writeln!(out, "  - {u}");
        }
    }
    if d.id.starts_with("RS-") {
        let _ = writeln!(
            out,
            "\nDocs: https://github.com/Widyasa/rulesift/blob/master/docs/rules/{}.md",
            d.id
        );
    }
    out
}

/// Run `rules`.
pub fn run(g: &GlobalArgs, a: &RulesArgs) -> anyhow::Result<ExitCode> {
    match &a.cmd {
        RulesCmd::List { dirs } => {
            print!("{}", render_list(&load_rules(g, dirs, &[])?));
            Ok(ExitCode::SUCCESS)
        }
        RulesCmd::Show { id, dirs } => {
            let set = load_rules(g, dirs, &[])?;
            let Some(r) = set.get(id) else {
                bail!("unknown rule id {id:?} (see `rulesift rules list`)");
            };
            print!("{}", render_show(r));
            Ok(ExitCode::SUCCESS)
        }
        RulesCmd::Test { dir } => {
            if !dir.is_dir() {
                bail!("rule directory {} does not exist", dir.display());
            }
            let rules = loader::load_dir(dir)
                .with_context(|| format!("cannot load rules from {}", dir.display()))?;
            let (out, failed) = render_test(&rules, g.quiet);
            print!("{out}");
            Ok(if failed > 0 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
    }
}

/// `test` output and number of failed examples.
pub fn render_test(rules: &[Rule], quiet: bool) -> (String, usize) {
    let mut out = String::new();
    let (mut total, mut failed) = (0, 0);
    for r in rules {
        let res = examples::run(r);
        if res.is_empty() {
            let _ = writeln!(out, "warn  {}: no examples", r.id());
            continue;
        }
        let bad: Vec<_> = res.iter().filter(|x| !x.passed).collect();
        total += res.len();
        failed += bad.len();
        if bad.is_empty() {
            if !quiet {
                let _ = writeln!(out, "ok    {} ({} examples)", r.id(), res.len());
            }
        } else {
            for b in bad {
                let kind = if b.should_match {
                    "should_match"
                } else {
                    "should_not_match"
                };
                let _ = writeln!(
                    out,
                    "FAIL  {} {kind} {:?}: {}",
                    r.id(),
                    b.text,
                    b.problem.as_deref().unwrap_or("failed")
                );
            }
        }
    }
    let _ = writeln!(
        out,
        "\n{} rules, {total} examples, {failed} failed",
        rules.len()
    );
    (out, failed)
}
