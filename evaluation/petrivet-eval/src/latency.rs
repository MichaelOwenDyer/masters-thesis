//! `petrivet-eval latency`: campaign D of SPEC 9 (latency, E6).
//!
//! Instances: `per_class` small `gen` nets per generator class (seeded, deterministic choice), queries Q1 and Q3.
//! For every (instance, query) and every variant one sequence of `sequence` back-to-back calls is timed; one
//! record per call goes to `results/latency.jsonl` (position in the sequence, nanoseconds, variant, query,
//! instance, class, env id).
//!
//! Variants:
//! * `pv-inproc-parsed`: the library called from this process, the net already parsed. A fresh copy of the
//!   parsed net is made before every call (outside the timing), so each call computes the answer; the library
//!   caches liveness and deadlock results per `PetriNet` object, and calls on one object would otherwise measure
//!   a cache lookup from the second call on. Position 0 is the cold call of the sequence; position 0 of the very
//!   first in-process call of the run additionally has `process_first` (lazy initialisation such as loading Z3).
//! * `pv-inproc-cached`: the same, but all calls of a sequence use one object. Position 0 computes, the rest read
//!   the cache: the "steady state of an embedded caller that keeps its net".
//! * `pv-inproc-parse`: file read, PNML parse and query in every call (what the CLI does minus process start).
//! * `pv-dispatch`, `lola`, `tina`: the usual adapters, one subprocess per call. The latency is the wall time of
//!   spawn to `wait4` return as measured by the runner (no memory limiter and no output beyond a small cap, so
//!   that no `systemd-run` wrapper or large capture distorts it). LoLA's PNML conversion happens once, outside
//!   the timing.
//!
//! Startup baseline (same sequence length): `baseline-noop` (spawn of `true`) and `baseline-pv-version`
//! (`petrivet --version`), kind `baseline`, no query and no instance.
//!
//! Caveat: wall times of the subprocess variants include the runner's pipe and thread set-up; the baseline runs
//! through the same runner, so the difference between a variant and its baseline is the tool's own cost.
//! A sequence is written in one piece when it is complete, so a rerun skips finished sequences (resumable); a
//! sequence whose call produced no answer (spawn failure, crash, rejected input, timeout) stops there and its
//! last record has `aborted` set.

use crate::adapters::convert::{self, ConvertError};
use crate::adapters::{self, Adapter, CommandSpec, Finished, Query, Request};
use crate::config::Config;
use crate::env::{self, EnvInput};
use crate::instance::{self, Instance};
use crate::record::Verdict;
use crate::run::RunCfg;
use crate::runner::{self, Limiter, Limits};
use petrivet::net::Net;
use petrivet::system::PetriNet;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// `[latency]` of eval.toml.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LatencyCfg {
    /// Calls per sequence.
    pub sequence: u32,
    /// Instances per generator class.
    pub per_class: usize,
    /// `size_bucket` of the manifest that counts as small (the testkit names buckets by place range).
    pub bucket: String,
    pub queries: Vec<String>,
    pub variants: Vec<String>,
    /// Safety cap per subprocess call, not a measurement.
    pub wall_cap_secs: u64,
    /// Calls of each startup baseline.
    pub baseline_reps: u32,
    /// Output file name inside the results directory.
    pub file: String,
    /// Seed for the instance choice and the order of the sequences; defaults to `[select] seed`.
    pub seed: Option<u64>,
}

impl Default for LatencyCfg {
    fn default() -> Self {
        LatencyCfg {
            sequence: 200,
            per_class: 20,
            bucket: "2-5".into(),
            queries: vec!["Q1".into(), "Q3".into()],
            variants: ["pv-inproc-parsed", "pv-inproc-cached", "pv-inproc-parse", "pv-dispatch", "lola", "tina"].map(String::from).to_vec(),
            wall_cap_secs: 30,
            baseline_reps: 200,
            file: "latency.jsonl".into(),
            seed: None,
        }
    }
}

#[derive(Deserialize, Default)]
struct Sections {
    #[serde(default)]
    run: RunCfg,
    #[serde(default)]
    solvers: adapters::SolversCfg,
    #[serde(default)]
    latency: LatencyCfg,
}

fn load_sections(config_path: &Path) -> Result<Sections, String> {
    let text = std::fs::read_to_string(config_path).map_err(|e| format!("{}: {e}", config_path.display()))?;
    let mut s: Sections = toml::from_str(&text).map_err(|e| format!("{}: {e}", config_path.display()))?;
    if s.run.results_dir.is_relative() {
        let base = config_path.canonicalize().map_err(|e| e.to_string())?.parent().unwrap().to_path_buf();
        s.run.results_dir = base.join(&s.run.results_dir);
    }
    Ok(s)
}

