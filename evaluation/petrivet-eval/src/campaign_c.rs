//! Campaign C of SPEC 9 (timeout behaviour, E3b): `pv-dispatch` with `--timeout-ms t` for every t of
//! `[run] c_timeouts_ms`, on the cells (instance, query, target) of campaign B whose unconstrained `pv-dispatch`
//! run took more than 2 x the largest t, so that the deadline actually triggers. One run per (cell, t), three
//! repetitions if the first run is under 1 s (only TRUE, FALSE, COUNT and UNKNOWN are repeated).
//!
//! Records go to `runs.jsonl` with campaign `C`. The run_id is `record::run_id("C", instance, query, target,
//! "<solver>@t<t>", rep)`, so t is part of the id and an interrupted sweep resumes cell by cell. Per record
//! `timeout_ms` is t, `analyze_ms` is petrivet's in-process analysis time (parse excluded), `wall_ms` the external
//! wall time, and `note` starts with `overshoot_ms=<analyze_ms - t>` (absent if petrivet reported no time).
//! UNKNOWN (the deadline fired or the procedure gave up) against a definite verdict is the `verdict` field.
//!
//! `--instances a,b` picks the cells by hand instead (the pilot): no B results, no threshold, queries default
//! to Q1,Q3. `--results-subdir pilot` writes everything under `<results>/pilot/`.

use super::{Brief, Ctx, Job, RunCfg, RunArgs, Unit, in_shard, load_manifest, load_sections, parse_shard, target_path, target_text, to_brief};
use crate::adapters::{self, Adapter, CommandSpec, Finished, Parsed, Query, Request, ToolInfo};
use crate::config::Config;
use crate::env::{self, EnvInput};
use crate::instance::Instance;
use crate::record::{self, RunLog, RunRecord, Verdict};
use crate::runner::{self, Limiter};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::time::Duration;

const SOLVER: &str = "pv-dispatch";

/// Repetitions of a cell whose first run is under one second (SPEC 9 C).
const SHORT_REPS: u32 = 3;

/// The wrapped adapter with `--timeout-ms t`; it reports the id of the wrapped solver.
struct Timed {
    inner: Box<dyn Adapter>,
    t: u64,
}

impl Adapter for Timed {
    fn solver_id(&self) -> &str {
        self.inner.solver_id()
    }
    fn supports(&self, query: Query) -> bool {
        self.inner.supports(query)
    }
    fn command(&self, req: &Request) -> CommandSpec {
        self.inner.command(&Request {
            instance_path: req.instance_path,
            query: req.query,
            target_file: req.target_file,
            internal_timeout_ms: Some(self.t),
            converted: req.converted,
        })
    }
    fn parse(&self, stdout: &str, exit_code: i32) -> Parsed {
        self.inner.parse(stdout, exit_code)
    }
    fn nonzero_exit(&self, stdout: &str, stderr: &str, exit_code: i32) -> Parsed {
        self.inner.nonzero_exit(stdout, stderr, exit_code)
    }
    fn classify(&self, f: &Finished) -> Parsed {
        self.inner.classify(f)
    }
    fn classify_for(&self, query: Query, f: &Finished) -> Parsed {
        self.inner.classify_for(query, f)
    }
    fn tool(&self) -> Option<ToolInfo> {
        self.inner.tool()
    }
    fn converter(&self) -> Option<String> {
        self.inner.converter()
    }
    fn programs(&self) -> Vec<String> {
        self.inner.programs()
    }
}

type CellKey = (String, String, String);

