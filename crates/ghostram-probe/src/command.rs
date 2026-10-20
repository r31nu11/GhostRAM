//! Bounded, auditable subprocess execution.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ghostram_core::error::GhostramError;

const OUTPUT_LIMIT: usize = 64 * 1024;
const ERROR_LIMIT: usize = 16 * 1024;
#[cfg(unix)]
const POLL_INTERVAL: Duration = Duration::from_millis(10);
#[cfg(unix)]
const READER_GRACE: Duration = Duration::from_millis(500);

/// Result of a bounded child-process invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandObservation {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) timed_out: bool,
}

impl CommandObservation {
    pub(crate) fn succeeded(&self) -> bool {
        !self.timed_out && self.exit_code == Some(0)
    }

    pub(crate) fn first_output_line(&self) -> Option<String> {
        self.stdout
            .lines()
            .chain(self.stderr.lines())
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| shorten(line, 240))
    }
}

fn shorten(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let shortened: String = value.chars().take(limit.saturating_sub(3)).collect();
    format!("{shortened}...")
}

/// Runs a program with an explicit argument array and a bounded timeout.
///
/// The child inherits no stdin. stdout and stderr are captured with memory
/// limits. No shell is involved.
///
/// On Unix, the child is placed in its own process group. On timeout the whole
/// group is sent `SIGKILL`, then the direct child is reaped. Descendants that
/// deliberately leave the process group are not contained; output polling has a
/// bounded drain grace so such a descendant cannot block the caller indefinitely.
///
/// On non-Unix platforms bounded execution is unsupported and returns an error.
/// The diagnostic collector reports unsupported platforms without invoking it.
pub(crate) fn run_bounded(
    program: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandObservation, GhostramError> {
    run_bounded_with(program, args, timeout, |_| {})
}

#[cfg(unix)]
fn run_bounded_with(
    program: &Path,
    args: &[&str],
    timeout: Duration,
    configure: impl FnOnce(&mut Command),
) -> Result<CommandObservation, GhostramError> {
    use std::os::unix::process::CommandExt;

    let program_text = program.display().to_string();
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut command);
    command.process_group(0);

    let mut child = command.spawn().map_err(|error| GhostramError::Command {
        program: program_text.clone(),
        message: error.to_string(),
    })?;

    let stdout = child
        .stdout
        .take()
        .map(|pipe| PipeCapture::new(pipe, OUTPUT_LIMIT));
    let stderr = child
        .stderr
        .take()
        .map(|pipe| PipeCapture::new(pipe, ERROR_LIMIT));
    let (mut stdout, mut stderr) = match (stdout, stderr) {
        (Some(Ok(stdout)), Some(Ok(stderr))) => (Some(stdout), Some(stderr)),
        (stdout, stderr) => {
            let error = stdout
                .and_then(Result::err)
                .or_else(|| stderr.and_then(Result::err))
                .unwrap_or_else(|| std::io::Error::other("output pipe setup failed"));
            terminate_process_group(&mut child);
            let _ = child.wait();
            return Err(GhostramError::Command {
                program: program_text,
                message: error.to_string(),
            });
        }
    };

    let started = Instant::now();
    let mut timed_out = false;
    let exit_status = loop {
        let remaining = timeout.saturating_sub(started.elapsed());
        poll_output(
            stdout.as_mut(),
            stderr.as_mut(),
            remaining.min(POLL_INTERVAL),
        );
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() >= timeout => {
                timed_out = true;
                terminate_process_group(&mut child);
                break child.wait().ok();
            }
            Ok(None) => {}
            Err(error) => {
                terminate_process_group(&mut child);
                let _ = child.wait();
                return Err(GhostramError::Command {
                    program: program_text,
                    message: error.to_string(),
                });
            }
        }
    };

    drain_remaining(stdout.as_mut(), stderr.as_mut(), READER_GRACE);

    let stdout = stdout.map(PipeCapture::finish).unwrap_or_default();
    let stderr = stderr.map(PipeCapture::finish).unwrap_or_default();

    Ok(CommandObservation {
        stdout,
        stderr,
        exit_code: exit_status.and_then(|status| status.code()),
        timed_out,
    })
}

#[cfg(not(unix))]
fn run_bounded_with(
    program: &Path,
    _args: &[&str],
    _timeout: Duration,
    _configure: impl FnOnce(&mut Command),
) -> Result<CommandObservation, GhostramError> {
    Err(GhostramError::Command {
        program: program.display().to_string(),
        message: "bounded subprocess execution is supported only on Unix".to_string(),
    })
}

#[cfg(unix)]
struct PipeCapture<R> {
    reader: Option<R>,
    captured: Vec<u8>,
    limit: usize,
}

