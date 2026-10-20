//! NVIDIA GPU discovery through `nvidia-smi`.

use std::time::Duration;

use ghostram_core::diagnostic::{CategoryReport, NvidiaGpuInfo};
use ghostram_core::quantity::Bytes;

use crate::command::run_bounded;
use crate::tools::find_executable;

const NVIDIA_SMI_TIMEOUT: Duration = Duration::from_secs(3);
const GPU_QUERY: &str =
    "--query-gpu=index,name,driver_version,memory.total,memory.used,compute_cap";

pub(crate) fn collect_nvidia() -> (Vec<NvidiaGpuInfo>, CategoryReport) {
    let Some(nvidia_smi) = find_executable("nvidia-smi") else {
        return (
            Vec::new(),
            CategoryReport::unavailable("nvidia-smi was not found on PATH"),
        );
    };

    let observation = match run_bounded(
        &nvidia_smi,
        &[GPU_QUERY, "--format=csv,noheader,nounits"],
        NVIDIA_SMI_TIMEOUT,
    ) {
        Ok(observation) => observation,
        Err(error) => {
            return (
                Vec::new(),
                CategoryReport::unavailable(format!("nvidia-smi could not be started: {error}")),
            );
        }
    };

    if observation.timed_out {
        return (
            Vec::new(),
            CategoryReport::unavailable("nvidia-smi exceeded the 3 second timeout"),
        );
    }
    if !observation.succeeded() {
        let detail = observation
            .first_output_line()
            .unwrap_or_else(|| "no output".to_string());
        return (
            Vec::new(),
            CategoryReport::unavailable(format!("nvidia-smi failed: {detail}")),
        );
    }

    match parse_nvidia_smi(&observation.stdout) {
        Ok(gpus) if gpus.is_empty() => (
            Vec::new(),
            CategoryReport::available_with_reason("nvidia-smi reported no GPUs"),
        ),
        Ok(gpus) => (gpus, CategoryReport::available()),
        Err(error) => (
            Vec::new(),
            CategoryReport::unavailable(format!("nvidia-smi output could not be parsed: {error}")),
        ),
    }
}

fn parse_nvidia_smi(text: &str) -> Result<Vec<NvidiaGpuInfo>, String> {
    let mut gpus = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < 6 {
            return Err(format!(
                "line {} has {} fields instead of at least 6",
                index + 1,
                fields.len()
            ));
        }
        let gpu_index = fields[0]
            .parse::<u32>()
            .map_err(|_| format!("line {} has an invalid GPU index", index + 1))?;
        let tail = &fields[fields.len() - 4..];
        let name = fields[1..fields.len() - 4].join(", ");
        if name.is_empty() {
            return Err(format!("line {} has an empty GPU name", index + 1));
        }
        gpus.push(NvidiaGpuInfo {
            index: gpu_index,
            name,
            driver_version: optional_text(tail[0]),
            memory_total_bytes: parse_mib(tail[1])
                .map_err(|error| format!("line {} total memory: {error}", index + 1))?,
            memory_used_bytes: parse_mib(tail[2])
                .map_err(|error| format!("line {} used memory: {error}", index + 1))?,
            compute_capability: optional_text(tail[3]),
        });
    }
    Ok(gpus)
}

fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value != "N/A" && value != "[N/A]").then(|| value.to_string())
}

fn parse_mib(value: &str) -> Result<Option<Bytes>, String> {
    let Some(value) = optional_text(value) else {
        return Ok(None);
    };
    let mib = value
        .parse::<u64>()
        .map_err(|_| format!("{value:?} is not an integer MiB value"))?;
    Bytes::from_mib(mib)
        .map(Some)
        .ok_or_else(|| format!("{value:?} MiB overflows a byte count"))
}

#[cfg(test)]
mod tests {
    use super::parse_nvidia_smi;
    use ghostram_core::quantity::Bytes;

    #[test]
    fn parses_nvidia_smi_csv() {
        let gpus = parse_nvidia_smi(include_str!("../tests/fixtures/nvidia-smi.csv")).unwrap();
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].index, 0);
        assert_eq!(gpus[0].name, "NVIDIA GeForce GTX 1050 Ti");
        assert_eq!(gpus[0].driver_version.as_deref(), Some("580.178.04"));
        assert_eq!(gpus[0].memory_total_bytes, Bytes::from_mib(4096));
        assert_eq!(gpus[0].memory_used_bytes, Bytes::from_mib(3));
        assert_eq!(gpus[0].compute_capability.as_deref(), Some("6.1"));
    }

    #[test]
    fn preserves_unavailable_fields() {
        let gpus = parse_nvidia_smi("0, Test GPU, [N/A], [N/A], 12, [N/A]\n").unwrap();
        assert_eq!(gpus[0].driver_version, None);
        assert_eq!(gpus[0].memory_total_bytes, None);
        assert_eq!(gpus[0].memory_used_bytes, Bytes::from_mib(12));
        assert_eq!(gpus[0].compute_capability, None);
    }

    #[test]
    fn parses_multiple_gpus_without_reordering() {
        let gpus =
            parse_nvidia_smi("1, NVIDIA B, 1.0, 2048, 10, 8.6\n0, NVIDIA A, 1.0, 4096, 20, 6.1\n")
                .unwrap();
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].index, 1);
        assert_eq!(gpus[1].index, 0);
    }

    #[test]
    fn rejects_a_truncated_row() {
        let error = parse_nvidia_smi("0, NVIDIA GPU\n").unwrap_err();
        assert!(error.contains("at least 6"));
    }

    #[test]
    fn rejects_a_non_numeric_memory_value() {
        let error = parse_nvidia_smi("0, GPU, 1.0, unknown, 1, 6.1\n").unwrap_err();
        assert!(error.contains("integer MiB"));
    }
}
