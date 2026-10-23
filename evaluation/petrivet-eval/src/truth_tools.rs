//! TRUTH from tools: labelling of `gen` instances by the AGREEMENT of LoLA and TINA (SPEC sections 3, 5.5, 7).
//!
//! INDEPENDENCE FROM PETRIVET (structural, not by convention). The input of this module is: instance manifests and
//! the runs of the two reference tools. Nothing else. Concretely:
//! * [`decide`] takes two [`ToolResult`]s and nothing else; it has no access to any other record.
//! * Records are read back from `runs.jsonl` only if `campaign == "TRUTH"`, `rep == 0` and `solver_id` is one of
//!   [`TOOLS`] (`lola`, `tina`); every other line (pv-* solvers, other campaigns) is dropped at the reader
//!   ([`read_tool_records`]) before anything looks at it.
//! * Scope and exclusion decisions use only the tool answers (and the manifest), so exclusions cannot depend on
//!   petrivet's answers. `detected_class_petrivet` of the manifest is never read; the class column of the summary
//!   uses `detected_class_testkit`.
//!
//! Rule per (instance, query, target): value = the common answer if both tools gave a definite TRUE/FALSE and agree;
//! otherwise excluded with reason `disagree`, `inconclusive:<tool>` or `both_inconclusive`.
//! Scope: if the Q2 truth of an instance is FALSE (unbounded), Q1, Q3 and Q4 are out of scope (not run, not labelled,
//! counted separately). QC and QR targets are always run and labelled by the same rule.
//!
//! Runs go through the same machinery as `run`/`calibrate` (`Ctx::execute`, campaign `TRUTH`, one repetition, no
//! pinning, no memory limit, wall limit = labelling limit) and are appended to `<results_dir>/runs.jsonl`; a rerun
//! skips every run_id already recorded (RUNNER_ERROR records are retried).

use crate::adapters::{self, Adapter, Query, SolversCfg};
use crate::instance::{Instance, TargetMarking};
use crate::record::{self, RunLog, RunRecord, Verdict};
use crate::run::{Ctx, Job, Unit};
use crate::runner::{self, Limiter};
use crate::truth::TruthLine;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const TOOLS: [&str; 2] = ["lola", "tina"];
const CAMPAIGN: &str = "TRUTH";
/// Order of the queries in reports; Q2 first matters for the scope rule.
const QUERY_ORDER: [Query; 6] = [Query::Q1, Query::Q2, Query::Q3, Query::Q4, Query::QR, Query::QC];

#[derive(clap::Args, Default)]
pub struct TruthArgs {
    /// Label `gen` instances with these tools (must be `lola,tina`); without it only known answers are written
    #[arg(long, value_delimiter = ',')]
    pub tools: Vec<String>,
    /// Wall-clock limit per labelling run, seconds (no pinning, no memory limit)
    #[arg(long, default_value_t = 60)]
    pub label_limit_secs: u64,
    /// Directory of runs.jsonl, raw/ and truth_excluded.jsonl (default: [run] results_dir of eval.toml)
    #[arg(long)]
    pub results_dir: Option<PathBuf>,
    /// Parallel labelling jobs
    #[arg(long, default_value_t = 3)]
    pub jobs: usize,
    /// Label only N gen instances, chosen with a seeded shuffle (quick trial)
    #[arg(long)]
    pub only_gen_sample: Option<usize>,
}

/// What one tool said about one (instance, query, target).
#[derive(Clone, Debug, PartialEq)]
pub struct ToolResult {
    pub tool: String,
    pub outcome: Verdict,
    pub detail: Option<String>,
    pub cmd: String,
    pub run_id: String,
}

