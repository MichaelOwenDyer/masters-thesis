//! Cache coherence of the library: the answers to deadlock-freedom, boundedness, liveness, quasi-liveness,
//! exact liveness and exact boundedness are cached per `PetriNet`, so the answer to one question must not
//! depend on which questions were asked before. For generated nets of every class and size bucket the six
//! questions are asked (a) on a fresh `PetriNet` each and (b) on one `PetriNet` in several seeded permutations;
//! all answers (including `None` against `Some`) must be identical. (c) Where the reachability graph is
//! small, every `Some` answer is compared with an explicit-state computation over the markings found by the
//! library's explorer, with edges from the testkit's own firing rule.
//!
//! Finding recorded while writing this test: `is_deadlock_free().call()` (no timeout) does not terminate on an
//! unbounded net that has no deadlock and no structural shortcut (the fallback explores the infinite reachability
//! graph with `deadline: None`); the CLI reports `unknown` after its own deadline. For unbounded nets the test
//! therefore asks this one question with a timeout.
//!
//! Any failure is a library finding (or a generator bug): the message prints the minimal failing net as PNML.

use petrivet::net::{Net, Place};
use petrivet::state_space::ExplorationOrder;
use petrivet::system::PetriNet;
use petrivet::system::marking::Marking;
use petrivet::system::liveness::LivenessLevel;
use petrivet::system::boundedness::Boundedness;
use petrivet_testkit::raw::RawNet;
use petrivet_testkit::{Bucket, Class, generate_instance};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

const MASTER_SEED: u64 = 20_261_022;
/// 6 classes x 3 buckets x 12 = 216 nets.
const PER_CELL: u32 = 12;
/// Nets whose coverability graph exceeds this many markings are skipped (liveness would fall back to
/// explicit exploration of a huge graph).
const COVER_CAP: usize = 4_000;
/// Reachability graphs above this many markings are not used as ground truth.
const REACH_CAP: usize = 4_000;
const UNBOUNDED_DF_TIMEOUT: Duration = Duration::from_millis(100);
const PERMUTATIONS: u64 = 4;
const QUESTIONS: usize = 6;
const NAMES: [&str; QUESTIONS] = ["deadlock_free", "bounded", "live(L4)", "quasi_live(L1)", "liveness()", "boundedness()"];

/// Asks question `q` and returns the answer rendered as text (so `None` and `Some` compare strictly).
///
/// `df_timeout`: deadlock-freedom has no termination guarantee on an unbounded net without deadlocks (the
/// fallback explores the infinite reachability graph, see the module docs), so for such nets it is asked with a
/// timeout; a timed-out `None` is then not a coherence difference (see `differs`).
fn ask(sys: &PetriNet<Net>, q: usize, df_timeout: Option<Duration>) -> String {
    if std::env::var_os("CC_TRACE").is_some() {
        eprintln!("  ask {}", NAMES[q]);
    }
    match q {
        0 => match df_timeout {
            Some(t) => format!("{:?}", sys.is_deadlock_free().timeout(t).call()),
            None => format!("{:?}", sys.is_deadlock_free().call()),
        },
        1 => format!("{:?}", sys.is_bounded().call()),
        2 => format!("{:?}", sys.is_live().call()),
        3 => format!("{:?}", sys.is_live().level(LivenessLevel::L1).call()),
        4 => format!("{:?}", sys.liveness().call()),
        5 => format!("{:?}", sys.boundedness().call()),
        _ => unreachable!(),
    }
}

fn fresh(raw: &RawNet) -> PetriNet<Net> {
    raw.to_petrivet().expect("generated net is valid")
}

/// Explicit ground truth over the library's reachability graph, `None` if it exceeds the cap.
struct Truth {
    deadlock_free: bool,
    quasi_live: bool,
    live: bool,
    level: LivenessLevel,
    max_tokens: u32,
}

