//! The TINA 4.0.0 adapter (`tina`): programs `tina`, `sift` and `scan` (PATH or eval.toml; `TINA_BIN`, `SIFT_BIN`,
//! `SCAN_BIN` override). TINA reads the PNML directly, no conversion.
//!
//! | Query | Command | Reading |
//! |---|---|---|
//! | Q0 | `tina -R -q N.pnml` | `N marking(s), M transition(s)` (states, edges), only if the report says `bounded` |
//! | Q1 | same | `N dead marking(s)`: N > 0 means a deadlock exists (Q1 false) |
//! | Q2 | same | the `bounded` / `not bounded` line |
//! | Q3 | same | the `live` / `not live` line (bounded nets only) |
//! | Q4 | same | `N dead transition(s)`: 0 means quasi-live |
//! | Q5 | `sift -R -f safe -q N.pnml` | `some state violates condition` means not 1-safe |
//! | Q6 | script: `sift -R -k N.pnml N.ktz`, then per place `scan N.ktz -n "-(p = m0)" -b` (TRUE iff no state differs), stops at the first TRUE | |
//! | QR | script: `sift -R -rs -k N.pnml N.ktz`, then `scan N.ktz -r "p1 = a1 /\ ..." -b` over ALL places | `TRUE` / `FALSE` |
//! | QC | same with `p >= a` over the places with a > 0; all-zero target: trivially true | |
//!
//! Scripts are one `sh -c` job; the ktz file lives in a per-job temporary directory created and removed inside the
//! same timed command. Exit 1 is INPUT_REJECTED; every TINA tool exits 0 on completed runs.
//!
//! UNVERIFIED: settled by calibrate on the box: Q2 on unbounded nets, Q5 (`sift -f safe`), Q6 (`scan -n ... -b` and
//! the `sift -k` profile), QC (`>=` in `scan -r`), place names needing braces, and the report of unbounded nets.

use super::{
    Adapter, CommandSpec, Finished, Parsed, Query, Request, TinaCfg, ToolInfo, answer_parsed, classify_with, env_or, first_line, script_answer,
    sh_quote, shell_signal_exit,
};
use super::convert::places_with_marking;
use crate::record::Verdict;
use std::collections::BTreeMap;

pub struct Tina {
    tina: String,
    sift: String,
    scan: String,
}

impl Tina {
    pub fn new(cfg: &TinaCfg) -> Self {
        Tina { tina: env_or("TINA_BIN", &cfg.tina), sift: env_or("SIFT_BIN", &cfg.sift), scan: env_or("SCAN_BIN", &cfg.scan) }
    }

    /// The command of a query on the PNML `net`. `places` are `(id, m0)` in document order, `target` the place-id map
    /// of a QR/QC target (zero entries may be missing).
    pub fn build(&self, query: Query, net: &str, places: &[(String, u32)], target: Option<&BTreeMap<String, u32>>) -> CommandSpec {
        let empty = BTreeMap::new();
        let t = target.unwrap_or(&empty);
        match query {
            Query::Q0 | Query::Q1 | Query::Q2 | Query::Q3 | Query::Q4 => CommandSpec::new(self.tina.as_str(), ["-R", "-q", net]),
            // UNVERIFIED: settled by calibrate on the box (the atom `safe` and the violation line)
            Query::Q5 => CommandSpec::new(self.sift.as_str(), ["-R", "-f", "safe", "-q", net]),
            Query::Q6 => {
                // verified on the box: `scan -n F -b` is TRUE iff NO state satisfies F, so the stable-place test is the
                // negation `-(p = m0)`; the unnegated formula (the first version) was TRUE iff p never holds m0
                let checks = places.iter().map(|(p, m0)| ("-n", format!("-({} = {m0})", place_term(p)))).collect();
                self.script(net, &["-R", "-k"], checks, true)
            }
            Query::QR => {
                if places.is_empty() {
                    return constant("true");
                }
                let f = places.iter().map(|(p, _)| format!("{} = {}", place_term(p), t.get(p).copied().unwrap_or(0))).collect::<Vec<_>>().join(" /\\ ");
                self.script(net, &["-R", "-rs", "-k"], vec![("-r", f)], false)
            }
            Query::QC => {
                // UNVERIFIED: settled by calibrate on the box (`>=` in a scan -r formula)
                let parts: Vec<String> =
                    places.iter().filter_map(|(p, _)| t.get(p).copied().filter(|a| *a > 0).map(|a| format!("{} >= {a}", place_term(p)))).collect();
                if parts.is_empty() {
                    return constant("true"); // the all-zero target is covered by every marking
                }
                self.script(net, &["-R", "-rs", "-k"], vec![("-r", parts.join(" /\\ "))], false)
            }
        }
    }

