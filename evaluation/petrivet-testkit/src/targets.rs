//! Candidate target markings for QR and QC (spec 5.5). Candidates carry an *expectation* only;
//! labels come from LoLA and TINA agreeing, never from here.

use crate::raw::RawNet;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Reachability,
    Coverability,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Reachability => "QR",
            Kind::Coverability => "QC",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub kind: Kind,
    pub marking: Vec<u32>,
    /// What the proposer hopes for (yes = reachable / coverable). Not a label.
    pub expect_yes: bool,
    /// For QR-yes: replaying it from M0 ends exactly in `marking`.
    /// For QC-yes: replaying it ends in a marking >= `marking`.
    pub firing_sequence: Option<Vec<usize>>,
}

const PER_POLARITY: usize = 2;
const WALKS: usize = 12;
const MAX_WALK_LEN: usize = 20;

/// Up to `2 + 2` candidates per kind: reachable ends of random walks, those minus tokens (QC yes),
/// random markings in the box M0 + 2 (QR no), and walk markings with one place pushed beyond
/// everything seen (QC no).
pub fn propose_targets(raw: &RawNet, seed: &[u8; 32]) -> Vec<Target> {
    let mut rng = ChaCha8Rng::from_seed(*blake3::Hasher::new().update(seed).update(b"targets").finalize().as_bytes());
    let mut seen: HashSet<Vec<u32>> = HashSet::from([raw.marking.clone()]);
    let mut max_seen = raw.marking.clone();
    let mut walks: Vec<(Vec<usize>, Vec<u32>)> = Vec::new();
    for _ in 0..WALKS {
        let (mut m, mut seq) = (raw.marking.clone(), Vec::new());
        for _ in 0..rng.random_range(1..=MAX_WALK_LEN) {
            let enabled: Vec<usize> = (0..raw.transitions()).filter(|&t| raw.is_enabled(&m, t)).collect();
            if enabled.is_empty() {
                break;
            }
            let t = enabled[rng.random_range(0..enabled.len())];
            raw.fire(&mut m, t);
            seq.push(t);
            seen.insert(m.clone());
            m.iter().zip(&mut max_seen).for_each(|(&x, mx)| *mx = (*mx).max(x));
        }
        if !seq.is_empty() {
            walks.push((seq, m));
        }
    }
    let mut out: Vec<Target> = Vec::new();
    let push = |out: &mut Vec<Target>, kind, marking: Vec<u32>, expect_yes, seq: Option<Vec<usize>>| {
        let dup = out.iter().any(|x| x.kind == kind && x.marking == marking);
        let n = out.iter().filter(|x| x.kind == kind && x.expect_yes == expect_yes).count();
        if !dup && n < PER_POLARITY {
            out.push(Target { kind, marking, expect_yes, firing_sequence: seq });
        }
    };
    for (seq, m) in &walks {
        if *m != raw.marking {
            push(&mut out, Kind::Reachability, m.clone(), true, Some(seq.clone()));
        }
        let mut c = m.clone();
        for _ in 0..rng.random_range(1..=3) {
            let marked: Vec<usize> = (0..c.len()).filter(|&p| c[p] > 0).collect();
            if c.iter().sum::<u32>() > 1 {
                c[marked[rng.random_range(0..marked.len())]] -= 1;
            }
        }
        push(&mut out, Kind::Coverability, c, true, Some(seq.clone()));
        let p = rng.random_range(0..m.len());
        let mut u = m.clone();
        u[p] = max_seen[p] + 1;
        push(&mut out, Kind::Coverability, u, false, None);
    }
    for _ in 0..100 {
        let m: Vec<u32> = raw.marking.iter().map(|&k| rng.random_range(0..=k + 2)).collect();
        if m.iter().any(|&k| k > 0) && !seen.contains(&m) {
            push(&mut out, Kind::Reachability, m, false, None);
        }
    }
    out
}
