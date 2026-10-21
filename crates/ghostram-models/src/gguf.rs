//! Bounded, read-only GGUF header and metadata parsing.
//!
//! The parser never reads tensor payloads and never allocates a buffer from an
//! untrusted element count. Unknown metadata values are skipped only when their
//! encoded type gives a safe way to determine their length.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufReader, ErrorKind, Read, Seek, SeekFrom};

use crate::catalog::{
    ArchitectureFields, ArrayInfo, CatalogWarning, Field, FileType, GeneralMetadata, MetadataState,
    MetadataValueType, ModelMetadata, TensorTypeCount, TokenizerMetadata, WarningCode,
};
use crate::error::{GgufError, GgufErrorKind};
use ghostram_core::quantity::Bytes;

/// GGUF versions accepted by this parser.
pub const SUPPORTED_GGUF_VERSIONS: [u32; 2] = [2, 3];

/// Alignment required by the GGUF specification when `general.alignment` is
/// absent.
pub const DEFAULT_GGUF_ALIGNMENT: u32 = 32;

/// Maximum metadata key/value pairs in one file.
pub const MAX_METADATA_ENTRIES: u64 = 65_536;
/// Maximum encoded metadata-key length.
pub const MAX_METADATA_KEY_BYTES: u64 = 65_535;
/// Maximum combined encoded metadata-key bytes retained for duplicate checks.
pub const MAX_TOTAL_METADATA_KEY_BYTES: u64 = 8 * 1024 * 1024;
/// Maximum retained size for a recognized string value.
pub const MAX_RETAINED_STRING_BYTES: u64 = 16 * 1024;
/// Maximum encoded length for any string, including skipped strings.
pub const MAX_ENCODED_STRING_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum element count of one metadata array.
pub const MAX_ARRAY_ELEMENTS: u64 = 4_000_000;
/// Maximum sum of element counts across all arrays in one file.
pub const MAX_TOTAL_ARRAY_ELEMENTS: u64 = 8_000_000;
/// Maximum nested-array depth.
pub const MAX_ARRAY_NESTING: u32 = 8;
/// Maximum tensor descriptors in one file.
pub const MAX_TENSOR_COUNT: u64 = 1_000_000;
/// Maximum encoded tensor-name length.
pub const MAX_TENSOR_NAME_BYTES: u64 = 64;
/// Maximum combined encoded tensor-name bytes.
pub const MAX_TOTAL_TENSOR_NAME_BYTES: u64 = 32 * 1024 * 1024;
/// Maximum tensor dimensions supported by current GGML.
pub const MAX_TENSOR_DIMENSIONS: u32 = 4;
/// Maximum value of one tensor dimension.
pub const MAX_TENSOR_DIMENSION: u64 = i64::MAX as u64;
/// Maximum distinct tensor type identifiers retained per file.
pub const MAX_DISTINCT_TENSOR_TYPES: usize = 256;

const IO_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct GgufLimits {
    metadata_entries: u64,
    metadata_key_bytes: u64,
    total_metadata_key_bytes: u64,
    retained_string_bytes: u64,
    encoded_string_bytes: u64,
    array_elements: u64,
    total_array_elements: u64,
    array_nesting: u32,
    tensor_count: u64,
    tensor_name_bytes: u64,
    total_tensor_name_bytes: u64,
    tensor_dimensions: u32,
    tensor_dimension: u64,
    distinct_tensor_types: usize,
}

impl Default for GgufLimits {
    fn default() -> Self {
        Self {
            metadata_entries: MAX_METADATA_ENTRIES,
            metadata_key_bytes: MAX_METADATA_KEY_BYTES,
            total_metadata_key_bytes: MAX_TOTAL_METADATA_KEY_BYTES,
            retained_string_bytes: MAX_RETAINED_STRING_BYTES,
            encoded_string_bytes: MAX_ENCODED_STRING_BYTES,
            array_elements: MAX_ARRAY_ELEMENTS,
            total_array_elements: MAX_TOTAL_ARRAY_ELEMENTS,
            array_nesting: MAX_ARRAY_NESTING,
            tensor_count: MAX_TENSOR_COUNT,
            tensor_name_bytes: MAX_TENSOR_NAME_BYTES,
            total_tensor_name_bytes: MAX_TOTAL_TENSOR_NAME_BYTES,
            tensor_dimensions: MAX_TENSOR_DIMENSIONS,
            tensor_dimension: MAX_TENSOR_DIMENSION,
            distinct_tensor_types: MAX_DISTINCT_TENSOR_TYPES,
        }
    }
}

/// Parses the GGUF header, metadata, and tensor descriptors from the start of
/// `reader`.
///
/// `file_size` must be the observed size of the same file. The parser reads no
/// tensor payload bytes. The caller is responsible for rewinding or reopening the
/// reader for any content hash.
///
/// # Errors
///
/// Returns a typed [`GgufError`] for unsupported versions, unsafe or malformed
/// structures, truncation, and resource-limit violations. I/O failures are
/// returned without panicking.
pub fn parse_gguf<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
) -> Result<ModelMetadata, GgufError> {
    parse_gguf_with_limits(reader, file_size, &GgufLimits::default())
}

pub(crate) fn parse_gguf_with_limits<R: Read + Seek>(
    reader: &mut R,
    file_size: u64,
    limits: &GgufLimits,
) -> Result<ModelMetadata, GgufError> {
    let buffered = BufReader::with_capacity(IO_BUFFER_BYTES, reader);
    let mut decoder = Decoder {
        reader: buffered,
        offset: 0,
        file_size,
        limits,
        alignment: None,
    };
    decoder.parse()
}

struct Decoder<'a, R: Read + Seek> {
    reader: BufReader<R>,
    offset: u64,
    file_size: u64,
    limits: &'a GgufLimits,
    alignment: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
struct NumericValue {
    value: u64,
    value_type: MetadataValueType,
    offset: u64,
}

#[derive(Debug, Clone, Copy)]
enum StringKey {
    Architecture,
    Name,
    Basename,
    Finetune,
    SizeLabel,
    TokenizerModel,
    ChatTemplate,
    HuggingfaceJson,
}

#[derive(Debug, Clone, Copy)]
enum ArrayKey {
    Tokens,
    Scores,
    TokenTypes,
    Merges,
    AddedTokens,
}

enum RetainedString {
    Value(String),
    Omitted,
}

