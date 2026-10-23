//! PNML to LoLA conversion with `ndrio -PNML -lola`, cached outside the timed run, and the small PNML scans the
//! LoLA and TINA adapters need (transition ids, initial markings).
//!
//! Cache: `<results_dir>/convert/<instance_id>.lola` plus `<instance_id>.lola.ms` (the conversion time of the run
//! that created it, reported as `convert_ms` of every later run). A conversion fails if `ndrio` exits nonzero or
//! leaves an empty or missing output file (it does both on invalid XML); failures are not cached.

use super::first_line;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(Debug)]
pub struct Converted {
    pub path: PathBuf,
    pub ms: f64,
}

#[derive(Debug, PartialEq)]
pub enum ConvertError {
    /// The tool ran and refused: INPUT_REJECTED with detail `conversion failed: <msg>`. `ms` is the time spent.
    Failed { msg: String, ms: f64 },
    /// `ndrio` could not be started or the cache is not writable: RUNNER_ERROR.
    Infra(String),
}

pub fn cache_path(results_dir: &Path, instance_id: &str) -> PathBuf {
    results_dir.join("convert").join(format!("{instance_id}.lola"))
}

/// Converts `pnml` (skipped if the cache file exists and is not empty).
pub fn ensure_lola(ndrio: &str, pnml: &Path, results_dir: &Path, instance_id: &str) -> Result<Converted, ConvertError> {
    let out = cache_path(results_dir, instance_id);
    let ms_file = PathBuf::from(format!("{}.ms", out.display()));
    if std::fs::metadata(&out).is_ok_and(|m| m.len() > 0) {
        let ms = std::fs::read_to_string(&ms_file).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0.0);
        return Ok(Converted { path: out, ms });
    }
    let dir = out.parent().unwrap();
    std::fs::create_dir_all(dir).map_err(|e| ConvertError::Infra(format!("{}: {e}", dir.display())))?;
    // Unique temporary name: jobs of the same instance may convert at the same time.
    let tmp = dir.join(format!("{instance_id}.{}.{:?}.tmp", std::process::id(), std::thread::current().id()).replace(['(', ')'], ""));
    let _ = std::fs::remove_file(&tmp);
    let t = Instant::now();
    let res = Command::new(ndrio).args(["-PNML", "-lola"]).arg(pnml).arg(&tmp).stdin(Stdio::null()).output();
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    let res = match res {
        Ok(r) => r,
        Err(e) => return Err(ConvertError::Infra(format!("cannot run {ndrio}: {e}"))),
    };
    let size = std::fs::metadata(&tmp).map_or(0, |m| m.len());
    if !res.status.success() || size == 0 {
        let _ = std::fs::remove_file(&tmp);
        let text = format!("{}\n{}", String::from_utf8_lossy(&res.stderr), String::from_utf8_lossy(&res.stdout));
        let line = first_line(&text);
        let msg = if !line.is_empty() {
            line
        } else if res.status.success() {
            "empty output file".to_owned()
        } else {
            format!("exit {}", res.status.code().map_or("signal".to_owned(), |c| c.to_string()))
        };
        return Err(ConvertError::Failed { msg, ms });
    }
    let _ = std::fs::write(&ms_file, format!("{ms}"));
    std::fs::rename(&tmp, &out).map_err(|e| ConvertError::Infra(format!("{}: {e}", out.display())))?;
    Ok(Converted { path: out, ms })
}

/// Opening tags `<tag ...>` of a PNML text: (attribute text, self-closing, byte offset after the tag).
fn tags<'a>(pnml: &'a str, tag: &'a str) -> impl Iterator<Item = (&'a str, bool, usize)> + 'a {
    let open = format!("<{tag}");
    let mut pos = 0;
    std::iter::from_fn(move || {
        loop {
            let i = pnml[pos..].find(&open)? + pos;
            let after = i + open.len();
            pos = after;
            if !pnml[after..].starts_with([' ', '\n', '\t', '\r', '/', '>']) {
                continue;
            }
            let end = pnml[after..].find('>')? + after;
            pos = end + 1;
            let attrs = &pnml[after..end];
            return Some((attrs.trim_end_matches('/'), attrs.ends_with('/'), end + 1));
        }
    })
}