impl ToolResult {
    fn definite(&self) -> Option<bool> {
        match self.outcome {
            Verdict::True => Some(true),
            Verdict::False => Some(false),
            _ => None,
        }
    }
    fn missing(tool: &str) -> Self {
        ToolResult { tool: tool.into(), outcome: Verdict::RunnerError, detail: Some("no run record".into()), cmd: String::new(), run_id: String::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    Disagree,
    /// The named tool gave no definite answer while the other one did.
    Inconclusive(String),
    BothInconclusive,
}

impl Reason {
    pub fn text(&self) -> String {
        match self {
            Reason::Disagree => "disagree".into(),
            Reason::Inconclusive(t) => format!("inconclusive:{t}"),
            Reason::BothInconclusive => "both_inconclusive".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Label(bool),
    Excluded(Reason),
}

/// The decision rule. Definite means TRUE or FALSE; everything else (UNKNOWN, TIMEOUT, MEMOUT, NOT_SUPPORTED, CRASH,
/// INPUT_REJECTED, RUNNER_ERROR, and also COUNT or SHARD_FAILED, which a yes/no query never legitimately yields) is
/// inconclusive. Takes the two tool results and nothing else.
pub fn decide(lola: &ToolResult, tina: &ToolResult) -> Decision {
    match (lola.definite(), tina.definite()) {
        (Some(a), Some(b)) if a == b => Decision::Label(a),
        (Some(_), Some(_)) => Decision::Excluded(Reason::Disagree),
        (Some(_), None) => Decision::Excluded(Reason::Inconclusive(tina.tool.clone())),
        (None, Some(_)) => Decision::Excluded(Reason::Inconclusive(lola.tool.clone())),
        (None, None) => Decision::Excluded(Reason::BothInconclusive),
    }
}

/// Scope rule of SPEC section 3: on a net whose Q2 truth is FALSE, Q1, Q3 and Q4 are out of scope.
pub fn out_of_scope(query: Query, q2_truth: Option<bool>) -> bool {
    matches!(query, Query::Q1 | Query::Q3 | Query::Q4) && q2_truth == Some(false)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Labelled(bool),
    Excluded(Reason),
    OutOfScope,
}

/// One considered (instance, query, target), for the summary.
#[derive(Clone, Debug)]
pub struct Row {
    pub class: String,
    pub query: Query,
    pub status: Status,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolEntry {
    pub tool: String,
    pub outcome: String,
    pub detail: Option<String>,
    pub cmd: String,
    pub run_id: String,
}

/// A line of `truth_excluded.jsonl`.
#[derive(Clone, Debug, Serialize)]
pub struct Excluded {
    pub instance_id: String,
    pub path: String,
    pub corpus_source: String,
    pub class: String,
    pub query: String,
    pub target_id: Option<String>,
    pub target: Option<String>,
    pub reason: String,
    pub results: Vec<ToolEntry>,
}

pub struct LabelCfg<'a> {
    pub manifests: &'a [PathBuf],
    pub results_dir: &'a Path,
    pub wall_secs: u64,
    pub parallel: usize,
    pub sample: Option<usize>,
    pub seed: u64,
    pub solvers: &'a SolversCfg,
    pub out_cap_kib: usize,
}

#[derive(Default)]
pub struct Labelled {
    pub truth: Vec<TruthLine>,
    pub excluded: Vec<Excluded>,
    pub rows: Vec<Row>,
    /// Number of gen instances considered (after sampling).
    pub instances: usize,
    pub ran: usize,
    pub skipped: usize,
    /// RUNNER_ERROR runs among the used records (infrastructure failures).
    pub runner_errors: usize,
}

/// Instances of source `gen` from jsonl manifests (other sources and `.json` manifests are ignored), deduplicated by
/// id, sorted by id.
fn load_gen(manifests: &[PathBuf]) -> Result<Vec<Instance>, String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in manifests.iter().filter(|p| p.extension().is_none_or(|e| e != "json")) {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let base = path.canonicalize().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
        for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
            let at = |e: String| format!("{}:{}: {e}", path.display(), i + 1);
            let v: Value = serde_json::from_str(line).map_err(|e| at(e.to_string()))?;
            if v.get("source").and_then(Value::as_str) != Some("gen") {
                continue;
            }
            let mut inst: Instance = serde_json::from_value(v).map_err(|e| at(e.to_string()))?;
            inst.base = base.clone();
            if seen.insert(inst.instance_id.clone()) {
                out.push(inst);
            }
        }
    }
    out.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    Ok(out)
}

fn sample(mut v: Vec<Instance>, n: Option<usize>, seed: u64) -> Vec<Instance> {
    if let Some(n) = n {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        v.shuffle(&mut rng);
        v.truncate(n);
        v.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    }
    v
}

fn which(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(program).is_file()))
}

/// Records of the two tools in campaign TRUTH, repetition 0, by run_id (the last record of a run_id wins). Every
/// other line of runs.jsonl is dropped here (see the module comment).
pub fn read_tool_records(results_dir: &Path) -> HashMap<String, RunRecord> {
    let mut map = HashMap::new();
    if let Ok(text) = std::fs::read_to_string(record::runs_path(results_dir)) {
        for line in text.lines() {
            if let Ok(r) = serde_json::from_str::<RunRecord>(line)
                && r.campaign == CAMPAIGN
                && r.rep == 0
                && TOOLS.contains(&r.solver_id.as_str())
            {
                map.insert(r.run_id.clone(), r);
            }
        }
    }
    map
}

fn class_of(inst: &Instance) -> String {
    inst.detected_class_testkit.clone().or_else(|| inst.generator_class.clone()).unwrap_or_else(|| "unknown".into())
}

