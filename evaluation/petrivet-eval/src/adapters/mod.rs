//! Solver adapters: map a query to a command line and a command's output to a verdict.
//!
//! One small module per tool: `pv` (petrivet CLI), `lola` (with `convert`), `tina`, and `mock` (tests).

pub mod convert;
pub mod lola;
#[cfg(test)]
pub mod mock;
pub mod pv;
pub mod tina;

use crate::record::Verdict;
use std::path::Path;

/// The queries of SPEC section 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Query {
    Q0,
    Q1,
    Q2,
    Q3,
    Q4,
    Q5,
    Q6,
    QR,
    QC,
}

impl Query {
    pub fn as_str(self) -> &'static str {
        match self {
            Query::Q0 => "Q0",
            Query::Q1 => "Q1",
            Query::Q2 => "Q2",
            Query::Q3 => "Q3",
            Query::Q4 => "Q4",
            Query::Q5 => "Q5",
            Query::Q6 => "Q6",
            Query::QR => "QR",
            Query::QC => "QC",
        }
    }

    pub fn needs_target(self) -> bool {
        matches!(self, Query::QR | Query::QC)
    }

    /// Applicability by source (SPEC section 3). `gen`/`fam` nets have no MCC-only queries.
    pub fn applies_to_source(self, source: &str) -> bool {
        if source == "calibration" {
            return true;
        }
        let mcc_small = source == "mcc-small";
        let mcc = source.starts_with("mcc");
        let synthetic = source == "gen" || source == "fam";
        match self {
            Query::Q0 | Query::Q6 => mcc_small,
            Query::Q1 | Query::Q3 | Query::Q4 => synthetic || mcc,
            Query::Q2 => synthetic,
            Query::Q5 => mcc,
            Query::QR | Query::QC => source == "gen" || mcc_small,
        }
    }
}

impl std::str::FromStr for Query {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Ok(match s.trim().to_ascii_uppercase().as_str() {
            "Q0" => Query::Q0,
            "Q1" => Query::Q1,
            "Q2" => Query::Q2,
            "Q3" => Query::Q3,
            "Q4" => Query::Q4,
            "Q5" => Query::Q5,
            "Q6" => Query::Q6,
            "QR" => Query::QR,
            "QC" => Query::QC,
            other => return Err(format!("unknown query {other:?}")),
        })
    }
}

/// A command to execute: program, arguments, extra environment.
#[derive(Clone, Debug, Default)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(program: impl Into<String>, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        CommandSpec { program: program.into(), args: args.into_iter().map(Into::into).collect(), env: Vec::new() }
    }

    /// Shell-ish rendering for the record (display only, not re-parsed).
    pub fn display(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(|a| if a.is_empty() || a.contains(char::is_whitespace) { format!("'{a}'") } else { a.to_owned() })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// What the adapter is asked to run.
pub struct Request<'a> {
    pub instance_path: &'a Path,
    pub query: Query,
    pub target_file: Option<&'a Path>,
    /// Internal (tool-side) timeout, campaign C; `None` in campaign B.
    pub internal_timeout_ms: Option<u64>,
    /// The converted net (LoLA format) if the adapter asked for a conversion ([`Adapter::converter`]); `None`
    /// before the conversion ran (dry runs).
    pub converted: Option<&'a Path>,
}

/// What the adapter extracts from a finished process.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub verdict: Verdict,
    pub parse_ms: Option<f64>,
    pub analyze_ms: Option<f64>,
    pub states: Option<u64>,
    pub edges: Option<u64>,
    pub note: Option<String>,
    /// See `RunRecord::detail`.
    pub detail: Option<String>,
}

impl Parsed {
    pub fn verdict(verdict: Verdict) -> Self {
        Parsed { verdict, parse_ms: None, analyze_ms: None, states: None, edges: None, note: None, detail: None }
    }
    pub fn with_detail(verdict: Verdict, detail: impl Into<String>) -> Self {
        Parsed { detail: Some(detail.into()), ..Parsed::verdict(verdict) }
    }
    pub fn crash(detail: impl Into<String>) -> Self {
        Parsed::with_detail(Verdict::Crash, detail)
    }
    /// The tool could not read the instance; `diagnostic` is cut to its first line, 200 characters.
    pub fn input_rejected(diagnostic: &str) -> Self {
        Parsed::with_detail(Verdict::InputRejected, first_line(diagnostic))
    }
    pub fn runner_error(reason: impl Into<String>) -> Self {
        Parsed::with_detail(Verdict::RunnerError, reason)
    }
}

