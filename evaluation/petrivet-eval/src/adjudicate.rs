//! ADJUDICATION stage (SPEC section 10): join solver runs with truth, classify outcomes and disagreement
//! patterns, write reproducers, print the summary and the E1 statistics.
//!
//! Reads `results/runs.jsonl` and `results/truth.jsonl` as plain JSON values (no dependency on the record or
//! truth modules) and writes `results/adjudication.jsonl` (one line per run, repetition 0 of each cell),
//! `results/adjudication_summary.json` and, optionally, one reproducer directory per DISAGREE, CRASH and
//! truth-less finding. Output is deterministic: sorted, no timestamps.

use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Args, Debug, Clone)]
pub struct AdjudicateArgs {
    /// Run log
    #[arg(long, default_value = "results/runs.jsonl")]
    pub runs: PathBuf,
    /// Truth file
    #[arg(long, default_value = "results/truth.jsonl")]
    pub truth: PathBuf,
    /// Manifests (instance source, instance path, target markings); optional
    #[arg(long, num_args = 1..)]
    pub manifest: Vec<PathBuf>,
    /// Adjudication output
    #[arg(long, default_value = "results/adjudication.jsonl")]
    pub out: PathBuf,
    /// Directory for reproducers (none are written when absent)
    #[arg(long, default_value = "results/reproducers")]
    pub reproducers: PathBuf,
    /// Only runs of these campaigns (comma separated)
    #[arg(long, value_delimiter = ',')]
    pub campaign: Vec<String>,
    /// Do not adjudicate: read all triage.json under --reproducers and print counts per tag and the untriaged list
    #[arg(long)]
    pub triage_summary: bool,
}

pub const TAGS: [&str; 5] = ["petrivet-bug", "reference-bug", "semantics-difference", "generator-bug", "conversion-bug"];

// ---------------------------------------------------------------------------------------------------------
// data
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
pub struct Run {
    pub run_id: String,
    #[serde(default)]
    pub campaign: String,
    pub instance_id: String,
    pub query: String,
    #[serde(default)]
    pub target_id: Option<String>,
    pub solver_id: String,
    #[serde(default)]
    pub rep: u32,
    pub verdict: String,
    #[serde(default)]
    pub states: Option<u64>,
    #[serde(default)]
    pub edges: Option<u64>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub cmd: String,
    #[serde(default)]
    pub raw_hash: Option<String>,
    #[serde(default)]
    pub stderr_hash: Option<String>,
    #[serde(default)]
    pub verdict_differs: bool,
}

#[derive(Clone, Debug)]
pub struct Truth {
    pub value: Value,
    pub source: String,
    pub corpus_source: String,
}

/// instance_id, query, target_id ("" for none)
pub type CellKey = (String, String, String);

pub type TruthMap = HashMap<CellKey, Truth>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ans {
    Bool(bool),
    Count(u64, u64),
}

impl Ans {
    pub fn show(&self) -> String {
        match self {
            Ans::Bool(true) => "TRUE".into(),
            Ans::Bool(false) => "FALSE".into(),
            Ans::Count(s, e) => format!("COUNT({s},{e})"),
        }
    }
}

pub fn answer_of(r: &Run) -> Option<Ans> {
    match r.verdict.as_str() {
        "TRUE" => Some(Ans::Bool(true)),
        "FALSE" => Some(Ans::Bool(false)),
        "COUNT" => Some(Ans::Count(r.states?, r.edges?)),
        _ => None,
    }
}

/// Compares a definite answer with a truth value. `None`: the kinds do not match (bool vs object), which is a
/// data problem, not a disagreement.
pub fn compare(ans: &Ans, truth: &Value) -> Option<bool> {
    match (ans, truth) {
        (Ans::Bool(a), Value::Bool(t)) => Some(a == t),
        (Ans::Count(s, e), Value::Object(o)) => {
            let st = o.get("STATES").and_then(Value::as_u64);
            let tr = o.get("TRANSITIONS").and_then(Value::as_u64);
            if st.is_none() && tr.is_none() {
                return None;
            }
            Some(st.is_none_or(|x| x == *s) && tr.is_none_or(|x| x == *e))
        }
        _ => None,
    }
}

pub fn truth_show(v: &Value) -> String {
    match v {
        Value::Bool(true) => "TRUE".into(),
        Value::Bool(false) => "FALSE".into(),
        Value::Object(o) => format!(
            "COUNT({},{})",
            o.get("STATES").map_or("?".into(), |x| x.to_string()),
            o.get("TRANSITIONS").map_or("?".into(), |x| x.to_string())
        ),
        other => other.to_string(),
    }
}

/// Outcome of one run. `truth`: the truth line of the cell, if any; `out_of_scope`: the cell is outside SPEC
/// section 3 for this instance. Returns the outcome name and an optional detail.
pub fn classify_run(r: &Run, truth: Option<&Truth>, out_of_scope: bool) -> (String, Option<String>) {
    let verdict = r.verdict.as_str();
    if verdict == "CRASH" {
        return ("CRASH".into(), r.detail.clone());
    }
    if out_of_scope {
        return ("OUT_OF_SCOPE".into(), Some(format!("unbounded instance, run verdict {verdict}")));
    }
    match verdict {
        "TRUE" | "FALSE" | "COUNT" => {
            let Some(ans) = answer_of(r) else {
                return ("RUNNER_ERROR".into(), Some("COUNT without states/edges".into()));
            };
            match truth {
                None => ("NO_TRUTH".into(), None),
                Some(t) => match compare(&ans, &t.value) {
                    Some(true) => ("AGREE".into(), None),
                    Some(false) => ("DISAGREE".into(), None),
                    None => ("RUNNER_ERROR".into(), Some(format!("answer {} does not match truth kind {}", ans.show(), truth_show(&t.value)))),
                },
            }
        }
        "UNKNOWN" | "NOT_SUPPORTED" | "TIMEOUT" | "MEMOUT" | "INPUT_REJECTED" | "RUNNER_ERROR" | "SHARD_FAILED" => (verdict.into(), r.detail.clone()),
        other => ("RUNNER_ERROR".into(), Some(format!("unknown verdict {other}"))),
    }
}

pub fn is_pv(solver: &str) -> bool {
    solver.starts_with("pv-")
}

