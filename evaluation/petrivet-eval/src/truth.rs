//! TRUTH stage for the sources whose truth is known without tools (SPEC section 7):
//! MCC oracle verdicts (`mcc-*`), construction (`fam`), hand answers (`calibration`).
//!
//! Reads manifest lines, emits `results/truth.jsonl`: `{instance_id, query, target_id, value, source, corpus_source, tools}`
//! where `source` is the origin of the answer (`mcc-oracle`, `construction`, `hand`) and `tools` is empty here.
//! Output is sorted and deterministic. Instances of source `gen` have no known answers; their truth comes from LoLA
//! and TINA agreeing (see [`truth_from_tools`], not implemented yet).

use crate::manifest_mcc::KnownAnswer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ORIGINS: [&str; 3] = ["mcc-oracle", "construction", "hand"];

#[derive(Debug, Deserialize)]
pub struct ManifestLine {
    pub instance_id: String,
    pub source: String,
    #[serde(default)]
    pub known_answers: Vec<KnownAnswer>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TruthLine {
    pub instance_id: String,
    pub query: String,
    pub target_id: Option<String>,
    pub value: Value,
    pub source: String,
    pub corpus_source: String,
    pub tools: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Skipped {
    /// instances of source `gen`: truth comes later from LoLA and TINA
    pub generated: usize,
    /// instances of other sources that have no known answer at all
    pub no_answers: usize,
}

/// TODO(truth-from-tools): truth for `gen` instances and for QR/QC targets on `gen` and `mcc-small` is the agreement of
/// LoLA and TINA, both conclusive within the labelling limit (SPEC section 7); disagreement or an inconclusive tool excludes
/// the instance or target, decided before any petrivet answer is seen. Not implemented: it needs the LoLA and TINA adapters.
#[allow(dead_code)]
pub fn truth_from_tools(_manifest_paths: &[PathBuf], _labelling_limit_secs: u64) -> Result<Vec<TruthLine>, String> {
    Err("truth from LoLA and TINA is not implemented yet".into())
}

fn parse_manifest(text: &str, what: &str) -> Result<Vec<ManifestLine>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| serde_json::from_str(l).map_err(|e| format!("{what}:{}: {e}", i + 1)))
        .collect()
}

/// Builds the sorted truth lines of the given manifest lines. Identical repeated answers (same instance in two manifests)
/// collapse; contradicting ones are an error.
pub fn build_truth(lines: &[ManifestLine]) -> Result<(Vec<TruthLine>, Skipped), String> {
    let mut skipped = Skipped::default();
    let mut map: BTreeMap<(String, String, Option<String>), TruthLine> = BTreeMap::new();
    for l in lines {
        if l.source == "gen" {
            skipped.generated += 1;
            continue;
        }
        if l.known_answers.is_empty() {
            skipped.no_answers += 1;
            continue;
        }
        for k in &l.known_answers {
            if !ORIGINS.contains(&k.origin.as_str()) {
                return Err(format!("{}: unknown origin {:?} for {}", l.instance_id, k.origin, k.query));
            }
            let needs_target = k.query == "QR" || k.query == "QC";
            if needs_target != k.target_id.is_some() {
                return Err(format!("{}: query {} {} a target_id", l.instance_id, k.query, if needs_target { "needs" } else { "must not have" }));
            }
            let t = TruthLine {
                instance_id: l.instance_id.clone(),
                query: k.query.clone(),
                target_id: k.target_id.clone(),
                value: k.verdict.clone(),
                source: k.origin.clone(),
                corpus_source: l.source.clone(),
                tools: vec![],
            };
            let key = (t.instance_id.clone(), t.query.clone(), t.target_id.clone());
            if let Some(prev) = map.get(&key) {
                if prev.value != t.value {
                    return Err(format!("{}: contradicting answers for {} ({} versus {})", t.instance_id, t.query, prev.value, t.value));
                }
                continue;
            }
            map.insert(key, t);
        }
    }
    Ok((map.into_values().collect(), skipped))
}

/// Per (corpus source, query): counts of true, false and other values (Q0 objects count as other).
pub fn summarize(lines: &[TruthLine]) -> BTreeMap<(String, String), (usize, usize, usize)> {
    let mut m: BTreeMap<(String, String), (usize, usize, usize)> = BTreeMap::new();
    for t in lines {
        let e = m.entry((t.corpus_source.clone(), t.query.clone())).or_default();
        match t.value {
            Value::Bool(true) => e.0 += 1,
            Value::Bool(false) => e.1 += 1,
            _ => e.2 += 1,
        }
    }
    m
}

