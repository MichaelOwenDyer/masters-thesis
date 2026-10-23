//! Mock adapter for tests: runs an arbitrary shell command and reads the first word of its stdout as
//! the verdict (`TRUE`, `FALSE`, ...). A non-zero exit is `ERROR`. Not reachable from the CLI.

use super::{Adapter, CommandSpec, Parsed, Query, Request};
use crate::record::Verdict;

pub struct Mock {
    pub id: String,
    pub shell: String,
}

impl Mock {
    pub fn new(id: &str, shell: &str) -> Self {
        Mock { id: id.to_owned(), shell: shell.to_owned() }
    }
}

impl Adapter for Mock {
    fn solver_id(&self) -> &str {
        &self.id
    }
    fn supports(&self, _query: Query) -> bool {
        true
    }
    fn command(&self, _req: &Request) -> CommandSpec {
        CommandSpec::new("sh", ["-c", self.shell.as_str()])
    }
    fn parse(&self, stdout: &str, exit_code: i32) -> Parsed {
        if exit_code != 0 {
            return Parsed::error(format!("exit code {exit_code}"));
        }
        let word = stdout.split_whitespace().next().unwrap_or("");
        match Verdict::from_name(word) {
            Some(v) => Parsed::verdict(v),
            None => Parsed::error(format!("unparsable output {word:?}")),
        }
    }
}