pub fn is_ref(solver: &str) -> bool {
    solver == "lola" || solver == "tina"
}

fn oracle_like(source: &str) -> bool {
    matches!(source, "mcc-oracle" | "hand" | "construction")
}

/// Pattern of one DISAGREE (SPEC section 10). `others`: definite answers of the other solvers in the same cell.
///
/// pv solver differing from truth:
///   `petrivet-alone` when lola and tina both answer and both equal the truth;
///   `petrivet-with-one-tool` when it agrees with one reference tool and the other reference tool differs from that answer;
///   `pv-modes-disagree` when another pv solver gave an opposite answer (and neither rule above applied);
///   otherwise `other`.
/// reference tool differing from truth:
///   `reference-vs-reference` when the other reference tool answered differently and the truth is not an oracle/hand/construction one;
///   `tool-vs-oracle` when the truth source is mcc-oracle, hand or construction;
///   otherwise `other`.
pub fn classify_pattern(solver: &str, ans: &Ans, truth: &Value, truth_source: &str, others: &BTreeMap<String, Ans>) -> &'static str {
    let is_truth = |a: &Ans| compare(a, truth) == Some(true);
    if is_pv(solver) {
        let lola = others.get("lola");
        let tina = others.get("tina");
        if let (Some(l), Some(t)) = (lola, tina) {
            if is_truth(l) && is_truth(t) {
                return "petrivet-alone";
            }
            if (l == ans) != (t == ans) {
                return "petrivet-with-one-tool";
            }
        }
        if others.iter().any(|(s, a)| is_pv(s) && a != ans) {
            return "pv-modes-disagree";
        }
        "other"
    } else if is_ref(solver) {
        let other = if solver == "lola" { "tina" } else { "lola" };
        if oracle_like(truth_source) {
            "tool-vs-oracle"
        } else if others.get(other).is_some_and(|a| a != ans) {
            "reference-vs-reference"
        } else {
            "other"
        }
    } else {
        "other"
    }
}

/// Disagreement among the answers of one cell without truth: two pv solvers opposite, or lola and tina opposite.
pub fn truthless_patterns(answers: &BTreeMap<String, Ans>) -> Vec<(&'static str, Vec<String>)> {
    let mut out = Vec::new();
    let pv: Vec<_> = answers.iter().filter(|(s, _)| is_pv(s)).collect();
    if pv.iter().any(|(_, a)| **a != *pv[0].1) {
        out.push(("pv-modes-disagree", pv.iter().map(|(s, _)| (*s).clone()).collect()));
    }
    if let (Some(l), Some(t)) = (answers.get("lola"), answers.get("tina"))
        && l != t
    {
        out.push(("reference-vs-reference", vec!["lola".into(), "tina".into()]));
    }
    out
}

/// One-sided 95 percent Clopper-Pearson upper bound for zero failures in n trials.
pub fn cp_upper_zero_failures(n: u64) -> f64 {
    1.0 - 0.05f64.powf(1.0 / n as f64)
}

// ---------------------------------------------------------------------------------------------------------
// adjudication proper
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct Line {
    pub run_id: String,
    pub campaign: String,
    pub instance_id: String,
    pub query: String,
    pub target_id: Option<String>,
    pub solver_id: String,
    pub outcome: String,
    pub truth_value: Option<Value>,
    pub truth_source: Option<String>,
    pub detail: Option<String>,
    /// extras: the run's own answer, the source of the instance, the pattern (DISAGREE and findings), a
    /// repetition whose verdict differed
    pub answer: Option<String>,
    pub source: Option<String>,
    pub pattern: Option<String>,
    pub verdict_differs: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub campaign: String,
    pub instance_id: String,
    pub query: String,
    pub target_id: Option<String>,
    pub pattern: String,
    pub answers: BTreeMap<String, String>,
    pub run_ids: Vec<String>,
}

pub struct Adjudication {
    pub lines: Vec<Line>,
    pub findings: Vec<Finding>,
    pub runs: BTreeMap<String, Run>,
}

pub fn adjudicate(runs: &[Run], truth: &TruthMap, sources: &HashMap<String, String>, campaigns: &[String]) -> Adjudication {
    // repetition 0 per cell and solver; verdict_differs on any repetition is remembered
    type CellId = (String, String, String, String);
    let mut rep0: BTreeMap<(CellId, String), &Run> = BTreeMap::new();
    let mut differs: BTreeSet<(CellId, String)> = BTreeSet::new();
    for r in runs {
        if r.instance_id == "BASELINE" || (!campaigns.is_empty() && !campaigns.contains(&r.campaign)) {
            continue;
        }
        let cell = (r.campaign.clone(), r.instance_id.clone(), r.query.clone(), r.target_id.clone().unwrap_or_default());
        let key = (cell, r.solver_id.clone());
        if r.verdict_differs {
            differs.insert(key.clone());
        }
        if r.rep == 0 {
            rep0.insert(key, r);
        }
    }
    // definite answers per cell
    let mut cells: BTreeMap<CellId, BTreeMap<String, Ans>> = BTreeMap::new();
    for ((cell, solver), r) in &rep0 {
        if let Some(a) = answer_of(r) {
            cells.entry(cell.clone()).or_default().insert(solver.clone(), a);
        }
    }
    let q2_false = |inst: &str| truth.get(&(inst.to_owned(), "Q2".to_owned(), String::new())).is_some_and(|t| t.value == Value::Bool(false));

    let mut lines = Vec::new();
    let mut run_map = BTreeMap::new();
    let mut finding_runs: BTreeMap<String, &'static str> = BTreeMap::new();
    let mut findings = Vec::new();
    for ((cell, solver), r) in &rep0 {
        let tkey: CellKey = (cell.1.clone(), cell.2.clone(), cell.3.clone());
        let t = truth.get(&tkey);
        let oos = matches!(cell.2.as_str(), "Q1" | "Q3" | "Q4") && q2_false(&cell.1);
        let (outcome, detail) = classify_run(r, t, oos);
        let ans = answer_of(r);
        let mut pattern = None;
        if outcome == "DISAGREE" {
            let mut others = cells.get(cell).cloned().unwrap_or_default();
            others.remove(solver);
            let t = t.expect("disagree has truth");
            pattern = Some(classify_pattern(solver, ans.as_ref().expect("disagree has answer"), &t.value, &t.source, &others).to_owned());
        }
        run_map.insert(r.run_id.clone(), (*r).clone());
        lines.push(Line {
            run_id: r.run_id.clone(),
            campaign: r.campaign.clone(),
            instance_id: r.instance_id.clone(),
            query: r.query.clone(),
            target_id: r.target_id.clone(),
            solver_id: solver.clone(),
            outcome,
            truth_value: t.map(|t| t.value.clone()),
            truth_source: t.map(|t| t.source.clone()),
            detail,
            answer: ans.map(|a| a.show()),
            source: sources.get(&r.instance_id).cloned().or_else(|| t.map(|t| t.corpus_source.clone())),
            pattern,
            verdict_differs: differs.contains(&(cell.clone(), solver.clone())),
        });
    }
    // disagreements without any truth line
    for (cell, answers) in &cells {
        let tkey: CellKey = (cell.1.clone(), cell.2.clone(), cell.3.clone());
        if truth.contains_key(&tkey) || (matches!(cell.2.as_str(), "Q1" | "Q3" | "Q4") && q2_false(&cell.1)) {
            continue;
        }
        for (pattern, involved) in truthless_patterns(answers) {
            let run_ids: Vec<String> = involved.iter().filter_map(|s| rep0.get(&(cell.clone(), s.clone())).map(|r| r.run_id.clone())).collect();
            for id in &run_ids {
                finding_runs.insert(id.clone(), pattern);
            }
            findings.push(Finding {
                campaign: cell.0.clone(),
                instance_id: cell.1.clone(),
                query: cell.2.clone(),
                target_id: if cell.3.is_empty() { None } else { Some(cell.3.clone()) },
                pattern: pattern.to_owned(),
                answers: answers.iter().map(|(s, a)| (s.clone(), a.show())).collect(),
                run_ids,
            });
        }
    }
    for l in &mut lines {
        if let Some(p) = finding_runs.get(&l.run_id) {
            l.pattern = Some((*p).to_owned());
        }
    }
    Adjudication { lines, findings, runs: run_map }
}