impl<R: Read + Seek> Decoder<'_, R> {
    fn parse(&mut self) -> Result<ModelMetadata, GgufError> {
        if self.file_size < 24 {
            return Err(GgufError::new(
                GgufErrorKind::Truncated,
                "GGUF input is shorter than the 24-byte header",
                Some(0),
            ));
        }

        let magic = self.read_array::<4>("file magic")?;
        if &magic != b"GGUF" {
            return Err(GgufError::new(
                GgufErrorKind::InvalidMagic,
                "magic bytes are not `GGUF`",
                Some(0),
            ));
        }

        let version_bytes = self.read_array::<4>("GGUF version")?;
        let version = u32::from_le_bytes(version_bytes);
        if !SUPPORTED_GGUF_VERSIONS.contains(&version) {
            let big_endian_version = u32::from_be_bytes(version_bytes);
            if SUPPORTED_GGUF_VERSIONS.contains(&big_endian_version) {
                return Err(GgufError::new(
                    GgufErrorKind::UnsupportedEndianness,
                    format!(
                        "big-endian GGUF version {big_endian_version} is not supported; \
                         little-endian v2 and v3 are supported"
                    ),
                    Some(4),
                ));
            }
            return Err(GgufError::new(
                GgufErrorKind::UnsupportedVersion,
                format!(
                    "GGUF version {version} is not supported; supported versions are {} and {}",
                    SUPPORTED_GGUF_VERSIONS[0], SUPPORTED_GGUF_VERSIONS[1]
                ),
                Some(4),
            ));
        }

        let tensor_count = self.read_u64("tensor count")?;
        if tensor_count > self.limits.tensor_count {
            return Err(GgufError::new(
                GgufErrorKind::LimitExceeded,
                format!(
                    "tensor count {tensor_count} exceeds the limit of {}",
                    self.limits.tensor_count
                ),
                Some(self.offset.saturating_sub(8)),
            ));
        }
        let metadata_entry_count = self.read_u64("metadata entry count")?;
        if metadata_entry_count > self.limits.metadata_entries {
            return Err(GgufError::new(
                GgufErrorKind::LimitExceeded,
                format!(
                    "metadata entry count {metadata_entry_count} exceeds the limit of {}",
                    self.limits.metadata_entries
                ),
                Some(self.offset.saturating_sub(8)),
            ));
        }

        let mut keys = HashSet::new();
        let mut total_key_bytes = 0_u64;
        let mut numeric_values = HashMap::new();
        let mut general = GeneralMetadata::default();
        let mut tokenizer = TokenizerMetadata::default();
        let mut warnings = Vec::new();
        let mut total_array_elements = 0_u64;

        for _ in 0..metadata_entry_count {
            let key_offset = self.offset;
            let key = self.read_key()?;
            let key_bytes = u64::try_from(key.len()).unwrap_or(u64::MAX);
            total_key_bytes = total_key_bytes.checked_add(key_bytes).ok_or_else(|| {
                GgufError::new(
                    GgufErrorKind::LimitExceeded,
                    "total metadata key length overflow",
                    Some(key_offset),
                )
            })?;
            if total_key_bytes > self.limits.total_metadata_key_bytes {
                return Err(GgufError::new(
                    GgufErrorKind::LimitExceeded,
                    format!(
                        "total metadata key bytes exceed the limit of {}",
                        self.limits.total_metadata_key_bytes
                    ),
                    Some(key_offset),
                ));
            }
            if !keys.insert(key.clone()) {
                return Err(GgufError::new(
                    GgufErrorKind::DuplicateMetadataKey,
                    format!("duplicate metadata key {}", quoted_key(&key)),
                    Some(key_offset),
                ));
            }

            let type_offset = self.offset;
            let value_type_id = self.read_u32("metadata value type")?;
            let value_type = MetadataValueType::from_id(value_type_id).ok_or_else(|| {
                GgufError::new(
                    GgufErrorKind::UnsupportedMetadataType,
                    format!("metadata value type code {value_type_id} is not documented"),
                    Some(type_offset),
                )
            })?;
            let string_key = string_key(&key);
            let array_key = array_key(&key);
            let numeric_key = is_numeric_key(&key);

            if key == "general.alignment" && value_type != MetadataValueType::Uint32 {
                return Err(GgufError::new(
                    GgufErrorKind::InvalidAlignment,
                    "general.alignment must be encoded as uint32",
                    Some(type_offset),
                ));
            }

            if value_type == MetadataValueType::Array {
                if let Some(string_key) = string_key {
                    push_warning(
                        &mut warnings,
                        WarningCode::UnexpectedMetadataType,
                        format!(
                            "metadata key {} was expected to be a string, not an array",
                            quoted_key(&key)
                        ),
                    );
                    if matches!(
                        string_key,
                        StringKey::ChatTemplate | StringKey::HuggingfaceJson
                    ) {
                        set_presence(&mut tokenizer, string_key);
                    }
                } else if numeric_key {
                    push_warning(
                        &mut warnings,
                        WarningCode::UnexpectedMetadataType,
                        format!(
                            "metadata key {} was expected to be an unsigned integer, not an array",
                            quoted_key(&key)
                        ),
                    );
                }
                let array = self.read_array_value(&mut total_array_elements)?;
                if let Some(array_key) = array_key {
                    if let Some(expected) = expected_array_element_type(array_key) {
                        if array.element_type != expected {
                            push_warning(
                                &mut warnings,
                                WarningCode::UnexpectedMetadataType,
                                format!(
                                    "metadata key {} was expected to contain {} elements, not {}",
                                    quoted_key(&key),
                                    expected.as_str(),
                                    array.element_type.as_str()
                                ),
                            );
                        }
                    }
                    set_array(&mut tokenizer, array_key, array);
                }
            } else {
                if let Some(array_key) = array_key {
                    push_warning(
                        &mut warnings,
                        WarningCode::UnexpectedMetadataType,
                        format!(
                            "metadata key {} was expected to be an array, not {}",
                            quoted_key(&key),
                            value_type.as_str()
                        ),
                    );
                    set_array_absent(&mut tokenizer, array_key);
                }

                if let Some(string_key) = string_key {
                    if value_type == MetadataValueType::String {
                        if matches!(
                            string_key,
                            StringKey::ChatTemplate | StringKey::HuggingfaceJson
                        ) {
                            self.skip_encoded_string("metadata presence value")?;
                            set_presence(&mut tokenizer, string_key);
                        } else {
                            match self.read_retained_string()? {
                                RetainedString::Value(value) => {
                                    set_string(&mut general, &mut tokenizer, string_key, value);
                                }
                                RetainedString::Omitted => {
                                    push_warning(
                                        &mut warnings,
                                        WarningCode::OversizedMetadataValue,
                                        format!(
                                            "metadata key {} exceeds the retained string limit of {} bytes",
                                            quoted_key(&key),
                                            self.limits.retained_string_bytes
                                        ),
                                    );
                                }
                            }
                        }
                    } else {
                        push_warning(
                            &mut warnings,
                            WarningCode::UnexpectedMetadataType,
                            format!(
                                "metadata key {} was expected to be a string, not {}",
                                quoted_key(&key),
                                value_type.as_str()
                            ),
                        );
                        if matches!(
                            string_key,
                            StringKey::ChatTemplate | StringKey::HuggingfaceJson
                        ) {
                            set_presence(&mut tokenizer, string_key);
                        }
                        self.skip_scalar_value(value_type, Some(&key))?;
                    }
                } else if numeric_key {
                    if is_unsigned_integer(value_type) {
                        let value_offset = self.offset;
                        let value = self.read_unsigned_value(value_type)?;
                        numeric_values.insert(
                            key,
                            NumericValue {
                                value,
                                value_type,
                                offset: value_offset,
                            },
                        );
                    } else {
                        push_warning(
                            &mut warnings,
                            WarningCode::UnexpectedMetadataType,
                            format!(
                                "metadata key {} was expected to be an unsigned integer, not {}",
                                quoted_key(&key),
                                value_type.as_str()
                            ),
                        );
                        self.skip_scalar_value(value_type, Some(&key))?;
                    }
                } else {
                    self.skip_scalar_value(value_type, Some(&key))?;
                }
            }
        }

        let alignment = parse_alignment(&numeric_values)?;
        self.alignment = Some(alignment.value);
        let (tensor_types, max_tensor_offset) = self.parse_tensor_descriptors(tensor_count)?;
        let tensor_data_offset = if tensor_count == 0 {
            self.offset
        } else {
            let aligned = align_up(self.offset, u64::from(alignment.value))?;
            if aligned > self.file_size {
                return Err(GgufError::new(
                    GgufErrorKind::Truncated,
                    "aligned tensor-data offset extends beyond the file",
                    Some(self.offset),
                ));
            }
            aligned
        };
        let tensor_data_bytes = self.file_size.saturating_sub(tensor_data_offset);
        if max_tensor_offset > tensor_data_bytes {
            return Err(GgufError::new(
                GgufErrorKind::InvalidTensor,
                format!(
                    "tensor data offset {max_tensor_offset} exceeds the available \
                     {tensor_data_bytes} bytes of tensor data"
                ),
                Some(self.offset),
            ));
        }

        if general
            .architecture
            .as_ref()
            .is_some_and(|value| value.value.is_empty())
        {
            general.architecture = None;
        }
        let metadata_state = if general.architecture.is_some() {
            MetadataState::Available
        } else {
            push_warning(
                &mut warnings,
                WarningCode::MissingArchitecture,
                "required general.architecture metadata is absent or empty".to_string(),
            );
            MetadataState::Incomplete
        };
        apply_general_numeric_fields(&mut general, &numeric_values, &mut warnings);
        let architecture_fields = extract_architecture_fields(&general, &numeric_values);
        general.quantization = quantization_hint(&general.file_type, &tensor_types);
        for tensor_type in &tensor_types {
            if tensor_type.name.is_none() {
                push_warning(
                    &mut warnings,
                    WarningCode::UnknownTensorType,
                    format!(
                        "tensor type identifier {} has no reviewed name",
                        tensor_type.type_id
                    ),
                );
            }
        }

        Ok(ModelMetadata {
            gguf_version: version,
            tensor_count,
            metadata_entry_count,
            metadata_state,
            alignment,
            tensor_data_offset: Bytes::new(tensor_data_offset),
            tensor_data_bytes: Bytes::new(tensor_data_bytes),
            general,
            architecture_fields,
            tokenizer,
            tensor_types,
            warnings,
        })
    }

    fn read_key(&mut self) -> Result<String, GgufError> {
        let length_offset = self.offset;
        let length = self.read_u64("metadata key length")?;
        if length > self.limits.metadata_key_bytes {
            return Err(GgufError::new(
                GgufErrorKind::LimitExceeded,
                format!(
                    "metadata key length {length} exceeds the limit of {}",
                    self.limits.metadata_key_bytes
                ),
                Some(length_offset),
            ));
        }
        if length == 0 {
            return Err(GgufError::new(
                GgufErrorKind::Malformed,
                "metadata key is empty",
                Some(length_offset),
            ));
        }
        let length = usize::try_from(length).map_err(|_| {
            GgufError::new(
                GgufErrorKind::LimitExceeded,
                "metadata key length does not fit in memory address space",
                Some(length_offset),
            )
        })?;
        let bytes = self.read_owned(length, "metadata key")?;
        String::from_utf8(bytes).map_err(|_| {
            GgufError::new(
                GgufErrorKind::Malformed,
                "metadata key is not valid UTF-8",
                Some(length_offset),
            )
        })
    }

    fn read_retained_string(&mut self) -> Result<RetainedString, GgufError> {
        let length_offset = self.offset;
        let length = self.read_u64("string length")?;
        if length > self.limits.encoded_string_bytes {
            return Err(GgufError::new(
                GgufErrorKind::LimitExceeded,
                format!(
                    "encoded string length {length} exceeds the limit of {}",
                    self.limits.encoded_string_bytes
                ),
                Some(length_offset),
            ));
        }
        if length > self.limits.retained_string_bytes {
            self.skip_exact(length, "skipped string value")?;
            return Ok(RetainedString::Omitted);
        }
        let length = usize::try_from(length).map_err(|_| {
            GgufError::new(
                GgufErrorKind::LimitExceeded,
                "retained string length does not fit in memory address space",
                Some(length_offset),
            )
        })?;
        let bytes = self.read_owned(length, "string value")?;
        String::from_utf8(bytes)
            .map(RetainedString::Value)
            .map_err(|_| {
                GgufError::new(
                    GgufErrorKind::Malformed,
                    "metadata string is not valid UTF-8",
                    Some(length_offset),
                )
            })
    }

    fn read_array_value(&mut self, total_array_elements: &mut u64) -> Result<ArrayInfo, GgufError> {
        let element_type_offset = self.offset;
        let element_type_id = self.read_u32("array element type")?;
        let element_type = MetadataValueType::from_id(element_type_id).ok_or_else(|| {
            GgufError::new(
                GgufErrorKind::UnsupportedMetadataType,
                format!("array element type code {element_type_id} is not documented"),
                Some(element_type_offset),
            )
        })?;
        let count_offset = self.offset;
        let count = self.read_u64("array element count")?;
        if count > self.limits.array_elements {
            return Err(GgufError::new(
                GgufErrorKind::LimitExceeded,
                format!(
                    "array element count {count} exceeds the limit of {}",
                    self.limits.array_elements
                ),
                Some(count_offset),
            ));
        }
        *total_array_elements = total_array_elements.checked_add(count).ok_or_else(|| {
            GgufError::new(
                GgufErrorKind::LimitExceeded,
                "total array element count overflow",
                Some(count_offset),
            )
        })?;
        if *total_array_elements > self.limits.total_array_elements {
            return Err(GgufError::new(
                GgufErrorKind::LimitExceeded,
                format!(
                    "total array elements exceed the limit of {}",
                    self.limits.total_array_elements
                ),
                Some(count_offset),
            ));
        }
        self.skip_array_elements(element_type, count, 0, total_array_elements)?;
        Ok(ArrayInfo {
            element_type,
            count,
        })
    }

    fn skip_array_elements(
        &mut self,
        element_type: MetadataValueType,
        count: u64,
        depth: u32,
        total_array_elements: &mut u64,
    ) -> Result<(), GgufError> {
        if element_type == MetadataValueType::Bool {
            for _ in 0..count {
                let offset = self.offset;
                if self.read_u8("boolean array element")? > 1 {
                    return Err(GgufError::new(
                        GgufErrorKind::Malformed,
                        "boolean array element is not zero or one",
                        Some(offset),
                    ));
                }
            }
            return Ok(());
        }
        if element_type != MetadataValueType::Array {
            if let Some(element_size) = element_type.fixed_size() {
                let byte_count = count.checked_mul(element_size).ok_or_else(|| {
                    GgufError::new(
                        GgufErrorKind::LimitExceeded,
                        "array byte length overflow",
                        Some(self.offset),
                    )
                })?;
                return self.skip_exact(byte_count, "array elements");
            }
        }

        match element_type {
            MetadataValueType::String => {
                for _ in 0..count {
                    self.skip_encoded_string("array string")?;
                }
                Ok(())
            }
            MetadataValueType::Array => {
                let next_depth = depth.checked_add(1).ok_or_else(|| {
                    GgufError::new(
                        GgufErrorKind::LimitExceeded,
                        "nested-array depth overflow",
                        Some(self.offset),
                    )
                })?;
                if next_depth > self.limits.array_nesting {
                    return Err(GgufError::new(
                        GgufErrorKind::LimitExceeded,
                        format!(
                            "nested-array depth exceeds the limit of {}",
                            self.limits.array_nesting
                        ),
                        Some(self.offset),
                    ));
                }
                for _ in 0..count {
                    let type_offset = self.offset;
                    let nested_type_id = self.read_u32("nested array element type")?;
                    let nested_type =
                        MetadataValueType::from_id(nested_type_id).ok_or_else(|| {
                            GgufError::new(
                                GgufErrorKind::UnsupportedMetadataType,
                                format!(
                                    "nested array element type code {nested_type_id} is not documented"
                                ),
                                Some(type_offset),
                            )
                        })?;
                    let count_offset = self.offset;
                    let nested_count = self.read_u64("nested array element count")?;
                    if nested_count > self.limits.array_elements {
                        return Err(GgufError::new(
                            GgufErrorKind::LimitExceeded,
                            format!(
                                "nested array element count {nested_count} exceeds the limit of {}",
                                self.limits.array_elements
                            ),
                            Some(count_offset),
                        ));
                    }
                    *total_array_elements = total_array_elements
                        .checked_add(nested_count)
                        .ok_or_else(|| {
                            GgufError::new(
                                GgufErrorKind::LimitExceeded,
                                "total array element count overflow",
                                Some(count_offset),
                            )
                        })?;
                    if *total_array_elements > self.limits.total_array_elements {
                        return Err(GgufError::new(
                            GgufErrorKind::LimitExceeded,
                            format!(
                                "total array elements exceed the limit of {}",
                                self.limits.total_array_elements
                            ),
                            Some(count_offset),
                        ));
                    }
                    self.skip_array_elements(
                        nested_type,
                        nested_count,
                        next_depth,
                        total_array_elements,
                    )?;
                }
                Ok(())
            }
            _ => unreachable!("fixed-size element types returned above"),
        }
    }

    fn skip_scalar_value(
        &mut self,
        value_type: MetadataValueType,
        key: Option<&str>,
    ) -> Result<(), GgufError> {
        match value_type {
            MetadataValueType::String => self.skip_encoded_string("string value"),
            MetadataValueType::Bool => {
                let offset = self.offset;
                let value = self.read_u8("boolean metadata value")?;
                if value > 1 {
                    let detail = key.map_or_else(
                        || "boolean metadata value is not zero or one".to_string(),
                        |key| {
                            format!(
                                "boolean value for metadata key {} is not zero or one",
                                quoted_key(key)
                            )
                        },
                    );
                    return Err(GgufError::new(
                        GgufErrorKind::Malformed,
                        detail,
                        Some(offset),
                    ));
                }
                Ok(())
            }
            MetadataValueType::Array => Err(GgufError::new(
                GgufErrorKind::Malformed,
                "array value reached the scalar decoder",
                Some(self.offset),
            )),
            _ => {
                let size = value_type
                    .fixed_size()
                    .expect("scalar type has a fixed size");
                self.skip_exact(size, "scalar metadata value")
            }
        }
    }

    fn read_unsigned_value(&mut self, value_type: MetadataValueType) -> Result<u64, GgufError> {
        match value_type {
            MetadataValueType::Uint8 => Ok(u64::from(self.read_u8("uint8 metadata value")?)),
            MetadataValueType::Uint16 => Ok(u64::from(self.read_u16("uint16 metadata value")?)),
            MetadataValueType::Uint32 => Ok(u64::from(self.read_u32("uint32 metadata value")?)),
            MetadataValueType::Uint64 => self.read_u64("uint64 metadata value"),
            _ => Err(GgufError::new(
                GgufErrorKind::Malformed,
                "non-unsigned metadata value reached the unsigned decoder",
                Some(self.offset),
            )),
        }
    }

    fn skip_encoded_string(&mut self, context: &str) -> Result<(), GgufError> {
        let offset = self.offset;
        let length = self.read_u64("skipped string length")?;
        if length > self.limits.encoded_string_bytes {
            return Err(GgufError::new(
                GgufErrorKind::LimitExceeded,
                format!(
                    "encoded string length {length} exceeds the limit of {}",
                    self.limits.encoded_string_bytes
                ),
                Some(offset),
            ));
        }
        self.skip_exact(length, context)
    }

    fn parse_tensor_descriptors(
        &mut self,
        tensor_count: u64,
    ) -> Result<(Vec<TensorTypeCount>, u64), GgufError> {
        let mut names = HashSet::new();
        let mut total_name_bytes = 0_u64;
        let mut types = BTreeMap::<u32, u64>::new();
        let mut max_tensor_offset = 0_u64;

        for _ in 0..tensor_count {
            let name_offset = self.offset;
            let name_length_offset = self.offset;
            let name_length = self.read_u64("tensor name length")?;
            if name_length > self.limits.tensor_name_bytes {
                return Err(GgufError::new(
                    GgufErrorKind::LimitExceeded,
                    format!(
                        "tensor name length {name_length} exceeds the limit of {}",
                        self.limits.tensor_name_bytes
                    ),
                    Some(name_length_offset),
                ));
            }
            total_name_bytes = total_name_bytes.checked_add(name_length).ok_or_else(|| {
                GgufError::new(
                    GgufErrorKind::LimitExceeded,
                    "total tensor-name byte count overflow",
                    Some(name_offset),
                )
            })?;
            if total_name_bytes > self.limits.total_tensor_name_bytes {
                return Err(GgufError::new(
                    GgufErrorKind::LimitExceeded,
                    format!(
                        "total tensor-name bytes exceed the limit of {}",
                        self.limits.total_tensor_name_bytes
                    ),
                    Some(name_offset),
                ));
            }
            let name_length = usize::try_from(name_length).map_err(|_| {
                GgufError::new(
                    GgufErrorKind::LimitExceeded,
                    "tensor name length does not fit in memory address space",
                    Some(name_length_offset),
                )
            })?;
            let name_bytes = self.read_owned(name_length, "tensor name")?;
            let name = String::from_utf8(name_bytes).map_err(|_| {
                GgufError::new(
                    GgufErrorKind::Malformed,
                    "tensor name is not valid UTF-8",
                    Some(name_offset),
                )
            })?;
            if name.is_empty() {
                return Err(GgufError::new(
                    GgufErrorKind::InvalidTensor,
                    "tensor name is empty",
                    Some(name_offset),
                ));
            }
            if !names.insert(name.clone()) {
                return Err(GgufError::new(
                    GgufErrorKind::DuplicateTensorName,
                    format!("duplicate tensor name {name:?}"),
                    Some(name_offset),
                ));
            }

            let dimensions_offset = self.offset;
            let dimensions = self.read_u32("tensor dimension count")?;
            if dimensions > self.limits.tensor_dimensions {
                return Err(GgufError::new(
                    GgufErrorKind::InvalidTensor,
                    format!(
                        "tensor {} has {dimensions} dimensions; at most {} are supported",
                        name, self.limits.tensor_dimensions
                    ),
                    Some(dimensions_offset),
                ));
            }
            let mut element_count = 1_u64;
            for _ in 0..dimensions {
                let dimension_offset = self.offset;
                let dimension = self.read_u64("tensor dimension")?;
                if dimension > self.limits.tensor_dimension {
                    return Err(GgufError::new(
                        GgufErrorKind::InvalidTensor,
                        format!(
                            "tensor {} dimension {dimension} exceeds the supported maximum {}",
                            name, self.limits.tensor_dimension
                        ),
                        Some(dimension_offset),
                    ));
                }
                element_count = element_count.checked_mul(dimension).ok_or_else(|| {
                    GgufError::new(
                        GgufErrorKind::InvalidTensor,
                        format!("tensor {name} element count overflows"),
                        Some(dimension_offset),
                    )
                })?;
                if element_count > self.limits.tensor_dimension {
                    return Err(GgufError::new(
                        GgufErrorKind::InvalidTensor,
                        format!("tensor {name} element count exceeds the supported maximum"),
                        Some(dimension_offset),
                    ));
                }
            }

            let type_offset = self.offset;
            let type_id = self.read_u32("tensor type")?;
            if let Some(count) = types.get_mut(&type_id) {
                *count = count.checked_add(1).ok_or_else(|| {
                    GgufError::new(
                        GgufErrorKind::LimitExceeded,
                        "tensor type count overflow",
                        Some(type_offset),
                    )
                })?;
            } else {
                if types.len() >= self.limits.distinct_tensor_types {
                    return Err(GgufError::new(
                        GgufErrorKind::LimitExceeded,
                        format!(
                            "distinct tensor type count exceeds the limit of {}",
                            self.limits.distinct_tensor_types
                        ),
                        Some(type_offset),
                    ));
                }
                types.insert(type_id, 1);
            }

            let offset_offset = self.offset;
            let data_offset = self.read_u64("tensor data offset")?;
            let alignment = self.alignment.ok_or_else(|| {
                GgufError::new(
                    GgufErrorKind::Malformed,
                    "alignment was not initialized before tensor descriptors",
                    Some(type_offset),
                )
            })?;
            if data_offset % u64::from(alignment) != 0 {
                return Err(GgufError::new(
                    GgufErrorKind::InvalidTensor,
                    format!(
                        "tensor {name} data offset {data_offset} is not a multiple of alignment {alignment}"
                    ),
                    Some(offset_offset),
                ));
            }
            max_tensor_offset = max_tensor_offset.max(data_offset);
        }

        let tensor_types = types
            .into_iter()
            .map(|(type_id, tensor_count)| TensorTypeCount {
                type_id,
                name: tensor_type_name(type_id).map(str::to_string),
                tensor_count,
            })
            .collect();
        Ok((tensor_types, max_tensor_offset))
    }

    fn read_array<const N: usize>(&mut self, context: &str) -> Result<[u8; N], GgufError> {
        let mut bytes = [0_u8; N];
        self.read_exact(&mut bytes, context)?;
        Ok(bytes)
    }

    fn read_u8(&mut self, context: &str) -> Result<u8, GgufError> {
        Ok(self.read_array::<1>(context)?[0])
    }

    fn read_u16(&mut self, context: &str) -> Result<u16, GgufError> {
        Ok(u16::from_le_bytes(self.read_array::<2>(context)?))
    }

    fn read_u32(&mut self, context: &str) -> Result<u32, GgufError> {
        Ok(u32::from_le_bytes(self.read_array::<4>(context)?))
    }

    fn read_u64(&mut self, context: &str) -> Result<u64, GgufError> {
        Ok(u64::from_le_bytes(self.read_array::<8>(context)?))
    }

    fn read_owned(&mut self, length: usize, context: &str) -> Result<Vec<u8>, GgufError> {
        let mut bytes = vec![0_u8; length];
        self.read_exact(&mut bytes, context)?;
        Ok(bytes)
    }

    fn read_exact(&mut self, bytes: &mut [u8], context: &str) -> Result<(), GgufError> {
        let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        self.ensure_available(length, context)?;
        let offset = self.offset;
        self.reader.read_exact(bytes).map_err(|error| {
            if error.kind() == ErrorKind::UnexpectedEof {
                GgufError::new(
                    GgufErrorKind::Truncated,
                    format!("file ended while reading {context}"),
                    Some(offset),
                )
            } else {
                GgufError::new(
                    GgufErrorKind::Io,
                    format!("could not read {context}: {error}"),
                    Some(offset),
                )
            }
        })?;
        self.offset = offset.checked_add(length).ok_or_else(|| {
            GgufError::new(
                GgufErrorKind::Malformed,
                "read offset overflow",
                Some(offset),
            )
        })?;
        Ok(())
    }

    fn skip_exact(&mut self, length: u64, context: &str) -> Result<(), GgufError> {
        self.ensure_available(length, context)?;
        let new_offset = self.offset.checked_add(length).ok_or_else(|| {
            GgufError::new(
                GgufErrorKind::Malformed,
                "skip offset overflow",
                Some(self.offset),
            )
        })?;
        self.reader
            .seek(SeekFrom::Start(new_offset))
            .map_err(|error| {
                GgufError::new(
                    GgufErrorKind::Io,
                    format!("could not seek past {context}: {error}"),
                    Some(self.offset),
                )
            })?;
        self.offset = new_offset;
        Ok(())
    }

    fn ensure_available(&self, length: u64, context: &str) -> Result<(), GgufError> {
        let remaining = self.file_size.saturating_sub(self.offset);
        if length > remaining {
            Err(GgufError::new(
                GgufErrorKind::Truncated,
                format!("{context} needs {length} bytes but only {remaining} remain in the file"),
                Some(self.offset),
            ))
        } else {
            Ok(())
        }
    }
}