fn attr_id(attrs: &str) -> Option<String> {
    let j = attrs.find("id=")?;
    let after = &attrs[j + 3..];
    let q = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let k = after[1..].find(q)?;
    Some(after[1..=k].to_owned())
}

/// Transition ids in document order.
pub fn transition_ids(pnml: &str) -> Vec<String> {
    tags(pnml, "transition").filter_map(|(a, _, _)| attr_id(a)).collect()
}

/// `(place id, initial tokens)` in document order; no `<initialMarking>` means 0. Plain scan, no XML parser.
pub fn places_with_marking(pnml: &str) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for (attrs, closed, body) in tags(pnml, "place") {
        let Some(id) = attr_id(attrs) else { continue };
        let mut m0 = 0;
        if !closed {
            let end = pnml[body..].find("</place>").map_or(pnml.len(), |e| e + body);
            let content = &pnml[body..end];
            if let Some(i) = content.find("<initialMarking")
                && let Some(t) = content[i..].find("<text>")
            {
                let rest = &content[i + t + 6..];
                if let Some(e) = rest.find("</text>") {
                    m0 = rest[..e].trim().parse().unwrap_or(0);
                }
            }
        }
        out.push((id, m0));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    const PNML: &str = r#"<pnml><net id="n"><page id="pg">
      <place id="p0"><name><text>x</text></name><initialMarking><text>3</text></initialMarking></place>
      <place id="p1"/>
      <place id='p2'><name><text>y</text></name></place>
      <transition id="t0"/><transition id="t1"><name><text>t</text></name></transition>
      <placeholder id="no"/>
    </page></net></pnml>"#;

    #[test]
    fn scans() {
        assert_eq!(places_with_marking(PNML), [("p0".to_owned(), 3), ("p1".to_owned(), 0), ("p2".to_owned(), 0)]);
        assert_eq!(transition_ids(PNML), ["t0", "t1"]);
        let cal = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../calibration/cal-01-circuit-live.pnml")).unwrap();
        assert_eq!(places_with_marking(&cal), [("p0".to_owned(), 1), ("p1".to_owned(), 0), ("p2".to_owned(), 0)]);
        assert_eq!(transition_ids(&cal), ["t0", "t1", "t2"]);
    }

    fn script(dir: &Path, name: &str, body: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p.display().to_string()
    }

    #[test]
    fn conversion_cache_and_failures() {
        let dir = std::env::temp_dir().join(format!("pe-conv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pnml = dir.join("a.pnml");
        std::fs::write(&pnml, "x").unwrap();
        // good tool: copies input to output, counts calls
        let good = script(&dir, "good", &format!("echo x >> {}/calls; cp \"$3\" \"$4\"", dir.display()));
        let c = ensure_lola(&good, &pnml, &dir, "id1").unwrap();
        assert!(c.path.is_file());
        let c2 = ensure_lola(&good, &pnml, &dir, "id1").unwrap();
        assert_eq!(c2.path, c.path);
        assert!((c2.ms - c.ms).abs() < 1e-6, "cached conversion time is reported again");
        assert_eq!(std::fs::read_to_string(dir.join("calls")).unwrap().lines().count(), 1, "second call is a cache hit");
        // exit 1 with an empty output file (what ndrio does on invalid XML)
        let bad = script(&dir, "bad", "echo 'unexpected error in pnml parser (XML declaration not well-formed @1:14)' >&2; : > \"$4\"; exit 1");
        match ensure_lola(&bad, &pnml, &dir, "id2") {
            Err(ConvertError::Failed { msg, .. }) => assert_eq!(msg, "unexpected error in pnml parser (XML declaration not well-formed @1:14)"),
            other => panic!("{other:?}"),
        }
        assert!(!cache_path(&dir, "id2").exists());
        // exit 0 but empty output
        let empty = script(&dir, "empty", ": > \"$4\"");
        assert!(matches!(ensure_lola(&empty, &pnml, &dir, "id3"), Err(ConvertError::Failed { msg, .. }) if msg == "empty output file"));
        // exit 0 and no output file at all
        let none = script(&dir, "none", "exit 0");
        assert!(matches!(ensure_lola(&none, &pnml, &dir, "id4"), Err(ConvertError::Failed { .. })));
        // tool missing: infrastructure
        assert!(matches!(ensure_lola("/nonexistent/ndrio", &pnml, &dir, "id5"), Err(ConvertError::Infra(_))));
    }
}
