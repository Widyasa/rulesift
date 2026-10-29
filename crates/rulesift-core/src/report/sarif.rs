//! SARIF 2.1.0 output (PRD §10.2) via `serde-sarif`.

use crate::model::{Report, Severity};
use crate::rules::loader::Rule;
use serde_sarif::sarif::{
    ArtifactLocation, Invocation, Location, Message, MultiformatMessageString, Notification,
    PhysicalLocation, PropertyBag, Region, ReportingDescriptor, Result as SarifResult, ResultLevel,
    Run, Sarif, Tool, ToolComponent,
};
use std::collections::{BTreeMap, BTreeSet};

/// Project URL used for `informationUri` and rule docs.
pub const INFO_URI: &str = "https://github.com/Widyasa/rulesift";

/// URL of the English doc page for a rule.
pub fn help_uri(id: &str) -> String {
    format!("{INFO_URI}/blob/master/docs/rules/{id}.md")
}

/// SARIF `level` for a severity: critical/high → error, medium → warning,
/// low → note.
pub fn level(s: Severity) -> ResultLevel {
    match s {
        Severity::Critical | Severity::High => ResultLevel::Error,
        Severity::Medium => ResultLevel::Warning,
        Severity::Low => ResultLevel::Note,
    }
}

/// GitHub `security-severity` score for a severity.
pub fn security_severity(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "9.5",
        Severity::High => "8.0",
        Severity::Medium => "5.0",
        Severity::Low => "2.0",
    }
}

/// Relative URI for `%SRCROOT%`: forward slashes, no leading `./`.
pub fn relative_uri(path: &str) -> String {
    let p = path.replace('\\', "/");
    let mut p = p.as_str();
    while let Some(rest) = p.strip_prefix("./") {
        p = rest;
    }
    p.to_string()
}

fn props(tags: Vec<String>, sev: Severity) -> PropertyBag {
    let mut extra = BTreeMap::new();
    extra.insert(
        "security-severity".to_string(),
        serde_json::Value::String(security_severity(sev).into()),
    );
    PropertyBag::builder()
        .tags(tags)
        .additional_properties(extra)
        .build()
}

fn descriptor(r: &Rule) -> ReportingDescriptor {
    let d = &r.def;
    let mut tags = vec!["security".to_string()];
    if let Some(c) = &d.category {
        tags.push(c.clone());
    }
    let mut md = format!("**{}** ({})\n\n{}\n", d.title, r.severity, d.message);
    if !d.references.is_empty() {
        md.push_str("\nReferences:\n");
        for x in &d.references {
            md.push_str(&format!("- {x}\n"));
        }
    }
    md.push_str(&format!("\nDocs: {}\n", help_uri(&d.id)));
    ReportingDescriptor::builder()
        .id(d.id.clone())
        .name(d.title.clone())
        .short_description(
            MultiformatMessageString::builder()
                .text(d.title.clone())
                .build(),
        )
        .full_description(
            MultiformatMessageString::builder()
                .text(d.message.clone())
                .build(),
        )
        .help(
            MultiformatMessageString::builder()
                .text(d.message.clone())
                .markdown(md)
                .build(),
        )
        .help_uri(help_uri(&d.id))
        .properties(props(tags, r.severity))
        .build()
}

fn fallback_descriptor(id: &str, title: &str, sev: Severity) -> ReportingDescriptor {
    ReportingDescriptor::builder()
        .id(id.to_string())
        .name(title.to_string())
        .short_description(
            MultiformatMessageString::builder()
                .text(title.to_string())
                .build(),
        )
        .help_uri(help_uri(id))
        .properties(props(vec!["security".into()], sev))
        .build()
}

