//! Scaling families (`fam`, spec 5.4): parametric nets whose verdicts are known by construction.
//!
//! Every family has a size parameter `n`, a builder returning a [`RawNet`], and the verdicts of
//! the queries Q1 (deadlock-free), Q2 (bounded), Q3 (live, L4) and Q4 (quasi-live) as a function
//! of `n`, each with a proof that does not enumerate the state space. A query that cannot be
//! settled by a short argument is left out of `answers`. Q0, Q5 and Q6 are never claimed.
//!
//! The tests in `tests/families.rs` compare every verdict with petrivet's explicit-state
//! procedures for small `n`; the verdicts here are the truth only where the argument is sound.
//!
//! Default sizes: `n = 2^k` for k in `1..=10` (see [`DEFAULT_MIN_EXP`], [`DEFAULT_MAX_EXP`]).
//! Reasoning: n = 2 is trivial for every tool; for the families with an exponential state space
//! (`sm_hub_and_spokes`: C(2n, n) markings; `sm_two_cycles_n_tokens`, `efc_forkjoin`,
//! `mg_forkjoin_cycle`, `mg_buffer_ring`) n = 16 already exceeds 10^6 to 10^8 markings, and the
//! quadratic ones (`ac_cycle`, `general_cycle`: about n^2 markings) reach 10^6 at n = 1024. The
//! linear ones (rings) mainly show the cost of the structural procedures. The by-hand pilot tunes
//! the range. Minimum sizes (`min_n`) are per family and documented at each construction.
//!
//! Glossary of the names used below.
//! - hub and spokes: a state machine with one central place and n arms; each arm is a pair of
//!   transitions that move a token from the centre onto the arm place and back.
//! - two cycles: two disjoint cyclic state machines A and B joined by a one-way switch A -> B, so
//!   the net is not strongly connected and tokens can never return to A.
//! - `n_tokens`: the same net with n initial tokens instead of one.
//! - choice ring: a ring of stages where each stage offers a choice between two parallel
//!   branches (a state machine; free choice only trivially).
//! - ring with sink: a cyclic state machine with one extra transition that drains tokens into a
//!   place that has no outgoing transition.
//! - buffer ring: a marked graph modelling a ring pipeline of one-slot buffers: the forward
//!   place r_i holds an item between stages i and i+1, the backward place b_i holds a free slot.
//! - unmarked circuit: a directed circuit of a marked graph that carries no token; its
//!   transitions can never fire (the token count of a circuit is invariant).
//! - forkjoin: a fork transition splits one token into n parallel branches that a join
//!   transition synchronises again (the unbounded marked-graph variant has no way back).
//! - leaky: a variant in which one choice branch loses part of its output, so a token that takes
//!   it gets stuck and a deadlock becomes reachable.
//! - unbounded representative: `mg_forkjoin_unbounded`, the only family that is not bounded.

use crate::raw::RawNet;
use crate::{Class, detected_name};
use petrivet::net::class::NetClass;
use serde::Serialize;