#[derive(clap::Args)]
pub struct LatencyArgs {
    /// `gen` manifest (`manifest.jsonl` written by the testkit's `gen`)
    manifest: PathBuf,
    /// Comma separated variants (default [latency] variants)
    #[arg(long, value_delimiter = ',')]
    variants: Vec<String>,
    /// Comma separated queries (default [latency] queries)
    #[arg(long, value_delimiter = ',')]
    queries: Vec<String>,
    /// Calls per sequence (default [latency] sequence)
    #[arg(long)]
    sequence: Option<u32>,
    /// Instances per class (default [latency] per_class)
    #[arg(long)]
    per_class: Option<usize>,
    /// Use only the first N selected instances (after the class-wise choice)
    #[arg(long)]
    limit: Option<usize>,
    /// Print the plan, run nothing
    #[arg(long)]
    dry_run: bool,
    /// Overrides [run] results_dir
    #[arg(long)]
    results_dir: Option<PathBuf>,
}

/// One line of the latency log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatencyRecord {
    /// Always `D`.
    pub campaign: String,
    /// `call` or `baseline`.
    pub kind: String,
    pub variant: String,
    pub query: Option<String>,
    pub instance_id: Option<String>,
    /// Generator class of the instance.
    pub class: Option<String>,
    pub places: Option<u64>,
    pub transitions: Option<u64>,
    /// Index in the sequence, from 0 (the cold call).
    pub position: u32,
    /// `cold` (position 0) or `steady`.
    pub phase: String,
    pub ns: u64,
    pub ms: f64,
    pub verdict: Verdict,
    /// First in-process call of the whole run (library initialisation, Z3 load).
    pub process_first: bool,
    /// Set on the last record of a sequence that stopped early.
    pub aborted: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub detail: Option<String>,
    pub cmd: Option<String>,
    pub env_id: String,
    pub started_unix_ms: u64,
}

/// What one timed call produced.
#[derive(Clone, Debug)]
struct Sample {
    ns: u64,
    verdict: Verdict,
    process_first: bool,
    exit_code: Option<i32>,
    signal: Option<i32>,
    detail: Option<String>,
}

impl Sample {
    fn answered(&self) -> bool {
        self.verdict.repeatable()
    }

    /// A sequence of one call that did not happen (conversion or parse failure).
    fn failed(verdict: Verdict, detail: String) -> Sample {
        Sample { ns: 0, verdict, process_first: false, exit_code: None, signal: None, detail: Some(detail) }
    }
}

/// Who a sequence belongs to; the fields of the record that do not depend on the call.
struct Label<'a> {
    kind: &'static str,
    variant: &'a str,
    query: Option<Query>,
    inst: Option<&'a Instance>,
    cmd: Option<String>,
    env_id: &'a str,
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn class_of(i: &Instance) -> String {
    i.generator_class.clone().or_else(|| i.detected_class_petrivet.clone()).unwrap_or_else(|| "unknown".into())
}

/// Turns the samples of one sequence into records. A sequence that stopped before `planned` calls (or ended on
/// a call without an answer) has `aborted` on its last record.
fn to_records(label: &Label, samples: &[Sample], planned: u32) -> Vec<LatencyRecord> {
    let started = now_unix_ms();
    let n = samples.len();
    samples
        .iter()
        .enumerate()
        .map(|(i, s)| LatencyRecord {
            campaign: "D".into(),
            kind: label.kind.into(),
            variant: label.variant.into(),
            query: label.query.map(|q| q.as_str().to_owned()),
            instance_id: label.inst.map(|i| i.instance_id.clone()),
            class: label.inst.map(class_of),
            places: label.inst.map(|i| i.places),
            transitions: label.inst.map(|i| i.transitions),
            position: i as u32,
            phase: if i == 0 { "cold" } else { "steady" }.into(),
            ns: s.ns,
            ms: s.ns as f64 / 1e6,
            verdict: s.verdict,
            process_first: s.process_first,
            aborted: i + 1 == n && (!s.answered() || n < planned as usize),
            exit_code: s.exit_code,
            signal: s.signal,
            detail: s.detail.clone(),
            cmd: label.cmd.clone(),
            env_id: label.env_id.to_owned(),
            started_unix_ms: started,
        })
        .collect()
}

