//! `petrivet-eval run`: campaign B of SPEC 9 (subprocess runs, performance and comparison).
//!
//! Plan: units = (instance, query, target) in seeded shuffled order, solvers shuffled inside each unit so that
//! they run next to each other. Execution is in rounds: round 0 holds repetition 0 of every cell
//! (unit x solver); the repetition rule (SPEC 8.2: 5 reps if the first run takes under 1 s, 3 if under 10 s,
//! else 1; only TRUE/FALSE/COUNT/UNKNOWN are repeated) is decided from the repetition 0 record, rounds 1..4
//! hold the further repetitions in the same order. A run is skipped if its run_id is already in runs.jsonl,
//! and a repetition-0 record found there is used for the rule, so an interrupted run resumes cleanly.

use crate::adapters::{self, Adapter, Query, Request, SolversCfg};
use crate::config::Config;
use crate::env::{self, EnvInput};
use crate::instance::{self, Instance};
use crate::record::{self, Brief, RunLog, RunRecord, Verdict};
use crate::runner::{self, Limiter, Limits};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// `[run]` of eval.toml.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RunCfg {
    pub wall_cap_secs: u64,
    pub mem_limit_gib: u64,
    pub parallel_jobs: usize,
    /// One core per running job is taken from this list (worker i gets `cores[i]`); empty: no pinning.
    pub cores: Vec<usize>,
    /// Relative to the directory of eval.toml.
    pub results_dir: PathBuf,
    /// Shuffle seed; defaults to `[select] seed`.
    pub seed: Option<u64>,
    /// `auto`, `cgroup`, `rlimit` or `none` (see runner.rs).
    pub mem_limiter: String,
    /// Bytes of stdout and stderr kept per run.
    pub out_cap_kib: usize,
}

impl Default for RunCfg {
    fn default() -> Self {
        RunCfg {
            wall_cap_secs: 60,
            mem_limit_gib: 8,
            parallel_jobs: 3,
            cores: Vec::new(),
            results_dir: PathBuf::from("results"),
            seed: None,
            mem_limiter: "auto".into(),
            out_cap_kib: 1024,
        }
    }
}

#[derive(Deserialize, Default)]
struct Sections {
    #[serde(default)]
    run: RunCfg,
    #[serde(default)]
    solvers: SolversCfg,
}

fn load_sections(config_path: &Path) -> Result<(RunCfg, SolversCfg), String> {
    let text = std::fs::read_to_string(config_path).map_err(|e| format!("{}: {e}", config_path.display()))?;
    let mut s: Sections = toml::from_str(&text).map_err(|e| format!("{}: {e}", config_path.display()))?;
    if s.run.results_dir.is_relative() {
        let base = config_path.canonicalize().map_err(|e| e.to_string())?.parent().unwrap().to_path_buf();
        s.run.results_dir = base.join(&s.run.results_dir);
    }
    Ok((s.run, s.solvers))
}

#[derive(clap::Args)]
pub struct RunArgs {
    /// Campaign; only B is implemented here
    #[arg(long, default_value = "B")]
    campaign: String,
    /// Manifest(s): `.jsonl` instance manifests or corpus/mcc/selected.json (default: selected.json of the corpus)
    #[arg(long, num_args = 1..)]
    manifest: Vec<PathBuf>,
    /// Comma separated solver ids
    #[arg(long, value_delimiter = ',', default_value = "pv-dispatch,pv-general,pv-poly")]
    solvers: Vec<String>,
    /// Comma separated queries
    #[arg(long, value_delimiter = ',', default_value = "Q1,Q3")]
    queries: Vec<String>,
    /// Use only the first N instances that have at least one job
    #[arg(long)]
    limit: Option<usize>,
    /// Print the job count and the first jobs, run nothing
    #[arg(long)]
    dry_run: bool,
    /// Overrides [run] results_dir
    #[arg(long)]
    results_dir: Option<PathBuf>,
}

/// One (instance, query, target) and the solvers that run on it, in execution order.
struct Unit {
    inst: usize,
    query: Query,
    target: Option<usize>,
    solvers: Vec<usize>,
}

