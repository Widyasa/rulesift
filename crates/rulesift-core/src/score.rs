//! Risk score (PRD §9).
//!
//! File score = min(100, round(Σ weights × M + B)):
//! - weights per severity (configurable, default critical 70, high 25,
//!   medium 10, low 3);
//! - M = always-loaded multiplier (default 1.5) for always-loaded files;
//! - B = toxic-combination bonus (default 20) when the same file or skill has
//!   an RS-SEC-002/003 finding and an RS-EXF-* or RS-EXEC-001/002 finding.
//!
//! A live RS-CFG-002 finding (hook with network or execution risk) of high
//! or critical severity raises the file score to at least `review_at`, in
//! every location (#138).
//!
//! Repo score = max file score. Suppressed findings do not count.

use crate::config::ScoreConfig;
use crate::model::{Breakdown, FileReport, Finding, Tier};
use std::collections::BTreeMap;

/// Is this finding a "reads secrets" leg of the toxic combination?
pub fn is_secret_leg(id: &str) -> bool {
    matches!(id, "RS-SEC-002" | "RS-SEC-003")
}

/// Is this finding a "sends or executes" leg of the toxic combination?
pub fn is_send_leg(id: &str) -> bool {
    id.starts_with("RS-EXF-") || matches!(id, "RS-EXEC-001" | "RS-EXEC-002")
}

/// Does this finding put its file in review at least? An auto-running hook
/// with a network or execution risk (RS-CFG-002) must stand out wherever it
/// is declared (#138); lowering the rule's severity below high opts out.
pub fn is_review_floor(f: &Finding) -> bool {
    f.rule_id == "RS-CFG-002" && f.severity >= crate::model::Severity::High
}

/// Do these (unsuppressed) findings form the toxic combination?
pub fn toxic<'a>(findings: impl IntoIterator<Item = &'a Finding>) -> bool {
    let (mut s, mut x) = (false, false);
    for f in findings.into_iter().filter(|f| !f.suppressed) {
        s |= is_secret_leg(&f.rule_id);
        x |= is_send_leg(&f.rule_id);
    }
    s && x
}

/// Score one file. `toxic_group` says whether the file's skill (or the file
/// itself) has the toxic combination.
pub fn score_file(
    findings: &[Finding],
    always_loaded: bool,
    toxic_group: bool,
    cfg: &ScoreConfig,
) -> (u32, Tier, Breakdown) {
    let live: Vec<&Finding> = findings.iter().filter(|f| !f.suppressed).collect();
    let weights: u32 = live.iter().map(|f| cfg.weight(f.severity)).sum();
    let multiplier = if always_loaded {
        cfg.always_loaded_multiplier
    } else {
        1.0
    };
    let bonus = if toxic_group && !live.is_empty() {
        cfg.toxic_combo_bonus
    } else {
        0
    };
    let raw = (f64::from(weights) * multiplier + f64::from(bonus)).round();
    let mut score = if raw >= 100.0 {
        100
    } else {
        raw.max(0.0) as u32
    };
    if live.iter().any(|f| is_review_floor(f)) {
        score = score.max(cfg.review_at);
    }
    let tier = Tier::from_score_with(score, cfg.review_at, cfg.block_at);
    (
        score,
        tier,
        Breakdown {
            weights,
            multiplier,
            bonus,
        },
    )
}

/// Inputs per file for [`score_all`].
#[derive(Debug, Clone, Default)]
pub struct FileMeta {
    /// Always loaded into the agent context.
    pub always_loaded: bool,
    /// Skill directory group (files of one skill share the toxic bonus).
    pub group: Option<String>,
}

/// Score every report in place. `meta[i]` belongs to `files[i]`.
pub fn score_all(files: &mut [FileReport], meta: &[FileMeta], cfg: &ScoreConfig) {
    let mut union: BTreeMap<String, Vec<&Finding>> = BTreeMap::new();
    for (f, m) in files.iter().zip(meta) {
        if let Some(g) = &m.group {
            union
                .entry(g.clone())
                .or_default()
                .extend(f.findings.iter());
        }
    }
    let toxic_groups: BTreeMap<String, bool> =
        union.into_iter().map(|(g, v)| (g, toxic(v))).collect();
    for (f, m) in files.iter_mut().zip(meta) {
        let t = match &m.group {
            Some(g) => toxic_groups.get(g).copied().unwrap_or(false),
            None => toxic(&f.findings),
        };
        let (score, tier, b) = score_file(&f.findings, m.always_loaded, t, cfg);
        f.score = score;
        f.tier = tier;
        f.breakdown = b;
    }
}

