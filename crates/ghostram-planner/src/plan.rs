//! Deterministic memory-plan construction from explicit inputs.
//!
//! The calculation core is pure: it does not scan directories, start
//! subprocesses, read the filesystem, or call an inference engine. The caller
//! supplies a model summary, a resource snapshot, and validated options.
//!
//! The plan compares only modeled weights and KV cache against effective
//! budgets. It never claims that a model will load, launch, or run safely.

use ghostram_core::diagnostic::DiagnosticState;
use ghostram_core::error::GhostramError;
use ghostram_core::quantity::Bytes;
use ghostram_models::{
    ArchitectureFields, CatalogWarning, Failure, Field, FileCatalogEntry, FilenameHints,
    FilenameRole, MetadataState, ParseStatus,
};
use serde::{Deserialize, Serialize};

use crate::budget::{Budget, BudgetComparison, sum_components};
use crate::error::{PlanError, PlanErrorKind};
use crate::evidence::Evidence;
use crate::kv::{KV_FORMULA_ID, KvCacheComponent, KvCacheType, estimate_kv_cache, layout_key_name};
use crate::resources::ResourceSnapshot;

/// Current plan schema identifier.
pub const MODELS_PLAN_SCHEMA_VERSION: &str = "ghostram.models.plan/1";
/// Default context length in tokens.
pub const DEFAULT_CONTEXT_TOKENS: u64 = 4096;
/// Maximum context length accepted by the planner.
pub const MAX_CONTEXT_TOKENS: u64 = 1_048_576;
/// Default number of independent sequences.
pub const DEFAULT_SEQUENCES: u64 = 1;
/// Maximum number of independent sequences accepted by the planner.
pub const MAX_SEQUENCES: u64 = 256;
/// Default host RAM reserve in MiB.
pub const DEFAULT_RAM_RESERVE_MIB: u64 = 1536;
/// Default device VRAM reserve in MiB.
pub const DEFAULT_VRAM_RESERVE_MIB: u64 = 1024;

const MIB: u64 = 1024 * 1024;

/// How the device for a hypothetical placement is selected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum DeviceSelection {
    /// Use the lowest-index observed NVIDIA device.
    #[default]
    FirstObserved,
    /// Use one explicit `nvidia-smi` device index.
    Explicit {
        /// Device index to select.
        index: u32,
    },
}

/// Planner options and the evidence for each value.
///
/// Candidate inputs are explicit command-line values or documented
/// product-policy defaults. A default is visible in the plan rather than being
/// silently treated as a measurement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanOptions {
    /// Context length per sequence, in tokens.
    pub context_tokens: Evidence<u64>,
    /// Number of independent sequences.
    pub sequences: Evidence<u64>,
    /// KV-cache element type.
    pub kv_cache_type: Evidence<KvCacheType>,
    /// Host memory withheld from the modeled plan.
    pub ram_reserve: Evidence<Bytes>,
    /// Device memory withheld from the modeled plan.
    pub vram_reserve: Evidence<Bytes>,
    /// Device selection policy.
    pub device_selection: DeviceSelection,
}

impl PlanOptions {
    /// Returns documented product-policy defaults.
    #[must_use]
    pub fn product_defaults() -> Self {
        Self {
            context_tokens: Evidence::product_default(
                DEFAULT_CONTEXT_TOKENS,
                "4,096-token planning default; not a model or engine maximum",
            ),
            sequences: Evidence::product_default(
                DEFAULT_SEQUENCES,
                "one independent sequence planning default",
            ),
            kv_cache_type: Evidence::product_default(
                KvCacheType::F16,
                "F16 KV-cache product default; F32 is supported, quantized KV caches are not modeled",
            ),
            ram_reserve: Evidence::product_default(
                Bytes::new(DEFAULT_RAM_RESERVE_MIB * MIB),
                "1,536 MiB host reserve; conservative product policy, not a universal requirement",
            ),
            vram_reserve: Evidence::product_default(
                Bytes::new(DEFAULT_VRAM_RESERVE_MIB * MIB),
                "1,024 MiB device reserve; conservative product policy, not a universal requirement",
            ),
            device_selection: DeviceSelection::FirstObserved,
        }
    }

    /// Validates that every option carries an explicit input or product default
    /// and that counts are inside documented planner limits.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError`] when an option is unknown, has the wrong evidence
    /// class, or is outside the supported range.
    pub fn validate(&self) -> Result<(), PlanError> {
        self.resolve().map(|_| ())
    }

