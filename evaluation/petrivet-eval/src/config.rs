use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub paths: Paths,
    pub select: SelectCfg,
    pub fetch: FetchCfg,
}

#[derive(Debug, Deserialize)]
pub struct Paths {
    pub oracle_dir: PathBuf,
    /// Optional: when absent or missing, the 2025 comparison is skipped.
    #[serde(default)]
    pub oracle_2025_zip: Option<PathBuf>,
    pub corpus_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct SelectCfg {
    pub seed: u64,
    pub small_max_states: u64,
    pub models_per_candidate: usize,
    pub small_target: usize,
}

#[derive(Debug, Deserialize)]
pub struct FetchCfg {
    pub base_url: String,
    pub oracle_url: String,
    pub quota_per_exam: usize,
    pub max_pnml_bytes: u64,
    pub curl_max_time_secs: u64,
}

impl Config {
    /// Loads the config; relative paths are resolved against the config file's directory.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut cfg: Config = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let base = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?
            .parent()
            .unwrap()
            .to_path_buf();
        for p in [Some(&mut cfg.paths.oracle_dir), cfg.paths.oracle_2025_zip.as_mut(), Some(&mut cfg.paths.corpus_dir)]
            .into_iter()
            .flatten()
        {
            if p.is_relative() {
                *p = base.join(&*p);
            }
        }
        Ok(cfg)
    }

    pub fn selection_path(&self) -> PathBuf {
        self.paths.corpus_dir.join("selection.json")
    }
    pub fn selected_path(&self) -> PathBuf {
        self.paths.corpus_dir.join("selected.json")
    }
    pub fn histogram_path(&self) -> PathBuf {
        self.paths.corpus_dir.join("histogram.json")
    }
    pub fn oracle_tarball(&self) -> PathBuf {
        self.paths.corpus_dir.join("oracle.tar.gz")
    }
    pub fn models_dir(&self) -> PathBuf {
        self.paths.corpus_dir.join("models")
    }
}
