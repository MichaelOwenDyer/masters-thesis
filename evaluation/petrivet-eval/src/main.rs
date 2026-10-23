mod config;
mod doctor;
mod fetch;
mod histogram;
mod oracle;
mod oracle_dl;
mod select;

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
}

fn main() {
    let cli = Cli::parse();
    let result = if matches!(cli.cmd, Cmd::Doctor) {
        doctor::run(&cli.config)
    } else {
        config::Config::load(&cli.config).and_then(|cfg| match cli.cmd {
            Cmd::Select { check } => select::run(&cfg, check),
            Cmd::Fetch { accept_changes } => fetch::run(&cfg, accept_changes),
            Cmd::Histogram => histogram::run(&cfg),
            Cmd::Oracle => oracle_dl::run(&cfg),
            Cmd::Doctor => unreachable!(),
        })
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
