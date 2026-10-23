//! `petrivet-eval calibrate`: runs the selected solvers on the hand-checked nets of `calibration/manifest.jsonl`
//! (campaign `CAL`, one repetition, no pinning) through the same job machinery as `run` (conversion step, classification),
//! compares every definite answer with the hand answers and prints one compact table per solver plus one block per
//! finding. Full results go to `<results_dir>/calibration.json`. Exit status is non-zero if any answer disagrees.
//!
//! Rules of comparison (SPEC section 6, CALIBRATE):
//! * Q0 compares states and edges; the other queries compare TRUE/FALSE.
//! * On unbounded nets (hand Q2 = false) only Q2 and the coverability targets (QC) are compared; every other query
//!   is out of scope there and is reported as informational (own column, no AGREE/DISAGREE counting).
//! * UNKNOWN on an unbounded net is expected and gets no block; on bounded nets it gets a one line block (a
//!   tool-incompleteness note, not a failure). NOT_SUPPORTED (declared by the tool or by the adapter) only shows in
//!   the table, never in a block; every DISAGREE and every other non-answer (INPUT_REJECTED,
//!   CRASH, TIMEOUT/MEMOUT, RUNNER_ERROR) gets the full block.
//! * Queries a solver does not support at all (`supports` false, `pv-poly` on Q0, Q6, QR, QC) are counted as
//!   not_supported without running anything and get no block.

use crate::adapters::{self, Adapter, Query};
use crate::config::Config;
use crate::instance::{Instance, TargetMarking};
use crate::manifest_mcc::KnownAnswer;
use crate::record::{RunRecord, Verdict};
use crate::run::{self, Ctx, Job, Unit};
use crate::runner::{self, Limiter};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(clap::Args)]
pub struct CalibrateArgs {
    /// Comma separated solver ids
    #[arg(long, value_delimiter = ',', default_value = "pv-dispatch,pv-general,pv-poly,lola,tina")]
    solvers: Vec<String>,
    /// Output directory (default: results/calibration next to eval.toml)
    #[arg(long)]
    results_dir: Option<PathBuf>,
    /// External wall-clock cap per run
    #[arg(long, default_value_t = 30)]
    wall_cap_secs: u64,
    /// Calibration manifest (default: calibration/manifest.jsonl next to eval.toml)
    #[arg(long)]
    manifest: Option<PathBuf>,
}

