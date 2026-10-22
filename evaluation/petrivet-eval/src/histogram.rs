//! Structure histogram of the selected models, parsed with petrivet.

use crate::config::Config;
use crate::fetch::Selected;
use crate::select::write_json;
use petrivet::system::PetriNet;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Default, Clone, Copy)]
pub struct Range {
    pub min: u64,
    pub median: u64,
    pub max: u64,
}

fn range(mut v: Vec<u64>) -> Option<Range> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    // lower median: index len/2 as in the selection
    Some(Range { min: v[0], median: v[v.len() / 2], max: *v.last().unwrap() })
}

#[derive(Debug, Serialize)]
pub struct Failure {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct StratumHistogram {
    pub models: usize,
    pub parsed: usize,
    pub parse_failures: Vec<Failure>,
    pub classes: BTreeMap<String, usize>,
    pub places: Option<Range>,
    pub transitions: Option<Range>,
    pub arcs: Option<Range>,
    /// Models whose PNML has at least one arc inscription above 1 (text scan; the importer now rejects such nets).
    pub models_with_weighted_arcs: usize,
    pub models_with_weighted_arcs_pct: f64,
    pub max_arc_weight: u64,
    /// Marked places in the initial marking (parsed models).
    pub marked_places: Option<Range>,
    pub total_initial_tokens: Option<Range>,
    pub models_with_initial_tokens: usize,
}

pub struct Parsed {
    pub class: String,
    places: u64,
    transitions: u64,
    arcs: u64,
    marked: u64,
    tokens: u64,
}

pub fn parse_with_petrivet(xml: String) -> Result<Parsed, String> {
    // Own thread with a large stack; a panic or an error is reported, never fatal.
    let h = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let sys = PetriNet::from_pnml(&xml).map_err(|e| e.to_string())?;
            let m = sys.marking();
            Ok::<_, String>(Parsed {
                class: format!("{:?}", sys.class()),
                places: u64::from(sys.place_count()),
                transitions: u64::from(sys.transition_count()),
                arcs: sys.arc_count() as u64,
                marked: m.support().count() as u64,
                tokens: u64::from(m.total_tokens()),
            })
        })
        .map_err(|e| e.to_string())?;
    match h.join() {
        Ok(r) => r,
        Err(p) => {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_else(|| "unknown panic".into());
            Err(format!("panic: {msg}"))
        }
    }
}

/// Short reason category for a petrivet parse error message.
pub fn classify_reason(msg: &str) -> String {
    if msg.contains("must be connected") {
        "not connected".into()
    } else if msg.contains("has weight") {
        "arc weight".into()
    } else if msg.starts_with("panic") {
        "parser panic".into()
    } else {
        "other parse error".into()
    }
}

/// Largest arc weight found in `<inscription>` elements (simple text scan: the first `<text>` after each
/// `<inscription`). Places' initial markings use `<initialMarking>` and are not matched.
pub fn max_inscription(xml: &str) -> u64 {
    let mut max = 1;
    let mut rest = xml;
    while let Some(i) = rest.find("<inscription") {
        rest = &rest[i + "<inscription".len()..];
        let Some(t) = rest.find("<text>") else { break };
        let after = &rest[t + "<text>".len()..];
        let end = after.find('<').unwrap_or(after.len());
        if let Ok(n) = after[..end].trim().parse::<u64>() {
            max = max.max(n);
        }
        rest = after;
    }
    max
}

