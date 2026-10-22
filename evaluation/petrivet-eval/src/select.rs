//! Spec 5.2: selection of the mcc-small and mcc-large subsets.

use crate::config::Config;
use crate::oracle::{self, Exam, ModelOracle, States, Verdict, family};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Parameters {
    pub seed: u64,
    pub small_max_states: u64,
    pub models_per_candidate: usize,
    pub small_target: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PoolCounts {
    pub total: usize,
    pub non_colored: usize,
    pub pool: usize,
    pub small_pool: usize,
    pub small_pool_families: usize,
    pub large_pool: usize,
    pub large_pool_families: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SmallCand {
    pub id: String,
    pub states: u64,
}

/// One family of mcc-small with all its pool models, nearest to the family median first.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SmallFamily {
    pub family: String,
    pub models: Vec<SmallCand>,
}

/// A candidate family of a ranked list, with fallback models (in preference order).
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub family: String,
    pub models: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Selection {
    pub parameters: Parameters,
    pub pool: PoolCounts,
    /// Families in drawing order (sorted by blake3(seed || family)).
    pub small_candidates: Vec<SmallFamily>,
    pub large_candidates: BTreeMap<Exam, Vec<Candidate>>,
}

fn small_ok(m: &ModelOracle, max: u64) -> bool {
    matches!(m.states, States::Known(n) if n <= max)
}

fn large_state(m: &ModelOracle, max: u64) -> bool {
    match m.states {
        States::Known(n) => n > max,
        _ => true,
    }
}

/// Seed for one examination: first 32 bytes of blake3(master_seed_le || exam tag).
fn exam_rng(seed: u64, exam: Exam) -> ChaCha8Rng {
    let mut h = blake3::Hasher::new();
    h.update(&seed.to_le_bytes());
    h.update(exam.tag().as_bytes());
    ChaCha8Rng::from_seed(*h.finalize().as_bytes())
}

fn pref_key(seed: u64, exam: Exam, id: &str) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(&seed.to_le_bytes());
    h.update(exam.tag().as_bytes());
    h.update(id.as_bytes());
    *h.finalize().as_bytes()
}

/// Indices `0..len` ordered by distance from the median index `len/2`: median first, then the nearest
/// neighbours alternately (lower neighbour before upper neighbour).
pub fn proximity_order(len: usize) -> Vec<usize> {
    if len == 0 {
        return vec![];
    }
    let m = len / 2;
    let mut out = vec![m];
    for d in 1..len {
        if m >= d {
            out.push(m - d);
        }
        if m + d < len {
            out.push(m + d);
        }
    }
    out
}

/// All models of a family sorted by (states, id), reordered by `proximity_order`.
pub fn family_order(mut ms: Vec<&ModelOracle>) -> Vec<&ModelOracle> {
    ms.sort_by(|a, b| (a.states.known(), &a.id).cmp(&(b.states.known(), &b.id)));
    proximity_order(ms.len()).into_iter().map(|i| ms[i]).collect()
}

fn family_key(seed: u64, family: &str) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(&seed.to_le_bytes());
    h.update(family.as_bytes());
    *h.finalize().as_bytes()
}

