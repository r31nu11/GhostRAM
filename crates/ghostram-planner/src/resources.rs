//! Observed host and device resources used by the planner.
//!
//! The planner never probes the machine itself. The CLI or another adapter
//! collects these observations and passes a snapshot to the calculation core.

use ghostram_core::diagnostic::{CategoryReport, MemoryInfo, NvidiaGpuInfo};
use ghostram_core::quantity::Bytes;
use serde::{Deserialize, Serialize};

use crate::evidence::Evidence;

/// Where an observation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    /// Linux `/proc/meminfo`.
    ProcMeminfo,
    /// `nvidia-smi` query output.
    NvidiaSmi,
}

impl ObservationSource {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProcMeminfo => "proc_meminfo",
            Self::NvidiaSmi => "nvidia_smi",
        }
    }
}

/// Host memory observations and their category state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostResources {
    /// Whether the memory probe was complete, partial, or unavailable.
    pub state: CategoryReport,
    /// Observation source.
    pub source: ObservationSource,
    /// Total usable RAM.
    pub total: Evidence<Bytes>,
    /// Estimated memory available for new work without swapping.
    pub available: Evidence<Bytes>,
}

/// One NVIDIA device observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceResources {
    /// `nvidia-smi` device index.
    pub index: u32,
    /// Device product name.
    pub name: String,
    /// Installed driver version, when reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_version: Option<String>,
    /// Compute capability, when reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_capability: Option<String>,
    /// Observation source.
    pub source: ObservationSource,
    /// Total VRAM.
    pub total: Evidence<Bytes>,
    /// VRAM reported as used.
    pub used: Evidence<Bytes>,
    /// VRAM treated as available for a new workload.
    ///
    /// This is derived from total minus used when both are reported. It is not
    /// a direct observation of free VRAM.
    pub free: Evidence<Bytes>,
}

impl DeviceResources {
    fn from_nvidia(gpu: &NvidiaGpuInfo) -> Self {
        let total = match gpu.memory_total_bytes {
            Some(value) => Evidence::observed(value),
            None => Evidence::unknown("nvidia-smi did not report total device memory"),
        };
        let used = match gpu.memory_used_bytes {
            Some(value) => Evidence::observed(value),
            None => Evidence::unknown("nvidia-smi did not report used device memory"),
        };
        let free = match (gpu.memory_total_bytes, gpu.memory_used_bytes) {
            (Some(total), Some(used)) => match total.as_u64().checked_sub(used.as_u64()) {
                Some(free) => Evidence::derived(
                    Bytes::new(free),
                    "nvidia-smi total memory minus used memory; nvidia-smi does not report free memory directly",
                ),
                None => Evidence::unknown(
                    "nvidia-smi reported used device memory greater than total device memory",
                ),
            },
            (None, _) => Evidence::unknown(
                "total device memory was not reported, so free device memory cannot be derived",
            ),
            (_, None) => Evidence::unknown(
                "used device memory was not reported, so free device memory cannot be derived",
            ),
        };

        Self {
            index: gpu.index,
            name: gpu.name.clone(),
            driver_version: gpu.driver_version.clone(),
            compute_capability: gpu.compute_capability.clone(),
            source: ObservationSource::NvidiaSmi,
            total,
            used,
            free,
        }
    }
}

/// Host and device observations for one plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    /// UTC RFC 3339 time at which probing started.
    pub observed_at: String,
    /// Host memory observations.
    pub host: HostResources,
    /// Whether NVIDIA device probing was complete, partial, or unavailable.
    pub devices_state: CategoryReport,
    /// NVIDIA devices observed, sorted by device index.
    pub devices: Vec<DeviceResources>,
}

impl ResourceSnapshot {
    /// Builds a planner snapshot from existing probe observations.
    ///
    /// Missing memory fields remain unknown. `MemFree` is never substituted for
    /// `MemAvailable`, and free device memory is explicitly derived from total
    /// minus used.
    #[must_use]
    pub fn from_observations(
        observed_at: impl Into<String>,
        host_state: CategoryReport,
        memory: Option<&MemoryInfo>,
        devices_state: CategoryReport,
        gpus: &[NvidiaGpuInfo],
    ) -> Self {
        let total = match memory.and_then(|memory| memory.total_bytes) {
            Some(value) => Evidence::observed(value),
            None => Evidence::unknown("/proc/meminfo did not report MemTotal"),
        };
        let available = match memory.and_then(|memory| memory.available_bytes) {
            Some(value) => Evidence::observed(value),
            None => Evidence::unknown(
                "MemAvailable was not reported; MemFree and MemTotal are not used as available-memory substitutes",
            ),
        };

        let mut devices = gpus
            .iter()
            .map(DeviceResources::from_nvidia)
            .collect::<Vec<_>>();
        devices.sort_by_key(|device| device.index);

        Self {
            observed_at: observed_at.into(),
            host: HostResources {
                state: host_state,
                source: ObservationSource::ProcMeminfo,
                total,
                available,
            },
            devices_state,
            devices,
        }
    }
}

