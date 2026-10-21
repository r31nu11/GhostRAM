//! Integration tests for the public planner contract.

use ghostram_core::diagnostic::{CategoryReport, MemoryInfo, NvidiaGpuInfo};
use ghostram_core::quantity::Bytes;
use ghostram_models::{
    ArchitectureFields, ArchitectureKey, AttentionLayoutKey, CatalogWarning, Failure, FailureKind,
    Field, FileCatalogEntry, FilenameHints, GeneralMetadata, HashAlgorithm, HashReport, HashStatus,
    MetadataState, ModelMetadata, ParseStatus, ShardHint, WarningCode,
};
use ghostram_planner::{
    DEFAULT_RAM_RESERVE_MIB, DEFAULT_VRAM_RESERVE_MIB, DeviceSelection, Evidence, EvidenceClass,
    MAX_CONTEXT_TOKENS, MODELS_PLAN_SCHEMA_VERSION, ModelInput, PlanEnvelope, PlanErrorKind,
    PlanOptions, PlanUnknownCode, PlanWarningCode, ResourceSnapshot, ScenarioId, ScenarioOutcome,
    WeightMethod, build_plan,
};

fn gib(value: u64) -> Bytes {
    Bytes::from_gib(value).unwrap()
}

fn mib(value: u64) -> Bytes {
    Bytes::from_mib(value).unwrap()
}

fn architecture_fields() -> ArchitectureFields {
    ArchitectureFields {
        context_length: Some(Field::explicit(131_072)),
        block_count: Some(Field::explicit(32)),
        embedding_length: Some(Field::explicit(4096)),
        attention_head_count: Some(Field::explicit(32)),
        attention_head_count_kv: Some(Field::explicit(8)),
        present_keys: vec![
            ArchitectureKey::ContextLength,
            ArchitectureKey::BlockCount,
            ArchitectureKey::EmbeddingLength,
            ArchitectureKey::AttentionHeadCount,
            ArchitectureKey::AttentionHeadCountKv,
        ],
        ..ArchitectureFields::default()
    }
}

fn model() -> ModelInput {
    ModelInput {
        path: "/models/test.gguf".to_string(),
        file_size: Some(gib(2)),
        parse_status: ParseStatus::Parsed,
        failure: None,
        gguf_version: Some(3),
        tensor_count: Some(100),
        metadata_state: Some(MetadataState::Available),
        architecture: Some(Field::explicit("llama".to_string())),
        name: Some(Field::explicit("Synthetic Test Model".to_string())),
        quantization: Some(Field::derived("Q4_K_M".to_string())),
        architecture_fields: architecture_fields(),
        filename_hints: FilenameHints::default(),
        metadata_warnings: Vec::new(),
    }
}

fn gpu(index: u32, total_gib: u64, used_gib: u64) -> NvidiaGpuInfo {
    NvidiaGpuInfo {
        index,
        name: format!("Synthetic GPU {index}"),
        driver_version: Some("999.0".to_string()),
        memory_total_bytes: Some(gib(total_gib)),
        memory_used_bytes: Some(gib(used_gib)),
        compute_capability: Some("6.1".to_string()),
    }
}

fn memory(total_gib: u64, available_gib: Option<u64>) -> MemoryInfo {
    MemoryInfo {
        total_bytes: Some(gib(total_gib)),
        available_bytes: available_gib.map(gib),
        free_bytes: Some(gib(1)),
    }
}

fn resources() -> ResourceSnapshot {
    ResourceSnapshot::from_observations(
        "2026-10-21T00:00:00Z",
        CategoryReport::available(),
        Some(&memory(16, Some(8))),
        CategoryReport::available(),
        &[gpu(0, 4, 1)],
    )
}

fn plan(model: &ModelInput, resources: &ResourceSnapshot, options: &PlanOptions) -> PlanEnvelope {
    let plan = build_plan(model, resources, options).expect("valid options build a plan");
    PlanEnvelope::new("0.1.0-test", "2026-10-21T00:00:01Z", plan)
}

fn warning_codes(envelope: &PlanEnvelope) -> Vec<PlanWarningCode> {
    envelope
        .plan
        .warnings
        .iter()
        .map(|warning| warning.code)
        .collect()
}

