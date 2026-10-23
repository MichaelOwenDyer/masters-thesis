use petrivet::system::PetriNet;
use petrivet_testkit::corpus::write_family_corpus;
use petrivet_testkit::families::{FAMILIES, FamilySpec, Query, generate, generate_all};
use std::collections::HashSet;

/// Explicit-state answers from petrivet: (deadlock-free, bounded, live, quasi-live).
/// `None` where the net is unbounded (no finite reachability graph).
fn explicit(spec: &FamilySpec, n: usize) -> (Option<bool>, bool, Option<bool>, Option<bool>) {
    let inst = generate(spec, n);
    let sys = PetriNet::from_pnml(&inst.pnml).unwrap();
    let bounded = sys.build_coverability_graph().is_bounded();
    match sys.try_build_reachability_graph() {
        Ok(rg) => {
            assert!(bounded, "{} n={n}: finite RG but coverability says unbounded", spec.name);
            (Some(rg.is_deadlock_free()), true, Some(rg.transition_liveness().is_live()), Some(rg.transition_liveness().is_quasi_live()))
        }
        Err(_) => {
            assert!(!bounded, "{} n={n}: no RG but coverability says bounded", spec.name);
            (None, false, None, None)
        }
    }
}

/// Bounded random-order breadth-first exploration with the testkit's own semantics, used only
/// for unbounded nets: returns (some deadlock seen, set of transitions that fired).
fn truncated(spec: &FamilySpec, n: usize, cap: usize) -> (bool, HashSet<usize>) {
    let raw = generate(spec, n).raw;
    let mut seen = HashSet::from([raw.marking.clone()]);
    let mut queue = std::collections::VecDeque::from([raw.marking.clone()]);
    let (mut dead, mut fired) = (false, HashSet::new());
    while let Some(m) = queue.pop_front() {
        let mut any = false;
        for t in 0..raw.transitions() {
            let mut next = m.clone();
            if raw.fire(&mut next, t) {
                any = true;
                fired.insert(t);
                if seen.len() < cap && seen.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }
        dead |= !any;
    }
    (dead, fired)
}

#[test]
fn declared_verdicts_match_petrivet_explicit_state() {
    for spec in FAMILIES {
        for n in spec.min_n..=6 {
            let (dl, bounded, live, quasi) = explicit(spec, n);
            let inst = generate(spec, n);
            if bounded {
                for k in &inst.known {
                    let got = match k.query {
                        Query::Q1 => dl,
                        Query::Q2 => Some(true),
                        Query::Q3 => live,
                        Query::Q4 => quasi,
                    };
                    assert_eq!(got, Some(k.verdict), "{} n={n} {:?}", spec.name, k.query);
                }
            } else {
                for k in &inst.known {
                    if k.query == Query::Q2 {
                        assert!(!k.verdict, "{} n={n}: unbounded but Q2 declared true", spec.name);
                    }
                }
                // Unbounded: the remaining verdicts are checked on a truncated state space.
                let (dead, fired) = truncated(spec, n, 5_000);
                let declared = |q| inst.known.iter().find(|k| k.query == q).map(|k| k.verdict);
                if declared(Query::Q1) == Some(true) {
                    assert!(!dead, "{} n={n}: deadlock found", spec.name);
                }
                if declared(Query::Q4) == Some(true) {
                    assert_eq!(fired.len(), inst.raw.transitions(), "{} n={n}", spec.name);
                }
            }
        }
    }
}

#[test]
fn every_family_declares_only_q1_to_q4_with_construction_origin() {
    for i in generate_all(1, 4) {
        assert!(!i.known.is_empty());
        let qs: HashSet<_> = i.known.iter().map(|k| k.query).collect();
        assert_eq!(qs.len(), i.known.len());
        assert!(i.known.iter().all(|k| k.origin == "construction"));
    }
}

#[test]
fn detected_class_is_consistent_with_intended() {
    use petrivet::net::class::NetClass as N;
    use petrivet_testkit::{Class, consistent};
    for i in generate_all(1, 5) {
        if i.intended == Class::General {
            assert_eq!(i.detected, N::General, "{} n={}", i.family, i.n);
        } else {
            assert!(consistent(i.intended, i.detected), "{} n={} detected {:?}", i.family, i.n, i.detected);
        }
    }
}

#[test]
fn pnml_round_trips() {
    for i in generate_all(1, 5) {
        let back = PetriNet::from_pnml(&i.pnml).unwrap();
        assert_eq!(back.to_pnml().unwrap(), i.pnml);
        assert_eq!(back.marking().total_tokens(), i.raw.tokens_total());
    }
}

#[test]
fn corpus_is_deterministic_and_manifest_is_valid() {
    let tmp = std::env::temp_dir().join(format!("fam-corpus-{}", std::process::id()));
    let (a, b) = (tmp.join("a"), tmp.join("b"));
    let r = write_family_corpus(&a, &generate_all(1, 5)).unwrap();
    write_family_corpus(&b, &generate_all(1, 5)).unwrap();
    for f in ["manifest.jsonl", "CORPUS.sha256"] {
        assert_eq!(std::fs::read(a.join(f)).unwrap(), std::fs::read(b.join(f)).unwrap());
    }
    let manifest = std::fs::read_to_string(a.join("manifest.jsonl")).unwrap();
    assert_eq!(manifest.lines().count(), r.written);
    assert_eq!(std::fs::read_to_string(a.join("CORPUS.sha256")).unwrap().lines().count(), r.written + 1);
    assert_eq!(r.duplicates_dropped, 0);
    for line in manifest.lines() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(v["source"], "fam");
        for f in ["instance_id", "path", "family", "size_param", "detected_class_petrivet", "places", "transitions", "arcs", "tokens_total", "known_answers"] {
            assert!(!v[f].is_null(), "missing {f}");
        }
        let id = v["instance_id"].as_str().unwrap();
        let bytes = std::fs::read(a.join(v["path"].as_str().unwrap())).unwrap();
        assert_eq!(blake3::hash(&bytes).to_hex().as_str(), id);
        for k in v["known_answers"].as_array().unwrap() {
            assert!(["Q1", "Q2", "Q3", "Q4"].contains(&k["query"].as_str().unwrap()));
            assert!(k["verdict"].is_boolean());
            assert_eq!(k["origin"], "construction");
        }
    }
    std::fs::remove_dir_all(tmp).ok();
}

