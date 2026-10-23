//! The petrivet CLI adapters `pv-dispatch`, `pv-general`, `pv-poly`.
//!
//! Command: `petrivet analyze FILE -p <prop> -m <mode> --json [--timeout-ms N] [--target FILE.json]`.
//! The binary is `PETRIVET_BIN` if set, else `[solvers.pv] command` of eval.toml (default `petrivet`,
//! looked up on PATH).
//!
//! Target markings in manifests are index based; [`target_json`] turns them into the place-id map the CLI
//! wants. ASSUMPTION: for `gen` nets place `i` has PNML id `p{i}` (testkit convention, `raw.rs`); for MCC
//! models place `i` is the i-th `<place id=...>` in document order; an instance may override both with an
//! explicit `place_ids` list.

use super::{Adapter, CommandSpec, Parsed, PvCfg, Query, Request, ToolInfo};
use crate::instance::{Instance, TargetMarking};
use crate::record::Verdict;
use serde::Deserialize;
use std::collections::BTreeMap;

pub struct Pv {
    id: String,
    mode: &'static str,
    program: String,
}

impl Pv {
    pub fn new(solver_id: &str, cfg: &PvCfg) -> Self {
        let mode = match solver_id {
            "pv-general" => "general",
            "pv-poly" => "poly",
            _ => "dispatch",
        };
        let program = std::env::var("PETRIVET_BIN").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| cfg.command.clone());
        Pv { id: solver_id.to_owned(), mode, program }
    }

    fn property(query: Query) -> &'static str {
        match query {
            Query::Q0 => "statespace",
            Query::Q1 => "df",
            Query::Q2 => "bounded",
            Query::Q3 => "live",
            Query::Q4 => "quasi-live",
            Query::Q5 => "onesafe",
            Query::Q6 => "stable",
            Query::QR => "reach",
            Query::QC => "cover",
        }
    }
}

#[derive(Deserialize)]
struct CliOut {
    #[serde(default)]
    property: String,
    verdict: String,
    #[serde(default)]
    parse_ms: Option<f64>,
    #[serde(default)]
    analyze_ms: Option<f64>,
    #[serde(default)]
    states: Option<u64>,
    #[serde(default)]
    edges: Option<u64>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_kind: Option<String>,
}

/// The last JSON line of stdout, if it parses as CLI output.
fn cli_json(stdout: &str) -> Option<CliOut> {
    let line = stdout.lines().rev().find(|l| l.trim_start().starts_with('{'))?;
    serde_json::from_str(line).ok()
}

impl Adapter for Pv {
    fn solver_id(&self) -> &str {
        &self.id
    }

    fn supports(&self, query: Query) -> bool {
        // Poly mode has no procedures for statespace and stable; reach and cover answer where an efficient
        // procedure applies (circuits, strongly connected state machines, trap refutation) and `unknown` otherwise.
        self.mode != "poly" || !matches!(query, Query::Q0 | Query::Q6)
    }

    fn command(&self, req: &Request) -> CommandSpec {
        let mut args: Vec<String> = vec![
            "analyze".into(),
            req.instance_path.display().to_string(),
            "-p".into(),
            Self::property(req.query).into(),
            "-m".into(),
            self.mode.into(),
            "--json".into(),
        ];
        if let Some(ms) = req.internal_timeout_ms {
            args.push("--timeout-ms".into());
            args.push(ms.to_string());
        }
        if let Some(t) = req.target_file {
            args.push("--target".into());
            args.push(t.display().to_string());
        }
        CommandSpec { program: self.program.clone(), args, env: Vec::new() }
    }