fn target_text(inst: &Instance, tid: &str) -> Option<String> {
    let t = inst.targets.iter().find(|t| t.target_id == tid)?;
    let m = match &t.marking {
        TargetMarking::Vector(v) => serde_json::to_string(v),
        TargetMarking::Map(m) => serde_json::to_string(m),
    };
    m.ok().map(|m| format!("{}{m}", t.kind))
}

fn tool_result(records: &HashMap<String, RunRecord>, inst: &Instance, q: Query, tid: Option<&str>, tool: &str) -> ToolResult {
    let id = record::run_id(CAMPAIGN, &inst.instance_id, q.as_str(), tid, tool, 0);
    match records.get(&id) {
        Some(r) => ToolResult { tool: tool.into(), outcome: r.verdict, detail: r.detail.clone(), cmd: r.cmd.clone(), run_id: id },
        None => ToolResult { run_id: id, ..ToolResult::missing(tool) },
    }
}

/// Targets of kind QR or QC of the instance, as `(query, target index)`.
fn target_units(inst: &Instance) -> Vec<(Query, usize)> {
    let mut v = Vec::new();
    for q in [Query::QR, Query::QC] {
        for (i, t) in inst.targets.iter().enumerate() {
            if t.kind.eq_ignore_ascii_case(q.as_str()) {
                v.push((q, i));
            }
        }
    }
    v
}

struct Exec<'a> {
    cfg: &'a LabelCfg<'a>,
    instances: &'a [Instance],
    adapters: &'a [Box<dyn Adapter>],
}

impl Exec<'_> {
    /// Runs every (unit, solver) whose run_id is not in `done`; appends to the log; returns (ran, skipped).
    fn execute(&self, units: Vec<Unit>, done: &mut HashSet<String>, log: &mut RunLog) -> Result<(usize, usize), String> {
        let mut jobs: Vec<Job> = Vec::new();
        let mut skipped = 0;
        for (ui, u) in units.iter().enumerate() {
            let inst = &self.instances[u.inst];
            let tid = u.target.map(|t| inst.targets[t].target_id.as_str());
            for &s in &u.solvers {
                let id = record::run_id(CAMPAIGN, &inst.instance_id, u.query.as_str(), tid, self.adapters[s].solver_id(), 0);
                if done.contains(&id) {
                    skipped += 1;
                } else {
                    jobs.push(Job { unit: ui, solver: s, rep: 0 });
                }
            }
        }
        if jobs.is_empty() {
            return Ok((0, skipped));
        }
        let ctx = Ctx {
            campaign: CAMPAIGN,
            instances: self.instances,
            units: &units,
            adapters: self.adapters,
            results_dir: self.cfg.results_dir,
            env_id: "truth",
            wall: Duration::from_secs(self.cfg.wall_secs),
            mem_kib: 0,
            out_cap: self.cfg.out_cap_kib * 1024,
            limiter: Limiter::None,
            cores: &[],
        };
        let total = jobs.len();
        let mut finished = 0usize;
        let mut failure: Option<String> = None;
        runner::run_pool(&jobs, self.cfg.parallel, |j, w| ctx.execute(j, w), |_, rec| {
            finished += 1;
            if finished.is_multiple_of(25) || finished == total {
                eprintln!("truth: {finished}/{total} labelling runs");
            }
            if let Err(e) = log.append(&rec) {
                failure.get_or_insert(e);
            }
            done.insert(rec.run_id.clone());
        });
        match failure {
            Some(e) => Err(e),
            None => Ok((total, skipped)),
        }
    }
}

