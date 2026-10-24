//! ITS-Tools (LIP6), a symbolic model checker (hierarchical set decision diagrams), run through its contest interface.
//!
//! One call per question, with the examinations of the Model Checking Contest:
//!
//! | query | examination | output line | answer |
//! |---|---|---|---|
//! | Q0 | `StateSpace` | `STATE_SPACE STATES n TECHNIQUES ...` | COUNT with `n` states (no transition count: the tool prints none) |
//! | Q1 | `ReachabilityDeadlock` | `FORMULA ReachabilityDeadlock TRUE\|FALSE ...` | inverted: TRUE means a deadlock is reachable |
//! | Q4 | `QuasiLiveness` | `FORMULA QuasiLiveness TRUE\|FALSE ...` | as is |
//! | Q5 | `OneSafe` | `FORMULA OneSafe TRUE\|FALSE ...` | as is |
//! | Q6 | `StableMarking` | `FORMULA StableMarking TRUE\|FALSE ...` | as is |
//!
//! Not supported: Q2 (the contest has no boundedness question), Q3 (liveness: the tool spends a fixed 30 s in a random
//! walk on every net it cannot refute at once, whatever `-timeout` says, which exceeds the 30 s cap of the campaigns),
//! QR and QC (they need hand-written contest property files).
//!
//! The tool wants a folder that holds `model.pnml`. Every call makes a temporary folder with a link to the net and
//! removes it at the end; the call is one `sh -s` job (script on standard input). `-timeout 3600` is the contest
//! setting and only steers the tool's internal strategies: the wall cap of the runner ends the run, as for every tool.
//! The program is the `its-tools` wrapper of `nix/its-tools.nix` (Java, heap, z3, a no-op `killall`).

use super::{Adapter, CommandSpec, Finished, ItsCfg, Parsed, Query, Request, ToolInfo, classify_with, env_or, first_line, sh_quote};
use crate::record::Verdict;

pub struct ItsTools {
    program: String,
}

fn examination(query: Query) -> Option<&'static str> {
    Some(match query {
        Query::Q0 => "StateSpace",
        Query::Q1 => "ReachabilityDeadlock",
        Query::Q4 => "QuasiLiveness",
        Query::Q5 => "OneSafe",
        Query::Q6 => "StableMarking",
        _ => return None,
    })
}

impl ItsTools {
    pub fn new(cfg: &ItsCfg) -> Self {
        ItsTools { program: env_or("ITS_BIN", &cfg.command) }
    }

    pub fn build(&self, query: Query, net: &str) -> CommandSpec {
        let Some(exam) = examination(query) else {
            return CommandSpec::new("sh", ["-c".to_owned(), format!("echo {} >&2; exit 1", sh_quote(&format!("ITS-Tools has no examination for {}", query.as_str())))]);
        };
        let script = format!(
            "d=$(mktemp -d) || {{ echo 'mktemp failed' >&2; exit 70; }}\ntrap 'rm -rf \"$d\"' EXIT\nln -s {net} \"$d/model.pnml\" || exit 70\n{program} -pnfolder \"$d\" -examination {exam} -its -timeout 3600 </dev/null\n",
            net = sh_quote(net),
            program = sh_quote(&self.program),
        );
        CommandSpec::sh_script(script)
    }

    /// Reads the verdict line of the examination of `query`; exit status 0 only.
    pub fn parse_q(&self, query: Query, stdout: &str) -> Parsed {
        let Some(exam) = examination(query) else { return Parsed::with_detail(Verdict::NotSupported, "no examination for this question") };
        if query == Query::Q0 {
            let number = |key: &str| {
                stdout.lines().find_map(|l| {
                    let mut w = l.split_whitespace();
                    (w.next() == Some("STATE_SPACE") && w.next() == Some(key)).then(|| w.next().and_then(|n| n.parse::<u64>().ok())).flatten()
                })
            };
            return match number("STATES") {
                Some(s) => Parsed { states: Some(s), edges: number("TRANSITIONS"), ..Parsed::verdict(Verdict::Count) },
                None => Parsed::with_detail(Verdict::Unknown, "no STATE_SPACE STATES line"),
            };
        }
        // the last line wins: a tool that revises its answer prints the final one last
        let answer = stdout.lines().rev().find_map(|l| {
            let mut w = l.split_whitespace();
            (w.next() == Some("FORMULA") && w.next() == Some(exam)).then(|| w.next().map(str::to_owned)).flatten()
        });
        match answer.as_deref() {
            Some("TRUE") => Parsed::verdict(if query == Query::Q1 { Verdict::False } else { Verdict::True }),
            Some("FALSE") => Parsed::verdict(if query == Query::Q1 { Verdict::True } else { Verdict::False }),
            Some(other) => Parsed::with_detail(Verdict::Unknown, format!("the tool answered {other}")),
            None => Parsed::with_detail(Verdict::Unknown, "no verdict line in the output"),
        }
    }

    /// A nonzero exit of the wrapper is a crash of the tool (Java exceptions and the like); the first diagnostic line is kept.
    pub fn nonzero(&self, stdout: &str, stderr: &str, code: i32) -> Parsed {
        let diag = if stderr.trim().is_empty() { stdout } else { stderr };
        Parsed::crash(format!("exit:{code} {}", first_line(diag)).trim().to_owned())
    }
}

impl Adapter for ItsTools {
    fn solver_id(&self) -> &str {
        "its-tools"
    }

    fn supports(&self, query: Query) -> bool {
        examination(query).is_some()
    }

