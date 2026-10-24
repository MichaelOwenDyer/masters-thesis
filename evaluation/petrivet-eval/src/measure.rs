//! The memory wrapper of campaign M: `petrivet-eval rss --out FILE -- PROGRAM ARGS...`.
//!
//! The peak memory that `wait4` reports for a child also contains what the child inherited from its parent at
//! the moment it was started, so a child of the (large) runner never shows less than the runner's own footprint
//! (in campaign B: a floor of about 143 MiB for every tool). The wrapper is a fresh, small process: it starts the
//! tool as its own child, waits for it with `wait4`, writes the tool's peak resident set size in KiB to FILE and
//! then ends the way the tool ended (same exit code, or the same signal), so that the runner sees the same
//! status as without the wrapper. Standard input, output and error are inherited, not touched.

use std::path::Path;
use std::process::Command;

/// How the wrapped command ended and its peak resident set size.
#[derive(Debug, PartialEq, Eq)]
pub struct Measured {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub peak_rss_kib: u64,
}

/// Runs `cmd` (program and arguments) to completion and measures it.
pub fn measure(cmd: &[String]) -> std::io::Result<Measured> {
    let Some((program, args)) = cmd.split_first() else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "no command"));
    };
    let child = Command::new(program).args(args).spawn()?;
    let pid = child.id() as libc::pid_t;
    let mut status: libc::c_int = 0;
    // SAFETY: plain wait4 on our own child; rusage is plain old data. The `Child` is not waited on elsewhere.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    loop {
        let r = unsafe { libc::wait4(pid, &mut status, 0, &mut ru) };
        if r == pid {
            break;
        }
        let err = std::io::Error::last_os_error();
        if r == -1 && err.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(err);
    }
    let maxrss = ru.ru_maxrss.max(0) as u64;
    Ok(Measured {
        exit_code: libc::WIFEXITED(status).then(|| libc::WEXITSTATUS(status)),
        signal: libc::WIFSIGNALED(status).then(|| libc::WTERMSIG(status)),
        peak_rss_kib: if cfg!(target_os = "macos") { maxrss / 1024 } else { maxrss },
    })
}

/// The wrapper process: measure, write the number to `out`, end like the command.
pub fn main(out: &Path, cmd: &[String]) -> ! {
    match measure(cmd) {
        Ok(m) => {
            if let Err(e) = std::fs::write(out, format!("{}\n", m.peak_rss_kib)) {
                eprintln!("rss: cannot write {}: {e}", out.display());
            }
            if let Some(sig) = m.signal {
                // SAFETY: restoring the default action and signalling ourselves is what ends the process the same way.
                unsafe {
                    libc::signal(sig, libc::SIG_DFL);
                    libc::raise(sig);
                }
                std::process::exit(128 + sig);
            }
            std::process::exit(m.exit_code.unwrap_or(1));
        }
        Err(e) => {
            eprintln!("rss: cannot run {}: {e}", cmd.first().map_or("", String::as_str));
            std::process::exit(127);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(script: &str) -> Vec<String> {
        vec!["sh".into(), "-c".into(), script.into()]
    }

    #[test]
    fn exit_code_and_memory_of_the_tool() {
        // a 30 MB shell variable: the tool's peak is at least that
        let m = measure(&sh("x=$(head -c 30000000 /dev/zero | tr '\\0' a); exit 3")).unwrap();
        assert_eq!((m.exit_code, m.signal), (Some(3), None));
        assert!(m.peak_rss_kib >= 25_000, "{m:?}");
    }

    #[test]
    fn a_small_tool_is_small() {
        let m = measure(&sh("true")).unwrap();
        assert!(m.peak_rss_kib < 20_000, "{m:?}");
    }

    #[test]
    fn signal_of_the_tool_is_reported() {
        let m = measure(&sh("kill -9 $$")).unwrap();
        assert_eq!((m.exit_code, m.signal), (None, Some(libc::SIGKILL)));
    }

    #[test]
    fn missing_program_is_an_error() {
        assert!(measure(&["/nonexistent/tool".to_owned()]).is_err());
        assert!(measure(&[]).is_err());
    }
}