#[test]
fn builds_qualified_cpu_and_device_scenarios() {
    let envelope = plan(&model(), &resources(), &PlanOptions::product_defaults());
    let plan = &envelope.plan;
    assert_eq!(plan.model.parse_status, ParseStatus::Parsed);
    assert_eq!(plan.selected_device_index, Some(0));
    assert_eq!(
        plan.components.model_weights.method,
        WeightMethod::GgufFileSizeProxy
    );
    assert_eq!(
        plan.components.model_weights.estimate.class(),
        EvidenceClass::Estimated
    );
    assert_eq!(
        plan.components.model_weights.basis.class(),
        EvidenceClass::Observed
    );
    assert_eq!(plan.components.kv_cache.estimate.value(), Some(&mib(512)));
    assert_eq!(plan.scenarios.len(), 2);
    assert_eq!(plan.scenarios[0].id, ScenarioId::CpuWeightsAndKv);
    assert_eq!(plan.scenarios[1].id, ScenarioId::DeviceWeightsCpuKv);
    assert_eq!(
        plan.scenarios[0].outcome,
        ScenarioOutcome::EstimatedComponentsWithinBudget
    );
    assert_eq!(
        plan.scenarios[1].outcome,
        ScenarioOutcome::EstimatedComponentsWithinBudget
    );
    assert_eq!(plan.selected_device_index, Some(0));
    assert_eq!(
        plan.context_estimate.method.as_str(),
        "host_budget_after_weight_proxy"
    );
    assert!(plan.context_estimate.estimate.value().is_some());
    assert!(plan.context_estimate.note.is_some());
}

#[test]
fn reports_exceeded_budgets_with_shortfalls() {
    let mut model = model();
    model.file_size = Some(gib(7));
    let envelope = plan(&model, &resources(), &PlanOptions::product_defaults());
    let plan = &envelope.plan;
    assert_eq!(
        plan.scenarios[0].outcome,
        ScenarioOutcome::EstimatedComponentsExceedBudget
    );
    assert_eq!(
        plan.scenarios[1].outcome,
        ScenarioOutcome::EstimatedComponentsExceedBudget
    );
    assert!(plan.scenarios[0].host.as_ref().unwrap().shortfall.is_some());
    assert!(
        plan.scenarios[1]
            .device
            .as_ref()
            .unwrap()
            .shortfall
            .is_some()
    );
    assert!(
        plan.scenarios[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("exceed"))
    );
    assert!(!plan.scenarios[0].caveats.is_empty());
}

#[test]
fn missing_host_available_memory_is_insufficient_not_zero() {
    let resources = ResourceSnapshot::from_observations(
        "2026-10-21T00:00:00Z",
        CategoryReport::partial("MemAvailable is missing"),
        Some(&memory(16, None)),
        CategoryReport::available(),
        &[gpu(0, 4, 1)],
    );
    let envelope = plan(&model(), &resources, &PlanOptions::product_defaults());
    let plan = &envelope.plan;
    assert_eq!(plan.scenarios[0].outcome, ScenarioOutcome::InsufficientData);
    assert_eq!(plan.scenarios[1].outcome, ScenarioOutcome::InsufficientData);
    assert!(
        plan.unknowns
            .iter()
            .any(|unknown| unknown.code == PlanUnknownCode::HostAvailableMemory)
    );
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::HostObservationIncomplete));
    assert!(
        plan.scenarios[0]
            .host
            .as_ref()
            .unwrap()
            .within_budget
            .is_none()
    );
}

#[test]
fn no_device_makes_the_device_scenario_insufficient() {
    let resources = ResourceSnapshot::from_observations(
        "2026-10-21T00:00:00Z",
        CategoryReport::available(),
        Some(&memory(16, Some(8))),
        CategoryReport::unavailable("nvidia-smi was not found on PATH"),
        &[],
    );
    let envelope = plan(&model(), &resources, &PlanOptions::product_defaults());
    let plan = &envelope.plan;
    assert_eq!(plan.selected_device_index, None);
    assert_eq!(
        plan.scenarios[0].outcome,
        ScenarioOutcome::EstimatedComponentsWithinBudget
    );
    assert_eq!(plan.scenarios[1].outcome, ScenarioOutcome::InsufficientData);
    assert!(plan.scenarios[1].device.is_none());
    assert!(
        plan.unknowns
            .iter()
            .any(|unknown| unknown.code == PlanUnknownCode::DeviceFreeMemory)
    );
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::DeviceObservationIncomplete));
}

#[test]
fn missing_kv_metadata_produces_a_partial_plan() {
    let mut model = model();
    model.architecture_fields.block_count = None;
    model
        .architecture_fields
        .present_keys
        .retain(|key| *key != ArchitectureKey::BlockCount);
    let envelope = plan(&model, &resources(), &PlanOptions::product_defaults());
    let plan = &envelope.plan;
    assert!(plan.components.kv_cache.estimate.value().is_none());
    assert!(
        plan.unknowns
            .iter()
            .any(|unknown| unknown.code == PlanUnknownCode::KvCache)
    );
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::KvCacheUnavailable));
    assert_eq!(plan.scenarios[0].outcome, ScenarioOutcome::InsufficientData);
    assert!(plan.components.model_weights.estimate.value().is_some());
}