/// `n` instances per generator class, small bucket only, in a seeded order that does not depend on the manifest
/// order (blake3 of seed and instance id); classes in name order. Returns indices into `instances`.
pub fn select(instances: &[Instance], per_class: usize, bucket: &str, seed: u64) -> Vec<usize> {
    let mut by_class: BTreeMap<String, Vec<(blake3::Hash, usize)>> = BTreeMap::new();
    for (i, inst) in instances.iter().enumerate() {
        if inst.source != "gen" || (!bucket.is_empty() && inst.size_bucket.as_deref() != Some(bucket)) {
            continue;
        }
        let mut h = blake3::Hasher::new();
        h.update(&seed.to_le_bytes()).update(inst.instance_id.as_bytes());
        by_class.entry(class_of(inst)).or_default().push((h.finalize(), i));
    }
    let mut out = Vec::new();
    for (_, mut v) in by_class {
        v.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        out.extend(v.into_iter().take(per_class).map(|(_, i)| i));
    }
    out
}

/// The nearest-rank percentile (`p` in 0..=100) of sorted values.
pub fn percentile(sorted: &[u64], p: f64) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    Some(sorted[rank.min(sorted.len()) - 1])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Inproc {
    Parsed,
    Cached,
    Parse,
}

#[derive(Clone, Copy, Debug)]
enum Variant {
    Inproc(Inproc),
    /// Index into the adapter list.
    Sub(usize),
}

/// The library's `dispatch` mode for Q1 and Q3, as the CLI calls it (no timeout).
fn ask(sys: &PetriNet<Net>, q: Query) -> Option<bool> {
    match q {
        Query::Q1 => sys.is_deadlock_free().call(),
        Query::Q3 => sys.liveness().call().map(|l| l.is_live()),
        _ => None,
    }
}

fn parse_net(path: &Path) -> Result<PetriNet<Net>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    PetriNet::from_pnml(&text).map_err(|e| e.to_string())
}

fn verdict_sample(ns: u64, r: std::thread::Result<Result<Option<bool>, String>>, process_first: bool) -> Sample {
    let (verdict, detail) = match r {
        Ok(Ok(Some(true))) => (Verdict::True, None),
        Ok(Ok(Some(false))) => (Verdict::False, None),
        Ok(Ok(None)) => (Verdict::Unknown, None),
        Ok(Err(e)) => (Verdict::InputRejected, Some(adapters::first_line(&e))),
        Err(_) => (Verdict::Crash, Some("panic".to_owned())),
    };
    Sample { ns, verdict, process_first, exit_code: None, signal: None, detail }
}

/// Times `n` in-process calls. `first` is true until the first in-process call of the run has been made.
/// Stops after the first call without an answer.
fn time_inproc(kind: Inproc, path: &Path, q: Query, n: u32, first: &mut bool) -> Vec<Sample> {
    let mut out = Vec::with_capacity(n as usize);
    // Parsed once, outside the timing; never queried itself, so that its cache stays empty and copies are cold.
    let pristine = match kind {
        Inproc::Parse => None,
        _ => match parse_net(path) {
            Ok(s) => Some(s),
            Err(e) => return vec![Sample::failed(Verdict::InputRejected, adapters::first_line(&e))],
        },
    };
    let mut shared = (kind == Inproc::Cached).then(|| pristine.clone().expect("parsed"));
    for _ in 0..n {
        let process_first = std::mem::replace(first, false);
        let (ns, r) = match kind {
            Inproc::Parsed => {
                let sys = pristine.clone().expect("parsed");
                let t = Instant::now();
                let r = catch_unwind(AssertUnwindSafe(|| Ok(black_box(ask(black_box(&sys), q)))));
                (t.elapsed().as_nanos() as u64, r)
            }
            Inproc::Cached => {
                let sys = shared.as_mut().expect("parsed");
                let t = Instant::now();
                let r = catch_unwind(AssertUnwindSafe(|| Ok(black_box(ask(black_box(&*sys), q)))));
                (t.elapsed().as_nanos() as u64, r)
            }
            Inproc::Parse => {
                let t = Instant::now();
                let r = catch_unwind(AssertUnwindSafe(|| parse_net(black_box(path)).map(|sys| black_box(ask(&sys, q)))));
                (t.elapsed().as_nanos() as u64, r)
            }
        };
        let s = verdict_sample(ns, r, process_first);
        let stop = !s.answered();
        out.push(s);
        if stop {
            break;
        }
    }
    out
}

/// Times `n` subprocess calls of one adapter command through the runner (wall time of spawn to wait). Stops
/// after the first call without an answer.
fn time_adapter(adapter: &dyn Adapter, q: Query, spec: &CommandSpec, limits: &Limits, n: u32) -> Vec<Sample> {
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let res = runner::run_command(spec, limits);
        let stdout = String::from_utf8_lossy(&res.stdout);
        let stderr = String::from_utf8_lossy(&res.stderr);
        let parsed = adapter.classify_for(
            q,
            &Finished {
                spawn_error: res.spawn_error.as_deref(),
                timed_out: res.timed_out,
                memout: res.memout,
                exit_code: res.exit_code,
                signal: res.signal,
                stdout: &stdout,
                stderr: &stderr,
            },
        );
        let s = Sample {
            ns: (res.wall_ms * 1e6).round() as u64,
            verdict: parsed.verdict,
            process_first: false,
            exit_code: res.exit_code,
            signal: res.signal,
            detail: parsed.detail,
        };
        let stop = !s.answered();
        out.push(s);
        if stop {
            break;
        }
    }
    out
}