// ---------------------------------------------------------------------------------------------------------
// summary
// ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct Counts {
    pub agree: u64,
    pub disagree: u64,
    pub unknown: u64,
    pub not_supported: u64,
    pub input_rejected: u64,
    pub crash: u64,
    pub timeout_memout: u64,
    pub runner_error: u64,
    pub no_truth: u64,
    pub out_of_scope: u64,
}

impl Counts {
    fn add(&mut self, outcome: &str) {
        match outcome {
            "AGREE" => self.agree += 1,
            "DISAGREE" => self.disagree += 1,
            "UNKNOWN" => self.unknown += 1,
            "NOT_SUPPORTED" => self.not_supported += 1,
            "INPUT_REJECTED" => self.input_rejected += 1,
            "CRASH" => self.crash += 1,
            "TIMEOUT" | "MEMOUT" => self.timeout_memout += 1,
            "NO_TRUTH" => self.no_truth += 1,
            "OUT_OF_SCOPE" => self.out_of_scope += 1,
            _ => self.runner_error += 1,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct E1Row {
    pub solver_id: String,
    pub source: String,
    pub query: String,
    pub n_conclusive: u64,
    pub n_agree: u64,
    pub n_disagree: u64,
    /// one-sided 95 percent Clopper-Pearson upper bound on the error rate; only when n_disagree = 0 and n_conclusive > 0
    pub cp_upper_95: Option<f64>,
}

pub fn counts_table(lines: &[Line]) -> BTreeMap<(String, String), Counts> {
    let mut m: BTreeMap<(String, String), Counts> = BTreeMap::new();
    for l in lines {
        m.entry((l.solver_id.clone(), l.query.clone())).or_default().add(&l.outcome);
    }
    m
}

pub fn e1_rows(lines: &[Line]) -> Vec<E1Row> {
    let mut m: BTreeMap<(String, String, String), (u64, u64)> = BTreeMap::new();
    for l in lines {
        if l.outcome == "AGREE" || l.outcome == "DISAGREE" {
            let e = m.entry((l.solver_id.clone(), l.source.clone().unwrap_or_else(|| "unknown".into()), l.query.clone())).or_default();
            if l.outcome == "AGREE" {
                e.0 += 1;
            } else {
                e.1 += 1;
            }
        }
    }
    m.into_iter()
        .map(|((solver_id, source, query), (a, d))| E1Row {
            solver_id,
            source,
            query,
            n_conclusive: a + d,
            n_agree: a,
            n_disagree: d,
            cp_upper_95: (d == 0 && a > 0).then(|| cp_upper_zero_failures(a)),
        })
        .collect()
}

fn short(id: &str) -> &str {
    &id[..id.len().min(12)]
}

pub fn render_summary(adj: &Adjudication) -> String {
    let mut s = String::new();
    let table = counts_table(&adj.lines);
    let _ = writeln!(s, "runs adjudicated: {}", adj.lines.len());
    let _ = writeln!(s, "{:<12} {:<4} {:>6} {:>8} {:>7} {:>6} {:>8} {:>6} {:>8} {:>6} {:>8} {:>6}", "solver", "q", "agree", "disagree", "unknown", "n/sup", "rejected", "crash", "to+mo", "rerr", "no_truth", "oos");
    for ((solver, q), c) in &table {
        let _ = writeln!(
            s,
            "{:<12} {:<4} {:>6} {:>8} {:>7} {:>6} {:>8} {:>6} {:>8} {:>6} {:>8} {:>6}",
            solver, q, c.agree, c.disagree, c.unknown, c.not_supported, c.input_rejected, c.crash, c.timeout_memout, c.runner_error, c.no_truth, c.out_of_scope
        );
    }
    let mut by_pattern: BTreeMap<String, Vec<&Line>> = BTreeMap::new();
    for l in adj.lines.iter().filter(|l| l.outcome == "DISAGREE") {
        by_pattern.entry(l.pattern.clone().unwrap_or_else(|| "other".into())).or_default().push(l);
    }
    let _ = writeln!(s, "\nDISAGREE: {}", by_pattern.values().map(Vec::len).sum::<usize>());
    for (p, ls) in &by_pattern {
        let _ = writeln!(s, "  pattern {p} ({})", ls.len());
        for l in ls {
            let _ = writeln!(
                s,
                "    {} {} {} {} answer={} truth={} source={}",
                short(&l.instance_id),
                l.query,
                l.target_id.as_deref().unwrap_or("-"),
                l.solver_id,
                l.answer.as_deref().unwrap_or("?"),
                l.truth_value.as_ref().map_or("?".into(), truth_show),
                l.truth_source.as_deref().unwrap_or("?")
            );
        }
    }
    let _ = writeln!(s, "\nFINDINGS without truth: {}", adj.findings.len());
    for f in &adj.findings {
        let ans: Vec<String> = f.answers.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let _ = writeln!(s, "  {} {} {} {} {}", f.pattern, short(&f.instance_id), f.query, f.target_id.as_deref().unwrap_or("-"), ans.join(" "));
    }
    let crashes: Vec<&Line> = adj.lines.iter().filter(|l| l.outcome == "CRASH").collect();
    let _ = writeln!(s, "\nCRASH: {}", crashes.len());
    for l in crashes {
        let _ = writeln!(s, "  {} {} {} {} detail={}", short(&l.instance_id), l.query, l.target_id.as_deref().unwrap_or("-"), l.solver_id, l.detail.as_deref().unwrap_or("-"));
    }
    let diffs = adj.lines.iter().filter(|l| l.verdict_differs).count();
    let _ = writeln!(s, "\nruns with a differing repetition: {diffs}");
    let _ = writeln!(s, "\nE1 (conclusive = agree + disagree; ub = one-sided 95% Clopper-Pearson upper bound, only for 0 disagreements)");
    let _ = writeln!(s, "{:<12} {:<12} {:<4} {:>10} {:>7} {:>9} {:>8}", "solver", "source", "q", "conclusive", "agree", "disagree", "ub");
    for r in e1_rows(&adj.lines) {
        let _ = writeln!(s, "{:<12} {:<12} {:<4} {:>10} {:>7} {:>9} {:>8}", r.solver_id, r.source, r.query, r.n_conclusive, r.n_agree, r.n_disagree, r.cp_upper_95.map_or("-".into(), |x| format!("{x:.4}")));
    }
    s
}

pub fn summary_json(adj: &Adjudication) -> Value {
    let table: Vec<Value> = counts_table(&adj.lines)
        .into_iter()
        .map(|((solver, query), c)| {
            let mut v = serde_json::to_value(&c).expect("counts");
            v["solver_id"] = json!(solver);
            v["query"] = json!(query);
            v
        })
        .collect();
    let disagreements: Vec<&Line> = adj.lines.iter().filter(|l| l.outcome == "DISAGREE").collect();
    let crashes: Vec<&Line> = adj.lines.iter().filter(|l| l.outcome == "CRASH").collect();
    json!({
        "runs": adj.lines.len(),
        "counts": table,
        "e1": e1_rows(&adj.lines),
        "disagreements": disagreements,
        "findings_without_truth": adj.findings,
        "crashes": crashes,
        "runs_with_differing_repetition": adj.lines.iter().filter(|l| l.verdict_differs).count(),
    })
}

// ---------------------------------------------------------------------------------------------------------
// input and output
// ---------------------------------------------------------------------------------------------------------

pub fn read_runs(path: &Path) -> Result<Vec<Run>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Run>(line) {
            Ok(r) => out.push(r),
            // a torn last line is tolerated, anything else is an error
            Err(_) if i + 1 == text.lines().count() => eprintln!("warning: {}:{}: unparsable last line skipped", path.display(), i + 1),
            Err(e) => return Err(format!("{}:{}: {e}", path.display(), i + 1)),
        }
    }
    Ok(out)
}

pub fn read_truth(path: &Path) -> Result<TruthMap, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut m = HashMap::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|e| format!("{}:{}: {e}", path.display(), i + 1))?;
        let get = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
        let (Some(inst), Some(q)) = (get("instance_id"), get("query")) else {
            return Err(format!("{}:{}: missing instance_id or query", path.display(), i + 1));
        };
        let value = v.get("value").cloned().unwrap_or(Value::Null);
        m.insert((inst, q, get("target_id").unwrap_or_default()), Truth { value, source: get("source").unwrap_or_default(), corpus_source: get("corpus_source").unwrap_or_default() });
    }
    Ok(m)
}

