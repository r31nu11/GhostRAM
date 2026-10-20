//! Discovery of externally installed llama.cpp executables.
//!
//! Only known executable names on `PATH` are inspected. Discovery never installs
//! anything, never launches an inference server, and only runs bounded read-only
//! version/device queries.

use std::path::Path;
use std::time::Duration;

use ghostram_core::diagnostic::{CategoryReport, DiagnosticState, EngineDiscovery, EngineFamily};

use crate::command::run_bounded;
use crate::tools::{find_executable, version_from};

const VERSION_TIMEOUT: Duration = Duration::from_secs(3);
const DEVICE_TIMEOUT: Duration = Duration::from_secs(5);
const ENGINE_NAMES: &[&str] = &["llama-server", "llama-bench", "llama-cli"];

pub(crate) fn collect_engines() -> (Vec<EngineDiscovery>, CategoryReport) {
    let mut engines = Vec::new();
    let mut partial_reasons = Vec::new();

    for name in ENGINE_NAMES {
        let Some(path) = find_executable(name) else {
            continue;
        };

        let mut reasons = Vec::new();
        let version = match run_bounded(&path, &["--version"], VERSION_TIMEOUT) {
            Ok(observation) if observation.timed_out => {
                reasons.push("version query timed out".to_string());
                None
            }
            Ok(observation) if !observation.succeeded() => {
                let detail = observation
                    .first_output_line()
                    .unwrap_or_else(|| "no output".to_string());
                reasons.push(format!("version query failed: {detail}"));
                version_from(&observation)
            }
            Ok(observation) => version_from(&observation),
            Err(error) => {
                reasons.push(format!("version query could not start: {error}"));
                None
            }
        };
        if version.is_none() && reasons.is_empty() {
            reasons.push("version query returned no version text".to_string());
        }

        let devices = if *name == "llama-server" {
            match collect_devices(&path) {
                Ok(lines) => Some(lines),
                Err(reason) => {
                    reasons.push(reason);
                    None
                }
            }
        } else {
            None
        };

        let state = if reasons.is_empty() {
            DiagnosticState::Available
        } else {
            DiagnosticState::Partial
        };
        let reason = (!reasons.is_empty()).then(|| reasons.join("; "));
        if let Some(reason) = &reason {
            partial_reasons.push(format!("{name}: {reason}"));
        }

        engines.push(EngineDiscovery {
            executable: (*name).to_string(),
            family: EngineFamily::LlamaCpp,
            path: path.to_string_lossy().into_owned(),
            version,
            devices,
            state,
            reason,
        });
    }

    let category = if partial_reasons.is_empty() {
        if engines.is_empty() {
            CategoryReport::available_with_reason("no llama.cpp executables were found on PATH")
        } else {
            CategoryReport::available()
        }
    } else {
        CategoryReport::partial(format!(
            "some engine probes were inconclusive: {}",
            partial_reasons.join("; ")
        ))
    };
    (engines, category)
}

fn collect_devices(path: &Path) -> Result<Vec<String>, String> {
    let observation = run_bounded(path, &["--list-devices"], DEVICE_TIMEOUT)
        .map_err(|error| format!("device query could not start: {error}"))?;
    if observation.timed_out {
        return Err("device query timed out".to_string());
    }
    if !observation.succeeded() {
        let detail = observation
            .first_output_line()
            .unwrap_or_else(|| "no output".to_string());
        return Err(format!("device query failed: {detail}"));
    }
    Ok(parse_device_lines(&observation.stdout))
}

fn parse_device_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(40)
        .map(|line| {
            let mut shortened: String = line.chars().take(237).collect();
            if line.chars().count() > 237 {
                shortened.push_str("...");
            }
            shortened
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_device_lines;
    use crate::command::CommandObservation;
    use crate::tools::version_from;

    #[test]
    fn parses_a_fixture_version_line() {
        let observation = CommandObservation {
            stdout: include_str!("../tests/fixtures/llama-version.txt").to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            timed_out: false,
        };
        assert_eq!(
            version_from(&observation).as_deref(),
            Some("version: 0.1.0-test (fixture build)")
        );
    }

    #[test]
    fn parses_fixture_device_lines() {
        let lines = parse_device_lines(include_str!("../tests/fixtures/llama-devices.txt"));
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Available devices:");
        assert!(lines[1].starts_with("CUDA0:"));
    }

    #[test]
    fn trims_and_preserves_device_lines() {
        let lines = parse_device_lines("Available devices:\n  CUDA0: GTX 1050 Ti\n\n");
        assert_eq!(lines, vec!["Available devices:", "CUDA0: GTX 1050 Ti"]);
    }

    #[test]
    fn bounds_the_device_list() {
        let text = (0..50)
            .map(|index| format!("device {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(parse_device_lines(&text).len(), 40);
    }
}