pub fn run(manifests: &[PathBuf], out: &Path) -> Result<(), String> {
    if manifests.is_empty() {
        return Err("--manifest needs at least one path".into());
    }
    let mut lines = Vec::new();
    for p in manifests {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        lines.extend(parse_manifest(&text, &p.display().to_string())?);
    }
    let (truth, skipped) = build_truth(&lines)?;
    let mut text = String::new();
    for t in &truth {
        text.push_str(&serde_json::to_string(t).map_err(|e| e.to_string())?);
        text.push('\n');
    }
    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;

    println!("wrote {} ({} truth lines from {} manifest lines)", out.display(), truth.len(), lines.len());
    println!("skipped: {} gen instances (truth later from LoLA and TINA), {} other instances without known answers", skipped.generated, skipped.no_answers);
    println!("{:<12} {:<6} {:>6} {:>6} {:>6} {:>6}", "source", "query", "count", "true", "false", "value");
    for ((src, q), (t, f, o)) in summarize(&truth) {
        println!("{src:<12} {q:<6} {:>6} {t:>6} {f:>6} {o:>6}", t + f + o);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest_mcc::mcc_known_answers;
    use crate::oracle::{Exam, Verdict};
    use serde_json::json;

    fn line(id: &str, source: &str, v: &[(Exam, Verdict)]) -> ManifestLine {
        ManifestLine {
            instance_id: id.into(),
            source: source.into(),
            known_answers: mcc_known_answers(&v.iter().copied().collect(), None),
        }
    }

    #[test]
    fn rd_true_gives_not_deadlock_free() {
        let (t, _) = build_truth(&[line("a", "mcc-small", &[(Exam::RD, Verdict::True), (Exam::L, Verdict::False)])]).unwrap();
        let q1 = t.iter().find(|x| x.query == "Q1").unwrap();
        assert_eq!(q1.value, json!(false));
        assert_eq!((q1.source.as_str(), q1.corpus_source.as_str(), q1.target_id.clone()), ("mcc-oracle", "mcc-small", None));
        assert!(q1.tools.is_empty());
    }

    #[test]
    fn unknown_verdicts_produce_no_line() {
        let (t, s) = build_truth(&[line("a", "mcc-large", &[(Exam::RD, Verdict::Unknown), (Exam::QL, Verdict::Unknown)])]).unwrap();
        assert!(t.is_empty());
        assert_eq!(s.no_answers, 1);
    }

    #[test]
    fn gen_is_skipped_and_counted() {
        let (t, s) = build_truth(&[line("g1", "gen", &[]), line("g2", "gen", &[]), line("a", "mcc-small", &[(Exam::OS, Verdict::True)])]).unwrap();
        assert_eq!(s, Skipped { generated: 2, no_answers: 0 });
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn output_is_sorted_and_independent_of_input_order() {
        let ls = || {
            vec![
                line("b", "mcc-small", &[(Exam::L, Verdict::True), (Exam::RD, Verdict::False)]),
                line("a", "mcc-large", &[(Exam::QL, Verdict::False)]),
                line("a", "mcc-large", &[(Exam::QL, Verdict::False)]),
            ]
        };
        let (x, _) = build_truth(&ls()).unwrap();
        let mut rev = ls();
        rev.reverse();
        let (y, _) = build_truth(&rev).unwrap();
        assert_eq!(x, y);
        let keys: Vec<_> = x.iter().map(|t| (t.instance_id.clone(), t.query.clone())).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
        assert_eq!(x.len(), 3, "the repeated answer collapses");
    }

    #[test]
    fn contradiction_and_bad_origin_are_errors() {
        let e = build_truth(&[line("a", "mcc-small", &[(Exam::L, Verdict::True)]), line("a", "mcc-small", &[(Exam::L, Verdict::False)])]);
        assert!(e.is_err());
        let mut l = line("a", "mcc-small", &[(Exam::L, Verdict::True)]);
        l.known_answers[0].origin = "lola".into();
        assert!(build_truth(&[l]).is_err());
    }

    #[test]
    fn target_answers_need_a_target_id() {
        let l = ManifestLine {
            instance_id: "a".into(),
            source: "calibration".into(),
            known_answers: vec![KnownAnswer { query: "QR".into(), verdict: json!(true), origin: "hand".into(), target_id: None }],
        };
        assert!(build_truth(&[l]).is_err());
    }

    #[test]
    fn summary_counts() {
        let (t, _) = build_truth(&[line("a", "mcc-small", &[(Exam::L, Verdict::True)]), line("b", "mcc-small", &[(Exam::L, Verdict::False)])]).unwrap();
        assert_eq!(summarize(&t)[&("mcc-small".to_string(), "Q3".to_string())], (1, 1, 0));
    }
}