#[test]
fn unsupported_attention_layout_is_explicit_in_plan_output() {
    let mut model = model();
    model.architecture_fields.attention_layout_keys = vec![
        AttentionLayoutKey::SlidingWindow,
        AttentionLayoutKey::KeyLengthSwa,
    ];
    let envelope = plan(&model, &resources(), &PlanOptions::product_defaults());
    assert!(envelope.plan.components.kv_cache.estimate.value().is_none());
    let codes = warning_codes(&envelope);
    assert!(codes.contains(&PlanWarningCode::UnsupportedAttentionLayout));
    assert!(codes.contains(&PlanWarningCode::KvCacheUnavailable));
    let json = serde_json::to_string(&envelope).unwrap();
    assert!(json.contains("unsupported_attention_layout"));
}

#[test]
fn malformed_model_keeps_the_observed_basis_but_has_unknown_weights() {
    let mut model = model();
    model.parse_status = ParseStatus::Malformed;
    model.failure = Some(Failure {
        kind: FailureKind::InvalidMagic,
        message: "magic bytes were not GGUF".to_string(),
        offset: Some(0),
    });
    model.metadata_state = None;
    model.architecture = None;
    model.architecture_fields = ArchitectureFields::default();
    let envelope = plan(&model, &resources(), &PlanOptions::product_defaults());
    let plan = &envelope.plan;
    assert_eq!(
        plan.components.model_weights.method,
        WeightMethod::Unavailable
    );
    assert!(plan.components.model_weights.estimate.value().is_none());
    assert_eq!(plan.components.model_weights.basis.value(), Some(&gib(2)));
    assert!(plan.components.kv_cache.estimate.value().is_none());
    assert_eq!(plan.scenarios[0].outcome, ScenarioOutcome::InsufficientData);
    assert_eq!(plan.scenarios[1].outcome, ScenarioOutcome::InsufficientData);
    assert!(plan.model.failure.is_some());
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::ModelNotParsed));
}

#[test]
fn sharded_filename_makes_total_weights_unknown() {
    let mut model = model();
    model.filename_hints.shard = Some(ShardHint {
        number: 1,
        total: 3,
    });
    let envelope = plan(&model, &resources(), &PlanOptions::product_defaults());
    let plan = &envelope.plan;
    assert!(plan.components.model_weights.estimate.value().is_none());
    assert!(plan.components.kv_cache.estimate.value().is_some());
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::ShardedFileSelected));
    assert_eq!(plan.scenarios[0].outcome, ScenarioOutcome::InsufficientData);
}

#[test]
fn explicit_device_selection_and_missing_index_are_deterministic() {
    let resources = ResourceSnapshot::from_observations(
        "2026-10-21T00:00:00Z",
        CategoryReport::available(),
        Some(&memory(16, Some(8))),
        CategoryReport::available(),
        &[gpu(1, 8, 1), gpu(0, 4, 1)],
    );
    let mut options = PlanOptions::product_defaults();
    options.device_selection = DeviceSelection::Explicit { index: 1 };
    let envelope = plan(&model(), &resources, &options);
    assert_eq!(envelope.plan.selected_device_index, Some(1));
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::MultipleDevicesObserved));

    options.device_selection = DeviceSelection::Explicit { index: 9 };
    let envelope = plan(&model(), &resources, &options);
    assert_eq!(envelope.plan.selected_device_index, None);
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::ExplicitDeviceMissing));
    assert_eq!(
        envelope.plan.scenarios[1].outcome,
        ScenarioOutcome::InsufficientData
    );
}

#[test]
fn reserves_at_or_above_availability_never_produce_negative_budgets() {
    let resources = ResourceSnapshot::from_observations(
        "2026-10-21T00:00:00Z",
        CategoryReport::available(),
        Some(&memory(16, Some(1))),
        CategoryReport::available(),
        &[gpu(0, 4, 1)],
    );
    let envelope = plan(&model(), &resources, &PlanOptions::product_defaults());
    let plan = &envelope.plan;
    let cpu_host = plan.scenarios[0].host.as_ref().unwrap();
    assert!(!cpu_host.budget.usable);
    assert_eq!(cpu_host.budget.effective.value(), Some(&Bytes::ZERO));
    assert_eq!(
        plan.scenarios[0].outcome,
        ScenarioOutcome::EstimatedComponentsExceedBudget
    );
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::HostBudgetExhaustedByReserve));
}

