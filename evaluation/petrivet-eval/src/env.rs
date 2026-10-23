//! `results/env.json`: the environment a set of runs was made in (SPEC 8.1).
//!
//! The file is an object `{env_id: {...}}`. `env_id` is the blake3 hex (first 16 characters) of the captured
//! content without the timestamp, so repeated runs on an unchanged setup share one entry. Things that cannot be
//! read here (cpufreq governor, turbo state on macOS) are recorded as null.

use crate::adapters::ToolInfo;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct EnvInput<'a> {
    pub results_dir: &'a Path,
    pub config_path: &'a Path,
    pub manifests: &'a [PathBuf],
    pub tools: Vec<(String, Option<ToolInfo>)>,
    pub pinned_cores: &'a [usize],
    pub parallel_jobs: usize,
    pub wall_cap_ms: u64,
    pub mem_limit_kib: u64,
    pub mem_limiter: &'a str,
}

fn read_trim(p: &str) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn cmd_out(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    let text = if out.stdout.is_empty() { out.stderr } else { out.stdout };
    let first = String::from_utf8_lossy(&text).lines().next()?.trim().to_owned();
    (!first.is_empty()).then_some(first)
}

fn cpu_model() -> Option<String> {
    if let Ok(t) = std::fs::read_to_string("/proc/cpuinfo")
        && let Some(l) = t.lines().find(|l| l.starts_with("model name"))
    {
        return l.split_once(':').map(|(_, v)| v.trim().to_owned());
    }
    cmd_out("sysctl", &["-n", "machdep.cpu.brand_string"])
}

fn ram_kib() -> Option<u64> {
    if let Ok(t) = std::fs::read_to_string("/proc/meminfo") {
        let l = t.lines().find(|l| l.starts_with("MemTotal:"))?;
        return l.split_whitespace().nth(1)?.parse().ok();
    }
    cmd_out("sysctl", &["-n", "hw.memsize"])?.parse::<u64>().ok().map(|b| b / 1024)
}

/// Turbo state: intel_pstate `no_turbo` (1 means turbo off) or cpufreq `boost` (0 means off). `Some(true)` is on.
fn turbo() -> Option<bool> {
    if let Some(v) = read_trim("/sys/devices/system/cpu/intel_pstate/no_turbo") {
        return Some(v == "0");
    }
    read_trim("/sys/devices/system/cpu/cpufreq/boost").map(|v| v == "1")
}

/// Governor of every core, collapsed: one value if all agree, else a comma list.
fn governor() -> Option<String> {
    let mut set = std::collections::BTreeSet::new();
    for e in std::fs::read_dir("/sys/devices/system/cpu").ok()?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("cpu")
            && name.len() > 3
            && name[3..].chars().all(|c| c.is_ascii_digit())
            && let Some(g) = read_trim(&format!("{}/cpufreq/scaling_governor", e.path().display()))
        {
            set.insert(g);
        }
    }
    (!set.is_empty()).then(|| set.into_iter().collect::<Vec<_>>().join(","))
}

fn hash_file(p: &Path) -> Option<String> {
    std::fs::read(p).ok().map(|b| blake3::hash(&b).to_hex().to_string())
}

/// Finds `program` as a path or on PATH.
pub fn which(program: &str) -> Option<PathBuf> {
    let p = Path::new(program);
    if p.components().count() > 1 {
        return p.is_file().then(|| p.to_path_buf());
    }
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(program)).find(|c| c.is_file())
}

fn unix_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Captures the environment, merges it into `<results_dir>/env.json` and returns the env_id.
pub fn capture(inp: &EnvInput) -> Result<String, String> {
    let cfg_dir = inp.config_path.canonicalize().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
    let git = |args: &[&str]| {
        let mut a = vec!["-C", cfg_dir.to_str().unwrap_or(".")];
        a.extend_from_slice(args);
        cmd_out("git", &a)
    };
    let mut solvers = serde_json::Map::new();
    for (id, tool) in &inp.tools {
        let Some(tool) = tool else { continue };
        let version = tool.version_args.as_ref().and_then(|a| {
            let a: Vec<&str> = a.iter().map(String::as_str).collect();
            cmd_out(&tool.program, &a)
        });
        let bin = which(&tool.program);
        solvers.insert(
            id.clone(),
            json!({
                "program": tool.program,
                "resolved": bin.as_ref().map(|b| b.display().to_string()),
                "version": version,
                "binary_blake3": bin.as_deref().and_then(hash_file),
            }),
        );
    }
    let manifests: serde_json::Map<String, Value> =
        inp.manifests.iter().map(|m| (m.display().to_string(), json!(hash_file(m)))).collect();
    let content = json!({
        "cpu_model": cpu_model(),
        "cpu_cores": std::thread::available_parallelism().ok().map(|n| n.get()),
        "governor": governor(),
        "turbo": turbo(),
        "kernel": cmd_out("uname", &["-sr"]),
        "ram_kib": ram_kib(),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "solvers": solvers,
        "rustc": cmd_out("rustc", &["--version"]),
        "cargo": cmd_out("cargo", &["--version"]),
        "git_sha": git(&["rev-parse", "HEAD"]),
        "git_dirty": git(&["status", "--porcelain"]).is_some(),
        "eval_toml_blake3": hash_file(inp.config_path),
        "corpus_manifests_blake3": manifests,
        "pinned_cores": inp.pinned_cores,
        "parallel_jobs": inp.parallel_jobs,
        "wall_cap_ms": inp.wall_cap_ms,
        "mem_limit_kib": inp.mem_limit_kib,
        "mem_limiter": inp.mem_limiter,
    });
    let env_id = blake3::hash(content.to_string().as_bytes()).to_hex()[..16].to_string();
    let mut entry = content;
    entry["timestamp_unix"] = json!(unix_secs());

    std::fs::create_dir_all(inp.results_dir).map_err(|e| e.to_string())?;
    let path = inp.results_dir.join("env.json");
    let mut all: serde_json::Map<String, Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    all.entry(env_id.clone()).or_insert(entry);
    let text = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(&path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(env_id)
}
