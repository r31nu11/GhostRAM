//! Versioned model-catalog records and the `ghostram.models.scan/1` envelope.
//!
//! These types are a public JSON contract. Review `docs/MODELS_SCHEMA.md` before
//! renaming fields or changing their meaning.

use serde::{Deserialize, Serialize};

use ghostram_core::error::GhostramError;
use ghostram_core::quantity::Bytes;

/// Current model-scan schema identifier.
pub const MODELS_SCAN_SCHEMA_VERSION: &str = "ghostram.models.scan/1";

/// How an interpreted value was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// The value was stored directly in GGUF metadata.
    Explicit,
    /// The value was derived from validated structural data.
    Derived,
    /// The GGUF specification default was used because the key was absent.
    Default,
}

/// A value together with its provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field<T> {
    /// Interpreted value.
    pub value: T,
    /// Where the value came from.
    pub source: Provenance,
}

impl<T> Field<T> {
    /// Creates an explicitly stored field.
    #[must_use]
    pub fn explicit(value: T) -> Self {
        Self {
            value,
            source: Provenance::Explicit,
        }
    }

    /// Creates a derived field.
    #[must_use]
    pub fn derived(value: T) -> Self {
        Self {
            value,
            source: Provenance::Derived,
        }
    }

    /// Creates a spec-default field.
    #[must_use]
    pub fn defaulted(value: T) -> Self {
        Self {
            value,
            source: Provenance::Default,
        }
    }
}

/// Whether enough required metadata was available to describe the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataState {
    /// A GGUF architecture was available.
    Available,
    /// The file was structurally valid but required descriptive metadata was
    /// absent or unusable.
    Incomplete,
}

/// GGUF metadata value types documented by the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataValueType {
    /// Unsigned 8-bit integer.
    Uint8,
    /// Signed 8-bit integer.
    Int8,
    /// Unsigned 16-bit integer.
    Uint16,
    /// Signed 16-bit integer.
    Int16,
    /// Unsigned 32-bit integer.
    Uint32,
    /// Signed 32-bit integer.
    Int32,
    /// 32-bit IEEE floating point.
    Float32,
    /// Boolean stored as one byte with value zero or one.
    Bool,
    /// UTF-8 string.
    String,
    /// Homogeneous or nested array.
    Array,
    /// Unsigned 64-bit integer.
    Uint64,
    /// Signed 64-bit integer.
    Int64,
    /// 64-bit IEEE floating point.
    Float64,
}

impl MetadataValueType {
    /// Maps the documented numeric type identifier.
    #[must_use]
    pub const fn from_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::Uint8),
            1 => Some(Self::Int8),
            2 => Some(Self::Uint16),
            3 => Some(Self::Int16),
            4 => Some(Self::Uint32),
            5 => Some(Self::Int32),
            6 => Some(Self::Float32),
            7 => Some(Self::Bool),
            8 => Some(Self::String),
            9 => Some(Self::Array),
            10 => Some(Self::Uint64),
            11 => Some(Self::Int64),
            12 => Some(Self::Float64),
            _ => None,
        }
    }

    /// Returns the fixed encoded element size, or `None` for strings and arrays.
    #[must_use]
    pub const fn fixed_size(self) -> Option<u64> {
        match self {
            Self::Uint8 | Self::Int8 | Self::Bool => Some(1),
            Self::Uint16 | Self::Int16 => Some(2),
            Self::Uint32 | Self::Int32 | Self::Float32 => Some(4),
            Self::Uint64 | Self::Int64 | Self::Float64 => Some(8),
            Self::String | Self::Array => None,
        }
    }

    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uint8 => "uint8",
            Self::Int8 => "int8",
            Self::Uint16 => "uint16",
            Self::Int16 => "int16",
            Self::Uint32 => "uint32",
            Self::Int32 => "int32",
            Self::Float32 => "float32",
            Self::Bool => "bool",
            Self::String => "string",
            Self::Array => "array",
            Self::Uint64 => "uint64",
            Self::Int64 => "int64",
            Self::Float64 => "float64",
        }
    }
}

/// Compact description of a tokenizer array without its element values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrayInfo {
    /// Encoded element type.
    pub element_type: MetadataValueType,
    /// Number of elements declared by the array.
    pub count: u64,
}