/// Repo score: max file score.
pub fn repo_score(files: &[FileReport]) -> u32 {
    files.iter().map(|f| f.score).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FileKind, Severity, Tool};

    fn f(id: &str, sev: Severity) -> Finding {
        Finding {
            rule_id: id.into(),
            severity: sev,
            title: String::new(),
            message: String::new(),
            path: "x".into(),
            line: 1,
            column: 1,
            end_line: 1,
            evidence: String::new(),
            decoded: None,
            fingerprint: String::new(),
            suppressed: false,
        }
    }

    use Severity::*;

    #[test]
    fn table() {
        let cfg = ScoreConfig::default();
        // (findings, always_loaded, toxic, score, tier)
        let cases: Vec<(Vec<Finding>, bool, bool, u32, Tier)> = vec![
            (vec![], false, false, 0, Tier::Ok),
            (vec![], true, true, 0, Tier::Ok),
            (vec![f("RS-X", Low)], false, false, 3, Tier::Info),
            (vec![f("RS-X", Medium)], false, false, 10, Tier::Info),
            (vec![f("RS-X", High)], false, false, 25, Tier::Info),
            (
                vec![f("RS-X", High), f("RS-Y", Medium), f("RS-Z", Low)],
                false,
                false,
                38,
                Tier::Info,
            ),
            (
                vec![f("RS-X", High), f("RS-Y", Medium), f("RS-Z", Medium)],
                false,
                false,
                45,
                Tier::Review,
            ),
            (vec![f("RS-X", Critical)], false, false, 70, Tier::Block),
            // always-loaded multiplier: 25 * 1.5 = 37.5 -> 38
            (vec![f("RS-X", High)], true, false, 38, Tier::Info),
            // 2 high * 1.5 = 75 -> block
            (
                vec![f("RS-X", High), f("RS-Y", High)],
                true,
                false,
                75,
                Tier::Block,
            ),
            // toxic bonus: 25 + 25 + 20 = 70
            (
                vec![f("RS-SEC-002", High), f("RS-EXF-002", High)],
                false,
                true,
                70,
                Tier::Block,
            ),
            // cap at 100
            (
                vec![f("RS-X", Critical), f("RS-Y", Critical)],
                true,
                true,
                100,
                Tier::Block,
            ),
            // medium * 1.5 = 15
            (vec![f("RS-X", Medium)], true, false, 15, Tier::Info),
        ];
        for (i, (fs, al, tx, score, tier)) in cases.into_iter().enumerate() {
            let (s, t, _) = score_file(&fs, al, tx, &cfg);
            assert_eq!((s, t), (score, tier), "case {i}");
        }
    }

    #[test]
    fn hook_risk_is_at_least_review() {
        // #138: RS-CFG-002 lands in review in every location
        let cfg = ScoreConfig::default();
        for al in [false, true] {
            let (s, t, b) = score_file(&[f("RS-CFG-002", High)], al, false, &cfg);
            assert_eq!((s, t), (40, Tier::Review), "always_loaded={al}");
            assert_eq!(b.weights, 25);
        }
        // a higher score is kept
        let (s, t, _) = score_file(&[f("RS-CFG-002", High), f("RS-X", High)], true, false, &cfg);
        assert_eq!((s, t), (75, Tier::Block));
        // suppressed or downgraded below high: no floor
        let mut sup = f("RS-CFG-002", High);
        sup.suppressed = true;
        assert_eq!(score_file(&[sup], false, false, &cfg).1, Tier::Ok);
        assert_eq!(
            score_file(&[f("RS-CFG-002", Medium)], false, false, &cfg).1,
            Tier::Info
        );
    }

    #[test]
    fn suppressed_do_not_count() {
        let mut a = f("RS-X", Critical);
        a.suppressed = true;
        let (s, t, b) = score_file(&[a.clone()], false, false, &ScoreConfig::default());
        assert_eq!((s, t, b.weights), (0, Tier::Ok, 0));
        let sec = f("RS-SEC-002", High);
        let mut exf = f("RS-EXF-001", High);
        exf.suppressed = true;
        assert!(!toxic([&sec, &exf]));
    }

    #[test]
    fn toxic_legs() {
        assert!(toxic([
            &f("RS-SEC-003", Medium),
            &f("RS-EXEC-001", Critical)
        ]));
        assert!(toxic([&f("RS-SEC-002", High), &f("RS-EXF-003", Medium)]));
        assert!(!toxic([&f("RS-SEC-001", High), &f("RS-EXF-002", High)]));
        assert!(!toxic([&f("RS-SEC-002", High), &f("RS-EXEC-004", High)]));
    }

    #[test]
    fn config_overrides() {
        let cfg = ScoreConfig {
            high: 50,
            always_loaded_multiplier: 2.0,
            toxic_combo_bonus: 5,
            block_at: 90,
            review_at: 20,
            ..ScoreConfig::default()
        };
        let (s, t, b) = score_file(&[f("RS-X", High)], true, false, &cfg);
        assert_eq!((s, t), (100, Tier::Block));
        assert_eq!(b.multiplier, 2.0);
        let (s, t, _) = score_file(&[f("RS-X", Medium)], false, false, &cfg);
        assert_eq!((s, t), (10, Tier::Info));
        let (s, t, b) = score_file(
            &[f("RS-SEC-002", Low), f("RS-EXF-001", Low)],
            false,
            true,
            &cfg,
        );
        assert_eq!((s, t, b.bonus), (11, Tier::Info, 5));
        let (_, t, _) = score_file(&[f("RS-X", High)], false, false, &cfg);
        assert_eq!(t, Tier::Review);
    }

    fn report(path: &str, findings: Vec<Finding>) -> FileReport {
        FileReport {
            path: path.into(),
            tool: Tool::AgentSkills,
            kind: FileKind::Skill,
            score: 0,
            tier: Tier::Ok,
            breakdown: Breakdown::default(),
            findings,
        }
    }

    #[test]
    fn skill_group_shares_toxic_bonus_and_repo_max() {
        let mut files = vec![
            report("s/a/SKILL.md", vec![f("RS-SEC-002", High)]),
            report("s/a/run.sh", vec![f("RS-EXF-002", High)]),
            report("s/b/SKILL.md", vec![f("RS-SEC-002", High)]),
        ];
        let meta = vec![
            FileMeta {
                always_loaded: false,
                group: Some("s/a".into()),
            },
            FileMeta {
                always_loaded: false,
                group: Some("s/a".into()),
            },
            FileMeta {
                always_loaded: false,
                group: Some("s/b".into()),
            },
        ];
        score_all(&mut files, &meta, &ScoreConfig::default());
        assert_eq!(files[0].score, 45);
        assert_eq!(files[1].score, 45);
        assert_eq!(files[2].score, 25);
        assert_eq!(files[0].breakdown.bonus, 20);
        assert_eq!(repo_score(&files), 45);
    }
}