/// Build the SARIF log. `rules` supplies the rule metadata (every loaded
/// rule is listed in `tool.driver.rules`). Suppressed findings are omitted.
pub fn to_sarif_log(report: &Report, rules: &[Rule]) -> Sarif {
    let mut r = report.clone();
    super::canonicalize(&mut r);
    let mut descriptors: Vec<ReportingDescriptor> = rules.iter().map(descriptor).collect();
    let mut known: BTreeSet<String> = rules.iter().map(|x| x.def.id.clone()).collect();
    let mut results = Vec::new();
    for f in &r.files {
        for x in f.findings.iter().filter(|x| !x.suppressed) {
            if known.insert(x.rule_id.clone()) {
                descriptors.push(fallback_descriptor(&x.rule_id, &x.title, x.severity));
            }
            let mut text = x.message.clone();
            if let Some(d) = &x.decoded {
                text.push_str(&format!(" Decoded text: \"{d}\""));
            }
            let region = Region::builder()
                .start_line(x.line as i64)
                .start_column(x.column as i64)
                .end_line(x.end_line.max(x.line) as i64)
                .build();
            let loc = Location::builder()
                .physical_location(
                    PhysicalLocation::builder()
                        .artifact_location(
                            ArtifactLocation::builder()
                                .uri(relative_uri(&x.path))
                                .uri_base_id("%SRCROOT%".to_string())
                                .build(),
                        )
                        .region(region)
                        .build(),
                )
                .build();
            let mut fp = BTreeMap::new();
            fp.insert("rulesift/v1".to_string(), x.fingerprint.clone());
            results.push(
                SarifResult::builder()
                    .rule_id(x.rule_id.clone())
                    .level(level(x.severity))
                    .message(Message::builder().text(text).build())
                    .locations(vec![loc])
                    .partial_fingerprints(fp)
                    .build(),
            );
        }
    }
    descriptors.sort_by(|a, b| a.id.cmp(&b.id));
    let driver = ToolComponent::builder()
        .name("RuleSift".to_string())
        .information_uri(INFO_URI.to_string())
        .version(r.tool.version.clone())
        .semantic_version(r.tool.version.clone())
        .rules(descriptors)
        .build();
    let mut run = Run::builder()
        .tool(Tool::builder().driver(driver).build())
        .results(results)
        .build();
    if !r.warnings.is_empty() {
        // Skipped or not fully scanned files are tool execution
        // notifications, so they are visible in code scanning too (#100).
        let notes: Vec<Notification> = r
            .warnings
            .iter()
            .map(|w| {
                Notification::builder()
                    .level(serde_json::Value::String("warning".into()))
                    .message(Message::builder().text(w.message.clone()).build())
                    .locations(vec![Location::builder()
                        .physical_location(
                            PhysicalLocation::builder()
                                .artifact_location(
                                    ArtifactLocation::builder()
                                        .uri(relative_uri(&w.path))
                                        .uri_base_id("%SRCROOT%".to_string())
                                        .build(),
                                )
                                .build(),
                        )
                        .build()])
                    .build()
            })
            .collect();
        run.invocations = Some(vec![Invocation::builder()
            .execution_successful(true)
            .tool_execution_notifications(notes)
            .build()]);
    }
    Sarif::builder()
        .schema(serde_sarif::sarif::SCHEMA_URL.to_string())
        .version(serde_json::Value::String("2.1.0".into()))
        .runs(vec![run])
        .build()
}