/// Cells of B's `runs.jsonl` for `pv-dispatch` whose median unconstrained wall time exceeds `threshold_ms`.
/// Runs that ended in TIMEOUT count (they took at least the cap); crashes, rejected inputs, memouts, runner
/// errors and unsupported queries do not. Returns the slow cells and the number of cells seen.
fn slow_cells(runs_jsonl: &str, threshold_ms: f64) -> (BTreeSet<CellKey>, usize) {
    let mut walls: BTreeMap<CellKey, Vec<f64>> = BTreeMap::new();
    for line in runs_jsonl.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        if v["campaign"] != "B" || v["solver_id"] != SOLVER {
            continue;
        }
        let Some(verdict) = v["verdict"].as_str().and_then(Verdict::from_name) else { continue };
        if !(verdict.repeatable() || verdict == Verdict::Timeout) {
            continue;
        }
        let Some(wall) = v["wall_ms"].as_f64() else { continue };
        let s = |k: &str| v[k].as_str().unwrap_or("").to_owned();
        walls.entry((s("instance_id"), s("query"), s("target_id"))).or_default().push(wall);
    }
    let seen = walls.len();
    let slow = walls.into_iter().filter(|(_, w)| median(w) > threshold_ms).map(|(k, _)| k).collect();
    (slow, seen)
}

