//! Instance manifests for the sources whose truth is known without tools: `mcc-small` / `mcc-large`
//! (verdicts of the MCC oracle) and `calibration` (hand-derived answers).
//!
//! One JSON line per instance, sorted by `instance_id`, no timestamps. The shape follows SPEC 5.6 and the
//! testkit's `manifest.jsonl`; `known_answers` is the addition that the `truth` stage reads.
//!
//! `known_answers` entries are `{query, verdict, origin}` (plus `target_id` for QR and QC):
//! `verdict` is a Boolean, or for Q0 an object `{STATES, TRANSITIONS, MAX_TOKEN_IN_PLACE?, MAX_TOKEN_PER_MARKING?}`.
//! MCC mapping: SS -> Q0, RD -> Q1 with inverted polarity (oracle TRUE means a deadlock exists, so
//! deadlock-free is the negation), L -> Q3, QL -> Q4, OS -> Q5, SM -> Q6. `?`, absent and `+inf` values are omitted, never guessed.

use crate::config::Config;
use crate::fetch::Selected;
use crate::histogram::max_inscription;
use crate::oracle::{Exam, Verdict};
use petrivet::net::class::NetClass;
use petrivet::system::PetriNet;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub const ORIGIN_MCC: &str = "mcc-oracle";
pub const ORIGIN_HAND: &str = "hand";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownAnswer {
    pub query: String,
    pub verdict: Value,
    pub origin: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TargetRecord {
    pub target_id: String,
    pub kind: String,
    pub marking: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_file: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Record {
    pub instance_id: String,
    pub path: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcc_model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcc_family: Option<String>,
    /// null when petrivet rejects the file (counted by the caller)
    pub detected_class_petrivet: Option<&'static str>,
    pub places: Option<u64>,
    pub transitions: Option<u64>,
    pub arcs: Option<u64>,
    pub tokens_total: Option<u64>,
    pub max_weight: u64,
    pub known_answers: Vec<KnownAnswer>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<TargetRecord>,
}

pub fn class_name(c: NetClass) -> &'static str {
    match c {
        NetClass::Circuit => "circuit",
        NetClass::StateMachine => "sm",
        NetClass::MarkedGraph => "mg",
        NetClass::FreeChoice => "efc",
        NetClass::AsymmetricChoice => "ac",
        NetClass::General => "general",
    }
}

pub fn hex_blake3(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[derive(Debug, Clone, Copy)]
pub struct Structure {
    pub class: &'static str,
    pub places: u64,
    pub transitions: u64,
    pub arcs: u64,
    pub tokens: u64,
}

/// Parses with petrivet in a thread with a large stack; an error or a panic is returned, never fatal.
pub fn parse_structure(xml: String) -> Result<Structure, String> {
    let h = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let sys = PetriNet::from_pnml(&xml).map_err(|e| e.to_string())?;
            Ok::<_, String>(Structure {
                class: class_name(sys.class()),
                places: u64::from(sys.place_count()),
                transitions: u64::from(sys.transition_count()),
                arcs: sys.arc_count() as u64,
                tokens: u64::from(sys.marking().total_tokens()),
            })
        })
        .map_err(|e| e.to_string())?;
    h.join().unwrap_or_else(|_| Err("parser panic".into()))
}

/// State-space values of an MCC StateSpace file; a field is `None` for `?`, `+inf` or when absent.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SsValues {
    pub states: Option<u64>,
    pub transitions: Option<u64>,
    pub max_token_in_place: Option<u64>,
    pub max_token_per_marking: Option<u64>,
}

pub fn parse_ss(content: &str) -> SsValues {
    let mut v = SsValues::default();
    for line in content.lines().skip(1) {
        let mut it = line.split_whitespace();
        if it.next() != Some("STATE_SPACE") {
            continue;
        }
        let (Some(key), Some(val)) = (it.next(), it.next()) else { continue };
        let n = val.parse::<u64>().ok();
        match key {
            "STATES" => v.states = n,
            "TRANSITIONS" => v.transitions = n,
            "MAX_TOKEN_IN_PLACE" => v.max_token_in_place = n,
            "MAX_TOKEN_PER_MARKING" => v.max_token_per_marking = n,
            _ => {}
        }
    }
    v
}

