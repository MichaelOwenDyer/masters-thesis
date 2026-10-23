//! The run record (SPEC 8.1), the append-only `results/runs.jsonl` log and the content-addressed raw store.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    True,
    False,
    /// State space size; the numbers are in `states` / `edges`.
    Count,
    /// The tool gave up by itself.
    Unknown,
    /// Killed by the external wall-clock limit.
    Timeout,
    Memout,
    NotSupported,
    Error,
    Panic,
    ShardFailed,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::True => "TRUE",
            Verdict::False => "FALSE",
            Verdict::Count => "COUNT",
            Verdict::Unknown => "UNKNOWN",
            Verdict::Timeout => "TIMEOUT",
            Verdict::Memout => "MEMOUT",
            Verdict::NotSupported => "NOT_SUPPORTED",
            Verdict::Error => "ERROR",
            Verdict::Panic => "PANIC",
            Verdict::ShardFailed => "SHARD_FAILED",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))] // used by the mock adapter and, later, truth
    pub fn from_name(s: &str) -> Option<Verdict> {
        Some(match s.trim().to_ascii_uppercase().as_str() {
            "TRUE" => Verdict::True,
            "FALSE" => Verdict::False,
            "COUNT" => Verdict::Count,
            "UNKNOWN" => Verdict::Unknown,
            "TIMEOUT" => Verdict::Timeout,
            "MEMOUT" => Verdict::Memout,
            "NOT_SUPPORTED" => Verdict::NotSupported,
            "ERROR" => Verdict::Error,
            "PANIC" => Verdict::Panic,
            "SHARD_FAILED" => Verdict::ShardFailed,
            _ => return None,
        })
    }

    /// Verdicts that are repeated under the repetition rule (SPEC 8.2): the tool produced an answer or gave up
    /// by itself. Timeouts, memouts, errors and unsupported queries are not repeated.
    pub fn repeatable(self) -> bool {
        matches!(self, Verdict::True | Verdict::False | Verdict::Count | Verdict::Unknown)
    }
}

/// One line of `results/runs.jsonl`. Fields up to `env_id` are those of SPEC 8.1; the rest are additions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: String,
    pub campaign: String,
    pub instance_id: String,
    pub query: String,
    pub target_id: Option<String>,
    pub solver_id: String,
    pub rep: u32,
    pub verdict: Verdict,
    /// blake3 of stdout / stderr, stored under `results/raw/`; `None` when the stream was empty.
    pub raw_hash: Option<String>,
    pub stderr_hash: Option<String>,
    pub wall_ms: f64,
    pub cpu_user_ms: f64,
    pub cpu_sys_ms: f64,
    /// Normalised to KiB (ru_maxrss is KiB on Linux, bytes on macOS).
    pub peak_rss_kib: Option<u64>,
    pub parse_ms: Option<f64>,
    pub analyze_ms: Option<f64>,
    pub timeout_ms: Option<u64>,
    pub limit_ms: u64,
    pub mem_limit_kib: u64,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub states: Option<u64>,
    pub edges: Option<u64>,
    pub convert_ms: Option<f64>,
    pub env_id: String,
    // --- additions ---
    /// Repetition 1 or later whose verdict (or state count) differs from repetition 0: a finding.
    #[serde(default)]
    pub verdict_differs: bool,
    /// Core the job was pinned to, if any.
    #[serde(default)]
    pub pinned_core: Option<usize>,
    /// `cgroup`, `rlimit-as` or `none` (see runner.rs).
    #[serde(default)]
    pub mem_limiter: String,
    #[serde(default)]
    pub note: Option<String>,
    /// Display form of the command line (not re-parsed).
    #[serde(default)]
    pub cmd: String,
    #[serde(default)]
    pub started_unix_ms: u64,
}

/// run_id = blake3(campaign, instance_id, query, target_id, solver_id, rep); fields are separated by NUL, a
/// missing target_id is the empty string. Hex, 64 characters.
pub fn run_id(campaign: &str, instance_id: &str, query: &str, target_id: Option<&str>, solver_id: &str, rep: u32) -> String {
    let mut h = blake3::Hasher::new();
    for part in [campaign, instance_id, query, target_id.unwrap_or(""), solver_id, &rep.to_string()] {
        h.update(part.as_bytes());
        h.update(&[0]);
    }
    h.finalize().to_hex().to_string()
}

/// What the resume logic needs from an earlier record.
#[derive(Clone, Debug, Deserialize)]
pub struct Brief {
    pub run_id: String,
    pub verdict: Verdict,
    pub wall_ms: f64,
    #[serde(default)]
    pub states: Option<u64>,
    #[serde(default)]
    pub edges: Option<u64>,
}

/// Append-only writer. Each record is one `write` of a complete line.
pub struct RunLog {
    file: File,
}

impl RunLog {
    pub fn open(results_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(results_dir).map_err(|e| format!("{}: {e}", results_dir.display()))?;
        let path = runs_path(results_dir);
        let mut file = OpenOptions::new().create(true).append(true).read(true).open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        // A crash may have left a partial last line; start a fresh line so the next record stays parsable.
        let mut tail = Vec::new();
        file.read_to_end(&mut tail).map_err(|e| e.to_string())?;
        if tail.last().is_some_and(|b| *b != b'\n') {
            file.write_all(b"\n").map_err(|e| e.to_string())?;
        }
        Ok(RunLog { file })
    }