    fn resolve(&self) -> Result<ResolvedOptions, PlanError> {
        let context_tokens = require_input(&self.context_tokens, "context_tokens")?;
        if !(1..=MAX_CONTEXT_TOKENS).contains(&context_tokens) {
            return Err(PlanError::new(
                PlanErrorKind::ContextOutOfRange,
                format!(
                    "context_tokens must be between 1 and {MAX_CONTEXT_TOKENS}, got {context_tokens}"
                ),
            ));
        }
        let sequences = require_input(&self.sequences, "sequences")?;
        if !(1..=MAX_SEQUENCES).contains(&sequences) {
            return Err(PlanError::new(
                PlanErrorKind::SequencesOutOfRange,
                format!("sequences must be between 1 and {MAX_SEQUENCES}, got {sequences}"),
            ));
        }
        require_input(&self.ram_reserve, "ram_reserve")?;
        require_input(&self.vram_reserve, "vram_reserve")?;
        Ok(ResolvedOptions {
            context_tokens,
            sequences,
            kv_cache_type: require_input(&self.kv_cache_type, "kv_cache_type")?,
            device_selection: self.device_selection,
        })
    }
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self::product_defaults()
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedOptions {
    context_tokens: u64,
    sequences: u64,
    kv_cache_type: KvCacheType,
    device_selection: DeviceSelection,
}

fn require_input<T: Copy>(evidence: &Evidence<T>, name: &str) -> Result<T, PlanError> {
    match evidence {
        Evidence::ExplicitInput { value } | Evidence::ProductDefault { value, .. } => Ok(*value),
        Evidence::Unknown { reason } => Err(PlanError::new(
            PlanErrorKind::MissingOption,
            format!("{name} has no value: {reason}"),
        )),
        other => Err(PlanError::new(
            PlanErrorKind::InvalidOptionEvidence,
            format!(
                "{name} must be explicit_input or product_default, not {}",
                other.class().as_str()
            ),
        )),
    }
}

/// Planner-relevant model data extracted from one catalog entry.
///
/// This is a compact boundary type. It retains architecture fields and parser
/// warnings needed for planning, but no tokenizer values, tensor payloads, file
/// hashes, or unrestricted metadata dumps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInput {
    /// Absolute path of the explicitly selected file.
    pub path: String,
    /// Observed filesystem size.
    pub file_size: Option<Bytes>,
    /// Structural parse outcome.
    pub parse_status: ParseStatus,
    /// Failure detail for a non-parsed file.
    pub failure: Option<Failure>,
    /// Parsed GGUF version.
    pub gguf_version: Option<u32>,
    /// Declared tensor descriptor count.
    pub tensor_count: Option<u64>,
    /// Whether required descriptive metadata was available.
    pub metadata_state: Option<MetadataState>,
    /// General architecture field.
    pub architecture: Option<Field<String>>,
    /// General model name.
    pub name: Option<Field<String>>,
    /// Quantization hint.
    pub quantization: Option<Field<String>>,
    /// Recognized architecture fields.
    pub architecture_fields: ArchitectureFields,
    /// Filename-only hints.
    pub filename_hints: FilenameHints,
    /// Non-fatal catalog warnings.
    pub metadata_warnings: Vec<CatalogWarning>,
}

impl From<&FileCatalogEntry> for ModelInput {
    fn from(entry: &FileCatalogEntry) -> Self {
        let metadata = entry.metadata.as_ref();
        Self {
            path: entry.path.clone(),
            file_size: entry.file_size,
            parse_status: entry.parse_status,
            failure: entry.failure.clone(),
            gguf_version: metadata.map(|metadata| metadata.gguf_version),
            tensor_count: metadata.map(|metadata| metadata.tensor_count),
            metadata_state: metadata.map(|metadata| metadata.metadata_state),
            architecture: metadata.and_then(|metadata| metadata.general.architecture.clone()),
            name: metadata.and_then(|metadata| metadata.general.name.clone()),
            quantization: metadata.and_then(|metadata| metadata.general.quantization.clone()),
            architecture_fields: metadata
                .map(|metadata| metadata.architecture_fields.clone())
                .unwrap_or_default(),
            filename_hints: entry.filename_hints.clone(),
            metadata_warnings: metadata
                .map(|metadata| metadata.warnings.clone())
                .unwrap_or_default(),
        }
    }
}

/// Compact model summary included in a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSummary {
    /// Absolute path of the selected file.
    pub path: String,
    /// Observed file size, or an unknown quantity with a reason.
    pub file_size: Evidence<Bytes>,
    /// Structural parse outcome.
    pub parse_status: ParseStatus,
    /// Failure detail for a non-parsed file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
    /// Parsed GGUF version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gguf_version: Option<u32>,
    /// Declared tensor descriptor count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tensor_count: Option<u64>,
    /// Metadata state, when metadata was available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_state: Option<MetadataState>,
    /// General architecture field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<Field<String>>,
    /// General model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Field<String>>,
    /// Quantization hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<Field<String>>,
    /// Declared context length, when present in GGUF metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_context_tokens: Option<Field<u64>>,
    /// Filename-only hints.
    pub filename_hints: FilenameHints,
    /// Non-fatal catalog warnings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata_warnings: Vec<CatalogWarning>,
}

impl ModelSummary {
    fn from_input(input: &ModelInput) -> Self {
        let file_size = match input.file_size {
            Some(value) => Evidence::observed(value),
            None => Evidence::unknown("filesystem size was not observed"),
        };
        Self {
            path: input.path.clone(),
            file_size,
            parse_status: input.parse_status,
            failure: input.failure.clone(),
            gguf_version: input.gguf_version,
            tensor_count: input.tensor_count,
            metadata_state: input.metadata_state,
            architecture: input.architecture.clone(),
            name: input.name.clone(),
            quantization: input.quantization.clone(),
            declared_context_tokens: input.architecture_fields.context_length.clone(),
            filename_hints: input.filename_hints.clone(),
            metadata_warnings: input.metadata_warnings.clone(),
        }
    }
}