/// Serialize the SARIF log as pretty JSON with a trailing newline.
pub fn to_sarif(report: &Report, rules: &[Rule]) -> String {
    let mut s =
        serde_json::to_string_pretty(&to_sarif_log(report, rules)).expect("sarif serializes");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::loader::{load, LoadOptions, RuleSet};
    use crate::scan::{scan, ScanOptions};
    use std::path::PathBuf;

    const SARIF_SCHEMA: &str = include_str!("../../../../tests/schemas/sarif-2.1.0.json");

    fn rules() -> RuleSet {
        load(&LoadOptions {
            builtin: true,
            ..Default::default()
        })
        .unwrap()
    }

    fn risky(rules: &RuleSet) -> Report {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/repos/risky");
        let mut r = scan(
            rules,
            &ScanOptions {
                paths: vec![root.clone()],
                root: ".".into(),
                discovery: crate::discovery::DiscoveryOptions {
                    anchor: Some(root.clone()),
                    ..Default::default()
                },
                ..ScanOptions::default()
            },
        )
        .unwrap()
        .report;
        let prefix = format!("{}/", root.to_string_lossy());
        for f in &mut r.files {
            f.path = format!("./{}", f.path.trim_start_matches(&prefix));
            for x in &mut f.findings {
                x.path = format!("./{}", x.path.trim_start_matches(&prefix));
            }
        }
        r
    }

    #[test]
    fn validates_against_sarif_schema() {
        let set = rules();
        let text = to_sarif(&risky(&set), &set.rules);
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let schema: serde_json::Value = serde_json::from_str(SARIF_SCHEMA).unwrap();
        let val = jsonschema::draft4::new(&schema).unwrap();
        let errs: Vec<String> = val
            .iter_errors(&v)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .collect();
        assert!(errs.is_empty(), "{errs:#?}");
        assert_eq!(v["version"], "2.1.0");
        assert_eq!(v["runs"][0]["tool"]["driver"]["name"], "RuleSift");
        assert_eq!(v["runs"][0]["tool"]["driver"]["informationUri"], INFO_URI);
    }

    #[test]
    fn levels_and_security_severity() {
        assert!(matches!(level(Severity::Critical), ResultLevel::Error));
        assert!(matches!(level(Severity::High), ResultLevel::Error));
        assert!(matches!(level(Severity::Medium), ResultLevel::Warning));
        assert!(matches!(level(Severity::Low), ResultLevel::Note));
        assert_eq!(security_severity(Severity::Critical), "9.5");
        assert_eq!(security_severity(Severity::High), "8.0");
        assert_eq!(security_severity(Severity::Medium), "5.0");
        assert_eq!(security_severity(Severity::Low), "2.0");

        let set = rules();
        let v: serde_json::Value =
            serde_json::from_str(&to_sarif(&risky(&set), &set.rules)).unwrap();
        let results = v["runs"][0]["results"].as_array().unwrap();
        let exec = results
            .iter()
            .find(|r| r["ruleId"] == "RS-EXEC-001")
            .unwrap();
        assert_eq!(exec["level"], "error");
        let rules = v["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
        assert_eq!(rules.len(), set.rules.len());
        let d = rules.iter().find(|r| r["id"] == "RS-EXEC-001").unwrap();
        assert_eq!(d["properties"]["security-severity"], "9.5");
        assert_eq!(d["helpUri"], help_uri("RS-EXEC-001"));
        let tags = d["properties"]["tags"].as_array().unwrap();
        assert_eq!(tags[0], "security");
        assert!(d["help"]["markdown"].as_str().unwrap().contains("Docs:"));
    }

    #[test]
    fn paths_are_relative_to_srcroot() {
        let set = rules();
        let v: serde_json::Value =
            serde_json::from_str(&to_sarif(&risky(&set), &set.rules)).unwrap();
        for r in v["runs"][0]["results"].as_array().unwrap() {
            let a = &r["locations"][0]["physicalLocation"]["artifactLocation"];
            assert_eq!(a["uriBaseId"], "%SRCROOT%");
            let uri = a["uri"].as_str().unwrap();
            assert!(!uri.starts_with('/') && !uri.starts_with("./"), "{uri}");
            assert!(r["partialFingerprints"]["rulesift/v1"]
                .as_str()
                .unwrap()
                .starts_with("sha256:"));
            assert!(
                r["locations"][0]["physicalLocation"]["region"]["startLine"]
                    .as_i64()
                    .unwrap()
                    >= 1
            );
        }
        assert_eq!(relative_uri("./a/./b.md"), "a/./b.md");
        assert_eq!(relative_uri(".\\x\\y.md"), "x/y.md");
    }

    #[test]
    fn suppressed_findings_are_omitted() {
        let set = rules();
        let mut r = risky(&set);
        for f in &mut r.files {
            for x in &mut f.findings {
                x.suppressed = true;
            }
        }
        let v: serde_json::Value = serde_json::from_str(&to_sarif(&r, &set.rules)).unwrap();
        assert!(v["runs"][0]["results"].as_array().unwrap().is_empty());
    }
}