#[test]
fn context_upper_bound_is_capped_and_is_not_the_declared_context() {
    let resources = ResourceSnapshot::from_observations(
        "2026-10-21T00:00:00Z",
        CategoryReport::available(),
        Some(&memory(2_097_152, Some(1_048_576))),
        CategoryReport::available(),
        &[gpu(0, 4, 1)],
    );
    let envelope = plan(&model(), &resources, &PlanOptions::product_defaults());
    let context = &envelope.plan.context_estimate;
    assert_eq!(context.estimate.value(), Some(&MAX_CONTEXT_TOKENS));
    assert!(context.capped_at_limit);
    assert_eq!(context.method.as_str(), "host_budget_after_weight_proxy");
    let codes = warning_codes(&envelope);
    assert!(codes.contains(&PlanWarningCode::ContextEstimateCapped));
    assert!(codes.contains(&PlanWarningCode::ContextEstimateExceedsDeclared));
    let json = serde_json::to_string(&envelope).unwrap();
    assert!(json.contains("budget upper bound"));
}

#[test]
fn requested_context_beyond_declared_metadata_is_warned() {
    let mut options = PlanOptions::product_defaults();
    options.context_tokens = Evidence::explicit_input(200_000);
    let envelope = plan(&model(), &resources(), &options);
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::RequestedContextExceedsDeclared));
    assert!(
        envelope
            .plan
            .warnings
            .iter()
            .any(|warning| warning.code == PlanWarningCode::RequestedContextExceedsDeclared)
    );
}

#[test]
fn explicit_f32_and_multiple_sequences_are_visible_and_affect_the_estimate() {
    let mut options = PlanOptions::product_defaults();
    options.kv_cache_type = Evidence::explicit_input(ghostram_planner::KvCacheType::F32);
    options.sequences = Evidence::explicit_input(2);
    let envelope = plan(&model(), &resources(), &options);
    let kv = &envelope.plan.components.kv_cache;
    assert_eq!(kv.estimate.value(), Some(&gib(2)));
    assert_eq!(
        envelope.plan.options.kv_cache_type.class(),
        EvidenceClass::ExplicitInput
    );
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::IndependentSequencesAssumption));
    assert!(
        envelope.plan.scenarios[0]
            .caveats
            .iter()
            .any(|caveat| caveat.contains("independent"))
    );
}

#[test]
fn runtime_overhead_is_explicitly_unknown_everywhere() {
    let envelope = plan(&model(), &resources(), &PlanOptions::product_defaults());
    assert!(envelope.plan.components.runtime_overhead.value().is_none());
    assert!(
        envelope
            .plan
            .unknowns
            .iter()
            .any(|unknown| unknown.code == PlanUnknownCode::RuntimeOverhead)
    );
    assert!(warning_codes(&envelope).contains(&PlanWarningCode::RuntimeOverheadUnmodeled));
}

#[test]
fn product_defaults_are_visible_and_not_presented_as_measurements() {
    let options = PlanOptions::product_defaults();
    assert_eq!(
        options.context_tokens.class(),
        EvidenceClass::ProductDefault
    );
    assert_eq!(options.sequences.class(), EvidenceClass::ProductDefault);
    assert_eq!(options.kv_cache_type.class(), EvidenceClass::ProductDefault);
    assert_eq!(options.ram_reserve.class(), EvidenceClass::ProductDefault);
    assert_eq!(options.vram_reserve.class(), EvidenceClass::ProductDefault);
    assert_eq!(
        options.ram_reserve.value(),
        Some(&mib(DEFAULT_RAM_RESERVE_MIB))
    );
    assert_eq!(
        options.vram_reserve.value(),
        Some(&mib(DEFAULT_VRAM_RESERVE_MIB))
    );
    let envelope = plan(&model(), &resources(), &options);
    assert_eq!(
        envelope.plan.options.ram_reserve.class(),
        EvidenceClass::ProductDefault
    );
    assert!(
        envelope
            .plan
            .options
            .ram_reserve
            .reason()
            .unwrap()
            .contains("not a universal")
    );
}