    fn parse(&self, stdout: &str, _exit_code: i32) -> Parsed {
        let Some(out) = cli_json(stdout) else {
            return Parsed::runner_error("unparseable output");
        };
        let mut p = Parsed {
            parse_ms: out.parse_ms,
            analyze_ms: out.analyze_ms,
            states: out.states,
            edges: out.edges,
            ..Parsed::verdict(Verdict::Unknown)
        };
        p.verdict = match out.verdict.as_str() {
            "true" if out.property == "statespace" && out.states.is_some() => Verdict::Count,
            "true" => Verdict::True,
            "false" => Verdict::False,
            "unknown" => Verdict::Unknown,
            "not_supported" => Verdict::NotSupported,
            "unbounded" => {
                p.note = Some("unbounded".into());
                Verdict::Unknown
            }
            // A clean exit with an error verdict, or a verdict we do not know, is not something the CLI does.
            other => return Parsed::runner_error(format!("unexpected verdict {other:?} with exit 0")),
        };
        p
    }

    /// Exit 1 with a JSON error line: `error_kind` `panic` or `internal` is CRASH with that detail, `input` is
    /// INPUT_REJECTED. Without `error_kind` (older CLI) a message containing "panicked" is CRASH(panic), anything
    /// else INPUT_REJECTED. Other cases use the default rule.
    fn nonzero_exit(&self, stdout: &str, stderr: &str, exit_code: i32) -> Parsed {
        if exit_code == 1
            && let Some(out) = cli_json(stdout)
            && out.verdict == "error"
        {
            let msg = out.error.as_deref().unwrap_or("");
            return match out.error_kind.as_deref() {
                Some(k @ ("panic" | "internal")) => Parsed::crash(k),
                Some(_) => Parsed::input_rejected(msg),
                None if msg.contains("panicked") => Parsed::crash("panic"),
                None => Parsed::input_rejected(msg),
            };
        }
        let diag = if stderr.trim().is_empty() { stdout } else { stderr };
        if diag.trim().is_empty() { Parsed::crash(format!("exit:{exit_code}")) } else { Parsed::input_rejected(diag) }
    }

    fn tool(&self) -> Option<ToolInfo> {
        Some(ToolInfo { program: self.program.clone(), version_args: Some(vec!["--version".into()]) })
    }
}

/// Place ids of a PNML text in document order (`<place id="...">`). Plain scan, no XML parser.
pub fn place_ids_from_pnml(pnml: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = pnml;
    while let Some(i) = rest.find("<place") {
        rest = &rest[i + 6..];
        if !rest.starts_with([' ', '\n', '\t', '\r']) {
            continue;
        }
        let tag_end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..tag_end];
        if let Some(j) = tag.find("id=") {
            let after = &tag[j + 3..];
            if let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'')
                && let Some(k) = after[1..].find(q)
            {
                ids.push(after[1..=k].to_owned());
            }
        }
    }
    ids
}