/// Times `n` spawns of a command whose output is irrelevant (startup baseline); a clean exit counts as an answer.
fn time_command(spec: &CommandSpec, limits: &Limits, n: u32) -> Vec<Sample> {
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let res = runner::run_command(spec, limits);
        let ok = res.spawn_error.is_none() && !res.timed_out && res.exit_code == Some(0);
        let s = Sample {
            ns: (res.wall_ms * 1e6).round() as u64,
            verdict: if ok { Verdict::Unknown } else { Verdict::RunnerError },
            process_first: false,
            exit_code: res.exit_code,
            signal: res.signal,
            detail: res.spawn_error.clone().or_else(|| (!ok).then(|| format!("exit {:?}", res.exit_code))),
        };
        out.push(s);
        if !ok {
            break;
        }
    }
    out
}

/// Sequence key: `kind|variant|query|instance_id`.
fn key(kind: &str, variant: &str, query: Option<&str>, instance_id: Option<&str>) -> String {
    format!("{kind}|{variant}|{}|{}", query.unwrap_or(""), instance_id.unwrap_or(""))
}

fn read_log(path: &Path) -> Vec<LatencyRecord> {
    std::fs::read_to_string(path)
        .map(|t| t.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default()
}

/// Keys of sequences that are finished: `planned` records, or a record flagged `aborted`.
fn finished_keys(records: &[LatencyRecord], planned: u32) -> std::collections::HashSet<String> {
    let mut count: HashMap<String, u32> = HashMap::new();
    let mut done = std::collections::HashSet::new();
    for r in records {
        let k = key(&r.kind, &r.variant, r.query.as_deref(), r.instance_id.as_deref());
        let c = count.entry(k.clone()).or_default();
        *c += 1;
        if r.aborted || *c >= planned {
            done.insert(k);
        }
    }
    done
}

/// Appends records in one write, starting on a fresh line if a crash left a torn one.
struct Log {
    file: std::fs::File,
}

impl Log {
    fn open(path: &Path) -> Result<Log, String> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        let torn = std::fs::read(path).ok().is_some_and(|b| b.last().is_some_and(|c| *c != b'\n'));
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if torn {
            file.write_all(b"\n").map_err(|e| e.to_string())?;
        }
        Ok(Log { file })
    }

    fn append(&mut self, records: &[LatencyRecord]) -> Result<(), String> {
        let mut buf = String::new();
        for r in records {
            buf.push_str(&serde_json::to_string(r).map_err(|e| e.to_string())?);
            buf.push('\n');
        }
        self.file.write_all(buf.as_bytes()).and_then(|_| self.file.flush()).map_err(|e| e.to_string())
    }
}