fn explicit(raw: &RawNet) -> Option<Truth> {
    // same construction as `RawNet::to_petrivet`, but keeping the place handles in raw index order
    // (`Net::places()` does not iterate in creation order)
    let mut b = Net::builder();
    let places: Vec<Place> = (0..raw.places).map(|_| b.add_place()).collect();
    for t in 0..raw.transitions() {
        let tr = b.add_transition();
        raw.pre[t].iter().for_each(|&p| { b.add_arc((places[p], tr)); });
        raw.post[t].iter().for_each(|&p| { b.add_arc((tr, places[p])); });
    }
    let marking: Marking<u32> = places.iter().zip(&raw.marking).map(|(&p, &k)| (p, k)).collect();
    let sys = PetriNet::new(b.build().unwrap(), marking);
    let mut ex = sys.explore_reachability_graph(ExplorationOrder::BreadthFirst);
    let vec_of = |m: &Marking<u32>| -> Vec<u32> { places.iter().map(|&p| m.get(p)).collect() };
    let mut markings: Vec<Vec<u32>> = vec![vec_of(&ex.initial_marking())];
    while let Some(step) = ex.explore_next() {
        if step.is_new {
            markings.push(vec_of(&step.marking));
            if markings.len() > REACH_CAP {
                return None;
            }
        }
    }
    assert_eq!(markings.len(), ex.marking_count(), "explorer marking count");
    let index: HashMap<&Vec<u32>, usize> = markings.iter().enumerate().map(|(i, m)| (m, i)).collect();
    let nt = raw.transitions();
    // edges[i] = (transition, target)
    let mut edges: Vec<Vec<(usize, usize)>> = vec![Vec::new(); markings.len()];
    for (i, m) in markings.iter().enumerate() {
        for t in 0..nt {
            let mut next = m.clone();
            if raw.fire(&mut next, t) {
                let j = *index.get(&next).unwrap_or_else(|| panic!("{}", report(raw, &format!("successor {next:?} of {m:?} via t{t} not among the explorer markings {markings:?}"))));
                edges[i].push((t, j));
            }
        }
    }
    let n = markings.len();
    // reverse reachability closure per marking: reach[i] = set of markings reachable from i (n is small)
    let reach: Vec<HashSet<usize>> = (0..n)
        .map(|s| {
            let mut seen = HashSet::from([s]);
            let mut stack = vec![s];
            while let Some(u) = stack.pop() {
                for &(_, v) in &edges[u] {
                    if seen.insert(v) {
                        stack.push(v);
                    }
                }
            }
            seen
        })
        .collect();
    let mut level = LivenessLevel::L4;
    let mut live = true;
    let mut quasi = true;
    for t in 0..nt {
        let enabled_at: Vec<usize> = (0..n).filter(|&i| edges[i].iter().any(|&(x, _)| x == t)).collect();
        let l = if enabled_at.is_empty() {
            LivenessLevel::L0
        } else if (0..n).all(|i| reach[i].iter().any(|j| enabled_at.contains(j))) {
            LivenessLevel::L4
        } else if (0..n).any(|i| edges[i].iter().any(|&(x, j)| x == t && reach[j].contains(&i))) {
            // finite graph: t on a cycle is L3 (and L2)
            LivenessLevel::L3
        } else {
            LivenessLevel::L1
        };
        live &= l == LivenessLevel::L4;
        quasi &= l != LivenessLevel::L0;
        level = level.min(l);
    }
    Some(Truth {
        deadlock_free: edges.iter().all(|e| !e.is_empty()),
        quasi_live: quasi,
        live,
        level,
        max_tokens: markings.iter().flatten().copied().max().unwrap_or(0),
    })
}

/// A difference between two answers; for a timed-out question `None` against `Some` is tolerated, `Some(a)`
/// against `Some(b)` never.
fn differs(a: &str, b: &str, timed: bool) -> bool {
    a != b && !(timed && (a == "None" || b == "None"))
}

/// `Some(bounded)` if the coverability graph is small enough to run the questions at all, `None` otherwise.
fn small_enough(raw: &RawNet) -> Option<bool> {
    let sys = fresh(raw);
    let mut ex = sys.explore_coverability_graph(ExplorationOrder::BreadthFirst);
    let mut bounded = true;
    while let Some(step) = ex.explore_next() {
        bounded &= !step.marking.has_omega();
        if ex.marking_count() > COVER_CAP {
            return None;
        }
    }
    Some(bounded)
}

