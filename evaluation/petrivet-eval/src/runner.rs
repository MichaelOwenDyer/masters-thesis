//! Subprocess runner and job pool.
//!
//! One command per call, measured from the outside:
//! * wall time: monotonic clock from just before spawn to the moment `wait4` returns (a dedicated thread blocks
//!   in `wait4`, so there is no polling latency);
//! * CPU user/sys and peak RSS: `rusage` returned by `wait4` for the child (`ru_maxrss` is KiB on Linux and
//!   bytes on macOS, normalised to KiB here). Caveat: Linux folds the parent's RSS at fork into the child's
//!   `ru_maxrss` (exec keeps the maximum), so tiny runs report at least the size of this process; the startup
//!   baseline run of each solver makes that offset visible;
//! * wall limit: on expiry the whole process group is sent SIGKILL (the child is its own group leader) and the
//!   result is `timed_out`; the group is also killed after a normal exit to reap stragglers;
//! * memory limit, by [`Limiter`]:
//!   - `Cgroup` (Linux): the command is wrapped in `systemd-run --user --scope -p MemoryMax=.. -p MemorySwapMax=0`,
//!     which puts the process (systemd-run execs the command, same pid) into a transient cgroup v2 scope; the
//!     kernel OOM-kills it above the limit. This is the preferred method: it limits resident memory, not
//!     address space. Used when a probe at startup succeeds.
//!   - `RlimitAs` (Linux fallback): `setrlimit(RLIMIT_AS)` in the child before exec. This limits virtual address
//!     space, which for programs that reserve large virtual areas (Z3, thread stacks) is stricter than RSS.
//!   - `None`: no limit (macOS, or forced).
//!
//!   MEMOUT is detected heuristically (see [`looks_like_memout`]): the process died of SIGKILL without our
//!   doing and peaked at >= 90 percent of the limit, or stderr contains an allocator failure message.
//! * CPU pinning (Linux): `sched_setaffinity` to a single core in the child before exec. This is what
//!   `taskset -c N` does, without the extra wrapper process;
//! * stdout/stderr: captured through pipes by reader threads, kept up to a cap, the rest discarded.
//!
//! The Linux specific parts are behind `cfg(target_os = "linux")` and could not be exercised on the macOS
//! development machine.

use crate::adapters::CommandSpec;
use std::io::Read;
use std::path::Path;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limiter {
    None,
    Cgroup,
    RlimitAs,
}

impl Limiter {
    pub fn name(self) -> &'static str {
        match self {
            Limiter::None => "none",
            Limiter::Cgroup => "cgroup",
            Limiter::RlimitAs => "rlimit-as",
        }
    }

    /// `auto` picks cgroup if `systemd-run --user --scope` works, else rlimit-as (Linux), else none.
    pub fn resolve(preference: &str) -> Result<Limiter, String> {
        match preference {
            "none" => Ok(Limiter::None),
            "cgroup" if cgroup_available() => Ok(Limiter::Cgroup),
            "cgroup" => Err("mem_limiter = \"cgroup\" but `systemd-run --user --scope -p MemoryMax=..` does not work here".into()),
            "rlimit" if cfg!(target_os = "linux") => Ok(Limiter::RlimitAs),
            "rlimit" => Err("mem_limiter = \"rlimit\" is only supported on Linux".into()),
            "auto" => Ok(if cgroup_available() {
                Limiter::Cgroup
            } else if cfg!(target_os = "linux") {
                Limiter::RlimitAs
            } else {
                Limiter::None
            }),
            other => Err(format!("unknown mem_limiter {other:?} (auto, cgroup, rlimit, none)")),
        }
    }
}