fn parse_alignment(
    numeric_values: &HashMap<String, NumericValue>,
) -> Result<Field<u32>, GgufError> {
    let Some(value) = numeric_values.get("general.alignment") else {
        return Ok(Field::defaulted(DEFAULT_GGUF_ALIGNMENT));
    };
    if value.value_type != MetadataValueType::Uint32 {
        return Err(GgufError::new(
            GgufErrorKind::InvalidAlignment,
            "general.alignment must be encoded as uint32",
            Some(value.offset),
        ));
    }
    let alignment = u32::try_from(value.value).map_err(|_| {
        GgufError::new(
            GgufErrorKind::InvalidAlignment,
            "general.alignment does not fit in uint32",
            Some(value.offset),
        )
    })?;
    if alignment < 8 || alignment % 8 != 0 {
        return Err(GgufError::new(
            GgufErrorKind::InvalidAlignment,
            format!("general.alignment {alignment} must be a nonzero multiple of 8"),
            Some(value.offset),
        ));
    }
    Ok(Field::explicit(alignment))
}

fn apply_general_numeric_fields(
    general: &mut GeneralMetadata,
    numeric_values: &HashMap<String, NumericValue>,
    warnings: &mut Vec<CatalogWarning>,
) {
    if let Some(value) = numeric_values.get("general.quantization_version") {
        match u32::try_from(value.value) {
            Ok(code) => general.quantization_version = Some(Field::explicit(code)),
            Err(_) => push_warning(
                warnings,
                WarningCode::UnexpectedMetadataType,
                "general.quantization_version does not fit in uint32".to_string(),
            ),
        }
    }
    if let Some(value) = numeric_values.get("general.file_type") {
        match u32::try_from(value.value) {
            Ok(code) => {
                general.file_type = Some(Field::explicit(FileType {
                    code,
                    label: file_type_label(code).map(str::to_string),
                }));
            }
            Err(_) => push_warning(
                warnings,
                WarningCode::UnexpectedMetadataType,
                "general.file_type does not fit in uint32".to_string(),
            ),
        }
    }
    if let Some(value) = numeric_values.get("general.parameter_count") {
        general.parameter_count = Some(Field::explicit(value.value));
    }
}

