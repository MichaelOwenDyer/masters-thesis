//! Corpus writer: `<instance_id>.pnml`, `manifest.jsonl`, `CORPUS.sha256` (sha256sum format).
//! Output depends only on the instances (no timestamps, input order preserved).
//! Instances whose PNML bytes coincide (common for tiny circuits) are written once; the first wins.

use crate::{GEN_VERSION, Instance, detected_name};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::{fs, io, path::Path};

#[derive(Serialize)]
struct TargetRecord {
    target_id: String,
    kind: &'static str,
    marking: Vec<u32>,
    /// Proposer's expectation, not a label (labels live in truth.jsonl).
    expectation: &'static str,
    firing_sequence: Option<Vec<usize>>,
}

#[derive(Serialize)]
struct Record {
    instance_id: String,
    path: String,
    source: &'static str,
    generator_class: &'static str,
    detected_class_petrivet: &'static str,
    places: usize,
    transitions: usize,
    arcs: usize,
    tokens_total: u32,
    max_weight: u32,
    marking_regime: &'static str,
    size_bucket: &'static str,
    seed: String,
    attempts: u32,
    gen_version: &'static str,
    initial_marking: Vec<u32>,
    targets: Vec<TargetRecord>,
}

pub struct WriteReport {
    pub written: usize,
    pub duplicates_dropped: usize,
}

fn record(i: &Instance) -> Record {
    let targets = i
        .targets
        .iter()
        .map(|t| {
            let mut h = blake3::Hasher::new();
            h.update(i.id.as_bytes()).update(t.kind.name().as_bytes());
            t.marking.iter().for_each(|k| { h.update(&k.to_le_bytes()); });
            TargetRecord {
                target_id: h.finalize().to_hex()[..16].to_string(),
                kind: t.kind.name(),
                marking: t.marking.clone(),
                expectation: if t.expect_yes { "yes" } else { "no" },
                firing_sequence: t.firing_sequence.clone(),
            }
        })
        .collect();
    Record {
        instance_id: i.id.clone(),
        path: format!("{}.pnml", i.id),
        source: "gen",
        generator_class: i.class.name(),
        detected_class_petrivet: detected_name(i.detected),
        places: i.raw.places,
        transitions: i.raw.transitions(),
        arcs: i.raw.arcs(),
        tokens_total: i.raw.tokens_total(),
        max_weight: 1,
        marking_regime: i.regime.name(),
        size_bucket: i.bucket.name(),
        seed: i.seed.iter().map(|b| format!("{b:02x}")).collect(),
        attempts: i.attempts,
        gen_version: GEN_VERSION,
        initial_marking: i.raw.marking.clone(),
        targets,
    }
}

pub fn write_corpus(dir: &Path, instances: &[Instance]) -> io::Result<WriteReport> {
    fs::create_dir_all(dir)?;
    let mut seen = HashSet::new();
    let mut manifest = String::new();
    let mut hashes: Vec<(String, String)> = Vec::new();
    for i in instances.iter().filter(|i| seen.insert(i.id.clone())) {
        let name = format!("{}.pnml", i.id);
        fs::write(dir.join(&name), &i.pnml)?;
        hashes.push((name, hex(&Sha256::digest(i.pnml.as_bytes()))));
        manifest.push_str(&serde_json::to_string(&record(i)).map_err(io::Error::other)?);
        manifest.push('\n');
    }
    fs::write(dir.join("manifest.jsonl"), &manifest)?;
    hashes.push(("manifest.jsonl".into(), hex(&Sha256::digest(manifest.as_bytes()))));
    hashes.sort();
    let listing: String = hashes.iter().map(|(n, h)| format!("{h}  {n}\n")).collect();
    fs::write(dir.join("CORPUS.sha256"), listing)?;
    Ok(WriteReport { written: seen.len(), duplicates_dropped: instances.len() - seen.len() })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