/// How the model-weight component was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightMethod {
    /// The GGUF file size is used as a low-confidence proxy.
    GgufFileSizeProxy,
    /// No weight estimate was produced.
    Unavailable,
}

impl WeightMethod {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GgufFileSizeProxy => "gguf_file_size_proxy",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Model-weight component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightComponent {
    /// Observed file size used as the basis, when known.
    pub basis: Evidence<Bytes>,
    /// Estimated resident weights, or an unknown quantity with a reason.
    pub estimate: Evidence<Bytes>,
    /// Estimation method.
    pub method: WeightMethod,
}

/// Modeled components of one plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanComponents {
    /// Model weights.
    pub model_weights: WeightComponent,
    /// KV cache.
    pub kv_cache: KvCacheComponent,
    /// Backend and runtime overhead, which this planner does not model.
    pub runtime_overhead: Evidence<Bytes>,
}

/// Placement scenario identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioId {
    /// Weights and KV cache on host RAM.
    CpuWeightsAndKv,
    /// Weights on the selected device and KV cache on host RAM.
    DeviceWeightsCpuKv,
}

impl ScenarioId {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CpuWeightsAndKv => "cpu_weights_and_kv",
            Self::DeviceWeightsCpuKv => "device_weights_cpu_kv",
        }
    }
}

/// Qualified scenario outcomes. None of these means that a model will launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioOutcome {
    /// Every modeled component is within its resource budget.
    EstimatedComponentsWithinBudget,
    /// At least one modeled component exceeds its resource budget.
    EstimatedComponentsExceedBudget,
    /// A required observation or estimate is unavailable.
    InsufficientData,
}

impl ScenarioOutcome {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EstimatedComponentsWithinBudget => "estimated_components_within_budget",
            Self::EstimatedComponentsExceedBudget => "estimated_components_exceed_budget",
            Self::InsufficientData => "insufficient_data",
        }
    }
}

/// One placement scenario and its resource comparisons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenario {
    /// Scenario identifier.
    pub id: ScenarioId,
    /// Human-readable placement description.
    pub description: String,
    /// Host budget comparison, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<BudgetComparison>,
    /// Device budget comparison, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<BudgetComparison>,
    /// Qualified outcome.
    pub outcome: ScenarioOutcome,
    /// Why the outcome is not a within-budget result.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    /// Scope limits for this scenario.
    pub caveats: Vec<String>,
}

/// How the budget-limited context upper bound was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextMethod {
    /// Host effective budget minus the file-size weight proxy, divided by
    /// per-token KV bytes.
    HostBudgetAfterWeightProxy,
    /// No upper bound was produced.
    Unavailable,
}

impl ContextMethod {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostBudgetAfterWeightProxy => "host_budget_after_weight_proxy",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Optional budget-limited context upper bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextEstimate {
    /// Estimated token upper bound, or an unknown quantity with a reason.
    pub estimate: Evidence<u64>,
    /// Calculation method.
    pub method: ContextMethod,
    /// Whether the value was capped by the planner's context range.
    pub capped_at_limit: bool,
    /// Scope note for the value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Stable plan warning categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanWarningCode {
    /// Weight estimate uses the GGUF file size.
    FileSizeProxy,
    /// Runtime overhead is not modeled.
    RuntimeOverheadUnmodeled,
    /// The selected model was not parsed.
    ModelNotParsed,
    /// Required descriptive metadata was incomplete.
    MetadataIncomplete,
    /// The selected file appears to be one shard.
    ShardedFileSelected,
    /// The filename suggests a sidecar or adapter.
    SidecarFilenameHint,
    /// Attention layout metadata is outside the supported KV profile.
    UnsupportedAttentionLayout,
    /// KV cache is unavailable.
    KvCacheUnavailable,
    /// Requested context exceeds the declared GGUF context length.
    RequestedContextExceedsDeclared,
    /// Host memory observation is incomplete.
    HostObservationIncomplete,
    /// NVIDIA device observation is incomplete.
    DeviceObservationIncomplete,
    /// More than one NVIDIA device was observed.
    MultipleDevicesObserved,
    /// The explicitly selected device was not observed.
    ExplicitDeviceMissing,
    /// Multiple sequences use an independent full-context assumption.
    IndependentSequencesAssumption,
    /// Host availability does not exceed the configured reserve.
    HostBudgetExhaustedByReserve,
    /// Device availability does not exceed the configured reserve.
    DeviceBudgetExhaustedByReserve,
    /// The context upper bound was capped by the planner range.
    ContextEstimateCapped,
    /// The context upper bound exceeds the declared GGUF context length.
    ContextEstimateExceedsDeclared,
    /// The filesystem size needed for the weight proxy is unavailable.
    FileSizeUnavailable,
}

impl PlanWarningCode {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileSizeProxy => "file_size_proxy",
            Self::RuntimeOverheadUnmodeled => "runtime_overhead_unmodeled",
            Self::ModelNotParsed => "model_not_parsed",
            Self::MetadataIncomplete => "metadata_incomplete",
            Self::ShardedFileSelected => "sharded_file_selected",
            Self::SidecarFilenameHint => "sidecar_filename_hint",
            Self::UnsupportedAttentionLayout => "unsupported_attention_layout",
            Self::KvCacheUnavailable => "kv_cache_unavailable",
            Self::RequestedContextExceedsDeclared => "requested_context_exceeds_declared",
            Self::HostObservationIncomplete => "host_observation_incomplete",
            Self::DeviceObservationIncomplete => "device_observation_incomplete",
            Self::MultipleDevicesObserved => "multiple_devices_observed",
            Self::ExplicitDeviceMissing => "explicit_device_missing",
            Self::IndependentSequencesAssumption => "independent_sequences_assumption",
            Self::HostBudgetExhaustedByReserve => "host_budget_exhausted_by_reserve",
            Self::DeviceBudgetExhaustedByReserve => "device_budget_exhausted_by_reserve",
            Self::ContextEstimateCapped => "context_estimate_capped",
            Self::ContextEstimateExceedsDeclared => "context_estimate_exceeds_declared",
            Self::FileSizeUnavailable => "file_size_unavailable",
        }
    }
}

