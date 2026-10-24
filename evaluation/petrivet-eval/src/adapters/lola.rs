//! The LoLA 2.0 adapter (`lola`). Commands follow `evaluation-spec/adapters-notes.md` section (b).
//!
//! The PNML is converted once per instance with `ndrio -PNML -lola` (see [`super::convert`], outside the timed run;
//! place and transition names are the PNML ids). Every command passes `--quiet --json=/dev/stdout`; there are no LoLA
//! time or marking limits (the external wall limit applies). LoLA exits 0 for every completed analysis, yes or no, so
//! the verdict is read from `analysis.result` of the JSON.
//!
//! | Query | Command |
//! |---|---|
//! | Q0 | `--check=full --stubborn=off`, `analysis.stats.{states,edges}` |
//! | Q1 | `--formula="EF DEADLOCK"`, result inverted (true = a deadlock exists = not deadlock-free) |
//! | Q2 | `--search=cover --encoder=full --formula="IMPOSSIBLE (p1 = oo OR ...)"` over all places (true = bounded) |
//! | Q3 | script: `AGEF FIREABLE(t)` per transition, stops at the first false (a combined formula over all transitions ran out of memory on tiny nets) |
//! | Q4 | script: `EF FIREABLE(t)` per transition, stops at the first false |
//! | Q5 | `--formula="AG (p1 <= 1 AND ...)"` |
//! | Q6 | script: `AG (p = m0)` per place, stops at the first true |
//! | QR | `--formula="EF (p1 = a1 AND ...)"` over ALL places, zeros included |
//! | QC | `--formula="EF (pi >= ai AND ...)"` over places with ai > 0; all-zero target: trivially true |
//!
//! Multi-run queries (Q3, Q4, Q6) are ONE `sh -c` job (the runner measures the whole script): the sub-runs go in order,
//! each sub-run JSON is echoed, the first non-zero exit code of a sub-run ends the script with that code, and the
//! last line is `PETRIVET_ANSWER true|false|unknown`.
//!
//! UNVERIFIED: settled by calibrate on the box: the combined formulas of Q2, Q3, Q5, `IMPOSSIBLE (... = oo)` with
//! `--search=cover`, the semantics of a null result, and the behaviour of LoLA on unbounded nets.

use super::convert::{places_with_marking, transition_ids};
use super::{
    Adapter, CommandSpec, Finished, LolaCfg, NdrioCfg, Parsed, Query, Request, ToolInfo, answer_parsed, classify_with, env_or, first_line,
    script_answer, sh_quote, shell_signal_exit,
};
use crate::record::Verdict;
use std::collections::BTreeMap;

pub struct Lola {
    program: String,
    ndrio: String,
}

impl Lola {
    pub fn new(cfg: &LolaCfg, ndrio: &NdrioCfg) -> Self {
        Lola { program: env_or("LOLA_BIN", &cfg.command), ndrio: env_or("NDRIO_BIN", &ndrio.command) }
    }

    fn lola_cmd(&self, net: &str, extra: &[String]) -> CommandSpec {
        let mut args = vec![net.to_owned()];
        args.extend(extra.iter().cloned());
        args.push("--quiet".into());
        args.push("--json=/dev/stdout".into());
        CommandSpec { program: self.program.clone(), args, env: Vec::new(), stdin: None }
    }

    fn formula_cmd(&self, net: &str, pre: &[&str], formula: String) -> CommandSpec {
        let mut extra: Vec<String> = pre.iter().map(|s| (*s).to_owned()).collect();
        extra.push(format!("--formula={formula}"));
        self.lola_cmd(net, &extra)
    }

