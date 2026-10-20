//! Vulkan summary parsing and discovery.

use std::time::Duration;

use ghostram_core::diagnostic::{CategoryReport, VulkanDevice, VulkanDeviceType, VulkanInfo};

use crate::command::run_bounded;
use crate::tools::find_executable;

const VULKANINFO_TIMEOUT: Duration = Duration::from_secs(8);

pub(crate) fn collect_vulkan() -> (Option<VulkanInfo>, CategoryReport) {
    let Some(vulkaninfo) = find_executable("vulkaninfo") else {
        return (
            None,
            CategoryReport::unavailable("vulkaninfo was not found on PATH"),
        );
    };

    let observation = match run_bounded(&vulkaninfo, &["--summary"], VULKANINFO_TIMEOUT) {
        Ok(observation) => observation,
        Err(error) => {
            return (
                None,
                CategoryReport::unavailable(format!("vulkaninfo could not be started: {error}")),
            );
        }
    };

    if observation.timed_out {
        return (
            None,
            CategoryReport::unavailable("vulkaninfo exceeded the 8 second timeout"),
        );
    }

    match parse_vulkan_summary(&observation.stdout) {
        Ok(info) if info.devices.is_empty() => (
            Some(info),
            CategoryReport::available_with_reason("vulkaninfo reported no physical devices"),
        ),
        Ok(info) if observation.succeeded() => (Some(info), CategoryReport::available()),
        Ok(info) => {
            let detail = observation
                .first_output_line()
                .unwrap_or_else(|| "no output".to_string());
            (
                Some(info),
                CategoryReport::partial(format!(
                    "vulkaninfo exited with status {:?}; summary was parsed from stdout: {detail}",
                    observation.exit_code
                )),
            )
        }
        Err(error) => {
            let detail = observation
                .first_output_line()
                .unwrap_or_else(|| "no output".to_string());
            (
                None,
                CategoryReport::unavailable(format!(
                    "vulkaninfo output could not be parsed: {error}; {detail}"
                )),
            )
        }
    }
}

fn parse_vulkan_summary(text: &str) -> Result<VulkanInfo, String> {
    let mut instance_version = None;
    let mut devices = Vec::new();
    let mut current: Option<VulkanDeviceBuilder> = None;
    let mut in_devices = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Vulkan Instance Version:") {
            instance_version = optional(value.trim());
            continue;
        }
        if trimmed == "Devices:" {
            in_devices = true;
            continue;
        }
        if !in_devices {
            continue;
        }

        if let Some(index) = parse_gpu_heading(trimmed) {
            if let Some(device) = current.take() {
                devices.push(device.finish()?);
            }
            current = Some(VulkanDeviceBuilder::new(index));
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if let Some(builder) = current.as_mut() {
            builder.set(key.trim(), value.trim());
        }
    }

    if let Some(device) = current.take() {
        devices.push(device.finish()?);
    }

    if instance_version.is_none() && devices.is_empty() {
        return Err("no Vulkan instance version or physical devices were found".to_string());
    }
    Ok(VulkanInfo {
        instance_version,
        devices,
    })
}

fn parse_gpu_heading(line: &str) -> Option<u32> {
    let rest = line.strip_prefix("GPU")?;
    let index = rest.strip_suffix(':')?;
    index.parse().ok()
}

fn optional(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

struct VulkanDeviceBuilder {
    index: u32,
    name: Option<String>,
    device_type: VulkanDeviceType,
    api_version: Option<String>,
    driver_version: Option<String>,
    driver_name: Option<String>,
    driver_info: Option<String>,
    vendor_id: Option<String>,
    device_id: Option<String>,
}

impl VulkanDeviceBuilder {
    fn new(index: u32) -> Self {
        Self {
            index,
            name: None,
            device_type: VulkanDeviceType::Other,
            api_version: None,
            driver_version: None,
            driver_name: None,
            driver_info: None,
            vendor_id: None,
            device_id: None,
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        match key {
            "deviceName" => self.name = optional(value),
            "deviceType" => self.device_type = parse_device_type(value),
            "apiVersion" => self.api_version = optional(value),
            "driverVersion" => self.driver_version = optional(value),
            "driverName" => self.driver_name = optional(value),
            "driverInfo" => self.driver_info = optional(value),
            "vendorID" => self.vendor_id = optional(value),
            "deviceID" => self.device_id = optional(value),
            _ => {}
        }
    }

    fn finish(self) -> Result<VulkanDevice, String> {
        let name = self
            .name
            .ok_or_else(|| format!("GPU{} has no deviceName", self.index))?;
        Ok(VulkanDevice {
            index: self.index,
            name,
            device_type: self.device_type,
            api_version: self.api_version,
            driver_version: self.driver_version,
            driver_name: self.driver_name,
            driver_info: self.driver_info,
            vendor_id: self.vendor_id,
            device_id: self.device_id,
        })
    }
}

fn parse_device_type(value: &str) -> VulkanDeviceType {
    match value {
        "PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU" => VulkanDeviceType::IntegratedGpu,
        "PHYSICAL_DEVICE_TYPE_DISCRETE_GPU" => VulkanDeviceType::DiscreteGpu,
        "PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU" => VulkanDeviceType::VirtualGpu,
        "PHYSICAL_DEVICE_TYPE_CPU" => VulkanDeviceType::Cpu,
        _ => VulkanDeviceType::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_vulkan_summary;
    use ghostram_core::diagnostic::VulkanDeviceType;

    #[test]
    fn parses_instance_and_multiple_devices() {
        let info =
            parse_vulkan_summary(include_str!("../tests/fixtures/vulkaninfo-summary.txt")).unwrap();
        assert_eq!(info.instance_version.as_deref(), Some("1.4.363"));
        assert_eq!(info.devices.len(), 2);
        assert_eq!(info.devices[0].name, "Intel(R) UHD Graphics 630 (CFL GT2)");
        assert_eq!(info.devices[0].device_type, VulkanDeviceType::IntegratedGpu);
        assert_eq!(info.devices[0].vendor_id.as_deref(), Some("0x8086"));
        assert_eq!(info.devices[1].name, "NVIDIA GeForce GTX 1050 Ti");
        assert_eq!(info.devices[1].device_type, VulkanDeviceType::DiscreteGpu);
        assert_eq!(info.devices[1].vendor_id.as_deref(), Some("0x10de"));
        assert_eq!(info.devices[1].driver_info.as_deref(), Some("580.178.04"));
    }

    #[test]
    fn maps_unknown_device_types_to_other() {
        let info = parse_vulkan_summary(
            "Vulkan Instance Version: 1.3.0\nDevices:\nGPU0:\n\tdeviceName = Unknown\n\tdeviceType = PHYSICAL_DEVICE_TYPE_UNKNOWN\n",
        )
        .unwrap();
        assert_eq!(info.devices[0].device_type, VulkanDeviceType::Other);
    }

    #[test]
    fn rejects_a_device_without_a_name() {
        let error =
            parse_vulkan_summary("Vulkan Instance Version: 1.3.0\nDevices:\nGPU0:\n\tdeviceType = PHYSICAL_DEVICE_TYPE_CPU\n")
                .unwrap_err();
        assert!(error.contains("no deviceName"));
    }

    #[test]
    fn rejects_empty_output() {
        assert!(parse_vulkan_summary("no useful output\n").is_err());
    }
}
