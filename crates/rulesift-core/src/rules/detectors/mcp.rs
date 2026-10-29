//! Detectors over the normalized MCP server and hook models (RS-MCP-001,
//! 002, 003, 004, 006 and RS-CFG-002), so each rule works for every tool.

use super::entropy::{is_placeholder, looks_random, redact};
use super::{truncate, unicode, Loc};
use crate::analyze::Analyzed;
use crate::formats::hooks::Hook;
use crate::formats::mcp::McpServer;
use crate::rules::loader::Rule;
use crate::text::LineIndex;
use std::sync::OnceLock;

/// All MCP servers of a file (MCP configs and plugin manifests).
pub fn servers(a: &Analyzed) -> Vec<&McpServer> {
    let mut v: Vec<&McpServer> = Vec::new();
    if let Some(m) = &a.mcp {
        v.extend(m.servers.iter());
    }
    if let Some(p) = &a.plugin {
        v.extend(p.mcp.servers.iter());
    }
    v
}

/// All hooks of a file (hooks files, settings, plugin manifests).
pub fn hooks(a: &Analyzed) -> Vec<&Hook> {
    let mut v: Vec<&Hook> = Vec::new();
    if let Some(h) = &a.hooks {
        v.extend(h.hooks.iter());
    }
    if let Some(p) = &a.plugin {
        v.extend(p.hooks.hooks.iter());
    }
    v
}

fn loc(li: &LineIndex, line: usize, evidence: String, message: String) -> Loc {
    let mut l = Loc::at_line(li, line, truncate(&unicode::render(&evidence), 200));
    l.message = Some(message);
    l
}

fn field_line(s: &McpServer, f: &str) -> usize {
    s.field_lines.get(f).copied().unwrap_or(s.line)
}

fn basename(cmd: &str) -> String {
    let b = cmd.rsplit(['/', '\\']).next().unwrap_or(cmd);
    b.to_ascii_lowercase().trim_end_matches(".exe").to_string()
}

fn command_line(s: &McpServer) -> String {
    let mut v = vec![s.command.clone().unwrap_or_default()];
    v.extend(s.args.iter().cloned());
    v.join(" ")
}

/// RS-MCP-001: the server command runs a shell with an inline script.
pub fn mcp_shell(a: &Analyzed) -> Vec<Loc> {
    let li = LineIndex::new(&a.doc.text);
    let mut out = Vec::new();
    for s in servers(a) {
        let Some(cmd) = &s.command else { continue };
        let mut words: Vec<String> = cmd.split_whitespace().map(str::to_string).collect();
        words.extend(s.args.iter().cloned());
        let hit = words.windows(2).any(|w| {
            let b = basename(&w[0]);
            let flag = w[1].to_ascii_lowercase();
            (matches!(b.as_str(), "sh" | "bash" | "zsh" | "dash" | "ksh" | "fish")
                && (flag == "-c"
                    || flag.starts_with('-') && flag.ends_with('c') && flag.len() <= 4))
                || (b == "cmd" && (flag == "/c" || flag == "/k"))
                || (matches!(b.as_str(), "powershell" | "pwsh")
                    && matches!(
                        flag.as_str(),
                        "-c" | "-command" | "/c" | "-encodedcommand" | "-enc" | "-e"
                    ))
        });
        if hit {
            out.push(loc(
                &li,
                field_line(s, "command"),
                format!("{}: {}", s.name, command_line(s)),
                format!(
                    "MCP server `{}` starts a shell with an inline script; the script can do anything and changes are easy to hide. Run the server binary directly.",
                    s.name
                ),
            ));
        }
    }
    out
}

fn secret_name(k: &str) -> bool {
    let u = k.to_ascii_uppercase();
    [
        "TOKEN",
        "KEY",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "AUTH",
        "CREDENTIAL",
        "COOKIE",
        "SESSION",
    ]
    .iter()
    .any(|w| u.contains(w))
}

fn env_reference(v: &str) -> bool {
    let t = v.trim();
    is_placeholder(t) || t.starts_with("${") || t.starts_with("{env:") || t.contains("${")
}