pub fn select(models: &[ModelOracle], p: &Parameters) -> Selection {
    let non_col: Vec<&ModelOracle> = models.iter().filter(|m| !m.is_colored_id()).collect();
    let pool: Vec<&ModelOracle> = non_col.iter().copied().filter(|m| m.any_definite()).collect();

    let small_pool: Vec<&ModelOracle> = pool.iter().copied().filter(|m| small_ok(m, p.small_max_states)).collect();
    let large_pool: Vec<&ModelOracle> = pool
        .iter()
        .copied()
        .filter(|m| large_state(m, p.small_max_states) && m.any_large_definite())
        .collect();

    let mut by_family: BTreeMap<&str, Vec<&ModelOracle>> = BTreeMap::new();
    for m in &small_pool {
        by_family.entry(family(&m.id)).or_default().push(m);
    }
    let mut small_candidates: Vec<SmallFamily> = by_family
        .iter()
        .map(|(fam, ms)| SmallFamily {
            family: (*fam).to_string(),
            models: family_order(ms.clone())
                .into_iter()
                .map(|m| SmallCand { id: m.id.clone(), states: m.states.known().unwrap() })
                .collect(),
        })
        .collect();
    small_candidates.sort_by_key(|f| family_key(p.seed, &f.family));
    let small_families: BTreeSet<&str> = small_candidates.iter().map(|s| s.family.as_str()).collect();

    let mut large_families = BTreeSet::new();
    let mut large_candidates = BTreeMap::new();
    for &exam in &Exam::LARGE {
        let mut fam_models: BTreeMap<&str, Vec<&ModelOracle>> = BTreeMap::new();
        for m in large_pool.iter().copied().filter(|m| m.definite(exam)) {
            let f = family(&m.id);
            large_families.insert(f);
            if !small_families.contains(f) {
                fam_models.entry(f).or_default().push(m);
            }
        }
        // BTreeMap iteration is sorted by family, so the shuffle input is canonical.
        let mut fams: Vec<(&str, Vec<&ModelOracle>)> = fam_models.into_iter().collect();
        fams.shuffle(&mut exam_rng(p.seed, exam));
        let list = fams
            .into_iter()
            .map(|(f, mut ms)| {
                ms.sort_by_key(|m| pref_key(p.seed, exam, &m.id));
                Candidate {
                    family: f.to_string(),
                    models: ms.iter().take(p.models_per_candidate).map(|m| m.id.clone()).collect(),
                }
            })
            .collect::<Vec<_>>();
        large_candidates.insert(exam, list);
    }

    let fam_count = |ms: &[&ModelOracle]| ms.iter().map(|m| family(&m.id)).collect::<BTreeSet<_>>().len();
    Selection {
        parameters: Parameters { seed: p.seed, small_max_states: p.small_max_states, models_per_candidate: p.models_per_candidate, small_target: p.small_target },
        pool: PoolCounts {
            total: models.len(),
            non_colored: non_col.len(),
            pool: pool.len(),
            small_pool: small_pool.len(),
            small_pool_families: fam_count(&small_pool),
            large_pool: large_pool.len(),
            large_pool_families: fam_count(&large_pool),
        },
        small_candidates,
        large_candidates,
    }
}