fn median(v: &[f64]) -> f64 {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// Which cells to plan.
enum Filter {
    /// Hand-picked instances (pilot): every query cell of these instances.
    Instances(HashSet<usize>),
    /// The slow cells of B.
    Cells(BTreeSet<CellKey>),
}

/// Units in manifest order: one per cell passing the filter, solvers = indexes into the per-t adapter list.
fn plan_units(instances: &[Instance], adapter: &dyn Adapter, queries: &[Query], filter: &Filter, shard: Option<(u64, u64)>, n_ts: usize) -> Vec<Unit> {
    let mut units = Vec::new();
    for (ii, inst) in instances.iter().enumerate() {
        if shard.is_some_and(|sh| !in_shard(&inst.instance_id, sh)) || !inst.abs_path().is_file() {
            continue;
        }
        let wanted = |q: Query, tid: &str| match filter {
            Filter::Instances(set) => set.contains(&ii),
            Filter::Cells(c) => c.contains(&(inst.instance_id.clone(), q.as_str().to_owned(), tid.to_owned())),
        };
        for &q in queries {
            if !q.applies_to_source(&inst.source) || !adapter.supports(q) {
                continue;
            }
            if q.needs_target() {
                for (ti, t) in inst.targets.iter().enumerate().filter(|(_, t)| t.kind.eq_ignore_ascii_case(q.as_str())) {
                    if wanted(q, &t.target_id) {
                        units.push(Unit { inst: ii, query: q, target: Some(ti), solvers: (0..n_ts).collect() });
                    }
                }
            } else if wanted(q, "") {
                units.push(Unit { inst: ii, query: q, target: None, solvers: (0..n_ts).collect() });
            }
        }
    }
    units
}

/// run_id of a C run: t is part of the solver field.
fn c_run_id(ctx: &Ctx, ts: &[u64], job: &Job) -> String {
    let u = &ctx.units[job.unit];
    let inst = &ctx.instances[u.inst];
    let tid = u.target.map(|t| inst.targets[t].target_id.as_str());
    let solver = format!("{}@t{}", ctx.adapters[job.solver].solver_id(), ts[job.solver]);
    record::run_id(ctx.campaign, &inst.instance_id, u.query.as_str(), tid, &solver, job.rep)
}

/// Total repetitions wanted after the first run of a cell.
fn reps_wanted_c(first: &Brief) -> u32 {
    if first.verdict.repeatable() && first.wall_ms < 1000.0 { SHORT_REPS } else { 1 }
}

#[derive(Default)]
struct Stats {
    written: usize,
    skipped: usize,
    differing: usize,
}

/// The round loop: `ctx.adapters[i]` must be the `Timed` adapter of `ts[i]`; every unit lists all of them.
fn run_rounds_c(ctx: &Ctx, ts: &[u64], parallel: usize, log: &mut RunLog, done: &mut HashMap<String, Brief>) -> Result<Stats, String> {
    let mut st = Stats::default();
    for round in 0..SHORT_REPS {
        let mut jobs: Vec<Job> = Vec::new();
        for (ui, u) in ctx.units.iter().enumerate() {
            for &s in &u.solvers {
                let first_id = c_run_id(ctx, ts, &Job { unit: ui, solver: s, rep: 0 });
                let wanted = if round == 0 { 1 } else { done.get(&first_id).map_or(1, reps_wanted_c) };
                if round >= wanted {
                    continue;
                }
                let job = Job { unit: ui, solver: s, rep: round };
                if done.contains_key(&c_run_id(ctx, ts, &job)) {
                    st.skipped += 1;
                } else {
                    jobs.push(job);
                }
            }
        }
        if jobs.is_empty() {
            continue;
        }
        println!("round {round}: {} jobs", jobs.len());
        let total = jobs.len();
        let mut finished = 0usize;
        let mut failure: Option<String> = None;
        runner::run_pool(&jobs, parallel, |j, w| ctx.execute(j, w), |ji, mut rec: RunRecord| {
            finished += 1;
            let job = &jobs[ji];
            let t = ts[job.solver];
            rec.run_id = c_run_id(ctx, ts, job);
            rec.timeout_ms = Some(t);
            let overshoot = rec.analyze_ms.map(|a| a - t as f64);
            if let Some(o) = overshoot {
                rec.note = Some(match rec.note.take() {
                    Some(n) => format!("overshoot_ms={o:.3}; {n}"),
                    None => format!("overshoot_ms={o:.3}"),
                });
            }
            if job.rep > 0
                && let Some(first) = done.get(&c_run_id(ctx, ts, &Job { rep: 0, ..*job }))
            {
                rec.verdict_differs = first.verdict != rec.verdict || (rec.verdict == Verdict::Count && (first.states != rec.states || first.edges != rec.edges));
            }
            if rec.verdict_differs {
                st.differing += 1;
            }
            let inst = &ctx.instances[ctx.units[job.unit].inst];
            println!(
                "[r{round} {finished}/{total}] {} {} t={t} rep{} -> {} wall {:.1} ms{}{}",
                rec.query, inst.mcc_model_id.as_deref().unwrap_or(&inst.path), rec.rep, rec.verdict.as_str(), rec.wall_ms,
                overshoot.map(|o| format!(", overshoot {o:.1} ms")).unwrap_or_default(),
                if rec.verdict_differs { "  DIFFERS FROM REP 0" } else { "" },
            );
            st.written += 1;
            if let Err(e) = log.append(&rec) {
                failure.get_or_insert(e);
            }
            done.insert(rec.run_id.clone(), to_brief(&rec));
        });
        if let Some(e) = failure {
            return Err(format!("cannot append to runs.jsonl: {e}"));
        }
    }
    Ok(st)
}

/// Per t over every planned run with a record in runs.jsonl: runs, UNKNOWN, definite, other, and overshoot
/// median and maximum in ms.
fn print_summary(ctx: &Ctx, ts: &[u64], results_dir: &Path) {
    let mut planned: HashSet<String> = HashSet::new();
    for ui in 0..ctx.units.len() {
        for &s in &ctx.units[ui].solvers {
            for rep in 0..SHORT_REPS {
                planned.insert(c_run_id(ctx, ts, &Job { unit: ui, solver: s, rep }));
            }
        }
    }
    let text = std::fs::read_to_string(record::runs_path(results_dir)).unwrap_or_default();
    // t -> (unknown, definite, other, overshoots)
    let mut per_t: BTreeMap<u64, (usize, usize, usize, Vec<f64>)> = BTreeMap::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        if !v["run_id"].as_str().is_some_and(|id| planned.contains(id)) {
            continue;
        }
        let Some(t) = v["timeout_ms"].as_u64() else { continue };
        let e = per_t.entry(t).or_default();
        match v["verdict"].as_str().and_then(Verdict::from_name) {
            Some(Verdict::Unknown) => e.0 += 1,
            Some(Verdict::True | Verdict::False | Verdict::Count) => e.1 += 1,
            _ => e.2 += 1,
        }
        if let Some(a) = v["analyze_ms"].as_f64() {
            e.3.push(a - t as f64);
        }
    }
    println!("summary per t (overshoot = analyze_ms - t, parse excluded):");
    for (t, (unknown, definite, other, over)) in &per_t {
        let max = over.iter().copied().fold(f64::NAN, f64::max);
        println!("  t={t:<6} UNKNOWN={unknown} definite={definite} other={other}  overshoot ms: median {:.1}, max {max:.1} (n={})", median(over), over.len());
    }
}