    pub fn append(&mut self, rec: &RunRecord) -> Result<(), String> {
        let mut line = serde_json::to_string(rec).map_err(|e| e.to_string())?;
        line.push('\n');
        self.file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        self.file.flush().map_err(|e| e.to_string())
    }
}

pub fn runs_path(results_dir: &Path) -> PathBuf {
    results_dir.join("runs.jsonl")
}

/// All records already in the log, by run_id. Unparsable lines (a torn last line) are skipped.
pub fn read_existing(results_dir: &Path) -> HashMap<String, Brief> {
    let mut map = HashMap::new();
    if let Ok(text) = std::fs::read_to_string(runs_path(results_dir)) {
        for line in text.lines() {
            if let Ok(b) = serde_json::from_str::<Brief>(line) {
                map.insert(b.run_id.clone(), b);
            }
        }
    }
    map
}

/// Stores `bytes` under `results/raw/<first two hex>/<hash>` (idempotent) and returns the blake3 hex; `None` for
/// empty input.
pub fn store_raw(results_dir: &Path, bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let hash = blake3::hash(bytes).to_hex().to_string();
    let dir = results_dir.join("raw").join(&hash[..2]);
    let path = dir.join(&hash);
    if !path.exists() && std::fs::create_dir_all(&dir).is_ok() {
        let tmp = dir.join(format!("{hash}.tmp{}", std::process::id()));
        if std::fs::write(&tmp, bytes).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
    Some(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn sample(id: &str) -> RunRecord {
        RunRecord {
            run_id: id.into(),
            campaign: "B".into(),
            instance_id: "i".into(),
            query: "Q1".into(),
            target_id: None,
            solver_id: "mock".into(),
            rep: 0,
            verdict: Verdict::NotSupported,
            raw_hash: None,
            stderr_hash: None,
            wall_ms: 1.5,
            cpu_user_ms: 0.0,
            cpu_sys_ms: 0.0,
            peak_rss_kib: Some(10),
            parse_ms: None,
            analyze_ms: None,
            timeout_ms: None,
            limit_ms: 1000,
            mem_limit_kib: 1024,
            exit_code: Some(0),
            signal: None,
            states: None,
            edges: None,
            convert_ms: None,
            env_id: "e".into(),
            verdict_differs: false,
            pinned_core: None,
            mem_limiter: "none".into(),
            note: None,
            cmd: String::new(),
            started_unix_ms: 0,
        }
    }

    #[test]
    fn run_id_is_stable_and_discriminating() {
        let a = run_id("B", "inst", "Q1", None, "pv-dispatch", 0);
        assert_eq!(a, run_id("B", "inst", "Q1", None, "pv-dispatch", 0));
        assert_eq!(a.len(), 64);
        assert_ne!(a, run_id("B", "inst", "Q1", None, "pv-dispatch", 1));
        assert_ne!(a, run_id("B", "inst", "Q1", Some("t"), "pv-dispatch", 0));
        assert_ne!(a, run_id("A", "inst", "Q1", None, "pv-dispatch", 0));
        // field boundaries matter
        assert_ne!(run_id("B", "ab", "c", None, "s", 0), run_id("B", "a", "bc", None, "s", 0));
        // frozen value: changing the scheme breaks resumability of old results
        assert_eq!(a, blake3::hash(b"B\0inst\0Q1\0\0pv-dispatch\x000\0").to_hex().to_string());
    }

    #[test]
    fn log_roundtrip_and_resume() {
        let dir = std::env::temp_dir().join(format!("pe-rec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut log = RunLog::open(&dir).unwrap();
        log.append(&sample("r1")).unwrap();
        drop(log);
        // torn line
        let mut f = OpenOptions::new().append(true).open(runs_path(&dir)).unwrap();
        f.write_all(b"{\"run_id\":\"r2\",\"verd").unwrap();
        drop(f);
        let mut log = RunLog::open(&dir).unwrap();
        log.append(&sample("r3")).unwrap();
        let ex = read_existing(&dir);
        assert!(ex.contains_key("r1") && ex.contains_key("r3") && !ex.contains_key("r2"));
        assert_eq!(ex["r1"].verdict, Verdict::NotSupported);
        let line = std::fs::read_to_string(runs_path(&dir)).unwrap().lines().next().unwrap().to_owned();
        assert!(line.contains("\"verdict\":\"NOT_SUPPORTED\""));
    }

    #[test]
    fn raw_store() {
        let dir = std::env::temp_dir().join(format!("pe-raw-{}", std::process::id()));
        assert_eq!(store_raw(&dir, b""), None);
        let h = store_raw(&dir, b"hello").unwrap();
        assert_eq!(std::fs::read(dir.join("raw").join(&h[..2]).join(&h)).unwrap(), b"hello");
        assert_eq!(store_raw(&dir, b"hello").unwrap(), h);
    }
}