    /// `sift <flags> N.pnml N.ktz`, then `scan N.ktz <mode> <formula> -b` per check. `search_true`: Q6 style, the first
    /// TRUE ends the script with `true` and the final answer is false (unknown if a check gave neither TRUE nor FALSE);
    /// otherwise the single check decides.
    fn script(&self, net: &str, sift_flags: &[&str], checks: Vec<(&str, String)>, search_true: bool) -> CommandSpec {
        let flags = sift_flags.join(" ");
        let mut s = format!("sift={}; scan={}; net={}; unk=0\n", sh_quote(&self.sift), sh_quote(&self.scan), sh_quote(net));
        s.push_str("d=$(mktemp -d) || { echo 'mktemp failed' >&2; exit 70; }\ntrap 'rm -rf \"$d\"' EXIT\nk=\"$d/net.ktz\"\n");
        s.push_str(&format!("\"$sift\" {flags} \"$net\" \"$k\" </dev/null; rc=$?\n[ \"$rc\" -eq 0 ] || exit \"$rc\"\n"));
        s.push_str(
            "chk() {\n  out=$(\"$scan\" \"$k\" \"$1\" \"$2\" -b </dev/null); rc=$?\n  printf '%s\\n' \"$out\"\n  [ \"$rc\" -eq 0 ] || exit \"$rc\"\n  if printf '%s\\n' \"$out\" | grep -qx TRUE; then res=true\n  elif printf '%s\\n' \"$out\" | grep -qx FALSE; then res=false\n  else res=other; fi\n}\n",
        );
        if search_true {
            for (mode, f) in checks {
                s.push_str(&format!("chk {mode} {}\n", sh_quote(&f)));
                s.push_str("[ \"$res\" = true ] && { echo 'PETRIVET_ANSWER true'; exit 0; }\n[ \"$res\" = other ] && unk=1\n");
            }
            s.push_str("if [ \"$unk\" -eq 1 ]; then echo 'PETRIVET_ANSWER unknown'; else echo 'PETRIVET_ANSWER false'; fi\n");
        } else {
            for (mode, f) in checks {
                s.push_str(&format!("chk {mode} {}\n", sh_quote(&f)));
            }
            s.push_str("echo \"PETRIVET_ANSWER $res\"\n");
        }
        CommandSpec::sh_script(s)
    }