fn instance_selectors(instances: &[Instance], selectors: &[String]) -> Result<HashSet<usize>, String> {
    let mut out = HashSet::new();
    for sel in selectors {
        let hits: Vec<usize> = (0..instances.len())
            .filter(|&i| instances[i].instance_id == *sel || instances[i].mcc_model_id.as_deref() == Some(sel.as_str()) || instances[i].path == *sel)
            .collect();
        if hits.is_empty() {
            return Err(format!("--instances {sel:?}: no instance with this id, MCC model id or path in the manifests"));
        }
        out.extend(hits);
    }
    Ok(out)
}

pub(super) fn run(cfg: &Config, config_path: &Path, args: RunArgs) -> Result<(), String> {
    if !args.solvers.is_empty() && args.solvers != [SOLVER] {
        return Err(format!("campaign C runs only {SOLVER} (got --solvers {})", args.solvers.join(",")));
    }
    let (rcfg, scfg) = load_sections(config_path)?;
    let ts = check_timeouts(&rcfg)?;
    let threshold_ms = 2.0 * *ts.iter().max().unwrap() as f64;
    let shard = args.shard.as_deref().map(parse_shard).transpose()?;
    let b_dir = args.results_dir.clone().unwrap_or_else(|| rcfg.results_dir.clone());
    let results_dir = match args.results_subdir.as_deref() {
        None => b_dir.clone(),
        Some(s) if !s.is_empty() && !s.contains(['/', '\\']) && s != ".." && s != "." => b_dir.join(s),
        Some(s) => return Err(format!("--results-subdir {s:?}: expected a single directory name")),
    };

    let manifests = if args.manifest.is_empty() {
        let jl = cfg.paths.corpus_dir.join("manifest.jsonl");
        vec![if jl.exists() { jl } else { cfg.selected_path() }]
    } else {
        args.manifest.clone()
    };
    let mut instances: Vec<Instance> = Vec::new();
    let mut seen = HashSet::new();
    for m in &manifests {
        for i in load_manifest(m)? {
            if seen.insert(i.instance_id.clone()) {
                instances.push(i);
            }
        }
    }
    let base = adapters::build(SOLVER, &scfg)?;
    let (filter, queries, selection) = if args.instances.is_empty() {
        let runs = record::runs_path(&b_dir);
        let text = std::fs::read_to_string(&runs).map_err(|e| format!("campaign C needs the results of campaign B, cannot read {}: {e}; run `petrivet-eval run --campaign B` first", runs.display()))?;
        let (slow, seen) = slow_cells(&text, threshold_ms);
        if seen == 0 {
            return Err(format!("no campaign B {SOLVER} runs with a usable verdict in {}; run `petrivet-eval run --campaign B` first", runs.display()));
        }
        let queries: Vec<Query> = if args.queries.is_empty() {
            ["Q0", "Q1", "Q2", "Q3", "Q4", "Q5", "Q6", "QR", "QC"].iter().filter_map(|q| q.parse().ok()).collect()
        } else {
            args.queries.iter().map(|q| q.parse()).collect::<Result<_, _>>()?
        };
        (Filter::Cells(slow), queries, format!("{seen} B cells of {SOLVER}, slow = median wall > {threshold_ms} ms"))
    } else {
        let queries: Vec<Query> = (if args.queries.is_empty() { vec!["Q1".to_owned(), "Q3".to_owned()] } else { args.queries.clone() }).iter().map(|q| q.parse()).collect::<Result<_, _>>()?;
        (Filter::Instances(instance_selectors(&instances, &args.instances)?), queries, format!("explicit instances {:?}, no threshold", args.instances))
    };

    let mut units = plan_units(&instances, base.as_ref(), &queries, &filter, shard, ts.len());
    if let Some(n) = args.limit {
        let mut keep: Vec<usize> = Vec::new();
        for u in &units {
            if !keep.contains(&u.inst) {
                keep.push(u.inst);
            }
        }
        keep.truncate(n);
        units.retain(|u| keep.contains(&u.inst));
    }
    let seed = rcfg.seed.unwrap_or(cfg.select.seed);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    units.shuffle(&mut rng);
    for u in &mut units {
        u.solvers.shuffle(&mut rng);
    }
    // One adapter per t; the job's solver index is the index into `ts`.
    let adapters: Vec<Box<dyn Adapter>> = ts.iter().map(|&t| Box::new(Timed { inner: adapters::build(SOLVER, &scfg).unwrap(), t }) as Box<dyn Adapter>).collect();

    let n_instances = units.iter().map(|u| u.inst).collect::<BTreeSet<_>>().len();
    let cells = units.len() * ts.len();
    let wall = Duration::from_secs(args.wall_cap_secs.unwrap_or(rcfg.wall_cap_secs));
    let mem_kib = rcfg.mem_limit_gib * 1024 * 1024;
    let mut parallel = rcfg.parallel_jobs.max(1);
    if !rcfg.cores.is_empty() && rcfg.cores.len() < parallel {
        eprintln!("warning: only {} cores listed in [run] cores, parallel jobs reduced from {parallel}", rcfg.cores.len());
        parallel = rcfg.cores.len();
    }

    if args.dry_run {
        println!("campaign C: {n_instances} instances, {} (instance, query, target) cells x {} timeouts {ts:?} = {cells} first-run jobs (repetitions add up to {} per cell when the first run is under 1 s)", units.len(), ts.len(), SHORT_REPS - 1);
        println!("selection: {selection}; queries {:?}, seed {seed}, results {}", queries.iter().map(|q| q.as_str()).collect::<Vec<_>>(), results_dir.display());
        let dummy = Ctx {
            campaign: "C", instances: &instances, units: &units, adapters: &adapters, results_dir: &results_dir, env_id: "dry",
            wall, mem_kib, out_cap: 0, limiter: Limiter::None, cores: &rcfg.cores,
        };
        let mut shown = 0;
        'outer: for (ui, u) in units.iter().enumerate() {
            for &s in &u.solvers {
                let inst = &instances[u.inst];
                let path = inst.abs_path();
                let tf = u.target.and_then(|t| target_text(inst, &inst.targets[t]).ok()).map(|t| target_path(&results_dir, &t));
                let spec = adapters[s].command(&Request { instance_path: &path, query: u.query, target_file: tf.as_deref(), internal_timeout_ms: None, converted: None });
                let id = c_run_id(&dummy, &ts, &Job { unit: ui, solver: s, rep: 0 });
                println!("  {} {} t={} {} {}  [{}]", SOLVER, u.query.as_str(), ts[s], inst.mcc_model_id.as_deref().unwrap_or(&inst.instance_id[..inst.instance_id.len().min(12)]), spec.display(), &id[..12]);
                shown += 1;
                if shown >= 10 {
                    break 'outer;
                }
            }
        }
        return Ok(());
    }
    if cells == 0 {
        return Err(format!("no jobs: no cell qualifies ({selection}; check --queries, --manifest and the B results)"));
    }

    let limiter = Limiter::resolve(&rcfg.mem_limiter)?;
    let env_id = env::capture(&EnvInput {
        results_dir: &results_dir,
        config_path,
        manifests: &manifests,
        tools: vec![(SOLVER.to_owned(), base.tool())],
        pinned_cores: &rcfg.cores,
        parallel_jobs: parallel,
        wall_cap_ms: wall.as_millis() as u64,
        mem_limit_kib: mem_kib,
        mem_limiter: limiter.name(),
    })?;
    println!("campaign C, {selection}");
    println!("env_id {env_id}; memory limiter: {}; parallel jobs: {parallel}; cores: {:?}; timeouts {ts:?}", limiter.name(), rcfg.cores);

    let mut done = record::read_existing(&results_dir);
    let mut log = RunLog::open(&results_dir)?;
    let ctx = Ctx {
        campaign: "C", instances: &instances, units: &units, adapters: &adapters, results_dir: &results_dir, env_id: &env_id,
        wall, mem_kib, out_cap: rcfg.out_cap_kib * 1024, limiter, cores: &rcfg.cores,
    };
    let st = run_rounds_c(&ctx, &ts, parallel, &mut log, &mut done)?;
    println!("\ncampaign C: {} runs written to {}, {} skipped (already recorded), {} repetitions differ from repetition 0", st.written, record::runs_path(&results_dir).display(), st.skipped, st.differing);
    print_summary(&ctx, &ts, &results_dir);
    Ok(())
}