/// One plan warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanWarning {
    /// Stable warning category.
    pub code: PlanWarningCode,
    /// Human-readable detail.
    pub message: String,
}

/// Stable unknown-quantity categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanUnknownCode {
    /// Runtime and backend overhead.
    RuntimeOverhead,
    /// Resident model weights.
    ModelWeights,
    /// KV cache.
    KvCache,
    /// Host available memory.
    HostAvailableMemory,
    /// Selected-device free memory.
    DeviceFreeMemory,
}

impl PlanUnknownCode {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeOverhead => "runtime_overhead",
            Self::ModelWeights => "model_weights",
            Self::KvCache => "kv_cache",
            Self::HostAvailableMemory => "host_available_memory",
            Self::DeviceFreeMemory => "device_free_memory",
        }
    }
}

/// One unresolved quantity retained by the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanUnknown {
    /// Stable unknown category.
    pub code: PlanUnknownCode,
    /// Why the quantity is unknown.
    pub description: String,
}

/// Complete memory plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Compact model summary.
    pub model: ModelSummary,
    /// Options and their evidence.
    pub options: PlanOptions,
    /// Host and device observations.
    pub resources: ResourceSnapshot,
    /// Selected device index, when one was observed and selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_device_index: Option<u32>,
    /// Modeled components.
    pub components: PlanComponents,
    /// CPU and hypothetical device scenarios, in deterministic order.
    pub scenarios: Vec<Scenario>,
    /// Budget-limited context upper bound.
    pub context_estimate: ContextEstimate,
    /// Unresolved quantities.
    pub unknowns: Vec<PlanUnknown>,
    /// Planning warnings.
    pub warnings: Vec<PlanWarning>,
    /// Scope limits that apply to the whole plan.
    pub caveats: Vec<String>,
}

/// Versioned plan envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanEnvelope {
    /// Schema identifier.
    pub schema_version: String,
    /// GhostRAM application version.
    pub application_version: String,
    /// UTC RFC 3339 generation time.
    pub generated_at: String,
    /// Plan payload.
    pub plan: Plan,
}

impl PlanEnvelope {
    /// Creates an envelope with the current schema identifier.
    #[must_use]
    pub fn new(
        application_version: impl Into<String>,
        generated_at: impl Into<String>,
        plan: Plan,
    ) -> Self {
        Self {
            schema_version: MODELS_PLAN_SCHEMA_VERSION.to_string(),
            application_version: application_version.into(),
            generated_at: generated_at.into(),
            plan,
        }
    }

    /// Checks that this envelope uses the schema this build understands.
    ///
    /// # Errors
    ///
    /// Returns [`GhostramError::InvalidInput`] for a different schema version.
    pub fn validate_schema(&self) -> Result<(), GhostramError> {
        if self.schema_version == MODELS_PLAN_SCHEMA_VERSION {
            Ok(())
        } else {
            Err(GhostramError::InvalidInput {
                message: format!(
                    "unsupported plan schema version {:?}; expected {MODELS_PLAN_SCHEMA_VERSION}",
                    self.schema_version
                ),
            })
        }
    }
}