/// Labels the `gen` instances of the manifests. See the module comment for the rule.
pub fn label(cfg: &LabelCfg) -> Result<Labelled, String> {
    let instances = sample(load_gen(cfg.manifests)?, cfg.sample, cfg.seed);
    let mut out = Labelled { instances: instances.len(), ..Labelled::default() };
    if instances.is_empty() {
        return Ok(out);
    }
    if let Some(i) = instances.iter().find(|i| !i.abs_path().is_file()) {
        return Err(format!("{}: net file missing", i.abs_path().display()));
    }
    let adapters: Vec<Box<dyn Adapter>> = TOOLS.iter().map(|s| adapters::build(s, cfg.solvers)).collect::<Result<_, _>>()?;
    for a in &adapters {
        let missing: Vec<String> = a.programs().into_iter().filter(|p| !which(p)).collect();
        if !missing.is_empty() {
            return Err(format!("{} is not installed (missing: {})", a.solver_id(), missing.join(", ")));
        }
    }
    let ex = Exec { cfg, instances: &instances, adapters: &adapters };
    let both: Vec<usize> = vec![0, 1];

    // Resume: everything recorded (except RUNNER_ERROR, which is retried) is done.
    let mut done: HashSet<String> = read_tool_records(cfg.results_dir).into_iter().filter(|(_, r)| r.verdict != Verdict::RunnerError).map(|(k, _)| k).collect();
    let mut log = RunLog::open(cfg.results_dir)?;

    // Phase 1: Q2 and every QR/QC target.
    let mut units = Vec::new();
    for (ii, inst) in instances.iter().enumerate() {
        units.push(Unit { inst: ii, query: Query::Q2, target: None, solvers: both.clone() });
        for (q, ti) in target_units(inst) {
            units.push(Unit { inst: ii, query: q, target: Some(ti), solvers: both.clone() });
        }
    }
    let (r, s) = ex.execute(units, &mut done, &mut log)?;
    out.ran += r;
    out.skipped += s;

    // Phase 2: Q1, Q3, Q4 unless Q2 truth is FALSE.
    let records = read_tool_records(cfg.results_dir);
    let q2_of = |inst: &Instance, records: &HashMap<String, RunRecord>| match decide(&tool_result(records, inst, Query::Q2, None, "lola"), &tool_result(records, inst, Query::Q2, None, "tina")) {
        Decision::Label(b) => Some(b),
        Decision::Excluded(_) => None,
    };
    let mut units = Vec::new();
    for (ii, inst) in instances.iter().enumerate() {
        let q2 = q2_of(inst, &records);
        for q in [Query::Q1, Query::Q3, Query::Q4] {
            if !out_of_scope(q, q2) {
                units.push(Unit { inst: ii, query: q, target: None, solvers: both.clone() });
            }
        }
    }
    let (r, s) = ex.execute(units, &mut done, &mut log)?;
    out.ran += r;
    out.skipped += s;

    // Decisions.
    let records = read_tool_records(cfg.results_dir);
    let mut truth: BTreeMap<(String, String, Option<String>), TruthLine> = BTreeMap::new();
    let mut excluded: BTreeMap<(String, String, Option<String>), Excluded> = BTreeMap::new();
    for inst in &instances {
        let q2 = q2_of(inst, &records);
        let mut cells: Vec<(Query, Option<String>)> = QUERY_ORDER.iter().filter(|q| !q.needs_target()).map(|q| (*q, None)).collect();
        cells.extend(target_units(inst).into_iter().map(|(q, ti)| (q, Some(inst.targets[ti].target_id.clone()))));
        for (q, tid) in cells {
            let row = |status: Status| Row { class: class_of(inst), query: q, status };
            if out_of_scope(q, q2) {
                out.rows.push(row(Status::OutOfScope));
                continue;
            }
            let (l, t) = (tool_result(&records, inst, q, tid.as_deref(), "lola"), tool_result(&records, inst, q, tid.as_deref(), "tina"));
            out.runner_errors += [&l, &t].iter().filter(|r| r.outcome == Verdict::RunnerError).count();
            let key = (inst.instance_id.clone(), q.as_str().to_owned(), tid.clone());
            match decide(&l, &t) {
                Decision::Label(b) => {
                    truth.insert(
                        key,
                        TruthLine {
                            instance_id: inst.instance_id.clone(),
                            query: q.as_str().into(),
                            target_id: tid.clone(),
                            value: Value::Bool(b),
                            source: "lola+tina".into(),
                            corpus_source: "gen".into(),
                            tools: TOOLS.iter().map(|s| (*s).to_owned()).collect(),
                        },
                    );
                    out.rows.push(row(Status::Labelled(b)));
                }
                Decision::Excluded(reason) => {
                    let entry = |r: &ToolResult| ToolEntry { tool: r.tool.clone(), outcome: r.outcome.as_str().into(), detail: r.detail.clone(), cmd: r.cmd.clone(), run_id: r.run_id.clone() };
                    excluded.insert(
                        key,
                        Excluded {
                            instance_id: inst.instance_id.clone(),
                            path: inst.path.clone(),
                            corpus_source: "gen".into(),
                            class: class_of(inst),
                            query: q.as_str().into(),
                            target_id: tid.clone(),
                            target: tid.as_deref().and_then(|t| target_text(inst, t)),
                            reason: reason.text(),
                            results: vec![entry(&l), entry(&t)],
                        },
                    );
                    out.rows.push(row(Status::Excluded(reason)));
                }
            }
        }
    }
    out.truth = truth.into_values().collect();
    out.excluded = excluded.into_values().collect();
    Ok(out)
}

pub fn excluded_path(results_dir: &Path) -> PathBuf {
    results_dir.join("truth_excluded.jsonl")
}