/// What the reproducers need from the manifests.
#[derive(Default)]
pub struct ManifestInfo {
    pub source: HashMap<String, String>,
    pub path: HashMap<String, PathBuf>,
    pub targets: HashMap<(String, String), Value>,
}

pub fn read_manifests(paths: &[PathBuf]) -> Result<ManifestInfo, String> {
    let mut info = ManifestInfo::default();
    for p in paths {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let base = p.canonicalize().ok().and_then(|q| q.parent().map(Path::to_path_buf)).unwrap_or_default();
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(line).map_err(|e| format!("{}:{}: {e}", p.display(), i + 1))?;
            let Some(id) = v.get("instance_id").and_then(Value::as_str) else { continue };
            if let Some(s) = v.get("source").and_then(Value::as_str) {
                info.source.insert(id.to_owned(), s.to_owned());
            }
            if let Some(path) = v.get("path").and_then(Value::as_str) {
                info.path.insert(id.to_owned(), base.join(path));
            }
            for t in v.get("targets").and_then(Value::as_array).into_iter().flatten() {
                if let Some(tid) = t.get("target_id").and_then(Value::as_str) {
                    info.targets.insert((id.to_owned(), tid.to_owned()), t.clone());
                }
            }
        }
    }
    Ok(info)
}

fn read_raw(raw_dir: &Path, hash: &Option<String>, limit: usize) -> Option<String> {
    let h = hash.as_ref()?;
    let bytes = std::fs::read(raw_dir.join(h.get(..2)?).join(h)).ok()?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if text.len() > limit {
        let mut cut = limit;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
        text.push_str("\n[truncated]");
    }
    Some(text)
}

