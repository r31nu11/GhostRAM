//! Lightweight, read-only observations used by the memory planner.
//!
//! The full doctor report also inspects storage, tools, and engines. Planning
//! only needs host memory and NVIDIA device memory, so this module composes the
//! existing collectors without running unrelated probes.

use ghostram_core::diagnostic::{CategoryReport, MemoryInfo, NvidiaGpuInfo};
use ghostram_core::time::now_rfc3339;

use crate::memory::collect_memory_and_swap;
use crate::nvidia::collect_nvidia;

/// Host and device observations with the time they were collected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningObservations {
    /// UTC RFC 3339 time at which collection started.
    pub observed_at: String,
    /// Host memory fields from `/proc/meminfo`, when readable.
    pub memory: Option<MemoryInfo>,
    /// Whether the memory category is complete, partial, or unavailable.
    pub memory_state: CategoryReport,
    /// NVIDIA devices reported by `nvidia-smi`, in probe order.
    pub nvidia_gpus: Vec<NvidiaGpuInfo>,
    /// Whether the NVIDIA category is complete, partial, or unavailable.
    pub nvidia_state: CategoryReport,
}

/// Collects the host and device observations needed for one memory plan.
///
/// Missing tools and devices are represented by category states; they do not
/// make collection fail. The function never installs software, starts an
/// engine, or loads a model.
#[must_use]
pub fn collect_planning_observations() -> PlanningObservations {
    let observed_at = now_rfc3339();
    let (memory, memory_state, _, _) = collect_memory_and_swap();
    let (nvidia_gpus, nvidia_state) = collect_nvidia();
    PlanningObservations {
        observed_at,
        memory,
        memory_state,
        nvidia_gpus,
        nvidia_state,
    }
}
