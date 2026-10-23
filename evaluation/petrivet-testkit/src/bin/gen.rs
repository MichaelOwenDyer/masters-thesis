//! `gen --master-seed N --per-cell K --out DIR`: writes the `gen` corpus and prints statistics.
//! `gen --families --out DIR [--min-exp A] [--max-exp B]`: writes the `fam` corpus (n = 2^A ..= 2^B).

use petrivet_testkit::corpus::write_corpus;
use petrivet_testkit::{Bucket, Class, Instance, Regime, consistent, detected_name, generate_instance};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    if args.iter().any(|a| a == "--families") {
        return families_main(&get);
    }
    let (Some(seed), Some(k), Some(out)) = (get("--master-seed"), get("--per-cell"), get("--out")) else {
        eprintln!("usage: gen --master-seed N --per-cell K --out DIR");
        std::process::exit(2);
    };
    let (seed, k, out): (u64, u32, PathBuf) = (seed.parse().expect("seed"), k.parse().expect("per-cell"), out.into());

    let mut all: Vec<Instance> = Vec::new();
    println!("class bucket   accepted unique  rej%   |P|   |T| arcs  sparse/medium/dense");
    let mut confusion: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for class in Class::ALL {
        for bucket in Bucket::ALL {
            let cell: Vec<Instance> = (0..k).map(|i| generate_instance(seed, class, bucket, i)).collect();
            let n = cell.len() as f64;
            let attempts: u32 = cell.iter().map(|i| i.attempts).sum();
            let mean = |f: fn(&Instance) -> usize| cell.iter().map(&f).sum::<usize>() as f64 / n;
            let regimes: Vec<usize> = Regime::ALL.iter().map(|&r| cell.iter().filter(|i| i.regime == r).count()).collect();
            let unique = cell.iter().map(|i| &i.id).collect::<std::collections::HashSet<_>>().len();
            println!(
                "{:<7} {:<6} {:>8} {:>6} {:>5.1} {:>5.1} {:>5.1} {:>5.1}  {}/{}/{}",
                class.name(), bucket.name(), cell.len(), unique,
                100.0 * f64::from(attempts - cell.len() as u32) / f64::from(attempts),
                mean(|i| i.raw.places), mean(|i| i.raw.transitions()), mean(|i| i.raw.arcs()),
                regimes[0], regimes[1], regimes[2],
            );
            for i in &cell {
                *confusion.entry((class.name(), detected_name(i.detected))).or_default() += 1;
            }
            all.extend(cell);
        }
    }
    println!("\nintended -> petrivet detected (counts over all buckets); '!' marks inconsistent pairs");
    for ((intended, detected), n) in &confusion {
        let class = Class::ALL.iter().find(|c| c.name() == *intended).unwrap();
        let ok = all.iter().filter(|i| i.class == *class && detected_name(i.detected) == *detected).all(|i| consistent(*class, i.detected));
        println!("{intended:<8} {detected:<8} {n:>6} {}", if ok { "" } else { "!" });
    }
    let mismatches = all.iter().filter(|i| !consistent(i.class, i.detected)).count();
    let report = write_corpus(&out, &all).expect("write corpus");
    println!("\ninconsistent instances: {mismatches}");
    println!("wrote {} PNML files ({} duplicate PNML dropped) to {}", report.written, report.duplicates_dropped, out.display());
}

fn families_main(get: &dyn Fn(&str) -> Option<String>) {
    use petrivet_testkit::corpus::write_family_corpus;
    use petrivet_testkit::families::{DEFAULT_MAX_EXP, DEFAULT_MIN_EXP, detected, generate_all};
    let Some(out) = get("--out") else {
        eprintln!("usage: gen --families --out DIR [--min-exp A] [--max-exp B]");
        std::process::exit(2);
    };
    let exp = |flag: &str, default: u32| get(flag).map_or(default, |v| v.parse().expect("exponent"));
    let (lo, hi) = (exp("--min-exp", DEFAULT_MIN_EXP), exp("--max-exp", DEFAULT_MAX_EXP));
    let all = generate_all(lo, hi);
    println!("family            n   places transitions  tokens detected  declared verdicts");
    for i in &all {
        let verdicts: Vec<String> =
            i.known.iter().map(|k| format!("{:?}={}", k.query, if k.verdict { "T" } else { "F" })).collect();
        println!(
            "{:<16} {:>5} {:>7} {:>11} {:>7} {:<9} {}",
            i.family, i.n, i.raw.places, i.raw.transitions(), i.raw.tokens_total(), detected(i), verdicts.join(" ")
        );
    }
    let report = write_family_corpus(&PathBuf::from(&out), &all).expect("write corpus");
    println!("wrote {} PNML files ({} duplicate PNML dropped) to {out}", report.written, report.duplicates_dropped);
}