#[cfg(test)]
mod tests {
    use ghostram_core::diagnostic::{CategoryReport, MemoryInfo, NvidiaGpuInfo};
    use ghostram_core::quantity::Bytes;

    use super::{Evidence, ObservationSource, ResourceSnapshot};

    fn memory(total: u64, available: Option<u64>, free: Option<u64>) -> MemoryInfo {
        MemoryInfo {
            total_bytes: Some(Bytes::from_kib(total).unwrap()),
            available_bytes: available.map(|value| Bytes::from_kib(value).unwrap()),
            free_bytes: free.map(|value| Bytes::from_kib(value).unwrap()),
        }
    }

    fn gpu(total: Option<u64>, used: Option<u64>) -> NvidiaGpuInfo {
        NvidiaGpuInfo {
            index: 0,
            name: "NVIDIA Test".to_string(),
            driver_version: Some("1.0".to_string()),
            memory_total_bytes: total.map(|value| Bytes::from_mib(value).unwrap()),
            memory_used_bytes: used.map(|value| Bytes::from_mib(value).unwrap()),
            compute_capability: Some("6.1".to_string()),
        }
    }

    #[test]
    fn maps_observed_host_memory_without_using_free_as_available() {
        let snapshot = ResourceSnapshot::from_observations(
            "2026-10-21T00:00:00Z",
            CategoryReport::available(),
            Some(&memory(8_000_000, Some(6_000_000), Some(1_000_000))),
            CategoryReport::unavailable("nvidia-smi was not found"),
            &[],
        );
        assert_eq!(snapshot.host.source, ObservationSource::ProcMeminfo);
        assert_eq!(
            snapshot.host.total.value(),
            Some(&Bytes::from_kib(8_000_000).unwrap())
        );
        assert_eq!(
            snapshot.host.available.value(),
            Some(&Bytes::from_kib(6_000_000).unwrap())
        );
        assert!(snapshot.devices.is_empty());
    }

    #[test]
    fn missing_memavailable_remains_unknown() {
        let snapshot = ResourceSnapshot::from_observations(
            "2026-10-21T00:00:00Z",
            CategoryReport::partial("MemAvailable is missing"),
            Some(&memory(8_000_000, None, Some(1_000_000))),
            CategoryReport::unavailable("nvidia-smi was not found"),
            &[],
        );
        assert!(snapshot.host.available.value().is_none());
        assert!(
            snapshot
                .host
                .available
                .reason()
                .unwrap()
                .contains("MemAvailable")
        );
    }

    #[test]
    fn derives_free_device_memory_and_preserves_context() {
        let snapshot = ResourceSnapshot::from_observations(
            "2026-10-21T00:00:00Z",
            CategoryReport::available(),
            Some(&memory(8_000_000, Some(6_000_000), None)),
            CategoryReport::available(),
            &[gpu(Some(4096), Some(1024))],
        );
        let device = &snapshot.devices[0];
        assert_eq!(device.source, ObservationSource::NvidiaSmi);
        assert_eq!(device.total.value(), Some(&Bytes::from_mib(4096).unwrap()));
        assert_eq!(device.used.value(), Some(&Bytes::from_mib(1024).unwrap()));
        assert_eq!(device.free.value(), Some(&Bytes::from_mib(3072).unwrap()));
        assert!(
            device
                .free
                .reason()
                .unwrap()
                .contains("total memory minus used memory")
        );
    }

    #[test]
    fn inconsistent_or_missing_device_fields_remain_unknown() {
        let snapshot = ResourceSnapshot::from_observations(
            "2026-10-21T00:00:00Z",
            CategoryReport::available(),
            Some(&memory(8_000_000, Some(6_000_000), None)),
            CategoryReport::available(),
            &[gpu(Some(4096), Some(5000)), gpu(None, Some(1))],
        );
        assert!(snapshot.devices[0].free.value().is_none());
        assert!(snapshot.devices[1].free.value().is_none());
    }

    #[test]
    fn sorts_devices_by_index() {
        let mut second = gpu(Some(8192), Some(0));
        second.index = 2;
        let mut first = gpu(Some(4096), Some(0));
        first.index = 1;
        let snapshot = ResourceSnapshot::from_observations(
            "2026-10-21T00:00:00Z",
            CategoryReport::available(),
            Some(&memory(8_000_000, Some(6_000_000), None)),
            CategoryReport::available(),
            &[second, first],
        );
        assert_eq!(
            snapshot.devices.iter().map(|d| d.index).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn snapshot_round_trips_through_json() {
        let snapshot = ResourceSnapshot::from_observations(
            "2026-10-21T00:00:00Z",
            CategoryReport::available(),
            Some(&memory(8_000_000, Some(6_000_000), None)),
            CategoryReport::available(),
            &[gpu(Some(4096), Some(1024))],
        );
        let encoded = serde_json::to_string(&snapshot).unwrap();
        let decoded: ResourceSnapshot = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, snapshot);
        assert_eq!(
            serde_json::to_value(Evidence::observed(Bytes::new(1))).unwrap()["evidence"],
            "observed"
        );
    }
}