    /// The command of a query. `places` are `(id, m0)` in document order, `target` is the place-id map of a QR/QC target
    /// (zero entries may be missing).
    pub fn build(&self, query: Query, net: &str, places: &[(String, u32)], transitions: &[String], target: Option<&BTreeMap<String, u32>>) -> CommandSpec {
        let join = |items: Vec<String>, sep: &str| items.join(sep);
        match query {
            Query::Q0 => self.lola_cmd(net, &["--check=full".into(), "--stubborn=off".into()]),
            Query::Q1 => self.formula_cmd(net, &[], "EF DEADLOCK".into()),
            Query::Q2 if places.is_empty() => constant("true"),
            Query::Q2 => {
                // UNVERIFIED: settled by calibrate on the box (one big OR with --search=cover, null result semantics)
                let f = join(places.iter().map(|(p, _)| format!("{p} = oo")).collect(), " OR ");
                self.formula_cmd(net, &["--search=cover", "--encoder=full"], format!("IMPOSSIBLE ({f})"))
            }
            Query::Q3 if transitions.is_empty() => constant("true"),
            Query::Q3 => {
                // the combined AG ((EF FIREABLE(t0)) AND ...) formula ran out of memory on nets of six transitions
                // (calibrate on the box); one verified AGEF run per transition, early exit on the first false
                let checks = transitions.iter().map(|t| format!("AGEF FIREABLE({t})")).collect();
                self.script(net, checks, "false", "true")
            }
            Query::Q4 => {
                // UNVERIFIED: settled by calibrate on the box (early exit on the first false; behaviour on limit hits)
                let checks = transitions.iter().map(|t| format!("EF FIREABLE({t})")).collect();
                self.script(net, checks, "false", "true")
            }
            Query::Q5 if places.is_empty() => constant("true"),
            Query::Q5 => {
                // UNVERIFIED: settled by calibrate on the box (the combined AG (p <= 1 AND ...) formula)
                let f = join(places.iter().map(|(p, _)| format!("{p} <= 1")).collect(), " AND ");
                self.formula_cmd(net, &[], format!("AG ({f})"))
            }
            Query::Q6 => {
                // UNVERIFIED: settled by calibrate on the box (`AG (p = m0)` per place, early exit on the first true)
                let checks = places.iter().map(|(p, m0)| format!("AG ({p} = {m0})")).collect();
                self.script(net, checks, "true", "false")
            }
            Query::QR => {
                let empty = BTreeMap::new();
                let t = target.unwrap_or(&empty);
                if places.is_empty() {
                    return constant("true");
                }
                let f = join(places.iter().map(|(p, _)| format!("{p} = {}", t.get(p).copied().unwrap_or(0))).collect(), " AND ");
                self.formula_cmd(net, &[], format!("EF ({f})"))
            }
            Query::QC => {
                // UNVERIFIED: settled by calibrate on the box (QC on unbounded nets, termination)
                let empty = BTreeMap::new();
                let t = target.unwrap_or(&empty);
                let parts: Vec<String> =
                    places.iter().filter_map(|(p, _)| t.get(p).copied().filter(|a| *a > 0).map(|a| format!("{p} >= {a}"))).collect();
                if parts.is_empty() {
                    return constant("true"); // the all-zero target is covered by every marking
                }
                self.formula_cmd(net, &[], format!("EF ({})", join(parts, " AND ")))
            }
        }
    }

    /// One `sh -c` job running `EF`/`AG` sub-runs in order: a sub-run whose result equals `stop` ends the script with
    /// that answer; a sub-run without a result (null or missing) makes the final answer unknown unless a later
    /// sub-run stops the script; a nonzero exit status of a sub-run ends the script with that status.
    fn script(&self, net: &str, checks: Vec<String>, stop: &str, otherwise: &str) -> CommandSpec {
        let mut s = format!("lola={}; net={}; unk=0\n", sh_quote(&self.program), sh_quote(net));
        s.push_str(
            "chk() {\n  out=$(\"$lola\" \"$net\" \"--formula=$1\" --quiet --json=/dev/stdout </dev/null); rc=$?\n  printf '%s\\n' \"$out\"\n  [ \"$rc\" -eq 0 ] || exit \"$rc\"\n  case \"$out\" in\n    *'\"result\": true'*|*'\"result\":true'*) res=true;;\n    *'\"result\": false'*|*'\"result\":false'*) res=false;;\n    *) res=other;;\n  esac\n  if [ \"$res\" = \"$2\" ]; then echo \"PETRIVET_ANSWER $2\"; exit 0; fi\n  [ \"$res\" = other ] && unk=1\n}\n",
        );
        for c in checks {
            s.push_str(&format!("chk {} {stop}\n", sh_quote(&c)));
        }
        s.push_str(&format!("if [ \"$unk\" -eq 1 ]; then echo 'PETRIVET_ANSWER unknown'; else echo 'PETRIVET_ANSWER {otherwise}'; fi\n"));
        CommandSpec::sh_script(s)
    }