#[test]
fn option_validation_rejects_unknown_estimated_and_out_of_range_values() {
    let model = model();
    let resources = resources();

    let mut options = PlanOptions::product_defaults();
    options.context_tokens = Evidence::explicit_input(0);
    assert_eq!(
        build_plan(&model, &resources, &options).unwrap_err().kind(),
        PlanErrorKind::ContextOutOfRange
    );

    options = PlanOptions::product_defaults();
    options.sequences = Evidence::explicit_input(0);
    assert_eq!(
        build_plan(&model, &resources, &options).unwrap_err().kind(),
        PlanErrorKind::SequencesOutOfRange
    );

    options = PlanOptions::product_defaults();
    options.context_tokens = Evidence::estimated(4096, "not an input");
    assert_eq!(
        build_plan(&model, &resources, &options).unwrap_err().kind(),
        PlanErrorKind::InvalidOptionEvidence
    );

    options = PlanOptions::product_defaults();
    options.ram_reserve = Evidence::unknown("reserve not supplied");
    assert_eq!(
        build_plan(&model, &resources, &options).unwrap_err().kind(),
        PlanErrorKind::MissingOption
    );
}

#[test]
fn envelope_round_trips_and_rejects_a_foreign_schema() {
    let envelope = plan(&model(), &resources(), &PlanOptions::product_defaults());
    assert_eq!(envelope.schema_version, MODELS_PLAN_SCHEMA_VERSION);
    envelope.validate_schema().unwrap();
    let encoded = serde_json::to_string_pretty(&envelope).unwrap();
    let decoded: PlanEnvelope = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, envelope);
    assert_eq!(decoded.schema_version, "ghostram.models.plan/1");

    let mut foreign = envelope;
    foreign.schema_version = "ghostram.models.plan/2".to_string();
    assert!(foreign.validate_schema().is_err());
}

#[test]
fn json_is_compact_no_secret_or_metadata_dump_and_no_unqualified_claims() {
    let envelope = plan(&model(), &resources(), &PlanOptions::product_defaults());
    let json = serde_json::to_string(&envelope).unwrap();
    assert!(json.contains("\"schema_version\":\"ghostram.models.plan/1\""));
    assert!(json.contains("\"evidence\":\"estimated\""));
    assert!(json.contains("\"evidence\":\"product_default\""));
    assert!(json.contains("\"outcome\":\"estimated_components_within_budget\""));
    assert!(!json.contains("feed_forward_length"));
    assert!(!json.contains("present_keys"));
    assert!(!json.contains("attention_layout_keys"));
    assert!(!json.contains("model_fits"));
    assert!(!json.contains("launchable"));
    assert!(!json.contains("safe_to_run"));
    assert!(!json.contains("\"outcome\":\"supported\""));
}

#[test]
fn catalog_entry_conversion_preserves_planning_markers_and_warnings() {
    let warning = CatalogWarning {
        code: WarningCode::UnexpectedMetadataType,
        message: "llama.attention.head_count_kv had an unexpected type".to_string(),
    };
    let metadata = ModelMetadata {
        gguf_version: 3,
        tensor_count: 123,
        metadata_entry_count: 20,
        metadata_state: MetadataState::Available,
        alignment: Field::explicit(32),
        tensor_data_offset: Bytes::new(4096),
        tensor_data_bytes: Bytes::new(8192),
        general: GeneralMetadata {
            architecture: Some(Field::explicit("llama".to_string())),
            name: Some(Field::explicit("Boundary Model".to_string())),
            quantization: Some(Field::derived("Q4_K_M".to_string())),
            ..GeneralMetadata::default()
        },
        architecture_fields: architecture_fields(),
        tokenizer: Default::default(),
        tensor_types: Vec::new(),
        warnings: vec![warning.clone()],
    };
    let entry = FileCatalogEntry {
        path: "/models/boundary.gguf".to_string(),
        file_size: Some(gib(2)),
        modified_at: None,
        parse_status: ParseStatus::Parsed,
        failure: None,
        metadata: Some(metadata),
        hash: HashReport {
            algorithm: HashAlgorithm::Sha256,
            status: HashStatus::NotRequested,
            sha256: None,
            bytes_hashed: Bytes::ZERO,
            reason: None,
        },
        filename_hints: FilenameHints::default(),
    };

    let input = ModelInput::from(&entry);
    assert_eq!(input.path, "/models/boundary.gguf");
    assert_eq!(input.file_size, Some(gib(2)));
    assert_eq!(input.architecture.as_ref().unwrap().value, "llama");
    assert_eq!(
        input.architecture_fields.present_keys,
        architecture_fields().present_keys
    );
    assert_eq!(input.metadata_warnings, vec![warning]);
    let envelope = plan(&input, &resources(), &PlanOptions::product_defaults());
    assert_eq!(envelope.plan.model.tensor_count, Some(123));
    assert_eq!(
        envelope.plan.model.name.as_ref().unwrap().value,
        "Boundary Model"
    );
    assert_eq!(envelope.plan.model.metadata_warnings.len(), 1);
}