    pub fn parse_q(&self, query: Query, stdout: &str) -> Parsed {
        if let Some(word) = script_answer(stdout) {
            return answer_parsed(word);
        }
        let r = Report::parse(stdout);
        let unk = |d: String| Parsed::with_detail(Verdict::Unknown, d);
        let verdict = |b: bool| Parsed::verdict(if b { Verdict::True } else { Verdict::False });
        match query {
            Query::Q0 => match (r.bounded.as_deref(), r.counts) {
                (Some("bounded"), Some((s, e))) => Parsed { states: Some(s), edges: Some(e), ..Parsed::verdict(Verdict::Count) },
                (Some("bounded"), None) => unk("no marking(s) line in the report".into()),
                (Some(other), _) => unk(format!("report says {other:?}")),
                (None, _) => unk("no boundedness line in the report".into()),
            },
            Query::Q1 => match r.dead_markings {
                Some(n) => verdict(n == 0),
                None => unk("no dead marking(s) line in the report".into()),
            },
            Query::Q2 => match r.bounded.as_deref() {
                Some("bounded") => verdict(true),
                Some("not bounded") => verdict(false),
                // UNVERIFIED: settled by calibrate on the box (what `tina -R -q` prints for unbounded nets)
                Some(other) => unk(format!("report says {other:?}")),
                None => unk("no boundedness line in the report".into()),
            },
            Query::Q3 => {
                if r.bounded.as_deref().is_some_and(|b| b != "bounded") {
                    return unk(format!("report says {:?}", r.bounded.unwrap_or_default()));
                }
                match r.live.as_deref() {
                    Some("live") => verdict(true),
                    Some("not live") => verdict(false),
                    Some(other) => unk(format!("report says {other:?}")),
                    None => unk("no liveness line in the report".into()),
                }
            }
            Query::Q4 => match r.dead_transitions {
                Some(n) => verdict(n == 0),
                None => unk("no dead transition(s) line in the report".into()),
            },
            Query::Q5 => {
                if stdout.lines().any(|l| l.trim().starts_with("some state violates condition")) {
                    verdict(false)
                } else if r.counts.is_some() {
                    verdict(true)
                } else {
                    unk(format!("sift output not recognised: {}", first_line(stdout)))
                }
            }
            // scripts always carry an answer line
            Query::Q6 | Query::QR | Query::QC => Parsed::runner_error("unparseable output"),
        }
    }

    /// Exit 1: INPUT_REJECTED with the first diagnostic line (default rule); above 128: a signal inside a script.
    pub fn nonzero(&self, stdout: &str, stderr: &str, code: i32) -> Parsed {
        if let Some(p) = shell_signal_exit(code) {
            return p;
        }
        let diag = if stderr.trim().is_empty() { stdout } else { stderr };
        if diag.trim().is_empty() { Parsed::crash(format!("exit:{code}")) } else { Parsed::input_rejected(diag) }
    }
}

/// Names that need no braces in a selt formula; others are written `{name}`.
/// UNVERIFIED: settled by calibrate on the box (the brace syntax; calibration nets use plain `p0`, `p1`, ...).
fn place_term(name: &str) -> String {
    let simple = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if simple { name.to_owned() } else { format!("{{{name}}}") }
}

fn constant(answer: &str) -> CommandSpec {
    CommandSpec::new("sh", ["-c".to_owned(), format!("echo 'PETRIVET_ANSWER {answer}'")])
}

/// The text report of `tina -R -q` (lines of the digest at the end start with `#` and are ignored).
#[derive(Debug, Default, PartialEq)]
pub struct Report {
    /// The raw boundedness line: `bounded`, `not bounded`, or another line that mentions boundedness.
    pub bounded: Option<String>,
    /// `(markings, transitions)` of the marking graph.
    pub counts: Option<(u64, u64)>,
    /// The raw liveness line: `live`, `not live`, `possibly live` or a line saying the analysis does not apply.
    pub live: Option<String>,
    pub dead_markings: Option<u64>,
    pub dead_transitions: Option<u64>,
}

impl Report {
    pub fn parse(text: &str) -> Report {
        let mut r = Report::default();
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            if line == "bounded" || line == "not bounded" {
                r.bounded = Some(line.to_owned());
            } else if line == "net unbounded" {
                // what `tina -R -q` prints for an unbounded net (verified on the box), followed by `unbounded places include: ...`
                r.bounded = Some("not bounded".to_owned());
            } else if line.contains("bounded") {
                r.bounded.get_or_insert_with(|| line.to_owned());
            } else if line == "live" || line == "not live" || line == "possibly live" || line.contains("not applicable") {
                r.live.get_or_insert_with(|| line.to_owned());
            } else if let Some((n, rest)) = line.split_once(" dead marking(s), ") {
                if rest.contains("live marking(s)") {
                    r.dead_markings.get_or_insert(n.parse().unwrap_or(u64::MAX));
                }
            } else if let Some((n, rest)) = line.split_once(" dead transition(s), ") {
                if rest.contains("live transition(s)") {
                    r.dead_transitions.get_or_insert(n.parse().unwrap_or(u64::MAX));
                }
            } else if let Some((m, rest)) = line.split_once(" marking(s), ")
                && let (Ok(m), Some(t)) = (m.parse::<u64>(), rest.strip_suffix(" transition(s)").and_then(|t| t.parse::<u64>().ok()))
            {
                r.counts.get_or_insert((m, t));
            }
        }
        r
    }
}

