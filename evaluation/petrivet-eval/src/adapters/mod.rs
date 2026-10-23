//! Solver adapters: map a query to a command line and a command's output to a verdict.
//!
//! One small module per tool. `pv` (petrivet CLI) and `mock` (tests) exist; LoLA and TINA plug in by
//! implementing [`Adapter`] and adding a line to [`build`].

#[cfg(test)]
pub mod mock;
pub mod pv;

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
}

impl Parsed {
    pub fn verdict(verdict: Verdict) -> Self {
        Parsed { verdict, parse_ms: None, analyze_ms: None, states: None, edges: None, note: None }
    }
    pub fn error(note: impl Into<String>) -> Self {
        Parsed { note: Some(note.into()), ..Parsed::verdict(Verdict::Error) }
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
    /// Called when the process exited by itself (`exit_code` is its status); never after a kill by
    /// the runner (timeout, memout, signal), which the runner records itself.
    fn parse(&self, stdout: &str, exit_code: i32) -> Parsed;
    fn tool(&self) -> Option<ToolInfo> {
        None
    }
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

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct SolversCfg {
    #[serde(default)]
    pub pv: PvCfg,
}

/// Builds the adapter for a solver id from the CLI.
pub fn build(solver_id: &str, cfg: &SolversCfg) -> Result<Box<dyn Adapter>, String> {
    match solver_id {
        "pv-dispatch" | "pv-general" | "pv-poly" => Ok(Box::new(pv::Pv::new(solver_id, &cfg.pv))),
        other => Err(format!("unknown solver {other:?} (known: pv-dispatch, pv-general, pv-poly)")),
    }
}