pub const FAM_VERSION: &str = "fam-2";
pub const DEFAULT_MIN_EXP: u32 = 1;
pub const DEFAULT_MAX_EXP: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Query {
    /// Deadlock-free.
    Q1,
    /// Bounded.
    Q2,
    /// Live (L4).
    Q3,
    /// Quasi-live (L1).
    Q4,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KnownAnswer {
    pub query: Query,
    pub verdict: bool,
    pub origin: &'static str,
}

pub struct FamilySpec {
    pub name: &'static str,
    pub intended: Class,
    /// Smallest admissible n (smaller sizes of the geometric series are skipped).
    pub min_n: usize,
    pub build: fn(usize) -> RawNet,
    /// Verdicts as a function of n (valid for every n >= `min_n`).
    pub answers: fn(usize) -> Vec<(Query, bool)>,
}

#[derive(Clone, Debug)]
pub struct FamilyInstance {
    pub family: &'static str,
    pub n: usize,
    pub intended: Class,
    pub raw: RawNet,
    pub pnml: String,
    /// blake3 of the PNML bytes, hex.
    pub id: String,
    pub detected: NetClass,
    pub known: Vec<KnownAnswer>,
}

pub fn generate(spec: &FamilySpec, n: usize) -> FamilyInstance {
    assert!(n >= spec.min_n, "{} needs n >= {}", spec.name, spec.min_n);
    let raw = (spec.build)(n);
    assert!(raw.is_connected(), "family bug: {} n={n} disconnected", spec.name);
    let sys = raw.to_petrivet().expect("petrivet rejected a family net");
    let pnml = sys.to_pnml().expect("PNML export failed");
    let id = blake3::hash(pnml.as_bytes()).to_hex().to_string();
    let known = (spec.answers)(n)
        .into_iter()
        .map(|(query, verdict)| KnownAnswer { query, verdict, origin: "construction" })
        .collect();
    FamilyInstance { family: spec.name, n, intended: spec.intended, raw, pnml, id, detected: sys.class(), known }
}

/// All families at sizes 2^min_exp ..= 2^max_exp (sizes below a family's `min_n` are skipped).
/// Order: family order of [`FAMILIES`], then increasing n.
pub fn generate_all(min_exp: u32, max_exp: u32) -> Vec<FamilyInstance> {
    FAMILIES
        .iter()
        .flat_map(|spec| {
            (min_exp..=max_exp)
                .map(|k| 1usize << k)
                .filter(|&n| n >= spec.min_n)
                .map(move |n| generate(spec, n))
        })
        .collect()
}

pub fn detected(i: &FamilyInstance) -> &'static str {
    detected_name(i.detected)
}

use Query::{Q1, Q2, Q3, Q4};

pub const FAMILIES: &[FamilySpec] = &[
    FamilySpec { name: "circuit", intended: Class::Circuit, min_n: 2, build: circuit,                                                       answers: |_| vec![(Q1, true), (Q2, true), (Q3, true), (Q4, true)] },
    FamilySpec { name: "sm_hub_and_spokes", intended: Class::StateMachine, min_n: 2, build: sm_hub_and_spokes,                              answers: |_| vec![(Q1, true), (Q2, true), (Q3, true), (Q4, true)] },
    FamilySpec { name: "sm_two_cycles", intended: Class::StateMachine, min_n: 4, build: |n| sm_two_cycles(n, 1),                     answers: |_| vec![(Q1, true), (Q2, true), (Q3, false), (Q4, true)] },
    FamilySpec { name: "sm_two_cycles_n_tokens", intended: Class::StateMachine, min_n: 4, build: |n| sm_two_cycles(n, n as u32),            answers: |_| vec![(Q1, true), (Q2, true), (Q3, false), (Q4, true)] },
    FamilySpec { name: "sm_choice_ring", intended: Class::StateMachine, min_n: 2, build: sm_choice_ring,                                    answers: |_| vec![(Q1, true), (Q2, true), (Q3, true), (Q4, true)] },
    FamilySpec { name: "sm_ring_with_sink", intended: Class::StateMachine, min_n: 2, build: sm_ring_with_sink,                              answers: |_| vec![(Q1, false), (Q2, true), (Q3, false), (Q4, true)] },
    FamilySpec { name: "mg_buffer_ring", intended: Class::MarkedGraph, min_n: 3, build: mg_buffer_ring,                                     answers: |_| vec![(Q1, true), (Q2, true), (Q3, true), (Q4, true)] },
    FamilySpec { name: "mg_buffer_ring_unmarked_circuit", intended: Class::MarkedGraph, min_n: 4, build: mg_buffer_ring_unmarked_circuit,   answers: |_| vec![(Q1, false), (Q2, true), (Q3, false), (Q4, false)] },
    FamilySpec { name: "mg_forkjoin_cycle", intended: Class::MarkedGraph, min_n: 2, build: mg_forkjoin_cycle,                               answers: |_| vec![(Q1, true), (Q2, true), (Q3, true), (Q4, true)] },
    FamilySpec { name: "mg_forkjoin_unbounded", intended: Class::MarkedGraph, min_n: 2, build: mg_forkjoin_unbounded,                       answers: |_| vec![(Q1, true), (Q2, false), (Q3, true), (Q4, true)] },
    FamilySpec { name: "efc_forkjoin", intended: Class::ExtendedFreeChoice, min_n: 2, build: |n| efc_forkjoin(n, false),              answers: |_| vec![(Q1, true), (Q2, true), (Q3, true), (Q4, true)] },
    FamilySpec { name: "efc_forkjoin_leaky", intended: Class::ExtendedFreeChoice, min_n: 2, build: |n| efc_forkjoin(n, true),          answers: |_| vec![(Q1, false), (Q2, true), (Q3, false), (Q4, true)] },
    FamilySpec { name: "ac_cycle", intended: Class::AsymmetricChoice, min_n: 2, build: |n| ac_cycle(n, true),                       answers: |_| vec![(Q1, true), (Q2, true), (Q3, true), (Q4, true)] },
    FamilySpec { name: "ac_cycle_no_low_token", intended: Class::AsymmetricChoice, min_n: 2, build: |n| ac_cycle(n, false),         answers: |_| vec![(Q1, true), (Q2, true), (Q3, false), (Q4, false)] },
    FamilySpec { name: "general_cycle", intended: Class::General, min_n: 2, build: general_cycle,                                           answers: |_| vec![(Q1, true), (Q2, true), (Q3, true), (Q4, true)] },
];