#[derive(Clone, Copy)]
struct Job {
    unit: usize,
    solver: usize,
    rep: u32,
}

const BASELINE_PNML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<pnml xmlns="http://www.pnml.org/version-2009/grammar/pnml">
  <net id="baseline" type="http://www.pnml.org/version-2009/grammar/ptnet">
    <page id="pg">
      <place id="p0"><initialMarking><text>1</text></initialMarking></place>
      <transition id="t0"/>
      <arc id="a0" source="p0" target="t0"/>
      <arc id="a1" source="t0" target="p0"/>
    </page>
  </net>
</pnml>
"#;

const BASELINE_ID: &str = "BASELINE";

/// Total repetitions wanted after a first run (SPEC 8.2).
pub fn reps_wanted(first: &Brief) -> u32 {
    if !first.verdict.repeatable() {
        1
    } else if first.wall_ms < 1000.0 {
        5
    } else if first.wall_ms < 10_000.0 {
        3
    } else {
        1
    }
}

/// Path of the target JSON for a marking (content addressed, written by the worker).
fn target_path(results_dir: &Path, json: &str) -> PathBuf {
    results_dir.join("targets").join(format!("{}.json", &blake3::hash(json.as_bytes()).to_hex()[..16]))
}

fn target_text(inst: &Instance, target: &instance::Target) -> Result<String, String> {
    let pnml = if inst.source.starts_with("mcc") && inst.place_ids.is_none() {
        std::fs::read_to_string(inst.abs_path()).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    adapters::pv::target_json(inst, &target.marking, &pnml)
}

struct Ctx<'a> {
    campaign: &'a str,
    instances: &'a [Instance],
    units: &'a [Unit],
    adapters: &'a [Box<dyn Adapter>],
    results_dir: &'a Path,
    env_id: &'a str,
    wall: Duration,
    mem_kib: u64,
    out_cap: usize,
    limiter: Limiter,
    cores: &'a [usize],
}

