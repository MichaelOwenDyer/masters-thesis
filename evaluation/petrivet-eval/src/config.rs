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
    pub oracle_2025_zip: PathBuf,
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
        for p in [
            &mut cfg.paths.oracle_dir,
            &mut cfg.paths.oracle_2025_zip,
            &mut cfg.paths.corpus_dir,
        ] {
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
    pub fn models_dir(&self) -> PathBuf {
        self.paths.corpus_dir.join("models")
    }
}