/// RS-MCP-002: literal secret in `env` or `headers`.
pub fn mcp_literal_secret(a: &Analyzed) -> Vec<Loc> {
    let li = LineIndex::new(&a.doc.text);
    let mut out = Vec::new();
    for s in servers(a) {
        for (field, map) in [("env", &s.env), ("headers", &s.headers)] {
            for (k, v) in map {
                let val = v
                    .trim()
                    .strip_prefix("Bearer ")
                    .or_else(|| v.trim().strip_prefix("bearer "))
                    .or_else(|| v.trim().strip_prefix("Basic "))
                    .unwrap_or(v.trim());
                if env_reference(val) {
                    continue;
                }
                let known = !super::secrets::find(val).is_empty();
                let secretish = secret_name(k) && val.chars().count() >= 8;
                if known || looks_random(val) || secretish {
                    out.push(loc(
                        &li,
                        field_line(s, field),
                        format!("{}.{field}.{k} = {}", s.name, redact(val)),
                        format!(
                            "MCP server `{}` has a literal secret in {field} `{k}` ({}). Use a reference such as `${{{k}}}` and keep the value out of the repo.",
                            s.name,
                            redact(val)
                        ),
                    ));
                }
            }
        }
    }
    out
}

fn npm_pinned(spec: &str) -> bool {
    let (scope_less, _) = match spec.strip_prefix('@') {
        Some(r) => (r.split_once('/').map(|(_, n)| n).unwrap_or(r), true),
        None => (spec, false),
    };
    match scope_less.rsplit_once('@') {
        Some((_, ver)) => !ver.is_empty() && !matches!(ver, "latest" | "next" | "*" | "x"),
        None => false,
    }
}

fn first_positional<'a>(args: &'a [String], with_value: &[&str]) -> Option<&'a str> {
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" {
            return args.get(i + 1).map(String::as_str);
        }
        if a.starts_with('-') {
            let name = a.split('=').next().unwrap_or(a);
            i += if with_value.contains(&name) && !a.contains('=') {
                2
            } else {
                1
            };
            continue;
        }
        return Some(a);
    }
    None
}

/// Why a runner invocation is unpinned, if it is.
pub fn unpinned(command: &str, args: &[String]) -> Option<String> {
    let mut words: Vec<String> = command.split_whitespace().map(str::to_string).collect();
    words.extend(args.iter().cloned());
    let b = basename(words.first()?);
    let rest = &words[1..];
    match b.as_str() {
        "npx" | "bunx" | "pnpx" => {
            if let Some(p) = rest.iter().position(|w| w == "-p" || w == "--package") {
                let spec = rest.get(p + 1)?;
                return (!npm_pinned(spec)).then(|| format!("{b} package {spec} has no version"));
            }
            let spec = first_positional(
                rest,
                &["--registry", "--cache", "--userconfig", "-c", "--call"],
            )?;
            (!npm_pinned(spec)).then(|| format!("{b} runs {spec} without a pinned version"))
        }
        "pnpm" | "yarn" | "npm" => {
            let i = rest.iter().position(|w| w == "dlx" || w == "exec")?;
            let spec = first_positional(&rest[i + 1..], &["--package", "-p"])?;
            (!npm_pinned(spec)).then(|| format!("{b} dlx runs {spec} without a pinned version"))
        }
        "uvx" | "pipx" => {
            let rest: &[String] = if b == "pipx" {
                let i = rest.iter().position(|w| w == "run")?;
                &rest[i + 1..]
            } else {
                rest
            };
            if let Some(p) = rest.iter().position(|w| w == "--from") {
                let spec = rest.get(p + 1)?;
                return (!spec.contains("==") && !spec.contains('@'))
                    .then(|| format!("{b} --from {spec} has no version"));
            }
            let spec = first_positional(rest, &["--python", "--with", "--index-url", "-p"])?;
            (!spec.contains("==") && !spec.contains('@') && !spec.starts_with("git+"))
                .then(|| format!("{b} runs {spec} without a pinned version"))
        }
        "docker" | "podman" => {
            let i = rest.iter().position(|w| w == "run")?;
            let image = first_positional(
                &rest[i + 1..],
                &[
                    "-e",
                    "--env",
                    "-v",
                    "--volume",
                    "--name",
                    "-p",
                    "--publish",
                    "--network",
                    "--entrypoint",
                    "-w",
                    "--workdir",
                    "-u",
                    "--user",
                    "--mount",
                    "--env-file",
                    "-l",
                    "--label",
                    "--platform",
                    "--pull",
                    "-m",
                    "--memory",
                    "--cpus",
                    "-h",
                    "--hostname",
                    "--add-host",
                    "--cap-add",
                    "--cap-drop",
                ],
            )?;
            if image.contains("@sha256:") {
                return None;
            }
            let last = image.rsplit('/').next().unwrap_or(image);
            match last.split_once(':') {
                Some((_, "latest")) => Some(format!("{b} image {image} uses :latest")),
                Some(_) => None,
                None => Some(format!("{b} image {image} has no tag (implicit :latest)")),
            }
        }
        _ => None,
    }
}