pub fn by_name(name: &str) -> Option<&'static FamilySpec> {
    FAMILIES.iter().find(|f| f.name == name)
}

fn marked(mut net: RawNet, marking: &[(usize, u32)]) -> RawNet {
    for &(p, k) in marking {
        net.marking[p] = k;
    }
    net
}

// ---------------------------------------------------------------------------------------------
// circuit: places p_i, transitions t_i, arcs p_i -> t_i -> p_{(i+1) mod n}; one token on p_0.
// Proof: the token is the only token and is conserved (every transition moves it), so the net is
// 1-bounded (Q2). Exactly one transition, the one after the token, is enabled in every marking,
// so no deadlock (Q1). The token visits every place in turn, so every transition fires again
// from every reachable marking (Q3, hence Q4).
fn circuit(n: usize) -> RawNet {
    let pt: Vec<_> = (0..n).map(|i| (i, i)).collect();
    let tp: Vec<_> = (0..n).map(|i| (i, (i + 1) % n)).collect();
    marked(RawNet::from_arcs(n, n, &pt, &tp), &[(0, 1)])
}

// sm_hub_and_spokes: hub and spokes. Place 0 is the centre, place 1+i the arm i; t_enter_i = centre -> arm_i
// (transition 2i), t_exit_i = arm_i -> centre (2i+1). n tokens on the centre.
// Proof: every transition has one input and one output place, so the token count n is
// conserved (Q2). Net is strongly connected. Live (Q3): in any marking some token exists; if
// it is on arm_k fire t_exit_k to bring it to the centre; from a token on the centre, t_enter_i
// fires, then t_exit_i. So every transition can be enabled from every marking, hence Q4 and Q1
// (live implies deadlock-free as the net has transitions). R has C(2n, n) markings.
fn sm_hub_and_spokes(n: usize) -> RawNet {
    let (mut pt, mut tp) = (Vec::new(), Vec::new());
    for i in 0..n {
        pt.push((0, 2 * i));
        tp.push((2 * i, 1 + i));
        pt.push((1 + i, 2 * i + 1));
        tp.push((2 * i + 1, 0));
    }
    marked(RawNet::from_arcs(n + 1, 2 * n, &pt, &tp), &[(0, n as u32)])
}