/// Pins this process (and so its children and threads) to one core; Linux only, failures ignored.
fn pin_process(core: Option<usize>) {
    #[cfg(target_os = "linux")]
    if let Some(c) = core {
        // SAFETY: plain sched_setaffinity on ourselves with a zeroed, locally built set.
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_SET(c, &mut set);
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = core;
}

/// (count, p50, p95, p99) in ns of the answered calls of the records for which `pick` holds.
fn percentiles(records: &[LatencyRecord], pick: impl Fn(&LatencyRecord) -> bool) -> Option<(usize, u64, u64, u64)> {
    let mut v: Vec<u64> = records.iter().filter(|r| r.verdict.repeatable() && pick(r)).map(|r| r.ns).collect();
    v.sort_unstable();
    Some((v.len(), percentile(&v, 50.0)?, percentile(&v, 95.0)?, percentile(&v, 99.0)?))
}

fn fmt_ns(ns: u64) -> String {
    if ns >= 1_000_000 { format!("{:.2} ms", ns as f64 / 1e6) } else { format!("{:.1} us", ns as f64 / 1e3) }
}

fn print_summary(records: &[LatencyRecord]) {
    println!("\nlatency summary (answered calls; p50 / p95 / p99):");
    let mut groups: Vec<(String, Option<String>)> = records.iter().map(|r| (r.variant.clone(), r.query.clone())).collect();
    groups.sort();
    groups.dedup();
    for (variant, query) in groups {
        for (label, cold) in [("cold  ", true), ("steady", false)] {
            if let Some((n, a, b, c)) = percentiles(records, |r| r.variant == variant && r.query == query && (r.position == 0) == cold) {
                println!(
                    "  {variant:<20} {:<3} {label} n={n:<5} {} / {} / {}",
                    query.as_deref().unwrap_or("-"),
                    fmt_ns(a),
                    fmt_ns(b),
                    fmt_ns(c)
                );
            }
        }
    }
}

pub fn run(cfg: &Config, config_path: &Path, args: LatencyArgs) -> Result<(), String> {
    let Sections { run: rcfg, solvers: scfg, latency: lcfg } = load_sections(config_path)?;
    let n = args.sequence.unwrap_or(lcfg.sequence).max(1);
    let per_class = args.per_class.unwrap_or(lcfg.per_class);
    let seed = lcfg.seed.unwrap_or(cfg.select.seed);
    let results_dir = args.results_dir.clone().unwrap_or_else(|| rcfg.results_dir.clone());
    let out_path = results_dir.join(&lcfg.file);

    let queries: Vec<Query> = (if args.queries.is_empty() { &lcfg.queries } else { &args.queries })
        .iter()
        .map(|q| q.parse::<Query>())
        .collect::<Result<_, _>>()?;
    if let Some(q) = queries.iter().find(|q| !matches!(q, Query::Q1 | Query::Q3)) {
        return Err(format!("query {}: the latency campaign covers Q1 and Q3 only", q.as_str()));
    }
    let variant_names: &Vec<String> = if args.variants.is_empty() { &lcfg.variants } else { &args.variants };
    let mut adapters: Vec<Box<dyn Adapter>> = Vec::new();
    let mut variants: Vec<(String, Variant)> = Vec::new();
    for v in variant_names {
        let variant = match v.as_str() {
            "pv-inproc-parsed" => Variant::Inproc(Inproc::Parsed),
            "pv-inproc-cached" => Variant::Inproc(Inproc::Cached),
            "pv-inproc-parse" => Variant::Inproc(Inproc::Parse),
            "pv-dispatch" | "lola" | "tina" => {
                adapters.push(adapters::build(v, &scfg)?);
                Variant::Sub(adapters.len() - 1)
            }
            other => {
                return Err(format!(
                    "unknown variant {other:?} (known: pv-inproc-parsed, pv-inproc-cached, pv-inproc-parse, pv-dispatch, lola, tina)"
                ));
            }
        };
        variants.push((v.clone(), variant));
    }

    let instances = instance::load(&args.manifest)?;
    let mut chosen = select(&instances, per_class, &lcfg.bucket, seed);
    if let Some(l) = args.limit {
        chosen.truncate(l);
    }
    let mut per: BTreeMap<String, usize> = BTreeMap::new();
    for &i in &chosen {
        *per.entry(class_of(&instances[i])).or_default() += 1;
    }
    for (class, c) in &per {
        if *c < per_class {
            eprintln!("warning: class {class} has only {c} small instances (wanted {per_class})");
        }
    }

    // Units: (instance, query) in seeded shuffled order, variants shuffled inside each unit so that they run
    // next to each other and drift does not favour one.
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut units: Vec<(usize, Query)> = chosen.iter().flat_map(|&i| queries.iter().map(move |&q| (i, q))).collect();
    units.shuffle(&mut rng);
    let orders: Vec<Vec<usize>> = units
        .iter()
        .map(|_| {
            let mut o: Vec<usize> = (0..variants.len()).collect();
            o.shuffle(&mut rng);
            o
        })
        .collect();

    println!(
        "campaign D: {} instances in {} classes, {} units, {} variants, sequence {n}, seed {seed}, output {}",
        chosen.len(),
        per.len(),
        units.len(),
        variants.len(),
        out_path.display()
    );
    if args.dry_run {
        for (ui, (i, q)) in units.iter().enumerate().take(10) {
            let names: Vec<&str> = orders[ui].iter().map(|&v| variants[v].0.as_str()).collect();
            let inst = &instances[*i];
            println!("  {} {} {} {}", &inst.instance_id[..inst.instance_id.len().min(12)], class_of(inst), q.as_str(), names.join(","));
        }
        return Ok(());
    }
    if units.is_empty() {
        return Err("no instances selected (check the manifest: source gen, size_bucket, generator_class)".into());
    }

    let env_id = env::capture(&EnvInput {
        results_dir: &results_dir,
        config_path,
        manifests: std::slice::from_ref(&args.manifest),
        tools: adapters.iter().map(|a| (a.solver_id().to_owned(), a.tool())).collect(),
        pinned_cores: &rcfg.cores,
        parallel_jobs: 1,
        wall_cap_ms: lcfg.wall_cap_secs * 1000,
        mem_limit_kib: rcfg.mem_limit_gib * 1024 * 1024,
        mem_limiter: Limiter::None.name(),
    })?;
    pin_process(rcfg.cores.first().copied());
    let limits = Limits {
        wall: Duration::from_secs(lcfg.wall_cap_secs),
        mem_kib: rcfg.mem_limit_gib * 1024 * 1024,
        core: rcfg.cores.first().copied(),
        out_cap: 64 * 1024,
        limiter: Limiter::None,
        measure_rss: false,
    };
    let done = finished_keys(&read_log(&out_path), n);
    let mut log = Log::open(&out_path)?;

    // Startup baselines.
    let pv = adapters::env_or("PETRIVET_BIN", &scfg.pv.command);
    let reps = lcfg.baseline_reps.max(1);
    for (name, spec) in [
        ("baseline-noop", CommandSpec::new("true", Vec::<String>::new())),
        ("baseline-pv-version", CommandSpec::new(pv, ["--version"])),
    ] {
        if done.contains(&key("baseline", name, None, None)) {
            continue;
        }
        let samples = time_command(&spec, &limits, reps);
        let label = Label { kind: "baseline", variant: name, query: None, inst: None, cmd: Some(spec.display()), env_id: &env_id };
        let mut ns: Vec<u64> = samples.iter().map(|s| s.ns).collect();
        ns.sort_unstable();
        println!("{name}: {} calls, p50 {}", samples.len(), percentile(&ns, 50.0).map_or("-".into(), fmt_ns));
        log.append(&to_records(&label, &samples, reps))?;
    }

    // Warm-up: read every instance file once (page cache).
    for &i in &chosen {
        let _ = std::fs::read(instances[i].abs_path());
    }

    let mut first_inproc = true;
    let total = units.len() * variants.len();
    let mut count = 0usize;
    for (ui, &(ii, q)) in units.iter().enumerate() {
        let inst = &instances[ii];
        let path = inst.abs_path();
        for &vi in &orders[ui] {
            count += 1;
            let (name, variant) = &variants[vi];
            if done.contains(&key("call", name, Some(q.as_str()), Some(&inst.instance_id))) {
                continue;
            }
            let mut cmd = None;
            let samples = match *variant {
                Variant::Inproc(kind) => time_inproc(kind, &path, q, n, &mut first_inproc),
                Variant::Sub(ai) => {
                    let adapter = adapters[ai].as_ref();
                    if !adapter.supports(q) {
                        continue;
                    }
                    // Conversion (LoLA: ndrio), cached and outside the timing.
                    let mut converted: Option<PathBuf> = None;
                    let mut early: Option<Sample> = None;
                    if let Some(ndrio) = adapter.converter() {
                        match convert::ensure_lola(&ndrio, &path, &results_dir, &inst.instance_id) {
                            Ok(c) => converted = Some(c.path),
                            Err(ConvertError::Failed { msg, .. }) => {
                                early = Some(Sample::failed(Verdict::InputRejected, format!("conversion failed: {msg}")));
                            }
                            Err(ConvertError::Infra(e)) => early = Some(Sample::failed(Verdict::RunnerError, format!("conversion: {e}"))),
                        }
                    }
                    let spec = adapter.command(&Request { instance_path: &path, query: q, target_file: None, internal_timeout_ms: None, converted: converted.as_deref() });
                    cmd = Some(spec.display());
                    match early {
                        Some(s) => vec![s],
                        None => time_adapter(adapter, q, &spec, &limits, n),
                    }
                }
            };
            let label = Label { kind: "call", variant: name, query: Some(q), inst: Some(inst), cmd, env_id: &env_id };
            let mut ns: Vec<u64> = samples.iter().skip(1).map(|s| s.ns).collect();
            ns.sort_unstable();
            println!(
                "[{count}/{total}] {name} {} {} {}: {} calls, cold {}, steady p50 {}{}",
                q.as_str(),
                class_of(inst),
                &inst.instance_id[..inst.instance_id.len().min(10)],
                samples.len(),
                samples.first().map_or("-".into(), |s| fmt_ns(s.ns)),
                percentile(&ns, 50.0).map_or("-".into(), fmt_ns),
                samples.last().filter(|s| !s.answered()).map_or(String::new(), |s| format!("  STOPPED: {}", s.verdict.as_str())),
            );
            log.append(&to_records(&label, &samples, n))?;
        }
    }
    print_summary(&read_log(&out_path).into_iter().filter(|r| r.env_id == env_id).collect::<Vec<_>>());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::mock::Mock;

    const LIVE_NET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<pnml xmlns="http://www.pnml.org/version-2009/grammar/pnml">
  <net id="n" type="http://www.pnml.org/version-2009/grammar/ptnet">
    <page id="pg">
      <place id="p0"><initialMarking><text>1</text></initialMarking></place>
      <transition id="t0"/>
      <arc id="a0" source="p0" target="t0"/>
      <arc id="a1" source="t0" target="p0"/>
    </page>
  </net>
</pnml>
"#;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pe-lat-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn limits() -> Limits {
        Limits { wall: Duration::from_secs(10), mem_kib: 8 * 1024 * 1024, core: None, out_cap: 4096, limiter: Limiter::None, measure_rss: false }
    }

    fn inst(id: &str, class: &str, bucket: &str, source: &str) -> Instance {
        Instance {
            instance_id: id.into(),
            path: format!("{id}.pnml"),
            source: source.into(),
            generator_class: Some(class.into()),
            size_bucket: Some(bucket.into()),
            ..Instance::default()
        }
    }

    fn request() -> Request<'static> {
        Request { instance_path: Path::new("x"), query: Query::Q1, target_file: None, internal_timeout_ms: None, converted: None }
    }

    #[test]
    fn selection_is_per_class_small_gen_and_order_independent() {
        let mut v = Vec::new();
        for c in ["sm", "mg"] {
            for i in 0..30 {
                v.push(inst(&format!("{c}-s{i}"), c, "2-5", "gen"));
                v.push(inst(&format!("{c}-m{i}"), c, "6-8", "gen"));
            }
        }
        v.push(inst("fam-1", "sm", "2-5", "fam"));
        let sel = select(&v, 20, "2-5", 7);
        assert_eq!(sel.len(), 40);
        assert!(sel.iter().all(|&i| v[i].source == "gen" && v[i].size_bucket.as_deref() == Some("2-5")));
        assert_eq!(sel.iter().filter(|&&i| class_of(&v[i]) == "sm").count(), 20);
        let ids = |s: &[usize], v: &[Instance]| {
            let mut x: Vec<String> = s.iter().map(|&i| v[i].instance_id.clone()).collect();
            x.sort();
            x
        };
        let mut rev = v.clone();
        rev.reverse();
        assert_eq!(ids(&sel, &v), ids(&select(&rev, 20, "2-5", 7), &rev));
        assert_ne!(ids(&sel, &v), ids(&select(&v, 20, "2-5", 8), &v));
        // fewer than wanted: all of them
        assert_eq!(select(&v[..4], 20, "2-5", 7).len(), 2);
    }

    #[test]
    fn nearest_rank_percentiles() {
        let v: Vec<u64> = (1..=100).collect();
        assert_eq!(percentile(&v, 50.0), Some(50));
        assert_eq!(percentile(&v, 95.0), Some(95));
        assert_eq!(percentile(&v, 99.0), Some(99));
        assert_eq!(percentile(&v, 100.0), Some(100));
        assert_eq!(percentile(&[7], 99.0), Some(7));
        assert_eq!(percentile(&[], 50.0), None);
    }

    #[test]
    fn inproc_variants_give_sequences_with_answers() {
        let d = tmp("inproc");
        let p = d.join("n.pnml");
        std::fs::write(&p, LIVE_NET).unwrap();
        let mut first = true;
        for kind in [Inproc::Parsed, Inproc::Cached, Inproc::Parse] {
            for q in [Query::Q1, Query::Q3] {
                let s = time_inproc(kind, &p, q, 5, &mut first);
                assert_eq!(s.len(), 5, "{kind:?} {q:?}");
                assert!(s.iter().all(|x| x.verdict == Verdict::True && x.ns > 0), "{kind:?} {q:?}: {s:?}");
            }
        }
        // only the very first in-process call is flagged
        let mut first = true;
        let s = time_inproc(Inproc::Parsed, &p, Query::Q1, 3, &mut first);
        assert!(s[0].process_first && !s[1].process_first && !first);
        assert!(!time_inproc(Inproc::Parsed, &p, Query::Q1, 1, &mut first)[0].process_first);
    }

    #[test]
    fn inproc_unreadable_net_is_one_rejected_sample() {
        let mut first = true;
        for kind in [Inproc::Parsed, Inproc::Parse] {
            let s = time_inproc(kind, Path::new("/nonexistent/x.pnml"), Query::Q1, 5, &mut first);
            assert_eq!(s.len(), 1);
            assert_eq!(s[0].verdict, Verdict::InputRejected);
        }
    }

    #[test]
    fn subprocess_sequence_with_mock_adapter() {
        let a = Mock::new("mock", "echo TRUE");
        let spec = a.command(&request());
        let s = time_adapter(&a, Query::Q1, &spec, &limits(), 4);
        assert_eq!(s.len(), 4);
        assert!(s.iter().all(|x| x.verdict == Verdict::True && x.ns > 0 && x.exit_code == Some(0)));
        // a failing call stops the sequence
        let bad = Mock::new("mock", "exit 3");
        let s = time_adapter(&bad, Query::Q1, &bad.command(&request()), &limits(), 4);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].verdict, Verdict::Crash);
        // an unspawnable program
        let missing = CommandSpec::new("/nonexistent/prog", Vec::<String>::new());
        let s = time_adapter(&a, Query::Q1, &missing, &limits(), 4);
        assert_eq!((s.len(), s[0].verdict), (1, Verdict::RunnerError));
    }

    #[test]
    fn baseline_spawn_is_timed() {
        let s = time_command(&CommandSpec::new("true", Vec::<String>::new()), &limits(), 3);
        assert_eq!(s.len(), 3);
        assert!(s.iter().all(|x| x.answered() && x.ns > 0));
        let s = time_command(&CommandSpec::new("/nonexistent/prog", Vec::<String>::new()), &limits(), 3);
        assert_eq!((s.len(), s[0].verdict), (1, Verdict::RunnerError));
    }

    #[test]
    fn records_carry_labels_and_abort_flag_and_resume() {
        let i = inst("abc", "sm", "2-5", "gen");
        let label = Label { kind: "call", variant: "pv-dispatch", query: Some(Query::Q3), inst: Some(&i), cmd: None, env_id: "env1" };
        let ok = |ns| Sample { ns, verdict: Verdict::False, process_first: false, exit_code: Some(0), signal: None, detail: None };
        let recs = to_records(&label, &[ok(1500), ok(2500), ok(3500)], 3);
        assert_eq!(recs.iter().map(|r| r.position).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!((recs[0].phase.as_str(), recs[1].phase.as_str()), ("cold", "steady"));
        assert_eq!((recs[0].ns, recs[0].ms), (1500, 0.0015));
        assert!(recs.iter().all(|r| !r.aborted && r.class.as_deref() == Some("sm") && r.env_id == "env1" && r.campaign == "D"));
        let line = serde_json::to_string(&recs[0]).unwrap();
        assert!(line.contains("\"verdict\":\"FALSE\"") && line.contains("\"query\":\"Q3\""));
        // short sequence: aborted on the last record, so it counts as finished
        let short = to_records(&label, &[ok(1), ok(2)], 3);
        assert!(!short[0].aborted && short[1].aborted);
        assert!(finished_keys(&short, 3).contains(&key("call", "pv-dispatch", Some("Q3"), Some("abc"))));
        // incomplete without the flag is not finished
        let mut partial = recs[..2].to_vec();
        assert!(finished_keys(&partial, 3).is_empty());
        partial.push(recs[2].clone());
        assert_eq!(finished_keys(&partial, 3).len(), 1);
    }

    #[test]
    fn log_roundtrip_with_torn_line() {
        let d = tmp("log");
        let p = d.join("latency.jsonl");
        let i = inst("abc", "sm", "2-5", "gen");
        let label = Label { kind: "call", variant: "pv-inproc-parsed", query: Some(Query::Q1), inst: Some(&i), cmd: None, env_id: "e" };
        let s = Sample { ns: 10, verdict: Verdict::True, process_first: true, exit_code: None, signal: None, detail: None };
        let recs = to_records(&label, &[s.clone(), s], 2);
        Log::open(&p).unwrap().append(&recs).unwrap();
        std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"{\"campaign\":\"D\",\"ki").unwrap();
        Log::open(&p).unwrap().append(&recs).unwrap();
        let back = read_log(&p);
        assert_eq!(back.len(), 4);
        assert!(back[0].process_first);
    }

    #[test]
    fn summary_percentiles_ignore_unanswered_and_split_cold_steady() {
        let i = inst("abc", "sm", "2-5", "gen");
        let label = Label { kind: "call", variant: "v", query: Some(Query::Q1), inst: Some(&i), cmd: None, env_id: "e" };
        let mk = |ns, verdict| Sample { ns, verdict, process_first: false, exit_code: None, signal: None, detail: None };
        let mut s = vec![mk(1000, Verdict::True)];
        s.extend((1..=100).map(|k| mk(k, Verdict::True)));
        s.push(mk(999_999, Verdict::Crash));
        let recs = to_records(&label, &s, 102);
        assert_eq!(percentiles(&recs, |r| r.position == 0), Some((1, 1000, 1000, 1000)));
        // steady: positions 1..=100 answered (the crash at 101 is dropped)
        assert_eq!(percentiles(&recs, |r| r.position > 0), Some((100, 50, 95, 99)));
    }
}
