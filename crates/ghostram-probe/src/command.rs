//! Bounded, auditable subprocess execution.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use ghostram_core::error::GhostramError;

const OUTPUT_LIMIT: usize = 64 * 1024;
const ERROR_LIMIT: usize = 16 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

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

fn read_bounded(mut reader: impl Read, limit: usize) -> std::io::Result<(String, bool)> {
    let mut captured = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut truncated = false;

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        let remaining = limit.saturating_sub(captured.len());
        if remaining > 0 {
            let take = read.min(remaining);
            captured.extend_from_slice(&buffer[..take]);
        }
        if read > remaining {
            truncated = true;
        }
    }

    Ok((String::from_utf8_lossy(&captured).into_owned(), truncated))
}

/// Runs a program with an explicit argument array and a bounded timeout.
///
/// The child inherits no stdin. stdout and stderr are captured with memory
/// limits. On timeout the child is killed and reaped before returning. No shell
/// is involved.
pub(crate) fn run_bounded(
    program: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandObservation, GhostramError> {
    let program_text = program.display().to_string();
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| GhostramError::Command {
            program: program_text.clone(),
            message: error.to_string(),
        })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_handle = stdout.map(|pipe| thread::spawn(move || read_bounded(pipe, OUTPUT_LIMIT)));
    let stderr_handle = stderr.map(|pipe| thread::spawn(move || read_bounded(pipe, ERROR_LIMIT)));

    let started = Instant::now();
    let mut timed_out = false;
    let exit_status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GhostramError::Command {
                    program: program_text,
                    message: error.to_string(),
                });
            }
        }
    };

    let (stdout, _stdout_truncated) = stdout_handle
        .and_then(|handle| handle.join().ok())
        .and_then(Result::ok)
        .unwrap_or_default();
    let (stderr, _stderr_truncated) = stderr_handle
        .and_then(|handle| handle.join().ok())
        .and_then(Result::ok)
        .unwrap_or_default();

    Ok(CommandObservation {
        stdout,
        stderr,
        exit_code: exit_status.and_then(|status| status.code()),
        timed_out,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::run_bounded;

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

    #[test]
    fn kills_a_timed_out_child() {
        let result =
            run_bounded(Path::new("/bin/sleep"), &["2"], Duration::from_millis(50)).unwrap();
        assert!(result.timed_out);
        assert_eq!(result.exit_code, None);
    }
}
