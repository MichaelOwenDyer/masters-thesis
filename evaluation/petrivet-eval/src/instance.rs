//! Instance records (SPEC 5.6) and the loaders that produce them.
//!
//! Two sources: a JSONL manifest (one instance per line; unknown fields ignored, missing optional ones
//! defaulted) and `corpus/mcc/selected.json` (turned into the same records on the fly).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A target marking: either an index based vector (manifest convention) or an explicit place-id map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TargetMarking {
    Vector(Vec<u32>),
    Map(BTreeMap<String, u32>),
}

impl Default for TargetMarking {
    fn default() -> Self {
        TargetMarking::Vector(Vec::new())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Target {
    #[serde(default)]
    pub target_id: String,
    /// `QR` or `QC`.
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub marking: TargetMarking,
    #[serde(default)]
    pub firing_sequence: Option<serde_json::Value>,
}

/// An answer known from outside (for example the MCC oracle). Not used by the runner.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KnownAnswer {
    #[serde(default)]
    pub query: String,
    /// A boolean for yes/no queries, an object for Q0 (state-space size); whatever the manifest writer put there.
    #[serde(default)]
    pub verdict: serde_json::Value,
    #[serde(default)]
    pub origin: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Instance {
    #[serde(default)]
    pub instance_id: String,
    /// Relative to the directory the manifest lives in (see [`Instance::abs_path`]).
    #[serde(default)]
    pub path: String,
    /// `gen`, `fam`, `mcc-small`, `mcc-large`.
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub places: u64,
    #[serde(default)]
    pub transitions: u64,
    #[serde(default)]
    pub arcs: u64,
    #[serde(default)]
    pub tokens_total: Option<u64>,
    #[serde(default)]
    pub max_weight: Option<u64>,
    #[serde(default)]
    pub generator_class: Option<String>,
    #[serde(default)]
    pub detected_class_testkit: Option<String>,
    #[serde(default)]
    pub detected_class_petrivet: Option<String>,
    #[serde(default)]
    pub marking_regime: Option<String>,
    #[serde(default)]
    pub size_bucket: Option<String>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub gen_version: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    /// Number or string, depending on the family.
    #[serde(default)]
    pub size_param: Option<serde_json::Value>,
    #[serde(default)]
    pub mcc_model_id: Option<String>,
    #[serde(default)]
    pub mcc_family: Option<String>,
    /// Optional explicit place ids in index order (overrides the `p{i}` / document-order assumption).
    #[serde(default)]
    pub place_ids: Option<Vec<String>>,
    #[serde(default)]
    pub targets: Vec<Target>,
    #[serde(default)]
    pub known_answers: Vec<KnownAnswer>,
    /// Directory `path` is relative to; set by the loaders, not part of the record.
    #[serde(skip)]
    pub base: PathBuf,
}

impl Instance {
    pub fn abs_path(&self) -> PathBuf {
        self.base.join(&self.path)
    }
}

/// Loads a manifest by extension: `.jsonl` is a manifest, `.json` is `selected.json`.
pub fn load(path: &Path) -> Result<Vec<Instance>, String> {
    if path.extension().is_some_and(|e| e == "json") { load_selected(path) } else { load_jsonl(path) }
}

/// One JSON object per line; blank lines are skipped. `path` is resolved against the manifest's directory.
pub fn load_jsonl(path: &Path) -> Result<Vec<Instance>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let base = path.canonicalize().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut inst: Instance = serde_json::from_str(line).map_err(|e| format!("{}:{}: {e}", path.display(), i + 1))?;
        if inst.path.is_empty() {
            return Err(format!("{}:{}: instance without path", path.display(), i + 1));
        }
        if inst.instance_id.is_empty() {
            let bytes = std::fs::read(base.join(&inst.path)).map_err(|e| format!("{}: {e}", inst.path))?;
            inst.instance_id = blake3::hash(&bytes).to_hex().to_string();
        }
        inst.base = base.clone();
        out.push(inst);
    }
    Ok(out)
}

#[derive(Deserialize)]
struct SelectedFile {
    models: Vec<SelectedModel>,
}

#[derive(Deserialize)]
struct SelectedModel {
    id: String,
    #[serde(default)]
    family: Option<String>,
    #[serde(default)]
    stratum: String,
    #[serde(default)]
    verdicts: BTreeMap<String, String>,
}

/// `corpus/mcc/selected.json` to instances. Model files are `models/<id>/model.pnml` next to the file;
/// `instance_id` is the blake3 hex of the PNML bytes; models whose file is missing are an error.
///
/// `known_answers` come from the MCC oracle verdicts: L is Q3, QL is Q4, OS is Q5, SM is Q6, and RD is
/// Q1 with the polarity inverted (an oracle TRUE for ReachabilityDeadlock means a deadlock exists), origin
/// `mcc-oracle:<EXAM>`. Truth handling proper lives elsewhere; these entries are informational.
pub fn load_selected(path: &Path) -> Result<Vec<Instance>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file: SelectedFile = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let base = path.canonicalize().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
    let mut out = Vec::new();
    for m in file.models {
        let rel = format!("models/{}/model.pnml", m.id);
        let bytes = std::fs::read(base.join(&rel)).map_err(|e| format!("{}: {e}", base.join(&rel).display()))?;
        let known_answers = m
            .verdicts
            .iter()
            .filter_map(|(exam, v)| {
                let v = v.to_ascii_uppercase();
                if v != "TRUE" && v != "FALSE" {
                    return None;
                }
                let (query, verdict) = match exam.as_str() {
                    "L" => ("Q3", v),
                    "QL" => ("Q4", v),
                    "OS" => ("Q5", v),
                    "SM" => ("Q6", v),
                    "RD" => ("Q1", if v == "TRUE" { "FALSE".to_owned() } else { "TRUE".to_owned() }),
                    _ => return None,
                };
                Some(KnownAnswer { query: query.to_owned(), verdict: serde_json::Value::String(verdict), origin: format!("mcc-oracle:{exam}") })
            })
            .collect();
        out.push(Instance {
            instance_id: blake3::hash(&bytes).to_hex().to_string(),
            path: rel,
            source: if m.stratum == "small" { "mcc-small" } else { "mcc-large" }.to_owned(),
            family: m.family.clone(),
            mcc_model_id: Some(m.id),
            mcc_family: m.family,
            known_answers,
            base: base.clone(),
            ..Instance::default()
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerant_jsonl() {
        let dir = std::env::temp_dir().join(format!("pe-inst-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.pnml"), "x").unwrap();
        let line = r#"{"instance_id":"abc","path":"a.pnml","source":"gen","places":3,"future_field":[1,2],
            "targets":[{"target_id":"t1","kind":"QR","marking":[0,1,2]},{"target_id":"t2","kind":"QC","marking":{"p1":2},"extra":1}]}"#
            .replace('\n', " ");
        std::fs::write(dir.join("m.jsonl"), format!("{line}\n\n{{\"path\":\"a.pnml\"}}\n")).unwrap();
        let v = load(&dir.join("m.jsonl")).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].targets.len(), 2);
        assert_eq!(v[0].targets[1].marking, TargetMarking::Map([("p1".to_owned(), 2)].into()));
        assert_eq!(v[1].instance_id, blake3::hash(b"x").to_hex().to_string());
        // the loader resolves paths through the canonical manifest location (on macOS the temp
        // directory is a symlink: /var -> /private/var), so compare against the canonical directory
        assert_eq!(v[0].abs_path(), dir.canonicalize().unwrap().join("a.pnml"));
    }
}
