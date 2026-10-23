//! `petrivet-eval doctor`: prerequisite checks. Data check failures give a non-zero exit; missing tools do not.

use crate::config::Config;
use crate::fetch::Selected;
use crate::select::Selection;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Row {
    check: String,
    status: &'static str,
    detail: String,
}

fn row(rows: &mut Vec<Row>, check: &str, ok: bool, detail: impl Into<String>) -> bool {
    rows.push(Row { check: check.into(), status: if ok { "ok" } else { "FAIL" }, detail: detail.into() });
    ok
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(name)).find(|p| p.is_file())
}

/// Runs a tool with null stdin, at most `secs` seconds; returns trimmed stdout+stderr.
fn run_tool(path: &Path, args: &[&str], secs: u64) -> String {
    let child = Command::new(path).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let Ok(mut child) = child else { return "could not start".into() };
    let t = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if t.elapsed() < Duration::from_secs(secs) => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                break;
            }
        }
    }
    let Ok(out) = child.wait_with_output() else { return "no output".into() };
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let first: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).take(2).collect();
    let mut s = first.join(" | ");
    if s.len() > 100 {
        s.truncate(100);
        s.push_str("...");
    }
    s
}

pub fn run(config_path: &Path) -> Result<(), String> {
    let mut rows = Vec::new();
    let mut data_ok = true;
    let cfg = match Config::load(config_path) {
        Ok(c) => {
            row(&mut rows, "config loads", true, config_path.display().to_string());
            Some(c)
        }
        Err(e) => {
            data_ok = false;
            row(&mut rows, "config loads", false, e);
            None
        }
    };
    if let Some(cfg) = &cfg {
        let selection: Option<Selection> = std::fs::read_to_string(cfg.selection_path()).ok().and_then(|t| serde_json::from_str(&t).ok());
        let on_disk = std::fs::read_dir(&cfg.paths.oracle_dir)
            .map(|r| r.filter_map(Result::ok).filter(|e| e.path().is_dir()).count())
            .unwrap_or(0);
        let (ok, d) = match &selection {
            Some(s) => (on_disk == s.pool.total, format!("{on_disk} models in {}, selection.json expects {}", cfg.paths.oracle_dir.display(), s.pool.total)),
            None => (on_disk > 0, format!("{on_disk} models in {} (no selection.json to compare)", cfg.paths.oracle_dir.display())),
        };
        data_ok &= row(&mut rows, "oracle dir model count", ok, d);

        let (ok, d) = match crate::select::check(cfg) {
            Ok(true) => (true, "recomputed selection equals selection.json".to_string()),
            Ok(false) => (false, "recomputed selection DIFFERS from selection.json".to_string()),
            Err(e) => (false, e),
        };
        data_ok &= row(&mut rows, "select --check", ok, d);

        match std::fs::read_to_string(cfg.selected_path()).ok().and_then(|t| serde_json::from_str::<Selected>(&t).ok()) {
            None => data_ok &= row(&mut rows, "selected models verified", false, format!("{} missing or unreadable (run `fetch`)", cfg.selected_path().display())),
            Some(sel) => {
                let mut bad = Vec::new();
                for m in &sel.models {
                    let p = cfg.models_dir().join(&m.id).join("model.pnml");
                    match std::fs::read(&p) {
                        Ok(b) if format!("{:x}", Sha256::digest(&b)) == m.pnml_sha256 => {}
                        Ok(_) => bad.push(format!("{} (sha256 differs)", m.id)),
                        Err(_) => bad.push(format!("{} (missing)", m.id)),
                    }
                }
                let d = if bad.is_empty() {
                    format!("{} models, model.pnml present with recorded sha256", sel.models.len())
                } else {
                    format!("{} of {} bad: {}", bad.len(), sel.models.len(), bad.iter().take(5).cloned().collect::<Vec<_>>().join(", "))
                };
                data_ok &= row(&mut rows, "selected models verified", bad.is_empty(), d);
            }
        }
    }
    // external tools: reported, never fatal
    for (tool, args) in [("lola", &["--version"][..]), ("petrivet", &["--version"][..]), ("tina", &[][..]), ("ndrio", &[][..]), ("selt", &[][..])] {
        let (status, detail) = match find_on_path(tool) {
            None => ("missing", "not on PATH".to_string()),
            Some(p) => {
                let info = if matches!(tool, "lola" | "petrivet" | "ndrio") { run_tool(&p, args, 5) } else { String::new() };
                ("found", format!("{}{}", p.display(), if info.is_empty() { String::new() } else { format!("  [{info}]") }))
            }
        };
        rows.push(Row { check: format!("tool {tool}"), status, detail });
    }

    let w = rows.iter().map(|r| r.check.len()).max().unwrap_or(0);
    for r in &rows {
        println!("{:<w$}  {:<7}  {}", r.check, r.status, r.detail);
    }
    if data_ok { Ok(()) } else { Err("doctor: at least one data check failed".into()) }
}