// sm_two_cycles / sm_two_cycles_n_tokens: two cycles A and B of half = max(n/2, 2) places each (n >= 4 so sizes differ) (transitions
// a_i: A_i -> A_{i+1}, b_i: B_i -> B_{i+1}) plus a one-way switch A_0 -> B_0. `tokens` tokens on
// A_0 (1 for sm_two_cycles as in the benchmark, n for the n_tokens variant).
// Proof: state machine, token count conserved (Q2). Deadlock-free (Q1): a token on A_i enables
// a_i, a token on B_i enables b_i, and at least one token exists. Quasi-live (Q4): tokens walk
// A_0 .. A_{half-1} so every a_i fires, switch fires from A_0, and then the token walks B so
// every b_i fires. Not live (not Q3): firing the switch `tokens` times puts every token in B
// (a token on A_0 is always available to switch while any token remains in A, since a token
// can walk round A back to A_0); then no token can ever reach A again, so every a_i and the
// switch are dead there.
fn sm_two_cycles(n: usize, tokens: u32) -> RawNet {
    let half = (n / 2).max(2);
    let (mut pt, mut tp) = (Vec::new(), Vec::new());
    for i in 0..half {
        pt.push((i, i));
        tp.push((i, (i + 1) % half));
        pt.push((half + i, half + i));
        tp.push((half + i, half + (i + 1) % half));
    }
    pt.push((0, 2 * half));
    tp.push((2 * half, half));
    marked(RawNet::from_arcs(2 * half, 2 * half + 1, &pt, &tp), &[(0, tokens)])
}

// sm_ring_with_sink: ring places p_i = i (i < n), transitions t_i = i: p_i -> p_{(i+1) mod n},
// plus `leak` = transition n: p_0 -> s, where s = place n has no outgoing transition.
// max(1, n/2) tokens on p_0. min_n = 2 (a ring needs two places to be more than a self-loop).
// Proof: state machine, so the token count is conserved (Q2). Not deadlock-free (not Q1):
// repeatedly walk a token round the ring to p_0 and fire `leak`; after as many leaks as there are
// tokens every token is on s, and no transition is enabled (every transition other than `leak`
// consumes from a ring place, `leak` from p_0, all empty). That marking is reachable, but only
// after at least one leak per token, i.e. late. Not live (not Q3): in that deadlock nothing can
// ever fire. Quasi-live (Q4): a token walks the ring and fires every t_i, and fires `leak` from p_0.
fn sm_ring_with_sink(n: usize) -> RawNet {
    let (mut pt, mut tp) = (Vec::new(), Vec::new());
    for i in 0..n {
        pt.push((i, i));
        tp.push((i, (i + 1) % n));
    }
    pt.push((0, n));
    tp.push((n, n));
    marked(RawNet::from_arcs(n + 1, n + 1, &pt, &tp), &[(0, (n / 2).max(1) as u32)])
}

// mg_forkjoin_unbounded (benchmark `marked_graph/live`): shared place 0 with self-loop on t_fork
// (transition 0); t_fork also marks p_i = place 1+i for each of n paths; t_join (transition 1)
// consumes one token from every p_i and has no output place. One token on the shared place.
// This is the unbounded representative of the set: it is deliberately not bounded and not
// strongly connected, so that petrivet's structural procedures for unbounded marked graphs are
// exercised. min_n = 2 (with one path it would degenerate to a chain).
// Proof: marked graph. Q3: t_fork is enabled in every marking (the shared token never leaves,
// since t_fork returns it), and after one t_fork every p_i holds a token, so t_join is enabled;
// from any marking firing t_fork once enables t_join, so both can always be enabled again.
// Hence Q4 and Q1. Not bounded (not Q2): t_fork can fire arbitrarily often while t_join stays
// disabled, so each p_i accumulates unboundedly many tokens. NOTE: unbounded, so spec section 3
// excludes Q1, Q3, Q4 for truth from tools; the construction argument above still holds, but
// the analysis may want to treat these three as "construction only".
fn mg_forkjoin_unbounded(n: usize) -> RawNet {
    let mut pt = vec![(0, 0)];
    let mut tp = vec![(0, 0)];
    for i in 0..n {
        tp.push((0, 1 + i));
        pt.push((1 + i, 1));
    }
    marked(RawNet::from_arcs(n + 1, 2, &pt, &tp), &[(0, 1)])
}