const QUERIES: [Query; 9] = [Query::Q0, Query::Q1, Query::Q2, Query::Q3, Query::Q4, Query::Q5, Query::Q6, Query::QR, Query::QC];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bucket {
    Agree,
    Disagree,
    Unknown,
    NotSupported,
    InputRejected,
    Crash,
    Timeout,
    RunnerError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Judged {
    pub bucket: Bucket,
    /// Out of scope for the comparison (Q0, Q1, Q3..Q6, QR on an unbounded net): counted in its own column.
    pub informational: bool,
}

fn matches_hand(query: Query, hand: &Value, v: Verdict, states: Option<u64>, edges: Option<u64>) -> bool {
    if query == Query::Q0 {
        return v == Verdict::Count && hand.get("STATES").and_then(Value::as_u64) == states && hand.get("TRANSITIONS").and_then(Value::as_u64) == edges && states.is_some();
    }
    match (v, hand.as_bool()) {
        (Verdict::True, Some(b)) => b,
        (Verdict::False, Some(b)) => !b,
        _ => false,
    }
}

/// Judges one result. `hand` is `None` where no hand answer exists (Q0 on an unbounded net): a definite answer
/// there is a disagreement (the net has no finite state space).
pub fn judge(query: Query, unbounded: bool, hand: Option<&Value>, verdict: Verdict, states: Option<u64>, edges: Option<u64>) -> Judged {
    let informational = unbounded && !matches!(query, Query::Q2 | Query::QC);
    let bucket = match verdict {
        Verdict::True | Verdict::False | Verdict::Count => match hand {
            Some(h) if matches_hand(query, h, verdict, states, edges) => Bucket::Agree,
            _ => Bucket::Disagree,
        },
        Verdict::Unknown => Bucket::Unknown,
        Verdict::NotSupported => Bucket::NotSupported,
        Verdict::InputRejected => Bucket::InputRejected,
        Verdict::Crash => Bucket::Crash,
        Verdict::Timeout | Verdict::Memout => Bucket::Timeout,
        Verdict::RunnerError | Verdict::ShardFailed => Bucket::RunnerError,
    };
    Judged { bucket, informational }
}

/// Does the result get a block, and a full one? `(print, full)`.
fn block_kind(j: Judged, unbounded: bool) -> (bool, bool) {
    match j.bucket {
        Bucket::Agree => (false, false),
        Bucket::NotSupported => (false, false),
        Bucket::Unknown => (!unbounded, false),
        _ => (true, true),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Cell {
    pub solver: String,
    pub net: String,
    pub query: String,
    pub target: Option<String>,
    pub hand: Option<Value>,
    pub unbounded: bool,
    pub bucket: Bucket,
    pub informational: bool,
    pub verdict: String,
    pub detail: Option<String>,
    pub states: Option<u64>,
    pub edges: Option<u64>,
    pub wall_ms: f64,
    pub convert_ms: Option<f64>,
    pub cmd: String,
    pub stdout: String,
    pub stderr: String,
    /// The solver does not offer the query at all (nothing was run).
    pub static_unsupported: bool,
}

struct Hand {
    all: Vec<KnownAnswer>,
}

impl Hand {
    fn get(&self, query: Query, target_id: Option<&str>) -> Option<&Value> {
        self.all.iter().find(|k| k.origin == "hand" && k.query == query.as_str() && k.target_id.as_deref() == target_id).map(|k| &k.verdict)
    }
    fn unbounded(&self) -> bool {
        self.get(Query::Q2, None).and_then(Value::as_bool) == Some(false)
    }
}

/// Instances and their hand answers. `known_answers` are read separately because their verdicts are booleans or
/// objects, which the plain instance loader does not accept.
fn load_manifest(path: &Path) -> Result<Vec<(Instance, Hand)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let base = path.canonicalize().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
        let at = |e: String| format!("{}:{}: {e}", path.display(), i + 1);
        let mut v: Value = serde_json::from_str(line).map_err(|e| at(e.to_string()))?;
        let known = v.as_object_mut().and_then(|o| o.remove("known_answers")).unwrap_or(Value::Array(vec![]));
        let all: Vec<KnownAnswer> = serde_json::from_value(known).map_err(|e| at(e.to_string()))?;
        let mut inst: Instance = serde_json::from_value(v).map_err(|e| at(e.to_string()))?;
        inst.base = base.clone();
        out.push((inst, Hand { all }));
    }
    Ok(out)
}

fn which(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(program).is_file()))
}