/// First non-empty line, at most 200 characters.
pub fn first_line(text: &str) -> String {
    text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").chars().take(200).collect()
}

/// Name of a signal number (`SIGSEGV`), `SIG<n>` for the unlisted.
pub fn signal_name(sig: i32) -> String {
    let names = [
        (libc::SIGSEGV, "SIGSEGV"), (libc::SIGABRT, "SIGABRT"), (libc::SIGBUS, "SIGBUS"), (libc::SIGILL, "SIGILL"),
        (libc::SIGFPE, "SIGFPE"), (libc::SIGKILL, "SIGKILL"), (libc::SIGTERM, "SIGTERM"), (libc::SIGPIPE, "SIGPIPE"),
        (libc::SIGINT, "SIGINT"), (libc::SIGHUP, "SIGHUP"), (libc::SIGTRAP, "SIGTRAP"), (libc::SIGSYS, "SIGSYS"),
    ];
    names.iter().find(|(n, _)| *n == sig).map_or_else(|| format!("SIG{sig}"), |(_, s)| (*s).to_owned())
}

/// Everything the runner observed about a finished (or never started) process.
pub struct Finished<'a> {
    /// Could not spawn or wait; the reason.
    pub spawn_error: Option<&'a str>,
    /// Killed by our wall-clock limit.
    pub timed_out: bool,
    /// Memory limit hit (runner heuristic).
    pub memout: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: &'a str,
    pub stderr: &'a str,
}

/// Default classification of a finished process, in this order (first match wins):
/// 1. spawn failure: RUNNER_ERROR; 2. our wall kill: TIMEOUT; 3. memory limit: MEMOUT;
/// 4. signal we did not send: CRASH (`abort` for SIGABRT, else `signal:<NAME>`);
/// 5. exit code 101: CRASH(`panic`);
/// 6. any other nonzero exit: the adapter's [`Adapter::nonzero_exit`] (default: INPUT_REJECTED with the first
///    diagnostic line, or CRASH(`exit:<code>`) if there was no diagnostic at all);
/// 7. exit 0: the adapter's [`Adapter::parse`] (RUNNER_ERROR `unparseable output` if it cannot read the output).
pub fn default_classify<A: Adapter + ?Sized>(adapter: &A, f: &Finished) -> Parsed {
    classify_with(f, |out| adapter.parse(out, 0), |o, e, c| adapter.nonzero_exit(o, e, c))
}

/// The classification order of [`default_classify`] with the exit-0 and nonzero-exit readers passed in (adapters
/// whose reading depends on the query use this from [`Adapter::classify_for`]).
pub fn classify_with(f: &Finished, parse: impl FnOnce(&str) -> Parsed, nonzero: impl FnOnce(&str, &str, i32) -> Parsed) -> Parsed {
    if let Some(e) = f.spawn_error {
        return Parsed::runner_error(e);
    }
    if f.timed_out {
        return Parsed::verdict(Verdict::Timeout);
    }
    if f.memout {
        return Parsed::verdict(Verdict::Memout);
    }
    if let Some(sig) = f.signal {
        return Parsed::crash(if sig == libc::SIGABRT { "abort".to_owned() } else { format!("signal:{}", signal_name(sig)) });
    }
    match f.exit_code {
        Some(101) => Parsed::crash("panic"),
        Some(0) => parse(f.stdout),
        Some(code) => nonzero(f.stdout, f.stderr, code),
        None => Parsed::runner_error("no exit status"),
    }
}

/// How to ask a tool for its version and which binary to hash (for `env.json`).
pub struct ToolInfo {
    pub program: String,
    pub version_args: Option<Vec<String>>,
}

pub trait Adapter: Send + Sync {
    fn solver_id(&self) -> &str;
    fn supports(&self, query: Query) -> bool;
    fn command(&self, req: &Request) -> CommandSpec;
    /// Reads the output of a run that exited with status 0 (tool verdicts true, false, unknown, ... as before).
    /// Output that cannot be read is `Parsed::runner_error("unparseable output")`.
    fn parse(&self, stdout: &str, exit_code: i32) -> Parsed;
    /// Reads a run that exited with a nonzero status other than 101 (no signal, no limit hit). Default rule:
    /// first diagnostic line of stderr (else stdout) is an INPUT_REJECTED; no diagnostic at all is CRASH(`exit:<code>`).
    fn nonzero_exit(&self, stdout: &str, stderr: &str, exit_code: i32) -> Parsed {
        let diag = if stderr.trim().is_empty() { stdout } else { stderr };
        if diag.trim().is_empty() { Parsed::crash(format!("exit:{exit_code}")) } else { Parsed::input_rejected(diag) }
    }
    /// The whole classification; override to change the rules of a tool (LoLA, TINA) as a whole.
    fn classify(&self, f: &Finished) -> Parsed {
        default_classify(self, f)
    }
    /// Classification with the query known; the default ignores it. Adapters whose output reading depends on the
    /// query (LoLA, TINA) override this; the runner calls it instead of `classify`.
    fn classify_for(&self, _query: Query, f: &Finished) -> Parsed {
        self.classify(f)
    }
    fn tool(&self) -> Option<ToolInfo> {
        None
    }
    /// Program that converts the PNML before the timed run (LoLA: `ndrio`); `None` if the tool reads PNML.
    fn converter(&self) -> Option<String> {
        None
    }
    /// Every program the adapter needs (checked on PATH by `calibrate`).
    fn programs(&self) -> Vec<String> {
        self.tool().map(|t| vec![t.program]).unwrap_or_default()
    }
}

