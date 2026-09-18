//! Shared subprocess lifecycle helpers.
//!
//! Spawning child processes correctly is easy to get wrong: if the child fills
//! the stderr pipe buffer while the parent is still reading stdout (or vice
//! versa), both sides deadlock. [`spawn_streaming`] drains stderr on a
//! background thread from the moment of spawn, and [`SpawnedProcess`] reaps the
//! child on every exit path — including early returns, via `Drop`.

use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Maximum stderr bytes retained for error reporting.
///
/// The stderr reader thread keeps draining past this cap (to prevent the child
/// blocking on a full pipe) but stops appending to the buffer.
const STDERR_CAP: usize = 256 * 1024;

/// A spawned child process with concurrent stderr draining.
///
/// Created by [`spawn_streaming`]. The caller streams [`stdout`](Self::stdout)
/// to EOF while a background thread accumulates stderr. The child is always
/// reaped: explicitly via [`wait`](Self::wait)/[`kill_reap`](Self::kill_reap),
/// or by `Drop` if the handle is abandoned.
pub struct SpawnedProcess {
    /// The spawned child handle.
    pub child: Child,
    /// Buffered stdout for line-by-line streaming (take once).
    stdout: Option<BufReader<ChildStdout>>,
    /// Background thread draining stderr.
    stderr_handle: Option<thread::JoinHandle<String>>,
}

impl SpawnedProcess {
    /// Take the buffered stdout reader. Panics if called twice.
    pub fn take_stdout(&mut self) -> BufReader<ChildStdout> {
        self.stdout.take().expect("stdout already taken")
    }

    /// Kill the child (best effort).
    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }

    /// Wait for the child to exit, collecting drained stderr output.
    ///
    /// Returns `(exit_status, stderr)`.
    pub fn wait(mut self) -> std::io::Result<(ExitStatus, String)> {
        let status = self.child.wait();
        let stderr = self.join_stderr();
        status.map(|s| (s, stderr))
    }

    /// Kill the child if still running, reap it, and return collected stderr.
    pub fn kill_reap(mut self) -> (String, Option<ExitStatus>) {
        let _ = self.child.kill();
        let status = self.child.wait().ok();
        let stderr = self.join_stderr();
        (stderr, status)
    }

    fn join_stderr(&mut self) -> String {
        self.stderr_handle
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_default()
    }
}

impl Drop for SpawnedProcess {
    fn drop(&mut self) {
        // Never leave a zombie: if the handle is dropped without wait(), kill
        // and reap the child. wait() on an already-exited child returns
        // immediately with the cached status.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Spawn `cmd` with piped stdout/stderr, draining stderr on a background thread.
///
/// `stdin` is set to null, `stdout` is piped for streaming, and stderr is piped
/// into a background accumulator thread. This eliminates the classic deadlock
/// where a verbose child fills the stderr pipe buffer and blocks on write while
/// the parent is still waiting for stdout EOF.
pub fn spawn_streaming(cmd: &mut Command) -> std::io::Result<SpawnedProcess> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = BufReader::new(child.stdout.take().expect("stdout was piped"));
    let stderr = child.stderr.take().expect("stderr was piped");

    let stderr_handle = thread::spawn(move || {
        let mut buf = String::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if buf.len() < STDERR_CAP {
                buf.push_str(&line);
                buf.push('\n');
            }
        }
        buf
    });

    Ok(SpawnedProcess {
        child,
        stdout: Some(stdout),
        stderr_handle: Some(stderr_handle),
    })
}

/// Error from [`run_with_timeout`].
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    /// The command could not be spawned.
    #[error("Failed to spawn command: {0}")]
    Spawn(#[source] std::io::Error),
    /// The command exceeded the timeout and was abandoned.
    #[error("Command timed out after {0:?}")]
    Timeout(Duration),
    /// I/O error while waiting for the command.
    #[error("Command failed: {0}")]
    Io(#[source] std::io::Error),
}

/// Run a command to completion with a timeout, capturing stdout and stderr.
///
/// On timeout the child is orphaned (it cannot be killed through this API) but
/// the caller is freed rather than blocking forever. Used for short-lived
/// tooling calls such as `bd`, `gh`, and `openspec` where a hung subprocess
/// must not wedge the session.
pub fn run_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<std::process::Output, ProcessError> {
    let mut cmd = Command::new(program);
    cmd.args(args);

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || tx.send(cmd.output()));

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(ProcessError::Spawn(e)),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(ProcessError::Timeout(timeout)),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(ProcessError::Io(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "worker thread disconnected",
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_streaming_captures_stdout_and_stderr() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo out; echo err >&2"]);
        let mut proc = spawn_streaming(&mut cmd).unwrap();

        let stdout = proc.take_stdout();
        let lines: Vec<String> = stdout.lines().map_while(Result::ok).collect();
        let (status, stderr) = proc.wait().unwrap();

        assert!(status.success());
        assert_eq!(lines, vec!["out"]);
        assert_eq!(stderr.trim(), "err");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_spawn_streaming_large_stderr_no_deadlock() {
        // >64KB of stderr while stdout is still open. Without concurrent
        // draining the child blocks on write(2) and stdout never reaches EOF.
        let mut cmd = Command::new("sh");
        cmd.args([
            "-c",
            "for i in $(seq 1 5000); do echo \"stderr $i\" >&2; done; echo done",
        ]);
        let mut proc = spawn_streaming(&mut cmd).unwrap();

        let stdout = proc.take_stdout();
        let lines: Vec<String> = stdout.lines().map_while(Result::ok).collect();
        let (status, stderr) = proc.wait().unwrap();

        assert!(status.success());
        assert_eq!(lines, vec!["done"]);
        assert!(stderr.contains("stderr 5000"));
    }

    #[test]
    fn test_kill_reap() {
        let mut cmd = Command::new("sleep");
        cmd.arg("30");
        let proc = spawn_streaming(&mut cmd).unwrap();
        let (_stderr, status) = proc.kill_reap();
        // Killed: status is Some(non-success) on unix
        assert!(status.is_some());
        assert!(!status.unwrap().success());
    }

    #[test]
    fn test_drop_reaps_child() {
        // Dropping without wait() must not leave a zombie.
        let pid;
        {
            let mut cmd = Command::new("sleep");
            cmd.arg("30");
            let proc = spawn_streaming(&mut cmd).unwrap();
            pid = proc.child.id();
        } // proc dropped here — Drop kills + waits

        // The process should be gone (kill was sent).
        #[cfg(unix)]
        {
            let status = Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output()
                .unwrap();
            assert!(!status.status.success(), "child {pid} still running");
        }
    }

    #[test]
    fn test_run_with_timeout_success() {
        let output = run_with_timeout("echo", &["hi"], Duration::from_secs(10)).unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hi");
    }

    #[test]
    fn test_run_with_timeout_timeout() {
        let result = run_with_timeout("sleep", &["30"], Duration::from_millis(100));
        assert!(matches!(result, Err(ProcessError::Timeout(_))));
    }

    #[test]
    fn test_run_with_timeout_missing_program() {
        let result = run_with_timeout("afk_nonexistent_program_xyz", &[], Duration::from_secs(5));
        assert!(matches!(result, Err(ProcessError::Spawn(_))));
    }
}