/// Rewrites `truth_excluded.jsonl` (derived from the runs, so it is replaced, not appended to).
pub fn write_excluded(results_dir: &Path, excluded: &[Excluded]) -> Result<(), String> {
    let mut text = String::new();
    for e in excluded {
        text.push_str(&serde_json::to_string(e).map_err(|e| e.to_string())?);
        text.push('\n');
    }
    let p = excluded_path(results_dir);
    std::fs::create_dir_all(results_dir).map_err(|e| e.to_string())?;
    std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))
}

/// One dense, pasteable block per LoLA/TINA disagreement.
pub fn render_disagreements(excluded: &[Excluded]) -> String {
    use std::fmt::Write;
    let mut o = String::new();
    let n = excluded.iter().filter(|e| e.reason == "disagree").count();
    if n == 0 {
        return o;
    }
    let _ = writeln!(o, "\nFINDING: {n} LoLA/TINA disagreement(s) (excluded from truth; each needs a look)");
    for e in excluded.iter().filter(|e| e.reason == "disagree") {
        let _ = writeln!(o, "DISAGREE instance={} path={} class={} query={} target={} marking={}", e.instance_id, e.path, e.class, e.query, e.target_id.as_deref().unwrap_or("-"), e.target.as_deref().unwrap_or("-"));
        for r in &e.results {
            let _ = writeln!(o, "  {}: {}{}", r.tool, r.outcome, r.detail.as_deref().map(|d| format!(" ({d})")).unwrap_or_default());
            let _ = writeln!(o, "    cmd: {}", r.cmd);
        }
    }
    o
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub considered: usize,
    pub labelled: usize,
    pub disagree: usize,
    pub inc_lola: usize,
    pub inc_tina: usize,
    pub both_inc: usize,
    pub out_of_scope: usize,
}

impl Counts {
    fn add(&mut self, s: &Status) {
        self.considered += 1;
        match s {
            Status::Labelled(_) => self.labelled += 1,
            Status::OutOfScope => self.out_of_scope += 1,
            Status::Excluded(Reason::Disagree) => self.disagree += 1,
            Status::Excluded(Reason::BothInconclusive) => self.both_inc += 1,
            Status::Excluded(Reason::Inconclusive(t)) if t == "lola" => self.inc_lola += 1,
            Status::Excluded(Reason::Inconclusive(_)) => self.inc_tina += 1,
        }
    }
    fn plus(&mut self, o: &Counts) {
        self.considered += o.considered;
        self.labelled += o.labelled;
        self.disagree += o.disagree;
        self.inc_lola += o.inc_lola;
        self.inc_tina += o.inc_tina;
        self.both_inc += o.both_inc;
        self.out_of_scope += o.out_of_scope;
    }
}

/// Counts per (query, class); queries in report order, classes alphabetical.
pub fn summarize(rows: &[Row]) -> BTreeMap<(usize, String), Counts> {
    let mut m: BTreeMap<(usize, String), Counts> = BTreeMap::new();
    for r in rows {
        let qi = QUERY_ORDER.iter().position(|q| *q == r.query).unwrap_or(usize::MAX);
        m.entry((qi, r.class.clone())).or_default().add(&r.status);
    }
    m
}

