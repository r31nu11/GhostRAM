//! Human-readable rendering for `ghostram doctor` and `ghostram models scan`.

use std::fmt::Write;

use ghostram_core::diagnostic::{
    DiagnosticEnvelope, DiagnosticState, MemoryInfo, StorageInfo, SystemInfo, ToolInfo,
};
use ghostram_core::quantity::Bytes;
use ghostram_models::{
    ArrayInfo, FailureKind, Field, FileCatalogEntry, FilenameRole, HashStatus, MetadataState,
    MetadataValueType, ModelMetadata, ModelsScanEnvelope, ParseStatus, Provenance, ScanState,
    WarningCode,
};

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

/// Renders a complete human-readable model-scan report.
#[must_use]
pub fn render_models_scan(envelope: &ModelsScanEnvelope) -> String {
    let mut output = String::new();
    let scan = &envelope.scan;

    let _ = writeln!(output, "GhostRAM model scan");
    let _ = writeln!(output, "Application: {}", envelope.application_version);
    let _ = writeln!(output, "Schema: {}", envelope.schema_version);
    let _ = writeln!(output, "Generated: {}", envelope.generated_at);
    for root in &scan.roots {
        let _ = writeln!(output, "Root: {}", display_text(root, 500));
    }
    let _ = writeln!(
        output,
        "Options: recursive={}, sha256={}",
        yes_no(scan.options.recursive),
        yes_no(scan.options.hash)
    );
    let _ = writeln!(output, "State: {}", scan_state_name(scan.state));
    output.push('\n');

    let summary = &scan.summary;
    let _ = writeln!(output, "Summary");
    let _ = writeln!(output, "  candidates: {}", summary.candidates);
    let _ = writeln!(
        output,
        "  parsed: {} ({} with warnings, {} incomplete metadata)",
        summary.parsed, summary.parsed_with_warnings, summary.incomplete_metadata
    );
    let _ = writeln!(output, "  unsupported: {}", summary.unsupported);
    let _ = writeln!(output, "  malformed: {}", summary.malformed);
    let _ = writeln!(output, "  unreadable: {}", summary.unreadable);
    let _ = writeln!(output, "  changed: {}", summary.changed);
    if scan.options.hash {
        let _ = writeln!(
            output,
            "  sha256: {} computed, {} failed, {} changed ({} read)",
            summary.hash_computed, summary.hash_failed, summary.hash_changed, summary.bytes_hashed
        );
    } else {
        let _ = writeln!(output, "  sha256: not requested");
    }
    output.push('\n');

    let skipped = &scan.skipped;
    let _ = writeln!(output, "Skipped");
    let _ = writeln!(output, "  symlinks: {}", skipped.symlinks);
    let _ = writeln!(
        output,
        "  non-regular .gguf candidates: {}",
        skipped.non_regular_files
    );
    let _ = writeln!(
        output,
        "  non-GGUF regular files: {}",
        skipped.non_gguf_files
    );
    let _ = writeln!(
        output,
        "  non-UTF-8 candidate paths: {}",
        skipped.non_utf8_paths
    );
    let _ = writeln!(
        output,
        "  depth-limited directories: {}",
        skipped.max_depth_directories
    );
    let _ = writeln!(
        output,
        "  entry limit reached: {}",
        yes_no(skipped.entry_limit_reached)
    );
    let _ = writeln!(
        output,
        "  unreadable directories: {}",
        skipped.unreadable_directories
    );
    let _ = writeln!(
        output,
        "  unreadable entries: {}",
        skipped.unreadable_entries
    );
    let _ = writeln!(
        output,
        "  issue list truncated: {}",
        yes_no(skipped.issues_truncated)
    );

    if !scan.issues.is_empty() {
        output.push('\n');
        let _ = writeln!(output, "Traversal issues");
        for issue in &scan.issues {
            let _ = writeln!(
                output,
                "  {}: {}: {}",
                issue_kind_name(issue.kind),
                display_text(&issue.path, 500),
                display_text(&issue.message, 300)
            );
        }
    }

    output.push('\n');
    let _ = writeln!(output, "Files");
    if scan.files.is_empty() {
        let _ = writeln!(output, "  none");
    } else {
        for file in &scan.files {
            write_catalog_file(&mut output, file);
        }
    }

    output
}