impl Ctx<'_> {
    fn run_id(&self, job: &Job) -> String {
        let u = &self.units[job.unit];
        let inst = &self.instances[u.inst];
        let tid = u.target.map(|t| inst.targets[t].target_id.as_str());
        record::run_id(self.campaign, &inst.instance_id, u.query.as_str(), tid, self.adapters[job.solver].solver_id(), job.rep)
    }

    /// Runs one job and builds its record (the caller sets `verdict_differs` and appends).
    fn execute(&self, job: &Job, worker: usize) -> RunRecord {
        let u = &self.units[job.unit];
        let inst = &self.instances[u.inst];
        let adapter = &self.adapters[job.solver];
        let target = u.target.map(|t| &inst.targets[t]);
        let mut note = None;
        let mut target_file = None;
        if let Some(t) = target {
            match target_text(inst, t) {
                Ok(text) => {
                    let p = target_path(self.results_dir, &text);
                    if !p.exists() {
                        let _ = std::fs::create_dir_all(p.parent().unwrap());
                        if let Err(e) = std::fs::write(&p, &text) {
                            note = Some(format!("cannot write target file: {e}"));
                        }
                    }
                    target_file = Some(p);
                }
                Err(e) => note = Some(format!("target: {e}")),
            }
        }
        let path = inst.abs_path();
        let req = Request { instance_path: &path, query: u.query, target_file: target_file.as_deref(), internal_timeout_ms: None };
        let spec = adapter.command(&req);
        let core = (!self.cores.is_empty()).then(|| self.cores[worker % self.cores.len()]);
        let limits = Limits { wall: self.wall, mem_kib: self.mem_kib, core, out_cap: self.out_cap, limiter: self.limiter };
        let started = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        let res = if note.is_some() { runner::RunResult { spawn_error: note.clone(), ..Default::default() } } else { runner::run_command(&spec, &limits) };

        let mut parsed = None;
        let verdict = if let Some(e) = &res.spawn_error {
            note = Some(e.clone());
            Verdict::Error
        } else if res.timed_out {
            Verdict::Timeout
        } else if res.memout {
            Verdict::Memout
        } else if let Some(code) = res.exit_code {
            let p = adapter.parse(&String::from_utf8_lossy(&res.stdout), code);
            let v = p.verdict;
            note = p.note.clone();
            parsed = Some(p);
            v
        } else {
            note = Some(format!("killed by signal {}", res.signal.unwrap_or(0)));
            Verdict::Error
        };
        if res.truncated {
            note = Some(format!("{}output truncated at {} bytes", note.map(|n| n + "; ").unwrap_or_default(), self.out_cap));
        }
        let tid = target.map(|t| t.target_id.clone());
        RunRecord {
            run_id: self.run_id(job),
            campaign: self.campaign.to_owned(),
            instance_id: inst.instance_id.clone(),
            query: u.query.as_str().to_owned(),
            target_id: tid,
            solver_id: adapter.solver_id().to_owned(),
            rep: job.rep,
            verdict,
            raw_hash: record::store_raw(self.results_dir, &res.stdout),
            stderr_hash: record::store_raw(self.results_dir, &res.stderr),
            wall_ms: res.wall_ms,
            cpu_user_ms: res.cpu_user_ms,
            cpu_sys_ms: res.cpu_sys_ms,
            peak_rss_kib: res.peak_rss_kib,
            parse_ms: parsed.as_ref().and_then(|p| p.parse_ms),
            analyze_ms: parsed.as_ref().and_then(|p| p.analyze_ms),
            timeout_ms: None,
            limit_ms: self.wall.as_millis() as u64,
            mem_limit_kib: self.mem_kib,
            exit_code: res.exit_code,
            signal: res.signal,
            states: parsed.as_ref().and_then(|p| p.states),
            edges: parsed.as_ref().and_then(|p| p.edges),
            convert_ms: None,
            env_id: self.env_id.to_owned(),
            verdict_differs: false,
            pinned_core: core,
            mem_limiter: self.limiter.name().to_owned(),
            note,
            cmd: spec.display(),
            started_unix_ms: started,
        }
    }
}