// mg_forkjoin_cycle: places home = 0, a_i = 1+i, c_i = 1+n+i; transitions fork = 0, s_i = 1+i,
// join = n+1. Arcs: home -> fork -> a_i -> s_i -> c_i -> join -> home. One token on home.
// min_n = 2 (n = 1 would be a plain circuit).
// Proof: marked graph (every place has one producer and one consumer), strongly connected. Every
// simple circuit is home -> fork -> a_i -> s_i -> c_i -> join -> home for one i, so every circuit
// passes through `home`; each circuit's token count is invariant and initially 1. Hence every
// circuit is marked (live by Commoner/Murata: Q3, hence Q4 and Q1) and every place lies on a
// circuit with one token (1-bounded: Q2). After `fork` the s_i fire independently, so 2^n
// markings of {a_i, c_i} choices are reachable.
fn mg_forkjoin_cycle(n: usize) -> RawNet {
    let (mut pt, mut tp) = (vec![(0, 0)], Vec::new());
    for i in 0..n {
        tp.push((0, 1 + i)); // fork -> a_i
        pt.push((1 + i, 1 + i)); // a_i -> s_i
        tp.push((1 + i, 1 + n + i)); // s_i -> c_i
        pt.push((1 + n + i, n + 1)); // c_i -> join
    }
    tp.push((n + 1, 0)); // join -> home
    marked(RawNet::from_arcs(2 * n + 1, n + 2, &pt, &tp), &[(0, 1)])
}

// mg_buffer_ring: n >= 3 transitions t_i (pipeline stages) on a ring; forward place r_i (t_i ->
// t_{i+1}) and backward place b_i (t_{i+1} -> t_i), i mod n. Places r_i = i, b_i = n+i.
// Buffer interpretation: the pair (r_i, b_i) is a one-slot buffer between stage i and stage i+1;
// a token on r_i is an item in the slot, a token on b_i is a free slot (r_i + b_i = 1 always).
// Marking: every slot free (b_i = 1 for all i) and one item at the start (r_0 = 1). t_i has input
// places r_{i-1} (an item from upstream) and b_i (a free slot downstream).
// Proof: marked graph (each place has one producer and one consumer). Its simple directed
// circuits are the forward ring (contains r_0, marked), the backward ring (all b_i marked),
// and the 2-cycles {r_i, b_i} (contain b_i, marked): a circuit cannot reverse direction except
// as a 2-cycle. Every circuit is marked, so the net is live (Commoner / Murata) (Q3, hence Q4
// and Q1), and each place lies on a circuit with at most 2 tokens, whose token count is
// invariant, so it is bounded (Q2). Reachable markings grow exponentially with n.
fn mg_buffer_ring(n: usize) -> RawNet {
    let (mut pt, mut tp) = (Vec::new(), Vec::new());
    for i in 0..n {
        let prev = (i + n - 1) % n;
        pt.push((prev, i)); // r_{i-1} -> t_i
        pt.push((n + i, i)); // b_i -> t_i
        tp.push((i, i)); // t_i -> r_i
        tp.push((i, n + prev)); // t_i -> b_{i-1}
    }
    let mut m: Vec<(usize, u32)> = (0..n).map(|i| (n + i, 1)).collect();
    m.push((0, 1));
    marked(RawNet::from_arcs(2 * n, n, &pt, &tp), &m)
}

// mg_buffer_ring_unmarked_circuit: the net of mg_buffer_ring with the token of the free-slot
// place b_k removed, k = n/2, so the buffer between t_k and t_{k+1} has neither an item nor a
// free slot. min_n = 4 (so that k = n/2 >= 2: r_k is then unmarked, since only r_0 is
// marked initially, and the other transitions have room to run before the deadlock).
// Proof: marked graph, strongly connected. The 2-circuit {r_k, b_k} has token count 0 (r_k = 0
// initially since k >= 1 and only r_0 is marked), and the token count of a circuit is invariant
// under firing, so the circuit stays unmarked forever. Its transitions t_k (needs b_k) and
// t_{k+1} (needs r_k) can therefore never fire (not Q4, hence not Q3). Every transition is
// downstream of every circuit in a strongly connected marked graph: a transition that needs
// tokens from the dead part eventually runs out of them. The net is bounded (Q2: token counts
// of the circuits through each place are invariant and finite). Each firing of a live transition
// consumes a token that cannot be replenished without t_k or t_{k+1}, so the number of firings
// is bounded and every maximal run ends in a deadlock (not Q1) after a long run of the others.
fn mg_buffer_ring_unmarked_circuit(n: usize) -> RawNet {
    let mut net = mg_buffer_ring(n);
    net.marking[n + n / 2] = 0;
    net
}