fn answer(query: &str, verdict: Value, origin: &str) -> KnownAnswer {
    KnownAnswer { query: query.into(), verdict, origin: origin.into(), target_id: None }
}

/// Known answers of one MCC model. Pure: no I/O.
pub fn mcc_known_answers(verdicts: &BTreeMap<Exam, Verdict>, ss: Option<&SsValues>) -> Vec<KnownAnswer> {
    let mut out = Vec::new();
    if let Some(ss) = ss
        && let (Some(states), Some(transitions)) = (ss.states, ss.transitions)
    {
        let mut o = Map::new();
        o.insert("STATES".into(), json!(states));
        o.insert("TRANSITIONS".into(), json!(transitions));
        if let Some(x) = ss.max_token_in_place {
            o.insert("MAX_TOKEN_IN_PLACE".into(), json!(x));
        }
        if let Some(x) = ss.max_token_per_marking {
            o.insert("MAX_TOKEN_PER_MARKING".into(), json!(x));
        }
        out.push(answer("Q0", Value::Object(o), ORIGIN_MCC));
    }
    // (examination, query, invert)
    for (exam, query, invert) in [
        (Exam::RD, "Q1", true),
        (Exam::L, "Q3", false),
        (Exam::QL, "Q4", false),
        (Exam::OS, "Q5", false),
        (Exam::SM, "Q6", false),
    ] {
        let b = match verdicts.get(&exam) {
            Some(Verdict::True) => true,
            Some(Verdict::False) => false,
            Some(Verdict::Unknown) | None => continue,
        };
        out.push(answer(query, json!(b != invert), ORIGIN_MCC));
    }
    out
}

fn to_lines(records: &[Record]) -> Result<String, String> {
    let mut s = String::new();
    for r in records {
        s.push_str(&serde_json::to_string(r).map_err(|e| e.to_string())?);
        s.push('\n');
    }
    Ok(s)
}

fn write_text(path: &Path, text: &str) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Sorts by instance_id and drops later records with an id already seen (identical PNML bytes). Returns the number dropped.
fn sort_dedup(records: &mut Vec<Record>) -> usize {
    records.sort_by(|a, b| a.instance_id.cmp(&b.instance_id).then_with(|| a.path.cmp(&b.path)));
    let n = records.len();
    records.dedup_by(|b, a| a.instance_id == b.instance_id);
    n - records.len()
}

