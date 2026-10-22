mod config;
mod fetch;
mod histogram;
mod oracle;
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
    Select,
    /// Download the selected models from the 2026 archive (resumable; writes selected.json)
    Fetch,
    /// Parse every selected model with petrivet and report the class histogram (writes histogram.json)
    Histogram,
}

fn main() {
    let cli = Cli::parse();
    let result = config::Config::load(&cli.config).and_then(|cfg| match cli.cmd {
        Cmd::Select => select::run(&cfg),
        Cmd::Fetch => fetch::run(&cfg),
        Cmd::Histogram => histogram::run(&cfg),
    });
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