    /// Reads the output of a run that exited with status 0.
    pub fn parse_q(&self, query: Query, stdout: &str) -> Parsed {
        if let Some(word) = script_answer(stdout) {
            return answer_parsed(word);
        }
        let Some(json) = json_line(stdout) else { return Parsed::runner_error("unparseable output") };
        let analysis = json.get("analysis");
        if query == Query::Q0 {
            let stats = analysis.and_then(|a| a.get("stats"));
            let (s, e) = (stats.and_then(|x| x.get("states")).and_then(|v| v.as_u64()), stats.and_then(|x| x.get("edges")).and_then(|v| v.as_u64()));
            return match (s, e) {
                (Some(s), Some(e)) => Parsed { states: Some(s), edges: Some(e), ..Parsed::verdict(Verdict::Count) },
                _ => Parsed::with_detail(Verdict::Unknown, "no analysis.stats in the LoLA output"),
            };
        }
        match analysis.and_then(|a| a.get("result")).and_then(|r| r.as_bool()) {
            // result true: a deadlock is reachable, so the net is not deadlock-free
            Some(r) if query == Query::Q1 => Parsed::verdict(if r { Verdict::False } else { Verdict::True }),
            Some(r) => Parsed::verdict(if r { Verdict::True } else { Verdict::False }),
            None => Parsed::with_detail(Verdict::Unknown, "analysis.result is null or missing"),
        }
    }

    /// Exit 1: INPUT_REJECTED with the `lola:` diagnostic line; exit 2 (premature termination): UNKNOWN; above 128:
    /// the shell reports a signal of a sub-run; anything else: the default rule.
    pub fn nonzero(&self, stdout: &str, stderr: &str, code: i32) -> Parsed {
        match code {
            2 => Parsed::with_detail(Verdict::Unknown, "premature termination (exit 2)"),
            1 => {
                if let Some(l) = stderr.lines().map(str::trim).find(|l| l.starts_with("lola:")) {
                    return Parsed::input_rejected(l);
                }
                default_nonzero(stdout, stderr, code)
            }
            c => shell_signal_exit(c).unwrap_or_else(|| default_nonzero(stdout, stderr, c)),
        }
    }
}

fn default_nonzero(stdout: &str, stderr: &str, code: i32) -> Parsed {
    let diag = if stderr.trim().is_empty() { stdout } else { stderr };
    if diag.trim().is_empty() { Parsed::crash(format!("exit:{code}")) } else { Parsed::input_rejected(&first_line(diag)) }
}

/// A command that prints a fixed answer (empty place or transition lists, the all-zero coverability target).
fn constant(answer: &str) -> CommandSpec {
    CommandSpec::new("sh", ["-c".to_owned(), format!("echo 'PETRIVET_ANSWER {answer}'")])
}

fn json_line(stdout: &str) -> Option<serde_json::Value> {
    stdout.lines().rev().filter(|l| l.trim_start().starts_with('{')).find_map(|l| serde_json::from_str(l).ok())
}

fn failing(msg: String) -> CommandSpec {
    CommandSpec::new("sh", ["-c".to_owned(), format!("echo {} >&2; exit 1", sh_quote(&msg))])
}

impl Adapter for Lola {
    fn solver_id(&self) -> &str {
        "lola"
    }

    fn supports(&self, _query: Query) -> bool {
        true
    }