// sm_choice_ring (benchmark `free_choice`): ring of n stages; stage i has entry place p_i and two
// parallel branches p_i -> choose -> q -> commit -> p_{i+1}. One token on p_0. Every transition
// has exactly one input and one output place, so this is a state machine (free choice only
// trivially, since the branches of a stage share only the single-output place p_i).
// Proof: every transition has one input and one output, so the single token is conserved (Q2).
// The token is always either on a stage place (both choose transitions enabled) or on a q place
// (its commit enabled): no deadlock (Q1). From any marking the token can reach any stage place
// and take either branch, so every transition can be enabled again (Q3, Q4).
fn sm_choice_ring(n: usize) -> RawNet {
    let (mut pt, mut tp) = (Vec::new(), Vec::new());
    for i in 0..n {
        for c in 0..2 {
            let (choose, commit, q) = (4 * i + 2 * c, 4 * i + 2 * c + 1, n + 2 * i + c);
            pt.push((i, choose));
            tp.push((choose, q));
            pt.push((q, commit));
            tp.push((commit, (i + 1) % n));
        }
    }
    marked(RawNet::from_arcs(3 * n, 4 * n, &pt, &tp), &[(0, 1)])
}

// efc_forkjoin / efc_forkjoin_leaky: ring of n stages; stage i has place p_i = i with two choice transitions
// u_i = 3i, v_i = 3i+1 (input only p_i; outputs x_i = n+2i and y_i = n+2i+1), and a join
// w_i = 3i+2 (inputs x_i and y_i, output p_{i+1}). max(1, n/2) tokens on p_0. This is (extended)
// free choice and genuinely not a state machine or marked graph.
// Proof: Q2: place weights p = 2, x = y = 1 give a positive invariant (u, v: 2 -> 1+1; w:
// 1+1 -> 2), so tokens are bounded by the weighted initial total. Q3 by Commoner's theorem
// (free choice is live iff every siphon contains a marked trap): let S be a non-empty siphon.
// If p_{i+1} is in S then w_i, which produces into it, must consume from S, so x_i or y_i is
// in S; if x_i (or y_i) is in S then u_i, which produces into it, must consume from S, so p_i
// is in S. Going round the ring S contains every p_i and one z_i of {x_i, y_i} per stage; if
// no p is in S, S is empty as above. The set T = {p_i} + {z_i} is a trap (T-outputs u_i, v_i
// produce z_i in T, w_i produces p_{i+1}, and for z_i in T: its only consumer w_i produces
// p_{i+1}), T is inside S, and p_0 is marked. So the net is live (Q3), hence quasi-live (Q4)
// and deadlock-free (Q1).
// Leaky variant (`leaky = true`): the choice transition v_i produces only x_i, not y_i (u_i still
// produces both). Still extended free choice: u_i and v_i share only the input place p_i, and
// x_i, y_i are consumed only by w_i. A token that takes v_i is stuck: w_i needs y_i, which only
// u_i produces.
// Proof (leaky): Q2: weights p = 2, x = y = 1 are a non-increasing invariant (u: 2 -> 2, v:
// 2 -> 1, w: 2 -> 2), so the weighted total never exceeds its initial value and the net is
// bounded. Not Q1: fire v_0 for every token on p_0 (all tokens start there); then all tokens sit
// on x_0, p_0 is empty, w_0 lacks y_0, and no transition is enabled: a reachable deadlock. Not Q3:
// nothing can fire in that deadlock. Q4: from the initial marking fire u_i then w_i at stages
// 0..i-1 to move a token to p_i, then u_i and w_i (w_i needs both x_i and y_i, so it is enabled
// after u_i) cover u_i, w_i; separately firing v_i from p_i covers v_i. Every transition of every
// stage is thereby fired in some run.
fn efc_forkjoin(n: usize, leaky: bool) -> RawNet {
    let (mut pt, mut tp) = (Vec::new(), Vec::new());
    for i in 0..n {
        let (u, v, w) = (3 * i, 3 * i + 1, 3 * i + 2);
        let (x, y) = (n + 2 * i, n + 2 * i + 1);
        for t in [u, v] {
            pt.push((i, t));
            tp.push((t, x));
            if !(leaky && t == v) {
                tp.push((t, y));
            }
        }
        pt.push((x, w));
        pt.push((y, w));
        tp.push((w, (i + 1) % n));
    }
    marked(RawNet::from_arcs(3 * n, 3 * n, &pt, &tp), &[(0, (n / 2).max(1) as u32)])
}