pub fn run_mcc(cfg: &Config, out: Option<&Path>) -> Result<(), String> {
    let selected: Selected = serde_json::from_str(
        &std::fs::read_to_string(cfg.selected_path()).map_err(|e| format!("{} (run `fetch` first): {e}", cfg.selected_path().display()))?,
    )
    .map_err(|e| e.to_string())?;
    std::panic::set_hook(Box::new(|_| {}));
    let mut records = Vec::new();
    let (mut parse_failures, mut without_ss) = (Vec::new(), 0usize);
    for m in &selected.models {
        let rel = format!("models/{}/model.pnml", m.id);
        let path = cfg.paths.corpus_dir.join(&rel);
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let source = match m.stratum.split(':').next() {
            Some("small") => "mcc-small",
            Some("large") => "mcc-large",
            other => return Err(format!("{}: unknown stratum {other:?}", m.id)),
        };
        let ss_path = cfg.paths.oracle_dir.join(&m.id).join(format!("{}-SS.out", m.id));
        let ss = std::fs::read_to_string(&ss_path).ok().map(|c| parse_ss(&c));
        let known_answers = mcc_known_answers(&m.verdicts, ss.as_ref());
        if !known_answers.iter().any(|k| k.query == "Q0") {
            without_ss += 1;
        }
        let xml = String::from_utf8_lossy(&bytes).into_owned();
        let max_weight = max_inscription(&xml);
        let st = match parse_structure(xml) {
            Ok(s) => Some(s),
            Err(e) => {
                parse_failures.push(format!("{}: {e}", m.id));
                None
            }
        };
        records.push(Record {
            instance_id: hex_blake3(&bytes),
            path: rel,
            source: source.into(),
            mcc_model_id: Some(m.id.clone()),
            mcc_family: Some(m.family.clone()),
            detected_class_petrivet: st.map(|s| s.class),
            places: st.map(|s| s.places),
            transitions: st.map(|s| s.transitions),
            arcs: st.map(|s| s.arcs),
            tokens_total: st.map(|s| s.tokens),
            max_weight,
            known_answers,
            targets: vec![],
        });
    }
    let _ = std::panic::take_hook();
    let dups = sort_dedup(&mut records);
    let default = cfg.paths.corpus_dir.join("manifest.jsonl");
    let out = out.unwrap_or(&default);
    write_text(out, &to_lines(&records)?)?;

    println!("wrote {} ({} instances)", out.display(), records.len());
    for s in ["mcc-small", "mcc-large"] {
        let rs: Vec<_> = records.iter().filter(|r| r.source == s).collect();
        let mut by_q: BTreeMap<&str, usize> = BTreeMap::new();
        for k in rs.iter().flat_map(|r| &r.known_answers) {
            *by_q.entry(k.query.as_str()).or_default() += 1;
        }
        println!("  {s}: {} instances, known answers per query {by_q:?}", rs.len());
    }
    println!("  models without a usable StateSpace pair (Q0 omitted): {without_ss}");
    println!("  duplicate PNML bytes dropped: {dups}; petrivet parse failures: {}", parse_failures.len());
    for f in &parse_failures {
        println!("    parse failure {f}");
    }
    Ok(())
}

/// Calibration manifest from `<dir>/answers.json` and the `cal-*.pnml` files next to it.
/// `answers.json`: net -> {Q0: {states, transitions}|null, Q1..Q6: bool|"n/a", targets: [{kind, marking, expected, why}]}.
pub fn run_calibration(dir: &Path, out: Option<&Path>) -> Result<(), String> {
    let text = std::fs::read_to_string(dir.join("answers.json")).map_err(|e| format!("{}/answers.json: {e}", dir.display()))?;
    let answers: BTreeMap<String, Value> = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    std::panic::set_hook(Box::new(|_| {}));
    let mut records = Vec::new();
    for (name, a) in answers.iter().filter(|(k, _)| !k.starts_with('_')) {
        let rel = format!("{name}.pnml");
        let bytes = std::fs::read(dir.join(&rel)).map_err(|e| format!("{rel}: {e}"))?;
        let id = hex_blake3(&bytes);
        let st = parse_structure(String::from_utf8_lossy(&bytes).into_owned()).map_err(|e| format!("{rel}: {e}"))?;
        let (known_answers, targets) = calibration_answers(name, &id, a, st.places as usize)?;
        records.push(Record {
            instance_id: id,
            path: rel,
            source: "calibration".into(),
            mcc_model_id: None,
            mcc_family: None,
            detected_class_petrivet: Some(st.class),
            places: Some(st.places),
            transitions: Some(st.transitions),
            arcs: Some(st.arcs),
            tokens_total: Some(st.tokens),
            max_weight: 1,
            known_answers,
            targets,
        });
    }
    let _ = std::panic::take_hook();
    let dups = sort_dedup(&mut records);
    if dups > 0 {
        return Err(format!("{dups} calibration nets have identical PNML bytes"));
    }
    let default = dir.join("manifest.jsonl");
    let out = out.unwrap_or(&default);
    write_text(out, &to_lines(&records)?)?;
    println!("wrote {} ({} calibration instances)", out.display(), records.len());
    Ok(())
}