fn spec(name: &str) -> &'static FamilySpec {
    FAMILIES.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("no family {name}"))
}

fn verdict(spec: &FamilySpec, n: usize, q: Query) -> Option<bool> {
    generate(spec, n).known.iter().find(|k| k.query == q).map(|k| k.verdict)
}

/// Full (untruncated) exploration of a bounded family: (some reachable deadlock, fired transitions).
fn full(spec: &FamilySpec, n: usize) -> (bool, HashSet<usize>) {
    let (dead, fired) = truncated(spec, n, 2_000_000);
    (dead, fired)
}

#[test]
fn every_false_q1_and_q4_has_an_explicit_counterexample() {
    for spec in FAMILIES {
        for n in spec.min_n..=6 {
            let inst = generate(spec, n);
            if verdict(spec, n, Query::Q2) != Some(true) {
                continue;
            }
            let (dead, fired) = full(spec, n);
            if verdict(spec, n, Query::Q1) == Some(false) {
                assert!(dead, "{} n={n}: Q1 false but no reachable deadlock", spec.name);
            }
            if verdict(spec, n, Query::Q4) == Some(false) {
                assert!(fired.len() < inst.raw.transitions(), "{} n={n}: Q4 false but all transitions fire", spec.name);
            }
            // Q3 false without a dead transition must come from a reachable deadlock.
            if verdict(spec, n, Query::Q3) == Some(false) && fired.len() == inst.raw.transitions() {
                assert!(dead || spec.name.starts_with("sm_two_cycles"), "{} n={n}: Q3 false without witness", spec.name);
            }
        }
    }
}