pub fn run(cfg: &Config, config_path: &Path, args: RunArgs) -> Result<(), String> {
    if !args.campaign.eq_ignore_ascii_case("B") {
        return Err(format!("campaign {:?} is not implemented (only B)", args.campaign));
    }
    let (rcfg, scfg) = load_sections(config_path)?;
    let results_dir = args.results_dir.clone().unwrap_or_else(|| rcfg.results_dir.clone());

    let manifests = if args.manifest.is_empty() {
        let jl = cfg.paths.corpus_dir.join("manifest.jsonl");
        vec![if jl.exists() { jl } else { cfg.selected_path() }]
    } else {
        args.manifest.clone()
    };
    let mut instances: Vec<Instance> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for m in &manifests {
        for i in instance::load(m)? {
            if seen.insert(i.instance_id.clone()) {
                instances.push(i);
            }
        }
    }
    let queries: Vec<Query> = args.queries.iter().map(|q| q.parse()).collect::<Result<_, _>>()?;
    let adapters: Vec<Box<dyn Adapter>> = args.solvers.iter().map(|s| adapters::build(s, &scfg)).collect::<Result<_, _>>()?;

    // Plan: units in manifest order, then limit, then seeded shuffle.
    let mut units: Vec<Unit> = Vec::new();
    let mut missing = 0;
    for (ii, inst) in instances.iter().enumerate() {
        if !inst.abs_path().is_file() {
            missing += 1;
            continue;
        }
        for &q in &queries {
            if !q.applies_to_source(&inst.source) {
                continue;
            }
            let solvers: Vec<usize> = (0..adapters.len()).filter(|&s| adapters[s].supports(q)).collect();
            if solvers.is_empty() {
                continue;
            }
            if q.needs_target() {
                for (ti, t) in inst.targets.iter().enumerate().filter(|(_, t)| t.kind.eq_ignore_ascii_case(q.as_str())) {
                    let _ = t;
                    units.push(Unit { inst: ii, query: q, target: Some(ti), solvers: solvers.clone() });
                }
            } else {
                units.push(Unit { inst: ii, query: q, target: None, solvers });
            }
        }
    }
    if missing > 0 {
        eprintln!("warning: {missing} instance files are missing and were skipped");
    }
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

    let n_instances = units.iter().map(|u| u.inst).collect::<std::collections::BTreeSet<_>>().len();
    let cells: usize = units.iter().map(|u| u.solvers.len()).sum();
    let wall = Duration::from_secs(rcfg.wall_cap_secs);
    let mem_kib = rcfg.mem_limit_gib * 1024 * 1024;
    let mut parallel = rcfg.parallel_jobs.max(1);
    if !rcfg.cores.is_empty() && rcfg.cores.len() < parallel {
        eprintln!("warning: only {} cores listed in [run] cores, parallel jobs reduced from {parallel}", rcfg.cores.len());
        parallel = rcfg.cores.len();
    }

    if args.dry_run {
        println!("campaign B: {n_instances} instances, {} units, {cells} first-run jobs (repetitions add up to 4 per cell after the first run)", units.len());
        println!("solvers {:?}, queries {:?}, seed {seed}, results {}", args.solvers, args.queries, results_dir.display());
        let dummy = Ctx {
            campaign: "B", instances: &instances, units: &units, adapters: &adapters, results_dir: &results_dir, env_id: "dry",
            wall, mem_kib, out_cap: 0, limiter: Limiter::None, cores: &rcfg.cores,
        };
        let mut shown = 0;
        'outer: for (ui, u) in units.iter().enumerate() {
            for &s in &u.solvers {
                let inst = &instances[u.inst];
                let path = inst.abs_path();
                let tf = u.target.and_then(|t| target_text(inst, &inst.targets[t]).ok()).map(|t| target_path(&results_dir, &t));
                let spec = adapters[s].command(&Request { instance_path: &path, query: u.query, target_file: tf.as_deref(), internal_timeout_ms: None });
                let id = dummy.run_id(&Job { unit: ui, solver: s, rep: 0 });
                println!("  {} {} {} {}  [{}]", adapters[s].solver_id(), u.query.as_str(), inst.mcc_model_id.as_deref().unwrap_or(&inst.instance_id[..inst.instance_id.len().min(12)]), spec.display(), &id[..12]);
                shown += 1;
                if shown >= 10 {
                    break 'outer;
                }
            }
        }
        return Ok(());
    }
    if cells == 0 {
        return Err("no jobs: nothing applies (check --queries, --solvers and the manifests)".into());
    }

    let limiter = Limiter::resolve(&rcfg.mem_limiter)?;
    let env_id = env::capture(&EnvInput {
        results_dir: &results_dir,
        config_path,
        manifests: &manifests,
        tools: adapters.iter().map(|a| (a.solver_id().to_owned(), a.tool())).collect(),
        pinned_cores: &rcfg.cores,
        parallel_jobs: parallel,
        wall_cap_ms: wall.as_millis() as u64,
        mem_limit_kib: mem_kib,
        mem_limiter: limiter.name(),
    })?;
    println!("env_id {env_id}; memory limiter: {}; parallel jobs: {parallel}; cores: {:?}", limiter.name(), rcfg.cores);

    let mut done: HashMap<String, Brief> = record::read_existing(&results_dir);
    let mut log = RunLog::open(&results_dir)?;
    let ctx = Ctx {
        campaign: "B", instances: &instances, units: &units, adapters: &adapters, results_dir: &results_dir, env_id: &env_id,
        wall, mem_kib, out_cap: rcfg.out_cap_kib * 1024, limiter, cores: &rcfg.cores,
    };

    // Startup baseline: one no-op run per solver on a trivial net.
    let base_dir = results_dir.join("baseline");
    std::fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;
    let base_pnml = base_dir.join("trivial.pnml");
    std::fs::write(&base_pnml, BASELINE_PNML).map_err(|e| e.to_string())?;
    let base_inst = Instance { instance_id: BASELINE_ID.into(), path: "trivial.pnml".into(), source: "gen".into(), base: base_dir.clone(), places: 1, transitions: 1, arcs: 2, ..Instance::default() };
    let base_insts = [base_inst];
    for (si, a) in adapters.iter().enumerate() {
        let Some(q) = [Query::Q1, Query::Q3, Query::Q2, Query::Q4, Query::Q5].into_iter().find(|q| a.supports(*q)) else { continue };
        let bunits = [Unit { inst: 0, query: q, target: None, solvers: vec![si] }];
        let bctx = Ctx { instances: &base_insts, units: &bunits, ..ctx_clone(&ctx) };
        let job = Job { unit: 0, solver: si, rep: 0 };
        let id = bctx.run_id(&job);
        if done.contains_key(&id) {
            continue;
        }
        let rec = bctx.execute(&job, 0);
        println!("baseline {} {}: {} {:.1} ms, rss {} KiB", rec.solver_id, rec.query, rec.verdict.as_str(), rec.wall_ms, rec.peak_rss_kib.unwrap_or(0));
        log.append(&rec)?;
        done.insert(id, to_brief(&rec));
    }

    // Warm-up: read every instance file once.
    let mut warm = 0u64;
    for ii in units.iter().map(|u| u.inst).collect::<std::collections::BTreeSet<_>>() {
        warm += std::fs::read(instances[ii].abs_path()).map_or(0, |b| b.len() as u64);
    }
    println!("warm-up: read {n_instances} instance files ({warm} bytes)");

    let stats = run_rounds(&ctx, parallel, &mut log, &mut done)?;
    let (written, skipped, differing, summary) = (stats.written, stats.skipped, stats.differing, stats.summary);
    println!("\nsummary: {written} runs written to {}, {skipped} skipped (already recorded), {differing} repetitions differ from repetition 0", record::runs_path(&results_dir).display());
    for (solver, counts) in &summary {
        let line: Vec<String> = counts.iter().map(|(v, n)| format!("{}={n}", v.as_str())).collect();
        println!("  {solver:<12} {}", line.join("  "));
    }
    Ok(())
}

