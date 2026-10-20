//! Operating-system and kernel discovery.

use std::fs;
use std::path::Path;

use ghostram_core::diagnostic::{CategoryReport, OsInfo};

const OS_RELEASE_PATHS: &[&str] = &["/etc/os-release", "/usr/lib/os-release"];
const KERNEL_RELEASE_PATH: &str = "/proc/sys/kernel/osrelease";

pub(crate) fn collect_os() -> (Option<OsInfo>, CategoryReport) {
    let mut info = OsInfo {
        name: None,
        version: None,
        id: None,
        kernel: fs::read_to_string(KERNEL_RELEASE_PATH)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        architecture: Some(std::env::consts::ARCH.to_string()),
    };

    let mut release_error = None;
    for path in OS_RELEASE_PATHS {
        match fs::read_to_string(Path::new(path)) {
            Ok(text) => {
                let parsed = parse_os_release(&text);
                info.name = parsed.name;
                info.version = parsed.version;
                info.id = parsed.id;
                break;
            }
            Err(error) => release_error = Some(format!("{path}: {error}")),
        }
    }

    let has_os_data = info.name.is_some() || info.id.is_some();
    let category =
        if has_os_data && info.kernel.is_some() {
            CategoryReport::available()
        } else if has_os_data || info.kernel.is_some() {
            CategoryReport::partial("some operating-system fields are unavailable")
        } else {
            CategoryReport::unavailable(release_error.unwrap_or_else(|| {
                "no OS release metadata or kernel release was readable".to_string()
            }))
        };

    if has_os_data || info.kernel.is_some() {
        (Some(info), category)
    } else {
        (None, category)
    }
}

fn parse_os_release(text: &str) -> OsInfo {
    OsInfo {
        name: field(text, "PRETTY_NAME").or_else(|| field(text, "NAME")),
        version: field(text, "VERSION").or_else(|| field(text, "VERSION_ID")),
        id: field(text, "ID"),
        kernel: None,
        architecture: None,
    }
}

fn field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (candidate, value) = line.split_once('=')?;
        if candidate.trim() != key {
            return None;
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value);
        let value = value
            .replace("\\\"", "\"")
            .replace("\\$", "$")
            .replace("\\\\", "\\");
        (!value.is_empty()).then_some(value)
    })
}

#[cfg(test)]
mod tests {
    use super::parse_os_release;

    #[test]
    fn parses_quoted_os_release_fields() {
        let info = parse_os_release(include_str!("../tests/fixtures/os-release.txt"));
        assert_eq!(info.name.as_deref(), Some("CachyOS Linux"));
        assert_eq!(info.version.as_deref(), Some("rolling"));
        assert_eq!(info.id.as_deref(), Some("cachyos"));
    }

    #[test]
    fn falls_back_to_name_when_pretty_name_is_absent() {
        let info = parse_os_release("NAME=Example\nVERSION_ID=1\nID=example\n");
        assert_eq!(info.name.as_deref(), Some("Example"));
        assert_eq!(info.version.as_deref(), Some("1"));
        assert_eq!(info.id.as_deref(), Some("example"));
    }

    #[test]
    fn ignores_malformed_and_empty_fields() {
        let info = parse_os_release("BROKEN\nPRETTY_NAME=\"\"\nID=test\n");
        assert_eq!(info.name, None);
        assert_eq!(info.id.as_deref(), Some("test"));
    }
}