    fn command(&self, req: &Request) -> CommandSpec {
        // the link must not depend on the working directory of the script
        let net = std::path::absolute(req.instance_path).unwrap_or_else(|_| req.instance_path.to_path_buf());
        self.build(req.query, &net.display().to_string())
    }

    fn parse(&self, stdout: &str, _exit_code: i32) -> Parsed {
        self.parse_q(Query::Q4, stdout)
    }

    fn nonzero_exit(&self, stdout: &str, stderr: &str, exit_code: i32) -> Parsed {
        self.nonzero(stdout, stderr, exit_code)
    }

    fn classify_for(&self, query: Query, f: &Finished) -> Parsed {
        classify_with(f, |out| self.parse_q(query, out), |o, e, c| self.nonzero(o, e, c))
    }

    fn tool(&self) -> Option<ToolInfo> {
        Some(ToolInfo { program: self.program.clone(), version_args: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn its() -> ItsTools {
        ItsTools { program: "its-tools".into() }
    }

    fn tool(name: &str) -> String {
        std::fs::read_to_string(format!("{}/testdata/tools/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn supported_questions() {
        let supported: Vec<_> = [Query::Q0, Query::Q1, Query::Q2, Query::Q3, Query::Q4, Query::Q5, Query::Q6, Query::QR, Query::QC].into_iter().filter(|q| its().supports(*q)).collect();
        assert_eq!(supported, vec![Query::Q0, Query::Q1, Query::Q4, Query::Q5, Query::Q6]);
    }

    #[test]
    fn script_stages_a_folder_with_the_net() {
        let c = its().build(Query::Q5, "/data/it's net.pnml");
        assert_eq!((c.program.as_str(), c.args.as_slice()), ("sh", ["-s".to_owned()].as_slice()));
        let s = c.script_text();
        assert!(s.contains("ln -s '/data/it'\\''s net.pnml' \"$d/model.pnml\""), "{s}");
        assert!(s.contains("trap 'rm -rf \"$d\"' EXIT"));
        assert!(s.contains("'its-tools' -pnfolder \"$d\" -examination OneSafe -its -timeout 3600 </dev/null"));
        for (q, e) in [(Query::Q0, "StateSpace"), (Query::Q1, "ReachabilityDeadlock"), (Query::Q4, "QuasiLiveness"), (Query::Q6, "StableMarking")] {
            assert!(its().build(q, "N.pnml").script_text().contains(&format!("-examination {e} ")));
        }
    }

    #[test]
    fn verdicts_from_captured_output() {
        let v = |q, f: &str| its().parse_q(q, &tool(f));
        // the contest question is whether a deadlock is reachable: Q1 asks for the opposite
        assert_eq!(v(Query::Q1, "its_ReachabilityDeadlock_false.txt").verdict, Verdict::True);
        assert_eq!(v(Query::Q1, "its_ReachabilityDeadlock_true.txt").verdict, Verdict::False);
        assert_eq!(v(Query::Q4, "its_QuasiLiveness_true.txt").verdict, Verdict::True);
        assert_eq!(v(Query::Q5, "its_OneSafe_false.txt").verdict, Verdict::False);
        assert_eq!(v(Query::Q6, "its_StableMarking_false.txt").verdict, Verdict::False);
    }

    #[test]
    fn state_space_gives_states_without_transitions() {
        let p = its().parse_q(Query::Q0, &tool("its_StateSpace.txt"));
        assert_eq!((p.verdict, p.states, p.edges), (Verdict::Count, Some(16587), None));
        // a transition count is taken when the tool prints one
        let p = its().parse_q(Query::Q0, "STATE_SPACE STATES  5 TECHNIQUES X\nSTATE_SPACE TRANSITIONS 9 TECHNIQUES X\n");
        assert_eq!((p.states, p.edges), (Some(5), Some(9)));
    }

    #[test]
    fn no_verdict_or_other_answer_is_unknown() {
        assert_eq!(its().parse_q(Query::Q4, "Total runtime 10 ms.\n").verdict, Verdict::Unknown);
        assert_eq!(its().parse_q(Query::Q0, "FORMULA StateSpace TRUE\n").verdict, Verdict::Unknown);
        let p = its().parse_q(Query::Q5, "FORMULA OneSafe CANNOT_COMPUTE TECHNIQUES\n");
        assert_eq!((p.verdict, p.detail.as_deref()), (Verdict::Unknown, Some("the tool answered CANNOT_COMPUTE")));
        // a line of another examination does not count
        assert_eq!(its().parse_q(Query::Q5, "FORMULA Liveness TRUE\n").verdict, Verdict::Unknown);
    }

    #[test]
    fn the_last_verdict_line_wins() {
        let out = "FORMULA OneSafe FALSE TECHNIQUES A\nFORMULA OneSafe TRUE TECHNIQUES B\n";
        assert_eq!(its().parse_q(Query::Q5, out).verdict, Verdict::True);
    }

    #[test]
    fn crash_keeps_the_first_diagnostic() {
        let f = |code, err: &'static str| Finished { spawn_error: None, timed_out: false, memout: false, exit_code: Some(code), signal: None, stdout: "", stderr: err };
        let p = its().classify_for(Query::Q4, &f(1, "Exception in thread \"main\" java.lang.OutOfMemoryError\n"));
        assert_eq!(p.verdict, Verdict::Crash);
        assert!(p.detail.unwrap().starts_with("exit:1 Exception in thread"));
        assert_eq!(its().classify_for(Query::Q4, &f(0, "")).verdict, Verdict::Unknown);
    }
}