/// General descriptive model metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralMetadata {
    /// `general.architecture`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<Field<String>>,
    /// `general.name`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Field<String>>,
    /// `general.basename`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basename: Option<Field<String>>,
    /// `general.finetune`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finetune: Option<Field<String>>,
    /// `general.size_label`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_label: Option<Field<String>>,
    /// `general.quantization_version`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization_version: Option<Field<u32>>,
    /// `general.file_type` with a label only when the reviewed specification
    /// defines one for the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<Field<FileType>>,
    /// `general.parameter_count`, when explicitly stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_count: Option<Field<u64>>,
    /// Human-readable quantization hint. It is explicit when derived from a
    /// documented `general.file_type` label, otherwise derived from the most
    /// common tensor type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<Field<String>>,
}

/// Numeric GGUF file-type identifier and an optional reviewed label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileType {
    /// Numeric `general.file_type` value.
    pub code: u32,
    /// Spec label, when the reviewed specification defines this code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Architecture-specific values recognized across GGUF model families.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureFields {
    /// Context length in tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<Field<u64>>,
    /// Embedding width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_length: Option<Field<u64>>,
    /// Number of transformer blocks or layers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_count: Option<Field<u64>>,
    /// Feed-forward width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed_forward_length: Option<Field<u64>>,
    /// Number of attention heads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_head_count: Option<Field<u64>>,
    /// Number of key/value attention heads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_head_count_kv: Option<Field<u64>>,
    /// Attention key width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_key_length: Option<Field<u64>>,
    /// Attention value width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_value_length: Option<Field<u64>>,
    /// Total expert count, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expert_count: Option<Field<u64>>,
    /// Number of experts used per token, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expert_used_count: Option<Field<u64>>,
    /// RoPE dimension count, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rope_dimension_count: Option<Field<u64>>,
}

/// Tokenizer metadata summarized without token strings or merge tables.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerMetadata {
    /// `tokenizer.ggml.model`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<Field<String>>,
    /// Token vocabulary array summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<ArrayInfo>,
    /// Token score array summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scores: Option<ArrayInfo>,
    /// Token type array summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_types: Option<ArrayInfo>,
    /// Merge table summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merges: Option<ArrayInfo>,
    /// Added-token array summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_tokens: Option<ArrayInfo>,
    /// Whether `tokenizer.chat_template` was present.
    pub chat_template_present: bool,
    /// Whether `tokenizer.huggingface.json` was present.
    pub huggingface_json_present: bool,
}

/// One tensor type and how many tensors use it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TensorTypeCount {
    /// Numeric `ggml_type` identifier.
    pub type_id: u32,
    /// Reviewed name, or absent for a removed or unknown identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Number of tensor descriptors using this type.
    pub tensor_count: u64,
}

/// Stable warning categories for a structurally valid file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    /// Required `general.architecture` metadata was absent or unusable.
    MissingArchitecture,
    /// A recognized key had an unexpected metadata type.
    UnexpectedMetadataType,
    /// A recognized string was too large to retain and was omitted.
    OversizedMetadataValue,
    /// A tensor used a type identifier without a reviewed name.
    UnknownTensorType,
}

/// One non-fatal catalog warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogWarning {
    /// Stable warning category.
    pub code: WarningCode,
    /// Human-readable detail. Never contains a full metadata blob.
    pub message: String,
}

/// Extracted GGUF metadata for one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Parsed GGUF version (`2` or `3`).
    pub gguf_version: u32,
    /// Tensor descriptor count declared by the file.
    pub tensor_count: u64,
    /// Metadata entry count declared by the file.
    pub metadata_entry_count: u64,
    /// Whether required descriptive metadata was available.
    pub metadata_state: MetadataState,
    /// Alignment used to locate tensor data.
    pub alignment: Field<u32>,
    /// Byte offset of the aligned tensor-data region.
    pub tensor_data_offset: Bytes,
    /// Bytes available after the tensor-data offset. Tensor contents were not
    /// read or validated.
    pub tensor_data_bytes: Bytes,
    /// General metadata.
    pub general: GeneralMetadata,
    /// Architecture-specific numeric metadata.
    pub architecture_fields: ArchitectureFields,
    /// Tokenizer metadata counts and presence flags.
    pub tokenizer: TokenizerMetadata,
    /// Tensor types used, sorted by numeric identifier.
    pub tensor_types: Vec<TensorTypeCount>,
    /// Non-fatal warnings, in deterministic parse order.
    pub warnings: Vec<CatalogWarning>,
}