#[test]
fn new_families_have_their_expected_witnesses() {
    for n in 4..=6 {
        // sm_ring_with_sink: every token ends in the sink; a deadlock is reachable, all else fires.
        let s = spec("sm_ring_with_sink");
        let (dead, fired) = full(s, n);
        assert!(dead && fired.len() == generate(s, n).raw.transitions());
        // mg_buffer_ring_unmarked_circuit: t_k, t_{k+1} (k = n/2) are dead, everything ends in deadlock.
        let s = spec("mg_buffer_ring_unmarked_circuit");
        let (dead, fired) = full(s, n);
        let k = n / 2;
        assert!(dead && !fired.contains(&k) && !fired.contains(&((k + 1) % n)));
        let raw = generate(s, n).raw;
        assert_eq!(raw.marking[n + k], 0);
        assert_eq!(raw.marking[k], 0);
        // every maximal run ends in a deadlock: the reachability graph is finite and acyclic.
        assert!(acyclic(&raw), "{} n={n}: reachable cycle", s.name);
        let sys = PetriNet::from_pnml(&generate(s, n).pnml).unwrap();
        let rg = sys.try_build_reachability_graph().unwrap();
        assert!(!rg.is_deadlock_free());
        assert!(!rg.transition_liveness().is_quasi_live());
        // efc_forkjoin_leaky: deadlock reachable with all tokens on x_0.
        let s = spec("efc_forkjoin_leaky");
        let (dead, fired) = full(s, n);
        assert!(dead && fired.len() == generate(s, n).raw.transitions());
        // ac_cycle_no_low_token: no t_low (transitions n..2n) ever fires, no deadlock.
        let s = spec("ac_cycle_no_low_token");
        let (dead, fired) = full(s, n);
        assert!(!dead && (0..n).all(|t| fired.contains(&t)) && (n..2 * n).all(|t| !fired.contains(&t)));
    }
}

#[test]
fn glossary_names_are_stable_and_complete() {
    let names: Vec<_> = FAMILIES.iter().map(|f| f.name).collect();
    assert_eq!(
        names,
        [
            "circuit", "sm_hub_and_spokes", "sm_two_cycles", "sm_two_cycles_n_tokens", "sm_choice_ring",
            "sm_ring_with_sink", "mg_buffer_ring", "mg_buffer_ring_unmarked_circuit", "mg_forkjoin_cycle",
            "mg_forkjoin_unbounded", "efc_forkjoin", "efc_forkjoin_leaky", "ac_cycle", "ac_cycle_no_low_token",
            "general_cycle"
        ]
    );
    assert_eq!(petrivet_testkit::families::FAM_VERSION, "fam-2");
    assert_eq!(spec("sm_choice_ring").intended, petrivet_testkit::Class::StateMachine);
}

/// True iff the (finite) reachability graph has no cycle (Kahn's algorithm).
fn acyclic(raw: &petrivet_testkit::raw::RawNet) -> bool {
    use std::collections::{HashMap, VecDeque};
    let mut succ: HashMap<Vec<u32>, Vec<Vec<u32>>> = HashMap::new();
    let mut queue = VecDeque::from([raw.marking.clone()]);
    succ.insert(raw.marking.clone(), vec![]);
    while let Some(m) = queue.pop_front() {
        for t in 0..raw.transitions() {
            let mut next = m.clone();
            if raw.fire(&mut next, t) {
                succ.get_mut(&m).unwrap().push(next.clone());
                if !succ.contains_key(&next) {
                    succ.insert(next.clone(), vec![]);
                    queue.push_back(next);
                }
            }
        }
    }
    let mut indeg: HashMap<&Vec<u32>, usize> = succ.keys().map(|k| (k, 0)).collect();
    for v in succ.values().flatten() {
        *indeg.get_mut(v).unwrap() += 1;
    }
    let mut ready: Vec<&Vec<u32>> = indeg.iter().filter(|(_, d)| **d == 0).map(|(k, _)| *k).collect();
    let mut seen = 0;
    while let Some(m) = ready.pop() {
        seen += 1;
        for v in &succ[m] {
            let d = indeg.get_mut(v).unwrap();
            *d -= 1;
            if *d == 0 {
                ready.push(v);
            }
        }
    }
    seen == succ.len()
}
