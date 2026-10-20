//! Engine-independent diagnostic models and the versioned doctor envelope.
//!
//! The JSON field names in this module are part of the diagnostic schema. See
//! `docs/DOCTOR_SCHEMA.md` before renaming a public field.

use serde::{Deserialize, Serialize};

use crate::error::GhostramError;
use crate::quantity::Bytes;

/// Current diagnostic schema identifier.
pub const DOCTOR_SCHEMA_VERSION: &str = "ghostram.doctor/1";

/// Whether a diagnostic category is usable, incomplete, or unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticState {
    /// Data was collected successfully.
    Available,
    /// Some data was collected, but at least one part is missing or inconclusive.
    Partial,
    /// No usable data was collected.
    Unavailable,
}

/// State and optional explanation for one diagnostic category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryReport {
    /// Category state.
    pub state: DiagnosticState,
    /// Why the category is partial or unavailable, or another useful note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CategoryReport {
    /// A fully available category.
    #[must_use]
    pub const fn available() -> Self {
        Self {
            state: DiagnosticState::Available,
            reason: None,
        }
    }

    /// A fully available category with an explanatory note.
    #[must_use]
    pub fn available_with_reason(reason: impl Into<String>) -> Self {
        Self {
            state: DiagnosticState::Available,
            reason: Some(reason.into()),
        }
    }

    /// A category with some missing or inconclusive data.
    #[must_use]
    pub fn partial(reason: impl Into<String>) -> Self {
        Self {
            state: DiagnosticState::Partial,
            reason: Some(reason.into()),
        }
    }

    /// A category with no usable data.
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            state: DiagnosticState::Unavailable,
            reason: Some(reason.into()),
        }
    }
}

/// State of each diagnostic category in a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryStates {
    /// Operating-system and kernel information.
    pub os: CategoryReport,
    /// CPU model, architecture, and counts.
    pub cpu: CategoryReport,
    /// System memory information.
    pub memory: CategoryReport,
    /// Swap totals, usage, and devices.
    pub swap: CategoryReport,
    /// zram device detection.
    pub zram: CategoryReport,
    /// Filesystem capacity and available space.
    pub storage: CategoryReport,
    /// NVIDIA GPU and driver information.
    pub nvidia_gpu: CategoryReport,
    /// Vulkan instance and adapter information.
    pub vulkan: CategoryReport,
    /// Development and GPU tool discovery.
    pub tools: CategoryReport,
    /// External llama.cpp executable discovery.
    pub engines: CategoryReport,
}

impl CategoryStates {
    /// Combines category states into one report-level state.
    #[must_use]
    pub fn overall_state(&self) -> DiagnosticState {
        let states = [
            &self.os,
            &self.cpu,
            &self.memory,
            &self.swap,
            &self.zram,
            &self.storage,
            &self.nvidia_gpu,
            &self.vulkan,
            &self.tools,
            &self.engines,
        ];

        if states
            .iter()
            .all(|category| category.state == DiagnosticState::Available)
        {
            DiagnosticState::Available
        } else if states
            .iter()
            .all(|category| category.state == DiagnosticState::Unavailable)
        {
            DiagnosticState::Unavailable
        } else {
            DiagnosticState::Partial
        }
    }
}

/// Operating-system information read from OS metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsInfo {
    /// Distribution name, for example `CachyOS Linux`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Distribution version, when declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Distribution identifier, for example `cachyos`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Kernel release, for example `7.2.11-1-cachyos`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel: Option<String>,
    /// Target architecture, for example `x86_64`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
}

/// CPU information read from `/proc/cpuinfo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU vendor string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    /// CPU model string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Target architecture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    /// Number of logical CPUs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_cpus: Option<u32>,
    /// Number of distinct physical cores, when detectable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_cores: Option<u32>,
}

/// RAM information from `/proc/meminfo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total usable RAM.
    pub total_bytes: Option<Bytes>,
    /// Estimated memory available for new work without swapping.
    pub available_bytes: Option<Bytes>,
    /// Unused RAM.
    pub free_bytes: Option<Bytes>,
}

/// One swap device from `/proc/swaps`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwapDevice {
    /// Device or file name.
    pub name: String,
    /// Kernel-reported type, for example `partition` or `file`.
    pub kind: String,
    /// Device size.
    pub size_bytes: Bytes,
    /// Bytes currently in use.
    pub used_bytes: Bytes,
    /// Swap priority, when reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// Swap totals and devices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwapInfo {
    /// Total swap size.
    pub total_bytes: Option<Bytes>,
    /// Used swap.
    pub used_bytes: Option<Bytes>,
    /// Free swap.
    pub free_bytes: Option<Bytes>,
    /// Devices backing swap.
    pub devices: Vec<SwapDevice>,
}