fn read_raw(results_dir: &Path, hash: &Option<String>) -> String {
    hash.as_ref().and_then(|h| std::fs::read(results_dir.join("raw").join(&h[..2]).join(h)).ok()).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

fn head(text: &str, n: usize) -> String {
    let t: String = text.chars().take(n).collect();
    let t = t.replace('\n', "\\n");
    if text.chars().count() > n { format!("{t}...") } else { t }
}

fn net_name(inst: &Instance) -> String {
    Path::new(&inst.path).file_stem().map_or_else(|| inst.path.clone(), |s| s.to_string_lossy().into_owned())
}

fn target_label(inst: &Instance, tid: &str) -> String {
    let Some(t) = inst.targets.iter().find(|t| t.target_id == tid) else { return tid.to_owned() };
    match &t.marking {
        TargetMarking::Vector(v) => format!("{}{}", t.kind, serde_json::to_string(v).unwrap_or_default()),
        TargetMarking::Map(m) => format!("{}{}", t.kind, serde_json::to_string(m).unwrap_or_default()),
    }
}

pub fn run(cfg: &Config, config_path: &Path, args: CalibrateArgs) -> Result<(), String> {
    let _ = cfg;
    let (rcfg, scfg) = run::load_sections(config_path)?;
    let cfg_dir = config_path.canonicalize().map_err(|e| e.to_string())?.parent().unwrap().to_path_buf();
    let results_dir = args.results_dir.clone().unwrap_or_else(|| cfg_dir.join("results/calibration"));
    let manifest = args.manifest.clone().unwrap_or_else(|| cfg_dir.join("calibration/manifest.jsonl"));
    let loaded = load_manifest(&manifest)?;
    let instances: Vec<Instance> = loaded.iter().map(|(i, _)| i.clone()).collect();
    for i in &instances {
        if !i.abs_path().is_file() {
            return Err(format!("{}: net file missing", i.abs_path().display()));
        }
    }
    let adapters: Vec<Box<dyn Adapter>> = args.solvers.iter().map(|s| adapters::build(s, &scfg)).collect::<Result<_, _>>()?;
    std::fs::create_dir_all(&results_dir).map_err(|e| format!("{}: {e}", results_dir.display()))?;

    // Installed solvers run; the others are reported as not installed.
    let missing: Vec<Vec<String>> = adapters.iter().map(|a| a.programs().into_iter().filter(|p| !which(p)).collect()).collect();
    let installed: Vec<usize> = (0..adapters.len()).filter(|i| missing[*i].is_empty()).collect();

    let mut units: Vec<Unit> = Vec::new();
    let mut statics: Vec<(usize, usize, Query, Option<usize>)> = Vec::new(); // (solver, instance, query, target)
    for (ii, inst) in instances.iter().enumerate() {
        for q in QUERIES {
            let targets: Vec<Option<usize>> = if q.needs_target() {
                inst.targets.iter().enumerate().filter(|(_, t)| t.kind.eq_ignore_ascii_case(q.as_str())).map(|(i, _)| Some(i)).collect()
            } else {
                vec![None]
            };
            for t in targets {
                let (run_on, not): (Vec<usize>, Vec<usize>) = installed.iter().partition(|s| adapters[**s].supports(q));
                statics.extend(not.iter().map(|s| (*s, ii, q, t)));
                if !run_on.is_empty() {
                    units.push(Unit { inst: ii, query: q, target: t, solvers: run_on });
                }
            }
        }
    }
    let limiter = Limiter::resolve(&rcfg.mem_limiter)?;
    let ctx = Ctx {
        campaign: "CAL",
        instances: &instances,
        units: &units,
        adapters: &adapters,
        results_dir: &results_dir,
        env_id: "calibrate",
        wall: Duration::from_secs(args.wall_cap_secs),
        mem_kib: rcfg.mem_limit_gib * 1024 * 1024,
        out_cap: rcfg.out_cap_kib * 1024,
        limiter,
        cores: &[],
    };
    let jobs: Vec<Job> = units.iter().enumerate().flat_map(|(ui, u)| u.solvers.iter().map(move |&s| Job { unit: ui, solver: s, rep: 0 })).collect();
    eprintln!("calibrate: {} nets, {} runs, wall cap {} s, solvers {}", instances.len(), jobs.len(), args.wall_cap_secs, args.solvers.join(","));

    let make_cell = |solver: &str, ii: usize, q: Query, t: Option<usize>, rec: Option<&RunRecord>| -> Cell {
        let (inst, hand) = (&loaded[ii].0, &loaded[ii].1);
        let tid = t.map(|t| inst.targets[t].target_id.as_str());
        let h = hand.get(q, tid).cloned();
        let unbounded = hand.unbounded();
        let (verdict, states, edges) = rec.map_or((Verdict::NotSupported, None, None), |r| (r.verdict, r.states, r.edges));
        let j = judge(q, unbounded, h.as_ref(), verdict, states, edges);
        Cell {
            solver: solver.to_owned(),
            net: net_name(inst),
            query: q.as_str().to_owned(),
            target: tid.map(|t| target_label(inst, t)),
            hand: h,
            unbounded,
            bucket: j.bucket,
            informational: j.informational,
            verdict: verdict.as_str().to_owned(),
            detail: rec.and_then(|r| r.detail.clone()),
            states,
            edges,
            wall_ms: rec.map_or(0.0, |r| r.wall_ms),
            convert_ms: rec.and_then(|r| r.convert_ms),
            cmd: rec.map(|r| r.cmd.clone()).unwrap_or_default(),
            stdout: rec.map(|r| read_raw(&results_dir, &r.raw_hash)).unwrap_or_default(),
            stderr: rec.map(|r| read_raw(&results_dir, &r.stderr_hash)).unwrap_or_default(),
            static_unsupported: rec.is_none(),
        }
    };

    let mut cells: Vec<Cell> = Vec::new();
    let total = jobs.len();
    let mut done = 0usize;
    runner::run_pool(&jobs, rcfg.parallel_jobs, |j, w| ctx.execute(j, w), |ji, rec| {
        let j = &jobs[ji];
        let u = &units[j.unit];
        cells.push(make_cell(adapters[j.solver].solver_id(), u.inst, u.query, u.target, Some(&rec)));
        done += 1;
        if done.is_multiple_of(50) {
            eprintln!("calibrate: {done}/{total}");
        }
    });
    for (s, ii, q, t) in statics {
        cells.push(make_cell(adapters[s].solver_id(), ii, q, t, None));
    }
    let order = |c: &Cell| (args.solvers.iter().position(|s| *s == c.solver), c.net.clone(), c.query.clone(), c.target.clone());
    cells.sort_by_key(order);

    let not_installed: Vec<(String, Vec<String>)> =
        args.solvers.iter().zip(&missing).filter(|(_, m)| !m.is_empty()).map(|(s, m)| (s.clone(), m.clone())).collect();
    let disagreements = cells.iter().filter(|c| c.bucket == Bucket::Disagree && !c.informational).count();
    let report = render(&args.solvers, &not_installed, &cells, args.wall_cap_secs);
    print!("{report}");

    let json = serde_json::json!({
        "campaign": "CAL", "wall_cap_secs": args.wall_cap_secs, "solvers": args.solvers,
        "not_installed": not_installed.iter().map(|(s, m)| serde_json::json!({"solver": s, "missing": m})).collect::<Vec<_>>(),
        "disagreements": disagreements, "cells": cells,
    });
    let out = results_dir.join("calibration.json");
    std::fs::write(&out, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("full results: {}", out.display());
    if disagreements > 0 {
        return Err(format!("{disagreements} answers disagree with the hand answers"));
    }
    Ok(())
}

/// The tables and finding blocks as text.
pub fn render(solvers: &[String], not_installed: &[(String, Vec<String>)], cells: &[Cell], wall_cap_secs: u64) -> String {
    use std::fmt::Write;
    let mut o = String::new();
    let _ = writeln!(o, "CALIBRATE (hand answers; wall cap {wall_cap_secs} s). Columns: agree disagree unknown not_supported input_rejected crash timeout(+memout) runner_error | info = out of scope (unbounded nets: all but Q2, QC)");
    for s in solvers {
        if let Some((_, m)) = not_installed.iter().find(|(n, _)| n == s) {
            let _ = writeln!(o, "\n{s}: not installed (missing: {})", m.join(", "));
            continue;
        }
        let _ = writeln!(o, "\n{s}");
        let _ = writeln!(o, "{:<3} {:>5} {:>8} {:>7} {:>13} {:>14} {:>5} {:>7} {:>12} {:>4}", "", "agree", "disagree", "unknown", "not_supported", "input_rejected", "crash", "timeout", "runner_error", "info");
        for q in QUERIES {
            let mut n = [0usize; 8];
            let mut info = 0;
            for c in cells.iter().filter(|c| c.solver == *s && c.query == q.as_str()) {
                if c.informational {
                    info += 1;
                } else {
                    n[c.bucket as usize] += 1;
                }
            }
            let f = |x: usize| if x == 0 { ".".to_owned() } else { x.to_string() };
            let _ = writeln!(o, "{:<3} {:>5} {:>8} {:>7} {:>13} {:>14} {:>5} {:>7} {:>12} {:>4}", q.as_str(), f(n[0]), f(n[1]), f(n[2]), f(n[3]), f(n[4]), f(n[5]), f(n[6]), f(n[7]), f(info));
        }
    }
    let mut findings = String::new();
    for c in cells.iter().filter(|c| !c.static_unsupported) {
        let j = Judged { bucket: c.bucket, informational: c.informational };
        let (print, full) = block_kind(j, c.unbounded);
        if !print {
            continue;
        }
        let tag = match (c.bucket, c.informational) {
            (Bucket::Disagree, false) => "DISAGREE".to_owned(),
            (Bucket::Disagree, true) => "INFO-DISAGREE".to_owned(),
            (b, _) => format!("{b:?}").to_ascii_uppercase(),
        };
        let hand = c.hand.as_ref().map_or("none".to_owned(), |h| if c.query == "Q0" { format!("{}/{}", h["STATES"], h["TRANSITIONS"]) } else { h.to_string() });
        let got = if c.verdict == "COUNT" { format!("COUNT {}/{}", c.states.unwrap_or(0), c.edges.unwrap_or(0)) } else { c.verdict.clone() };
        let _ = writeln!(
            findings,
            "[{tag}] {} {} {} {}{} hand={hand} got={got}{}",
            c.solver,
            c.net,
            c.query,
            c.target.as_deref().unwrap_or("-"),
            if c.unbounded { " (unbounded)" } else { "" },
            c.detail.as_deref().map(|d| format!(" ({d})")).unwrap_or_default()
        );
        if full {
            let cmd: String = c.cmd.chars().take(3000).collect();
            let _ = writeln!(findings, "  cmd: {cmd}");
            let _ = writeln!(findings, "  stdout: {}", head(&c.stdout, 300));
            let _ = writeln!(findings, "  stderr: {}", head(&c.stderr, 300));
        }
    }
    if findings.is_empty() {
        let _ = writeln!(o, "\nno findings");
    } else {
        let _ = writeln!(o, "\nFINDINGS (full blocks for DISAGREE and failures; one line for UNKNOWN on bounded nets)");
        o.push_str(&findings);
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn j(q: Query, unb: bool, hand: Option<Value>, v: Verdict, s: Option<u64>, e: Option<u64>) -> (Bucket, bool) {
        let r = judge(q, unb, hand.as_ref(), v, s, e);
        (r.bucket, r.informational)
    }

    #[test]
    fn agreement_and_disagreement() {
        let t = Some(json!(true));
        assert_eq!(j(Query::Q1, false, t.clone(), Verdict::True, None, None), (Bucket::Agree, false));
        assert_eq!(j(Query::Q1, false, t.clone(), Verdict::False, None, None), (Bucket::Disagree, false));
        assert_eq!(j(Query::Q1, false, Some(json!(false)), Verdict::False, None, None), (Bucket::Agree, false));
        // a COUNT for a boolean query is a disagreement
        assert_eq!(j(Query::Q1, false, t, Verdict::Count, Some(1), Some(1)).0, Bucket::Disagree);
        // Q0 compares both numbers
        let h = Some(json!({"STATES": 5, "TRANSITIONS": 6}));
        assert_eq!(j(Query::Q0, false, h.clone(), Verdict::Count, Some(5), Some(6)).0, Bucket::Agree);
        assert_eq!(j(Query::Q0, false, h.clone(), Verdict::Count, Some(5), Some(7)).0, Bucket::Disagree);
        assert_eq!(j(Query::Q0, false, h, Verdict::True, None, None).0, Bucket::Disagree);
    }

    #[test]
    fn non_answers_are_bucketed() {
        let t = Some(json!(true));
        let b = |v| j(Query::Q3, false, t.clone(), v, None, None).0;
        assert_eq!(b(Verdict::Unknown), Bucket::Unknown);
        assert_eq!(b(Verdict::NotSupported), Bucket::NotSupported);
        assert_eq!(b(Verdict::InputRejected), Bucket::InputRejected);
        assert_eq!(b(Verdict::Crash), Bucket::Crash);
        assert_eq!(b(Verdict::Timeout), Bucket::Timeout);
        assert_eq!(b(Verdict::Memout), Bucket::Timeout);
        assert_eq!(b(Verdict::RunnerError), Bucket::RunnerError);
    }

    #[test]
    fn unbounded_nets_compare_only_q2_and_qc() {
        let t = Some(json!(false));
        // informational: Q1, Q3, Q4 (and the others) are reported but not counted
        for q in [Query::Q0, Query::Q1, Query::Q3, Query::Q4, Query::Q5, Query::Q6, Query::QR] {
            assert!(j(q, true, t.clone(), Verdict::Unknown, None, None).1, "{q:?}");
        }
        assert_eq!(j(Query::Q2, true, t.clone(), Verdict::False, None, None), (Bucket::Agree, false));
        assert_eq!(j(Query::Q2, true, t.clone(), Verdict::True, None, None), (Bucket::Disagree, false));
        assert_eq!(j(Query::QC, true, Some(json!(true)), Verdict::False, None, None), (Bucket::Disagree, false));
        // a state count for a net without a finite state space is wrong, but informational
        assert_eq!(j(Query::Q0, true, None, Verdict::Count, Some(10), Some(10)), (Bucket::Disagree, true));
        // blocks: unknown on an unbounded net is silent, on a bounded one it is a one line block
        let unk = Judged { bucket: Bucket::Unknown, informational: false };
        assert_eq!(block_kind(unk, true), (false, false));
        assert_eq!(block_kind(unk, false), (true, false));
        assert_eq!(block_kind(Judged { bucket: Bucket::NotSupported, informational: false }, false), (false, false));
        assert_eq!(block_kind(Judged { bucket: Bucket::Crash, informational: true }, true), (true, true));
        assert_eq!(block_kind(Judged { bucket: Bucket::Disagree, informational: false }, false), (true, true));
        assert_eq!(block_kind(Judged { bucket: Bucket::Agree, informational: false }, false), (false, false));
    }

    fn cell(solver: &str, q: &str, bucket: Bucket, unb: bool, info: bool) -> Cell {
        Cell {
            solver: solver.into(), net: "cal-01".into(), query: q.into(), target: None, hand: Some(json!(true)), unbounded: unb, bucket, informational: info,
            verdict: "FALSE".into(), detail: None, states: None, edges: None, wall_ms: 1.0, convert_ms: None, cmd: "lola N.lola".into(),
            stdout: "x".repeat(400), stderr: "bad\nline".into(), static_unsupported: false,
        }
    }

    #[test]
    fn rendering_table_and_blocks() {
        let cells = vec![
            cell("lola", "Q1", Bucket::Agree, false, false),
            cell("lola", "Q1", Bucket::Disagree, false, false),
            cell("lola", "Q3", Bucket::Unknown, true, true),
            cell("lola", "Q4", Bucket::Unknown, false, false),
            cell("lola", "Q5", Bucket::Crash, true, true),
        ];
        let out = render(&["lola".into(), "tina".into()], &[("tina".into(), vec!["tina".into(), "sift".into()])], &cells, 30);
        assert!(out.contains("tina: not installed (missing: tina, sift)"));
        let q1 = out.lines().find(|l| l.starts_with("Q1 ")).unwrap();
        assert_eq!(q1.split_whitespace().collect::<Vec<_>>(), ["Q1", "1", "1", ".", ".", ".", ".", ".", ".", "."]);
        let q3 = out.lines().find(|l| l.starts_with("Q3 ")).unwrap();
        assert_eq!(q3.split_whitespace().last(), Some("1"), "informational column");
        // one full block for the disagreement and for the crash; a short one for the bounded unknown; none for the unbounded unknown
        assert_eq!(out.matches("[DISAGREE] lola cal-01 Q1").count(), 1);
        assert!(out.contains("[UNKNOWN] lola cal-01 Q4"));
        assert!(!out.contains("lola cal-01 Q3 "));
        assert!(out.contains("[CRASH] lola cal-01 Q5"));
        assert!(out.contains("  cmd: lola N.lola"));
        assert!(out.contains(&format!("  stdout: {}...", "x".repeat(300))));
        assert!(out.contains("  stderr: bad\\nline"));
    }

    #[test]
    fn manifest_with_boolean_and_object_answers_loads() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../calibration/manifest.jsonl");
        let loaded = load_manifest(Path::new(path)).unwrap();
        assert_eq!(loaded.len(), 30);
        let unbounded = loaded.iter().filter(|(_, h)| h.unbounded()).count();
        assert_eq!(unbounded, 5);
        let (inst, hand) = loaded.iter().find(|(i, _)| i.path.starts_with("cal-10")).unwrap();
        assert_eq!(hand.get(Query::Q0, None).unwrap()["STATES"], 5);
        assert_eq!(hand.get(Query::QR, Some(&inst.targets[0].target_id)), Some(&json!(true)));
        assert!(inst.targets.iter().all(|t| matches!(t.marking, TargetMarking::Vector(_))));
    }
}
