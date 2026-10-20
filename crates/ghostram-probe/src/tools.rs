//! PATH lookup and bounded tool version discovery.

use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ghostram_core::diagnostic::{CategoryReport, ToolInfo};

use crate::command::{CommandObservation, run_bounded};

const VERSION_TIMEOUT: Duration = Duration::from_secs(2);

struct ToolSpec {
    name: &'static str,
    version_args: &'static [&'static str],
}

const TOOL_SPECS: &[ToolSpec] = &[
    ToolSpec {
        name: "cmake",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "ninja",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "nvcc",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "cc",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "gcc",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "clang",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "git",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "python3",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "nvidia-smi",
        version_args: &["--version"],
    },
    ToolSpec {
        name: "vulkaninfo",
        version_args: &[],
    },
    ToolSpec {
        name: "ollama",
        version_args: &["--version"],
    },
];

/// Looks up an executable on `PATH` using explicit path components.
///
/// The function does not invoke a shell, does not scan the filesystem, and does
/// not follow paths outside the directories in `PATH`.
#[must_use]
pub(crate) fn find_executable(name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return None;
    }

    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path) {
        if directory.as_os_str().is_empty() {
            continue;
        }
        let candidate = directory.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

/// Collects tool paths and version text. Missing tools remain explicit entries.
pub(crate) fn collect_tools() -> (Vec<ToolInfo>, CategoryReport) {
    let mut tools = Vec::with_capacity(TOOL_SPECS.len());
    let mut version_failures = Vec::new();

    for spec in TOOL_SPECS {
        let path = find_executable(spec.name);
        let version = match (&path, spec.version_args.is_empty()) {
            (Some(path), false) => match run_bounded(path, spec.version_args, VERSION_TIMEOUT) {
                Ok(observation) if observation.timed_out => {
                    version_failures.push(format!("{}: version query timed out", spec.name));
                    None
                }
                Ok(observation) if !observation.succeeded() => {
                    version_failures.push(format!(
                        "{}: version query exited with status {:?}",
                        spec.name, observation.exit_code
                    ));
                    observation.first_output_line()
                }
                Ok(observation) => observation.first_output_line(),
                Err(error) => {
                    version_failures.push(format!("{}: {error}", spec.name));
                    None
                }
            },
            _ => None,
        };

        tools.push(ToolInfo {
            name: spec.name.to_string(),
            path: path.map(|path| path.to_string_lossy().into_owned()),
            version,
        });
    }

    let report = if version_failures.is_empty() {
        CategoryReport::available()
    } else {
        CategoryReport::partial(format!(
            "some tool version probes failed: {}",
            version_failures.join("; ")
        ))
    };
    (tools, report)
}

/// Returns the first useful line from a command observation, when present.
pub(crate) fn version_from(observation: &CommandObservation) -> Option<String> {
    observation.first_output_line()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{find_executable, is_executable};

    #[test]
    fn rejects_path_like_names() {
        assert_eq!(find_executable(""), None);
        assert_eq!(find_executable("../bin/sh"), None);
        assert_eq!(find_executable("/bin/sh"), None);
        assert_eq!(find_executable("."), None);
    }

    #[test]
    fn treats_a_known_binary_as_executable() {
        assert!(is_executable(Path::new("/bin/sh")));
    }

    #[test]
    fn treats_a_directory_as_non_executable() {
        assert!(!is_executable(Path::new("/")));
    }
}
