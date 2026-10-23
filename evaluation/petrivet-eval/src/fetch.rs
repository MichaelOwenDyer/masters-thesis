//! Fresh download of the selected models from the 2026 archive, comparison with the 2025 copies.

use crate::config::Config;
use crate::oracle::{self, Exam, ModelOracle, Verdict, family};
use crate::select::{Selection, write_json};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectedModel {
    pub id: String,
    pub family: String,
    pub stratum: String,
    /// mcc-small draw pass (1 or 2); null for mcc-large
    pub pass: Option<u8>,
    pub states: Option<u64>,
    pub states_raw: String,
    pub verdicts: BTreeMap<Exam, Verdict>,
    pub pnml_sha256: String,
    pub pnml_bytes: u64,
    pub changed_since_2025: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Skipped {
    pub id: String,
    pub stratum: String,
    pub reason: String,
    pub detail: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StratumCounts {
    pub drawn: usize,
    pub accepted: usize,
    pub rejected: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub small_target: usize,
    pub small_target_reached: bool,
    pub small_pass1: usize,
    pub small_pass2: usize,
    pub families_contributing_1: usize,
    pub families_contributing_2: usize,
    pub small_families_in_pool: usize,
    pub max_per_family: usize,
    /// missing models per large stratum when the ranked list was exhausted before the quota
    pub large_shortfall: BTreeMap<String, usize>,
    pub counts: BTreeMap<String, StratumCounts>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Selected {
    pub summary: Summary,
    pub models: Vec<SelectedModel>,
    pub skipped: Vec<Skipped>,
}

/// Why a drawn candidate does not count: a short reason and a detail text.
#[derive(Debug, Clone, PartialEq)]
pub struct Rejection {
    pub reason: String,
    pub detail: String,
}

fn rej(reason: &str, detail: impl Into<String>) -> Rejection {
    Rejection { reason: reason.into(), detail: detail.into() }
}

pub struct SmallDraw {
    pub picked: Vec<(String, u8)>,
    pub rejected: Vec<(String, Rejection)>,
}

/// The two-pass drawing rule for mcc-small (see eval.toml). `try_model` returns Ok when the model counts as drawn.
pub fn draw_small(
    families: &[crate::select::SmallFamily],
    target: usize,
    try_model: &mut dyn FnMut(&str) -> Result<(), Rejection>,
) -> SmallDraw {
    let mut d = SmallDraw { picked: vec![], rejected: vec![] };
    let mut cursor = vec![0usize; families.len()];
    for pass in [1u8, 2] {
        if pass == 2 && d.picked.len() >= target {
            break;
        }
        for (fi, f) in families.iter().enumerate() {
            if pass == 2 && d.picked.len() >= target {
                break;
            }
            while cursor[fi] < f.models.len() {
                let id = &f.models[cursor[fi]].id;
                cursor[fi] += 1;
                match try_model(id) {
                    Ok(()) => {
                        d.picked.push((id.clone(), pass));
                        break;
                    }
                    Err(r) => d.rejected.push((id.clone(), r)),
                }
            }
        }
    }
    d
}

fn run_cmd(cmd: &mut Command) -> Result<(), String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn is_colored(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("iscolored")).is_ok_and(|s| s.trim().eq_ignore_ascii_case("true"))
}

fn copy_verdicts(cfg: &Config, id: &str, dir: &Path) {
    for e in Exam::ALL {
        let name = format!("{id}-{}.out", e.tag());
        let src = cfg.paths.oracle_dir.join(id).join(&name);
        if src.exists() {
            let _ = std::fs::copy(src, dir.join(&name));
        }
    }
}

/// Gets the model files (download if absent, honouring recorded skips); Ok(true) when freshly downloaded.
fn fetch_files(cfg: &Config, id: &str, cap: Option<u64>) -> Result<bool, Rejection> {
    let dir = cfg.models_dir().join(id);
    let pnml = dir.join("model.pnml");
    let skip = dir.join("skipped.txt");
    let too_large = |b: u64| rej("too large", format!("{b} bytes"));
    if pnml.exists() {
        let bytes = std::fs::metadata(&pnml).map(|m| m.len()).unwrap_or(0);
        if is_colored(&dir) {
            return Err(rej("colored", "iscolored marker is TRUE"));
        }
        if cap.is_some_and(|c| bytes > c) {
            return Err(too_large(bytes));
        }
        return Ok(false);
    }
    if let Ok(s) = std::fs::read_to_string(&skip) {
        let s = s.trim();
        if s == "colored" {
            return Err(rej("colored", "iscolored marker is TRUE"));
        }
        // if the cap was raised since, fall through and download again
        if let Some(n) = s.strip_prefix("too-large ").and_then(|n| n.parse().ok())
            && cap.is_some_and(|c| n > c)
        {
            return Err(too_large(n));
        }
    }
    let tmp = cfg.models_dir().join(format!("{id}.partial"));
    let _ = std::fs::remove_dir_all(&tmp);
    let fail = |e: String| {
        let _ = std::fs::remove_dir_all(&tmp);
        rej("download failed", e)
    };
    std::fs::create_dir_all(&tmp).map_err(|e| fail(e.to_string()))?;
    let tgz = tmp.join("download.tgz");
    let url = format!("{}/{id}.tgz", cfg.fetch.base_url);
    run_cmd(
        Command::new("curl")
            .args(["-fsSL", "--retry", "2", "--max-time", &cfg.fetch.curl_max_time_secs.to_string(), "-o"])
            .arg(&tgz)
            .arg(&url),
    )
    .map_err(|e| fail(format!("download {url}: {e}")))?;
    let ex = tmp.join("x");
    std::fs::create_dir_all(&ex).map_err(|e| fail(e.to_string()))?;
    run_cmd(Command::new("tar").arg("-xzf").arg(&tgz).arg("-C").arg(&ex).args(["--strip-components=1"]))
        .map_err(|e| fail(format!("tar {id}: {e}")))?;
    let bytes = std::fs::metadata(ex.join("model.pnml"))
        .map_err(|_| fail(format!("{id}: archive has no model.pnml")))?
        .len();
    let _ = std::fs::remove_file(&tgz);
    let reject = if is_colored(&ex) {
        Some(("colored".to_string(), rej("colored", "iscolored marker is TRUE")))
    } else if cap.is_some_and(|c| bytes > c) {
        Some((format!("too-large {bytes}"), too_large(bytes)))
    } else {
        None
    };
    let _ = std::fs::remove_dir_all(&dir);
    if let Some((note, r)) = reject {
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(skip, note);
        return Err(r);
    }
    std::fs::rename(&ex, &dir).map_err(|e| fail(e.to_string()))?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(true)
}

/// Idempotent and resumable: existing files are kept, recorded skips honoured, the parse check is repeated
/// (unparseable models stay on disk but never count).
fn ensure_model(cfg: &Config, id: &str, cap: Option<u64>, downloaded: &AtomicUsize) -> Result<(), Rejection> {
    if fetch_files(cfg, id, cap)? {
        downloaded.fetch_add(1, Ordering::SeqCst);
    }
    let dir = cfg.models_dir().join(id);
    copy_verdicts(cfg, id, &dir);
    let xml = std::fs::read_to_string(dir.join("model.pnml")).map_err(|e| rej("other parse error", format!("read: {e}")))?;
    crate::histogram::parse_with_petrivet(xml)
        .map(|_| ())
        .map_err(|e| rej(&crate::histogram::classify_reason(&e), e))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

struct Zip2025(Option<zip::ZipArchive<std::fs::File>>);

impl Zip2025 {
    fn open(path: Option<&Path>) -> Self {
        let z = path.and_then(|p| std::fs::File::open(p).ok()).and_then(|f| zip::ZipArchive::new(f).ok());
        Zip2025(z)
    }
    fn available(&self) -> bool {
        self.0.is_some()
    }
    fn pnml(&mut self, id: &str) -> Option<Vec<u8>> {
        let z = self.0.as_mut()?;
        let mut f = z.by_name(&format!("oracle/models/{id}/model.pnml")).ok()?;
        let mut v = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut v).ok()?;
        Some(v)
    }
}

fn describe(cfg: &Config, o: &ModelOracle, stratum: &str, pass: Option<u8>, zip: &mut Zip2025, prev: Option<&SelectedModel>) -> Result<SelectedModel, String> {
    let path: PathBuf = cfg.models_dir().join(&o.id).join("model.pnml");
    let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let sha = hex(&Sha256::digest(&data));
    let changed = if zip.available() {
        zip.pnml(&o.id).map(|old| old != data)
    } else {
        // comparison skipped: keep the recorded value only if the file is the one it was recorded for
        prev.filter(|p| p.pnml_sha256 == sha).and_then(|p| p.changed_since_2025)
    };
    Ok(SelectedModel {
        id: o.id.clone(),
        family: family(&o.id).to_string(),
        stratum: stratum.to_string(),
        pass,
        states: o.states.known(),
        states_raw: o.states.raw(),
        verdicts: o.verdicts.clone(),
        pnml_sha256: sha,
        pnml_bytes: data.len() as u64,
        changed_since_2025: changed,
    })
}

pub fn print_skip_table(sel: &Selected) {
    let reasons: BTreeSet<&String> = sel.summary.counts.values().flat_map(|c| c.rejected.keys()).collect();
    println!("drawn candidates per stratum (accepted = counted in the sample, rejected by reason)");
    print!("  {:<9} {:>6} {:>8}", "stratum", "drawn", "accepted");
    for r in &reasons {
        print!(" {:>17}", r);
    }
    println!();
    for (s, c) in &sel.summary.counts {
        print!("  {:<9} {:>6} {:>8}", s, c.drawn, c.accepted);
        for r in &reasons {
            print!(" {:>17}", c.rejected.get(*r).copied().unwrap_or(0));
        }
        println!();
    }
}

pub fn run(cfg: &Config, accept_changes: bool) -> Result<(), String> {
    let t0 = Instant::now();
    let previous: Option<Selected> = std::fs::read_to_string(cfg.selected_path()).ok().and_then(|t| serde_json::from_str(&t).ok());
    let sel: Selection = serde_json::from_str(
        &std::fs::read_to_string(cfg.selection_path()).map_err(|e| format!("{} (run `select` first): {e}", cfg.selection_path().display()))?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(cfg.models_dir()).map_err(|e| e.to_string())?;
    let downloaded = AtomicUsize::new(0);
    let mut skipped: Vec<Skipped> = Vec::new();
    let mut picked: Vec<(String, String, Option<u8>)> = Vec::new(); // (id, stratum, pass)

    // mcc-small
    let target = sel.parameters.small_target;
    let n_models = sel.small_candidates.iter().map(|f| f.models.len()).sum::<usize>();
    let mut tried = 0usize;
    let draw = draw_small(&sel.small_candidates, target, &mut |id| {
        tried += 1;
        eprintln!("[small try {tried}/{n_models}] {id}");
        ensure_model(cfg, id, None, &downloaded)
    });
    for (id, r) in &draw.rejected {
        skipped.push(Skipped { id: id.clone(), stratum: "small".into(), reason: r.reason.clone(), detail: r.detail.clone() });
    }
    let mut per_family: BTreeMap<String, usize> = BTreeMap::new();
    for (id, pass) in &draw.picked {
        picked.push((id.clone(), "small".into(), Some(*pass)));
        *per_family.entry(family(id).to_string()).or_default() += 1;
    }
    let pass1 = draw.picked.iter().filter(|p| p.1 == 1).count();
    let pass2 = draw.picked.len() - pass1;

    // mcc-large: walk ranked lists in order, one exam after the other
    let mut used_families: BTreeSet<String> = BTreeSet::new();
    let mut large_shortfall = BTreeMap::new();
    for &exam in &Exam::LARGE {
        let stratum = format!("large:{}", exam.tag());
        let mut have = 0usize;
        for cand in &sel.large_candidates[&exam] {
            if have >= cfg.fetch.quota_per_exam {
                break;
            }
            if used_families.contains(&cand.family) {
                continue;
            }
            for id in &cand.models {
                eprintln!("[{stratum} {have}/{}] {id}", cfg.fetch.quota_per_exam);
                match ensure_model(cfg, id, Some(cfg.fetch.max_pnml_bytes), &downloaded) {
                    Ok(()) => {
                        picked.push((id.clone(), stratum.clone(), None));
                        used_families.insert(cand.family.clone());
                        have += 1;
                        break;
                    }
                    Err(r) => skipped.push(Skipped { id: id.clone(), stratum: stratum.clone(), reason: r.reason, detail: r.detail }),
                }
            }
        }
        if have < cfg.fetch.quota_per_exam {
            large_shortfall.insert(stratum.clone(), cfg.fetch.quota_per_exam - have);
        }
    }

    // describe
    let mut zip = Zip2025::open(cfg.paths.oracle_2025_zip.as_deref());
    if !zip.available() {
        println!("2025 comparison skipped (no oracle_2025_zip configured or file missing); keeping recorded changed_since_2025 values for unchanged files");
    }
    let mut models = Vec::new();
    for (id, stratum, pass) in &picked {
        let o = oracle::load_model(&cfg.paths.oracle_dir, id);
        let prev = previous.as_ref().and_then(|p| p.models.iter().find(|m| m.id == *id));
        models.push(describe(cfg, &o, stratum, *pass, &mut zip, prev)?);
    }
    let order = |s: &str| if s == "small" { "0".to_string() } else { format!("1{s}") };
    models.sort_by(|a, b| (order(&a.stratum), &a.id).cmp(&(order(&b.stratum), &b.id)));
    skipped.sort_by(|a, b| (&a.stratum, &a.id).cmp(&(&b.stratum, &b.id)));

    let mut counts: BTreeMap<String, StratumCounts> = BTreeMap::new();
    for m in &models {
        let c = counts.entry(m.stratum.clone()).or_default();
        c.accepted += 1;
        c.drawn += 1;
    }
    for s in &skipped {
        let c = counts.entry(s.stratum.clone()).or_default();
        c.drawn += 1;
        *c.rejected.entry(s.reason.clone()).or_default() += 1;
    }
    let summary = Summary {
        small_target: target,
        small_target_reached: draw.picked.len() >= target,
        small_pass1: pass1,
        small_pass2: pass2,
        families_contributing_1: per_family.values().filter(|&&n| n == 1).count(),
        families_contributing_2: per_family.values().filter(|&&n| n == 2).count(),
        small_families_in_pool: sel.small_candidates.len(),
        max_per_family: per_family.values().copied().max().unwrap_or(0),
        large_shortfall,
        counts,
    };
    let out = Selected { summary, models, skipped };
    // verification against the previously recorded selected.json
    let mut mismatched: Vec<String> = Vec::new();
    let mut dropped: Vec<String> = Vec::new();
    if let Some(prev) = &previous {
        for p in &prev.models {
            match out.models.iter().find(|m| m.id == p.id) {
                Some(m) if m.pnml_sha256 != p.pnml_sha256 => mismatched.push(p.id.clone()),
                Some(_) => {}
                None => dropped.push(p.id.clone()),
            }
        }
    }
    let diverged = !mismatched.is_empty() || !dropped.is_empty();
    if !diverged || accept_changes {
        write_json(&cfg.selected_path(), &out)?;
    }

    let count = |p: &dyn Fn(&SelectedModel) -> bool| out.models.iter().filter(|m| p(m)).count();
    let su = &out.summary;
    println!("fetch finished in {:.0}s; downloaded now {}", t0.elapsed().as_secs_f64(), downloaded.load(Ordering::SeqCst));
    println!(
        "mcc-small: {} models (target {}, reached: {}); pass 1: {}, pass 2: {}; families with 1 model: {}, with 2: {}, of {} in the pool; max per family {}",
        count(&|m| m.stratum == "small"), su.small_target, su.small_target_reached, su.small_pass1, su.small_pass2,
        su.families_contributing_1, su.families_contributing_2, su.small_families_in_pool, su.max_per_family
    );
    if !su.small_target_reached {
        println!("  TARGET NOT REACHED: {} of {} (pool exhausted)", draw.picked.len(), target);
    }
    for e in Exam::LARGE {
        let st = format!("large:{}", e.tag());
        let short = su.large_shortfall.get(&st).copied().unwrap_or(0);
        println!("  {:<9} {}{}", st, count(&|m| m.stratum == st), if short > 0 { format!("  (ranked list exhausted, {short} missing)") } else { String::new() });
    }
    println!("  verdict balance among selected small models");
    println!("  {:<4} {:>5} {:>5} {:>5}", "exam", "true", "false", "?");
    for e in [Exam::L, Exam::QL, Exam::RD, Exam::OS, Exam::SM] {
        let c = |v: Verdict| count(&|m| m.stratum == "small" && m.verdicts.get(&e).copied().unwrap_or(Verdict::Unknown) == v);
        println!("  {:<4} {:>5} {:>5} {:>5}", e.tag(), c(Verdict::True), c(Verdict::False), c(Verdict::Unknown));
    }
    println!(
        "changed since 2025: {} changed, {} unchanged, {} no 2025 copy",
        count(&|m| m.changed_since_2025 == Some(true)),
        count(&|m| m.changed_since_2025 == Some(false)),
        count(&|m| m.changed_since_2025.is_none()),
    );
    print_skip_table(&out);
    if diverged && !accept_changes {
        println!("!!! VERIFICATION FAILED: the fetched models differ from the recorded {}", cfg.selected_path().display());
        for id in &mismatched {
            println!("!!!   pnml sha256 changed: {id}");
        }
        for id in &dropped {
            println!("!!!   recorded model no longer drawn: {id}");
        }
        return Err(format!(
            "{} model(s) changed upstream and {} recorded model(s) are no longer drawn; selected.json NOT rewritten (use --accept-changes)",
            mismatched.len(),
            dropped.len()
        ));
    }
    if diverged {
        println!("accepted changes ({} changed, {} dropped); wrote {}", mismatched.len(), dropped.len(), cfg.selected_path().display());
    } else {
        println!("verified against previous selected.json: no mismatch; wrote {}", cfg.selected_path().display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::{SmallCand, SmallFamily};

    fn fam(name: &str, n: usize) -> SmallFamily {
        SmallFamily {
            family: name.into(),
            models: (0..n).map(|i| SmallCand { id: format!("{name}-PT-{i}"), states: i as u64 }).collect(),
        }
    }

    #[test]
    fn median_fails_next_neighbour_is_taken() {
        // candidate order within a family is already nearest-to-median first; the first A model fails
        let fams = vec![fam("A", 3), fam("B", 1)];
        let mut p = |id: &str| if id == "A-PT-0" { Err(rej("not connected", "x")) } else { Ok(()) };
        let d = draw_small(&fams, 2, &mut p);
        assert_eq!(d.picked, vec![("A-PT-1".to_string(), 1), ("B-PT-0".to_string(), 1)]);
        assert_eq!(d.rejected.len(), 1);
    }

    #[test]
    fn second_pass_until_target() {
        let fams = vec![fam("A", 3), fam("B", 1), fam("C", 2)];
        let d = draw_small(&fams, 4, &mut |_: &str| Ok(()));
        let ids: Vec<_> = d.picked.iter().map(|(i, p)| format!("{i}/{p}")).collect();
        // pass 1 takes one per family; pass 2 adds A's second, then B is exhausted, then C's second
        assert_eq!(ids, ["A-PT-0/1", "B-PT-0/1", "C-PT-0/1", "A-PT-1/2"]);
        // pass 1 is not truncated by the target
        assert_eq!(draw_small(&fams, 2, &mut |_: &str| Ok(())).picked.len(), 3);
    }

    #[test]
    fn pool_exhausted_below_target() {
        let fams = vec![fam("A", 2), fam("B", 1)];
        let mut p = |id: &str| if id == "A-PT-0" { Err(rej("arc weight", "w")) } else { Ok(()) };
        let d = draw_small(&fams, 10, &mut p);
        assert_eq!(d.picked.len(), 2);
    }
}