fn write_catalog_file(output: &mut String, file: &FileCatalogEntry) {
    let _ = writeln!(output, "  {}", display_text(&file.path, 500));
    let _ = writeln!(
        output,
        "    status: {}",
        parse_status_name(file.parse_status)
    );
    let size = file
        .file_size
        .map_or_else(|| "unknown".to_string(), |size| size.to_string());
    let modified = file.modified_at.as_deref().unwrap_or("unknown");
    let _ = writeln!(output, "    size: {size}; modified: {modified}");

    if let Some(failure) = &file.failure {
        let offset = failure
            .offset
            .map_or_else(String::new, |offset| format!(" at byte {offset}"));
        let _ = writeln!(
            output,
            "    error [{}]{offset}: {}",
            failure_kind_name(failure.kind),
            display_text(&failure.message, 400)
        );
    }

    write_filename_hints(output, file);
    if let Some(metadata) = &file.metadata {
        write_model_metadata(output, metadata);
    }
    write_hash(output, file);
}

fn write_filename_hints(output: &mut String, file: &FileCatalogEntry) {
    let mut hints = Vec::new();
    if let Some(role) = file.filename_hints.role {
        hints.push(format!("role={}", filename_role_name(role)));
    }
    if let Some(shard) = file.filename_hints.shard {
        hints.push(format!("shard={}/{}", shard.number, shard.total));
    }
    if !hints.is_empty() {
        let _ = writeln!(output, "    filename hints: {}", hints.join(", "));
    }
}

fn write_model_metadata(output: &mut String, metadata: &ModelMetadata) {
    let _ = writeln!(
        output,
        "    gguf: version {}; metadata entries {}; tensor descriptors {}",
        metadata.gguf_version, metadata.metadata_entry_count, metadata.tensor_count
    );
    let _ = writeln!(
        output,
        "    metadata: {}",
        metadata_state_name(metadata.metadata_state)
    );
    let _ = writeln!(
        output,
        "    alignment: {} bytes ({})",
        metadata.alignment.value,
        provenance_name(metadata.alignment.source)
    );
    let _ = writeln!(
        output,
        "    tensor data: offset {}; {} available (payload not read)",
        metadata.tensor_data_offset, metadata.tensor_data_bytes
    );

    write_optional_string(output, "architecture", &metadata.general.architecture);
    write_optional_string(output, "name", &metadata.general.name);
    write_optional_string(output, "basename", &metadata.general.basename);
    write_optional_string(output, "finetune", &metadata.general.finetune);
    write_optional_string(output, "size label", &metadata.general.size_label);
    if let Some(file_type) = &metadata.general.file_type {
        let label = file_type.value.label.as_deref().unwrap_or("unknown");
        let _ = writeln!(
            output,
            "    file type: {} ({label}; {})",
            file_type.value.code,
            provenance_name(file_type.source)
        );
    }
    if let Some(quantization) = &metadata.general.quantization {
        let _ = writeln!(
            output,
            "    quantization: {} ({})",
            display_text(&quantization.value, 120),
            provenance_name(quantization.source)
        );
    }
    if let Some(parameter_count) = &metadata.general.parameter_count {
        let _ = writeln!(
            output,
            "    parameter count: {} ({})",
            parameter_count.value,
            provenance_name(parameter_count.source)
        );
    }
    if let Some(quantization_version) = &metadata.general.quantization_version {
        let _ = writeln!(
            output,
            "    quantization version: {} ({})",
            quantization_version.value,
            provenance_name(quantization_version.source)
        );
    }

    let fields = &metadata.architecture_fields;
    write_optional_count(output, "context length", &fields.context_length, "tokens");
    write_optional_count(output, "embedding length", &fields.embedding_length, "");
    write_optional_count(output, "layers", &fields.block_count, "");
    write_optional_count(
        output,
        "feed-forward length",
        &fields.feed_forward_length,
        "",
    );
    write_optional_count(output, "attention heads", &fields.attention_head_count, "");
    write_optional_count(
        output,
        "attention KV heads",
        &fields.attention_head_count_kv,
        "",
    );
    write_optional_count(
        output,
        "attention key length",
        &fields.attention_key_length,
        "",
    );
    write_optional_count(
        output,
        "attention value length",
        &fields.attention_value_length,
        "",
    );
    write_optional_count(output, "experts", &fields.expert_count, "");
    write_optional_count(output, "experts per token", &fields.expert_used_count, "");
    write_optional_count(output, "RoPE dimensions", &fields.rope_dimension_count, "");

    let tokenizer = &metadata.tokenizer;
    write_optional_string(output, "tokenizer model", &tokenizer.model);
    if let Some(tokens) = &tokenizer.tokens {
        write_array_info(output, "tokens", tokens);
    }
    if let Some(scores) = &tokenizer.scores {
        write_array_info(output, "token scores", scores);
    }
    if let Some(token_types) = &tokenizer.token_types {
        write_array_info(output, "token types", token_types);
    }
    if let Some(merges) = &tokenizer.merges {
        write_array_info(output, "merges", merges);
    }
    if let Some(added_tokens) = &tokenizer.added_tokens {
        write_array_info(output, "added tokens", added_tokens);
    }
    let _ = writeln!(
        output,
        "    chat template present: {}",
        yes_no(tokenizer.chat_template_present)
    );
    let _ = writeln!(
        output,
        "    Hugging Face tokenizer JSON present: {}",
        yes_no(tokenizer.huggingface_json_present)
    );

    if !metadata.tensor_types.is_empty() {
        let mut types = Vec::new();
        for tensor_type in &metadata.tensor_types {
            let name = tensor_type.name.as_deref().unwrap_or("unknown");
            types.push(format!(
                "{name} (type {}, {})",
                tensor_type.type_id, tensor_type.tensor_count
            ));
        }
        let _ = writeln!(output, "    tensor types: {}", types.join(", "));
    }

    const HUMAN_WARNING_LIMIT: usize = 20;
    for warning in metadata.warnings.iter().take(HUMAN_WARNING_LIMIT) {
        let _ = writeln!(
            output,
            "    warning [{}]: {}",
            warning_code_name(warning.code),
            display_text(&warning.message, 400)
        );
    }
    if metadata.warnings.len() > HUMAN_WARNING_LIMIT {
        let omitted = metadata.warnings.len() - HUMAN_WARNING_LIMIT;
        let _ = writeln!(
            output,
            "    warning: {omitted} additional warnings omitted; use --json for all"
        );
    }
}