/// Builds a deterministic plan from explicit inputs.
///
/// # Errors
///
/// Returns [`PlanError`] when options are unknown, use invalid evidence, or fall
/// outside documented planner limits. Model and observation failures are
/// represented inside the returned plan instead of aborting it.
pub fn build_plan(
    model: &ModelInput,
    resources: &ResourceSnapshot,
    options: &PlanOptions,
) -> Result<Plan, PlanError> {
    let resolved = options.resolve()?;
    let summary = ModelSummary::from_input(model);
    let mut warnings = Vec::new();
    let mut unknowns = Vec::new();

    add_model_warnings(model, &mut warnings);
    let weights = estimate_weights(model, &mut warnings);
    let kv_cache = estimate_kv_cache(
        model.parse_status,
        model
            .architecture
            .as_ref()
            .map(|architecture| architecture.value.as_str()),
        &model.architecture_fields,
        resolved.context_tokens,
        resolved.sequences,
        resolved.kv_cache_type,
    );
    add_kv_warnings(model, &kv_cache, &mut warnings);

    if resolved.sequences > 1 {
        push_warning(
            &mut warnings,
            PlanWarningCode::IndependentSequencesAssumption,
            format!(
                "{} independent sequences are modeled; each sequence is assumed to hold the full context, so a unified KV cache may use less",
                resolved.sequences
            ),
        );
    }
    let declared_context = model
        .architecture_fields
        .context_length
        .as_ref()
        .map(|context| context.value);
    if let Some(declared) = declared_context {
        if resolved.context_tokens > declared {
            push_warning(
                &mut warnings,
                PlanWarningCode::RequestedContextExceedsDeclared,
                format!(
                    "requested context {} exceeds the declared GGUF context length {declared}; the model may not support the request",
                    resolved.context_tokens
                ),
            );
        }
    }

    add_resource_warnings(resources, &mut warnings);
    let selected_device = select_device(resources, resolved.device_selection, &mut warnings);
    let host_budget = Budget::new(
        resources.host.available.clone(),
        options.ram_reserve.clone(),
    );
    if !host_budget.usable && host_budget.available.value().is_some() {
        if let Some(reason) = &host_budget.reason {
            push_warning(
                &mut warnings,
                PlanWarningCode::HostBudgetExhaustedByReserve,
                reason.clone(),
            );
        }
    }
    let device_budget = selected_device
        .map(|device| Budget::new(device.free.clone(), options.vram_reserve.clone()));
    if let Some(budget) = &device_budget {
        if !budget.usable && budget.available.value().is_some() {
            if let Some(reason) = &budget.reason {
                push_warning(
                    &mut warnings,
                    PlanWarningCode::DeviceBudgetExhaustedByReserve,
                    reason.clone(),
                );
            }
        }
    }

    let runtime_overhead = Evidence::unknown(
        "backend workspace, compute buffers, graph allocations, and driver runtime overhead are not modeled",
    );
    let components = PlanComponents {
        model_weights: weights,
        kv_cache,
        runtime_overhead: runtime_overhead.clone(),
    };

    let scenarios = build_scenarios(
        &components,
        &host_budget,
        device_budget.as_ref(),
        selected_device,
        resolved.sequences,
    );
    let context_estimate =
        estimate_context(&components, &host_budget, declared_context, &mut warnings);

    add_component_unknowns(&components, resources, selected_device, &mut unknowns);
    push_warning(
        &mut warnings,
        PlanWarningCode::RuntimeOverheadUnmodeled,
        "backend workspace, compute buffers, and driver runtime overhead are excluded from every scenario",
    );

    Ok(Plan {
        model: summary,
        options: options.clone(),
        resources: resources.clone(),
        selected_device_index: selected_device.map(|device| device.index),
        components,
        scenarios,
        context_estimate,
        unknowns,
        warnings,
        caveats: plan_caveats(),
    })
}

fn add_model_warnings(model: &ModelInput, warnings: &mut Vec<PlanWarning>) {
    if model.parse_status != ParseStatus::Parsed {
        let detail = model
            .failure
            .as_ref()
            .map(|failure| format!(": {}", failure.message))
            .unwrap_or_default();
        push_warning(
            warnings,
            PlanWarningCode::ModelNotParsed,
            format!(
                "model parse status is {}{}; parsed metadata is unavailable",
                parse_status_name(model.parse_status),
                detail
            ),
        );
    }
    if model.metadata_state == Some(MetadataState::Incomplete) {
        push_warning(
            warnings,
            PlanWarningCode::MetadataIncomplete,
            "required descriptive metadata is incomplete; some components may be unavailable",
        );
    }
    if let Some(shard) = model.filename_hints.shard {
        if shard.total > 1 {
            push_warning(
                warnings,
                PlanWarningCode::ShardedFileSelected,
                format!(
                    "filename suggests shard {} of {}; shard grouping is not implemented, so total model weights are unknown",
                    shard.number, shard.total
                ),
            );
        }
    }
    if let Some(role) = model.filename_hints.role {
        push_warning(
            warnings,
            PlanWarningCode::SidecarFilenameHint,
            format!(
                "filename suggests a {} role; only the selected file is planned and no base model is combined",
                role_name(role)
            ),
        );
    }
}