fn extract_architecture_fields(
    general: &GeneralMetadata,
    numeric_values: &HashMap<String, NumericValue>,
) -> ArchitectureFields {
    let Some(architecture) = general
        .architecture
        .as_ref()
        .map(|architecture| architecture.value.as_str())
    else {
        return ArchitectureFields::default();
    };
    let get = |suffix: &str| {
        numeric_values
            .get(&format!("{architecture}.{suffix}"))
            .map(|value| Field::explicit(value.value))
    };
    ArchitectureFields {
        context_length: get("context_length"),
        embedding_length: get("embedding_length"),
        block_count: get("block_count"),
        feed_forward_length: get("feed_forward_length"),
        attention_head_count: get("attention.head_count"),
        attention_head_count_kv: get("attention.head_count_kv"),
        attention_key_length: get("attention.key_length"),
        attention_value_length: get("attention.value_length"),
        expert_count: get("expert_count"),
        expert_used_count: get("expert_used_count"),
        rope_dimension_count: get("rope.dimension_count"),
    }
}

fn quantization_hint(
    file_type: &Option<Field<FileType>>,
    tensor_types: &[TensorTypeCount],
) -> Option<Field<String>> {
    if let Some(file_type) = file_type {
        if let Some(label) = &file_type.value.label {
            let quantization = if label == "ALL_F32" {
                "F32".to_string()
            } else {
                label
                    .strip_prefix("MOSTLY_")
                    .unwrap_or(label.as_str())
                    .to_string()
            };
            return Some(Field::explicit(quantization));
        }
    }
    tensor_types
        .iter()
        .filter_map(|tensor_type| {
            tensor_type
                .name
                .as_ref()
                .map(|name| (tensor_type.tensor_count, tensor_type.type_id, name))
        })
        .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
        .map(|(_, _, name)| Field::derived(name.clone()))
}

