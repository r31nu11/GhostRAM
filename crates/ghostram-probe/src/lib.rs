//! Read-only Linux diagnostics for GhostRAM.
//!
//! Collection is separated from presentation. This crate may read `/proc`,
//! `/sys`, OS metadata, and run bounded read-only system utilities. It never
//! installs software, modifies system configuration, starts an inference server,
//! or loads a model.

mod command;
mod cpu;
mod engine;
mod memory;
mod nvidia;
mod os;
mod planning;
mod report;
mod storage;
mod tools;
mod vulkan;
mod zram;

pub use planning::{PlanningObservations, collect_planning_observations};
pub use report::collect_report;
