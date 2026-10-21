//! `ghostram` command-line entry point.

mod render;

use std::fmt;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use ghostram_core::quantity::Bytes;
use ghostram_core::time::now_rfc3339;
use ghostram_models::{ScanOptions, inspect_file, scan_directory};
use ghostram_planner::{
    DeviceSelection, Evidence, KvCacheType, ModelInput, PlanEnvelope, PlanOptions,
    ResourceSnapshot, build_plan,
};
use ghostram_probe::{collect_planning_observations, collect_report};

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
        Some("plan") => run_models_plan(&arguments[1..]),
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

fn run_models_plan(arguments: &[String]) -> Result<(), CliError> {
    let Some(parsed) = parse_models_plan_arguments(arguments)? else {
        return print_models_plan_help();
    };
    let entry = inspect_file(Path::new(&parsed.path)).map_err(|error| CliError::Operation {
        message: format!("model plan failed: {error}"),
    })?;
    let ghostram_probe::PlanningObservations {
        observed_at,
        memory,
        memory_state,
        nvidia_gpus,
        nvidia_state,
    } = collect_planning_observations();
    let resources = ResourceSnapshot::from_observations(
        observed_at,
        memory_state,
        memory.as_ref(),
        nvidia_state,
        &nvidia_gpus,
    );
    let model = ModelInput::from(&entry);
    let plan = build_plan(&model, &resources, &parsed.options)
        .map_err(|error| CliError::Usage(error.to_string()))?;
    let envelope = PlanEnvelope::new(APPLICATION_VERSION, now_rfc3339(), plan);

    if parsed.json {
        write_json(&envelope, "memory plan")
    } else {
        write_text(&render::render_plan(&envelope))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ModelsPlanArguments {
    path: String,
    options: PlanOptions,
    json: bool,
}

fn parse_models_plan_arguments(
    arguments: &[String],
) -> Result<Option<ModelsPlanArguments>, CliError> {
    let mut options = PlanOptions::product_defaults();
    let mut json = false;
    let mut path = None;
    let mut positional_only = false;
    let mut seen_context = false;
    let mut seen_sequences = false;
    let mut seen_kv_cache = false;
    let mut seen_ram_reserve = false;
    let mut seen_vram_reserve = false;
    let mut seen_device = false;

    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if !positional_only {
            match argument.as_str() {
                "--help" | "-h" => return Ok(None),
                "--json" if !json => {
                    json = true;
                    index += 1;
                    continue;
                }
                "--json" => {
                    return Err(CliError::Usage(
                        "`--json` may be specified only once".to_string(),
                    ));
                }
                "--context" => {
                    reject_duplicate(&mut seen_context, "--context")?;
                    let value = next_option_value(arguments, index, "--context")?;
                    options.context_tokens =
                        Evidence::explicit_input(parse_u64(value, "--context")?);
                    index += 2;
                    continue;
                }
                "--sequences" => {
                    reject_duplicate(&mut seen_sequences, "--sequences")?;
                    let value = next_option_value(arguments, index, "--sequences")?;
                    options.sequences = Evidence::explicit_input(parse_u64(value, "--sequences")?);
                    index += 2;
                    continue;
                }
                "--kv-cache" => {
                    reject_duplicate(&mut seen_kv_cache, "--kv-cache")?;
                    let value = next_option_value(arguments, index, "--kv-cache")?;
                    let cache_type = match value.to_ascii_lowercase().as_str() {
                        "f16" => KvCacheType::F16,
                        "f32" => KvCacheType::F32,
                        _ => {
                            return Err(CliError::Usage(format!(
                                "`--kv-cache` must be `f16` or `f32`, got `{value}`"
                            )));
                        }
                    };
                    options.kv_cache_type = Evidence::explicit_input(cache_type);
                    index += 2;
                    continue;
                }
                "--ram-reserve-mib" => {
                    reject_duplicate(&mut seen_ram_reserve, "--ram-reserve-mib")?;
                    let value = next_option_value(arguments, index, "--ram-reserve-mib")?;
                    options.ram_reserve =
                        Evidence::explicit_input(parse_mib(value, "--ram-reserve-mib")?);
                    index += 2;
                    continue;
                }
                "--vram-reserve-mib" => {
                    reject_duplicate(&mut seen_vram_reserve, "--vram-reserve-mib")?;
                    let value = next_option_value(arguments, index, "--vram-reserve-mib")?;
                    options.vram_reserve =
                        Evidence::explicit_input(parse_mib(value, "--vram-reserve-mib")?);
                    index += 2;
                    continue;
                }
                "--device" => {
                    reject_duplicate(&mut seen_device, "--device")?;
                    let value = next_option_value(arguments, index, "--device")?;
                    let index_value = value.parse::<u32>().map_err(|_| {
                        CliError::Usage(format!(
                            "`--device` must be a non-negative integer device index, got `{value}`"
                        ))
                    })?;
                    options.device_selection = DeviceSelection::Explicit { index: index_value };
                    index += 2;
                    continue;
                }
                "--" => {
                    positional_only = true;
                    index += 1;
                    continue;
                }
                other if other.starts_with('-') && other != "-" => {
                    return Err(CliError::Usage(format!(
                        "unexpected option `{other}` for `ghostram models plan`"
                    )));
                }
                _ => {}
            }
        }
        if path.is_some() {
            return Err(CliError::Usage(
                "`ghostram models plan` accepts exactly one model file path".to_string(),
            ));
        }
        path = Some(argument.clone());
        index += 1;
    }

    let path = path.ok_or_else(|| {
        CliError::Usage("`ghostram models plan` requires a GGUF file path".to_string())
    })?;
    if path.is_empty() {
        return Err(CliError::Usage(
            "model plan path must not be empty".to_string(),
        ));
    }
    options
        .validate()
        .map_err(|error| CliError::Usage(error.to_string()))?;

    Ok(Some(ModelsPlanArguments {
        path,
        options,
        json,
    }))
}

fn reject_duplicate(seen: &mut bool, flag: &str) -> Result<(), CliError> {
    if *seen {
        return Err(CliError::Usage(format!(
            "`{flag}` may be specified only once"
        )));
    }
    *seen = true;
    Ok(())
}

fn next_option_value<'a>(
    arguments: &'a [String],
    index: usize,
    flag: &str,
) -> Result<&'a str, CliError> {
    arguments
        .get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage(format!("`{flag}` requires a value")))
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, CliError> {
    value.parse::<u64>().map_err(|_| {
        CliError::Usage(format!(
            "`{flag}` must be a non-negative integer, got `{value}`"
        ))
    })
}