fn estimate_weights(model: &ModelInput, warnings: &mut Vec<PlanWarning>) -> WeightComponent {
    let basis = match model.file_size {
        Some(value) => Evidence::observed(value),
        None => Evidence::unknown("filesystem size was not observed"),
    };
    if model.parse_status != ParseStatus::Parsed {
        return WeightComponent {
            basis,
            estimate: Evidence::unknown(
                "the selected file was not parsed as a supported GGUF structure, so the file size is not used as a weight proxy",
            ),
            method: WeightMethod::Unavailable,
        };
    }
    let Some(file_size) = model.file_size else {
        push_warning(
            warnings,
            PlanWarningCode::FileSizeUnavailable,
            "filesystem size was unavailable, so model weights cannot be estimated",
        );
        return WeightComponent {
            basis,
            estimate: Evidence::unknown(
                "filesystem size was unavailable, so the file-size weight proxy cannot be applied",
            ),
            method: WeightMethod::Unavailable,
        };
    };
    if let Some(shard) = model.filename_hints.shard {
        if shard.total > 1 {
            return WeightComponent {
                basis,
                estimate: Evidence::unknown(format!(
                    "the file appears to be shard {} of {}; shard grouping is not implemented",
                    shard.number, shard.total
                )),
                method: WeightMethod::Unavailable,
            };
        }
    }
    push_warning(
        warnings,
        PlanWarningCode::FileSizeProxy,
        "model weights use GGUF file size as a low-confidence proxy; exact resident tensor allocation, alignment padding, quantization storage, and mmap behavior are not modeled",
    );
    WeightComponent {
        basis,
        estimate: Evidence::estimated(
            file_size,
            "low-confidence proxy: GGUF file size; exact resident tensor allocation, alignment padding, quantization storage, and mmap behavior are not modeled",
        ),
        method: WeightMethod::GgufFileSizeProxy,
    }
}

fn add_kv_warnings(model: &ModelInput, kv: &KvCacheComponent, warnings: &mut Vec<PlanWarning>) {
    if !model.architecture_fields.attention_layout_keys.is_empty() {
        let keys = model
            .architecture_fields
            .attention_layout_keys
            .iter()
            .map(|key| layout_key_name(*key))
            .collect::<Vec<_>>()
            .join(", ");
        push_warning(
            warnings,
            PlanWarningCode::UnsupportedAttentionLayout,
            format!("attention layout metadata is outside the uniform KV profile: {keys}"),
        );
    }
    if let Some(unavailable) = &kv.unavailable {
        push_warning(
            warnings,
            PlanWarningCode::KvCacheUnavailable,
            format!("{}: {}", unavailable.code.as_str(), unavailable.message),
        );
    }
}

fn add_resource_warnings(resources: &ResourceSnapshot, warnings: &mut Vec<PlanWarning>) {
    if resources.host.state.state != DiagnosticState::Available {
        push_warning(
            warnings,
            PlanWarningCode::HostObservationIncomplete,
            format!(
                "host memory observation is {}: {}",
                diagnostic_state_name(resources.host.state.state),
                resources
                    .host
                    .state
                    .reason
                    .as_deref()
                    .unwrap_or("no reason reported")
            ),
        );
    }
    if resources.devices_state.state != DiagnosticState::Available || resources.devices.is_empty() {
        let detail = if resources.devices.is_empty() {
            "no NVIDIA device was reported".to_string()
        } else {
            resources
                .devices_state
                .reason
                .clone()
                .unwrap_or_else(|| "no reason reported".to_string())
        };
        push_warning(
            warnings,
            PlanWarningCode::DeviceObservationIncomplete,
            format!(
                "NVIDIA device observation is {}: {detail}",
                diagnostic_state_name(resources.devices_state.state)
            ),
        );
    }
    if resources.devices.len() > 1 {
        push_warning(
            warnings,
            PlanWarningCode::MultipleDevicesObserved,
            format!(
                "{} NVIDIA devices were observed; only one selected device is planned",
                resources.devices.len()
            ),
        );
    }
}

fn select_device<'a>(
    resources: &'a ResourceSnapshot,
    selection: DeviceSelection,
    warnings: &mut Vec<PlanWarning>,
) -> Option<&'a crate::resources::DeviceResources> {
    match selection {
        DeviceSelection::FirstObserved => resources.devices.first(),
        DeviceSelection::Explicit { index } => {
            let device = resources
                .devices
                .iter()
                .find(|device| device.index == index);
            if device.is_none() {
                push_warning(
                    warnings,
                    PlanWarningCode::ExplicitDeviceMissing,
                    format!("explicit device index {index} was not observed"),
                );
            }
            device
        }
    }
}