fn file_type_label(code: u32) -> Option<&'static str> {
    match code {
        0 => Some("ALL_F32"),
        1 => Some("MOSTLY_F16"),
        2 => Some("MOSTLY_Q4_0"),
        3 => Some("MOSTLY_Q4_1"),
        4 => Some("MOSTLY_Q4_1_SOME_F16"),
        5 => Some("MOSTLY_Q4_2"),
        6 => Some("MOSTLY_Q4_3"),
        7 => Some("MOSTLY_Q8_0"),
        8 => Some("MOSTLY_Q5_0"),
        9 => Some("MOSTLY_Q5_1"),
        10 => Some("MOSTLY_Q2_K"),
        11 => Some("MOSTLY_Q3_K_S"),
        12 => Some("MOSTLY_Q3_K_M"),
        13 => Some("MOSTLY_Q3_K_L"),
        14 => Some("MOSTLY_Q4_K_S"),
        15 => Some("MOSTLY_Q4_K_M"),
        16 => Some("MOSTLY_Q5_K_S"),
        17 => Some("MOSTLY_Q5_K_M"),
        18 => Some("MOSTLY_Q6_K"),
        _ => None,
    }
}

fn tensor_type_name(type_id: u32) -> Option<&'static str> {
    match type_id {
        0 => Some("F32"),
        1 => Some("F16"),
        2 => Some("Q4_0"),
        3 => Some("Q4_1"),
        6 => Some("Q5_0"),
        7 => Some("Q5_1"),
        8 => Some("Q8_0"),
        9 => Some("Q8_1"),
        10 => Some("Q2_K"),
        11 => Some("Q3_K"),
        12 => Some("Q4_K"),
        13 => Some("Q5_K"),
        14 => Some("Q6_K"),
        15 => Some("Q8_K"),
        16 => Some("IQ2_XXS"),
        17 => Some("IQ2_XS"),
        18 => Some("IQ3_XXS"),
        19 => Some("IQ1_S"),
        20 => Some("IQ4_NL"),
        21 => Some("IQ3_S"),
        22 => Some("IQ2_S"),
        23 => Some("IQ4_XS"),
        24 => Some("I8"),
        25 => Some("I16"),
        26 => Some("I32"),
        27 => Some("I64"),
        28 => Some("F64"),
        29 => Some("IQ1_M"),
        30 => Some("BF16"),
        34 => Some("TQ1_0"),
        35 => Some("TQ2_0"),
        39 => Some("MXFP4"),
        40 => Some("NVFP4"),
        41 => Some("Q1_0"),
        42 => Some("Q2_0"),
        _ => None,
    }
}

fn string_key(key: &str) -> Option<StringKey> {
    match key {
        "general.architecture" => Some(StringKey::Architecture),
        "general.name" => Some(StringKey::Name),
        "general.basename" => Some(StringKey::Basename),
        "general.finetune" => Some(StringKey::Finetune),
        "general.size_label" => Some(StringKey::SizeLabel),
        "tokenizer.ggml.model" => Some(StringKey::TokenizerModel),
        "tokenizer.chat_template" => Some(StringKey::ChatTemplate),
        "tokenizer.huggingface.json" => Some(StringKey::HuggingfaceJson),
        _ => None,
    }
}

fn array_key(key: &str) -> Option<ArrayKey> {
    match key {
        "tokenizer.ggml.tokens" => Some(ArrayKey::Tokens),
        "tokenizer.ggml.scores" => Some(ArrayKey::Scores),
        "tokenizer.ggml.token_type" => Some(ArrayKey::TokenTypes),
        "tokenizer.ggml.merges" => Some(ArrayKey::Merges),
        "tokenizer.ggml.added_tokens" => Some(ArrayKey::AddedTokens),
        _ => None,
    }
}

fn expected_array_element_type(key: ArrayKey) -> Option<MetadataValueType> {
    match key {
        ArrayKey::Tokens | ArrayKey::Merges | ArrayKey::AddedTokens => {
            Some(MetadataValueType::String)
        }
        ArrayKey::Scores => Some(MetadataValueType::Float32),
        ArrayKey::TokenTypes => Some(MetadataValueType::Int32),
    }
}

fn is_numeric_key(key: &str) -> bool {
    const EXACT: [&str; 4] = [
        "general.quantization_version",
        "general.file_type",
        "general.parameter_count",
        "general.alignment",
    ];
    const SUFFIXES: [&str; 11] = [
        "context_length",
        "embedding_length",
        "block_count",
        "feed_forward_length",
        "attention.head_count",
        "attention.head_count_kv",
        "attention.key_length",
        "attention.value_length",
        "expert_count",
        "expert_used_count",
        "rope.dimension_count",
    ];
    EXACT.contains(&key) || SUFFIXES.iter().any(|suffix| has_dotted_suffix(key, suffix))
}

fn has_dotted_suffix(key: &str, suffix: &str) -> bool {
    key.len() > suffix.len()
        && key.ends_with(suffix)
        && key.as_bytes()[key.len() - suffix.len() - 1] == b'.'
}

fn is_unsigned_integer(value_type: MetadataValueType) -> bool {
    matches!(
        value_type,
        MetadataValueType::Uint8
            | MetadataValueType::Uint16
            | MetadataValueType::Uint32
            | MetadataValueType::Uint64
    )
}

fn set_string(
    general: &mut GeneralMetadata,
    tokenizer: &mut TokenizerMetadata,
    key: StringKey,
    value: String,
) {
    match key {
        StringKey::Architecture => general.architecture = Some(Field::explicit(value)),
        StringKey::Name => general.name = Some(Field::explicit(value)),
        StringKey::Basename => general.basename = Some(Field::explicit(value)),
        StringKey::Finetune => general.finetune = Some(Field::explicit(value)),
        StringKey::SizeLabel => general.size_label = Some(Field::explicit(value)),
        StringKey::TokenizerModel => tokenizer.model = Some(Field::explicit(value)),
        StringKey::ChatTemplate | StringKey::HuggingfaceJson => {
            set_presence(tokenizer, key);
        }
    }
}