/// Single-quotes a word for `sh`.
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The value of the last line `PETRIVET_ANSWER <word>` of a job script's stdout.
pub fn script_answer(stdout: &str) -> Option<&str> {
    stdout.lines().rev().find_map(|l| l.trim().strip_prefix("PETRIVET_ANSWER ")).map(str::trim)
}

/// Verdict of a script answer word (`true`, `false`, anything else is UNKNOWN).
pub fn answer_parsed(word: &str) -> Parsed {
    match word {
        "true" => Parsed::verdict(Verdict::True),
        "false" => Parsed::verdict(Verdict::False),
        other => Parsed::with_detail(Verdict::Unknown, format!("a sub-run gave no result ({other})")),
    }
}

/// Exit status above 128 of a `sh -c` job whose child died of a signal: CRASH(`signal:<NAME>`).
pub fn shell_signal_exit(code: i32) -> Option<Parsed> {
    (129..=192).contains(&code).then(|| {
        let sig = code - 128;
        Parsed::crash(if sig == libc::SIGABRT { "abort".to_owned() } else { format!("signal:{}", signal_name(sig)) })
    })
}

/// `[solvers.pv]` of eval.toml.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct PvCfg {
    #[serde(default = "default_pv_command")]
    pub command: String,
}

fn default_pv_command() -> String {
    "petrivet".to_owned()
}

impl Default for PvCfg {
    fn default() -> Self {
        PvCfg { command: default_pv_command() }
    }
}

/// `[solvers.lola]` of eval.toml (`LOLA_BIN` overrides).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(default)]
pub struct LolaCfg {
    pub command: String,
}

impl Default for LolaCfg {
    fn default() -> Self {
        LolaCfg { command: "lola".to_owned() }
    }
}

/// `[solvers.ndrio]` of eval.toml (`NDRIO_BIN` overrides).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(default)]
pub struct NdrioCfg {
    pub command: String,
}

impl Default for NdrioCfg {
    fn default() -> Self {
        NdrioCfg { command: "ndrio".to_owned() }
    }
}

/// `[solvers.tina]` of eval.toml (`TINA_BIN`, `SIFT_BIN`, `SCAN_BIN` override).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(default)]
pub struct TinaCfg {
    pub tina: String,
    pub sift: String,
    pub scan: String,
}

impl Default for TinaCfg {
    fn default() -> Self {
        TinaCfg { tina: "tina".to_owned(), sift: "sift".to_owned(), scan: "scan".to_owned() }
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct SolversCfg {
    #[serde(default)]
    pub pv: PvCfg,
    #[serde(default)]
    pub lola: LolaCfg,
    #[serde(default)]
    pub tina: TinaCfg,
    #[serde(default)]
    pub ndrio: NdrioCfg,
}

/// Environment variable if set and non-empty, else the configured program.
pub fn env_or(var: &str, configured: &str) -> String {
    std::env::var(var).ok().filter(|s| !s.is_empty()).unwrap_or_else(|| configured.to_owned())
}

/// Builds the adapter for a solver id from the CLI.
pub fn build(solver_id: &str, cfg: &SolversCfg) -> Result<Box<dyn Adapter>, String> {
    match solver_id {
        "pv-dispatch" | "pv-general" | "pv-poly" => Ok(Box::new(pv::Pv::new(solver_id, &cfg.pv))),
        "lola" => Ok(Box::new(lola::Lola::new(&cfg.lola, &cfg.ndrio))),
        "tina" => Ok(Box::new(tina::Tina::new(&cfg.tina))),
        other => Err(format!("unknown solver {other:?} (known: pv-dispatch, pv-general, pv-poly, lola, tina)")),
    }
}