/// Result category for one candidate `.gguf` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseStatus {
    /// The structural header and metadata were parsed.
    Parsed,
    /// The file uses a version, byte order, or metadata shape outside the
    /// supported boundary.
    Unsupported,
    /// The file is truncated, inconsistent, or otherwise not a valid supported
    /// GGUF structure.
    Malformed,
    /// The file could not be read.
    Unreadable,
    /// The file changed while it was being inspected.
    Changed,
}

/// Stable machine-readable failure kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// Magic bytes were not `GGUF`.
    InvalidMagic,
    /// The version is not 2 or 3.
    UnsupportedVersion,
    /// The version indicates a byte order other than little-endian.
    UnsupportedEndianness,
    /// A metadata type code is outside the documented range.
    UnsupportedMetadataType,
    /// Alignment metadata was present in an unusable form or was not a valid
    /// nonzero multiple of 8.
    InvalidAlignment,
    /// A metadata key occurred more than once.
    DuplicateMetadataKey,
    /// A tensor name occurred more than once.
    DuplicateTensorName,
    /// A tensor descriptor was internally invalid or out of bounds.
    InvalidTensor,
    /// The declared structure extends beyond the file.
    Truncated,
    /// Other structural inconsistency.
    Malformed,
    /// A documented resource limit was exceeded.
    LimitExceeded,
    /// A filesystem read failed.
    Io,
    /// File identity, size, or modification time changed during inspection.
    Changed,
}

/// Structured failure detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    /// Stable machine-readable failure category.
    pub kind: FailureKind,
    /// Human-readable detail.
    pub message: String,
    /// Byte offset associated with the failure, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

/// Hash algorithm used by the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    /// SHA-256.
    Sha256,
}

/// Content-hash outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashStatus {
    /// No hash was requested.
    NotRequested,
    /// A complete digest was produced.
    Computed,
    /// Hashing was requested but failed.
    Failed,
    /// Hashing started but the file changed; no digest is reported.
    Changed,
}

/// Per-file content-hash report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashReport {
    /// Algorithm. The field is present even when hashing was not requested.
    pub algorithm: HashAlgorithm,
    /// Hash outcome.
    pub status: HashStatus,
    /// Lowercase hexadecimal SHA-256 digest, only for `computed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Bytes read while hashing; zero when hashing was not requested.
    pub bytes_hashed: Bytes,
    /// Failure detail for `failed` or `changed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Heuristic role suggested by a filename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilenameRole {
    /// Multimodal projector sidecar.
    Mmproj,
    /// Multi-token-prediction sidecar.
    Mtp,
    /// LoRA adapter.
    Lora,
    /// Vocabulary-only component.
    Vocab,
}

/// Shard position parsed from a filename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardHint {
    /// One-based shard number.
    pub number: u32,
    /// Total number of shards.
    pub total: u32,
}

/// Filename-only hints. They never assert model-family identity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilenameHints {
    /// Sidecar or adapter role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<FilenameRole>,
    /// Shard position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard: Option<ShardHint>,
}

/// One candidate `.gguf` file in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCatalogEntry {
    /// Absolute, non-canonicalized path under the selected root.
    pub path: String,
    /// File size observed before inspection, when filesystem metadata was
    /// available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<Bytes>,
    /// Modification time observed before inspection, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    /// Structural parse outcome.
    pub parse_status: ParseStatus,
    /// Failure detail for non-parsed files or changed files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
    /// Extracted metadata retained only when the structural parse succeeded and
    /// no file change was observed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ModelMetadata>,
    /// Hash outcome.
    pub hash: HashReport,
    /// Filename-only heuristics.
    pub filename_hints: FilenameHints,
}

/// Overall scan state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanState {
    /// The selected tree was traversed and every candidate parsed.
    Complete,
    /// Valid results are present, but some candidates or entries were omitted or
    /// failed.
    Partial,
    /// The scan completed but found no candidate `.gguf` files.
    Empty,
}

/// Scan options. The same value is accepted by the scanner and recorded in the
/// contract.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOptions {
    /// Whether subdirectories were traversed.
    pub recursive: bool,
    /// Whether SHA-256 was requested.
    pub hash: bool,
}

