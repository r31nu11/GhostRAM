//! Human-readable rendering for `ghostram doctor`.

use std::fmt::Write;

use ghostram_core::diagnostic::{
    DiagnosticEnvelope, DiagnosticState, MemoryInfo, StorageInfo, SystemInfo, ToolInfo,
};
use ghostram_core::quantity::Bytes;

/// Renders a complete human-readable diagnostic report.
#[must_use]
pub fn render_doctor(envelope: &DiagnosticEnvelope) -> String {
    let mut output = String::new();
    let report = &envelope.report;

    let _ = writeln!(output, "GhostRAM doctor");
    let _ = writeln!(output, "Application: {}", envelope.application_version);
    let _ = writeln!(output, "Schema: {}", envelope.schema_version);
    let _ = writeln!(output, "Generated: {}", envelope.generated_at);
    let _ = writeln!(output, "Overall: {}", state_name(report.overall_state));
    output.push('\n');

    let _ = writeln!(output, "Categories");
    let categories = &report.categories;
    for (name, category) in [
        ("os", &categories.os),
        ("cpu", &categories.cpu),
        ("memory", &categories.memory),
        ("swap", &categories.swap),
        ("zram", &categories.zram),
        ("storage", &categories.storage),
        ("nvidia_gpu", &categories.nvidia_gpu),
        ("vulkan", &categories.vulkan),
        ("tools", &categories.tools),
        ("engines", &categories.engines),
    ] {
        let _ = write!(output, "  {name}: {}", state_name(category.state));
        if let Some(reason) = &category.reason {
            let _ = write!(output, " ({reason})");
        }
        output.push('\n');
    }
    output.push('\n');

    let _ = writeln!(output, "System");
    write_os(&mut output, &report.system);
    write_cpu(&mut output, &report.system);
    write_memory(&mut output, &report.system);
    write_swap(&mut output, &report.system);
    write_zram(&mut output, &report.system);
    write_storage(&mut output, &report.system);
    write_nvidia(&mut output, &report.system);
    write_vulkan(&mut output, &report.system);
    write_tools(&mut output, &report.system);
    write_engines(&mut output, &report.system);

    output
}

fn state_name(state: DiagnosticState) -> &'static str {
    match state {
        DiagnosticState::Available => "available",
        DiagnosticState::Partial => "partial",
        DiagnosticState::Unavailable => "unavailable",
    }
}

fn display_bytes(value: Option<Bytes>) -> String {
    value.map_or_else(|| "unknown".to_string(), |value| value.to_string())
}

fn write_os(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  OS:");
    let Some(os) = &system.os else {
        let _ = writeln!(output, "    unavailable");
        return;
    };
    let _ = writeln!(
        output,
        "    name: {}",
        os.name.as_deref().unwrap_or("unknown")
    );
    let _ = writeln!(
        output,
        "    version: {}",
        os.version.as_deref().unwrap_or("unknown")
    );
    let _ = writeln!(output, "    id: {}", os.id.as_deref().unwrap_or("unknown"));
    let _ = writeln!(
        output,
        "    kernel: {}",
        os.kernel.as_deref().unwrap_or("unknown")
    );
    let _ = writeln!(
        output,
        "    architecture: {}",
        os.architecture.as_deref().unwrap_or("unknown")
    );
}

fn write_cpu(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  CPU:");
    let Some(cpu) = &system.cpu else {
        let _ = writeln!(output, "    unavailable");
        return;
    };
    let _ = writeln!(
        output,
        "    model: {}",
        cpu.model.as_deref().unwrap_or("unknown")
    );
    let _ = writeln!(
        output,
        "    vendor: {}",
        cpu.vendor.as_deref().unwrap_or("unknown")
    );
    let _ = writeln!(
        output,
        "    logical CPUs: {}",
        cpu.logical_cpus
            .map_or_else(|| "unknown".to_string(), |value| value.to_string())
    );
    let _ = writeln!(
        output,
        "    physical cores: {}",
        cpu.physical_cores
            .map_or_else(|| "unknown".to_string(), |value| value.to_string())
    );
}

fn write_memory(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  Memory:");
    let Some(memory) = &system.memory else {
        let _ = writeln!(output, "    unavailable");
        return;
    };
    write_memory_info(output, memory);
}

fn write_memory_info(output: &mut String, memory: &MemoryInfo) {
    let _ = writeln!(output, "    total: {}", display_bytes(memory.total_bytes));
    let _ = writeln!(
        output,
        "    available: {}",
        display_bytes(memory.available_bytes)
    );
    let _ = writeln!(output, "    free: {}", display_bytes(memory.free_bytes));
}

fn write_swap(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  Swap:");
    let Some(swap) = &system.swap else {
        let _ = writeln!(output, "    unavailable");
        return;
    };
    let _ = writeln!(output, "    total: {}", display_bytes(swap.total_bytes));
    let _ = writeln!(output, "    used: {}", display_bytes(swap.used_bytes));
    let _ = writeln!(output, "    free: {}", display_bytes(swap.free_bytes));
    if swap.devices.is_empty() {
        let _ = writeln!(output, "    devices: none");
    } else {
        for device in &swap.devices {
            let _ = writeln!(
                output,
                "    device: {} ({}, {}, used {})",
                device.name, device.kind, device.size_bytes, device.used_bytes
            );
        }
    }
}

