//! Minimal parser for MCC oracle verdict files.
//!
//! File format: first line `<ModelId> <Examination>`, then `STATE_SPACE STATES <n|?|+inf> TECHNIQUES ...`
//! or `FORMULA <Name> TRUE|FALSE|? TECHNIQUES ...`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Exam {
    SS,
    L,
    QL,
    RD,
    OS,
    SM,
}

impl Exam {
    pub const ALL: [Exam; 6] = [Exam::SS, Exam::L, Exam::QL, Exam::RD, Exam::OS, Exam::SM];
    /// Examinations that can stratify mcc-large (formula-style, a Boolean per model).
    pub const LARGE: [Exam; 4] = [Exam::L, Exam::RD, Exam::QL, Exam::OS];
    pub fn tag(self) -> &'static str {
        match self {
            Exam::SS => "SS",
            Exam::L => "L",
            Exam::QL => "QL",
            Exam::RD => "RD",
            Exam::OS => "OS",
            Exam::SM => "SM",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    #[serde(rename = "true")]
    True,
    #[serde(rename = "false")]
    False,
    #[serde(rename = "?")]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum States {
    Known(u64),
    /// `+inf`: the oracle states the state space is infinite.
    Inf,
    /// `?`: no consensus.
    Unknown,
    /// No StateSpace file or no STATES line.
    Absent,
}

impl States {
    pub fn definite(self) -> bool {
        matches!(self, States::Known(_) | States::Inf)
    }
    pub fn raw(self) -> String {
        match self {
            States::Known(n) => n.to_string(),
            States::Inf => "+inf".into(),
            States::Unknown => "?".into(),
            States::Absent => "absent".into(),
        }
    }
    pub fn known(self) -> Option<u64> {
        if let States::Known(n) = self { Some(n) } else { None }
    }
}

pub fn parse_states(content: &str) -> States {
    for line in content.lines().skip(1) {
        let mut it = line.split_whitespace();
        if it.next() == Some("STATE_SPACE") && it.next() == Some("STATES") {
            return match it.next() {
                Some("?") | None => States::Unknown,
                Some(v) => match v.parse::<u64>() {
                    Ok(n) => States::Known(n),
                    Err(_) if v.contains("inf") => States::Inf,
                    Err(_) => States::Unknown,
                },
            };
        }
    }
    States::Absent
}

/// All FORMULA lines must agree on a definite value, otherwise the result is `Unknown`.
/// `None` when there is no FORMULA line.
pub fn parse_formula(content: &str) -> Option<Verdict> {
    let mut result: Option<Verdict> = None;
    for line in content.lines().skip(1) {
        let mut it = line.split_whitespace();
        if it.next() != Some("FORMULA") {
            continue;
        }
        let _name = it.next();
        let v = match it.next() {
            Some("TRUE") => Verdict::True,
            Some("FALSE") => Verdict::False,
            _ => Verdict::Unknown,
        };
        result = Some(match result {
            None => v,
            Some(prev) if prev == v => v,
            Some(_) => Verdict::Unknown,
        });
    }
    result
}

#[derive(Debug, Clone)]
pub struct ModelOracle {
    pub id: String,
    pub states: States,
    /// Present only for examinations whose file exists and has a FORMULA line.
    pub verdicts: BTreeMap<Exam, Verdict>,
}

impl ModelOracle {
    pub fn verdict(&self, e: Exam) -> Verdict {
        self.verdicts.get(&e).copied().unwrap_or(Verdict::Unknown)
    }
    pub fn definite(&self, e: Exam) -> bool {
        match e {
            Exam::SS => self.states.definite(),
            _ => self.verdict(e) != Verdict::Unknown,
        }
    }
    pub fn any_definite(&self) -> bool {
        Exam::ALL.iter().any(|&e| self.definite(e))
    }
    pub fn any_large_definite(&self) -> bool {
        Exam::LARGE.iter().any(|&e| self.definite(e))
    }
    pub fn is_colored_id(&self) -> bool {
        self.id.contains("-COL-")
    }
}

/// Family: the model id up to the first `-PT-` (the whole id when absent).
pub fn family(id: &str) -> &str {
    match id.find("-PT-") {
        Some(i) => &id[..i],
        None => id,
    }
}

pub fn load_model(dir: &Path, id: &str) -> ModelOracle {
    let read = |e: Exam| std::fs::read_to_string(dir.join(id).join(format!("{id}-{}.out", e.tag()))).ok();
    let states = read(Exam::SS).map_or(States::Absent, |c| parse_states(&c));
    let mut verdicts = BTreeMap::new();
    for e in [Exam::L, Exam::QL, Exam::RD, Exam::OS, Exam::SM] {
        if let Some(v) = read(e).and_then(|c| parse_formula(&c)) {
            verdicts.insert(e, v);
        }
    }
    ModelOracle { id: id.to_string(), states, verdicts }
}

/// Loads every model directory, sorted by id.
pub fn load_all(dir: &Path) -> Result<Vec<ModelOracle>, String> {
    let mut ids: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    ids.sort();
    Ok(ids.iter().map(|id| load_model(dir, id)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_variants() {
        let n = "A SS\nSTATE_SPACE STATES 29641 TECHNIQUES X\nSTATE_SPACE TRANSITIONS 5 TECHNIQUES X\n";
        assert_eq!(parse_states(n), States::Known(29641));
        assert_eq!(parse_states("A SS\nSTATE_SPACE STATES ? TECHNIQUES X\n"), States::Unknown);
        assert_eq!(parse_states("A SS\nSTATE_SPACE STATES +inf TECHNIQUES X\n"), States::Inf);
        assert_eq!(parse_states("A SS\n"), States::Absent);
        assert!(States::Inf.definite() && !States::Unknown.definite());
    }

    #[test]
    fn formula_variants() {
        assert_eq!(parse_formula("A L\nFORMULA A-L TRUE TECHNIQUES X\n"), Some(Verdict::True));
        assert_eq!(parse_formula("A L\nFORMULA A-L FALSE TECHNIQUES X\n"), Some(Verdict::False));
        assert_eq!(parse_formula("A L\nFORMULA A-L ? TECHNIQUES X\n"), Some(Verdict::Unknown));
        assert_eq!(parse_formula("A L\n"), None);
    }

    #[test]
    fn family_extraction() {
        assert_eq!(family("ASLink-PT-01a"), "ASLink");
        assert_eq!(family("ARMCacheCoherence-PT-none"), "ARMCacheCoherence");
        assert_eq!(family("Foo-Bar-PT-x-PT-y"), "Foo-Bar");
        assert_eq!(family("NoPt"), "NoPt");
    }
}