// ac_cycle (benchmark `asym_choice`): n segments with main place m_i = i and low place l_i =
// n+i; t_high_i (transition i): m_i -> m_{i+1}; t_low_i (transition n+i): m_i + l_i ->
// m_{i+1} + l_{i+1}. Tokens: m_0 = 1, l_0 = 1. l_i's consumers {t_low_i} are a subset of m_i's
// {t_high_i, t_low_i}: asymmetric choice, not free choice.
// Proof: transitions touching the m places move the single m token one step, so the m token
// count is 1 and the l token count is 1 (t_low moves the l token too): bounded (Q2). The m
// token always enables t_high at its position: no deadlock (Q1). From a marking with the m
// token on i and the l token on j, t_high steps carry the m token to j (the ring is closed),
// where t_low_j fires; t_high_i fires wherever the m token is. So every transition can be
// enabled again from every reachable marking (Q3, Q4). About n^2 markings.
// ac_cycle_no_low_token (`low_token = false`): the same net without the token on l_0, so no
// t_low can ever fire (the l places are produced only by t_low, which needs an l token).
// Proof: the m token cycles via t_high forever: bounded (Q2; the m token is conserved) and
// deadlock-free (Q1). Every t_low_i needs a token on l_i, but all l_i start empty and are
// produced only by a t_low, so no t_low ever fires: not quasi-live (not Q4), hence not live (not Q3).
fn ac_cycle(n: usize, low_token: bool) -> RawNet {
    let (mut pt, mut tp) = (Vec::new(), Vec::new());
    for i in 0..n {
        let next = (i + 1) % n;
        pt.push((i, i));
        tp.push((i, next));
        pt.push((i, n + i));
        pt.push((n + i, n + i));
        tp.push((n + i, next));
        tp.push((n + i, n + next));
    }
    let m: &[(usize, u32)] = if low_token { &[(0, 1), (n, 1)] } else { &[(0, 1)] };
    marked(RawNet::from_arcs(2 * n, 2 * n, &pt, &tp), m)
}

// general_cycle (benchmark `general`): n clusters; a-place a_i = i, b-place b_i = n+i;
// transitions shared_i = 3i (a_i + b_i -> a_{i+1} + b_{i+1}), priv_a_i = 3i+1 (a_i -> a_{i+1}),
// priv_b_i = 3i+2 (b_i -> b_{i+1}). Tokens a_0 = 1, b_0 = 1. Symmetric confusion makes it a
// general net.
// Proof: priv moves and shared moves keep exactly one token in the a places and one in the b
// places: bounded (Q2). The a token always enables priv_a at its position: no deadlock (Q1).
// From any marking (a on i, b on j) both tokens can walk independently around the ring with
// the priv transitions, so each priv_a_k, priv_b_k fires, and walking both to the same k
// enables shared_k: every transition can be enabled again (Q3, Q4).
fn general_cycle(n: usize) -> RawNet {
    let (mut pt, mut tp) = (Vec::new(), Vec::new());
    for i in 0..n {
        let next = (i + 1) % n;
        let (s, pa, pb) = (3 * i, 3 * i + 1, 3 * i + 2);
        pt.push((i, s));
        pt.push((n + i, s));
        tp.push((s, next));
        tp.push((s, n + next));
        pt.push((i, pa));
        tp.push((pa, next));
        pt.push((n + i, pb));
        tp.push((pb, n + next));
    }
    marked(RawNet::from_arcs(2 * n, 3 * n, &pt, &tp), &[(0, 1), (n, 1)])
}