#[cfg(unix)]
impl<R: Read + std::os::fd::AsRawFd> PipeCapture<R> {
    fn new(reader: R, limit: usize) -> std::io::Result<Self> {
        set_nonblocking(reader.as_raw_fd())?;
        Ok(Self {
            reader: Some(reader),
            captured: Vec::new(),
            limit,
        })
    }

    fn fd(&self) -> Option<std::os::fd::RawFd> {
        self.reader.as_ref().map(std::os::fd::AsRawFd::as_raw_fd)
    }

    fn is_open(&self) -> bool {
        self.reader.is_some()
    }

    fn drain(&mut self) {
        let mut buffer = [0_u8; 8192];
        loop {
            let read = match self.reader.as_mut() {
                Some(reader) => reader.read(&mut buffer),
                None => return,
            };
            match read {
                Ok(0) => {
                    self.reader = None;
                    return;
                }
                Ok(read) => {
                    let remaining = self.limit.saturating_sub(self.captured.len());
                    if remaining > 0 {
                        let take = read.min(remaining);
                        self.captured.extend_from_slice(&buffer[..take]);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return,
                Err(_) => {
                    self.reader = None;
                    return;
                }
            }
        }
    }

    fn finish(self) -> String {
        String::from_utf8_lossy(&self.captured).into_owned()
    }
}

#[cfg(unix)]
fn set_nonblocking(fd: std::os::fd::RawFd) -> std::io::Result<()> {
    // SAFETY: `fd` is an open pipe descriptor owned by the caller; fcntl only
    // queries and updates descriptor flags.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: same descriptor and valid flags obtained from F_GETFL.
    let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if result < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn poll_output(
    stdout: Option<&mut PipeCapture<std::process::ChildStdout>>,
    stderr: Option<&mut PipeCapture<std::process::ChildStderr>>,
    wait: Duration,
) {
    if stdout.as_ref().is_none_or(|pipe| !pipe.is_open())
        && stderr.as_ref().is_none_or(|pipe| !pipe.is_open())
    {
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
        return;
    }

    let mut descriptors = [
        libc::pollfd {
            fd: stdout.as_ref().and_then(|pipe| pipe.fd()).unwrap_or(-1),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: stderr.as_ref().and_then(|pipe| pipe.fd()).unwrap_or(-1),
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    let timeout_ms = wait.as_millis().min(i32::MAX as u128) as i32;
    // SAFETY: `descriptors` is a valid array of pollfd entries and the timeout
    // is a non-negative millisecond count.
    let result = unsafe { libc::poll(descriptors.as_mut_ptr(), 2, timeout_ms) };
    if result <= 0 {
        return;
    }

    if descriptors[0].revents != 0 {
        if let Some(pipe) = stdout {
            pipe.drain();
        }
    }
    if descriptors[1].revents != 0 {
        if let Some(pipe) = stderr {
            pipe.drain();
        }
    }
}

#[cfg(unix)]
fn drain_remaining(
    mut stdout: Option<&mut PipeCapture<std::process::ChildStdout>>,
    mut stderr: Option<&mut PipeCapture<std::process::ChildStderr>>,
    grace: Duration,
) {
    let deadline = Instant::now() + grace;
    loop {
        let stdout_open = stdout.as_ref().is_some_and(|pipe| pipe.is_open());
        let stderr_open = stderr.as_ref().is_some_and(|pipe| pipe.is_open());
        if !stdout_open && !stderr_open {
            return;
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        poll_output(
            stdout.as_deref_mut(),
            stderr.as_deref_mut(),
            remaining.min(POLL_INTERVAL),
        );
        if Instant::now() >= deadline {
            break;
        }
    }

    // Drain any bytes already buffered without waiting for another writer.
    poll_output(stdout, stderr, Duration::ZERO);
}

#[cfg(unix)]
fn terminate_process_group(child: &mut std::process::Child) {
    if let Ok(pgid) = libc::pid_t::try_from(child.id()) {
        // SAFETY: the child was started with `process_group(0)`, so its process
        // group identifier is its own pid. A negative pid targets that group.
        let result = unsafe { libc::kill(-pgid, libc::SIGKILL) };
        if result != 0 {
            // The group may already be gone, or group signalling may have
            // failed; still try to terminate the direct child.
            let _ = child.kill();
        }
    } else {
        let _ = child.kill();
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use super::{run_bounded, run_bounded_with};

    #[test]
    fn captures_output_and_success() {
        let result =
            run_bounded(Path::new("/bin/echo"), &["hello"], Duration::from_secs(2)).unwrap();
        assert!(result.succeeded());
        assert_eq!(result.stdout.trim(), "hello");
        assert!(result.stderr.is_empty());
        assert!(!result.timed_out);
    }

    #[test]
    fn truncates_captured_output() {
        let long = "x".repeat(70 * 1024);
        let result = run_bounded(
            Path::new("/bin/echo"),
            &[long.as_str()],
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(result.succeeded());
        assert_eq!(result.stdout.trim().len(), 64 * 1024);
    }

    #[cfg(target_os = "linux")]
    const HELPER_FILTER: &str = "subprocess_lifecycle_helper";
    #[cfg(target_os = "linux")]
    const HELPER_ROLE_ENV: &str = "GHOSTRAM_PROBE_TEST_HELPER";
    #[cfg(target_os = "linux")]
    const HELPER_PID_FILE_ENV: &str = "GHOSTRAM_PROBE_TEST_PID_FILE";
    #[cfg(target_os = "linux")]
    const HELPER_DESCENDANT_PID_FILE_ENV: &str = "GHOSTRAM_PROBE_TEST_DESCENDANT_PID_FILE";

    /// Test-only subprocess roles used by the lifecycle tests. Normal test runs
    /// have no helper role and return immediately.
    #[cfg(target_os = "linux")]
    #[test]
    fn subprocess_lifecycle_helper() {
        let Ok(role) = env::var(HELPER_ROLE_ENV) else {
            return;
        };

        match role.as_str() {
            "sleep" => {
                write_pid_file(HELPER_PID_FILE_ENV);
                thread::sleep(Duration::from_secs(60));
            }
            "exit-7" => process::exit(7),
            "large-output" => {
                use std::io::Write;

                let mut stdout = std::io::stdout().lock();
                stdout.write_all(&vec![b'o'; 70 * 1024]).unwrap();
                stdout.flush().unwrap();
                let mut stderr = std::io::stderr().lock();
                stderr.write_all(&vec![b'e'; 20 * 1024]).unwrap();
                stderr.flush().unwrap();
            }
            "descendant-parent" => {
                write_pid_file(HELPER_PID_FILE_ENV);
                spawn_descendant("descendant-holder", false);
                thread::sleep(Duration::from_secs(60));
            }
            "escaped-descendant-parent" => {
                write_pid_file(HELPER_PID_FILE_ENV);
                spawn_descendant("descendant-holder", true);
                thread::sleep(Duration::from_secs(60));
            }
            "descendant-holder" => {
                write_pid_file(HELPER_DESCENDANT_PID_FILE_ENV);
                thread::sleep(Duration::from_secs(60));
            }
            other => panic!("unknown test-helper role: {other}"),
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bounds_both_output_streams() {
        let result = run_helper("large-output", Duration::from_secs(3), None, None)
            .expect("test helper should run");
        assert!(result.succeeded());
        assert_eq!(result.stdout.len(), 64 * 1024);
        assert_eq!(result.stderr.len(), 16 * 1024);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn terminates_and_reaps_a_timed_out_child() {
        let scratch = ScratchDir::new("direct-timeout");
        let pid_file = scratch.path.join("child.pid");
        let started = Instant::now();
        let result = run_helper("sleep", Duration::from_secs(2), Some(&pid_file), None)
            .expect("test helper should run");
        let elapsed = started.elapsed();

        assert!(result.timed_out, "helper should exceed its timeout");
        assert_eq!(result.exit_code, None);
        assert!(
            elapsed < Duration::from_secs(8),
            "timed-out call took {elapsed:?}"
        );
        let pid = read_pid(&pid_file);
        assert!(
            wait_until_not_running(pid),
            "direct child {pid} was not terminated and reaped"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn terminates_an_inherited_pipe_descendant_on_timeout() {
        let scratch = ScratchDir::new("descendant-timeout");
        let child_pid_file = scratch.path.join("child.pid");
        let descendant_pid_file = scratch.path.join("descendant.pid");
        let started = Instant::now();
        let result = run_helper(
            "descendant-parent",
            Duration::from_secs(2),
            Some(&child_pid_file),
            Some(&descendant_pid_file),
        )
        .expect("test helper should run");
        let elapsed = started.elapsed();

        assert!(result.timed_out, "helper should exceed its timeout");
        assert!(
            elapsed < Duration::from_secs(8),
            "timed-out call with a pipe-holding descendant took {elapsed:?}"
        );
        let child_pid = read_pid(&child_pid_file);
        let descendant_pid = read_pid(&descendant_pid_file);
        assert!(
            wait_until_not_running(child_pid),
            "direct child {child_pid} was not reaped"
        );
        assert!(
            wait_until_not_running(descendant_pid),
            "descendant {descendant_pid} survived process-group termination"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn bounded_return_survives_a_descendant_that_escapes_the_process_group() {
        let scratch = ScratchDir::new("escaped-descendant");
        let child_pid_file = scratch.path.join("child.pid");
        let descendant_pid_file = scratch.path.join("descendant.pid");
        let started = Instant::now();
        let result = run_helper(
            "escaped-descendant-parent",
            Duration::from_secs(2),
            Some(&child_pid_file),
            Some(&descendant_pid_file),
        )
        .expect("test helper should run");
        let elapsed = started.elapsed();

        assert!(result.timed_out, "helper should exceed its timeout");
        assert!(
            elapsed < Duration::from_secs(8),
            "escaped descendant delayed return to {elapsed:?}"
        );
        let child_pid = read_pid(&child_pid_file);
        let descendant_pid = read_pid(&descendant_pid_file);
        assert!(
            wait_until_not_running(child_pid),
            "direct child {child_pid} was not reaped"
        );

        // The escaped descendant intentionally remains alive; clean it up and
        // show that it did not block the bounded call.
        let guard = KillOnDrop::new(descendant_pid);
        assert!(
            Path::new(&format!("/proc/{descendant_pid}")).exists(),
            "escaped descendant {descendant_pid} should still exist after return"
        );
        assert!(
            guard.kill_and_wait(),
            "escaped descendant {descendant_pid} could not be cleaned up"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn reports_nonzero_exit_without_a_timeout() {
        let result = run_helper("exit-7", Duration::from_secs(3), None, None)
            .expect("test helper should run");
        assert!(!result.timed_out);
        assert_eq!(result.exit_code, Some(7));
        assert!(!result.succeeded());
    }

    #[cfg(target_os = "linux")]
    fn run_helper(
        role: &str,
        timeout: Duration,
        pid_file: Option<&Path>,
        descendant_pid_file: Option<&Path>,
    ) -> Result<super::CommandObservation, ghostram_core::error::GhostramError> {
        let executable = env::current_exe().expect("test executable path");
        let args = [HELPER_FILTER, "--nocapture"];
        run_bounded_with(&executable, &args, timeout, |command| {
            command.env(HELPER_ROLE_ENV, role);
            if let Some(path) = pid_file {
                command.env(HELPER_PID_FILE_ENV, path);
            }
            if let Some(path) = descendant_pid_file {
                command.env(HELPER_DESCENDANT_PID_FILE_ENV, path);
            }
        })
    }

    #[cfg(target_os = "linux")]
    #[allow(clippy::zombie_processes)]
    fn spawn_descendant(role: &str, escaped: bool) {
        // The parent helper is intentionally killed by the timeout before it can
        // reap this process; the test then verifies group cleanup or cleans up
        // the deliberately escaped process itself.
        use std::os::unix::process::CommandExt;

        let executable = env::current_exe().expect("test executable path");
        let mut command = process::Command::new(executable);
        command
            .args([HELPER_FILTER, "--nocapture"])
            .env(HELPER_ROLE_ENV, role);
        if let Some(path) = env::var_os(HELPER_DESCENDANT_PID_FILE_ENV) {
            command.env(HELPER_DESCENDANT_PID_FILE_ENV, path);
        }
        if escaped {
            command.process_group(0);
        }
        command
            .spawn()
            .expect("descendant test helper should start");
    }

    #[cfg(target_os = "linux")]
    fn write_pid_file(variable: &str) {
        let path = env::var_os(variable).expect("test-helper pid-file variable should be set");
        fs::write(path, process::id().to_string())
            .expect("test-helper pid file should be writable");
    }

    #[cfg(target_os = "linux")]
    fn read_pid(path: &Path) -> u32 {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
        text.trim()
            .parse()
            .unwrap_or_else(|error| panic!("invalid pid in {}: {error}", path.display()))
    }

    #[cfg(target_os = "linux")]
    fn wait_until_not_running(pid: u32) -> bool {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if !Path::new(&format!("/proc/{pid}")).exists() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(target_os = "linux")]
    struct ScratchDir {
        path: PathBuf,
    }

    #[cfg(target_os = "linux")]
    impl ScratchDir {
        fn new(label: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should follow the Unix epoch")
                .as_nanos();
            let path =
                env::temp_dir().join(format!("ghostram-probe-{label}-{}-{unique}", process::id()));
            fs::create_dir_all(&path).expect("scratch directory should be creatable");
            Self { path }
        }
    }

    #[cfg(target_os = "linux")]
    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[cfg(target_os = "linux")]
    struct KillOnDrop(Option<u32>);

    #[cfg(target_os = "linux")]
    impl KillOnDrop {
        fn new(pid: u32) -> Self {
            Self(Some(pid))
        }

        fn kill_and_wait(mut self) -> bool {
            let Some(pid) = self.0.take() else {
                return true;
            };
            // SAFETY: the pid belongs to a test-created process; SIGKILL has no
            // memory-safety implications. A dead or reused pid is not expected
            // while this guard exists.
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
            wait_until_not_running(pid)
        }
    }

    #[cfg(target_os = "linux")]
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            if let Some(pid) = self.0 {
                // SAFETY: best-effort test cleanup; see `kill_and_wait`.
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGKILL);
                }
            }
        }
    }
}