    fn command(&self, req: &Request) -> CommandSpec {
        let net = req.converted.map_or_else(|| req.instance_path.with_extension("lola"), |p| p.to_path_buf()).display().to_string();
        let needs_names = !matches!(req.query, Query::Q0 | Query::Q1);
        let (mut places, mut transitions) = (Vec::new(), Vec::new());
        if needs_names {
            let pnml = match std::fs::read_to_string(req.instance_path) {
                Ok(t) => t,
                Err(e) => return failing(format!("cannot read {}: {e}", req.instance_path.display())),
            };
            places = places_with_marking(&pnml);
            transitions = transition_ids(&pnml);
        }
        let target = match req.target_file {
            Some(f) if req.query.needs_target() => match std::fs::read_to_string(f).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str::<BTreeMap<String, u32>>(&t).map_err(|e| e.to_string())) {
                Ok(m) => Some(m),
                Err(e) => return failing(format!("cannot read target {}: {e}", f.display())),
            },
            _ => None,
        };
        self.build(req.query, &net, &places, &transitions, target.as_ref())
    }

    fn parse(&self, stdout: &str, _exit_code: i32) -> Parsed {
        // Without the query only the query independent reading is possible (Q1 would come out inverted).
        self.parse_q(Query::Q2, stdout)
    }

    fn nonzero_exit(&self, stdout: &str, stderr: &str, exit_code: i32) -> Parsed {
        self.nonzero(stdout, stderr, exit_code)
    }

    fn classify_for(&self, query: Query, f: &Finished) -> Parsed {
        classify_with(f, |out| self.parse_q(query, out), |o, e, c| self.nonzero(o, e, c))
    }

    fn tool(&self) -> Option<ToolInfo> {
        Some(ToolInfo { program: self.program.clone(), version_args: Some(vec!["--version".into()]) })
    }

    fn converter(&self) -> Option<String> {
        Some(self.ndrio.clone())
    }

    fn programs(&self) -> Vec<String> {
        vec![self.program.clone(), self.ndrio.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lola() -> Lola {
        Lola { program: "lola".into(), ndrio: "ndrio".into() }
    }

    fn net() -> (Vec<(String, u32)>, Vec<String>) {
        (vec![("p0".into(), 1), ("p1".into(), 0), ("p2".into(), 2)], vec!["t0".into(), "t1".into()])
    }

    fn tool(name: &str) -> String {
        std::fs::read_to_string(format!("{}/testdata/tools/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn argv(q: Query, target: Option<&BTreeMap<String, u32>>) -> Vec<String> {
        let (p, t) = net();
        let c = lola().build(q, "N.lola", &p, &t, target);
        assert_eq!(c.program, "lola", "{q:?}");
        c.args
    }

    fn tail() -> [&'static str; 2] {
        ["--quiet", "--json=/dev/stdout"]
    }

    #[test]
    fn single_run_commands() {
        let mk = |mid: &[&str]| -> Vec<String> { ["N.lola"].iter().chain(mid.iter()).chain(tail().iter()).map(|s| (*s).to_owned()).collect() };
        assert_eq!(argv(Query::Q0, None), mk(&["--check=full", "--stubborn=off"]));
        assert_eq!(argv(Query::Q1, None), mk(&["--formula=EF DEADLOCK"]));
        assert_eq!(argv(Query::Q2, None), mk(&["--search=cover", "--encoder=full", "--formula=IMPOSSIBLE (p0 = oo OR p1 = oo OR p2 = oo)"]));
        assert_eq!(argv(Query::Q5, None), mk(&["--formula=AG (p0 <= 1 AND p1 <= 1 AND p2 <= 1)"]));
        // QR lists every place, zeros included
        let tgt: BTreeMap<String, u32> = [("p2".to_owned(), 3)].into();
        assert_eq!(argv(Query::QR, Some(&tgt)), mk(&["--formula=EF (p0 = 0 AND p1 = 0 AND p2 = 3)"]));
        // QC lists only places with a positive entry
        assert_eq!(argv(Query::QC, Some(&tgt)), mk(&["--formula=EF (p2 >= 3)"]));
    }

    #[test]
    fn zero_target_cover_is_trivially_true() {
        let (p, t) = net();
        let zero: BTreeMap<String, u32> = BTreeMap::new();
        let c = lola().build(Query::QC, "N.lola", &p, &t, Some(&zero));
        assert_eq!((c.program.as_str(), c.args[1].as_str()), ("sh", "echo 'PETRIVET_ANSWER true'"));
        let p = lola().parse_q(Query::QC, "PETRIVET_ANSWER true\n");
        assert_eq!(p.verdict, Verdict::True);
    }

    #[test]
    fn script_commands_have_early_exit() {
        let (p, t) = net();
        let q4 = lola().build(Query::Q4, "N.lola", &p, &t, None);
        assert_eq!(q4.program, "sh");
        let s = q4.script_text();
        assert!(s.contains("lola='lola'; net='N.lola'"));
        assert!(s.contains("chk 'EF FIREABLE(t0)' false\nchk 'EF FIREABLE(t1)' false\n"));
        let q3 = lola().build(Query::Q3, "N.lola", &p, &t, None);
        assert!(q3.script_text().contains("chk 'AGEF FIREABLE(t0)' false\nchk 'AGEF FIREABLE(t1)' false\n"));
        assert!(s.contains("[ \"$rc\" -eq 0 ] || exit \"$rc\""));
        assert!(s.contains("if [ \"$res\" = \"$2\" ]; then echo \"PETRIVET_ANSWER $2\"; exit 0; fi"));
        assert!(s.ends_with("else echo 'PETRIVET_ANSWER true'; fi\n"));
        let q6 = lola().build(Query::Q6, "N.lola", &p, &t, None);
        let s = q6.script_text();
        assert!(s.contains("chk 'AG (p0 = 1)' true\nchk 'AG (p1 = 0)' true\nchk 'AG (p2 = 2)' true\n"));
        assert!(s.ends_with("else echo 'PETRIVET_ANSWER false'; fi\n"));
    }

    /// Runs a generated script against a fake `lola` that answers by formula; checks early exit and exit propagation.
    #[test]
    fn scripts_run_with_a_fake_lola() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("pe-lola-fake-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("lola");
        // result false for formulas mentioning t1, null for t2, exit 1 for t3, exit 2 for t4, true otherwise; logs calls
        std::fs::write(
            &fake,
            format!(
                "#!/bin/sh\necho \"$2\" >> {0}/calls\ncase \"$2\" in\n *t1*|*p1*) echo '{{\"analysis\": {{\"result\": false}}}}';;\n *t2*|*p2*) echo '{{\"analysis\": {{\"result\": null}}}}';;\n *t3*|*p3*) echo 'lola: syntax error' >&2; exit 1;;\n *t4*|*p4*) exit 2;;\n *) echo '{{\"analysis\": {{\"result\": true}}}}';;\nesac\n",
                dir.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let l = Lola { program: fake.display().to_string(), ndrio: "ndrio".into() };
        let run = |q: Query, ts: &[&str], ps: &[(&str, u32)]| {
            let _ = std::fs::remove_file(dir.join("calls"));
            let places: Vec<(String, u32)> = ps.iter().map(|(p, m)| ((*p).to_owned(), *m)).collect();
            let trans: Vec<String> = ts.iter().map(|t| (*t).to_owned()).collect();
            let c = l.build(q, "N.lola", &places, &trans, None);
            let o = c.output();
            let out = String::from_utf8_lossy(&o.stdout).into_owned();
            let calls = std::fs::read_to_string(dir.join("calls")).map_or(0, |t| t.lines().count());
            let f = Finished { spawn_error: None, timed_out: false, memout: false, exit_code: o.status.code(), signal: None, stdout: &out, stderr: &String::from_utf8_lossy(&o.stderr) };
            (l.classify_for(q, &f), calls)
        };
        // Q4: stops at the first false (t1): t0 true, t1 false, t0b never run
        let (p, calls) = run(Query::Q4, &["t0", "t1", "t0b"], &[]);
        assert_eq!((p.verdict, calls), (Verdict::False, 2));
        // all true
        let (p, calls) = run(Query::Q4, &["t0", "t0b"], &[]);
        assert_eq!((p.verdict, calls), (Verdict::True, 2));
        // a null result without a later false: unknown
        let (p, _) = run(Query::Q4, &["t0", "t2"], &[]);
        assert_eq!(p.verdict, Verdict::Unknown);
        // a null result followed by a false: still definite
        let (p, _) = run(Query::Q4, &["t2", "t1"], &[]);
        assert_eq!(p.verdict, Verdict::False);
        // first nonzero exit code propagates: exit 1 is INPUT_REJECTED with the lola line, exit 2 UNKNOWN
        let (p, calls) = run(Query::Q4, &["t0", "t3", "t0b"], &[]);
        assert_eq!((p.verdict, p.detail.as_deref(), calls), (Verdict::InputRejected, Some("lola: syntax error"), 2));
        let (p, _) = run(Query::Q4, &["t4"], &[]);
        assert_eq!(p.verdict, Verdict::Unknown);
        // Q6: stops at the first true; all false is false
        let (p, calls) = run(Query::Q6, &[], &[("p1", 0), ("pa", 1), ("pb", 0)]);
        assert_eq!((p.verdict, calls), (Verdict::True, 2));
        let (p, calls) = run(Query::Q6, &[], &[("p1", 0), ("p1x", 1)]);
        assert_eq!((p.verdict, calls), (Verdict::False, 2));
    }

    #[test]
    fn parses_captured_outputs() {
        let l = lola();
        let p = l.parse_q(Query::Q0, &tool("lola_q0_cyc.json"));
        assert_eq!((p.verdict, p.states, p.edges), (Verdict::Count, Some(2), Some(2)));
        let p = l.parse_q(Query::Q0, &tool("lola_q0_dead.json"));
        assert_eq!((p.verdict, p.states, p.edges), (Verdict::Count, Some(1), Some(0)));
        // Q1 is inverted: no deadlock reachable (result false) means deadlock-free
        assert_eq!(l.parse_q(Query::Q1, &tool("lola_deadlock_cyc.json")).verdict, Verdict::True);
        assert_eq!(l.parse_q(Query::Q1, &tool("lola_deadlock_dead.json")).verdict, Verdict::False);
        // AGEF FIREABLE reads straight
        assert_eq!(l.parse_q(Query::Q3, &tool("lola_agef_t0_cyc.json")).verdict, Verdict::True);
        assert_eq!(l.parse_q(Query::Q3, &tool("lola_agef_t0_dead.json")).verdict, Verdict::False);
        // JSON without an analysis key and exit 0 is unknown
        let p = l.parse_q(Query::Q3, &tool("lola_syntax_error.rc1.stdout"));
        assert_eq!(p.verdict, Verdict::Unknown);
        assert!(p.detail.unwrap().contains("null or missing"));
        assert_eq!(l.parse_q(Query::Q3, r#"{"analysis": {"result": null}}"#).verdict, Verdict::Unknown);
        assert_eq!(l.parse_q(Query::Q3, "garbage").detail.as_deref(), Some("unparseable output"));
    }

    fn fin<'a>(code: i32, stdout: &'a str, stderr: &'a str) -> Finished<'a> {
        Finished { spawn_error: None, timed_out: false, memout: false, exit_code: Some(code), signal: None, stdout, stderr }
    }

    #[test]
    fn classification() {
        let l = lola();
        let p = l.classify_for(Query::Q1, &fin(0, &tool("lola_deadlock_dead.json"), ""));
        assert_eq!(p.verdict, Verdict::False);
        // exit 1: the `lola:` line, not the echoed net line before it
        let p = l.classify_for(Query::Q1, &fin(1, &tool("lola_syntax_error.rc1.stdout"), &tool("lola_syntax_error.rc1.stderr")));
        assert_eq!((p.verdict, p.detail.as_deref()), (Verdict::InputRejected, Some("lola: syntax error -- aborting [#01]")));
        let p = l.classify_for(Query::Q1, &fin(2, "", ""));
        assert_eq!((p.verdict, p.detail.as_deref()), (Verdict::Unknown, Some("premature termination (exit 2)")));
        // a shell job reporting a signal of a sub-run
        let p = l.classify_for(Query::Q4, &fin(139, "", ""));
        assert_eq!((p.verdict, p.detail.as_deref()), (Verdict::Crash, Some("signal:SIGSEGV")));
        assert_eq!(l.classify_for(Query::Q4, &fin(1, "", "")).verdict, Verdict::Crash);
    }

    /// The whole job path (conversion, command, classification) with fake `ndrio` and `lola` scripts.
    #[test]
    fn job_machinery_with_fake_tools() {
        use crate::adapters::Adapter;
        use crate::instance::Instance;
        use crate::run::{Ctx, Job, Unit};
        use crate::runner::Limiter;
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("pe-lola-job-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mk = |name: &str, body: &str| {
            let p = dir.join(name);
            std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
            p.display().to_string()
        };
        let tools = format!("{}/testdata/tools", env!("CARGO_MANIFEST_DIR"));
        let cal = concat!(env!("CARGO_MANIFEST_DIR"), "/../calibration");
        std::fs::copy(format!("{cal}/cal-01-circuit-live.pnml"), dir.join("good.pnml")).unwrap();
        std::fs::write(dir.join("bad.pnml"), "<<<").unwrap();
        // ndrio: fails on bad.pnml like the real one (stderr line, exit 1, empty output), copies otherwise
        let ndrio = mk("ndrio", "case \"$3\" in *bad*) echo 'unexpected error in pnml parser (XML declaration not well-formed @1:14)' >&2; : > \"$4\"; exit 1;; *) cp \"$3\" \"$4\";; esac");
        let lola = mk("lola", &format!("case \"$*\" in *check=full*) cat {tools}/lola_q0_cyc.json;; *DEADLOCK*) cat {tools}/lola_deadlock_cyc.json;; *) exit 2;; esac"));
        let adapter: Box<dyn Adapter> = Box::new(Lola { program: lola, ndrio });
        let mkinst = |id: &str, path: &str| Instance { instance_id: id.into(), path: path.into(), source: "gen".into(), base: dir.clone(), ..Instance::default() };
        let insts = vec![mkinst("good", "good.pnml"), mkinst("bad", "bad.pnml")];
        let adapters = vec![adapter];
        let mut units = Vec::new();
        for i in 0..2 {
            for q in [Query::Q0, Query::Q1, Query::Q3] {
                units.push(Unit { inst: i, query: q, target: None, solvers: vec![0] });
            }
        }
        let ctx = Ctx {
            campaign: "T", instances: &insts, units: &units, adapters: &adapters, results_dir: &dir, env_id: "e",
            wall: std::time::Duration::from_secs(20), mem_kib: 1 << 20, out_cap: 65536, limiter: Limiter::None, cores: &[],
        };
        let rec = |u: usize| ctx.execute(&Job { unit: u, solver: 0, rep: 0 }, 0);
        let r = rec(0);
        assert_eq!((r.verdict, r.states, r.edges), (Verdict::Count, Some(2), Some(2)));
        assert!(r.convert_ms.is_some());
        assert!(r.cmd.contains("good.lola") && r.cmd.contains("--check=full --stubborn=off --quiet --json=/dev/stdout"), "{}", r.cmd);
        assert_eq!(rec(1).verdict, Verdict::True); // EF DEADLOCK false, inverted
        let r = rec(2); // fake lola exits 2 for the combined liveness formula: premature termination
        assert_eq!((r.verdict, r.detail.as_deref()), (Verdict::Unknown, Some("premature termination (exit 2)")));
        // conversion failure: INPUT_REJECTED, nothing run, the run goes on
        let r = rec(3);
        assert_eq!(r.verdict, Verdict::InputRejected);
        assert_eq!(r.detail.as_deref(), Some("conversion failed: unexpected error in pnml parser (XML declaration not well-formed @1:14)"));
        assert!(r.convert_ms.is_some() && r.exit_code.is_none());
        assert_eq!(rec(4).verdict, Verdict::InputRejected);
    }

    /// End to end with the real tools; run with `--ignored` where `lola` and `ndrio` are on PATH.
    #[test]
    #[ignore]
    fn real_lola_on_a_ring() {
        use super::super::convert;
        let dir = std::env::temp_dir().join(format!("pe-lola-real-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/../calibration/cal-01-circuit-live.pnml");
        let a = Lola::new(&LolaCfg::default(), &NdrioCfg::default());
        let c = convert::ensure_lola(&a.ndrio, std::path::Path::new(src), &dir, "ring").expect("ndrio conversion");
        let expect = [(Query::Q0, Verdict::Count), (Query::Q1, Verdict::True), (Query::Q2, Verdict::True), (Query::Q3, Verdict::True), (Query::Q4, Verdict::True), (Query::Q5, Verdict::True), (Query::Q6, Verdict::False)];
        for (q, v) in expect {
            let req = Request { instance_path: std::path::Path::new(src), query: q, target_file: None, internal_timeout_ms: None, converted: Some(&c.path) };
            let spec = a.command(&req);
            let o = std::process::Command::new(&spec.program).args(&spec.args).output().unwrap();
            let (out, err) = (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned());
            let p = a.classify_for(q, &fin(o.status.code().unwrap_or(-1), &out, &err));
            assert_eq!(p.verdict, v, "{q:?}: {out} {err}");
        }
    }
}
