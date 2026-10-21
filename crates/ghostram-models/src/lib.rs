//! Read-only GGUF model discovery and catalog metadata for GhostRAM.
//!
//! The metadata parser reads only GGUF headers, metadata, and tensor descriptors;
//! it never parses tensor payloads. Optional SHA-256 hashing streams the complete
//! file without interpreting it. The crate never writes model files and has no
//! cache, database, or network behavior.

mod catalog;
mod error;
mod filename;
mod gguf;
mod hash;
mod scan;

#[cfg(test)]
mod test_support;

pub use catalog::{
    ArchitectureFields, ArchitectureKey, ArrayInfo, AttentionLayoutKey, CatalogWarning, Failure,
    FailureKind, Field, FileCatalogEntry, FileType, FilenameHints, FilenameRole, GeneralMetadata,
    HashAlgorithm, HashReport, HashStatus, MODELS_SCAN_SCHEMA_VERSION, MetadataState,
    MetadataValueType, ModelMetadata, ModelsScan, ModelsScanEnvelope, ParseStatus, Provenance,
    ScanIssue, ScanIssueKind, ScanOptions, ScanState, ScanSummary, ShardHint, SkippedSummary,
    TensorTypeCount, TokenizerMetadata, WarningCode,
};
pub use error::{GgufError, GgufErrorKind, ScanError};
pub use filename::hints_from_filename;
pub use gguf::{
    DEFAULT_GGUF_ALIGNMENT, MAX_ARRAY_ELEMENTS, MAX_ARRAY_NESTING, MAX_DISTINCT_TENSOR_TYPES,
    MAX_ENCODED_STRING_BYTES, MAX_METADATA_ENTRIES, MAX_METADATA_KEY_BYTES,
    MAX_RETAINED_STRING_BYTES, MAX_TENSOR_COUNT, MAX_TENSOR_DIMENSION, MAX_TENSOR_DIMENSIONS,
    MAX_TENSOR_NAME_BYTES, MAX_TOTAL_ARRAY_ELEMENTS, MAX_TOTAL_METADATA_KEY_BYTES,
    MAX_TOTAL_TENSOR_NAME_BYTES, SUPPORTED_GGUF_VERSIONS, parse_gguf,
};
pub use hash::HASH_BUFFER_BYTES;
pub use scan::inspect_file;
pub use scan::{MAX_SCAN_DEPTH, MAX_SCAN_ENTRIES, MAX_SCAN_ISSUES, scan_directory};