/// One zram block device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZramDevice {
    /// Device name, for example `zram0`.
    pub name: String,
    /// Configured uncompressed size.
    pub disksize_bytes: Bytes,
    /// Selected compression algorithm, when exposed by sysfs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
}

/// zram device information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZramInfo {
    /// Detected zram block devices. An empty list means no zram device was found.
    pub devices: Vec<ZramDevice>,
}

/// One mounted filesystem reported by `df`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Filesystem device or pseudo-device.
    pub filesystem: String,
    /// Mount point.
    pub mount_point: String,
    /// Filesystem size.
    pub total_bytes: Bytes,
    /// Bytes currently used.
    pub used_bytes: Bytes,
    /// Bytes available to an unprivileged user.
    pub available_bytes: Bytes,
}

/// NVIDIA GPU information from `nvidia-smi`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvidiaGpuInfo {
    /// `nvidia-smi` GPU index.
    pub index: u32,
    /// GPU product name.
    pub name: String,
    /// Installed NVIDIA driver version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_version: Option<String>,
    /// Total VRAM.
    pub memory_total_bytes: Option<Bytes>,
    /// VRAM currently reported as used.
    pub memory_used_bytes: Option<Bytes>,
    /// Compute capability, for example `6.1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_capability: Option<String>,
}

/// Normalized Vulkan physical-device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VulkanDeviceType {
    /// Integrated GPU.
    IntegratedGpu,
    /// Discrete GPU.
    DiscreteGpu,
    /// Virtual GPU.
    VirtualGpu,
    /// CPU implementation.
    Cpu,
    /// Any other or unrecognized type.
    Other,
}

/// One Vulkan physical device from `vulkaninfo --summary`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VulkanDevice {
    /// Zero-based device index from the summary output.
    pub index: u32,
    /// Device name.
    pub name: String,
    /// Normalized physical-device type.
    pub device_type: VulkanDeviceType,
    /// Vulkan API version reported by the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Driver version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_version: Option<String>,
    /// Driver name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_name: Option<String>,
    /// Driver information string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_info: Option<String>,
    /// PCI vendor ID, for example `0x10de`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_id: Option<String>,
    /// PCI device ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

/// Vulkan instance and device information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VulkanInfo {
    /// Vulkan loader/instance version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_version: Option<String>,
    /// Visible physical devices.
    pub devices: Vec<VulkanDevice>,
}

/// A discovered development, GPU, or inference tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Executable name looked up on `PATH`.
    pub name: String,
    /// Absolute or PATH-derived executable path; `null` means it was not found.
    pub path: Option<String>,
    /// Version text when a bounded query succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Engine family associated with discovered executables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineFamily {
    /// Official llama.cpp or a compatible build.
    LlamaCpp,
}

/// An externally installed engine executable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineDiscovery {
    /// Executable name, for example `llama-server`.
    pub executable: String,
    /// Engine family.
    pub family: EngineFamily,
    /// Discovered path.
    pub path: String,
    /// Version text when the bounded probe succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Device listing when the bounded probe succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devices: Option<Vec<String>>,
    /// Availability of the executable's read-only probes.
    pub state: DiagnosticState,
    /// Why probes were incomplete or inconclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Collected system information. Missing fields are `None`; they are never
/// replaced with guessed values.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system and kernel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<OsInfo>,
    /// CPU data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuInfo>,
    /// Memory data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryInfo>,
    /// Swap data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap: Option<SwapInfo>,
    /// zram data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zram: Option<ZramInfo>,
    /// Mounted filesystems inspected by the probe.
    pub storage: Vec<StorageInfo>,
    /// NVIDIA GPUs.
    pub nvidia_gpus: Vec<NvidiaGpuInfo>,
    /// Vulkan information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vulkan: Option<VulkanInfo>,
    /// Tool availability.
    pub tools: Vec<ToolInfo>,
    /// External engine executables.
    pub engines: Vec<EngineDiscovery>,
}

/// Diagnostic data for one machine at one time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// Report-level state derived from the category states.
    pub overall_state: DiagnosticState,
    /// State of each diagnostic category.
    pub categories: CategoryStates,
    /// Collected system data.
    pub system: SystemInfo,
}

