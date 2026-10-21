//! `ghostram` command-line entry point.

mod render;

use std::fmt;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use ghostram_core::time::now_rfc3339;
use ghostram_models::{ScanOptions, scan_directory};
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
        Some("models") => run_models(&arguments[1..]),
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
        write_json(&envelope, "diagnostic report")
    } else {
        write_text(&render::render_doctor(&envelope))
    }
}

fn run_models(arguments: &[String]) -> Result<(), CliError> {
    match arguments.first().map(String::as_str) {
        None | Some("--help" | "-h" | "help") => print_models_help(),
        Some("scan") => run_models_scan(&arguments[1..]),
        Some(command) => Err(CliError::Usage(format!(
            "unrecognized model command `{command}`"
        ))),
    }
}

fn run_models_scan(arguments: &[String]) -> Result<(), CliError> {
    let Some(parsed) = parse_models_scan_arguments(arguments)? else {
        return print_models_scan_help();
    };
    let envelope = scan_directory(
        Path::new(&parsed.path),
        parsed.options,
        APPLICATION_VERSION,
        &now_rfc3339(),
    )
    .map_err(|error| CliError::Operation {
        message: format!("model scan failed: {error}"),
    })?;

    if parsed.json {
        write_json(&envelope, "model scan")
    } else {
        write_text(&render::render_models_scan(&envelope))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ModelsScanArguments {
    path: String,
    options: ScanOptions,
    json: bool,
}

fn parse_models_scan_arguments(
    arguments: &[String],
) -> Result<Option<ModelsScanArguments>, CliError> {
    let mut recursive = false;
    let mut hash = false;
    let mut json = false;
    let mut path = None;
    let mut positional_only = false;

    for argument in arguments {
        if !positional_only {
            match argument.as_str() {
                "--help" | "-h" => return Ok(None),
                "--recursive" if !recursive => {
                    recursive = true;
                    continue;
                }
                "--recursive" => {
                    return Err(CliError::Usage(
                        "`--recursive` may be specified only once".to_string(),
                    ));
                }
                "--hash" if !hash => {
                    hash = true;
                    continue;
                }
                "--hash" => {
                    return Err(CliError::Usage(
                        "`--hash` may be specified only once".to_string(),
                    ));
                }
                "--json" if !json => {
                    json = true;
                    continue;
                }
                "--json" => {
                    return Err(CliError::Usage(
                        "`--json` may be specified only once".to_string(),
                    ));
                }
                "--" => {
                    positional_only = true;
                    continue;
                }
                other if other.starts_with('-') && other != "-" => {
                    return Err(CliError::Usage(format!(
                        "unexpected option `{other}` for `ghostram models scan`"
                    )));
                }
                _ => {}
            }
        }
        if path.is_some() {
            return Err(CliError::Usage(
                "`ghostram models scan` accepts exactly one directory path".to_string(),
            ));
        }
        path = Some(argument.clone());
    }

    let path = path.ok_or_else(|| {
        CliError::Usage("`ghostram models scan` requires a directory path".to_string())
    })?;
    if path.is_empty() {
        return Err(CliError::Usage(
            "model scan path must not be empty; use `.` for the current directory".to_string(),
        ));
    }
    Ok(Some(ModelsScanArguments {
        path,
        options: ScanOptions { recursive, hash },
        json,
    }))
}

fn write_json<T: serde::Serialize>(value: &T, description: &str) -> Result<(), CliError> {
    let mut encoded = serde_json::to_vec_pretty(value).map_err(|error| CliError::Output {
        message: format!("could not serialize the {description}: {error}"),
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
Read-only local LLM diagnostics and model discovery.

USAGE:
    ghostram doctor [--json]
    ghostram models scan <path> [--recursive] [--hash] [--json]
    ghostram --help
    ghostram --version

COMMANDS:
    doctor          Inspect OS, kernel, CPU, memory, swap/zram, storage,
                    NVIDIA GPU, Vulkan devices, tools, and engine paths.
    models scan     Read compact GGUF metadata for candidate files under one
                    explicitly selected directory.

OPTIONS:
    --json          Print only the versioned JSON report on stdout.
    -h, --help      Print help. Use with a command for command help.
    -V, --version   Print the application version.

EXIT CODES:
    0               The command completed, including an empty or partial scan.
    1               The operation could not be completed or output failed.
    2               The invocation was invalid.

GhostRAM is read-only. It does not install engines or models, start inference,
benchmark hardware, download files, or change system settings.
"
    );
    write_text(&text)
}

fn print_models_help() -> Result<(), CliError> {
    let text = format!(
        "\
GhostRAM {APPLICATION_VERSION}
Read compact GGUF catalog metadata without loading model tensors.

USAGE:
    ghostram models scan <path> [--recursive] [--hash] [--json]

COMMANDS:
    scan    Inspect candidate `.gguf` files under one selected directory.

There is no implicit model directory and no cache or model database. Symlinks
are not followed. Run `ghostram models scan --help` for scan details.
"
    );
    write_text(&text)
}

fn print_models_scan_help() -> Result<(), CliError> {
    let text = format!(
        "\
GhostRAM {APPLICATION_VERSION}
Scan one explicitly selected directory for GGUF metadata.

USAGE:
    ghostram models scan <path> [--recursive] [--hash] [--json]

ARGUMENTS:
    <path>          Directory to inspect. No default directory is assumed.

OPTIONS:
    --recursive     Descend into subdirectories. Symlinks are never followed.
    --hash          Stream SHA-256 for every readable candidate. This reads the
                    complete file, including tensor payloads.
    --json          Print only the versioned `ghostram.models.scan/1` JSON.
    -h, --help      Print this help.

Only regular files whose names end in `.gguf` (ASCII case-insensitive) are
cataloged. Tensor payloads are never parsed during metadata extraction. The
command never writes to the selected tree, caches results, or contacts the
network. A completed scan with malformed, unsupported, unreadable, or changed
files exits 0 and reports those outcomes in the result.
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
    Operation { message: String },
    Output { message: String },
}

impl CliError {
    const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Operation { .. } | Self::Output { .. } => 1,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Operation { message } | Self::Output { message } => {
                formatter.write_str(message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ModelsScanArguments, parse_models_scan_arguments, run};
    use ghostram_models::ScanOptions;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn rejects_unknown_commands() {
        let error = run(&arguments(&["unknown"])).unwrap_err();
        assert!(error.to_string().contains("unrecognized"));
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn rejects_unknown_doctor_options() {
        let error = run(&arguments(&["doctor", "--bogus"])).unwrap_err();
        assert!(error.to_string().contains("unexpected argument"));
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn rejects_duplicate_json_flags() {
        let error = run(&arguments(&["doctor", "--json", "--json"])).unwrap_err();
        assert!(error.to_string().contains("only once"));
    }

    #[test]
    fn parses_models_scan_options_and_path() {
        let parsed =
            parse_models_scan_arguments(&arguments(&["--recursive", "--hash", "some/directory"]))
                .unwrap()
                .unwrap();
        assert_eq!(
            parsed,
            ModelsScanArguments {
                path: "some/directory".to_string(),
                options: ScanOptions {
                    recursive: true,
                    hash: true,
                },
                json: false,
            }
        );
    }

    #[test]
    fn models_scan_requires_one_path_and_rejects_duplicates() {
        let error = parse_models_scan_arguments(&arguments(&[])).unwrap_err();
        assert!(error.to_string().contains("requires a directory path"));
        assert_eq!(error.exit_code(), 2);

        let error = parse_models_scan_arguments(&arguments(&["first", "second"])).unwrap_err();
        assert!(error.to_string().contains("exactly one"));

        let error =
            parse_models_scan_arguments(&arguments(&["--hash", "--hash", "path"])).unwrap_err();
        assert!(error.to_string().contains("only once"));
    }

    #[test]
    fn models_scan_supports_an_option_terminator() {
        let parsed = parse_models_scan_arguments(&arguments(&["--", "--odd-path"]))
            .unwrap()
            .unwrap();
        assert_eq!(parsed.path, "--odd-path");
    }

    #[test]
    fn models_scan_rejects_unknown_options_and_empty_paths() {
        let error = parse_models_scan_arguments(&arguments(&["--bogus", "path"])).unwrap_err();
        assert!(error.to_string().contains("unexpected option"));

        let error = parse_models_scan_arguments(&arguments(&[""])).unwrap_err();
        assert!(error.to_string().contains("must not be empty"));
    }

    #[test]
    fn models_scan_help_is_not_an_invocation_error() {
        assert!(
            parse_models_scan_arguments(&arguments(&["--help"]))
                .unwrap()
                .is_none()
        );
    }
}