pub fn write_json<T: Serialize>(path: &std::path::Path, v: &T) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let mut s = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    s.push('\n');
    std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn run(cfg: &Config) -> Result<(), String> {
    let models = oracle::load_all(&cfg.paths.oracle_dir)?;
    let params = Parameters {
        seed: cfg.select.seed,
        small_max_states: cfg.select.small_max_states,
        models_per_candidate: cfg.select.models_per_candidate,
        small_target: cfg.select.small_target,
    };
    let sel = select(&models, &params);
    write_json(&cfg.selection_path(), &sel)?;
    let first: Vec<&ModelOracle> = sel
        .small_candidates
        .iter()
        .filter_map(|f| models.iter().find(|m| m.id == f.models[0].id))
        .collect();

    let p = &sel.pool;
    println!("MCC oracle: {}", cfg.paths.oracle_dir.display());
    println!("pool sizes");
    println!("  models total              {:>5}", p.total);
    println!("  non-colored (no -COL-)    {:>5}", p.non_colored);
    println!("  pool (>=1 definite)       {:>5}", p.pool);
    println!("  small pool (<= {:.0e} st.)  {:>5}  in {} families", params.small_max_states as f64, p.small_pool, p.small_pool_families);
    println!("  large pool                {:>5}  in {} families", p.large_pool, p.large_pool_families);
    println!("mcc-small: {} families, {} candidate models, target {} (drawn by `fetch`, two passes)", sel.small_candidates.len(), sel.small_candidates.iter().map(|f| f.models.len()).sum::<usize>(), params.small_target);
    println!("  verdict balance among the first-choice (median) models, before parse filtering");
    println!("  {:<4} {:>5} {:>5} {:>5}", "exam", "true", "false", "?");
    for e in [Exam::L, Exam::QL, Exam::RD, Exam::OS, Exam::SM] {
        let c = |v: Verdict| first.iter().filter(|m| m.verdict(e) == v).count();
        println!("  {:<4} {:>5} {:>5} {:>5}", e.tag(), c(Verdict::True), c(Verdict::False), c(Verdict::Unknown));
    }
    println!("mcc-large ranked candidate families (excluding small families)");
    for (e, l) in &sel.large_candidates {
        println!("  {:<4} {:>5}", e.tag(), l.len());
    }
    println!("wrote {}", cfg.selection_path().display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, st: States, vs: &[(Exam, Verdict)]) -> ModelOracle {
        ModelOracle { id: id.into(), states: st, verdicts: vs.iter().copied().collect() }
    }

    fn corpus() -> Vec<ModelOracle> {
        let mut v = vec![
            m("A-PT-1", States::Known(10), &[]),
            m("A-PT-2", States::Known(30), &[]),
            m("A-PT-3", States::Known(20), &[]),
            m("A-PT-4", States::Known(40), &[]),
            m("B-COL-1", States::Known(5), &[]),
            m("C-PT-1", States::Unknown, &[(Exam::L, Verdict::True)]),
            m("D-PT-1", States::Inf, &[(Exam::RD, Verdict::False)]),
            m("E-PT-1", States::Known(5_000_000), &[(Exam::L, Verdict::False), (Exam::QL, Verdict::True)]),
            m("F-PT-1", States::Unknown, &[(Exam::SM, Verdict::True)]),
            m("G-PT-1", States::Unknown, &[]),
        ];
        for i in 0..30 {
            v.push(m(&format!("Fam{i:02}-PT-1"), States::Unknown, &[(Exam::L, Verdict::True), (Exam::OS, Verdict::False)]));
        }
        v
    }

    fn params() -> Parameters {
        Parameters { seed: 7, small_max_states: 1_000_000, models_per_candidate: 3, small_target: 90 }
    }

    #[test]
    fn median_selection_and_proximity_order() {
        assert_eq!(proximity_order(4), vec![2, 1, 3, 0]);
        assert_eq!(proximity_order(5), vec![2, 1, 3, 0, 4]);
        assert_eq!(proximity_order(1), vec![0]);
        let c = corpus();
        let s = select(&c, &params());
        assert_eq!(s.small_candidates.len(), 1);
        let ids: Vec<_> = s.small_candidates[0].models.iter().map(|m| m.id.as_str()).collect();
        // sorted states: A-PT-1 (10), A-PT-3 (20), A-PT-2 (30), A-PT-4 (40); median index 2
        assert_eq!(ids, ["A-PT-2", "A-PT-3", "A-PT-4", "A-PT-1"]);
    }

    #[test]
    fn pool_counts() {
        let s = select(&corpus(), &params());
        assert_eq!(s.pool.total, 40);
        assert_eq!(s.pool.non_colored, 39);
        // G (all unknown) is not in the pool; F only has SM so is pool but not large
        assert_eq!(s.pool.pool, 4 + 1 + 1 + 1 + 1 + 30);
        assert_eq!(s.pool.large_pool, 1 + 1 + 1 + 30);
        assert!(s.large_candidates[&Exam::RD].iter().all(|c| c.family == "D"));
    }

    #[test]
    fn shuffle_is_deterministic_and_seed_dependent() {
        let c = corpus();
        let a = select(&c, &params());
        let b = select(&c, &params());
        assert_eq!(a, b);
        let mut p2 = params();
        p2.seed = 8;
        let d = select(&c, &p2);
        assert_ne!(a.large_candidates[&Exam::L], d.large_candidates[&Exam::L]);
        // L and OS use different streams
        let l: Vec<_> = a.large_candidates[&Exam::L].iter().map(|c| &c.family).collect();
        let o: Vec<_> = a.large_candidates[&Exam::OS].iter().map(|c| &c.family).collect();
        assert_ne!(l, o);
    }
}