/// Target id as in the testkit: first 16 hex characters of blake3(instance_id, kind, markings as u32 little endian).
pub fn target_id(instance_id: &str, kind: &str, marking: &[u32]) -> String {
    let mut h = blake3::Hasher::new();
    h.update(instance_id.as_bytes()).update(kind.as_bytes());
    marking.iter().for_each(|k| {
        h.update(&k.to_le_bytes());
    });
    h.finalize().to_hex()[..16].to_string()
}

fn calibration_answers(name: &str, id: &str, a: &Value, places: usize) -> Result<(Vec<KnownAnswer>, Vec<TargetRecord>), String> {
    let mut known = Vec::new();
    match &a["Q0"] {
        Value::Null => {}
        o => {
            let (s, t) = (o["states"].as_u64(), o["transitions"].as_u64());
            let (Some(s), Some(t)) = (s, t) else { return Err(format!("{name}: Q0 needs states and transitions")) };
            known.push(answer("Q0", json!({"STATES": s, "TRANSITIONS": t}), ORIGIN_HAND));
        }
    }
    for q in ["Q1", "Q2", "Q3", "Q4", "Q5", "Q6"] {
        match &a[q] {
            Value::Bool(b) => known.push(answer(q, json!(b), ORIGIN_HAND)),
            Value::String(s) if s == "n/a" => {}
            other => return Err(format!("{name}: {q} is {other}, expected bool or \"n/a\"")),
        }
    }
    let mut targets = Vec::new();
    for (k, t) in a["targets"].as_array().map(Vec::as_slice).unwrap_or(&[]).iter().enumerate() {
        let kind = t["kind"].as_str().filter(|k| *k == "QR" || *k == "QC").ok_or_else(|| format!("{name}: target {k}: kind must be QR or QC"))?;
        let marking: Vec<u32> = t["marking"]
            .as_array()
            .ok_or_else(|| format!("{name}: target {k}: marking missing"))?
            .iter()
            .map(|x| x.as_u64().map(|x| x as u32).ok_or_else(|| format!("{name}: target {k}: bad token count")))
            .collect::<Result<_, _>>()?;
        if marking.len() != places {
            return Err(format!("{name}: target {k}: {} entries for {places} places", marking.len()));
        }
        let expected = t["expected"].as_bool().ok_or_else(|| format!("{name}: target {k}: expected must be bool"))?;
        let tid = target_id(id, kind, &marking);
        let mut ka = answer(kind, json!(expected), ORIGIN_HAND);
        ka.target_id = Some(tid.clone());
        known.push(ka);
        targets.push(TargetRecord { target_id: tid, kind: kind.into(), marking, target_file: Some(format!("{name}.target-{}.json", k + 1)) });
    }
    Ok((known, targets))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verdicts(v: &[(Exam, Verdict)]) -> BTreeMap<Exam, Verdict> {
        v.iter().copied().collect()
    }

    fn q<'a>(k: &'a [KnownAnswer], name: &str) -> Option<&'a Value> {
        k.iter().find(|a| a.query == name).map(|a| &a.verdict)
    }

    #[test]
    fn reachability_deadlock_is_inverted() {
        let k = mcc_known_answers(&verdicts(&[(Exam::RD, Verdict::True)]), None);
        assert_eq!(q(&k, "Q1"), Some(&json!(false)), "RD TRUE means a deadlock exists, so deadlock-free is false");
        let k = mcc_known_answers(&verdicts(&[(Exam::RD, Verdict::False)]), None);
        assert_eq!(q(&k, "Q1"), Some(&json!(true)));
    }

    #[test]
    fn other_examinations_keep_polarity() {
        let v = verdicts(&[
            (Exam::L, Verdict::True),
            (Exam::QL, Verdict::False),
            (Exam::OS, Verdict::True),
            (Exam::SM, Verdict::False),
        ]);
        let k = mcc_known_answers(&v, None);
        assert_eq!(q(&k, "Q3"), Some(&json!(true)));
        assert_eq!(q(&k, "Q4"), Some(&json!(false)));
        assert_eq!(q(&k, "Q5"), Some(&json!(true)));
        assert_eq!(q(&k, "Q6"), Some(&json!(false)));
        assert!(k.iter().all(|a| a.origin == ORIGIN_MCC && a.target_id.is_none()));
    }

    #[test]
    fn unknown_and_missing_are_omitted() {
        let v = verdicts(&[(Exam::L, Verdict::Unknown), (Exam::RD, Verdict::Unknown), (Exam::OS, Verdict::True)]);
        let k = mcc_known_answers(&v, None);
        assert_eq!(k.iter().map(|a| a.query.as_str()).collect::<Vec<_>>(), vec!["Q5"]);
        assert!(mcc_known_answers(&BTreeMap::new(), None).is_empty());
    }

    #[test]
    fn state_space_object_and_omissions() {
        let c = "A StateSpace\nSTATE_SPACE STATES 29 TECHNIQUES X\nSTATE_SPACE TRANSITIONS 71 TECHNIQUES X\nSTATE_SPACE MAX_TOKEN_IN_PLACE 3 TECHNIQUES X\nSTATE_SPACE MAX_TOKEN_PER_MARKING ? TECHNIQUES X\n";
        let ss = parse_ss(c);
        let k = mcc_known_answers(&BTreeMap::new(), Some(&ss));
        assert_eq!(q(&k, "Q0"), Some(&json!({"STATES": 29, "TRANSITIONS": 71, "MAX_TOKEN_IN_PLACE": 3})));
        // transitions unknown or states infinite: no Q0
        let ss = parse_ss("A StateSpace\nSTATE_SPACE STATES 29 TECHNIQUES X\nSTATE_SPACE TRANSITIONS ? TECHNIQUES X\n");
        assert!(mcc_known_answers(&BTreeMap::new(), Some(&ss)).is_empty());
        let ss = parse_ss("A StateSpace\nSTATE_SPACE STATES +inf TECHNIQUES X\n");
        assert!(mcc_known_answers(&BTreeMap::new(), Some(&ss)).is_empty());
    }

    #[test]
    fn manifest_lines_are_deterministic() {
        let mk = |id: &str| Record {
            instance_id: id.into(),
            path: format!("{id}.pnml"),
            source: "mcc-small".into(),
            mcc_model_id: Some("M".into()),
            mcc_family: Some("F".into()),
            detected_class_petrivet: Some("general"),
            places: Some(1),
            transitions: Some(1),
            arcs: Some(2),
            tokens_total: Some(1),
            max_weight: 1,
            known_answers: mcc_known_answers(&verdicts(&[(Exam::RD, Verdict::True)]), None),
            targets: vec![],
        };
        let mut a = vec![mk("b"), mk("a"), mk("a")];
        let mut b = vec![mk("a"), mk("b"), mk("a")];
        assert_eq!(sort_dedup(&mut a), 1);
        sort_dedup(&mut b);
        assert_eq!(to_lines(&a).unwrap(), to_lines(&b).unwrap());
        assert!(to_lines(&a).unwrap().starts_with("{\"instance_id\":\"a\""));
    }

    #[test]
    fn calibration_answers_and_target_ids() {
        let a = json!({"Q0": {"states": 3, "transitions": 3}, "Q1": true, "Q2": true, "Q3": "n/a", "Q4": true, "Q5": true, "Q6": false,
            "targets": [{"kind": "QR", "marking": [0, 1, 0], "expected": true, "why": "x"}]});
        let (k, t) = calibration_answers("n", "id", &a, 3).unwrap();
        assert_eq!(q(&k, "Q3"), None);
        assert_eq!(q(&k, "Q0"), Some(&json!({"STATES": 3, "TRANSITIONS": 3})));
        let qr = k.iter().find(|x| x.query == "QR").unwrap();
        assert_eq!(qr.target_id.as_deref(), Some(t[0].target_id.as_str()));
        assert_eq!(t[0].target_id.len(), 16);
        assert!(calibration_answers("n", "id", &a, 4).is_err());
    }
}
