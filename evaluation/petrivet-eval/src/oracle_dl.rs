//! `petrivet-eval oracle`: download the MCC verdict archive and lay the .out files out per model.

use crate::config::Config;
use std::path::Path;
use std::process::Command;

/// Model id of a verdict file name: the name without the trailing `-<EXAM>.out`.
pub fn model_id_of(file_name: &str) -> Option<&str> {
    let stem = file_name.strip_suffix(".out")?;
    stem.rsplit_once('-').map(|(id, _)| id).filter(|id| !id.is_empty())
}

fn run_cmd(c: &mut Command) -> Result<(), String> {
    let out = c.output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

pub fn run(cfg: &Config) -> Result<(), String> {
    let tgz = cfg.oracle_tarball();
    std::fs::create_dir_all(&cfg.paths.corpus_dir).map_err(|e| e.to_string())?;
    if tgz.exists() {
        println!("using cached {}", tgz.display());
    } else {
        println!("downloading {}", cfg.fetch.oracle_url);
        let part = tgz.with_extension("gz.partial");
        run_cmd(
            Command::new("curl")
                .args(["-fsSL", "--retry", "2", "--max-time", &cfg.fetch.curl_max_time_secs.max(600).to_string(), "-o"])
                .arg(&part)
                .arg(&cfg.fetch.oracle_url),
        )
        .map_err(|e| format!("download failed: {e}"))?;
        std::fs::rename(&part, &tgz).map_err(|e| e.to_string())?;
    }
    let tmp = cfg.paths.corpus_dir.join("oracle.partial");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    run_cmd(Command::new("tar").arg("-xzf").arg(&tgz).arg("-C").arg(&tmp).arg("--strip-components=1"))
        .map_err(|e| format!("tar failed (delete {} to re-download): {e}", tgz.display()))?;
    let (models, files) = distribute(&tmp, &cfg.paths.oracle_dir)?;
    let _ = std::fs::remove_dir_all(&tmp);
    println!("{files} verdict files across {models} models in {}", cfg.paths.oracle_dir.display());
    Ok(())
}

/// Moves every `*.out` directly under `src` into `<dst>/<ModelId>/`. Returns (models, files).
fn distribute(src: &Path, dst: &Path) -> Result<(usize, usize), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let mut files = 0;
    for e in std::fs::read_dir(src).map_err(|e| e.to_string())?.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(id) = model_id_of(&name) else { continue };
        let dir = dst.join(id);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::rename(e.path(), dir.join(&name)).map_err(|e| e.to_string())?;
        files += 1;
    }
    if files == 0 {
        return Err(format!("no .out files found in the extracted archive ({})", src.display()));
    }
    // count what is on disk, so a re-run reports the full state
    let models = std::fs::read_dir(dst).map_err(|e| e.to_string())?.filter_map(Result::ok).filter(|e| e.path().is_dir()).count();
    let files_total: usize = std::fs::read_dir(dst)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|d| std::fs::read_dir(d.path()).map(|r| r.filter_map(Result::ok).filter(|f| f.file_name().to_string_lossy().ends_with(".out")).count()).unwrap_or(0))
        .sum();
    Ok((models, files_total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_from_file_name() {
        assert_eq!(model_id_of("Anderson-PT-04-SS.out"), Some("Anderson-PT-04"));
        assert_eq!(model_id_of("A-PT-1-QLA.out"), Some("A-PT-1"));
        assert_eq!(model_id_of("noext"), None);
        assert_eq!(model_id_of("-SS.out"), None);
    }
}