fn write_optional_string(output: &mut String, label: &str, field: &Option<Field<String>>) {
    if let Some(field) = field {
        let _ = writeln!(
            output,
            "    {label}: {} ({})",
            display_text(&field.value, 200),
            provenance_name(field.source)
        );
    }
}

fn write_optional_count(output: &mut String, label: &str, field: &Option<Field<u64>>, unit: &str) {
    if let Some(field) = field {
        let suffix = if unit.is_empty() {
            String::new()
        } else {
            format!(" {unit}")
        };
        let _ = writeln!(
            output,
            "    {label}: {}{suffix} ({})",
            field.value,
            provenance_name(field.source)
        );
    }
}

fn write_array_info(output: &mut String, label: &str, info: &ArrayInfo) {
    let _ = writeln!(
        output,
        "    {label}: {} elements of {}",
        info.count,
        value_type_name(info.element_type)
    );
}

fn write_hash(output: &mut String, file: &FileCatalogEntry) {
    let hash = &file.hash;
    match hash.status {
        HashStatus::NotRequested => {
            let _ = writeln!(output, "    sha256: not requested");
        }
        HashStatus::Computed => {
            let _ = writeln!(
                output,
                "    sha256: {} ({} read)",
                hash.sha256.as_deref().unwrap_or("missing"),
                hash.bytes_hashed
            );
        }
        HashStatus::Failed => {
            let _ = writeln!(
                output,
                "    sha256: failed after {} read: {}",
                hash.bytes_hashed,
                display_text(hash.reason.as_deref().unwrap_or("unknown error"), 300)
            );
        }
        HashStatus::Changed => {
            let _ = writeln!(
                output,
                "    sha256: changed; no digest retained ({} read)",
                hash.bytes_hashed
            );
        }
    }
}

fn scan_state_name(state: ScanState) -> &'static str {
    match state {
        ScanState::Complete => "complete",
        ScanState::Partial => "partial",
        ScanState::Empty => "empty",
    }
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

fn metadata_state_name(state: MetadataState) -> &'static str {
    match state {
        MetadataState::Available => "available",
        MetadataState::Incomplete => "incomplete",
    }
}

fn provenance_name(provenance: Provenance) -> &'static str {
    match provenance {
        Provenance::Explicit => "explicit",
        Provenance::Derived => "derived",
        Provenance::Default => "default",
    }
}

fn filename_role_name(role: FilenameRole) -> &'static str {
    match role {
        FilenameRole::Mmproj => "mmproj",
        FilenameRole::Mtp => "mtp",
        FilenameRole::Lora => "lora",
        FilenameRole::Vocab => "vocab",
    }
}