/// RS-MCP-003: unpinned package runner.
pub fn mcp_unpinned(a: &Analyzed) -> Vec<Loc> {
    let li = LineIndex::new(&a.doc.text);
    let mut out = Vec::new();
    for s in servers(a) {
        let Some(cmd) = &s.command else { continue };
        if let Some(why) = unpinned(cmd, &s.args) {
            out.push(loc(
                &li,
                field_line(s, "command"),
                format!("{}: {}", s.name, command_line(s)),
                format!(
                    "MCP server `{}`: {why}. Every start may run a different (possibly compromised) release; pin an exact version or digest.",
                    s.name
                ),
            ));
        }
    }
    out
}

/// Is `host` local?
fn local_host(host: &str) -> bool {
    let h = host
        .trim_start_matches('[')
        .split(']')
        .next()
        .unwrap_or(host);
    let h = h.split(':').next().unwrap_or(h).to_ascii_lowercase();
    h == "localhost"
        || h.ends_with(".localhost")
        || h.starts_with("127.")
        || h == "0.0.0.0"
        || h == "::1"
        || host.starts_with("[::1]")
        || h == "host.docker.internal"
}

/// RS-MCP-004: remote server over plain HTTP.
pub fn mcp_plain_http(a: &Analyzed) -> Vec<Loc> {
    let li = LineIndex::new(&a.doc.text);
    let mut out = Vec::new();
    for s in servers(a) {
        let Some(url) = &s.url else { continue };
        let Some(rest) = url
            .trim()
            .strip_prefix("http://")
            .or_else(|| url.trim().strip_prefix("HTTP://"))
        else {
            continue;
        };
        let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
        let host = host.rsplit('@').next().unwrap_or(host);
        if !local_host(host) {
            out.push(loc(
                &li,
                ["url", "httpUrl", "serverUrl"]
                    .into_iter()
                    .find_map(|f| s.field_lines.get(f).copied())
                    .unwrap_or(s.line),
                format!("{}: {url}", s.name),
                format!(
                    "MCP server `{}` is reached over plain http:// at {host}; tool calls, results, and credentials can be read or changed in transit. Use https://.",
                    s.name
                ),
            ));
        }
    }
    out
}

/// RS-MCP-006: server auto-trusted or tools auto-approved.
pub fn mcp_auto_trust(a: &Analyzed) -> Vec<Loc> {
    let li = LineIndex::new(&a.doc.text);
    let mut out = Vec::new();
    for s in servers(a) {
        if s.auto_trust {
            out.push(loc(
                &li,
                field_line(s, "trust"),
                format!("{}: trust = true", s.name),
                format!(
                    "MCP server `{}` is trusted without confirmation, so its tool calls run without asking.",
                    s.name
                ),
            ));
        }
        if !s.auto_approve.is_empty() {
            let field = ["alwaysAllow", "autoApprove", "auto_approve"]
                .into_iter()
                .find(|f| s.field_lines.contains_key(*f))
                .unwrap_or("autoApprove");
            out.push(loc(
                &li,
                field_line(s, field),
                format!("{}: {field} = {}", s.name, s.auto_approve.join(", ")),
                format!(
                    "MCP server `{}` auto-approves tools ({}), so they run without asking.",
                    s.name,
                    s.auto_approve.join(", ")
                ),
            ));
        }
    }
    out
}