/// Aggregate counts for the scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanSummary {
    /// Candidate `.gguf` files inspected.
    pub candidates: u64,
    /// Candidates with status `parsed`.
    pub parsed: u64,
    /// Parsed candidates with at least one warning.
    pub parsed_with_warnings: u64,
    /// Parsed candidates with incomplete descriptive metadata.
    pub incomplete_metadata: u64,
    /// Candidates with status `unsupported`.
    pub unsupported: u64,
    /// Candidates with status `malformed`.
    pub malformed: u64,
    /// Candidates with status `unreadable`.
    pub unreadable: u64,
    /// Candidates that changed during inspection.
    pub changed: u64,
    /// Files with a complete SHA-256 digest.
    pub hash_computed: u64,
    /// Files where hashing failed.
    pub hash_failed: u64,
    /// Files where hashing was discarded because the file changed.
    pub hash_changed: u64,
    /// Total bytes read for SHA-256.
    pub bytes_hashed: Bytes,
}

/// Entries intentionally not cataloged, grouped by reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedSummary {
    /// Symbolic links (files or directories). Never followed.
    pub symlinks: u64,
    /// Non-directory, non-symlink entries whose names end in `.gguf`.
    pub non_regular_files: u64,
    /// Regular files whose names do not end in `.gguf`.
    pub non_gguf_files: u64,
    /// Candidate paths that are not valid UTF-8 and cannot be represented safely.
    pub non_utf8_paths: u64,
    /// Directories not descended into because the depth limit was reached.
    pub max_depth_directories: u64,
    /// Whether the total directory-entry limit stopped traversal.
    pub entry_limit_reached: bool,
    /// Directories that could not be read.
    pub unreadable_directories: u64,
    /// Directory entries whose type or metadata could not be read.
    pub unreadable_entries: u64,
    /// Whether more issue details existed than the bounded issue list retained.
    pub issues_truncated: bool,
}

/// Stable traversal issue categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanIssueKind {
    /// A directory could not be read.
    UnreadableDirectory,
    /// A directory entry's type or metadata could not be read.
    UnreadableEntry,
    /// The total entry limit stopped further traversal.
    EntryLimitReached,
    /// A directory was not traversed because of the depth limit.
    DepthLimitReached,
}

/// One bounded traversal issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanIssue {
    /// Affected path, when available.
    pub path: String,
    /// Issue category.
    pub kind: ScanIssueKind,
    /// Human-readable detail.
    pub message: String,
}

/// Scan payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelsScan {
    /// Overall scan state.
    pub state: ScanState,
    /// Explicitly selected roots, as absolute non-canonicalized paths.
    pub roots: Vec<String>,
    /// Recorded scan options.
    pub options: ScanOptions,
    /// Aggregate counts.
    pub summary: ScanSummary,
    /// Entries not cataloged.
    pub skipped: SkippedSummary,
    /// Bounded traversal issue details.
    pub issues: Vec<ScanIssue>,
    /// Candidate files, sorted lexicographically by path.
    pub files: Vec<FileCatalogEntry>,
}

/// Versioned envelope around a model scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelsScanEnvelope {
    /// Schema identifier for the scan shape.
    pub schema_version: String,
    /// GhostRAM application version that generated the scan.
    pub application_version: String,
    /// UTC RFC 3339 generation time.
    pub generated_at: String,
    /// Scan payload.
    pub scan: ModelsScan,
}

impl ModelsScanEnvelope {
    /// Creates an envelope with the current schema version.
    #[must_use]
    pub fn new(
        application_version: impl Into<String>,
        generated_at: impl Into<String>,
        scan: ModelsScan,
    ) -> Self {
        Self {
            schema_version: MODELS_SCAN_SCHEMA_VERSION.to_string(),
            application_version: application_version.into(),
            generated_at: generated_at.into(),
            scan,
        }
    }

    /// Verifies that this envelope uses the schema version supported by this
    /// build.
    ///
    /// # Errors
    ///
    /// Returns [`GhostramError::InvalidInput`] when the schema identifier differs.
    pub fn validate_schema(&self) -> Result<(), GhostramError> {
        if self.schema_version == MODELS_SCAN_SCHEMA_VERSION {
            Ok(())
        } else {
            Err(GhostramError::InvalidInput {
                message: format!(
                    "unsupported model-scan schema {:?}; expected {:?}",
                    self.schema_version, MODELS_SCAN_SCHEMA_VERSION
                ),
            })
        }
    }
}
