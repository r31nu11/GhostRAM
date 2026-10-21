//! Deterministic memory planning and safety budgets for GhostRAM.
//!
//! The planner is a pure calculation core. It accepts model metadata, a resource
//! snapshot, and explicit options; it does not scan directories, probe hardware,
//! start subprocesses, read files, call an inference engine, or run benchmarks.
//!
//! Results are evidence-tagged estimates. A scenario can report that modeled
//! components are estimated to be within or above a budget, or that there is
//! insufficient data. It never claims that a model will load, launch, or run
//! safely.
//!
//! See `docs/MEMORY_PLANNER.md` and `docs/PLAN_SCHEMA.md` for the behavioral and
//! JSON contracts.

pub mod budget;
pub mod error;
pub mod evidence;
pub mod kv;
pub mod plan;
pub mod resources;

pub use budget::{Budget, BudgetComparison, sum_components};
pub use error::{PlanError, PlanErrorKind};
pub use evidence::{Evidence, EvidenceClass};
pub use kv::{
    KV_FORMULA_ID, KvCacheComponent, KvCacheType, KvDimensions, KvMethod, KvUnavailable,
    KvUnavailableCode, LLAMA_ARCHITECTURE, LLAMA_PROFILE_ID, estimate_kv_cache,
};
pub use plan::{
    ContextEstimate, ContextMethod, DEFAULT_CONTEXT_TOKENS, DEFAULT_RAM_RESERVE_MIB,
    DEFAULT_SEQUENCES, DEFAULT_VRAM_RESERVE_MIB, DeviceSelection, MAX_CONTEXT_TOKENS,
    MAX_SEQUENCES, MODELS_PLAN_SCHEMA_VERSION, ModelInput, ModelSummary, Plan, PlanComponents,
    PlanEnvelope, PlanOptions, PlanUnknown, PlanUnknownCode, PlanWarning, PlanWarningCode,
    Scenario, ScenarioId, ScenarioOutcome, WeightComponent, WeightMethod, build_plan,
};
pub use resources::{DeviceResources, HostResources, ObservationSource, ResourceSnapshot};
