//! zram discovery from sysfs.

use std::fs;

use ghostram_core::diagnostic::{CategoryReport, ZramDevice, ZramInfo};
use ghostram_core::quantity::Bytes;

const BLOCK_PATH: &str = "/sys/block";

pub(crate) fn collect_zram() -> (Option<ZramInfo>, CategoryReport) {
    let entries = match fs::read_dir(BLOCK_PATH) {
        Ok(entries) => entries,
        Err(error) => {
            return (
                None,
                CategoryReport::unavailable(format!("{BLOCK_PATH}: {error}")),
            );
        }
    };

    let mut devices = Vec::new();
    let mut errors = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(error.to_string());
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("zram") {
            continue;
        }

        let disksize_path = entry.path().join("disksize");
        let disksize = match fs::read_to_string(&disksize_path)
            .ok()
            .and_then(|value| parse_disksize(&value))
        {
            Some(value) => value,
            None => {
                errors.push(format!("{} could not be parsed", disksize_path.display()));
                continue;
            }
        };
        let algorithm = fs::read_to_string(entry.path().join("comp_algorithm"))
            .ok()
            .and_then(|value| parse_comp_algorithm(&value));
        devices.push(ZramDevice {
            name,
            disksize_bytes: disksize,
            algorithm,
        });
    }

    devices.sort_by(|left, right| left.name.cmp(&right.name));
    let report = if errors.is_empty() {
        CategoryReport::available()
    } else {
        CategoryReport::partial(format!(
            "some zram device details are unavailable: {}",
            errors.join("; ")
        ))
    };
    (Some(ZramInfo { devices }), report)
}

fn parse_disksize(text: &str) -> Option<Bytes> {
    text.trim().parse::<u64>().ok().map(Bytes::new)
}

fn parse_comp_algorithm(text: &str) -> Option<String> {
    text.split_whitespace()
        .find_map(|value| {
            value
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{parse_comp_algorithm, parse_disksize};
    use ghostram_core::quantity::Bytes;

    #[test]
    fn finds_the_selected_compression_algorithm() {
        assert_eq!(
            parse_comp_algorithm("[zstd] lz4 lz4hc lzo\n").as_deref(),
            Some("zstd")
        );
    }

    #[test]
    fn returns_none_when_no_algorithm_is_marked() {
        assert_eq!(parse_comp_algorithm("lz4 zstd\n"), None);
    }

    #[test]
    fn parses_exact_disksize_bytes() {
        assert_eq!(
            parse_disksize("7989100544\n"),
            Some(Bytes::new(7_989_100_544))
        );
        assert_eq!(parse_disksize("not-a-size"), None);
    }
}