/// The configured timeouts: sorted, deduplicated, non-empty and positive.
fn check_timeouts(rcfg: &RunCfg) -> Result<Vec<u64>, String> {
    let mut ts = rcfg.c_timeouts_ms.clone();
    ts.sort_unstable();
    ts.dedup();
    if ts.is_empty() || ts[0] == 0 {
        return Err("[run] c_timeouts_ms must be a non-empty list of positive milliseconds".into());
    }
    Ok(ts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::mock::Mock;

    fn setup(dir: &Path, shell: &str) -> (Vec<Instance>, Vec<Box<dyn Adapter>>) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("a.pnml"), "x").unwrap();
        let inst = Instance { instance_id: "inst1".into(), path: "a.pnml".into(), source: "gen".into(), base: dir.to_owned(), ..Instance::default() };
        let ts = [10u64, 100];
        let adapters = ts.iter().map(|&t| Box::new(Timed { inner: Box::new(Mock::new("mock", shell)), t }) as Box<dyn Adapter>).collect();
        (vec![inst], adapters)
    }

    fn ctx<'a>(i: &'a [Instance], u: &'a [Unit], a: &'a [Box<dyn Adapter>], dir: &'a Path) -> Ctx<'a> {
        Ctx {
            campaign: "C", instances: i, units: u, adapters: a, results_dir: dir, env_id: "e", wall: Duration::from_secs(5),
            mem_kib: 1 << 20, out_cap: 4096, limiter: Limiter::None, cores: &[],
        }
    }

    #[test]
    fn t_is_part_of_the_run_id_and_sweep_resumes() {
        let dir = std::env::temp_dir().join(format!("pe-c-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (insts, adapters) = setup(&dir, "echo UNKNOWN");
        let units = [Unit { inst: 0, query: Query::Q3, target: None, solvers: vec![0, 1] }];
        let ts = [10, 100];
        let c = ctx(&insts, &units, &adapters, &dir);
        assert_ne!(c_run_id(&c, &ts, &Job { unit: 0, solver: 0, rep: 0 }), c_run_id(&c, &ts, &Job { unit: 0, solver: 1, rep: 0 }));
        let mut log = RunLog::open(&dir).unwrap();
        let mut done = record::read_existing(&dir);
        // fast UNKNOWN: 3 repetitions per t
        let st = run_rounds_c(&c, &ts, 2, &mut log, &mut done).unwrap();
        assert_eq!(st.written, 6);
        let text = std::fs::read_to_string(record::runs_path(&dir)).unwrap();
        let recs: Vec<serde_json::Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert!(recs.iter().all(|r| r["campaign"] == "C" && r["solver_id"] == "mock" && r["verdict"] == "UNKNOWN"));
        assert_eq!(recs.iter().filter(|r| r["timeout_ms"] == 10).count(), 3);
        assert_eq!(recs.iter().filter(|r| r["timeout_ms"] == 100).count(), 3);
        assert_eq!(recs.iter().map(|r| r["run_id"].as_str().unwrap()).collect::<HashSet<_>>().len(), 6);
        // resume: nothing is redone
        let mut done = record::read_existing(&dir);
        let st = run_rounds_c(&c, &ts, 2, &mut log, &mut done).unwrap();
        assert_eq!((st.written, st.skipped), (0, 6));
    }

    #[test]
    fn slow_run_is_not_repeated() {
        let dir = std::env::temp_dir().join(format!("pe-c-slow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (insts, adapters) = setup(&dir, "sleep 1.1; echo TRUE");
        let units = [Unit { inst: 0, query: Query::Q1, target: None, solvers: vec![0] }];
        let c = ctx(&insts, &units, &adapters, &dir);
        let mut log = RunLog::open(&dir).unwrap();
        let st = run_rounds_c(&c, &[10, 100], 1, &mut log, &mut HashMap::new()).unwrap();
        assert_eq!(st.written, 1);
    }

    #[test]
    fn slow_cells_from_b() {
        let line = |inst: &str, solver: &str, camp: &str, verdict: &str, wall: f64| {
            format!("{{\"campaign\":\"{camp}\",\"instance_id\":\"{inst}\",\"query\":\"Q1\",\"target_id\":null,\"solver_id\":\"{solver}\",\"verdict\":\"{verdict}\",\"wall_ms\":{wall}}}\n")
        };
        let mut text = String::new();
        text += &line("slow", SOLVER, "B", "TRUE", 5000.0);
        text += &line("fast", SOLVER, "B", "TRUE", 5.0);
        text += &line("edge", SOLVER, "B", "TRUE", 2000.0); // not strictly more than the threshold
        text += &line("to", SOLVER, "B", "TIMEOUT", 60000.0);
        text += &line("crash", SOLVER, "B", "CRASH", 9000.0);
        text += &line("other", "lola", "B", "TRUE", 9000.0);
        text += &line("a", SOLVER, "A", "TRUE", 9000.0);
        text += "torn line {\n";
        let (slow, seen) = slow_cells(&text, 2000.0);
        let ids: Vec<&str> = slow.iter().map(|k| k.0.as_str()).collect();
        assert_eq!(ids, ["slow", "to"]);
        assert_eq!(seen, 4);
    }

    #[test]
    fn timed_adds_timeout_flag_and_keeps_solver_id() {
        let a = Timed { inner: adapters::build(SOLVER, &adapters::SolversCfg::default()).unwrap(), t: 100 };
        assert_eq!(a.solver_id(), SOLVER);
        let spec = a.command(&Request { instance_path: Path::new("x.pnml"), query: Query::Q3, target_file: None, internal_timeout_ms: None, converted: None });
        let line = spec.display();
        assert!(line.contains("--timeout-ms 100"), "{line}");
    }

    #[test]
    fn timeouts_are_validated_and_sorted() {
        let mut r = RunCfg::default();
        assert_eq!(check_timeouts(&r).unwrap(), [10, 100, 1000]);
        r.c_timeouts_ms = vec![100, 10, 100];
        assert_eq!(check_timeouts(&r).unwrap(), [10, 100]);
        r.c_timeouts_ms = vec![0, 5];
        assert!(check_timeouts(&r).is_err());
        r.c_timeouts_ms = vec![];
        assert!(check_timeouts(&r).is_err());
    }

    #[test]
    fn median_works() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[1.0, 2.0, 3.0, 10.0]), 2.5);
    }
}
