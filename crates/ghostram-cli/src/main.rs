//! `ghostram` command-line entry point.

mod render;

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

use ghostram_core::diagnostic::DiagnosticEnvelope;
use ghostram_probe::collect_report;

const APPLICATION_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();

    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ghostram: {error}");
            if matches!(error, CliError::Usage(_)) {
                eprintln!("Try 'ghostram --help' for usage.");
            }
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(arguments: &[String]) -> Result<(), CliError> {
    match arguments.first().map(String::as_str) {
        None | Some("--help" | "-h" | "help") => print_help(),
        Some("--version" | "-V") => {
            if arguments.len() > 1 {
                return Err(CliError::Usage(
                    "`--version` does not accept additional arguments".to_string(),
                ));
            }
            print_line(&format!("ghostram {APPLICATION_VERSION}"))
        }
        Some("doctor") => run_doctor(&arguments[1..]),
        Some(command) => Err(CliError::Usage(format!(
            "unrecognized command or option `{command}`"
        ))),
    }
}

fn run_doctor(arguments: &[String]) -> Result<(), CliError> {
    let mut json = false;
    for argument in arguments {
        match argument.as_str() {
            "--json" if !json => json = true,
            "--json" => {
                return Err(CliError::Usage(
                    "`--json` may be specified only once".to_string(),
                ));
            }
            "--help" | "-h" => return print_doctor_help(),
            other => {
                return Err(CliError::Usage(format!(
                    "unexpected argument `{other}` for `ghostram doctor`"
                )));
            }
        }
    }

    let envelope = collect_report(APPLICATION_VERSION);
    if json {
        write_json(&envelope)
    } else {
        write_text(&render::render_doctor(&envelope))
    }
}

fn write_json(envelope: &DiagnosticEnvelope) -> Result<(), CliError> {
    let mut encoded = serde_json::to_vec_pretty(envelope).map_err(|error| CliError::Output {
        message: format!("could not serialize the diagnostic report: {error}"),
    })?;
    encoded.push(b'\n');
    write_stdout(&encoded)
}

fn write_text(text: &str) -> Result<(), CliError> {
    write_stdout(text.as_bytes())
}

fn write_stdout(bytes: &[u8]) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(bytes)
        .and_then(|()| handle.flush())
        .map_err(|error| CliError::Output {
            message: format!("could not write to stdout: {error}"),
        })
}

fn print_help() -> Result<(), CliError> {
    let text = format!(
        "\
GhostRAM {APPLICATION_VERSION}
Read-only diagnostics for local LLM hardware and engine discovery.

USAGE:
    ghostram doctor [--json]
    ghostram --help
    ghostram --version

COMMANDS:
    doctor      Inspect OS, kernel, CPU, memory, swap/zram, storage,
                NVIDIA GPU, Vulkan devices, tools, and engine paths.

OPTIONS:
    --json      Print the versioned JSON report and nothing else on stdout.
    -h, --help  Print this help.
    -V, --version
                Print the application version.

EXIT CODES:
    0           The requested command completed.
    1           The report could not be generated or written.
    2           The invocation was invalid.

GhostRAM is read-only in this stage. It does not install engines or models,
start inference, benchmark hardware, or change system settings.
"
    );
    write_text(&text)
}

fn print_doctor_help() -> Result<(), CliError> {
    let text = format!(
        "\
GhostRAM {APPLICATION_VERSION}
Inspect this machine without modifying it.

USAGE:
    ghostram doctor [--json]

OPTIONS:
    --json      Print only the versioned JSON envelope on stdout.
    -h, --help  Print this help.

Missing optional tools, GPUs, Vulkan devices, and engines are reported as
unavailable or absent. They do not make the command fail.
"
    );
    write_text(&text)
}

fn print_line(line: &str) -> Result<(), CliError> {
    write_text(&format!("{line}\n"))
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Output { message: String },
}

impl CliError {
    const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Output { .. } => 1,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => formatter.write_str(message),
            Self::Output { message } => formatter.write_str(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn rejects_unknown_commands() {
        let error = run(&["unknown".to_string()]).unwrap_err();
        assert!(error.to_string().contains("unrecognized"));
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn rejects_unknown_doctor_options() {
        let error = run(&["doctor".to_string(), "--bogus".to_string()]).unwrap_err();
        assert!(error.to_string().contains("unexpected argument"));
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn rejects_duplicate_json_flags() {
        let error = run(&[
            "doctor".to_string(),
            "--json".to_string(),
            "--json".to_string(),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("only once"));
    }
}