/// JSON text of the target map `{place id: tokens}` (zero entries omitted) for the CLI.
pub fn target_json(inst: &Instance, marking: &TargetMarking, pnml: &str) -> Result<String, String> {
    let mut map: BTreeMap<String, u32> = BTreeMap::new();
    match marking {
        TargetMarking::Map(m) => map.extend(m.iter().filter(|(_, n)| **n > 0).map(|(k, n)| (k.clone(), *n))),
        TargetMarking::Vector(v) => {
            let ids: Vec<String> = match (&inst.place_ids, inst.source.starts_with("mcc")) {
                (Some(ids), _) => ids.clone(),
                (None, true) => place_ids_from_pnml(pnml),
                (None, false) => (0..v.len()).map(|i| format!("p{i}")).collect(),
            };
            for (i, n) in v.iter().enumerate() {
                if *n == 0 {
                    continue;
                }
                let id = ids.get(i).ok_or_else(|| format!("target index {i} beyond {} places", ids.len()))?;
                map.insert(id.clone(), *n);
            }
        }
    }
    serde_json::to_string(&map).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::Finished;

    fn pv(id: &str) -> Pv {
        Pv { id: id.into(), mode: if id == "pv-poly" { "poly" } else { "dispatch" }, program: "petrivet".into() }
    }

    #[test]
    fn parses_cli_json() {
        let a = pv("pv-dispatch");
        let p = a.parse("{\"tool\":\"petrivet\",\"property\":\"df\",\"verdict\":\"false\",\"parse_ms\":1.5,\"analyze_ms\":2.5}\n", 0);
        assert_eq!(p.verdict, Verdict::False);
        assert_eq!(p.analyze_ms, Some(2.5));
        let p = a.parse("{\"property\":\"statespace\",\"verdict\":\"true\",\"states\":10,\"edges\":20}", 0);
        assert_eq!((p.verdict, p.states, p.edges), (Verdict::Count, Some(10), Some(20)));
        assert_eq!(a.parse("{\"verdict\":\"unbounded\"}", 0).verdict, Verdict::Unknown);
        assert_eq!(a.parse("{\"verdict\":\"not_supported\"}", 0).verdict, Verdict::NotSupported);
        let p = a.parse("garbage", 0);
        assert_eq!((p.verdict, p.detail.as_deref()), (Verdict::RunnerError, Some("unparseable output")));
        assert_eq!(a.parse("{\"verdict\":\"error\"}", 0).verdict, Verdict::RunnerError);
    }

    fn classify(stdout: &str, code: i32) -> Parsed {
        let a = pv("pv-dispatch");
        a.classify(&Finished { spawn_error: None, timed_out: false, memout: false, exit_code: Some(code), signal: None, stdout, stderr: "" })
    }

    #[test]
    fn cli_error_json_classification() {
        let case = |line: &str| {
            let p = classify(line, 1);
            (p.verdict, p.detail)
        };
        let d = |s: &str| Some(s.to_owned());
        assert_eq!(case(r#"{"verdict":"error","error_kind":"input","error":"net is not connected\nsecond line"}"#), (Verdict::InputRejected, d("net is not connected")));
        assert_eq!(case(r#"{"verdict":"error","error_kind":"panic","error":"analysis panicked"}"#), (Verdict::Crash, d("panic")));
        assert_eq!(case(r#"{"verdict":"error","error_kind":"internal","error":"x"}"#), (Verdict::Crash, d("internal")));
        // older CLI without error_kind
        assert_eq!(case(r#"{"verdict":"error","error":"analysis panicked"}"#), (Verdict::Crash, d("panic")));
        assert_eq!(case(r#"{"verdict":"error","error":"arc a1 has weight 3"}"#), (Verdict::InputRejected, d("arc a1 has weight 3")));
        // long diagnostics are cut
        let long = format!(r#"{{"verdict":"error","error_kind":"input","error":"{}"}}"#, "x".repeat(500));
        assert_eq!(case(&long).1.unwrap().len(), 200);
        // exit 1 without JSON and no diagnostic is not an input rejection
        assert_eq!(case("").0, Verdict::Crash);
        // exit 0 through the same entry point reads the verdict
        assert_eq!(classify(r#"{"property":"df","verdict":"true"}"#, 0).verdict, Verdict::True);
        assert_eq!(classify("", 101).detail.as_deref(), Some("panic"));
    }

    #[test]
    fn poly_support() {
        let p = pv("pv-poly");
        assert!(p.supports(Query::Q3) && p.supports(Query::QR) && p.supports(Query::QC));
        assert!(!p.supports(Query::Q0) && !p.supports(Query::Q6));
        assert!(pv("pv-dispatch").supports(Query::QC));
    }

    #[test]
    fn place_ids_and_targets() {
        let pnml = "<net><place id=\"a\"><name/></place><place id=\"b\"/><transition id=\"t\"/></net>";
        assert_eq!(place_ids_from_pnml(pnml), ["a", "b"]);
        let mut inst = Instance { source: "gen".into(), ..Instance::default() };
        let j = target_json(&inst, &TargetMarking::Vector(vec![0, 2, 1]), "").unwrap();
        assert_eq!(j, "{\"p1\":2,\"p2\":1}");
        inst.source = "mcc-small".into();
        let j = target_json(&inst, &TargetMarking::Vector(vec![0, 3]), pnml).unwrap();
        assert_eq!(j, "{\"b\":3}");
    }
}