pub fn run(cfg: &Config) -> Result<(), String> {
    let selected: Selected = serde_json::from_str(
        &std::fs::read_to_string(cfg.selected_path()).map_err(|e| format!("{} (run `fetch` first): {e}", cfg.selected_path().display()))?,
    )
    .map_err(|e| e.to_string())?;

    std::panic::set_hook(Box::new(|_| {}));
    let mut out: BTreeMap<String, StratumHistogram> = BTreeMap::new();
    for stratum in ["small", "large"] {
        let mut classes: BTreeMap<String, usize> = BTreeMap::new();
        let (mut pl, mut tr, mut ar, mut mk, mut tk) = (vec![], vec![], vec![], vec![], vec![]);
        let mut failures = vec![];
        let (mut n, mut weighted, mut maxw, mut with_tokens) = (0usize, 0usize, 1u64, 0usize);
        for m in selected.models.iter().filter(|m| m.stratum.split(':').next() == Some(stratum)) {
            n += 1;
            let path = cfg.models_dir().join(&m.id).join("model.pnml");
            let xml = match std::fs::read_to_string(&path) {
                Ok(x) => x,
                Err(e) => {
                    failures.push(Failure { id: m.id.clone(), reason: format!("read: {e}") });
                    continue;
                }
            };
            let w = max_inscription(&xml);
            if w > 1 {
                weighted += 1;
            }
            maxw = maxw.max(w);
            match parse_with_petrivet(xml) {
                Ok(p) => {
                    *classes.entry(p.class).or_default() += 1;
                    pl.push(p.places);
                    tr.push(p.transitions);
                    ar.push(p.arcs);
                    mk.push(p.marked);
                    tk.push(p.tokens);
                    if p.marked > 0 {
                        with_tokens += 1;
                    }
                }
                Err(e) => failures.push(Failure { id: m.id.clone(), reason: e }),
            }
        }
        out.insert(
            stratum.to_string(),
            StratumHistogram {
                models: n,
                parsed: pl.len(),
                parse_failures: failures,
                classes,
                places: range(pl),
                transitions: range(tr),
                arcs: range(ar),
                models_with_weighted_arcs: weighted,
                models_with_weighted_arcs_pct: if n == 0 { 0.0 } else { (1000.0 * weighted as f64 / n as f64).round() / 10.0 },
                max_arc_weight: maxw,
                marked_places: range(mk),
                total_initial_tokens: range(tk),
                models_with_initial_tokens: with_tokens,
            },
        );
    }
    let _ = std::panic::take_hook();
    write_json(&cfg.histogram_path(), &out)?;

    let fmt = |r: &Option<Range>| r.map_or("-".into(), |r| format!("{}/{}/{}", r.min, r.median, r.max));
    for (name, h) in &out {
        println!("== {name}: {} models, {} parsed, {} parse failures", h.models, h.parsed, h.parse_failures.len());
        for (c, k) in &h.classes {
            println!("   class {c:<16} {k}");
        }
        println!("   places       min/median/max  {}", fmt(&h.places));
        println!("   transitions  min/median/max  {}", fmt(&h.transitions));
        println!("   arcs         min/median/max  {}", fmt(&h.arcs));
        println!("   marked places min/median/max {}", fmt(&h.marked_places));
        println!("   initial tokens min/median/max {}", fmt(&h.total_initial_tokens));
        println!("   models with initial tokens   {}", h.models_with_initial_tokens);
        println!(
            "   models with arc weight > 1   {} ({:.1}%), max weight {} (text scan)",
            h.models_with_weighted_arcs, h.models_with_weighted_arcs_pct, h.max_arc_weight
        );
        for f in &h.parse_failures {
            println!("   parse failure {}: {}", f.id, f.reason);
        }
    }
    crate::fetch::print_skip_table(&selected);
    println!("wrote {}", cfg.histogram_path().display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inscription_scan() {
        let xml = "<arc><inscription><text>3</text></inscription></arc><place><initialMarking><text>9</text></initialMarking></place><arc><inscription><text>1</text></inscription></arc>";
        assert_eq!(max_inscription(xml), 3);
        assert_eq!(max_inscription("<arc/>"), 1);
    }

    #[test]
    fn median_range() {
        let r = range(vec![5, 1, 9, 3]).unwrap();
        assert_eq!((r.min, r.median, r.max), (1, 5, 9));
    }
}
