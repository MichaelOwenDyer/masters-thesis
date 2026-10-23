mod adapters;
mod adjudicate;
mod calibrate;
mod config;
mod doctor;
mod env;
mod fetch;
mod histogram;
mod instance;
mod latency;
mod manifest_mcc;
mod oracle;
mod oracle_dl;
mod record;
mod run;
mod runner;
mod select;
mod truth;
mod truth_tools;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "petrivet-eval", about = "Evaluation pipeline for the petrivet thesis")]
struct Cli {
    /// Path of the config file (every other parameter lives in it)
    #[arg(long, default_value = "eval.toml", global = true)]
    config: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Select the mcc-small and mcc-large subsets from the MCC 2026 oracle (writes selection.json)
    Select {
        /// Do not write; exit non-zero if the fresh selection differs from selection.json
        #[arg(long)]
        check: bool,
    },
    /// Download the selected models from the 2026 archive (resumable; writes selected.json)
    Fetch {
        /// Rewrite selected.json even if fetched models differ from the recorded ones
        #[arg(long)]
        accept_changes: bool,
    },
    /// Parse every selected model with petrivet and report the class histogram (writes histogram.json)
    Histogram,
    /// Download the MCC verdict archive and extract it into paths.oracle_dir (idempotent)
    Oracle,
    /// Check prerequisites: config, oracle, selection, selected models, external tools
    Doctor,
    /// Join runs with truth, classify outcomes and disagreement patterns, write reproducers and the E1 statistics
    Adjudicate(adjudicate::AdjudicateArgs),
    /// Write the instance manifest of a source whose truth is known without tools (mcc: corpus/mcc/manifest.jsonl)
    Manifest {
        /// mcc or calibration
        #[arg(long)]
        source: String,
        /// Output file (default: <corpus_dir>/manifest.jsonl for mcc, <dir>/manifest.jsonl for calibration)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Calibration directory (source calibration only)
        #[arg(long, default_value = "calibration")]
        dir: PathBuf,
    },
    /// Write truth lines from the known answers of manifests (mcc oracle, construction, hand); gen is skipped
    Truth {
        /// Manifest files (jsonl), one or more
        #[arg(long, required = true, num_args = 1..)]
        manifest: Vec<PathBuf>,
        #[arg(long, default_value = "results/truth.jsonl")]
        out: PathBuf,
        /// With --tools lola,tina: also label the gen instances by LoLA/TINA agreement
        #[command(flatten)]
        tools: truth_tools::TruthArgs,
    },
    /// Run solvers on instances as subprocesses and append run records (campaign B)
    Run(run::RunArgs),
    /// Run the solvers on the hand-checked calibration nets and compare with the hand answers
    Calibrate(calibrate::CalibrateArgs),
    /// Latency campaign D (E6): in-process and subprocess call sequences on small gen nets, startup baselines
    Latency(latency::LatencyArgs),
}

fn main() {
    let cli = Cli::parse();
    let result = if let Cmd::Adjudicate(args) = &cli.cmd {
        adjudicate::run(args)
    } else if matches!(cli.cmd, Cmd::Doctor) {
        doctor::run(&cli.config)
    } else {
        config::Config::load(&cli.config).and_then(|cfg| match cli.cmd {
            Cmd::Select { check } => select::run(&cfg, check),
            Cmd::Fetch { accept_changes } => fetch::run(&cfg, accept_changes),
            Cmd::Histogram => histogram::run(&cfg),
            Cmd::Oracle => oracle_dl::run(&cfg),
            Cmd::Run(args) => run::run(&cfg, &cli.config, args),
            Cmd::Calibrate(args) => calibrate::run(&cfg, &cli.config, args),
            Cmd::Latency(args) => latency::run(&cfg, &cli.config, args),
            Cmd::Manifest { source, out, dir } => match source.as_str() {
                "mcc" => manifest_mcc::run_mcc(&cfg, out.as_deref()),
                "calibration" => manifest_mcc::run_calibration(&dir, out.as_deref()),
                other => Err(format!("--source {other}: expected mcc or calibration")),
            },
            Cmd::Truth { manifest, out, tools } => truth::run(&cfg, &cli.config, &manifest, &out, &tools),
            Cmd::Doctor | Cmd::Adjudicate(_) => unreachable!(),
        })
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
