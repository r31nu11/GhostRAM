//! CPU information discovery from `/proc/cpuinfo`.

use std::collections::BTreeSet;
use std::fs;

use ghostram_core::diagnostic::{CategoryReport, CpuInfo};

const CPUINFO_PATH: &str = "/proc/cpuinfo";

#[derive(Default)]
struct CpuBlock {
    processor: bool,
    vendor: Option<String>,
    model: Option<String>,
    physical_id: Option<u32>,
    core_id: Option<u32>,
    cores_per_socket: Option<u32>,
}

pub(crate) fn collect_cpu() -> (Option<CpuInfo>, CategoryReport) {
    let Ok(text) = fs::read_to_string(CPUINFO_PATH) else {
        return (
            None,
            CategoryReport::unavailable(format!("{CPUINFO_PATH} could not be read")),
        );
    };

    let info = parse_cpuinfo(&text);
    let has_data = info.model.is_some() || info.vendor.is_some() || info.logical_cpus.is_some();
    if !has_data {
        return (
            None,
            CategoryReport::unavailable(format!("{CPUINFO_PATH} contained no CPU records")),
        );
    }

    let complete = info.model.is_some()
        && info.vendor.is_some()
        && info.architecture.is_some()
        && info.logical_cpus.is_some();
    let category = if complete {
        CategoryReport::available()
    } else {
        CategoryReport::partial("some CPU fields are unavailable")
    };
    (Some(info), category)
}

fn parse_cpuinfo(text: &str) -> CpuInfo {
    let mut blocks = Vec::new();
    let mut current = CpuBlock::default();

    for line in text.lines() {
        if line.trim().is_empty() {
            if current.processor || current.model.is_some() || current.vendor.is_some() {
                blocks.push(std::mem::take(&mut current));
            }
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        match key {
            "processor" => current.processor = true,
            "vendor_id" => current.vendor = Some(value.to_string()),
            "model name" => current.model = Some(value.to_string()),
            "physical id" => current.physical_id = value.parse().ok(),
            "core id" => current.core_id = value.parse().ok(),
            "cpu cores" => current.cores_per_socket = value.parse().ok(),
            _ => {}
        }
    }
    if current.processor || current.model.is_some() || current.vendor.is_some() {
        blocks.push(current);
    }

    let logical_cpus = u32::try_from(blocks.iter().filter(|block| block.processor).count()).ok();
    let mut physical_ids = BTreeSet::new();
    let mut sockets = BTreeSet::new();
    let mut fallback_cores = None;

    for block in &blocks {
        if let (Some(physical_id), Some(core_id)) = (block.physical_id, block.core_id) {
            physical_ids.insert((physical_id, core_id));
            sockets.insert(physical_id);
        }
        if block.cores_per_socket.is_some() {
            fallback_cores = block.cores_per_socket;
        }
    }

    let physical_cores = if physical_ids.is_empty() {
        fallback_cores
    } else if sockets.len() <= 1 {
        u32::try_from(physical_ids.len()).ok()
    } else {
        fallback_cores.and_then(|cores| {
            u32::try_from(sockets.len())
                .ok()
                .and_then(|sockets| sockets.checked_mul(cores))
        })
    };

    CpuInfo {
        vendor: blocks.iter().find_map(|block| block.vendor.clone()),
        model: blocks.iter().find_map(|block| block.model.clone()),
        architecture: Some(std::env::consts::ARCH.to_string()),
        logical_cpus,
        physical_cores,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_cpuinfo;

    #[test]
    fn parses_model_counts_and_physical_cores() {
        let info = parse_cpuinfo(include_str!("../tests/fixtures/cpuinfo.txt"));
        assert_eq!(
            info.model.as_deref(),
            Some("Intel(R) Core(TM) i7-8750H CPU @ 2.20GHz")
        );
        assert_eq!(info.vendor.as_deref(), Some("GenuineIntel"));
        assert_eq!(info.logical_cpus, Some(4));
        assert_eq!(info.physical_cores, Some(2));
    }

    #[test]
    fn handles_incomplete_blocks_without_guessing() {
        let info = parse_cpuinfo("processor : 0\nmodel name : Test CPU\n");
        assert_eq!(info.logical_cpus, Some(1));
        assert_eq!(info.physical_cores, None);
        assert_eq!(info.vendor, None);
    }

    #[test]
    fn ignores_malformed_lines() {
        let info = parse_cpuinfo("not a field\nprocessor : 0\nmodel name : Test CPU\n");
        assert_eq!(info.logical_cpus, Some(1));
        assert_eq!(info.model.as_deref(), Some("Test CPU"));
    }
}
