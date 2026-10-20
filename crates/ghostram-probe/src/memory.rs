//! Memory and swap discovery from `/proc`.

use std::fs;

use ghostram_core::diagnostic::{CategoryReport, MemoryInfo, SwapDevice, SwapInfo};
use ghostram_core::quantity::Bytes;

const MEMINFO_PATH: &str = "/proc/meminfo";
const SWAPS_PATH: &str = "/proc/swaps";

#[derive(Default)]
struct MeminfoFields {
    total: Option<Bytes>,
    available: Option<Bytes>,
    free: Option<Bytes>,
    swap_total: Option<Bytes>,
    swap_free: Option<Bytes>,
}

pub(crate) fn collect_memory_and_swap() -> (
    Option<MemoryInfo>,
    CategoryReport,
    Option<SwapInfo>,
    CategoryReport,
) {
    let Ok(text) = fs::read_to_string(MEMINFO_PATH) else {
        let reason = format!("{MEMINFO_PATH} could not be read");
        return (
            None,
            CategoryReport::unavailable(reason.clone()),
            None,
            CategoryReport::unavailable(reason),
        );
    };

    let fields = parse_meminfo(&text);
    let memory = MemoryInfo {
        total_bytes: fields.total,
        available_bytes: fields.available,
        free_bytes: fields.free,
    };
    let memory_has_data = memory.total_bytes.is_some()
        || memory.available_bytes.is_some()
        || memory.free_bytes.is_some();
    let memory_category = if memory.total_bytes.is_some()
        && memory.available_bytes.is_some()
        && memory.free_bytes.is_some()
    {
        CategoryReport::available()
    } else if memory_has_data {
        CategoryReport::partial("some /proc/meminfo fields are missing")
    } else {
        CategoryReport::unavailable(format!(
            "{MEMINFO_PATH} contained no recognized memory fields"
        ))
    };

    let (devices, device_error) = match fs::read_to_string(SWAPS_PATH) {
        Ok(text) => match parse_swap_devices(&text) {
            Ok(devices) => (devices, None),
            Err(error) => (Vec::new(), Some(error)),
        },
        Err(error) => (Vec::new(), Some(format!("{SWAPS_PATH}: {error}"))),
    };

    let used = match (fields.swap_total, fields.swap_free) {
        (Some(total), Some(free)) if total.as_u64() >= free.as_u64() => {
            Some(Bytes::new(total.as_u64() - free.as_u64()))
        }
        _ => None,
    };
    let swap = SwapInfo {
        total_bytes: fields.swap_total,
        used_bytes: used,
        free_bytes: fields.swap_free,
        devices,
    };
    let swap_category = if fields.swap_total.is_none() {
        CategoryReport::unavailable("SwapTotal is missing from /proc/meminfo")
    } else if fields.swap_free.is_none() {
        CategoryReport::partial("SwapFree is missing from /proc/meminfo")
    } else if let Some(error) = device_error {
        CategoryReport::partial(format!("swap totals are available, but {error}"))
    } else {
        CategoryReport::available()
    };

    let memory = if memory_has_data { Some(memory) } else { None };
    let swap = if fields.swap_total.is_some() {
        Some(swap)
    } else {
        None
    };
    (memory, memory_category, swap, swap_category)
}

fn parse_meminfo(text: &str) -> MeminfoFields {
    let mut fields = MeminfoFields::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let bytes = parse_kib_value(value);
        match key.trim() {
            "MemTotal" => fields.total = bytes,
            "MemAvailable" => fields.available = bytes,
            "MemFree" => fields.free = bytes,
            "SwapTotal" => fields.swap_total = bytes,
            "SwapFree" => fields.swap_free = bytes,
            _ => {}
        }
    }
    fields
}

fn parse_kib_value(value: &str) -> Option<Bytes> {
    let number = value.split_whitespace().next()?;
    number.parse::<u64>().ok().and_then(Bytes::from_kib)
}

fn parse_swap_devices(text: &str) -> Result<Vec<SwapDevice>, String> {
    let mut devices = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 {
            return Err(format!(
                "{SWAPS_PATH} line {} has fewer than four fields",
                index + 1
            ));
        }
        let size_kib = fields[2]
            .parse::<u64>()
            .map_err(|_| format!("{SWAPS_PATH} line {} has an invalid size", index + 1))?;
        let used_kib = fields[3]
            .parse::<u64>()
            .map_err(|_| format!("{SWAPS_PATH} line {} has an invalid used value", index + 1))?;
        let size = Bytes::from_kib(size_kib)
            .ok_or_else(|| format!("{SWAPS_PATH} line {} size overflows", index + 1))?;
        let used = Bytes::from_kib(used_kib)
            .ok_or_else(|| format!("{SWAPS_PATH} line {} used value overflows", index + 1))?;
        devices.push(SwapDevice {
            name: fields[0].to_string(),
            kind: fields[1].to_string(),
            size_bytes: size,
            used_bytes: used,
            priority: fields.get(4).and_then(|value| value.parse().ok()),
        });
    }
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::{parse_meminfo, parse_swap_devices};
    use ghostram_core::quantity::Bytes;

    #[test]
    fn parses_meminfo_byte_counts() {
        let fields = parse_meminfo(include_str!("../tests/fixtures/meminfo.txt"));
        assert_eq!(fields.total, Bytes::from_kib(7_802_212));
        assert_eq!(fields.available, Bytes::from_kib(3_286_452));
        assert_eq!(fields.free, Bytes::from_kib(584_656));
        assert_eq!(fields.swap_total, Bytes::from_kib(7_801_852));
        assert_eq!(fields.swap_free, Bytes::from_kib(7_791_140));
    }

    #[test]
    fn preserves_zero_swap_as_a_measurement() {
        let fields = parse_meminfo("MemTotal: 1024 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n");
        assert_eq!(fields.swap_total, Some(Bytes::ZERO));
        assert_eq!(fields.swap_free, Some(Bytes::ZERO));
    }

    #[test]
    fn ignores_missing_and_malformed_fields() {
        let fields = parse_meminfo("MemTotal: nope kB\nMemAvailable: 2048 kB\n");
        assert_eq!(fields.total, None);
        assert_eq!(fields.available, Bytes::from_kib(2048));
    }

    #[test]
    fn parses_swap_devices() {
        let devices = parse_swap_devices(include_str!("../tests/fixtures/proc-swaps.txt")).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "/dev/zram0");
        assert_eq!(devices[0].kind, "partition");
        assert_eq!(devices[0].size_bytes, Bytes::from_kib(7_801_852).unwrap());
        assert_eq!(devices[0].used_bytes, Bytes::from_kib(10_712).unwrap());
        assert_eq!(devices[0].priority, Some(100));
    }

    #[test]
    fn rejects_a_truncated_swap_line() {
        let error =
            parse_swap_devices("Filename Type Size Used\n/dev/zram0 partition\n").unwrap_err();
        assert!(error.contains("fewer than four fields"));
    }
}