fn set_presence(tokenizer: &mut TokenizerMetadata, key: StringKey) {
    match key {
        StringKey::ChatTemplate => tokenizer.chat_template_present = true,
        StringKey::HuggingfaceJson => tokenizer.huggingface_json_present = true,
        _ => {}
    }
}

fn set_array(tokenizer: &mut TokenizerMetadata, key: ArrayKey, value: ArrayInfo) {
    match key {
        ArrayKey::Tokens => tokenizer.tokens = Some(value),
        ArrayKey::Scores => tokenizer.scores = Some(value),
        ArrayKey::TokenTypes => tokenizer.token_types = Some(value),
        ArrayKey::Merges => tokenizer.merges = Some(value),
        ArrayKey::AddedTokens => tokenizer.added_tokens = Some(value),
    }
}

fn set_array_absent(tokenizer: &mut TokenizerMetadata, key: ArrayKey) {
    match key {
        ArrayKey::Tokens => tokenizer.tokens = None,
        ArrayKey::Scores => tokenizer.scores = None,
        ArrayKey::TokenTypes => tokenizer.token_types = None,
        ArrayKey::Merges => tokenizer.merges = None,
        ArrayKey::AddedTokens => tokenizer.added_tokens = None,
    }
}

fn push_warning(warnings: &mut Vec<CatalogWarning>, code: WarningCode, message: String) {
    warnings.push(CatalogWarning { code, message });
}

fn align_up(offset: u64, alignment: u64) -> Result<u64, GgufError> {
    let remainder = offset % alignment;
    if remainder == 0 {
        return Ok(offset);
    }
    offset.checked_add(alignment - remainder).ok_or_else(|| {
        GgufError::new(
            GgufErrorKind::Malformed,
            "aligned tensor-data offset overflow",
            Some(offset),
        )
    })
}