impl Adapter for Tina {
    fn solver_id(&self) -> &str {
        "tina"
    }

    fn supports(&self, _query: Query) -> bool {
        true
    }

    fn command(&self, req: &Request) -> CommandSpec {
        let net = req.instance_path.display().to_string();
        let failing = |msg: String| CommandSpec::new("sh", ["-c".to_owned(), format!("echo {} >&2; exit 1", sh_quote(&msg))]);
        let mut places = Vec::new();
        if matches!(req.query, Query::Q6 | Query::QR | Query::QC) {
            match std::fs::read_to_string(req.instance_path) {
                Ok(t) => places = places_with_marking(&t),
                Err(e) => return failing(format!("cannot read {}: {e}", req.instance_path.display())),
            }
        }
        let target = match req.target_file {
            Some(f) if req.query.needs_target() => match std::fs::read_to_string(f).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str::<BTreeMap<String, u32>>(&t).map_err(|e| e.to_string())) {
                Ok(m) => Some(m),
                Err(e) => return failing(format!("cannot read target {}: {e}", f.display())),
            },
            _ => None,
        };
        self.build(req.query, &net, &places, target.as_ref())
    }

    fn parse(&self, stdout: &str, _exit_code: i32) -> Parsed {
        self.parse_q(Query::Q2, stdout)
    }

    fn nonzero_exit(&self, stdout: &str, stderr: &str, exit_code: i32) -> Parsed {
        self.nonzero(stdout, stderr, exit_code)
    }

    fn classify_for(&self, query: Query, f: &Finished) -> Parsed {
        classify_with(f, |out| self.parse_q(query, out), |o, e, c| self.nonzero(o, e, c))
    }

    fn tool(&self) -> Option<ToolInfo> {
        Some(ToolInfo { program: self.tina.clone(), version_args: None })
    }

    fn programs(&self) -> Vec<String> {
        vec![self.tina.clone(), self.sift.clone(), self.scan.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tina() -> Tina {
        Tina { tina: "tina".into(), sift: "sift".into(), scan: "scan".into() }
    }

    fn tool(name: &str) -> String {
        std::fs::read_to_string(format!("{}/testdata/tools/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn places() -> Vec<(String, u32)> {
        vec![("p0".into(), 1), ("p1".into(), 0), ("p2".into(), 2)]
    }

    #[test]
    fn report_parsing() {
        let r = Report::parse(&tool("tina_R_cyc.txt"));
        assert_eq!(
            r,
            Report { bounded: Some("bounded".into()), counts: Some((2, 2)), live: Some("live".into()), dead_markings: Some(0), dead_transitions: Some(0) }
        );
        let r = Report::parse(&tool("tina_R_dead.txt"));
        assert_eq!((r.live.as_deref(), r.dead_markings, r.dead_transitions, r.counts), (Some("not live"), Some(1), Some(2), Some((1, 0))));
        let r = Report::parse(&tool("tina_R_anderson_excerpt.txt"));
        assert_eq!((r.bounded.as_deref(), r.counts, r.live.as_deref(), r.dead_markings, r.dead_transitions), (Some("bounded"), Some((29641, 97516)), Some("live"), Some(0), Some(0)));
    }

    #[test]
    fn verdicts_from_captured_reports() {
        let t = tina();
        let cyc = tool("tina_R_cyc.txt");
        let dead = tool("tina_R_dead.txt");
        let anderson = tool("tina_R_anderson_excerpt.txt");
        let p = t.parse_q(Query::Q0, &cyc);
        assert_eq!((p.verdict, p.states, p.edges), (Verdict::Count, Some(2), Some(2)));
        let p = t.parse_q(Query::Q0, &anderson);
        assert_eq!((p.verdict, p.states, p.edges), (Verdict::Count, Some(29641), Some(97516)));
        let p = t.parse_q(Query::Q0, &dead);
        assert_eq!((p.states, p.edges), (Some(1), Some(0)));
        let v = |q, text: &str| t.parse_q(q, text).verdict;
        assert_eq!((v(Query::Q1, &cyc), v(Query::Q1, &dead), v(Query::Q1, &anderson)), (Verdict::True, Verdict::False, Verdict::True));
        assert_eq!((v(Query::Q2, &cyc), v(Query::Q2, &dead)), (Verdict::True, Verdict::True));
        assert_eq!((v(Query::Q3, &cyc), v(Query::Q3, &dead), v(Query::Q3, &anderson)), (Verdict::True, Verdict::False, Verdict::True));
        assert_eq!((v(Query::Q4, &cyc), v(Query::Q4, &dead), v(Query::Q4, &anderson)), (Verdict::True, Verdict::False, Verdict::True));
        // sift -R -f safe: the violation line means not 1-safe (here the deadlock capture has the same layout)
        assert_eq!(v(Query::Q5, &tool("sift_dead_cyc.txt")), Verdict::True);
        assert_eq!(v(Query::Q5, &tool("sift_dead_dead.txt")), Verdict::False);
        assert_eq!(v(Query::Q5, "garbage"), Verdict::Unknown);
        // scan answers come through the script answer line
        assert_eq!(v(Query::QR, "PETRIVET_ANSWER true\n"), Verdict::True);
        assert_eq!(v(Query::QR, "TRUE\nPETRIVET_ANSWER false\n"), Verdict::False);
        assert_eq!(v(Query::Q6, "PETRIVET_ANSWER other\n"), Verdict::Unknown);
    }

    /// `tina -R -q` on an unbounded net (captured on the box): `net unbounded`, no liveness verdict.
    #[test]
    fn unbounded_net_report_from_the_box() {
        let t = tina();
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/tools/tina_R_unbounded_cal11.txt")).unwrap();
        assert_eq!(t.parse_q(Query::Q2, &text).verdict, Verdict::False);
        for q in [Query::Q0, Query::Q3] {
            assert_eq!(t.parse_q(q, &text).verdict, Verdict::Unknown, "{q:?}");
        }
    }

    #[test]
    fn unbounded_and_odd_reports_are_unknown() {
        let t = tina();
        let unb = "parsed net {}\n\nnot bounded\n\n# net {}\n";
        assert_eq!(t.parse_q(Query::Q2, unb).verdict, Verdict::False);
        for q in [Query::Q0, Query::Q3] {
            let p = t.parse_q(q, unb);
            assert_eq!(p.verdict, Verdict::Unknown, "{q:?}");
        }
        assert_eq!(t.parse_q(Query::Q1, unb).verdict, Verdict::Unknown);
        let weird = "bounded\n\npossibly live\n1 marking(s), 0 transition(s)\n";
        let p = t.parse_q(Query::Q3, weird);
        assert_eq!((p.verdict, p.detail.as_deref()), (Verdict::Unknown, Some("report says \"possibly live\"")));
        let p = t.parse_q(Query::Q2, "unbounded places include: p1\n");
        assert_eq!((p.verdict, p.detail.as_deref()), (Verdict::Unknown, Some("report says \"unbounded places include: p1\"")));
        assert_eq!(t.parse_q(Query::Q3, "bounded\nnot applicable, net is unbounded\n").verdict, Verdict::Unknown);
    }

    #[test]
    fn commands() {
        let t = tina();
        let args = |q| t.build(q, "N.pnml", &places(), None);
        for q in [Query::Q0, Query::Q1, Query::Q2, Query::Q3, Query::Q4] {
            let c = args(q);
            assert_eq!((c.program.as_str(), c.args.as_slice()), ("tina", ["-R", "-q", "N.pnml"].map(String::from).as_slice()), "{q:?}");
        }
        let c = args(Query::Q5);
        assert_eq!((c.program.as_str(), c.args), ("sift", ["-R", "-f", "safe", "-q", "N.pnml"].map(String::from).to_vec()));
        // Q6: sift -k, one scan -n per place, early exit on the first TRUE
        let c = args(Query::Q6);
        assert_eq!(c.program, "sh");
        let s = c.script_text();
        assert!(s.contains("\"$sift\" -R -k \"$net\" \"$k\" </dev/null; rc=$?"));
        assert!(s.contains("d=$(mktemp -d)") && s.contains("trap 'rm -rf \"$d\"' EXIT"));
        assert!(s.contains("chk -n '-(p0 = 1)'\n[ \"$res\" = true ] && { echo 'PETRIVET_ANSWER true'; exit 0; }"));
        assert!(s.contains("chk -n '-(p1 = 0)'\n") && s.contains("chk -n '-(p2 = 2)'\n"));
        assert!(s.contains("[ \"$rc\" -eq 0 ] || exit \"$rc\""));
        assert!(s.ends_with("else echo 'PETRIVET_ANSWER false'; fi\n"));
        // QR: -rs profile, every place including zeros
        let tgt: BTreeMap<String, u32> = [("p2".to_owned(), 3)].into();
        let c = t.build(Query::QR, "N.pnml", &places(), Some(&tgt));
        let s = c.script_text();
        assert!(s.contains("\"$sift\" -R -rs -k \"$net\" \"$k\""));
        assert!(s.contains("chk -r 'p0 = 0 /\\ p1 = 0 /\\ p2 = 3'\n"));
        assert!(s.ends_with("echo \"PETRIVET_ANSWER $res\"\n"));
        // QC: places with a positive entry, >=
        let c = t.build(Query::QC, "N.pnml", &places(), Some(&tgt));
        assert!(c.script_text().contains("chk -r 'p2 >= 3'\n"));
        // zero target: trivially true
        let zero = BTreeMap::new();
        let c = t.build(Query::QC, "N.pnml", &places(), Some(&zero));
        assert_eq!(c.script_text(), "echo 'PETRIVET_ANSWER true'");
    }

    #[test]
    fn odd_place_names_get_braces() {
        assert_eq!(place_term("p_1"), "p_1");
        assert_eq!(place_term("a-b"), "{a-b}");
        assert_eq!(place_term("1x"), "{1x}");
    }

    fn fin<'a>(code: i32, stdout: &'a str, stderr: &'a str) -> Finished<'a> {
        Finished { spawn_error: None, timed_out: false, memout: false, exit_code: Some(code), signal: None, stdout, stderr }
    }

    #[test]
    fn classification() {
        let t = tina();
        let p = t.classify_for(Query::Q1, &fin(1, "unexpected error in pnml parser (XML declaration not well-formed @1:14)\n", ""));
        assert_eq!((p.verdict, p.detail.as_deref()), (Verdict::InputRejected, Some("unexpected error in pnml parser (XML declaration not well-formed @1:14)")));
        let p = t.classify_for(Query::Q1, &fin(1, "", &tool("tina_bad_xml.rc1.stderr")));
        assert_eq!(p.verdict, Verdict::InputRejected);
        assert_eq!(t.classify_for(Query::Q1, &fin(0, &tool("tina_R_dead.txt"), "")).verdict, Verdict::False);
        assert_eq!(t.classify_for(Query::QR, &fin(139, "", "")).detail.as_deref(), Some("signal:SIGSEGV"));
    }

    /// Runs the generated QR/Q6 scripts against fake `sift` and `scan` shell scripts serving the captured outputs.
    #[test]
    fn scripts_run_with_fake_tools() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("pe-tina-fake-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tools = format!("{}/testdata/tools", env!("CARGO_MANIFEST_DIR"));
        let mk = |name: &str, body: &str| {
            let p = dir.join(name);
            std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
            p.display().to_string()
        };
        let sift = mk("sift", &format!("cat {tools}/sift_rs_cyc.txt; for last; do :; done; : > \"$last\"; echo \"$@\" >> {}/sift_calls", dir.display()));
        // scan: TRUE when the formula mentions p1 = 1, FALSE otherwise; logs the calls
        let scan = mk("scan", &format!("echo \"$3\" >> {}/scan_calls\ncase \"$3\" in *'p1 = 1'*) echo TRUE;; *'p7'*) echo 'what';; *) echo FALSE;; esac", dir.display()));
        let t = Tina { tina: "tina".into(), sift, scan };
        let run = |q: Query, ps: &[(&str, u32)], tgt: Option<&BTreeMap<String, u32>>| {
            let _ = std::fs::remove_file(dir.join("scan_calls"));
            let places: Vec<(String, u32)> = ps.iter().map(|(p, m)| ((*p).to_owned(), *m)).collect();
            let c = t.build(q, "N.pnml", &places, tgt);
            let o = c.output();
            let out = String::from_utf8_lossy(&o.stdout).into_owned();
            let calls = std::fs::read_to_string(dir.join("scan_calls")).map_or(0, |t| t.lines().count());
            let err = String::from_utf8_lossy(&o.stderr).into_owned();
            (t.classify_for(q, &fin(o.status.code().unwrap_or(-1), &out, &err)).verdict, calls)
        };
        let tgt: BTreeMap<String, u32> = [("p1".to_owned(), 1)].into();
        // QR over all places: formula contains 'p1 = 1' -> TRUE
        assert_eq!(run(Query::QR, &[("p0", 0), ("p1", 1)], Some(&tgt)), (Verdict::True, 1));
        let tgt0: BTreeMap<String, u32> = [("p0".to_owned(), 1)].into();
        assert_eq!(run(Query::QR, &[("p0", 0), ("p1", 1)], Some(&tgt0)), (Verdict::False, 1));
        // Q6: p0 = 0 false, then p1 = 1 TRUE: stops after two scans
        assert_eq!(run(Query::Q6, &[("p0", 0), ("p1", 1), ("p2", 5)], None), (Verdict::True, 2));
        // all FALSE: false after every place
        assert_eq!(run(Query::Q6, &[("p0", 0), ("p2", 5)], None), (Verdict::False, 2));
        // an unrecognised scan output: unknown
        assert_eq!(run(Query::Q6, &[("p7", 0)], None), (Verdict::Unknown, 1));
        // the temporary ktz directory is gone: sift got a path under a directory that no longer exists
        let calls = std::fs::read_to_string(dir.join("sift_calls")).unwrap();
        let ktz = calls.lines().next().unwrap().split_whitespace().last().unwrap().to_owned();
        assert!(!std::path::Path::new(&ktz).exists() && !std::path::Path::new(&ktz).parent().unwrap().exists());
    }

    /// End to end with the real tools; run with `--ignored` where `tina`, `sift` and `scan` are on PATH.
    #[test]
    #[ignore]
    fn real_tina_on_a_ring() {
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/../calibration/cal-01-circuit-live.pnml");
        let a = Tina::new(&TinaCfg::default());
        let target = std::env::temp_dir().join(format!("pe-tina-target-{}.json", std::process::id()));
        std::fs::write(&target, r#"{"p1":1}"#).unwrap();
        let expect = [(Query::Q0, Verdict::Count), (Query::Q1, Verdict::True), (Query::Q2, Verdict::True), (Query::Q3, Verdict::True), (Query::Q4, Verdict::True), (Query::Q5, Verdict::True), (Query::Q6, Verdict::False), (Query::QR, Verdict::True), (Query::QC, Verdict::True)];
        for (q, v) in expect {
            let req = Request { instance_path: std::path::Path::new(src), query: q, target_file: q.needs_target().then_some(target.as_path()), internal_timeout_ms: None, converted: None };
            let spec = a.command(&req);
            let o = spec.output();
            let (out, err) = (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned());
            let p = a.classify_for(q, &fin(o.status.code().unwrap_or(-1), &out, &err));
            assert_eq!(p.verdict, v, "{q:?}: {out} {err}");
        }
    }
}