fn warning_code_name(code: WarningCode) -> &'static str {
    match code {
        WarningCode::MissingArchitecture => "missing_architecture",
        WarningCode::UnexpectedMetadataType => "unexpected_metadata_type",
        WarningCode::OversizedMetadataValue => "oversized_metadata_value",
        WarningCode::UnknownTensorType => "unknown_tensor_type",
    }
}

fn failure_kind_name(kind: FailureKind) -> &'static str {
    match kind {
        FailureKind::InvalidMagic => "invalid_magic",
        FailureKind::UnsupportedVersion => "unsupported_version",
        FailureKind::UnsupportedEndianness => "unsupported_endianness",
        FailureKind::UnsupportedMetadataType => "unsupported_metadata_type",
        FailureKind::InvalidAlignment => "invalid_alignment",
        FailureKind::DuplicateMetadataKey => "duplicate_metadata_key",
        FailureKind::DuplicateTensorName => "duplicate_tensor_name",
        FailureKind::InvalidTensor => "invalid_tensor",
        FailureKind::Truncated => "truncated",
        FailureKind::Malformed => "malformed",
        FailureKind::LimitExceeded => "limit_exceeded",
        FailureKind::Io => "io",
        FailureKind::Changed => "changed",
    }
}

fn issue_kind_name(kind: ghostram_models::ScanIssueKind) -> &'static str {
    match kind {
        ghostram_models::ScanIssueKind::UnreadableDirectory => "unreadable_directory",
        ghostram_models::ScanIssueKind::UnreadableEntry => "unreadable_entry",
        ghostram_models::ScanIssueKind::EntryLimitReached => "entry_limit_reached",
        ghostram_models::ScanIssueKind::DepthLimitReached => "depth_limit_reached",
    }
}

fn value_type_name(value_type: MetadataValueType) -> &'static str {
    value_type.as_str()
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn display_text(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (count, character) in value.chars().enumerate() {
        if count >= max_chars {
            output.push_str("...");
            break;
        }
        if character.is_control() || is_bidi_control(character) {
            for escaped in character.escape_unicode() {
                output.push(escaped);
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
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
    use super::{display_text, render_doctor, render_models_scan};
    use ghostram_core::diagnostic::{
        CategoryReport, CategoryStates, DiagnosticEnvelope, DiagnosticReport, DiagnosticState,
        SystemInfo,
    };
    use ghostram_core::quantity::Bytes;
    use ghostram_models::{
        ModelsScan, ModelsScanEnvelope, ScanOptions, ScanState, ScanSummary, SkippedSummary,
    };

    fn empty_model_envelope() -> ModelsScanEnvelope {
        ModelsScanEnvelope::new(
            "0.1.0-test",
            "2026-10-21T00:00:00Z",
            ModelsScan {
                state: ScanState::Empty,
                roots: vec!["/models".to_string()],
                options: ScanOptions::default(),
                summary: ScanSummary {
                    candidates: 0,
                    parsed: 0,
                    parsed_with_warnings: 0,
                    incomplete_metadata: 0,
                    unsupported: 0,
                    malformed: 0,
                    unreadable: 0,
                    changed: 0,
                    hash_computed: 0,
                    hash_failed: 0,
                    hash_changed: 0,
                    bytes_hashed: Bytes::ZERO,
                },
                skipped: SkippedSummary {
                    symlinks: 0,
                    non_regular_files: 0,
                    non_gguf_files: 0,
                    non_utf8_paths: 0,
                    max_depth_directories: 0,
                    entry_limit_reached: false,
                    unreadable_directories: 0,
                    unreadable_entries: 0,
                    issues_truncated: false,
                },
                issues: Vec::new(),
                files: Vec::new(),
            },
        )
    }

    #[test]
    fn renders_empty_model_scan_contract_and_root() {
        let rendered = render_models_scan(&empty_model_envelope());
        assert!(rendered.contains("GhostRAM model scan"));
        assert!(rendered.contains("Schema: ghostram.models.scan/1"));
        assert!(rendered.contains("Root: /models"));
        assert!(rendered.contains("State: empty"));
        assert!(rendered.contains("sha256: not requested"));
        assert!(rendered.contains("Files\n  none"));
    }

    #[test]
    fn escapes_terminal_and_bidi_controls_in_human_values() {
        let rendered = display_text("model\x1b[31m\u{202e}name", 100);
        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\u{202e}'));
        assert!(rendered.contains("\\u{1b}"));
        assert!(rendered.contains("\\u{202e}"));
    }

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