fn cgroup_available() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    Command::new("systemd-run")
        .args(["--user", "--scope", "--quiet", "-p", "MemoryMax=256M", "-p", "MemorySwapMax=0", "-p", "OOMPolicy=continue", "--", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[derive(Clone, Debug)]
pub struct Limits {
    pub wall: Duration,
    pub mem_kib: u64,
    pub core: Option<usize>,
    /// Bytes of each of stdout and stderr that are kept.
    pub out_cap: usize,
    pub limiter: Limiter,
    /// Campaign M: start the command through `petrivet-eval rss` and take its peak memory from there.
    pub measure_rss: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RunResult {
    pub wall_ms: f64,
    pub cpu_user_ms: f64,
    pub cpu_sys_ms: f64,
    pub peak_rss_kib: Option<u64>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub timed_out: bool,
    pub memout: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub truncated: bool,
    pub spawn_error: Option<String>,
}

/// Heuristic MEMOUT detection (see module docs).
pub fn looks_like_memout(failed: bool, signal: Option<i32>, peak_rss_kib: Option<u64>, mem_kib: u64, stderr: &[u8]) -> bool {
    let text = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    let msg = text.contains("memory allocation of")
        || text.contains("out of memory")
        || text.contains("bad_alloc")
        || text.contains("cannot allocate memory");
    // SIGKILL is the kernel OOM killer; SIGTERM is systemd stopping the scope after one of its processes was
    // OOM-killed (OOMPolicy=stop). Shell wrappers report either as exit status 128+n, mapped by the caller.
    let killed_big = matches!(signal, Some(libc::SIGKILL | libc::SIGTERM))
        && peak_rss_kib.is_some_and(|r| mem_kib > 0 && r.saturating_mul(10) >= mem_kib.saturating_mul(9));
    (failed && msg) || killed_big
}

/// The program and arguments to start: the command itself, or (campaign M) `petrivet-eval rss` around it.
fn program_and_args(spec: &CommandSpec, rss_file: Option<&Path>) -> (String, Vec<String>) {
    match (rss_file, std::env::current_exe()) {
        (Some(file), Ok(exe)) => {
            let mut args = vec!["rss".to_owned(), "--out".into(), file.display().to_string(), "--".into(), spec.program.clone()];
            args.extend(spec.args.iter().cloned());
            (exe.display().to_string(), args)
        }
        _ => (spec.program.clone(), spec.args.clone()),
    }
}

fn build_command(spec: &CommandSpec, limits: &Limits, rss_file: Option<&Path>) -> Command {
    let (program, args) = program_and_args(spec, rss_file);
    let mut cmd = if limits.limiter == Limiter::Cgroup {
        let mut c = Command::new("systemd-run");
        c.args(["--user", "--scope", "--quiet", "-p"])
            .arg(format!("MemoryMax={}K", limits.mem_kib))
            // OOMPolicy=continue: after the kernel OOM-kills the tool, the scope is not stopped with SIGTERM, so a
            // wrapper shell survives, reaps the tool (SIGKILL, peak RSS at the limit) and reports it as status 137
            .args(["-p", "MemorySwapMax=0", "-p", "OOMPolicy=continue", "--"])
            .arg(&program)
            .args(&args);
        c
    } else {
        let mut c = Command::new(&program);
        c.args(&args);
        c
    };
    cmd.envs(spec.env.iter().map(|(k, v)| (k, v)));
    cmd.stdin(if spec.stdin.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.process_group(0);
    #[cfg(target_os = "linux")]
    {
        let rlimit = (limits.limiter == Limiter::RlimitAs).then(|| limits.mem_kib.saturating_mul(1024));
        let core = limits.core;
        // SAFETY: only async-signal-safe calls (setrlimit, sched_setaffinity) between fork and exec; failures are ignored.
        unsafe {
            cmd.pre_exec(move || {
                if let Some(bytes) = rlimit {
                    let lim = libc::rlimit { rlim_cur: bytes as libc::rlim_t, rlim_max: bytes as libc::rlim_t };
                    libc::setrlimit(libc::RLIMIT_AS, &lim);
                }
                if let Some(c) = core {
                    let mut set: libc::cpu_set_t = std::mem::zeroed();
                    libc::CPU_SET(c, &mut set);
                    libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
                }
                Ok(())
            });
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = limits.core;
    cmd
}

fn drain<R: Read + Send + 'static>(mut r: R, cap: usize) -> std::thread::JoinHandle<(Vec<u8>, bool)> {
    std::thread::spawn(move || {
        let (mut buf, mut truncated, mut chunk) = (Vec::new(), false, vec![0u8; 64 * 1024]);
        loop {
            match r.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let room = cap.saturating_sub(buf.len());
                    buf.extend_from_slice(&chunk[..n.min(room)]);
                    truncated |= n > room;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        (buf, truncated)
    })
}

fn tv_ms(tv: libc::timeval) -> f64 {
    tv.tv_sec as f64 * 1000.0 + tv.tv_usec as f64 / 1000.0
}

/// Runs one command to completion (or to the limit). Never panics on spawn failure; see `spawn_error`.
pub fn run_command(spec: &CommandSpec, limits: &Limits) -> RunResult {
    static RSS_FILES: AtomicUsize = AtomicUsize::new(0);
    let rss_file = limits.measure_rss.then(|| std::env::temp_dir().join(format!("petrivet-rss-{}-{}", std::process::id(), RSS_FILES.fetch_add(1, Ordering::Relaxed))));
    let mut cmd = build_command(spec, limits, rss_file.as_deref());
    let start = Instant::now();
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return RunResult { spawn_error: Some(format!("cannot run {}: {e}", spec.program)), ..RunResult::default() };
        }
    };
    let pid = child.id() as libc::pid_t;
    // the script goes in from a thread: it may be larger than the pipe, and a child killed early closes the pipe
    let feed = spec.stdin.clone().zip(child.stdin.take()).map(|(text, mut pipe)| {
        std::thread::spawn(move || {
            use std::io::Write;
            let _ = pipe.write_all(text.as_bytes());
        })
    });
    let out = drain(child.stdout.take().expect("piped"), limits.out_cap);
    let err = drain(child.stderr.take().expect("piped"), limits.out_cap);

    let (tx, rx) = mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let mut status: libc::c_int = 0;
        // SAFETY: plain wait4 on our own child; rusage is plain old data.
        let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
        loop {
            let r = unsafe { libc::wait4(pid, &mut status, 0, &mut ru) };
            if r == pid {
                let _ = tx.send(Some((Instant::now(), status, ru)));
                return;
            }
            if r == -1 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            let _ = tx.send(None);
            return;
        }
    });

    let kill_group = || {
        // SAFETY: signalling the process group we created; ESRCH is fine.
        unsafe { libc::kill(-pid, libc::SIGKILL) };
    };
    let mut timed_out = false;
    let waited = match rx.recv_timeout(limits.wall) {
        Ok(v) => v,
        Err(_) => {
            timed_out = true;
            kill_group();
            rx.recv().ok().flatten()
        }
    };
    kill_group();
    let _ = waiter.join();
    if let Some(f) = feed {
        let _ = f.join();
    }
    let (stdout, t1) = out.join().unwrap_or_default();
    let (stderr, t2) = err.join().unwrap_or_default();

    let Some((end, status, ru)) = waited else {
        return RunResult { spawn_error: Some("wait4 failed".into()), stdout, stderr, ..RunResult::default() };
    };
    let (exit_code, signal) = if libc::WIFEXITED(status) {
        (Some(libc::WEXITSTATUS(status)), None)
    } else if libc::WIFSIGNALED(status) {
        (None, Some(libc::WTERMSIG(status)))
    } else {
        (None, None)
    };
    // Only a kill by us counts as a timeout; a process that finished in the race window keeps its result.
    timed_out &= signal == Some(libc::SIGKILL);
    let maxrss = ru.ru_maxrss.max(0) as u64;
    let mut peak_rss_kib = Some(if cfg!(target_os = "macos") { maxrss / 1024 } else { maxrss });
    if let Some(file) = &rss_file {
        // campaign M: the wrapper's figure replaces wait4's, which contains the runner's own footprint; a wrapper
        // that was killed with the tool (time-out) leaves no file and the run has no memory value
        peak_rss_kib = std::fs::read_to_string(file).ok().and_then(|t| t.trim().parse().ok());
        let _ = std::fs::remove_file(file);
    }
    let shell_signal = signal.or(match exit_code {
        Some(137) => Some(libc::SIGKILL),
        Some(143) => Some(libc::SIGTERM),
        _ => None,
    });
    let memout = !timed_out && looks_like_memout(exit_code != Some(0), shell_signal, peak_rss_kib, limits.mem_kib, &stderr);
    RunResult {
        wall_ms: end.duration_since(start).as_secs_f64() * 1000.0,
        cpu_user_ms: tv_ms(ru.ru_utime),
        cpu_sys_ms: tv_ms(ru.ru_stime),
        peak_rss_kib,
        exit_code,
        signal,
        timed_out,
        memout,
        stdout,
        stderr,
        truncated: t1 || t2,
        spawn_error: None,
    }
}

/// Runs `jobs` on at most `n` worker threads. `work(job, worker)` runs on a worker (worker ids are
/// `0..n`, so a core list can be indexed by worker: one core per running job); `on_result(index, result)` runs
/// on the calling thread in completion order. Jobs are taken in list order.
pub fn run_pool<J: Sync, R: Send>(jobs: &[J], n: usize, work: impl Fn(&J, usize) -> R + Sync, mut on_result: impl FnMut(usize, R)) {
    let n = n.clamp(1, jobs.len().max(1));
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel();
    std::thread::scope(|s| {
        for w in 0..n {
            let (tx, next, work) = (tx.clone(), &next, &work);
            s.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(job) = jobs.get(i) else { return };
                    if tx.send((i, work(job, w))).is_err() {
                        return;
                    }
                }
            });
        }
        drop(tx);
        for (i, r) in rx {
            on_result(i, r);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(script: &str) -> CommandSpec {
        CommandSpec::new("sh", ["-c", script])
    }

    fn limits(wall_ms: u64) -> Limits {
        Limits { wall: Duration::from_millis(wall_ms), mem_kib: 8 * 1024 * 1024, core: None, out_cap: 1024, limiter: Limiter::None, measure_rss: false }
    }

    #[test]
    fn rss_wrapper_surrounds_the_command() {
        let spec = CommandSpec::new("tool", ["-a", "b c"]);
        let (prog, args) = program_and_args(&spec, Some(Path::new("/tmp/f")));
        assert!(prog.contains("petrivet") || !prog.is_empty());
        assert_eq!(&args[..4], ["rss", "--out", "/tmp/f", "--"].map(String::from));
        assert_eq!(&args[4..], ["tool", "-a", "b c"].map(String::from));
        assert_eq!(program_and_args(&spec, None), ("tool".to_owned(), vec!["-a".to_owned(), "b c".to_owned()]));
    }

    #[test]
    fn script_larger_than_one_argument_runs_via_stdin() {
        // 300 KiB: `sh -c` would fail with "Argument list too long"; stdin may be larger than the pipe too
        let script = format!("{}echo TRUE\n", "x=1\n".repeat(75_000));
        let r = run_command(&CommandSpec::sh_script(script), &limits(10_000));
        assert!(r.spawn_error.is_none() && r.exit_code == Some(0), "{r:?}");
        assert!(String::from_utf8_lossy(&r.stdout).contains("TRUE"));
    }

    #[test]
    fn fast_command_is_measured() {
        let r = run_command(&sh("echo TRUE; i=0; while [ $i -lt 20000 ]; do i=$((i+1)); done"), &limits(10_000));
        assert_eq!(r.exit_code, Some(0));
        assert!(!r.timed_out && r.signal.is_none());
        assert_eq!(String::from_utf8_lossy(&r.stdout).trim(), "TRUE");
        assert!(r.wall_ms > 0.0 && r.wall_ms < 5000.0, "wall {}", r.wall_ms);
        assert!(r.cpu_user_ms + r.cpu_sys_ms > 0.0 && r.cpu_user_ms + r.cpu_sys_ms < 5000.0);
        let rss = r.peak_rss_kib.unwrap();
        assert!((100..4_000_000).contains(&rss), "rss {rss} KiB");
    }

    #[test]
    fn timeout_kills_the_process_group() {
        let pidfile = std::env::temp_dir().join(format!("pe-runner-{}.pid", std::process::id()));
        let _ = std::fs::remove_file(&pidfile);
        let script = format!("sleep 30 & echo $! > {}; wait", pidfile.display());
        let r = run_command(&sh(&script), &limits(400));
        assert!(r.timed_out, "{r:?}");
        assert_eq!(r.signal, Some(libc::SIGKILL));
        assert!(r.wall_ms >= 400.0 && r.wall_ms < 3000.0, "wall {}", r.wall_ms);
        let pid: i32 = std::fs::read_to_string(&pidfile).unwrap().trim().parse().unwrap();
        let gone = (0..40).any(|_| {
            std::thread::sleep(Duration::from_millis(50));
            // SAFETY: probing for existence only.
            unsafe { libc::kill(pid, 0) == -1 }
        });
        assert!(gone, "grandchild {pid} survived the group kill");
    }

    #[test]
    fn output_is_capped_and_spawn_errors_reported() {
        let r = run_command(&sh("yes | head -c 100000"), &limits(10_000));
        assert_eq!(r.stdout.len(), 1024);
        assert!(r.truncated);
        let r = run_command(&CommandSpec::new("/nonexistent/prog", Vec::<String>::new()), &limits(1000));
        assert!(r.spawn_error.is_some());
    }

    #[test]
    fn memout_heuristic() {
        assert!(looks_like_memout(true, Some(libc::SIGABRT), None, 1024, b"memory allocation of 99 bytes failed"));
        assert!(looks_like_memout(true, Some(libc::SIGKILL), Some(950), 1000, b""));
        assert!(!looks_like_memout(true, Some(libc::SIGKILL), Some(100), 1000, b""));
        // systemd stops the scope with SIGTERM after the OOM kill; only near the limit is it a MEMOUT
        assert!(looks_like_memout(true, Some(libc::SIGTERM), Some(950), 1000, b""));
        assert!(!looks_like_memout(true, Some(libc::SIGTERM), Some(100), 1000, b""));
        assert!(!looks_like_memout(false, None, Some(2000), 1000, b""));
    }

    #[test]
    fn pool_runs_in_parallel_with_worker_ids() {
        let jobs: Vec<u64> = (0..6).collect();
        let t = Instant::now();
        let mut seen = Vec::new();
        run_pool(&jobs, 3, |j, w| {
            std::thread::sleep(Duration::from_millis(100));
            (*j, w)
        }, |_, r| seen.push(r));
        assert_eq!(seen.len(), 6);
        assert!(seen.iter().all(|(_, w)| *w < 3));
        assert!(t.elapsed() < Duration::from_millis(500));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rlimit_as_stops_a_memory_hog() {
        // Not runnable on the macOS dev machine; written for the Linux box.
        let l = Limits { wall: Duration::from_secs(20), mem_kib: 64 * 1024, core: None, out_cap: 4096, limiter: Limiter::RlimitAs, measure_rss: false };
        let r = run_command(&sh("awk 'BEGIN{s=\"x\"; while(1) s = s s}'"), &l);
        assert!(!r.timed_out);
        assert!(r.exit_code != Some(0) || r.signal.is_some(), "{r:?}");
    }
}