#[derive(Default)]
struct Stats {
    written: usize,
    skipped: usize,
    differing: usize,
    summary: BTreeMap<String, BTreeMap<Verdict, usize>>,
}

/// The round loop (see module docs): runs everything not yet in `done`, appends to the log.
fn run_rounds(ctx: &Ctx, parallel: usize, log: &mut RunLog, done: &mut HashMap<String, Brief>) -> Result<Stats, String> {
    let mut st = Stats::default();
    for round in 0..5u32 {
        let mut jobs: Vec<Job> = Vec::new();
        for (ui, u) in ctx.units.iter().enumerate() {
            for &s in &u.solvers {
                let first_id = ctx.run_id(&Job { unit: ui, solver: s, rep: 0 });
                let wanted = if round == 0 { 1 } else { done.get(&first_id).map_or(1, reps_wanted) };
                if round >= wanted {
                    continue;
                }
                let job = Job { unit: ui, solver: s, rep: round };
                if done.contains_key(&ctx.run_id(&job)) {
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
        runner::run_pool(&jobs, parallel, |j, w| ctx.execute(j, w), |ji, mut rec| {
            finished += 1;
            let job = &jobs[ji];
            if job.rep > 0
                && let Some(first) = done.get(&ctx.run_id(&Job { rep: 0, ..*job }))
            {
                rec.verdict_differs = first.verdict != rec.verdict || (rec.verdict == Verdict::Count && (first.states != rec.states || first.edges != rec.edges));
            }
            if rec.verdict_differs {
                st.differing += 1;
            }
            let inst = &ctx.instances[ctx.units[job.unit].inst];
            println!(
                "[r{round} {finished}/{total}] {} {} {} rep{} -> {} {:.1} ms{}{}",
                rec.solver_id, rec.query, inst.mcc_model_id.as_deref().unwrap_or(&inst.path), rec.rep, rec.verdict.as_str(), rec.wall_ms,
                if rec.verdict_differs { "  DIFFERS FROM REP 0" } else { "" },
                rec.note.as_deref().filter(|_| matches!(rec.verdict, Verdict::Error | Verdict::Panic)).map(|n| format!("  ({n})")).unwrap_or_default(),
            );
            *st.summary.entry(rec.solver_id.clone()).or_default().entry(rec.verdict).or_default() += 1;
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

fn to_brief(r: &RunRecord) -> Brief {
    Brief { run_id: r.run_id.clone(), verdict: r.verdict, wall_ms: r.wall_ms, states: r.states, edges: r.edges }
}

/// Field-wise copy of a context (it only holds references and Copy data).
fn ctx_clone<'a>(c: &Ctx<'a>) -> Ctx<'a> {
    Ctx {
        campaign: c.campaign, instances: c.instances, units: c.units, adapters: c.adapters, results_dir: c.results_dir, env_id: c.env_id,
        wall: c.wall, mem_kib: c.mem_kib, out_cap: c.out_cap, limiter: c.limiter, cores: c.cores,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::mock::Mock;

    fn setup(dir: &Path, shell: &str) -> (Vec<Instance>, Vec<Box<dyn Adapter>>) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("a.pnml"), "x").unwrap();
        let inst = Instance { instance_id: "inst1".into(), path: "a.pnml".into(), source: "gen".into(), base: dir.to_owned(), ..Instance::default() };
        (vec![inst], vec![Box::new(Mock::new("mock", shell))])
    }

    fn ctx<'a>(i: &'a [Instance], u: &'a [Unit], a: &'a [Box<dyn Adapter>], dir: &'a Path, wall_ms: u64) -> Ctx<'a> {
        Ctx {
            campaign: "B", instances: i, units: u, adapters: a, results_dir: dir, env_id: "e", wall: Duration::from_millis(wall_ms),
            mem_kib: 1 << 20, out_cap: 4096, limiter: Limiter::None, cores: &[],
        }
    }

    #[test]
    fn rules_and_resume() {
        let dir = std::env::temp_dir().join(format!("pe-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (insts, adapters) = setup(&dir, "echo TRUE");
        let units = [Unit { inst: 0, query: Query::Q1, target: None, solvers: vec![0] }];
        let c = ctx(&insts, &units, &adapters, &dir, 5000);
        let mut log = RunLog::open(&dir).unwrap();
        let mut done = record::read_existing(&dir);
        let st = run_rounds(&c, 2, &mut log, &mut done).unwrap();
        // fast command: 5 repetitions
        assert_eq!(st.written, 5);
        assert_eq!(st.summary["mock"][&Verdict::True], 5);
        // second invocation: everything is skipped, nothing is written
        let mut done = record::read_existing(&dir);
        assert_eq!(done.len(), 5);
        let st = run_rounds(&c, 2, &mut log, &mut done).unwrap();
        assert_eq!((st.written, st.skipped), (0, 5));
        // a partially recorded cell (rep 0 only) is completed, not redone
        let text = std::fs::read_to_string(record::runs_path(&dir)).unwrap();
        let first: Vec<&str> = text.lines().filter(|l| l.contains("\"rep\":0")).collect();
        std::fs::write(record::runs_path(&dir), format!("{}\n", first[0])).unwrap();
        let mut log = RunLog::open(&dir).unwrap();
        let mut done = record::read_existing(&dir);
        let st = run_rounds(&c, 1, &mut log, &mut done).unwrap();
        assert_eq!((st.written, st.skipped), (4, 1));
    }

    #[test]
    fn timeout_is_recorded_and_not_repeated() {
        let dir = std::env::temp_dir().join(format!("pe-run-to-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (insts, adapters) = setup(&dir, "sleep 30");
        let units = [Unit { inst: 0, query: Query::Q3, target: None, solvers: vec![0] }];
        let c = ctx(&insts, &units, &adapters, &dir, 300);
        let mut log = RunLog::open(&dir).unwrap();
        let mut done = HashMap::new();
        let st = run_rounds(&c, 1, &mut log, &mut done).unwrap();
        assert_eq!(st.written, 1);
        assert_eq!(st.summary["mock"][&Verdict::Timeout], 1);
        let rec: serde_json::Value = serde_json::from_str(std::fs::read_to_string(record::runs_path(&dir)).unwrap().lines().next().unwrap()).unwrap();
        assert_eq!(rec["verdict"], "TIMEOUT");
        assert_eq!(rec["signal"], 9);
        assert_eq!(rec["limit_ms"], 300);
    }

    #[test]
    fn differing_repetitions_are_flagged() {
        let dir = std::env::temp_dir().join(format!("pe-run-df-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // alternate the answer using a counter file
        let counter = dir.join("n");
        let (insts, adapters) = setup(&dir, &format!("echo x >> {0}; if [ $(wc -l < {0}) -eq 1 ]; then echo TRUE; else echo FALSE; fi", counter.display()));
        let units = [Unit { inst: 0, query: Query::Q1, target: None, solvers: vec![0] }];
        let c = ctx(&insts, &units, &adapters, &dir, 5000);
        let mut log = RunLog::open(&dir).unwrap();
        let st = run_rounds(&c, 1, &mut log, &mut HashMap::new()).unwrap();
        assert_eq!(st.differing, 4);
    }

    #[test]
    fn repetition_rule() {
        let b = |v, w| Brief { run_id: String::new(), verdict: v, wall_ms: w, states: None, edges: None };
        assert_eq!(reps_wanted(&b(Verdict::True, 999.0)), 5);
        assert_eq!(reps_wanted(&b(Verdict::False, 1000.0)), 3);
        assert_eq!(reps_wanted(&b(Verdict::Unknown, 10_000.0)), 1);
        assert_eq!(reps_wanted(&b(Verdict::Timeout, 1.0)), 1);
        assert_eq!(reps_wanted(&b(Verdict::Error, 1.0)), 1);
    }

    /// End to end with the real petrivet binary (`PETRIVET_BIN` or the local release build); run with --ignored.
    #[test]
    #[ignore]
    fn smoke_pv_dispatch() {
        let bin = std::env::var("PETRIVET_BIN").unwrap_or_else(|_| "/Users/michael.dyer/RustroverProjects/petrivet/target/release/petrivet".into());
        if !Path::new(&bin).exists() {
            eprintln!("petrivet binary not available, skipping");
            return;
        }
        let dir = std::env::temp_dir().join(format!("pe-smoke-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("loop.pnml"), BASELINE_PNML).unwrap();
        let dead = BASELINE_PNML.replace("<arc id=\"a1\" source=\"t0\" target=\"p0\"/>", "");
        std::fs::write(dir.join("dead.pnml"), dead).unwrap();
        let mk = |id: &str, p: &str| Instance { instance_id: id.into(), path: p.into(), source: "gen".into(), base: dir.clone(), ..Instance::default() };
        let insts = vec![mk("loop", "loop.pnml"), mk("dead", "dead.pnml")];
        let adapters: Vec<Box<dyn Adapter>> = vec![Box::new(adapters::pv::Pv::new("pv-dispatch", &adapters::PvCfg { command: bin }))];
        let units: Vec<Unit> = (0..2).map(|i| Unit { inst: i, query: Query::Q1, target: None, solvers: vec![0] }).collect();
        let c = ctx(&insts, &units, &adapters, &dir, 20_000);
        let mut log = RunLog::open(&dir).unwrap();
        let mut done = HashMap::new();
        run_rounds(&c, 2, &mut log, &mut done).unwrap();
        let text = std::fs::read_to_string(record::runs_path(&dir)).unwrap();
        let verdicts = |inst: &str| -> Vec<String> {
            text.lines().map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()).filter(|r| r["instance_id"] == inst).map(|r| r["verdict"].as_str().unwrap().to_owned()).collect()
        };
        assert!(verdicts("loop").iter().all(|v| v == "TRUE"));
        assert!(verdicts("dead").iter().all(|v| v == "FALSE"));
    }
}