fn write_zram(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  zram:");
    let Some(zram) = &system.zram else {
        let _ = writeln!(output, "    unavailable");
        return;
    };
    if zram.devices.is_empty() {
        let _ = writeln!(output, "    none detected");
    } else {
        for device in &zram.devices {
            let algorithm = device.algorithm.as_deref().unwrap_or("unknown");
            let _ = writeln!(
                output,
                "    {}: {} ({algorithm})",
                device.name, device.disksize_bytes
            );
        }
    }
}

fn write_storage(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  Storage:");
    if system.storage.is_empty() {
        let _ = writeln!(output, "    unavailable");
        return;
    }
    for storage in &system.storage {
        write_storage_info(output, storage);
    }
}

fn write_storage_info(output: &mut String, storage: &StorageInfo) {
    let _ = writeln!(
        output,
        "    {} ({}): {} available of {} ({} used)",
        storage.mount_point,
        storage.filesystem,
        storage.available_bytes,
        storage.total_bytes,
        storage.used_bytes
    );
}

fn write_nvidia(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  NVIDIA GPUs:");
    if system.nvidia_gpus.is_empty() {
        let _ = writeln!(output, "    none detected");
        return;
    }
    for gpu in &system.nvidia_gpus {
        let _ = writeln!(
            output,
            "    [{}] {}: VRAM {} total, {} used; driver {}; compute {}",
            gpu.index,
            gpu.name,
            display_bytes(gpu.memory_total_bytes),
            display_bytes(gpu.memory_used_bytes),
            gpu.driver_version.as_deref().unwrap_or("unknown"),
            gpu.compute_capability.as_deref().unwrap_or("unknown")
        );
    }
}

fn write_vulkan(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  Vulkan:");
    let Some(vulkan) = &system.vulkan else {
        let _ = writeln!(output, "    unavailable");
        return;
    };
    let _ = writeln!(
        output,
        "    instance: {}",
        vulkan.instance_version.as_deref().unwrap_or("unknown")
    );
    if vulkan.devices.is_empty() {
        let _ = writeln!(output, "    devices: none");
    } else {
        for device in &vulkan.devices {
            let _ = writeln!(
                output,
                "    [{}] {} ({:?}, vendor {}, driver {})",
                device.index,
                device.name,
                device.device_type,
                device.vendor_id.as_deref().unwrap_or("unknown"),
                device.driver_info.as_deref().unwrap_or("unknown")
            );
        }
    }
}

fn write_tools(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  Tools:");
    for tool in &system.tools {
        write_tool(output, tool);
    }
}

fn write_tool(output: &mut String, tool: &ToolInfo) {
    match &tool.path {
        Some(path) => {
            let version = tool.version.as_deref().unwrap_or("version unknown");
            let _ = writeln!(output, "    {}: {path} ({version})", tool.name);
        }
        None => {
            let _ = writeln!(output, "    {}: not found", tool.name);
        }
    }
}

fn write_engines(output: &mut String, system: &SystemInfo) {
    let _ = writeln!(output, "  Engines:");
    if system.engines.is_empty() {
        let _ = writeln!(output, "    no llama.cpp executables found on PATH");
        return;
    }
    for engine in &system.engines {
        let _ = writeln!(
            output,
            "    {}: {} (llama.cpp); version {}; state {}",
            engine.executable,
            engine.path,
            engine.version.as_deref().unwrap_or("unknown"),
            state_name(engine.state)
        );
        if let Some(reason) = &engine.reason {
            let _ = writeln!(output, "      note: {reason}");
        }
        if let Some(devices) = &engine.devices {
            if devices.is_empty() {
                let _ = writeln!(output, "      devices: none reported");
            } else {
                for device in devices {
                    let _ = writeln!(output, "      device: {device}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::render_doctor;
    use ghostram_core::diagnostic::{
        CategoryReport, CategoryStates, DiagnosticEnvelope, DiagnosticReport, DiagnosticState,
        SystemInfo,
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

    #[test]
    fn renders_schema_version_and_engine_absence() {
        let report = DiagnosticReport {
            overall_state: DiagnosticState::Available,
            categories: categories(DiagnosticState::Available),
            system: SystemInfo::default(),
        };
        let envelope = DiagnosticEnvelope::new("0.1.0-test", "2026-10-20T00:00:00Z", report);
        let rendered = render_doctor(&envelope);
        assert!(rendered.contains("GhostRAM doctor"));
        assert!(rendered.contains("Schema: ghostram.doctor/1"));
        assert!(rendered.contains("no llama.cpp executables found on PATH"));
    }

    #[test]
    fn renders_unavailable_reasons() {
        let report = DiagnosticReport {
            overall_state: DiagnosticState::Partial,
            categories: categories(DiagnosticState::Partial),
            system: SystemInfo::default(),
        };
        let envelope = DiagnosticEnvelope::new("0.1.0-test", "2026-10-20T00:00:00Z", report);
        let rendered = render_doctor(&envelope);
        assert!(rendered.contains("Overall: partial"));
        assert!(rendered.contains("unavailable"));
    }
}