/// Writes `<dir>/<run_id>/README.md`, `triage.json` (an existing tag is kept) and a copy of the instance file.
fn write_reproducer(dir: &Path, line: &Line, adj: &Adjudication, info: &ManifestInfo, raw_dir: &Path) -> Result<(), String> {
    let run = &adj.runs[&line.run_id];
    let rdir = dir.join(&line.run_id);
    std::fs::create_dir_all(&rdir).map_err(|e| format!("{}: {e}", rdir.display()))?;
    let pattern = line.pattern.clone().unwrap_or_else(|| if line.outcome == "CRASH" { "crash".into() } else { "other".into() });
    let inst_path = info.path.get(&line.instance_id);
    let mut copied = None;
    if let Some(p) = inst_path.filter(|p| p.is_file()) {
        let name = format!("instance.{}", p.extension().and_then(|e| e.to_str()).unwrap_or("pnml"));
        if std::fs::copy(p, rdir.join(&name)).is_ok() {
            copied = Some(name);
        }
    }
    let target = line.target_id.as_deref().map(|t| {
        let marking = info.targets.get(&(line.instance_id.clone(), t.to_owned())).and_then(|v| v.get("marking")).map_or("(not in manifests)".into(), Value::to_string);
        format!("{t}, marking {marking}")
    });
    let mut md = String::new();
    let _ = writeln!(md, "# Reproducer {}\n", line.run_id);
    let _ = writeln!(md, "- outcome: {}", line.outcome);
    let _ = writeln!(md, "- pattern: {pattern}");
    let _ = writeln!(md, "- campaign: {}", line.campaign);
    let _ = writeln!(md, "- instance_id: {}", line.instance_id);
    let _ = writeln!(md, "- instance path: {}", inst_path.map_or("(unknown, pass --manifest)".into(), |p| p.display().to_string()));
    if let Some(c) = &copied {
        let _ = writeln!(md, "- instance copy: {c}");
    }
    let _ = writeln!(md, "- solver: {}", line.solver_id);
    let _ = writeln!(md, "- query: {}", line.query);
    let _ = writeln!(md, "- target: {}", target.unwrap_or_else(|| "none".into()));
    let _ = writeln!(md, "- truth: {} (source {})", line.truth_value.as_ref().map_or("none".into(), truth_show), line.truth_source.as_deref().unwrap_or("-"));
    let _ = writeln!(md, "- answer: {}", line.answer.clone().unwrap_or_else(|| run.verdict.clone()));
    if let Some(d) = &line.detail {
        let _ = writeln!(md, "- detail: {d}");
    }
    let _ = writeln!(md, "\n## Command\n\n```\n{}\n```", run.cmd);
    let others: Vec<String> = adj
        .lines
        .iter()
        .filter(|l| l.campaign == line.campaign && l.instance_id == line.instance_id && l.query == line.query && l.target_id == line.target_id && l.run_id != line.run_id)
        .map(|l| format!("- {}: {} ({})", l.solver_id, l.answer.clone().unwrap_or_else(|| "no answer".into()), l.outcome))
        .collect();
    if !others.is_empty() {
        let _ = writeln!(md, "\n## Other solvers on this cell\n\n{}", others.join("\n"));
    }
    match read_raw(raw_dir, &run.raw_hash, 20_000) {
        Some(t) => {
            let _ = writeln!(md, "\n## Raw output ({})\n\n```\n{t}\n```", run.raw_hash.as_deref().unwrap_or(""));
        }
        None => {
            let _ = writeln!(md, "\n## Raw output\n\n(none stored)");
        }
    }
    match read_raw(raw_dir, &run.stderr_hash, 2_000) {
        Some(t) => {
            let _ = writeln!(md, "\n## stderr (first 2000 bytes)\n\n```\n{t}\n```");
        }
        None => {
            let _ = writeln!(md, "\n## stderr\n\n(none)");
        }
    }
    std::fs::write(rdir.join("README.md"), md).map_err(|e| e.to_string())?;
    let tpath = rdir.join("triage.json");
    let tag = std::fs::read_to_string(&tpath).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v.get("tag").cloned()).unwrap_or(Value::Null);
    let triage = json!({"run_id": line.run_id, "pattern": pattern, "tag": tag});
    std::fs::write(&tpath, serde_json::to_string_pretty(&triage).expect("json") + "\n").map_err(|e| e.to_string())
}

pub fn triage_summary(dir: &Path) -> Result<String, String> {
    let mut counts: BTreeMap<String, u64> = TAGS.iter().map(|t| ((*t).to_owned(), 0)).collect();
    let mut untriaged: Vec<(String, String)> = Vec::new();
    let mut invalid = Vec::new();
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).collect(),
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    };
    entries.sort();
    for d in entries {
        let Ok(text) = std::fs::read_to_string(d.join("triage.json")) else { continue };
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", d.display()))?;
        let id = v.get("run_id").and_then(Value::as_str).unwrap_or("?").to_owned();
        let pattern = v.get("pattern").and_then(Value::as_str).unwrap_or("?").to_owned();
        match v.get("tag").and_then(Value::as_str) {
            None => untriaged.push((id, pattern)),
            Some(t) if counts.contains_key(t) => *counts.get_mut(t).expect("tag") += 1,
            Some(t) => invalid.push((id, t.to_owned())),
        }
    }
    let mut s = String::new();
    for (t, n) in &counts {
        let _ = writeln!(s, "{t:<22} {n}");
    }
    let _ = writeln!(s, "{:<22} {}", "untriaged", untriaged.len());
    for (id, p) in &untriaged {
        let _ = writeln!(s, "  {id} {p}");
    }
    for (id, t) in &invalid {
        let _ = writeln!(s, "  INVALID TAG {t:?} in {id}");
    }
    Ok(s)
}