fn parse_mib(value: &str, flag: &str) -> Result<Bytes, CliError> {
    let mib = parse_u64(value, flag)?;
    Bytes::from_mib(mib).ok_or_else(|| {
        CliError::Usage(format!(
            "`{flag}` value {mib} MiB cannot be represented in bytes"
        ))
    })
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
    ghostram models plan <path> [options] [--json]
    ghostram --help
    ghostram --version

COMMANDS:
    doctor          Inspect OS, kernel, CPU, memory, swap/zram, storage,
                    NVIDIA GPU, Vulkan devices, tools, and engine paths.
    models scan     Read compact GGUF metadata for candidate files under one
                    explicitly selected directory.
    models plan     Estimate modeled weights and KV cache against host and
                    device budgets for one explicitly selected GGUF file.

OPTIONS:
    --json          Print only the versioned JSON report on stdout.
    -h, --help      Print help. Use with a command for command help.
    -V, --version   Print the application version.

EXIT CODES:
    0               The command completed, including an empty or partial scan
                    or a plan with unavailable components.
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
    ghostram models plan <path> [options] [--json]

COMMANDS:
    scan    Inspect candidate `.gguf` files under one selected directory.
    plan    Estimate modeled weights and KV cache for one selected `.gguf` file.

There is no implicit model directory and no cache or model database. Symlinks
are not followed. Run `ghostram models scan --help` or
`ghostram models plan --help` for details.
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

fn print_models_plan_help() -> Result<(), CliError> {
    let text = format!(
        "\
GhostRAM {APPLICATION_VERSION}
Estimate a memory budget for one explicitly selected GGUF file.

USAGE:
    ghostram models plan <path> [options] [--json]

ARGUMENTS:
    <path>              GGUF file to inspect. Directories and symbolic links
                        are rejected; no file is auto-selected.

OPTIONS:
    --context <tokens>  Context length per sequence. Default: 4096.
    --sequences <n>     Independent full-context sequences. Default: 1.
    --kv-cache <type>   KV-cache element type: f16 or f32. Default: f16.
    --ram-reserve-mib <mib>
                        Host RAM withheld from the budget. Default: 1536.
    --vram-reserve-mib <mib>
                        Device VRAM withheld from the budget. Default: 1024.
    --device <index>    Use an explicit NVIDIA device index. Default: first
                        observed device.
    --json              Print only the versioned `ghostram.models.plan/1` JSON.
    -h, --help          Print this help.

The planner reads the selected file's GGUF metadata, then collects host memory
and NVIDIA device observations. It does not scan directories, hash files, use a
cache or database, download anything, or start an inference engine.

Model weights are a low-confidence proxy based on GGUF file size. The KV cache
is estimated only for uniform full-attention `llama` models. Scenario outcomes
cover modeled weights and KV cache only; they do not mean that a model will
load, launch, or run safely. Backend workspace and runtime overhead are
unverified.

A readable file with malformed or unsupported GGUF content exits 0 and reports
unknown or insufficient components. A missing, non-regular, or symbolic-link
path exits 1.
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
    use super::{
        ModelsPlanArguments, ModelsScanArguments, parse_models_plan_arguments,
        parse_models_scan_arguments, run,
    };
    use ghostram_core::quantity::Bytes;
    use ghostram_models::ScanOptions;
    use ghostram_planner::{DeviceSelection, Evidence, EvidenceClass, KvCacheType, PlanOptions};

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

    #[test]
    fn parses_models_plan_defaults_and_path() {
        let parsed = parse_models_plan_arguments(&arguments(&["some/model.gguf"]))
            .unwrap()
            .unwrap();
        assert_eq!(parsed.path, "some/model.gguf");
        assert!(!parsed.json);
        assert_eq!(parsed.options, PlanOptions::product_defaults());
        assert_eq!(
            parsed.options.context_tokens.class(),
            EvidenceClass::ProductDefault
        );
        assert_eq!(
            parsed.options.device_selection,
            DeviceSelection::FirstObserved
        );
    }

    #[test]
    fn parses_models_plan_explicit_values() {
        let parsed = parse_models_plan_arguments(&arguments(&[
            "--context",
            "2048",
            "--sequences",
            "2",
            "--kv-cache",
            "f32",
            "--ram-reserve-mib",
            "512",
            "--vram-reserve-mib",
            "256",
            "--device",
            "1",
            "--json",
            "some/model.gguf",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(
            parsed,
            ModelsPlanArguments {
                path: "some/model.gguf".to_string(),
                options: PlanOptions {
                    context_tokens: Evidence::explicit_input(2048),
                    sequences: Evidence::explicit_input(2),
                    kv_cache_type: Evidence::explicit_input(KvCacheType::F32),
                    ram_reserve: Evidence::explicit_input(Bytes::from_mib(512).unwrap()),
                    vram_reserve: Evidence::explicit_input(Bytes::from_mib(256).unwrap()),
                    device_selection: DeviceSelection::Explicit { index: 1 },
                },
                json: true,
            }
        );
    }

    #[test]
    fn models_plan_rejects_duplicates_unknown_options_and_missing_paths() {
        let error = parse_models_plan_arguments(&arguments(&[])).unwrap_err();
        assert!(error.to_string().contains("requires a GGUF file path"));
        assert_eq!(error.exit_code(), 2);

        let error = parse_models_plan_arguments(&arguments(&["a.gguf", "b.gguf"])).unwrap_err();
        assert!(error.to_string().contains("exactly one"));

        let error = parse_models_plan_arguments(&arguments(&[
            "--context",
            "1",
            "--context",
            "2",
            "a.gguf",
        ]))
        .unwrap_err();
        assert!(error.to_string().contains("only once"));

        let error =
            parse_models_plan_arguments(&arguments(&["--json", "--json", "a.gguf"])).unwrap_err();
        assert!(error.to_string().contains("only once"));

        let error = parse_models_plan_arguments(&arguments(&["--bogus", "a.gguf"])).unwrap_err();
        assert!(error.to_string().contains("unexpected option"));
    }

    #[test]
    fn models_plan_validates_ranges_types_and_reserve_overflow() {
        let error =
            parse_models_plan_arguments(&arguments(&["--context", "0", "a.gguf"])).unwrap_err();
        assert!(error.to_string().contains("context_tokens"));

        let error =
            parse_models_plan_arguments(&arguments(&["--sequences", "0", "a.gguf"])).unwrap_err();
        assert!(error.to_string().contains("sequences"));

        let error =
            parse_models_plan_arguments(&arguments(&["--kv-cache", "q4", "a.gguf"])).unwrap_err();
        assert!(error.to_string().contains("f16"));

        let error = parse_models_plan_arguments(&arguments(&[
            "--ram-reserve-mib",
            "18446744073709551615",
            "a.gguf",
        ]))
        .unwrap_err();
        assert!(error.to_string().contains("cannot be represented"));

        let error =
            parse_models_plan_arguments(&arguments(&["--device", "-1", "a.gguf"])).unwrap_err();
        assert!(error.to_string().contains("non-negative integer"));
    }

    #[test]
    fn models_plan_supports_an_option_terminator_and_help() {
        let parsed = parse_models_plan_arguments(&arguments(&["--", "--odd-path"]))
            .unwrap()
            .unwrap();
        assert_eq!(parsed.path, "--odd-path");

        assert!(
            parse_models_plan_arguments(&arguments(&["--help"]))
                .unwrap()
                .is_none()
        );
    }
}
