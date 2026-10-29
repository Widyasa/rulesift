//! Shared CLI helpers: config and rule loading, colors.

use crate::cli::GlobalArgs;
use rulesift_core::config::{self, Config};
use rulesift_core::rules::loader::{self, LoadOptions, RuleSet};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// Resolve and load the config (`--config` > `RULESIFT_CONFIG` > walk up
/// from `start`). Returns the config and the directory relative paths in it
/// resolve against.
pub fn load_config(g: &GlobalArgs, start: &Path) -> anyhow::Result<(Config, PathBuf)> {
    let env = std::env::var_os("RULESIFT_CONFIG").map(PathBuf::from);
    let (cfg, src) = config::load_resolved(g.config.as_deref(), env.as_deref(), start)?;
    let base = src
        .path()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_default();
    Ok((cfg, base))
}

/// Load built-in rules plus config and command-line rule directories,
/// applying disables and severity overrides.
pub fn build_rules(
    g: &GlobalArgs,
    cfg: &Config,
    base: &Path,
    dirs: &[PathBuf],
    disable: &[String],
) -> anyhow::Result<RuleSet> {
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

/// Whether stdout output should be colored.
pub fn use_color(g: &GlobalArgs) -> bool {
    !g.no_color
        && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && std::io::stdout().is_terminal()
}