fn build_scenarios(
    components: &PlanComponents,
    host_budget: &Budget,
    device_budget: Option<&Budget>,
    selected_device: Option<&crate::resources::DeviceResources>,
    sequences: u64,
) -> Vec<Scenario> {
    let cpu_sum = sum_components(
        &[
            &components.model_weights.estimate,
            &components.kv_cache.estimate,
        ],
        "sum of modeled model weights and KV cache for the CPU scenario",
    );
    let cpu_host = BudgetComparison::new(host_budget.clone(), cpu_sum);
    let cpu_outcome = evaluate_comparisons(&[Some(&cpu_host)]);
    let cpu_reasons = comparison_reasons(&[(Some(&cpu_host), "host")], None);
    let mut cpu_caveats = vec![
        "Weights and KV cache are treated as host-resident.".to_string(),
        scenario_scope_caveat(),
    ];
    add_sequence_caveat(&mut cpu_caveats, sequences);

    let device_host =
        BudgetComparison::new(host_budget.clone(), components.kv_cache.estimate.clone());
    let device_comparison = device_budget.map(|budget| {
        BudgetComparison::new(budget.clone(), components.model_weights.estimate.clone())
    });
    let (device_outcome, device_reasons) = if selected_device.is_none() {
        (
            ScenarioOutcome::InsufficientData,
            vec![
                "no NVIDIA device was selected or observed, so device weights cannot be compared"
                    .to_string(),
            ],
        )
    } else {
        let outcome = evaluate_comparisons(&[Some(&device_host), device_comparison.as_ref()]);
        let reasons = comparison_reasons(
            &[
                (Some(&device_host), "host"),
                (device_comparison.as_ref(), "device"),
            ],
            None,
        );
        (outcome, reasons)
    };
    let mut device_caveats = vec![
        "Weights are treated as fully device-resident; KV cache is treated as host-resident. Partial offload is not modeled.".to_string(),
        scenario_scope_caveat(),
    ];
    add_sequence_caveat(&mut device_caveats, sequences);

    vec![
        Scenario {
            id: ScenarioId::CpuWeightsAndKv,
            description: "Model weights and KV cache are assigned to host RAM.".to_string(),
            host: Some(cpu_host),
            device: None,
            outcome: cpu_outcome,
            reasons: cpu_reasons,
            caveats: cpu_caveats,
        },
        Scenario {
            id: ScenarioId::DeviceWeightsCpuKv,
            description: "Model weights are assigned to the selected NVIDIA device; KV cache is assigned to host RAM.".to_string(),
            host: Some(device_host),
            device: device_comparison,
            outcome: device_outcome,
            reasons: device_reasons,
            caveats: device_caveats,
        },
    ]
}

fn evaluate_comparisons(comparisons: &[Option<&BudgetComparison>]) -> ScenarioOutcome {
    if comparisons
        .iter()
        .flatten()
        .any(|comparison| comparison.within_budget == Some(false))
    {
        ScenarioOutcome::EstimatedComponentsExceedBudget
    } else if comparisons
        .iter()
        .flatten()
        .any(|comparison| comparison.within_budget.is_none())
        || comparisons.iter().any(Option::is_none)
    {
        ScenarioOutcome::InsufficientData
    } else {
        ScenarioOutcome::EstimatedComponentsWithinBudget
    }
}

fn comparison_reasons(
    comparisons: &[(Option<&BudgetComparison>, &str)],
    missing: Option<&str>,
) -> Vec<String> {
    let mut reasons = Vec::new();
    for (comparison, label) in comparisons {
        let Some(comparison) = comparison else {
            continue;
        };
        if let Some(shortfall) = &comparison.shortfall {
            if let Some(value) = shortfall.value() {
                reasons.push(format!(
                    "{label} modeled components exceed the effective budget by {value}"
                ));
            }
        }
        if comparison.within_budget.is_none() {
            if let Some(reason) = &comparison.budget.reason {
                reasons.push(format!("{label} budget: {reason}"));
            }
            if let Some(reason) = comparison.estimated_components.reason() {
                reasons.push(format!("{label} components: {reason}"));
            }
        }
    }
    if let Some(reason) = missing {
        reasons.push(reason.to_string());
    }
    reasons
}

fn estimate_context(
    components: &PlanComponents,
    host_budget: &Budget,
    declared_context: Option<u64>,
    warnings: &mut Vec<PlanWarning>,
) -> ContextEstimate {
    let Some(weights) = components.model_weights.estimate.value().copied() else {
        return unavailable_context(
            components
                .model_weights
                .estimate
                .reason()
                .unwrap_or("model weight estimate is unavailable"),
        );
    };
    let Some(kv_dimensions) = components.kv_cache.dimensions() else {
        return unavailable_context(
            components
                .kv_cache
                .estimate
                .reason()
                .unwrap_or("KV-cache dimensions are unavailable"),
        );
    };
    let Some(effective) = host_budget.effective.value().copied() else {
        return unavailable_context(
            host_budget
                .reason
                .as_deref()
                .unwrap_or("host effective budget is unavailable"),
        );
    };
    if !host_budget.usable || effective.as_u64() <= weights.as_u64() {
        return unavailable_context(
            "host effective budget does not exceed the file-size weight proxy",
        );
    }
    let Some(per_token) = kv_dimensions.per_token_bytes() else {
        return unavailable_context("per-token KV-cache arithmetic overflowed u64");
    };
    if per_token == 0 {
        return unavailable_context("per-token KV-cache bytes are zero");
    }
    let remaining = effective.as_u64() - weights.as_u64();
    let upper = remaining / per_token;
    if upper == 0 {
        return unavailable_context(
            "remaining host budget is smaller than one token of modeled KV cache",
        );
    }
    let capped = upper > MAX_CONTEXT_TOKENS;
    let tokens = upper.min(MAX_CONTEXT_TOKENS);
    if capped {
        push_warning(
            warnings,
            PlanWarningCode::ContextEstimateCapped,
            format!(
                "the budget-limited context upper bound was capped at the planner limit of {MAX_CONTEXT_TOKENS} tokens"
            ),
        );
    }
    if let Some(declared) = declared_context {
        if tokens > declared {
            push_warning(
                warnings,
                PlanWarningCode::ContextEstimateExceedsDeclared,
                format!(
                    "the budget-limited context upper bound {tokens} exceeds the declared GGUF context length {declared}; the model may not support the larger value"
                ),
            );
        }
    }
    ContextEstimate {
        estimate: Evidence::estimated(
            tokens,
            "upper bound from host effective budget after the file-size weight proxy; not a recommended or validated context, and runtime overhead is excluded",
        ),
        method: ContextMethod::HostBudgetAfterWeightProxy,
        capped_at_limit: capped,
        note: Some(
            "This is a budget upper bound, not a model, engine, or backend context limit."
                .to_string(),
        ),
    }
}