pub fn run(args: &AdjudicateArgs) -> Result<(), String> {
    if args.triage_summary {
        print!("{}", triage_summary(&args.reproducers)?);
        return Ok(());
    }
    let runs = read_runs(&args.runs)?;
    let truth = read_truth(&args.truth)?;
    let info = read_manifests(&args.manifest)?;
    let adj = adjudicate(&runs, &truth, &info.source, &args.campaign);

    let mut out = String::new();
    for l in &adj.lines {
        out.push_str(&serde_json::to_string(l).map_err(|e| e.to_string())?);
        out.push('\n');
    }
    if let Some(p) = args.out.parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    std::fs::write(&args.out, out).map_err(|e| format!("{}: {e}", args.out.display()))?;
    let summary_path = args.out.with_file_name("adjudication_summary.json");
    std::fs::write(&summary_path, serde_json::to_string_pretty(&summary_json(&adj)).map_err(|e| e.to_string())? + "\n").map_err(|e| format!("{}: {e}", summary_path.display()))?;

    let raw_dir = args.runs.parent().unwrap_or(Path::new(".")).join("raw");
    let in_finding: BTreeSet<&String> = adj.findings.iter().flat_map(|f| f.run_ids.iter()).collect();
    let mut n_repro = 0;
    for l in &adj.lines {
        if l.outcome == "DISAGREE" || l.outcome == "CRASH" || in_finding.contains(&l.run_id) {
            write_reproducer(&args.reproducers, l, &adj, &info, &raw_dir)?;
            n_repro += 1;
        }
    }
    print!("{}", render_summary(&adj));
    println!("\nwrote {} ({} lines), {}, {n_repro} reproducers in {}", args.out.display(), adj.lines.len(), summary_path.display(), args.reproducers.display());
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn run_rec(solver: &str, inst: &str, q: &str, target: Option<&str>, verdict: &str) -> Run {
        let rid = format!("{solver}-{inst}-{q}-{}", target.unwrap_or(""));
        Run {
            run_id: rid,
            campaign: "B".into(),
            instance_id: inst.into(),
            query: q.into(),
            target_id: target.map(Into::into),
            solver_id: solver.into(),
            rep: 0,
            verdict: verdict.into(),
            states: None,
            edges: None,
            detail: None,
            cmd: format!("{solver} cmd"),
            raw_hash: None,
            stderr_hash: None,
            verdict_differs: false,
        }
    }

    fn count_run(solver: &str, s: u64, e: u64) -> Run {
        let mut r = run_rec(solver, "i1", "Q0", None, "COUNT");
        r.states = Some(s);
        r.edges = Some(e);
        r
    }

    fn truth_of(value: Value, source: &str) -> Truth {
        Truth { value, source: source.into(), corpus_source: "mcc-small".into() }
    }

    fn tmap(entries: &[(&str, &str, &str, Value, &str)]) -> TruthMap {
        entries.iter().map(|(i, q, t, v, s)| ((i.to_string(), q.to_string(), t.to_string()), truth_of(v.clone(), s))).collect()
    }

    fn outcome(r: &Run, t: Option<&Truth>, oos: bool) -> String {
        classify_run(r, t, oos).0
    }

    #[test]
    fn bool_comparison() {
        let t = truth_of(json!(true), "mcc-oracle");
        assert_eq!(outcome(&run_rec("s", "i", "Q1", None, "TRUE"), Some(&t), false), "AGREE");
        assert_eq!(outcome(&run_rec("s", "i", "Q1", None, "FALSE"), Some(&t), false), "DISAGREE");
    }

    #[test]
    fn count_comparison() {
        let t = truth_of(json!({"STATES": 10, "TRANSITIONS": 20, "MAX_TOKEN_PER_MARKING": 3}), "mcc-oracle");
        assert_eq!(outcome(&count_run("s", 10, 20), Some(&t), false), "AGREE");
        assert_eq!(outcome(&count_run("s", 11, 20), Some(&t), false), "DISAGREE");
        assert_eq!(outcome(&count_run("s", 10, 21), Some(&t), false), "DISAGREE");
        // kind mismatch is a data problem, not a disagreement
        assert_eq!(outcome(&run_rec("s", "i1", "Q0", None, "TRUE"), Some(&t), false), "RUNNER_ERROR");
    }

    #[test]
    fn non_answers_never_disagree() {
        let t = truth_of(json!(true), "mcc-oracle");
        for v in ["UNKNOWN", "NOT_SUPPORTED", "TIMEOUT", "MEMOUT", "INPUT_REJECTED", "RUNNER_ERROR", "CRASH"] {
            assert_eq!(outcome(&run_rec("s", "i", "Q1", None, v), Some(&t), false), v);
            assert_eq!(outcome(&run_rec("s", "i", "Q1", None, v), None, false), v);
        }
    }

    #[test]
    fn no_truth_and_out_of_scope() {
        assert_eq!(outcome(&run_rec("s", "i", "Q1", None, "TRUE"), None, false), "NO_TRUTH");
        let t = truth_of(json!(true), "mcc-oracle");
        assert_eq!(outcome(&run_rec("s", "i", "Q1", None, "FALSE"), Some(&t), true), "OUT_OF_SCOPE");
        assert_eq!(outcome(&run_rec("s", "i", "Q1", None, "CRASH"), None, true), "CRASH");
        // via the full join: Q2 truth FALSE makes Q1/Q3/Q4 out of scope, Q5 stays in scope
        let truth = tmap(&[("i", "Q2", "", json!(false), "construction"), ("i", "Q1", "", json!(true), "construction"), ("i", "Q5", "", json!(true), "construction")]);
        let runs = vec![run_rec("lola", "i", "Q1", None, "FALSE"), run_rec("lola", "i", "Q3", None, "TRUE"), run_rec("lola", "i", "Q5", None, "TRUE")];
        let adj = adjudicate(&runs, &truth, &HashMap::new(), &[]);
        let o: Vec<_> = adj.lines.iter().map(|l| (l.query.as_str(), l.outcome.as_str())).collect();
        assert_eq!(o, vec![("Q1", "OUT_OF_SCOPE"), ("Q3", "OUT_OF_SCOPE"), ("Q5", "AGREE")]);
    }

    fn others(v: &[(&str, bool)]) -> BTreeMap<String, Ans> {
        v.iter().map(|(s, b)| ((*s).to_owned(), Ans::Bool(*b))).collect()
    }

    #[test]
    fn patterns() {
        let t = json!(true);
        let f = Ans::Bool(false);
        assert_eq!(classify_pattern("pv-dispatch", &f, &t, "mcc-oracle", &others(&[("lola", true), ("tina", true)])), "petrivet-alone");
        // pv agrees with tina, lola differs
        assert_eq!(classify_pattern("pv-dispatch", &f, &t, "mcc-oracle", &others(&[("lola", true), ("tina", false)])), "petrivet-with-one-tool");
        // refs both wrong with pv: no ref rule applies, no other pv mode
        assert_eq!(classify_pattern("pv-dispatch", &f, &t, "mcc-oracle", &others(&[("lola", false), ("tina", false)])), "other");
        assert_eq!(classify_pattern("pv-general", &f, &t, "hand", &others(&[("pv-dispatch", true)])), "pv-modes-disagree");
        assert_eq!(classify_pattern("lola", &f, &t, "mcc-oracle", &others(&[("tina", true)])), "tool-vs-oracle");
        assert_eq!(classify_pattern("tina", &f, &t, "construction", &others(&[])), "tool-vs-oracle");
        assert_eq!(classify_pattern("lola", &f, &t, "lola+tina", &others(&[("tina", true)])), "reference-vs-reference");
        assert_eq!(classify_pattern("lola", &f, &t, "lola+tina", &others(&[])), "other");
        assert_eq!(classify_pattern("mystery", &f, &t, "hand", &others(&[])), "other");
    }

    #[test]
    fn truthless_findings() {
        let a: BTreeMap<String, Ans> = others(&[("pv-dispatch", true), ("pv-general", false), ("lola", true), ("tina", false)]);
        let p = truthless_patterns(&a);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].0, "pv-modes-disagree");
        assert_eq!(p[1].0, "reference-vs-reference");
        assert!(truthless_patterns(&others(&[("pv-dispatch", true), ("pv-general", true), ("lola", true), ("tina", true)])).is_empty());
        // pv vs reference without truth is not a finding by itself
        assert!(truthless_patterns(&others(&[("pv-dispatch", true), ("lola", false)])).is_empty());
    }

    #[test]
    fn clopper_pearson() {
        let b = cp_upper_zero_failures(100);
        assert!((b - 0.0295).abs() < 1e-4, "{b}");
        assert!((cp_upper_zero_failures(1) - 0.95).abs() < 1e-12);
    }

    fn scenario() -> (Vec<Run>, TruthMap) {
        let truth = tmap(&[
            ("i1", "Q1", "", json!(true), "mcc-oracle"),
            ("i2", "Q1", "", json!(true), "mcc-oracle"),
            ("i3", "Q1", "", json!(false), "hand"),
            ("i4", "Q1", "", json!(true), "mcc-oracle"),
            ("i1", "Q0", "", json!({"STATES": 5, "TRANSITIONS": 7}), "mcc-oracle"),
        ]);
        let mut runs = vec![
            // i1: petrivet alone
            run_rec("pv-dispatch", "i1", "Q1", None, "FALSE"),
            run_rec("pv-general", "i1", "Q1", None, "TRUE"),
            run_rec("lola", "i1", "Q1", None, "TRUE"),
            run_rec("tina", "i1", "Q1", None, "TRUE"),
            // i2: tina differs from the oracle
            run_rec("lola", "i2", "Q1", None, "TRUE"),
            run_rec("tina", "i2", "Q1", None, "FALSE"),
            // i3: crash and timeout
            run_rec("pv-poly", "i3", "Q1", None, "CRASH"),
            run_rec("lola", "i3", "Q1", None, "TIMEOUT"),
            // i4: pv with one tool
            run_rec("pv-dispatch", "i4", "Q1", None, "FALSE"),
            run_rec("lola", "i4", "Q1", None, "FALSE"),
            run_rec("tina", "i4", "Q1", None, "TRUE"),
            // i5: no truth, modes and references disagree
            run_rec("pv-dispatch", "i5", "Q3", None, "TRUE"),
            run_rec("pv-general", "i5", "Q3", None, "FALSE"),
            run_rec("lola", "i5", "Q3", None, "TRUE"),
            run_rec("tina", "i5", "Q3", None, "FALSE"),
            count_run("pv-general", 5, 8),
            run_rec("pv-general", "BASELINE", "Q1", None, "TRUE"),
        ];
        runs[2].detail = Some("d".into());
        // repetition 1 differing from rep 0 is noted, not a separate line
        let mut rep1 = run_rec("lola", "i2", "Q1", None, "FALSE");
        rep1.rep = 1;
        rep1.run_id = "lola-rep1".into();
        rep1.verdict_differs = true;
        runs.push(rep1);
        (runs, truth)
    }

    #[test]
    fn join_and_patterns_end_to_end_in_memory() {
        let (runs, truth) = scenario();
        let adj = adjudicate(&runs, &truth, &HashMap::new(), &[]);
        let get = |id: &str| adj.lines.iter().find(|l| l.run_id == id).unwrap();
        assert_eq!(get("pv-dispatch-i1-Q1-").outcome, "DISAGREE");
        assert_eq!(get("pv-dispatch-i1-Q1-").pattern.as_deref(), Some("petrivet-alone"));
        assert_eq!(get("tina-i2-Q1-").pattern.as_deref(), Some("tool-vs-oracle"));
        assert_eq!(get("pv-poly-i3-Q1-").outcome, "CRASH");
        assert_eq!(get("lola-i3-Q1-").outcome, "TIMEOUT");
        assert_eq!(get("pv-dispatch-i4-Q1-").pattern.as_deref(), Some("petrivet-with-one-tool"));
        assert_eq!(get("pv-general-i1-Q0-").outcome, "DISAGREE");
        assert_eq!(get("pv-dispatch-i5-Q3-").outcome, "NO_TRUTH");
        assert!(adj.lines.iter().all(|l| l.instance_id != "BASELINE"));
        assert_eq!(adj.lines.len(), 16);
        assert!(get("lola-i2-Q1-").verdict_differs);
        assert_eq!(adj.findings.len(), 2);
        assert_eq!(adj.findings[0].pattern, "pv-modes-disagree");
        assert_eq!(get("lola-i5-Q3-").pattern.as_deref(), Some("reference-vs-reference"));
        // campaign filter
        assert!(adjudicate(&runs, &truth, &HashMap::new(), &["A".into()]).lines.is_empty());
    }

    #[test]
    fn e1_rows_and_counts() {
        let (runs, truth) = scenario();
        let adj = adjudicate(&runs, &truth, &HashMap::new(), &[]);
        let rows = e1_rows(&adj.lines);
        let lola_q1 = rows.iter().find(|r| r.solver_id == "lola" && r.query == "Q1").unwrap();
        assert_eq!((lola_q1.n_conclusive, lola_q1.n_agree, lola_q1.n_disagree, lola_q1.cp_upper_95), (3, 2, 1, None));
        let pg = rows.iter().find(|r| r.solver_id == "pv-general" && r.query == "Q1").unwrap();
        assert!((pg.cp_upper_95.unwrap() - 0.95).abs() < 1e-12);
        let pv = rows.iter().find(|r| r.solver_id == "pv-dispatch" && r.query == "Q1").unwrap();
        assert_eq!((pv.n_conclusive, pv.n_disagree, pv.cp_upper_95), (2, 2, None));
        let c = counts_table(&adj.lines);
        assert_eq!(c[&("lola".into(), "Q1".into())].timeout_memout, 1);
        assert_eq!(c[&("pv-poly".into(), "Q1".into())].crash, 1);
        assert_eq!(c[&("pv-dispatch".into(), "Q3".into())].no_truth, 1);
        // manifest source overrides the truth's corpus_source
        let mut src = HashMap::new();
        src.insert("i1".to_owned(), "calibration".to_owned());
        let adj = adjudicate(&runs, &truth, &src, &[]);
        assert!(e1_rows(&adj.lines).iter().any(|r| r.source == "calibration"));
    }

    #[test]
    fn deterministic_output() {
        let (runs, truth) = scenario();
        let mut shuffled = runs.clone();
        shuffled.reverse();
        let a = adjudicate(&runs, &truth, &HashMap::new(), &[]);
        let b = adjudicate(&shuffled, &truth, &HashMap::new(), &[]);
        let ja: Vec<String> = a.lines.iter().map(|l| serde_json::to_string(l).unwrap()).collect();
        let jb: Vec<String> = b.lines.iter().map(|l| serde_json::to_string(l).unwrap()).collect();
        assert_eq!(ja, jb);
        assert_eq!(render_summary(&a), render_summary(&b));
        assert_eq!(summary_json(&a), summary_json(&b));
    }

    #[test]
    fn end_to_end_files() {
        let dir = std::env::temp_dir().join(format!("pe-adj-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("results")).unwrap();
        let (runs, truth) = scenario();
        let mut text = String::new();
        for r in &runs {
            let mut v = json!({"run_id": r.run_id, "campaign": r.campaign, "instance_id": r.instance_id, "query": r.query, "target_id": r.target_id, "solver_id": r.solver_id, "rep": r.rep, "verdict": r.verdict, "states": r.states, "edges": r.edges, "cmd": r.cmd, "verdict_differs": r.verdict_differs});
            if r.run_id == "pv-dispatch-i1-Q1-" {
                let h = blake3::hash(b"raw out").to_hex().to_string();
                std::fs::create_dir_all(dir.join("results/raw").join(&h[..2])).unwrap();
                std::fs::write(dir.join("results/raw").join(&h[..2]).join(&h), b"raw out").unwrap();
                v["raw_hash"] = json!(h);
            }
            text.push_str(&v.to_string());
            text.push('\n');
        }
        std::fs::write(dir.join("results/runs.jsonl"), text).unwrap();
        let mut t = String::new();
        let mut keys: Vec<_> = truth.iter().collect();
        keys.sort_by_key(|(k, _)| (*k).clone());
        for ((i, q, tg), v) in keys {
            t.push_str(&json!({"instance_id": i, "query": q, "target_id": if tg.is_empty() { Value::Null } else { json!(tg) }, "value": v.value, "source": v.source, "corpus_source": v.corpus_source, "tools": []}).to_string());
            t.push('\n');
        }
        std::fs::write(dir.join("results/truth.jsonl"), t).unwrap();
        std::fs::write(dir.join("i1.pnml"), "<pnml/>").unwrap();
        std::fs::write(dir.join("manifest.jsonl"), json!({"instance_id": "i1", "path": "i1.pnml", "source": "mcc-small"}).to_string() + "\n").unwrap();

        let args = AdjudicateArgs {
            runs: dir.join("results/runs.jsonl"),
            truth: dir.join("results/truth.jsonl"),
            manifest: vec![dir.join("manifest.jsonl")],
            out: dir.join("results/adjudication.jsonl"),
            reproducers: dir.join("results/reproducers"),
            campaign: vec![],
            triage_summary: false,
        };
        run(&args).unwrap();
        let first = std::fs::read_to_string(&args.out).unwrap();
        assert_eq!(first.lines().count(), 16);
        let repro = args.reproducers.clone();
        let mut names: Vec<String> = std::fs::read_dir(&repro).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        names.sort();
        // DISAGREE: pv-dispatch i1, pv-general i1 Q0 count, tina i2, pv-dispatch and lola i4; CRASH: pv-poly i3; findings: 4 runs of i5
        assert_eq!(names.len(), 10, "{names:?}");
        let d = repro.join("pv-dispatch-i1-Q1-");
        let readme = std::fs::read_to_string(d.join("README.md")).unwrap();
        assert!(readme.contains("petrivet-alone") && readme.contains("pv-dispatch cmd") && readme.contains("raw out") && readme.contains("i1.pnml"), "{readme}");
        assert!(d.join("instance.pnml").is_file());
        let tri: Value = serde_json::from_str(&std::fs::read_to_string(d.join("triage.json")).unwrap()).unwrap();
        assert_eq!(tri, json!({"run_id": "pv-dispatch-i1-Q1-", "pattern": "petrivet-alone", "tag": null}));
        assert!(repro.join("pv-poly-i3-Q1-/triage.json").is_file());
        assert!(dir.join("results/adjudication_summary.json").is_file());

        // triage: tag one, rerun keeps the tag, summary counts it
        let tagged = json!({"run_id": "pv-dispatch-i1-Q1-", "pattern": "petrivet-alone", "tag": "petrivet-bug"});
        std::fs::write(d.join("triage.json"), tagged.to_string()).unwrap();
        run(&args).unwrap();
        assert_eq!(std::fs::read_to_string(&args.out).unwrap(), first);
        let sum = triage_summary(&repro).unwrap();
        assert!(sum.contains("petrivet-bug") && sum.lines().any(|l| l.starts_with("petrivet-bug") && l.ends_with('1')), "{sum}");
        assert!(sum.lines().any(|l| l.starts_with("untriaged") && l.ends_with(" 9")), "{sum}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
