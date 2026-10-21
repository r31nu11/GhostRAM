//! Assembly of the read-only doctor report.

use ghostram_core::diagnostic::{
    CategoryReport, CategoryStates, DiagnosticEnvelope, DiagnosticReport, DiagnosticState,
    SystemInfo,
};
use ghostram_core::time::now_rfc3339;

use crate::cpu::collect_cpu;
use crate::engine::collect_engines;
use crate::memory::collect_memory_and_swap;
use crate::nvidia::collect_nvidia;
use crate::os::collect_os;
use crate::storage::collect_storage;
use crate::tools::collect_tools;
use crate::vulkan::collect_vulkan;
use crate::zram::collect_zram;

/// Collects a read-only diagnostic report for the current machine.
///
/// Missing optional tools and hardware produce unavailable or partial categories;
/// they do not prevent the report from being generated.
#[must_use]
pub fn collect_report(application_version: &str) -> DiagnosticEnvelope {
    if std::env::consts::OS != "linux" {
        return unsupported_platform_report(application_version);
    }

    let (os, os_category) = collect_os();
    let (cpu, cpu_category) = collect_cpu();
    let (memory, memory_category, swap, swap_category) = collect_memory_and_swap();
    let (zram, zram_category) = collect_zram();
    let (storage, storage_category) = collect_storage();
    let (nvidia_gpus, nvidia_category) = collect_nvidia();
    let (vulkan, vulkan_category) = collect_vulkan();
    let (tools, tools_category) = collect_tools();
    let (engines, engines_category) = collect_engines();

    let categories = CategoryStates {
        os: os_category,
        cpu: cpu_category,
        memory: memory_category,
        swap: swap_category,
        zram: zram_category,
        storage: storage_category,
        nvidia_gpu: nvidia_category,
        vulkan: vulkan_category,
        tools: tools_category,
        engines: engines_category,
    };
    let overall_state = categories.overall_state();
    let report = DiagnosticReport {
        overall_state,
        categories,
        system: SystemInfo {
            os,
            cpu,
            memory,
            swap,
            zram,
            storage,
            nvidia_gpus,
            vulkan,
            tools,
            engines,
        },
    };
    DiagnosticEnvelope::new(application_version, now_rfc3339(), report)
}

fn unsupported_platform_report(application_version: &str) -> DiagnosticEnvelope {
    let reason = format!(
        "GhostRAM diagnostics currently support Linux only; this host reports {}",
        std::env::consts::OS
    );
    let categories = CategoryStates {
        os: CategoryReport::unavailable(reason.clone()),
        cpu: CategoryReport::unavailable(reason.clone()),
        memory: CategoryReport::unavailable(reason.clone()),
        swap: CategoryReport::unavailable(reason.clone()),
        zram: CategoryReport::unavailable(reason.clone()),
        storage: CategoryReport::unavailable(reason.clone()),
        nvidia_gpu: CategoryReport::unavailable(reason.clone()),
        vulkan: CategoryReport::unavailable(reason.clone()),
        tools: CategoryReport::unavailable(reason.clone()),
        engines: CategoryReport::unavailable(reason),
    };
    let report = DiagnosticReport {
        overall_state: DiagnosticState::Unavailable,
        categories,
        system: SystemInfo::default(),
    };
    DiagnosticEnvelope::new(application_version, now_rfc3339(), report)
}
