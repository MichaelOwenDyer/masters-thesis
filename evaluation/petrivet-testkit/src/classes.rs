//! Structure generators for the six classes, following GENERATORS.md, and the small
//! construction assertions that are the testkit's only structural checking.
//!
//! Deviations from the verbatim specification (all "repairs preserving the class", spec 5.3):
//! * every transition gets at least one input and one output place. SM and circuits have this
//!   by definition. MG: |T| <= |P| is required (each place feeds and is fed by one transition),
//!   and the two place->transition vectors are made surjective. EFC/AC: every choice class (tree node)
//!   gets at least one place, so each transition has an input; T->P arcs get one extra random
//!   output where a transition has none. General: one random arc is added where missing.
//! * SM uses |T| >= |P| (a connected SM needs |T| >= |P|-1; below that almost every candidate is rejected).
//! * EFC: 1 <= K <= min(|P|,|T|), every class non-empty. AC: 1 <= M <= |P|.

use crate::raw::RawNet;
use rand::Rng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    General,
    Circuit,
    StateMachine,
    MarkedGraph,
    ExtendedFreeChoice,
    AsymmetricChoice,
}

impl Class {
    pub const ALL: [Class; 6] = [
        Class::General,
        Class::Circuit,
        Class::StateMachine,
        Class::MarkedGraph,
        Class::ExtendedFreeChoice,
        Class::AsymmetricChoice,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Class::General => "general",
            Class::Circuit => "circuit",
            Class::StateMachine => "sm",
            Class::MarkedGraph => "mg",
            Class::ExtendedFreeChoice => "efc",
            Class::AsymmetricChoice => "ac",
        }
    }

    /// Stable tag hashed into the seed; never renumber.
    pub fn tag(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bucket {
    /// |P| in 2..=5
    Small,
    /// |P| in 6..=8
    Medium,
    /// |P| in 9..=12
    Large,
}

impl Bucket {
    pub const ALL: [Bucket; 3] = [Bucket::Small, Bucket::Medium, Bucket::Large];

    pub fn places(self) -> (usize, usize) {
        match self {
            Bucket::Small => (2, 5),
            Bucket::Medium => (6, 8),
            Bucket::Large => (9, 12),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Bucket::Small => "2-5",
            Bucket::Medium => "6-8",
            Bucket::Large => "9-12",
        }
    }

    pub fn tag(self) -> u8 {
        self as u8
    }
}

/// Samples (|P|, |T|) for a class: |T| in [ceil(|P|/2), 2|P|], narrowed per class as documented above.
fn sizes(class: Class, bucket: Bucket, rng: &mut ChaCha8Rng) -> (usize, usize) {
    let (lo, hi) = bucket.places();
    let p = rng.random_range(lo..=hi);
    let (tlo, thi) = match class {
        Class::Circuit => (p, p),
        Class::StateMachine => (p, 2 * p),
        Class::MarkedGraph => (p.div_ceil(2), p),
        _ => (p.div_ceil(2), 2 * p),
    };
    (p, rng.random_range(tlo..=thi))
}

pub fn generate_structure(class: Class, bucket: Bucket, rng: &mut ChaCha8Rng) -> RawNet {
    let (p, t) = sizes(class, bucket, rng);
    match class {
        Class::General => general(p, t, rng),
        Class::Circuit => {
            let pt: Vec<_> = (0..p).map(|i| (i, i)).collect();
            let tp: Vec<_> = (0..p).map(|i| (i, (i + 1) % p)).collect();
            RawNet::from_arcs(p, p, &pt, &tp)
        }
        Class::StateMachine => {
            let pt: Vec<_> = (0..t).map(|j| (rng.random_range(0..p), j)).collect();
            let tp: Vec<_> = (0..t).map(|j| (j, rng.random_range(0..p))).collect();
            RawNet::from_arcs(p, t, &pt, &tp)
        }
        Class::MarkedGraph => {
            // Surjective onto T: first |T| places take a permutation, the rest are random.
            let vec_onto = |rng: &mut ChaCha8Rng| {
                let mut perm: Vec<usize> = (0..t).collect();
                perm.shuffle(rng);
                let mut v: Vec<usize> = (0..p).map(|_| rng.random_range(0..t)).collect();
                v[..t].copy_from_slice(&perm);
                v.shuffle(rng);
                v
            };
            let (input, output) = (vec_onto(rng), vec_onto(rng));
            let tp: Vec<_> = (0..p).map(|i| (input[i], i)).collect();
            let pt: Vec<_> = (0..p).map(|i| (i, output[i])).collect();
            RawNet::from_arcs(p, t, &pt, &tp)
        }
        Class::ExtendedFreeChoice => {
            let k = rng.random_range(1..=p.min(t));
            let tclass = surjective(t, k, rng);
            // Places: a permutation prefix gives every class a place; K means "no output transitions".
            let mut pclass: Vec<usize> = (0..p).map(|_| rng.random_range(0..=k)).collect();
            let mut order: Vec<usize> = (0..p).collect();
            order.shuffle(rng);
            order.iter().take(k).enumerate().for_each(|(c, &pl)| pclass[pl] = c);
            let pt: Vec<_> = (0..p).flat_map(|pl| (0..t).map(move |tr| (pl, tr))).filter(|&(pl, tr)| pclass[pl] == tclass[tr]).collect();
            let tp = random_outputs(p, t, rng);
            repair_outputs(RawNet::from_arcs(p, t, &pt, &tp), rng)
        }
        Class::AsymmetricChoice => {
            let m = rng.random_range(1..=p);
            let parent: Vec<usize> = (0..m).map(|i| if i == 0 { 0 } else { rng.random_range(0..i) }).collect();
            let node_of_t: Vec<usize> = (0..t).map(|_| rng.random_range(0..m)).collect();
            let mut subset: Vec<Vec<usize>> = vec![Vec::new(); m];
            node_of_t.iter().enumerate().for_each(|(tr, &n)| subset[n].push(tr));
            for i in (1..m).rev() {
                let child = subset[i].clone();
                subset[parent[i]].extend(child);
            }
            let mut node_of_p: Vec<usize> = (0..p).map(|_| rng.random_range(0..m)).collect();
            let mut order: Vec<usize> = (0..p).collect();
            order.shuffle(rng);
            order.iter().take(m).enumerate().for_each(|(n, &pl)| node_of_p[pl] = n);
            let pt: Vec<_> = (0..p).flat_map(|pl| subset[node_of_p[pl]].iter().map(move |&tr| (pl, tr))).collect();
            let tp = random_outputs(p, t, rng);
            repair_outputs(RawNet::from_arcs(p, t, &pt, &tp), rng)
        }
    }
}

/// A class assignment of `n` items into `k` non-empty classes.
fn surjective(n: usize, k: usize, rng: &mut ChaCha8Rng) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).map(|_| rng.random_range(0..k)).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.shuffle(rng);
    order.iter().take(k).enumerate().for_each(|(c, &i)| v[i] = c);
    v
}