fn quoted_key(key: &str) -> String {
    const MAX_KEY_DISPLAY: usize = 120;
    if key.len() <= MAX_KEY_DISPLAY {
        return format!("{key:?}");
    }
    let mut end = MAX_KEY_DISPLAY;
    while !key.is_char_boundary(end) {
        end -= 1;
    }
    format!("{:?}...", &key[..end])
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        GgufLimits, MAX_ENCODED_STRING_BYTES, MAX_METADATA_KEY_BYTES, parse_gguf,
        parse_gguf_with_limits,
    };
    use crate::catalog::{
        MetadataState, MetadataValueType, ModelMetadata, ParseStatus, Provenance, WarningCode,
    };
    use crate::error::{GgufError, GgufErrorKind};
    use crate::test_support::GgufBuilder;

    fn parse(bytes: &[u8]) -> Result<ModelMetadata, GgufError> {
        let mut cursor = Cursor::new(bytes);
        parse_gguf(&mut cursor, u64::try_from(bytes.len()).unwrap())
    }

    fn parse_limited(bytes: &[u8], limits: &GgufLimits) -> Result<ModelMetadata, GgufError> {
        let mut cursor = Cursor::new(bytes);
        parse_gguf_with_limits(&mut cursor, u64::try_from(bytes.len()).unwrap(), limits)
    }

    fn minimal_builder() -> GgufBuilder {
        let mut builder = GgufBuilder::new(3);
        builder.u32("general.alignment", 32);
        builder.string("general.architecture", "llama");
        builder
    }

    fn minimal_file() -> Vec<u8> {
        minimal_builder().finish(32)
    }

    fn error_kind(bytes: &[u8]) -> GgufErrorKind {
        parse(bytes).unwrap_err().kind()
    }

    #[test]
    fn parses_v3_metadata_tensors_and_tokenizer_summaries() {
        let mut builder = GgufBuilder::new(3);
        builder.u32("general.alignment", 32);
        builder.string("general.architecture", "llama");
        builder.string("general.name", "Synthetic Llama");
        builder.u32("general.quantization_version", 2);
        builder.u32("general.file_type", 15);
        builder.u64("general.parameter_count", 8_000_000_000);
        builder.u64("llama.context_length", 8192);
        builder.u64("llama.block_count", 32);
        builder.u64("llama.attention.head_count", 32);
        builder.u64("llama.attention.head_count_kv", 8);
        builder.u64("llama.expert_count", 0);
        builder.u64("llama.unknown_numeric", 7);
        builder.string_array("tokenizer.ggml.tokens", &["hello", "world", "!"]);
        builder.array("tokenizer.ggml.scores", 6, 3, &[0_u8; 12]);
        builder.string_array("tokenizer.ggml.merges", &["a b"]);
        builder.array("tokenizer.ggml.token_type", 5, 3, &[0_u8; 12]);
        builder.string_array("tokenizer.ggml.added_tokens", &[]);
        builder.string(
            "tokenizer.chat_template",
            "{% for message in messages %}{{ message }}{% endfor %}",
        );
        builder.string("tokenizer.huggingface.json", "{\"synthetic\":true}");
        builder.tensor("token_embd.weight", &[16, 8], 1, 0);
        builder.tensor("blk.0.attn_q.weight", &[16, 8], 12, 32);
        let bytes = builder.finish_with_payload(32, &[0_u8; 64]);

        let metadata = parse(&bytes).unwrap();
        assert_eq!(metadata.gguf_version, 3);
        assert_eq!(metadata.tensor_count, 2);
        assert_eq!(metadata.alignment.value, 32);
        assert_eq!(metadata.alignment.source, Provenance::Explicit);
        assert_eq!(metadata.metadata_state, MetadataState::Available);
        assert_eq!(
            metadata.general.quantization.as_ref().unwrap().value,
            "Q4_K_M"
        );
        assert_eq!(
            metadata
                .general
                .file_type
                .as_ref()
                .unwrap()
                .value
                .label
                .as_deref(),
            Some("MOSTLY_Q4_K_M")
        );
        assert_eq!(
            metadata
                .architecture_fields
                .context_length
                .as_ref()
                .unwrap()
                .value,
            8192
        );
        assert_eq!(
            metadata
                .architecture_fields
                .attention_head_count_kv
                .as_ref()
                .unwrap()
                .value,
            8
        );
        assert_eq!(metadata.tokenizer.tokens.as_ref().unwrap().count, 3);
        assert_eq!(
            metadata.tokenizer.tokens.as_ref().unwrap().element_type,
            MetadataValueType::String
        );
        assert_eq!(metadata.tokenizer.scores.as_ref().unwrap().count, 3);
        assert_eq!(metadata.tokenizer.merges.as_ref().unwrap().count, 1);
        assert_eq!(metadata.tokenizer.token_types.as_ref().unwrap().count, 3);
        assert_eq!(metadata.tokenizer.added_tokens.as_ref().unwrap().count, 0);
        assert!(metadata.tokenizer.chat_template_present);
        assert!(metadata.tokenizer.huggingface_json_present);
        assert_eq!(
            metadata.tensor_types,
            vec![
                crate::catalog::TensorTypeCount {
                    type_id: 1,
                    name: Some("F16".to_string()),
                    tensor_count: 1,
                },
                crate::catalog::TensorTypeCount {
                    type_id: 12,
                    name: Some("Q4_K".to_string()),
                    tensor_count: 1,
                },
            ]
        );
        assert_eq!(metadata.tensor_data_offset.as_u64() % 32, 0);
        assert_eq!(metadata.tensor_data_bytes.as_u64(), 64);
        assert!(metadata.warnings.is_empty());
    }

    #[test]
    fn parses_v2_with_the_same_64_bit_counts() {
        let bytes = GgufBuilder::new(2).finish(32);
        let metadata = parse(&bytes).unwrap();
        assert_eq!(metadata.gguf_version, 2);
        assert_eq!(metadata.metadata_state, MetadataState::Incomplete);
    }

    #[test]
    fn rejects_v1_future_and_zero_versions() {
        for version in [0_u32, 1, 4, 99] {
            let bytes = GgufBuilder::new(version).finish(32);
            assert_eq!(
                error_kind(&bytes),
                GgufErrorKind::UnsupportedVersion,
                "version {version}"
            );
        }
    }

    #[test]
    fn rejects_big_endian_versions_explicitly() {
        let mut bytes = minimal_file();
        bytes[4..8].copy_from_slice(&3_u32.to_be_bytes());
        assert_eq!(error_kind(&bytes), GgufErrorKind::UnsupportedEndianness);
    }

    #[test]
    fn rejects_bad_magic_and_short_headers() {
        let mut bytes = minimal_file();
        bytes[0] = b'X';
        assert_eq!(error_kind(&bytes), GgufErrorKind::InvalidMagic);
        assert_eq!(error_kind(&[]), GgufErrorKind::Truncated);
        assert_eq!(error_kind(b"GGUF"), GgufErrorKind::Truncated);
    }

    #[test]
    fn supports_all_documented_scalar_metadata_types() {
        let mut builder = GgufBuilder::new(3);
        builder.string("general.architecture", "testarch");
        builder.u8("custom.u8", 255);
        builder.i8("custom.i8", -1);
        builder.u16("custom.u16", 65_535);
        builder.i16("custom.i16", -2);
        builder.u32("custom.u32", 4_000_000_000);
        builder.i32("custom.i32", -3);
        builder.f32_bits("custom.f32", 1.5_f32.to_bits());
        builder.boolean("custom.bool", true);
        builder.string("custom.string", "value");
        builder.u64("custom.u64", u64::MAX);
        builder.i64("custom.i64", -4);
        builder.f64_bits("custom.f64", 2.5_f64.to_bits());
        builder.array("custom.array", 4, 2, &[1_u8; 8]);
        let bytes = builder.finish(32);
        let metadata = parse(&bytes).unwrap();
        assert_eq!(metadata.metadata_entry_count, 14);
        assert!(metadata.warnings.is_empty());
    }

    #[test]
    fn skips_unknown_keys_and_rejects_duplicates() {
        let mut builder = minimal_builder();
        builder.string("general.custom", "ignored");
        builder.boolean("custom.enabled", false);
        let bytes = builder.finish(32);
        assert!(parse(&bytes).is_ok());

        let mut duplicate = minimal_builder();
        duplicate.u32("general.alignment", 32);
        let bytes = duplicate.finish(32);
        let error = parse(&bytes).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::DuplicateMetadataKey);
    }

    #[test]
    fn validates_scalar_and_array_booleans() {
        let mut builder = GgufBuilder::new(3);
        builder.raw_entry("custom.bad_bool", 7, &[2]);
        assert_eq!(error_kind(&builder.finish(32)), GgufErrorKind::Malformed);

        let mut builder = GgufBuilder::new(3);
        builder.array("custom.bad_bools", 7, 2, &[0, 2]);
        assert_eq!(error_kind(&builder.finish(32)), GgufErrorKind::Malformed);
    }

    #[test]
    fn supports_nested_arrays_within_the_depth_limit() {
        let mut nested_strings = Vec::new();
        nested_strings.extend_from_slice(&8_u32.to_le_bytes());
        nested_strings.extend_from_slice(&2_u64.to_le_bytes());
        for value in ["a", "bb"] {
            nested_strings.extend_from_slice(&u64::try_from(value.len()).unwrap().to_le_bytes());
            nested_strings.extend_from_slice(value.as_bytes());
        }
        let mut nested_arrays = Vec::new();
        nested_arrays.extend_from_slice(&9_u32.to_le_bytes());
        nested_arrays.extend_from_slice(&1_u64.to_le_bytes());
        nested_arrays.extend_from_slice(&nested_strings);

        let mut builder = minimal_builder();
        builder.array("custom.nested", 9, 1, &nested_arrays);
        let bytes = builder.finish(32);
        assert!(parse(&bytes).is_ok());
    }

    #[test]
    fn enforces_nested_array_depth() {
        let mut nested = Vec::new();
        nested.extend_from_slice(&9_u32.to_le_bytes());
        nested.extend_from_slice(&1_u64.to_le_bytes());
        nested.extend_from_slice(&9_u32.to_le_bytes());
        nested.extend_from_slice(&1_u64.to_le_bytes());
        nested.extend_from_slice(&8_u32.to_le_bytes());
        nested.extend_from_slice(&0_u64.to_le_bytes());

        let mut builder = GgufBuilder::new(3);
        builder.array("custom.nested", 9, 1, &nested);
        let bytes = builder.finish(32);
        let limits = GgufLimits {
            array_nesting: 1,
            ..GgufLimits::default()
        };
        let error = parse_limited(&bytes, &limits).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::LimitExceeded);
    }

    #[test]
    fn rejects_unsupported_metadata_and_array_element_types() {
        let mut builder = GgufBuilder::new(3);
        builder.raw_entry("custom.unknown", 13, &[]);
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::UnsupportedMetadataType
        );

        let mut builder = GgufBuilder::new(3);
        builder.array("custom.unknown_elements", 13, 0, &[]);
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::UnsupportedMetadataType
        );
    }

    #[test]
    fn rejects_excessive_counts_before_allocating() {
        let mut bytes = minimal_file();
        bytes[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(error_kind(&bytes), GgufErrorKind::LimitExceeded);

        let mut bytes = minimal_file();
        bytes[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(error_kind(&bytes), GgufErrorKind::LimitExceeded);
    }

    #[test]
    fn rejects_oversized_strings_and_array_counts() {
        let mut builder = GgufBuilder::new(3);
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&(MAX_ENCODED_STRING_BYTES + 1).to_le_bytes());
        builder.raw_entry("general.name", 8, &encoded);
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::LimitExceeded
        );

        let mut builder = GgufBuilder::new(3);
        builder.array("custom.huge", 4, u64::MAX, &[]);
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::LimitExceeded
        );
    }

    #[test]
    fn warns_when_a_recognized_string_is_too_large_to_retain() {
        let oversized = "x".repeat(usize::try_from(super::MAX_RETAINED_STRING_BYTES + 1).unwrap());
        let mut builder = GgufBuilder::new(3);
        builder.string("general.architecture", &oversized);
        let metadata = parse(&builder.finish(32)).unwrap();
        assert_eq!(metadata.metadata_state, MetadataState::Incomplete);
        assert!(
            metadata
                .warnings
                .iter()
                .any(|warning| warning.code == WarningCode::OversizedMetadataValue)
        );
    }

    #[test]
    fn rejects_oversized_metadata_keys() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"GGUF");
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u64.to_le_bytes());
        bytes.extend_from_slice(&1_u64.to_le_bytes());
        bytes.extend_from_slice(&(MAX_METADATA_KEY_BYTES + 1).to_le_bytes());
        assert_eq!(error_kind(&bytes), GgufErrorKind::LimitExceeded);
    }

    #[test]
    fn enforces_total_metadata_key_and_tensor_name_budgets() {
        let mut builder = GgufBuilder::new(3);
        builder.string("abcd", "value");
        let limits = GgufLimits {
            total_metadata_key_bytes: 3,
            ..GgufLimits::default()
        };
        let error = parse_limited(&builder.finish(32), &limits).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::LimitExceeded);

        let mut builder = GgufBuilder::new(3);
        builder.tensor("aa", &[1], 0, 0);
        builder.tensor("bb", &[1], 0, 0);
        let limits = GgufLimits {
            total_tensor_name_bytes: 3,
            ..GgufLimits::default()
        };
        let error = parse_limited(&builder.finish(32), &limits).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::LimitExceeded);
    }

    #[test]
    fn enforces_the_distinct_tensor_type_budget() {
        let mut builder = GgufBuilder::new(3);
        builder.tensor("first", &[1], 0, 0);
        builder.tensor("second", &[1], 1, 0);
        let limits = GgufLimits {
            distinct_tensor_types: 1,
            ..GgufLimits::default()
        };
        let error = parse_limited(&builder.finish(32), &limits).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::LimitExceeded);
    }

    #[test]
    fn enforces_total_array_element_budget() {
        let mut nested_strings = Vec::new();
        nested_strings.extend_from_slice(&8_u32.to_le_bytes());
        nested_strings.extend_from_slice(&2_u64.to_le_bytes());
        nested_strings.extend_from_slice(&[0_u8; 16]);

        let mut builder = GgufBuilder::new(3);
        builder.array("custom.first", 9, 1, &nested_strings);
        builder.array("custom.second", 9, 1, &nested_strings);
        let bytes = builder.finish(32);
        let limits = GgufLimits {
            total_array_elements: 3,
            ..GgufLimits::default()
        };
        let error = parse_limited(&bytes, &limits).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::LimitExceeded);
    }

    #[test]
    fn default_alignment_is_reported_with_default_provenance() {
        let bytes = GgufBuilder::new(3).finish(32);
        let metadata = parse(&bytes).unwrap();
        assert_eq!(metadata.alignment.value, super::DEFAULT_GGUF_ALIGNMENT);
        assert_eq!(metadata.alignment.source, Provenance::Default);
    }

    #[test]
    fn rejects_invalid_alignment_values_and_types() {
        for alignment in [0_u32, 7, 12, 17] {
            let mut builder = GgufBuilder::new(3);
            builder.u32("general.alignment", alignment);
            assert_eq!(
                error_kind(&builder.finish(32)),
                GgufErrorKind::InvalidAlignment,
                "alignment {alignment}"
            );
        }

        let mut builder = GgufBuilder::new(3);
        builder.u64("general.alignment", 32);
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::InvalidAlignment
        );

        let mut builder = GgufBuilder::new(3);
        builder.f32_bits("general.alignment", 32.0_f32.to_bits());
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::InvalidAlignment
        );

        let mut builder = GgufBuilder::new(3);
        builder.string("general.alignment", "32");
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::InvalidAlignment
        );

        let mut builder = GgufBuilder::new(3);
        builder.boolean("general.alignment", true);
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::InvalidAlignment
        );

        let mut builder = GgufBuilder::new(3);
        builder.array("general.alignment", 4, 1, &[32_u8; 4]);
        assert_eq!(
            error_kind(&builder.finish(32)),
            GgufErrorKind::InvalidAlignment
        );
    }

    #[test]
    fn accepts_explicit_alignment_and_checks_tensor_offsets() {
        let mut builder = GgufBuilder::new(3);
        builder.u32("general.alignment", 64);
        builder.tensor("weight", &[4], 0, 0);
        let bytes = builder.finish_with_payload(64, &[0_u8; 64]);
        let metadata = parse(&bytes).unwrap();
        assert_eq!(metadata.alignment.value, 64);
        assert_eq!(metadata.alignment.source, Provenance::Explicit);

        let mut builder = GgufBuilder::new(3);
        builder.u32("general.alignment", 64);
        builder.tensor("weight", &[4], 0, 3);
        let error = parse(&builder.finish_with_payload(64, &[0_u8; 64])).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::InvalidTensor);
    }

    #[test]
    fn rejects_tensor_offsets_outside_the_data_region() {
        let mut builder = GgufBuilder::new(3);
        builder.tensor("weight", &[4], 0, 64);
        let error = parse(&builder.finish(32)).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::InvalidTensor);
    }

    #[test]
    fn rejects_duplicate_tensor_names() {
        let mut builder = GgufBuilder::new(3);
        builder.tensor("weight", &[4], 0, 0);
        builder.tensor("weight", &[4], 0, 0);
        let error = parse(&builder.finish_with_payload(32, &[0_u8; 32])).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::DuplicateTensorName);
    }

    #[test]
    fn rejects_invalid_tensor_dimensions() {
        let mut builder = GgufBuilder::new(3);
        builder.tensor("weight", &[1, 2, 3, 4, 5], 0, 0);
        assert_eq!(
            error_kind(&builder.finish_with_payload(32, &[0_u8; 32])),
            GgufErrorKind::InvalidTensor
        );

        let mut builder = GgufBuilder::new(3);
        builder.tensor("weight", &[u64::MAX], 0, 0);
        assert_eq!(
            error_kind(&builder.finish_with_payload(32, &[0_u8; 32])),
            GgufErrorKind::InvalidTensor
        );

        let mut builder = GgufBuilder::new(3);
        builder.tensor("weight", &[i64::MAX as u64, 2], 0, 0);
        assert_eq!(
            error_kind(&builder.finish_with_payload(32, &[0_u8; 32])),
            GgufErrorKind::InvalidTensor
        );
    }

    #[test]
    fn records_unknown_tensor_types_without_rejecting_the_file() {
        let mut builder = minimal_builder();
        builder.tensor("weight", &[4], 999, 0);
        let metadata = parse(&builder.finish_with_payload(32, &[0_u8; 32])).unwrap();
        assert_eq!(metadata.tensor_types[0].type_id, 999);
        assert_eq!(metadata.tensor_types[0].name, None);
        assert!(
            metadata
                .warnings
                .iter()
                .any(|warning| warning.code == WarningCode::UnknownTensorType)
        );
    }

    #[test]
    fn extracts_architecture_fields_regardless_of_metadata_order() {
        let mut builder = GgufBuilder::new(3);
        builder.u64("newarch.context_length", 4096);
        builder.u64("newarch.block_count", 24);
        builder.string("general.architecture", "newarch");
        builder.tensor("weight", &[4], 0, 0);
        let metadata = parse(&builder.finish_with_payload(32, &[0_u8; 32])).unwrap();
        assert_eq!(metadata.metadata_state, MetadataState::Available);
        assert_eq!(
            metadata.architecture_fields.context_length.unwrap().value,
            4096
        );
        assert_eq!(metadata.architecture_fields.block_count.unwrap().value, 24);
    }

    #[test]
    fn catalogs_unknown_architectures_and_flags_missing_architecture() {
        let mut builder = GgufBuilder::new(3);
        builder.string("general.architecture", "brand_new_arch");
        let metadata = parse(&builder.finish(32)).unwrap();
        assert_eq!(metadata.metadata_state, MetadataState::Available);
        assert_eq!(
            metadata.general.architecture.unwrap().value,
            "brand_new_arch"
        );

        let metadata = parse(&GgufBuilder::new(3).finish(32)).unwrap();
        assert_eq!(metadata.metadata_state, MetadataState::Incomplete);
        assert!(
            metadata
                .warnings
                .iter()
                .any(|warning| warning.code == WarningCode::MissingArchitecture)
        );
    }

    #[test]
    fn warns_about_wrong_tokenizer_array_element_types() {
        let mut builder = GgufBuilder::new(3);
        builder.string("general.architecture", "llama");
        builder.array("tokenizer.ggml.tokens", 4, 1, &[0_u8; 4]);
        let metadata = parse(&builder.finish(32)).unwrap();
        assert_eq!(
            metadata.tokenizer.tokens.as_ref().unwrap().element_type,
            MetadataValueType::Uint32
        );
        assert!(
            metadata
                .warnings
                .iter()
                .any(|warning| warning.code == WarningCode::UnexpectedMetadataType)
        );
    }

    #[test]
    fn accepts_upstream_added_token_string_arrays() {
        let mut builder = minimal_builder();
        builder.string_array("tokenizer.ggml.added_tokens", &["<extra_a>", "<extra_b>"]);
        let metadata = parse(&builder.finish(32)).unwrap();
        let added_tokens = metadata.tokenizer.added_tokens.as_ref().unwrap();
        assert_eq!(added_tokens.count, 2);
        assert_eq!(added_tokens.element_type, MetadataValueType::String);
        assert!(
            !metadata
                .warnings
                .iter()
                .any(|warning| warning.code == WarningCode::UnexpectedMetadataType)
        );
    }

    #[test]
    fn warns_about_wrong_types_in_recognized_keys() {
        let mut builder = GgufBuilder::new(3);
        builder.u32("general.architecture", 7);
        builder.string("llama.block_count", "many");
        let metadata = parse(&builder.finish(32)).unwrap();
        assert_eq!(metadata.metadata_state, MetadataState::Incomplete);
        assert_eq!(
            metadata
                .warnings
                .iter()
                .filter(|warning| warning.code == WarningCode::UnexpectedMetadataType)
                .count(),
            2
        );
    }

    #[test]
    fn records_presence_of_wrongly_typed_presence_only_keys() {
        let mut builder = minimal_builder();
        builder.u32("tokenizer.chat_template", 1);
        let metadata = parse(&builder.finish(32)).unwrap();
        assert!(metadata.tokenizer.chat_template_present);
        assert!(
            metadata
                .warnings
                .iter()
                .any(|warning| warning.code == WarningCode::UnexpectedMetadataType)
        );
    }

    #[test]
    fn aligns_the_tensor_data_region() {
        let mut builder = GgufBuilder::new(3);
        builder.u32("general.alignment", 64);
        builder.string("general.architecture", "llama");
        builder.tensor("weight", &[4], 0, 0);
        let bytes = builder.finish_with_payload(64, &[0_u8; 16]);
        let metadata = parse(&bytes).unwrap();
        assert_eq!(metadata.tensor_data_offset.as_u64() % 64, 0);
        assert_eq!(metadata.tensor_data_bytes.as_u64(), 16);
    }

    #[test]
    fn every_prefix_of_a_valid_header_is_rejected_without_panicking() {
        let bytes = minimal_file();
        for length in 0..bytes.len() {
            let prefix = &bytes[..length];
            let mut cursor = Cursor::new(prefix);
            let result = parse_gguf(
                &mut cursor,
                u64::try_from(prefix.len()).expect("prefix length fits u64"),
            );
            assert!(
                result.is_err(),
                "prefix length {length} unexpectedly parsed"
            );
        }
    }

    struct ReadFailureAfter {
        inner: Cursor<Vec<u8>>,
        fail_at: u64,
    }

    impl std::io::Read for ReadFailureAfter {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let position = self.inner.position();
            if position >= self.fail_at {
                return Err(std::io::Error::other("synthetic parser read failure"));
            }
            let remaining = usize::try_from(self.fail_at - position).unwrap_or(usize::MAX);
            let allowed = buffer.len().min(remaining);
            self.inner.read(&mut buffer[..allowed])
        }
    }

    impl std::io::Seek for ReadFailureAfter {
        fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(position)
        }
    }

    #[test]
    fn reports_io_read_failures_without_panicking() {
        let bytes = minimal_file();
        let file_size = u64::try_from(bytes.len()).unwrap();
        let mut reader = ReadFailureAfter {
            inner: Cursor::new(bytes),
            fail_at: 30,
        };
        let error = parse_gguf(&mut reader, file_size).unwrap_err();
        assert_eq!(error.kind(), GgufErrorKind::Io);
        assert!(error.message().contains("synthetic parser read failure"));
    }

    #[test]
    fn unsupported_status_is_available_for_catalog_mapping() {
        // The status enum is a public contract; this assertion catches accidental
        // renaming during schema review.
        assert_eq!(
            serde_json::to_string(&ParseStatus::Unsupported).unwrap(),
            "\"unsupported\""
        );
    }
}