fn unavailable_context(reason: &str) -> ContextEstimate {
    ContextEstimate {
        estimate: Evidence::unknown(reason.to_string()),
        method: ContextMethod::Unavailable,
        capped_at_limit: false,
        note: None,
    }
}

fn add_component_unknowns(
    components: &PlanComponents,
    resources: &ResourceSnapshot,
    selected_device: Option<&crate::resources::DeviceResources>,
    unknowns: &mut Vec<PlanUnknown>,
) {
    push_unknown(
        unknowns,
        PlanUnknownCode::RuntimeOverhead,
        components
            .runtime_overhead
            .reason()
            .unwrap_or("runtime overhead is not modeled"),
    );
    if components.model_weights.estimate.value().is_none() {
        push_unknown(
            unknowns,
            PlanUnknownCode::ModelWeights,
            components
                .model_weights
                .estimate
                .reason()
                .unwrap_or("model weights are unavailable"),
        );
    }
    if components.kv_cache.estimate.value().is_none() {
        push_unknown(
            unknowns,
            PlanUnknownCode::KvCache,
            components
                .kv_cache
                .estimate
                .reason()
                .unwrap_or("KV cache is unavailable"),
        );
    }
    if resources.host.available.value().is_none() {
        push_unknown(
            unknowns,
            PlanUnknownCode::HostAvailableMemory,
            resources
                .host
                .available
                .reason()
                .unwrap_or("host available memory is unavailable"),
        );
    }
    match selected_device {
        Some(device) => {
            if device.free.value().is_none() {
                push_unknown(
                    unknowns,
                    PlanUnknownCode::DeviceFreeMemory,
                    device
                        .free
                        .reason()
                        .unwrap_or("free device memory is unavailable"),
                );
            }
        }
        None => push_unknown(
            unknowns,
            PlanUnknownCode::DeviceFreeMemory,
            "no NVIDIA device was selected or observed",
        ),
    }
}

fn push_warning(
    warnings: &mut Vec<PlanWarning>,
    code: PlanWarningCode,
    message: impl Into<String>,
) {
    if !warnings.iter().any(|warning| warning.code == code) {
        warnings.push(PlanWarning {
            code,
            message: message.into(),
        });
    }
}

fn push_unknown(unknowns: &mut Vec<PlanUnknown>, code: PlanUnknownCode, description: &str) {
    if !unknowns.iter().any(|unknown| unknown.code == code) {
        unknowns.push(PlanUnknown {
            code,
            description: description.to_string(),
        });
    }
}

fn scenario_scope_caveat() -> String {
    "Only modeled weights and KV cache are compared; launch success, engine support, backend workspace, and runtime overhead are unverified.".to_string()
}

fn add_sequence_caveat(caveats: &mut Vec<String>, sequences: u64) {
    if sequences > 1 {
        caveats.push(format!(
            "KV cache assumes {sequences} independent full-context sequences; a unified cache may use less."
        ));
    }
}

fn plan_caveats() -> Vec<String> {
    vec![
        "Planning estimates are not measurements; no inference engine was launched and no benchmark was run.".to_string(),
        "A scenario outcome does not predict that a model will load, launch, or run safely.".to_string(),
        scenario_scope_caveat(),
        "GGUF file size is a low-confidence proxy for resident model weights; tensor allocation, alignment, quantization storage, and mmap behavior are not modeled.".to_string(),
        format!(
            "KV-cache estimates use the {KV_FORMULA_ID} formula for a uniform full-attention profile and exclude backend padding, alignment, sliding windows, and workspace."
        ),
        "Partial offload, compute buffers, graph workspace, and driver/runtime overhead are not modeled.".to_string(),
        "Device presence does not prove that a CUDA or Vulkan backend is installed, compatible, or able to launch the model.".to_string(),
    ]
}

fn parse_status_name(status: ParseStatus) -> &'static str {
    match status {
        ParseStatus::Parsed => "parsed",
        ParseStatus::Unsupported => "unsupported",
        ParseStatus::Malformed => "malformed",
        ParseStatus::Unreadable => "unreadable",
        ParseStatus::Changed => "changed",
    }
}

fn diagnostic_state_name(state: DiagnosticState) -> &'static str {
    match state {
        DiagnosticState::Available => "available",
        DiagnosticState::Partial => "partial",
        DiagnosticState::Unavailable => "unavailable",
    }
}

fn role_name(role: FilenameRole) -> &'static str {
    match role {
        FilenameRole::Mmproj => "multimodal projector sidecar",
        FilenameRole::Mtp => "multi-token-prediction sidecar",
        FilenameRole::Lora => "LoRA adapter",
        FilenameRole::Vocab => "vocabulary sidecar",
    }
}