/// Arbitrary T->P arcs: each pair independently with probability a/|P|, a in [1, 2.5].
fn random_outputs(p: usize, t: usize, rng: &mut ChaCha8Rng) -> Vec<(usize, usize)> {
    let prob = (rng.random_range(1.0..2.5) / p as f64).min(0.9);
    (0..t).flat_map(|tr| (0..p).map(move |pl| (tr, pl))).filter(|_| rng.random_bool(prob)).collect()
}

fn repair_outputs(mut net: RawNet, rng: &mut ChaCha8Rng) -> RawNet {
    for post in net.post.iter_mut().filter(|v| v.is_empty()) {
        post.push(rng.random_range(0..net.places));
    }
    net
}

/// General net: P->T and T->P pairs each included with a per-net density (expected degree 1 to 2.5);
/// a transition lacking an input or output gets one random arc.
fn general(p: usize, t: usize, rng: &mut ChaCha8Rng) -> RawNet {
    let prob = (rng.random_range(1.0..2.5) / p as f64).min(0.9);
    let pt: Vec<_> = (0..t).flat_map(|tr| (0..p).map(move |pl| (pl, tr))).filter(|_| rng.random_bool(prob)).collect();
    let tp = random_outputs(p, t, rng);
    let mut net = repair_outputs(RawNet::from_arcs(p, t, &pt, &tp), rng);
    for pre in net.pre.iter_mut().filter(|v| v.is_empty()) {
        pre.push(rng.random_range(0..p));
    }
    net
}

// ---- construction assertions (the testkit's only structural checking) ----

pub fn is_state_machine(n: &RawNet) -> bool {
    n.pre.iter().chain(&n.post).all(|v| v.len() == 1)
}

pub fn is_marked_graph(n: &RawNet) -> bool {
    (0..n.places).all(|p| in_count(n, p) == 1 && out_set(n, p).len() == 1)
}

fn in_count(n: &RawNet, p: usize) -> usize {
    n.post.iter().filter(|v| v.contains(&p)).count()
}

fn out_set(n: &RawNet, p: usize) -> Vec<usize> {
    (0..n.transitions()).filter(|&t| n.pre[t].contains(&p)).collect()
}

/// Places sharing an output transition have identical output sets.
pub fn is_extended_free_choice(n: &RawNet) -> bool {
    let outs: Vec<_> = (0..n.places).map(|p| out_set(n, p)).collect();
    outs.iter().all(|a| outs.iter().all(|b| a == b || a.iter().all(|t| !b.contains(t))))
}

/// Places sharing an output transition have nested output sets.
pub fn is_asymmetric_choice(n: &RawNet) -> bool {
    let outs: Vec<_> = (0..n.places).map(|p| out_set(n, p)).collect();
    let sub = |a: &[usize], b: &[usize]| a.iter().all(|t| b.contains(t));
    outs.iter().all(|a| outs.iter().all(|b| sub(a, b) || sub(b, a) || a.iter().all(|t| !b.contains(t))))
}

pub fn has_no_source_or_sink(n: &RawNet) -> bool {
    n.pre.iter().chain(&n.post).all(|v| !v.is_empty())
}

/// The defining property of `class` (general: nothing to check).
pub fn satisfies(class: Class, n: &RawNet) -> bool {
    match class {
        Class::General => true,
        Class::Circuit => is_state_machine(n) && is_marked_graph(n),
        Class::StateMachine => is_state_machine(n),
        Class::MarkedGraph => is_marked_graph(n),
        Class::ExtendedFreeChoice => is_extended_free_choice(n),
        Class::AsymmetricChoice => is_asymmetric_choice(n),
    }
}