fn risk_rules() -> &'static [Rule] {
    static R: OnceLock<Vec<Rule>> = OnceLock::new();
    R.get_or_init(|| {
        crate::rules::loader::builtin()
            .unwrap_or_default()
            .into_iter()
            .filter(|r| {
                let id = r.id();
                id.starts_with("RS-EXEC-")
                    || matches!(
                        id,
                        "RS-EXF-002" | "RS-EXF-003" | "RS-SEC-002" | "RS-SEC-003"
                    )
            })
            .collect()
    })
}

fn network_re() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex::Regex::new(
            r"(?i)(?:^|[\s;&|(`$/\\])(?:curl|wget|nc|ncat|netcat|ssh|scp|sftp|rsync|ftp|telnet|socat|invoke-webrequest|iwr|invoke-restmethod|irm|start-bitstransfer)(?:\.exe)?(?:\s|$)|\bhttps?://|/dev/tcp/",
        )
        // INVARIANT: constant pattern.
        .unwrap_or_else(|_| unreachable!())
    })
}

/// RS-CFG-002: hook command with network or execution risk.
pub fn hook_risk(a: &Analyzed) -> Vec<Loc> {
    let li = LineIndex::new(&a.doc.text);
    let mut out = Vec::new();
    for h in hooks(a) {
        let cmd = h.command.as_str();
        let mut why: Vec<String> = risk_rules()
            .iter()
            .filter(|r| crate::rules::engine::node_matches_str(&r.node, cmd))
            .map(|r| format!("{} ({})", r.id(), r.def.title))
            .collect();
        if why.is_empty() && network_re().is_match(cmd) {
            why.push("makes a network call".into());
        }
        if !why.is_empty() {
            out.push(loc(
                &li,
                h.line,
                format!("{}: {cmd}", h.event),
                format!(
                    "Hook on `{}` runs automatically and {}: {}",
                    h.event,
                    if why.len() == 1 && why[0] == "makes a network call" {
                        "makes a network call".to_string()
                    } else {
                        format!("matches {}", why.join(", "))
                    },
                    truncate(&unicode::render(cmd), 120)
                ),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn unpinned_runners() {
        assert!(unpinned("npx", &s(&["-y", "example-mcp"])).is_some());
        assert!(unpinned("npx", &s(&["-y", "@acme/example-mcp@latest"])).is_some());
        assert!(unpinned("npx", &s(&["-y", "@acme/example-mcp"])).is_some());
        assert!(unpinned("npx", &s(&["-y", "@acme/example-mcp@1.2.3"])).is_none());
        assert!(unpinned("npx", &s(&["example-mcp@0.4.0", "--port", "1"])).is_none());
        assert!(unpinned("uvx", &s(&["example-mcp"])).is_some());
        assert!(unpinned("uvx", &s(&["example-mcp==0.3.0"])).is_none());
        assert!(unpinned("uvx", &s(&["--from", "example-mcp==1.0", "run"])).is_none());
        assert!(unpinned("pnpm", &s(&["dlx", "example-mcp"])).is_some());
        assert!(unpinned(
            "docker",
            &s(&["run", "-i", "--rm", "-e", "X", "ghcr.io/acme/mcp"])
        )
        .is_some());
        assert!(unpinned("docker", &s(&["run", "-i", "ghcr.io/acme/mcp:latest"])).is_some());
        assert!(unpinned("docker", &s(&["run", "-i", "ghcr.io/acme/mcp:1.4"])).is_none());
        assert!(unpinned("docker", &s(&["run", "localhost:5000/mcp@sha256:abc"])).is_none());
        assert!(unpinned("node", &s(&["server.js"])).is_none());
    }

    #[test]
    fn local_hosts() {
        for h in [
            "localhost:3000",
            "127.0.0.1:8080",
            "[::1]:9",
            "api.localhost",
            "host.docker.internal",
        ] {
            assert!(local_host(h), "{h}");
        }
        assert!(!local_host("mcp.x.example"));
    }
}