pub fn render_summary(rows: &[Row]) -> String {
    use std::fmt::Write;
    let mut o = String::new();
    let _ = writeln!(o, "\ngen labelling by LoLA+TINA (units = instance x query x target)");
    let _ = writeln!(o, "{:<5} {:<24} {:>10} {:>8} {:>8} {:>9} {:>9} {:>9} {:>8}", "query", "class", "considered", "labelled", "disagree", "inc:lola", "inc:tina", "both_inc", "out_scope");
    let mut totals: BTreeMap<usize, Counts> = BTreeMap::new();
    for ((qi, class), c) in summarize(rows) {
        let q = QUERY_ORDER.get(qi).map_or("?", |q| q.as_str());
        let _ = writeln!(o, "{q:<5} {class:<24} {:>10} {:>8} {:>8} {:>9} {:>9} {:>9} {:>8}", c.considered, c.labelled, c.disagree, c.inc_lola, c.inc_tina, c.both_inc, c.out_of_scope);
        totals.entry(qi).or_default().plus(&c);
    }
    let mut all = Counts::default();
    for (qi, c) in &totals {
        let q = QUERY_ORDER.get(*qi).map_or("?", |q| q.as_str());
        let _ = writeln!(o, "{q:<5} {:<24} {:>10} {:>8} {:>8} {:>9} {:>9} {:>9} {:>8}", "(all classes)", c.considered, c.labelled, c.disagree, c.inc_lola, c.inc_tina, c.both_inc, c.out_of_scope);
        all.plus(c);
    }
    let _ = writeln!(o, "{:<5} {:<24} {:>10} {:>8} {:>8} {:>9} {:>9} {:>9} {:>8}", "ALL", "", all.considered, all.labelled, all.disagree, all.inc_lola, all.inc_tina, all.both_inc, all.out_of_scope);
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{LolaCfg, NdrioCfg, TinaCfg};
    use std::os::unix::fs::PermissionsExt;

    fn tr(tool: &str, v: Verdict) -> ToolResult {
        ToolResult { tool: tool.into(), outcome: v, detail: None, cmd: String::new(), run_id: String::new() }
    }

    #[test]
    fn decision_rule() {
        use Verdict::*;
        let d = |a, b| decide(&tr("lola", a), &tr("tina", b));
        assert_eq!(d(True, True), Decision::Label(true));
        assert_eq!(d(False, False), Decision::Label(false));
        assert_eq!(d(True, False), Decision::Excluded(Reason::Disagree));
        assert_eq!(d(False, True), Decision::Excluded(Reason::Disagree));
        for bad in [Unknown, Timeout, Memout, NotSupported, Crash, InputRejected, RunnerError] {
            assert_eq!(d(True, bad), Decision::Excluded(Reason::Inconclusive("tina".into())), "{bad:?}");
            assert_eq!(d(bad, False), Decision::Excluded(Reason::Inconclusive("lola".into())), "{bad:?}");
            assert_eq!(d(bad, Unknown), Decision::Excluded(Reason::BothInconclusive), "{bad:?}");
        }
        assert_eq!(Reason::Inconclusive("tina".into()).text(), "inconclusive:tina");
        assert_eq!(Reason::BothInconclusive.text(), "both_inconclusive");
    }

    #[test]
    fn unbounded_scope_rule() {
        for q in [Query::Q1, Query::Q3, Query::Q4] {
            assert!(out_of_scope(q, Some(false)));
            assert!(!out_of_scope(q, Some(true)));
            assert!(!out_of_scope(q, None), "an inconclusive Q2 does not remove the other queries");
        }
        for q in [Query::Q2, Query::QR, Query::QC] {
            assert!(!out_of_scope(q, Some(false)), "{q:?}");
        }
    }

    #[test]
    fn summary_counts() {
        let row = |class: &str, q, status| Row { class: class.into(), query: q, status };
        let rows = vec![
            row("mg", Query::Q1, Status::Labelled(true)),
            row("mg", Query::Q1, Status::Excluded(Reason::Disagree)),
            row("mg", Query::Q1, Status::Excluded(Reason::Inconclusive("lola".into()))),
            row("sm", Query::Q1, Status::OutOfScope),
            row("sm", Query::QC, Status::Excluded(Reason::BothInconclusive)),
        ];
        let s = summarize(&rows);
        assert_eq!(s[&(0, "mg".to_string())], Counts { considered: 3, labelled: 1, disagree: 1, inc_lola: 1, ..Counts::default() });
        assert_eq!(s[&(0, "sm".to_string())].out_of_scope, 1);
        let text = render_summary(&rows);
        assert!(text.contains("(all classes)") && text.lines().any(|l| l.starts_with("ALL") && l.contains('5')));
    }

    // ---- end to end with fake tools ----

    fn mk(dir: &Path, name: &str, body: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p.display().to_string()
    }

    fn pnml(marker: &str) -> String {
        format!(
            "<?xml version=\"1.0\"?>\n<!-- {marker} -->\n<pnml><net id=\"n\"><page id=\"g\">\n<place id=\"p0\"><initialMarking><text>1</text></initialMarking></place>\n<place id=\"p1\"/>\n<transition id=\"t0\"/>\n<transition id=\"t1\"/>\n</page></net></pnml>\n"
        )
    }

    struct Env {
        dir: PathBuf,
        manifest: PathBuf,
        solvers: SolversCfg,
    }

    fn setup(name: &str) -> Env {
        let dir = std::env::temp_dir().join(format!("pe-truth-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tools = format!("{}/testdata/tools", env!("CARGO_MANIFEST_DIR"));
        std::fs::write(dir.join("gen-a.pnml"), pnml("BOUNDED")).unwrap();
        std::fs::write(dir.join("gen-b.pnml"), pnml("UNBOUNDED")).unwrap();
        let tgt = |id: &str, kind: &str, m: [u32; 2]| serde_json::json!({"target_id": id, "kind": kind, "marking": m});
        let lines = [
            serde_json::json!({"instance_id": "gen-a", "path": "gen-a.pnml", "source": "gen", "detected_class_testkit": "mg",
                "targets": [tgt("a-qr1", "QR", [0, 1]), tgt("a-qr2", "QR", [1, 1]), tgt("a-qc3", "QC", [1, 1]), tgt("a-qr4", "QR", [0, 7])]}),
            serde_json::json!({"instance_id": "gen-b", "path": "gen-b.pnml", "source": "gen", "detected_class_testkit": "free",
                "targets": [tgt("b-qr1", "QR", [0, 9]), tgt("b-qc2", "QC", [0, 2])]}),
            serde_json::json!({"instance_id": "fam-x", "path": "missing.pnml", "source": "fam"}),
        ];
        let manifest = dir.join("manifest.jsonl");
        std::fs::write(&manifest, lines.iter().map(|l| l.to_string() + "\n").collect::<String>()).unwrap();
        let calls = dir.join("lola_calls");
        let ndrio = mk(&dir, "ndrio", "cp \"$3\" \"$4\"");
        let lola = mk(
            &dir,
            "lola",
            &format!(
                "echo \"$*\" >> {calls}\nr=true\ncase \"$*\" in\n *'EF DEADLOCK'*) r=false;;\n *IMPOSSIBLE*) if grep -q UNBOUNDED \"$1\"; then r=false; fi;;\n *'p1 = 9'*|*'p1 = 7'*) r=null;;\n *'p0 = 1 AND p1 = 1'*) r=false;;\n *'p0 >= 1 AND p1 >= 1'*) r=false;;\nesac\necho \"{{\\\"analysis\\\": {{\\\"result\\\": $r}}}}\"",
                calls = calls.display()
            ),
        );
        let tina = mk(&dir, "tina", &format!("if grep -q UNBOUNDED \"$3\"; then echo 'not bounded'; else cat {tools}/tina_R_cyc.txt; fi"));
        let sift = mk(&dir, "sift", "for last; do :; done; : > \"$last\"; echo '2 marking(s), 2 transition(s)'");
        let scan = mk(&dir, "scan", "case \"$3\" in *'p1 = 7'*) echo what;; *'p0 = 1 /\\ p1 = 1'*) echo FALSE;; *) echo TRUE;; esac");
        let solvers = SolversCfg { lola: LolaCfg { command: lola }, ndrio: NdrioCfg { command: ndrio }, tina: TinaCfg { tina, sift, scan }, ..SolversCfg::default() };
        Env { dir, manifest, solvers }
    }

    fn cfg<'a>(e: &'a Env, manifests: &'a [PathBuf], results: &'a Path, sample: Option<usize>) -> LabelCfg<'a> {
        LabelCfg { manifests, results_dir: results, wall_secs: 60, parallel: 3, sample, seed: 1, solvers: &e.solvers, out_cap_kib: 64 }
    }

    fn keys(l: &Labelled) -> Vec<String> {
        l.truth.iter().map(|t| format!("{} {} {} {}", t.instance_id, t.query, t.target_id.as_deref().unwrap_or("-"), t.value)).collect()
    }

    #[test]
    fn end_to_end_with_fake_tools() {
        let e = setup("e2e");
        let results = e.dir.join("results");
        let ms = [e.manifest.clone()];
        let l = label(&cfg(&e, &ms, &results, None)).unwrap();
        assert_eq!(l.instances, 2);
        assert_eq!(
            keys(&l),
            [
                "gen-a Q1 - true", "gen-a Q2 - true", "gen-a Q3 - true", "gen-a Q4 - true", "gen-a QR a-qr1 true", "gen-a QR a-qr2 false",
                "gen-b Q2 - false", "gen-b QC b-qc2 true"
            ]
        );
        assert!(l.truth.iter().all(|t| t.source == "lola+tina" && t.corpus_source == "gen" && t.tools == ["lola", "tina"]));
        let ex: Vec<(String, String, String)> = l.excluded.iter().map(|x| (x.instance_id.clone(), x.target_id.clone().unwrap_or_default(), x.reason.clone())).collect();
        assert_eq!(
            ex,
            [
                ("gen-a".into(), "a-qc3".into(), "disagree".into()),
                ("gen-a".into(), "a-qr4".into(), "both_inconclusive".into()),
                ("gen-b".into(), "b-qr1".into(), "inconclusive:lola".into()),
            ]
        );
        assert_eq!(l.excluded[0].results.len(), 2);
        assert_eq!((l.excluded[0].results[0].outcome.as_str(), l.excluded[0].results[1].outcome.as_str()), ("FALSE", "TRUE"));
        assert!(l.excluded[0].results[0].cmd.contains("lola"));
        // unbounded net: Q1, Q3, Q4 neither run nor labelled
        let calls = std::fs::read_to_string(e.dir.join("lola_calls")).unwrap();
        assert!(!calls.lines().any(|c| c.contains("gen-b") && (c.contains("DEADLOCK") || c.contains("FIREABLE") || c.contains("AG ("))), "{calls}");
        assert!(calls.lines().any(|c| c.contains("gen-a") && c.contains("DEADLOCK")));
        assert_eq!(l.rows.iter().filter(|r| r.status == Status::OutOfScope).count(), 3);
        assert_eq!(l.ran, 22, "phase 1: 8 units, phase 2: 3 units (gen-a only), two tools each");
        // files
        write_excluded(&results, &l.excluded).unwrap();
        assert_eq!(std::fs::read_to_string(excluded_path(&results)).unwrap().lines().count(), 3);
        let runs = std::fs::read_to_string(record::runs_path(&results)).unwrap();
        assert!(runs.lines().all(|l| l.contains("\"campaign\":\"TRUTH\"")));
        assert!(results.join("raw").is_dir());
        let text = render_disagreements(&l.excluded);
        assert!(text.contains("DISAGREE instance=gen-a") && text.contains("query=QC") && text.contains("lola: FALSE") && text.contains("tina: TRUE") && text.contains("cmd:"));
        let sum = render_summary(&l.rows);
        assert!(sum.contains("mg") && sum.contains("free"));
    }

    #[test]
    fn second_invocation_runs_nothing() {
        let e = setup("resume");
        let results = e.dir.join("results");
        let ms = [e.manifest.clone()];
        let first = label(&cfg(&e, &ms, &results, None)).unwrap();
        let n = std::fs::read_to_string(record::runs_path(&results)).unwrap().lines().count();
        assert_eq!(n, first.ran);
        let second = label(&cfg(&e, &ms, &results, None)).unwrap();
        assert_eq!(second.ran, 0);
        assert_eq!(second.skipped, first.ran);
        assert_eq!(std::fs::read_to_string(record::runs_path(&results)).unwrap().lines().count(), n);
        assert_eq!(keys(&first), keys(&second));
        assert_eq!(first.excluded.len(), second.excluded.len());
    }

    #[test]
    fn petrivet_results_are_never_consulted() {
        let e = setup("pv");
        let results = e.dir.join("results");
        let ms = [e.manifest.clone()];
        let before = label(&cfg(&e, &ms, &results, None)).unwrap();
        // Inject petrivet-looking records: every existing record cloned as pv-*, with flipped verdicts, in campaign TRUTH
        // and in campaign B; also a pv record under a tool id in another campaign.
        let path = record::runs_path(&results);
        let text = std::fs::read_to_string(&path).unwrap();
        let mut extra = String::new();
        for line in text.lines() {
            let mut v: Value = serde_json::from_str(line).unwrap();
            let flipped = if v["verdict"] == "TRUE" { "FALSE" } else { "TRUE" };
            v["verdict"] = flipped.into();
            for (campaign, solver) in [("TRUTH", "pv-dispatch"), ("B", "pv-general"), ("B", "lola"), ("CAL", "tina")] {
                v["campaign"] = campaign.into();
                v["solver_id"] = solver.into();
                v["run_id"] = format!("{campaign}-{solver}-{}", v["run_id"].as_str().unwrap()).into();
                extra.push_str(&format!("{v}\n"));
            }
        }
        let mut all = text;
        all.push_str(&extra);
        std::fs::write(&path, all).unwrap();
        let after = label(&cfg(&e, &ms, &results, None)).unwrap();
        assert_eq!(after.ran, 0);
        assert_eq!(keys(&before), keys(&after));
        let reasons = |l: &Labelled| l.excluded.iter().map(|x| x.reason.clone()).collect::<Vec<_>>();
        assert_eq!(reasons(&before), reasons(&after));
        assert_eq!(read_tool_records(&results).len(), before.ran);
    }

    #[test]
    fn sample_is_seeded_and_small() {
        let e = setup("sample");
        let results = e.dir.join("results");
        let ms = [e.manifest.clone()];
        let l = label(&cfg(&e, &ms, &results, Some(1))).unwrap();
        assert_eq!(l.instances, 1);
        let again = label(&cfg(&e, &ms, &e.dir.join("results2"), Some(1))).unwrap();
        assert_eq!(keys(&l), keys(&again));
        assert!(l.truth.iter().map(|t| &t.instance_id).collect::<HashSet<_>>().len() == 1);
    }

    #[test]
    fn missing_tool_is_an_infrastructure_error() {
        let mut e = setup("missing");
        e.solvers.lola.command = "/nonexistent/lola".into();
        let ms = [e.manifest.clone()];
        let err = label(&cfg(&e, &ms, &e.dir.join("results"), None)).err().unwrap();
        assert!(err.contains("lola is not installed"), "{err}");
    }
}