fn report(raw: &RawNet, what: &str) -> String {
    format!("{what}\nnet: {} places, {} transitions, initial {:?}\npre {:?}\npost {:?}\nPNML:\n{}", raw.places, raw.transitions(), raw.marking, raw.pre, raw.post, raw.to_pnml().unwrap_or_default())
}

#[test]
fn answers_do_not_depend_on_question_order() {
    let mut nets = 0;
    let mut skipped = 0;
    let mut unbounded = 0;
    let mut with_truth = 0;
    let mut nones = [0usize; QUESTIONS];
    let mut failures: Vec<String> = Vec::new();
    for class in Class::ALL {
        for bucket in Bucket::ALL {
            for index in 0..PER_CELL {
                let inst = generate_instance(MASTER_SEED, class, bucket, index);
                let raw = &inst.raw;
                let Some(bounded) = small_enough(raw) else {
                    skipped += 1;
                    continue;
                };
                unbounded += usize::from(!bounded);
                let df_timeout = (!bounded).then_some(UNBOUNDED_DF_TIMEOUT);
                nets += 1;
                if std::env::var_os("CC_TRACE").is_some() {
                    eprintln!("net {} {:?} {:?} #{index} {}", class.name(), bucket, class, inst.id);
                }
                let label = format!("{} {:?} {:?} #{index} (instance {})", class.name(), bucket, class, inst.id);
                // (a) a fresh PetriNet per question
                let baseline: Vec<String> = (0..QUESTIONS).map(|q| ask(&fresh(raw), q, df_timeout)).collect();
                for (q, a) in baseline.iter().enumerate() {
                    nones[q] += usize::from(a == "None");
                }
                // (b) one PetriNet, several seeded permutations of the question order
                for p in 0..PERMUTATIONS {
                    let mut order: Vec<usize> = (0..QUESTIONS).collect();
                    order.shuffle(&mut ChaCha8Rng::seed_from_u64(MASTER_SEED ^ (u64::from(index) << 8) ^ p));
                    let sys = fresh(raw);
                    let mut got = vec![String::new(); QUESTIONS];
                    for &q in &order {
                        got[q] = ask(&sys, q, df_timeout);
                    }
                    for q in 0..QUESTIONS {
                        if differs(&got[q], &baseline[q], q == 0 && df_timeout.is_some()) {
                            failures.push(report(raw, &format!(
                                "CACHE INCOHERENCE {label}: {} fresh = {} but in order {order:?} = {}", NAMES[q], baseline[q], got[q])));
                        }
                    }
                }
                // (c) Some answers against the explicit-state computation
                if let Some(t) = explicit(raw) {
                    with_truth += 1;
                    let check = |q: usize, expected: String, failures: &mut Vec<String>| {
                        let a = &baseline[q];
                        if a != "None" && *a != format!("Some({expected})") {
                            failures.push(report(raw, &format!("WRONG ANSWER {label}: {} = {a}, explicit-state says {expected}", NAMES[q])));
                        }
                    };
                    check(0, t.deadlock_free.to_string(), &mut failures);
                    check(1, "true".into(), &mut failures);
                    check(2, t.live.to_string(), &mut failures);
                    check(3, t.quasi_live.to_string(), &mut failures);
                    check(4, format!("{:?}", t.level), &mut failures);
                    check(5, format!("{:?}", Boundedness::Bounded(t.max_tokens)), &mut failures);
                }
            }
        }
    }
    println!("nets checked: {nets} ({unbounded} unbounded), skipped (coverability graph above {COVER_CAP}): {skipped}, with explicit ground truth: {with_truth}");
    println!("None answers per question (fresh): {:?}", NAMES.iter().zip(nones).collect::<Vec<_>>());
    assert!(nets >= 150, "too few nets were small enough ({nets})");
    assert!(failures.is_empty(), "{} failures; first minimal reproducer (smallest net):\n{}", failures.len(), {
        let mut f = failures.clone();
        f.sort_by_key(String::len);
        f[0].clone()
    });
}