/// Versioned envelope around a diagnostic report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEnvelope {
    /// Schema identifier for the report shape.
    pub schema_version: String,
    /// GhostRAM application version that generated the report.
    pub application_version: String,
    /// UTC RFC 3339 generation time.
    pub generated_at: String,
    /// Report payload.
    pub report: DiagnosticReport,
}

impl DiagnosticEnvelope {
    /// Creates an envelope with the current schema version.
    #[must_use]
    pub fn new(
        application_version: impl Into<String>,
        generated_at: impl Into<String>,
        report: DiagnosticReport,
    ) -> Self {
        Self {
            schema_version: DOCTOR_SCHEMA_VERSION.to_string(),
            application_version: application_version.into(),
            generated_at: generated_at.into(),
            report,
        }
    }

    /// Verifies that this envelope uses the schema version supported by this
    /// build.
    ///
    /// # Errors
    ///
    /// Returns [`GhostramError::InvalidInput`] when the schema identifier differs.
    pub fn validate_schema(&self) -> Result<(), GhostramError> {
        if self.schema_version == DOCTOR_SCHEMA_VERSION {
            Ok(())
        } else {
            Err(GhostramError::InvalidInput {
                message: format!(
                    "unsupported diagnostic schema {:?}; expected {:?}",
                    self.schema_version, DOCTOR_SCHEMA_VERSION
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CategoryReport, CategoryStates, DOCTOR_SCHEMA_VERSION, DiagnosticEnvelope,
        DiagnosticReport, DiagnosticState,
    };

    fn categories(state: DiagnosticState) -> CategoryStates {
        let report = || CategoryReport {
            state,
            reason: None,
        };
        CategoryStates {
            os: report(),
            cpu: report(),
            memory: report(),
            swap: report(),
            zram: report(),
            storage: report(),
            nvidia_gpu: report(),
            vulkan: report(),
            tools: report(),
            engines: report(),
        }
    }

    fn report(state: DiagnosticState) -> DiagnosticReport {
        DiagnosticReport {
            overall_state: state,
            categories: categories(state),
            system: super::SystemInfo::default(),
        }
    }

    #[test]
    fn category_constructors_preserve_reasons() {
        assert_eq!(
            CategoryReport::available().state,
            DiagnosticState::Available
        );
        assert_eq!(
            CategoryReport::available_with_reason("no optional hardware")
                .reason
                .as_deref(),
            Some("no optional hardware")
        );
        assert_eq!(
            CategoryReport::partial("one field missing")
                .reason
                .as_deref(),
            Some("one field missing")
        );
        assert_eq!(
            CategoryReport::unavailable("tool not found").state,
            DiagnosticState::Unavailable
        );
    }

    #[test]
    fn computes_overall_state() {
        assert_eq!(
            categories(DiagnosticState::Available).overall_state(),
            DiagnosticState::Available
        );
        assert_eq!(
            categories(DiagnosticState::Unavailable).overall_state(),
            DiagnosticState::Unavailable
        );

        let mut mixed = categories(DiagnosticState::Available);
        mixed.nvidia_gpu = CategoryReport::unavailable("nvidia-smi not found");
        assert_eq!(mixed.overall_state(), DiagnosticState::Partial);
    }

    #[test]
    fn envelope_round_trips_as_json() {
        let envelope = DiagnosticEnvelope::new(
            "0.1.0-dev",
            "2026-10-20T12:00:00Z",
            report(DiagnosticState::Available),
        );
        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: DiagnosticEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, envelope);
        assert_eq!(decoded.schema_version, DOCTOR_SCHEMA_VERSION);
        decoded.validate_schema().unwrap();
    }

    #[test]
    fn rejects_unknown_schema_versions() {
        let mut envelope = DiagnosticEnvelope::new(
            "0.1.0-dev",
            "2026-10-20T12:00:00Z",
            report(DiagnosticState::Available),
        );
        envelope.schema_version = "ghostram.doctor/999".to_string();
        let error = envelope.validate_schema().unwrap_err();
        assert_eq!(error.code(), crate::error::ErrorCode::InvalidInput);
    }

    #[test]
    fn uses_stable_state_names() {
        assert_eq!(
            serde_json::to_string(&DiagnosticState::Available).unwrap(),
            "\"available\""
        );
        assert_eq!(
            serde_json::to_string(&DiagnosticState::Partial).unwrap(),
            "\"partial\""
        );
        assert_eq!(
            serde_json::to_string(&DiagnosticState::Unavailable).unwrap(),
            "\"unavailable\""
        );
    }
}
