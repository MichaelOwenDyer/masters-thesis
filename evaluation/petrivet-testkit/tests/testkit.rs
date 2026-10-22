use petrivet::system::PetriNet;
use petrivet_testkit::classes::{has_no_source_or_sink, satisfies};
use petrivet_testkit::corpus::write_corpus;
use petrivet_testkit::targets::Kind;
use petrivet_testkit::{Bucket, Class, Instance, consistent, generate_instance};

const SEEDS: u32 = 150;

fn all_instances(master: u64, n: u32) -> Vec<Instance> {
    let mut v = Vec::new();
    for class in Class::ALL {
        for bucket in Bucket::ALL {
            v.extend((0..n).map(|i| generate_instance(master, class, bucket, i)));
        }
    }
    v
}

#[test]
fn construction_assertions_and_shape() {
    for i in all_instances(1, SEEDS) {
        let (lo, hi) = i.bucket.places();
        let n = &i.raw;
        assert!((lo..=hi).contains(&n.places));
        assert!(n.transitions() >= n.places.div_ceil(2) && n.transitions() <= 2 * n.places);
        if i.class == Class::Circuit {
            assert_eq!(n.places, n.transitions());
        }
        assert!(satisfies(i.class, n), "{:?}", i.class);
        assert!(has_no_source_or_sink(n));
        assert!(n.is_connected());
        assert!(n.marking.iter().all(|&k| k <= 2));
        assert!((1..=6).contains(&n.tokens_total()));
    }
}

/// A mismatch for a non-general intended class is a generator bug or a petrivet bug.
#[test]
fn petrivet_detected_class_is_consistent_with_intended() {
    for i in all_instances(2, SEEDS) {
        assert!(consistent(i.class, i.detected), "{:?} detected as {:?}\n{}", i.class, i.detected, i.pnml);
    }
}

#[test]
fn single_index_regenerates_without_earlier_indices() {
    let alone = generate_instance(7, Class::ExtendedFreeChoice, Bucket::Medium, 37);
    let after: Vec<_> = (0..38).map(|i| generate_instance(7, Class::ExtendedFreeChoice, Bucket::Medium, i)).collect();
    assert_eq!(alone.pnml, after[37].pnml);
    assert_eq!(alone.seed, after[37].seed);
    assert_eq!(alone.targets, after[37].targets);
    assert_ne!(after[36].pnml, after[37].pnml);
}

/// Pins the generator output. If this fails after an intentional change, bump GEN_VERSION and update the hashes.
#[test]
fn golden_pnml_hashes() {
    let cases = [
        (42, Class::General, Bucket::Small, 0, "62a2a448cf31007baeecbd0f863fba230f100a75342ece1765be2308bd670f23"),
        (42, Class::StateMachine, Bucket::Medium, 5, "1ad07447a5b8c856e463d4f5947057c9d705fc5056be66a4af255036da3e29fb"),
        (42, Class::MarkedGraph, Bucket::Large, 9, "a0a22ff28f90fc9a8e48a820bace6304168f739cdb46f76006c52366e059afd1"),
        (42, Class::ExtendedFreeChoice, Bucket::Large, 3, "ab8e1ca9854c698583bc5e2e49e873cfde627283a30b7a5f5c9e1507408365ef"),
        (42, Class::AsymmetricChoice, Bucket::Medium, 11, "d0ab98cebcee4b2ef77059f44a2c3cfa8038f68589b8b9f5e6d1b57db634bced"),
        (42, Class::Circuit, Bucket::Small, 1, "648917e67c8f14d748e12b578b009e12fe4abf101d303ccb32817df555251c76"),
    ];
    for (m, c, b, idx, want) in cases {
        let i = generate_instance(m, c, b, idx);
        assert_eq!(blake3::hash(i.pnml.as_bytes()).to_hex().as_str(), want, "{c:?} {b:?} {idx}");
    }
}

#[test]
fn pnml_round_trips_through_petrivet_importer() {
    for i in all_instances(3, 20) {
        let back = PetriNet::from_pnml(&i.pnml).expect("petrivet must parse its own export");
        assert_eq!(back.class(), i.detected);
        assert_eq!(back.place_count() as usize, i.raw.places);
        assert_eq!(back.transition_count() as usize, i.raw.transitions());
        assert_eq!(back.arc_count(), i.raw.arcs());
        assert_eq!(back.marking().total_tokens(), i.raw.tokens_total());
        assert_eq!(back.to_pnml().unwrap(), i.pnml, "re-export differs");
    }
}

#[test]
fn pnml_ids_follow_raw_indices() {
    // Marking of place i must appear on PNML id p{i}.
    for i in all_instances(4, 10) {
        for (p, &k) in i.raw.marking.iter().enumerate() {
            let tag = format!("<place id=\"p{p}\"");
            let at = i.pnml.find(&tag).expect("place id present");
            let rest = &i.pnml[at + 1..];
            let end = at + 1 + rest.find("<place id").unwrap_or(usize::MAX).min(rest.find("<transition").unwrap());
            assert_eq!(i.pnml[at..end].contains("<initialMarking>"), k > 0, "{tag}");
        }
    }
}

#[test]
fn targets_replay_and_respect_limits() {
    let mut seen_kinds = std::collections::HashSet::new();
    for i in all_instances(5, 40) {
        for kind in [Kind::Reachability, Kind::Coverability] {
            for yes in [true, false] {
                let n = i.targets.iter().filter(|t| t.kind == kind && t.expect_yes == yes).count();
                assert!(n <= 2);
            }
        }
        for t in &i.targets {
            seen_kinds.insert((t.kind, t.expect_yes));
            assert_eq!(t.marking.len(), i.raw.places);
            if let (true, Some(seq)) = (t.expect_yes, &t.firing_sequence) {
                let m = i.raw.replay(seq).expect("sequence must be fireable");
                match t.kind {
                    Kind::Reachability => assert_eq!(m, t.marking),
                    Kind::Coverability => assert!(m.iter().zip(&t.marking).all(|(a, b)| a >= b)),
                }
            } else {
                assert!(!t.expect_yes || t.firing_sequence.is_some(), "yes targets come from walks");
            }
        }
    }
    assert_eq!(seen_kinds.len(), 4);
}

#[test]
fn corpus_is_reproducible_and_checksummed() {
    let tmp = std::env::temp_dir().join(format!("testkit-corpus-{}", std::process::id()));
    let (a, b) = (tmp.join("a"), tmp.join("b"));
    let r = write_corpus(&a, &all_instances(6, 5)).unwrap();
    write_corpus(&b, &all_instances(6, 5)).unwrap();
    for f in ["manifest.jsonl", "CORPUS.sha256"] {
        assert_eq!(std::fs::read(a.join(f)).unwrap(), std::fs::read(b.join(f)).unwrap());
    }
    let sums = std::fs::read_to_string(a.join("CORPUS.sha256")).unwrap();
    assert_eq!(sums.lines().count(), r.written + 1);
    let manifest = std::fs::read_to_string(a.join("manifest.jsonl")).unwrap();
    assert_eq!(manifest.lines().count(), r.written);
    let first: serde_json::Value = serde_json::from_str(manifest.lines().next().unwrap()).unwrap();
    assert_eq!(first["source"], "gen");
    std::fs::remove_dir_all(tmp).ok();
}
