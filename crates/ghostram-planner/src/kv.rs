//! Uniform full-attention KV-cache estimation for the supported `llama` profile.
//!
//! The formula is deliberately narrow. It models independent sequences with a
//! full requested context on every transformer layer and does not model sliding
//! windows, latent attention, variable per-layer dimensions, backend padding, or
//! workspace allocations.

use ghostram_core::quantity::Bytes;
use ghostram_models::{
    ArchitectureFields, ArchitectureKey, AttentionLayoutKey, Field, ParseStatus,
};
use serde::{Deserialize, Serialize};

use crate::evidence::Evidence;

/// Identifier of the only architecture profile supported by this planner.
pub const LLAMA_PROFILE_ID: &str = "llama_uniform_full_attention_v1";
/// Architecture name accepted by the profile.
pub const LLAMA_ARCHITECTURE: &str = "llama";
/// Formula identifier recorded in the plan.
pub const KV_FORMULA_ID: &str = "uniform_kv_cache_v1";

/// KV-cache element types modeled by this planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KvCacheType {
    /// IEEE 754 binary16, two bytes per element.
    F16,
    /// IEEE 754 binary32, four bytes per element.
    F32,
}

impl KvCacheType {
    /// Returns the element size in bytes.
    #[must_use]
    pub const fn bytes_per_element(self) -> u64 {
        match self {
            Self::F16 => 2,
            Self::F32 => 4,
        }
    }

    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::F16 => "f16",
            Self::F32 => "f32",
        }
    }
}

/// How the KV-cache estimate was produced, or why it is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KvMethod {
    /// Uniform full-attention formula.
    UniformFullAttention,
    /// No estimate was produced.
    Unavailable,
}

impl KvMethod {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UniformFullAttention => "uniform_full_attention",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Stable reasons why a KV-cache estimate is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KvUnavailableCode {
    /// The file was not parsed as a supported GGUF structure.
    ModelNotParsed,
    /// `general.architecture` is absent or empty.
    MissingArchitecture,
    /// The architecture is outside the supported profile.
    UnsupportedArchitecture,
    /// Attention-layout metadata outside the uniform profile is present.
    UnsupportedAttentionLayout,
    /// `block_count` is absent.
    MissingLayerCount,
    /// `block_count` is present but not a usable scalar.
    UnusableLayerCount,
    /// `block_count` is zero.
    ZeroLayerCount,
    /// `attention.head_count` is absent.
    MissingHeadCount,
    /// `attention.head_count` is present but not a usable scalar.
    UnusableHeadCount,
    /// `attention.head_count` is zero.
    ZeroHeadCount,
    /// `embedding_length` is absent.
    MissingEmbeddingLength,
    /// `embedding_length` is present but not a usable scalar.
    UnusableEmbeddingLength,
    /// `embedding_length` is zero.
    ZeroEmbeddingLength,
    /// `attention.head_count_kv` is present but not a usable scalar.
    UnusableKvHeadCount,
    /// `attention.head_count_kv` is zero.
    ZeroKvHeadCount,
    /// More KV heads than query heads were declared.
    KvHeadsExceedHeadCount,
    /// Head count is not divisible by the KV-head count.
    HeadCountNotDivisibleByKvHeads,
    /// `attention.key_length` is present but not a usable scalar.
    UnusableKeyLength,
    /// `attention.value_length` is present but not a usable scalar.
    UnusableValueLength,
    /// The derived or explicit key width is zero.
    ZeroKeyDimension,
    /// The derived or explicit value width is zero.
    ZeroValueDimension,
    /// The default head width cannot be derived from embedding and head count.
    HeadDimensionUnavailable,
    /// The requested context is zero.
    ZeroContextTokens,
    /// The requested sequence count is zero.
    ZeroSequences,
    /// Checked arithmetic overflowed a 64-bit byte or element count.
    ArithmeticOverflow,
}

impl KvUnavailableCode {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModelNotParsed => "model_not_parsed",
            Self::MissingArchitecture => "missing_architecture",
            Self::UnsupportedArchitecture => "unsupported_architecture",
            Self::UnsupportedAttentionLayout => "unsupported_attention_layout",
            Self::MissingLayerCount => "missing_layer_count",
            Self::UnusableLayerCount => "unusable_layer_count",
            Self::ZeroLayerCount => "zero_layer_count",
            Self::MissingHeadCount => "missing_head_count",
            Self::UnusableHeadCount => "unusable_head_count",
            Self::ZeroHeadCount => "zero_head_count",
            Self::MissingEmbeddingLength => "missing_embedding_length",
            Self::UnusableEmbeddingLength => "unusable_embedding_length",
            Self::ZeroEmbeddingLength => "zero_embedding_length",
            Self::UnusableKvHeadCount => "unusable_kv_head_count",
            Self::ZeroKvHeadCount => "zero_kv_head_count",
            Self::KvHeadsExceedHeadCount => "kv_heads_exceed_head_count",
            Self::HeadCountNotDivisibleByKvHeads => "head_count_not_divisible_by_kv_heads",
            Self::UnusableKeyLength => "unusable_key_length",
            Self::UnusableValueLength => "unusable_value_length",
            Self::ZeroKeyDimension => "zero_key_dimension",
            Self::ZeroValueDimension => "zero_value_dimension",
            Self::HeadDimensionUnavailable => "head_dimension_unavailable",
            Self::ZeroContextTokens => "zero_context_tokens",
            Self::ZeroSequences => "zero_sequences",
            Self::ArithmeticOverflow => "arithmetic_overflow",
        }
    }
}

/// A typed reason why the KV-cache estimate is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvUnavailable {
    /// Stable reason category.
    pub code: KvUnavailableCode,
    /// Human-readable detail.
    pub message: String,
}

/// Dimensions used by the uniform KV-cache formula.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvDimensions {
    /// Requested context length per sequence.
    pub context_tokens: u64,
    /// Number of independent sequences.
    pub sequences: u64,
    /// Transformer layers with K and V caches.
    pub layers: Field<u64>,
    /// Query attention heads.
    pub head_count: Field<u64>,
    /// Key/value attention heads.
    pub kv_heads: Field<u64>,
    /// Key width in elements.
    pub key_dimension: Field<u64>,
    /// Value width in elements.
    pub value_dimension: Field<u64>,
    /// Cache element type.
    pub cache_type: KvCacheType,
    /// Total K and V elements across all layers, sequences, and context tokens.
    pub elements: u64,
}

impl KvDimensions {
    /// Returns bytes of KV cache added by one context token across all requested
    /// sequences, or `None` on overflow.
    #[must_use]
    pub fn per_token_bytes(&self) -> Option<u64> {
        self.layers
            .value
            .checked_mul(self.kv_heads.value)?
            .checked_mul(
                self.key_dimension
                    .value
                    .checked_add(self.value_dimension.value)?,
            )?
            .checked_mul(self.cache_type.bytes_per_element())?
            .checked_mul(self.sequences)
    }
}

/// Estimated KV-cache component and its audit dimensions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvCacheComponent {
    /// Estimated bytes, or an unknown quantity with a reason.
    pub estimate: Evidence<Bytes>,
    /// Calculation method.
    pub method: KvMethod,
    /// Profile identifier when the architecture was recognized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Dimensions used by the calculation, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<KvDimensions>,
    /// Typed reason when no estimate was produced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<KvUnavailable>,
}

impl KvCacheComponent {
    /// Returns whether a numeric estimate is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.method == KvMethod::UniformFullAttention
    }

    /// Returns the audit dimensions, when available.
    #[must_use]
    pub fn dimensions(&self) -> Option<&KvDimensions> {
        self.dimensions.as_ref()
    }
}

/// Estimates a KV cache for the supported uniform full-attention `llama`
/// profile.
///
/// The estimate assumes every transformer layer holds K and V caches for every
/// requested sequence at the full requested context. It does not model sliding
/// windows, latent attention, variable per-layer dimensions, padding, alignment,
/// or backend workspace.
#[must_use]
pub fn estimate_kv_cache(
    parse_status: ParseStatus,
    architecture: Option<&str>,
    fields: &ArchitectureFields,
    context_tokens: u64,
    sequences: u64,
    cache_type: KvCacheType,
) -> KvCacheComponent {
    if parse_status != ParseStatus::Parsed {
        return unavailable(
            KvUnavailableCode::ModelNotParsed,
            "the selected file was not parsed as a supported GGUF structure, so KV-cache dimensions are unavailable",
            None,
        );
    }
    let Some(architecture) = architecture else {
        return unavailable(
            KvUnavailableCode::MissingArchitecture,
            "general.architecture is absent or empty",
            None,
        );
    };
    if architecture != LLAMA_ARCHITECTURE {
        return unavailable(
            KvUnavailableCode::UnsupportedArchitecture,
            format!(
                "architecture {} is outside the supported profile; only uniform full-attention `llama` models are modeled",
                quoted(architecture)
            ),
            None,
        );
    }
    let profile = Some(LLAMA_PROFILE_ID.to_string());
    if !fields.attention_layout_keys.is_empty() {
        let keys = fields
            .attention_layout_keys
            .iter()
            .map(|key| layout_key_name(*key))
            .collect::<Vec<_>>()
            .join(", ");
        return unavailable(
            KvUnavailableCode::UnsupportedAttentionLayout,
            format!(
                "the model declares attention-layout metadata outside the uniform full-attention profile: {keys}"
            ),
            profile,
        );
    }
    if context_tokens == 0 {
        return unavailable(
            KvUnavailableCode::ZeroContextTokens,
            "the requested context length must be positive",
            profile,
        );
    }
    if sequences == 0 {
        return unavailable(
            KvUnavailableCode::ZeroSequences,
            "the requested sequence count must be positive",
            profile,
        );
    }

    let Some(layers) = fields.block_count.clone() else {
        return unavailable(
            if field_present(fields, ArchitectureKey::BlockCount) {
                KvUnavailableCode::UnusableLayerCount
            } else {
                KvUnavailableCode::MissingLayerCount
            },
            layer_message(fields),
            profile,
        );
    };
    if layers.value == 0 {
        return unavailable(
            KvUnavailableCode::ZeroLayerCount,
            "`llama.block_count` is zero",
            profile,
        );
    }

    let Some(head_count) = fields.attention_head_count.clone() else {
        return unavailable(
            if field_present(fields, ArchitectureKey::AttentionHeadCount) {
                KvUnavailableCode::UnusableHeadCount
            } else {
                KvUnavailableCode::MissingHeadCount
            },
            head_count_message(fields),
            profile,
        );
    };
    if head_count.value == 0 {
        return unavailable(
            KvUnavailableCode::ZeroHeadCount,
            "`llama.attention.head_count` is zero",
            profile,
        );
    }

    let Some(embedding_length) = fields.embedding_length.clone() else {
        return unavailable(
            if field_present(fields, ArchitectureKey::EmbeddingLength) {
                KvUnavailableCode::UnusableEmbeddingLength
            } else {
                KvUnavailableCode::MissingEmbeddingLength
            },
            embedding_message(fields),
            profile,
        );
    };
    if embedding_length.value == 0 {
        return unavailable(
            KvUnavailableCode::ZeroEmbeddingLength,
            "`llama.embedding_length` is zero",
            profile,
        );
    }

    let kv_heads = match fields.attention_head_count_kv.clone() {
        Some(value) => value,
        None if field_present(fields, ArchitectureKey::AttentionHeadCountKv) => {
            return unavailable(
                KvUnavailableCode::UnusableKvHeadCount,
                "`llama.attention.head_count_kv` is present but was not a usable unsigned scalar; per-layer arrays and other layouts are not modeled",
                profile,
            );
        }
        None => Field::defaulted(head_count.value),
    };
    if kv_heads.value == 0 {
        return unavailable(
            KvUnavailableCode::ZeroKvHeadCount,
            "`llama.attention.head_count_kv` is zero",
            profile,
        );
    }
    if kv_heads.value > head_count.value {
        return unavailable(
            KvUnavailableCode::KvHeadsExceedHeadCount,
            format!(
                "KV head count {} exceeds query head count {}",
                kv_heads.value, head_count.value
            ),
            profile,
        );
    }
    if head_count.value % kv_heads.value != 0 {
        return unavailable(
            KvUnavailableCode::HeadCountNotDivisibleByKvHeads,
            format!(
                "query head count {} is not divisible by KV head count {}",
                head_count.value, kv_heads.value
            ),
            profile,
        );
    }

    let key_dimension = match fields.attention_key_length.clone() {
        Some(value) => value,
        None if field_present(fields, ArchitectureKey::AttentionKeyLength) => {
            return unavailable(
                KvUnavailableCode::UnusableKeyLength,
                "`llama.attention.key_length` is present but was not a usable unsigned scalar",
                profile,
            );
        }
        None => match default_head_dimension("key", embedding_length.value, head_count.value) {
            Ok(value) => value,
            Err((code, message)) => return unavailable(code, message, profile),
        },
    };
    let value_dimension = match fields.attention_value_length.clone() {
        Some(value) => value,
        None if field_present(fields, ArchitectureKey::AttentionValueLength) => {
            return unavailable(
                KvUnavailableCode::UnusableValueLength,
                "`llama.attention.value_length` is present but was not a usable unsigned scalar",
                profile,
            );
        }
        None => match default_head_dimension("value", embedding_length.value, head_count.value) {
            Ok(value) => value,
            Err((code, message)) => return unavailable(code, message, profile),
        },
    };
    if key_dimension.value == 0 {
        return unavailable(
            KvUnavailableCode::ZeroKeyDimension,
            "the key width is zero",
            profile,
        );
    }
    if value_dimension.value == 0 {
        return unavailable(
            KvUnavailableCode::ZeroValueDimension,
            "the value width is zero",
            profile,
        );
    }

    let Some(elements) = checked_elements(
        context_tokens,
        layers.value,
        kv_heads.value,
        key_dimension.value,
        value_dimension.value,
        sequences,
    ) else {
        return unavailable(
            KvUnavailableCode::ArithmeticOverflow,
            "checked KV-cache element arithmetic overflowed u64",
            profile,
        );
    };
    let Some(bytes) = elements.checked_mul(cache_type.bytes_per_element()) else {
        return unavailable(
            KvUnavailableCode::ArithmeticOverflow,
            "checked KV-cache byte arithmetic overflowed u64",
            profile,
        );
    };

    let dimensions = KvDimensions {
        context_tokens,
        sequences,
        layers,
        head_count,
        kv_heads,
        key_dimension,
        value_dimension,
        cache_type,
        elements,
    };
    KvCacheComponent {
        estimate: Evidence::estimated(
            Bytes::new(bytes),
            "uniform full-attention KV formula; backend padding, alignment, sliding windows, and workspace are not modeled",
        ),
        method: KvMethod::UniformFullAttention,
        profile,
        dimensions: Some(dimensions),
        unavailable: None,
    }
}

fn checked_elements(
    context_tokens: u64,
    layers: u64,
    kv_heads: u64,
    key_dimension: u64,
    value_dimension: u64,
    sequences: u64,
) -> Option<u64> {
    context_tokens
        .checked_mul(layers)?
        .checked_mul(kv_heads)?
        .checked_mul(key_dimension.checked_add(value_dimension)?)?
        .checked_mul(sequences)
}

fn default_head_dimension(
    name: &str,
    embedding_length: u64,
    head_count: u64,
) -> Result<Field<u64>, (KvUnavailableCode, String)> {
    if embedding_length % head_count != 0 {
        return Err((
            KvUnavailableCode::HeadDimensionUnavailable,
            format!(
                "`llama.embedding_length` ({embedding_length}) is not divisible by \
                 `llama.attention.head_count` ({head_count}), so the default {name} width is unavailable"
            ),
        ));
    }
    let dimension = embedding_length / head_count;
    if dimension == 0 {
        return Err((
            if name == "key" {
                KvUnavailableCode::ZeroKeyDimension
            } else {
                KvUnavailableCode::ZeroValueDimension
            },
            format!("the default {name} width is zero"),
        ));
    }
    Ok(Field::defaulted(dimension))
}

fn unavailable(
    code: KvUnavailableCode,
    message: impl Into<String>,
    profile: Option<String>,
) -> KvCacheComponent {
    let message = message.into();
    KvCacheComponent {
        estimate: Evidence::unknown(message.clone()),
        method: KvMethod::Unavailable,
        profile,
        dimensions: None,
        unavailable: Some(KvUnavailable { code, message }),
    }
}

fn field_present(fields: &ArchitectureFields, key: ArchitectureKey) -> bool {
    fields.present_keys.contains(&key)
}

fn layer_message(fields: &ArchitectureFields) -> &'static str {
    if field_present(fields, ArchitectureKey::BlockCount) {
        "`llama.block_count` is present but was not a usable unsigned scalar; per-layer arrays and other layouts are not modeled"
    } else {
        "`llama.block_count` is absent"
    }
}

fn head_count_message(fields: &ArchitectureFields) -> &'static str {
    if field_present(fields, ArchitectureKey::AttentionHeadCount) {
        "`llama.attention.head_count` is present but was not a usable unsigned scalar; per-layer arrays and other layouts are not modeled"
    } else {
        "`llama.attention.head_count` is absent"
    }
}

fn embedding_message(fields: &ArchitectureFields) -> &'static str {
    if field_present(fields, ArchitectureKey::EmbeddingLength) {
        "`llama.embedding_length` is present but was not a usable unsigned scalar"
    } else {
        "`llama.embedding_length` is absent"
    }
}

pub(crate) fn layout_key_name(key: AttentionLayoutKey) -> &'static str {
    match key {
        AttentionLayoutKey::SlidingWindow => "attention.sliding_window",
        AttentionLayoutKey::SlidingWindowPattern => "attention.sliding_window_pattern",
        AttentionLayoutKey::KeyLengthSwa => "attention.key_length_swa",
        AttentionLayoutKey::ValueLengthSwa => "attention.value_length_swa",
    }
}

fn quoted(value: &str) -> String {
    const MAX_DISPLAY: usize = 120;
    if value.len() <= MAX_DISPLAY {
        return format!("{value:?}");
    }
    let mut end = MAX_DISPLAY;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{:?}...", &value[..end])
}

#[cfg(test)]
mod tests {
    use ghostram_models::{
        ArchitectureFields, ArchitectureKey, AttentionLayoutKey, Field, ParseStatus, Provenance,
    };

    use super::{KvCacheType, KvUnavailableCode, estimate_kv_cache};
    use ghostram_core::quantity::Bytes;

    fn base_fields() -> ArchitectureFields {
        ArchitectureFields {
            block_count: Some(Field::explicit(32)),
            embedding_length: Some(Field::explicit(4096)),
            attention_head_count: Some(Field::explicit(32)),
            attention_head_count_kv: Some(Field::explicit(8)),
            present_keys: vec![
                ArchitectureKey::BlockCount,
                ArchitectureKey::EmbeddingLength,
                ArchitectureKey::AttentionHeadCount,
                ArchitectureKey::AttentionHeadCountKv,
            ],
            ..ArchitectureFields::default()
        }
    }

    fn estimate(
        fields: &ArchitectureFields,
        context: u64,
        sequences: u64,
    ) -> super::KvCacheComponent {
        estimate_kv_cache(
            ParseStatus::Parsed,
            Some("llama"),
            fields,
            context,
            sequences,
            KvCacheType::F16,
        )
    }

    #[test]
    fn computes_the_uniform_f16_and_f32_formula() {
        let fields = base_fields();
        let f16 = estimate(&fields, 4096, 1);
        assert_eq!(f16.estimate.value(), Some(&Bytes::new(536_870_912)));
        assert_eq!(
            f16.estimate.class(),
            crate::evidence::EvidenceClass::Estimated
        );
        let dimensions = f16.dimensions().unwrap();
        assert_eq!(dimensions.elements, 268_435_456);
        assert_eq!(dimensions.key_dimension.value, 128);
        assert_eq!(dimensions.key_dimension.source, Provenance::Default);
        assert_eq!(dimensions.value_dimension.value, 128);
        assert_eq!(dimensions.value_dimension.source, Provenance::Default);
        assert_eq!(dimensions.per_token_bytes(), Some(131_072));

        let f32 = estimate_kv_cache(
            ParseStatus::Parsed,
            Some("llama"),
            &fields,
            4096,
            1,
            KvCacheType::F32,
        );
        assert_eq!(f32.estimate.value(), Some(&Bytes::new(1_073_741_824)));
    }

    #[test]
    fn defaults_missing_kv_heads_to_the_query_head_count() {
        let mut fields = base_fields();
        fields.attention_head_count_kv = None;
        fields
            .present_keys
            .retain(|key| !matches!(key, ArchitectureKey::AttentionHeadCountKv));
        let component = estimate(&fields, 1, 1);
        let dimensions = component.dimensions().unwrap();
        assert_eq!(dimensions.kv_heads.value, 32);
        assert_eq!(dimensions.kv_heads.source, Provenance::Default);
        assert_eq!(dimensions.elements, 32 * 32 * (128 + 128));
    }

    #[test]
    fn uses_explicit_key_and_value_widths() {
        let mut fields = base_fields();
        fields.attention_key_length = Some(Field::explicit(64));
        fields.attention_value_length = Some(Field::explicit(64));
        fields
            .present_keys
            .push(ArchitectureKey::AttentionKeyLength);
        fields
            .present_keys
            .push(ArchitectureKey::AttentionValueLength);
        let component = estimate(&fields, 4096, 1);
        let dimensions = component.dimensions().unwrap();
        assert_eq!(dimensions.key_dimension.value, 64);
        assert_eq!(dimensions.key_dimension.source, Provenance::Explicit);
        assert_eq!(dimensions.value_dimension.value, 64);
        assert_eq!(component.estimate.value(), Some(&Bytes::new(268_435_456)));
    }

    #[test]
    fn multiplies_by_explicit_sequences() {
        let fields = base_fields();
        let one = estimate(&fields, 4096, 1);
        let two = estimate(&fields, 4096, 2);
        assert_eq!(
            two.estimate.value().unwrap().as_u64(),
            one.estimate.value().unwrap().as_u64() * 2
        );
    }

    #[test]
    fn rejects_unusable_or_missing_dimensions_without_substituting_zero() {
        let cases: Vec<(ArchitectureFields, KvUnavailableCode)> = vec![
            (
                ArchitectureFields {
                    block_count: None,
                    present_keys: vec![ArchitectureKey::BlockCount],
                    ..base_fields()
                },
                KvUnavailableCode::UnusableLayerCount,
            ),
            (
                ArchitectureFields {
                    block_count: None,
                    present_keys: vec![],
                    ..base_fields()
                },
                KvUnavailableCode::MissingLayerCount,
            ),
            (
                ArchitectureFields {
                    block_count: Some(Field::explicit(0)),
                    ..base_fields()
                },
                KvUnavailableCode::ZeroLayerCount,
            ),
            (
                ArchitectureFields {
                    attention_head_count: None,
                    present_keys: vec![ArchitectureKey::AttentionHeadCount],
                    ..base_fields()
                },
                KvUnavailableCode::UnusableHeadCount,
            ),
            (
                ArchitectureFields {
                    attention_head_count: Some(Field::explicit(0)),
                    ..base_fields()
                },
                KvUnavailableCode::ZeroHeadCount,
            ),
            (
                ArchitectureFields {
                    embedding_length: None,
                    present_keys: vec![ArchitectureKey::EmbeddingLength],
                    ..base_fields()
                },
                KvUnavailableCode::UnusableEmbeddingLength,
            ),
            (
                ArchitectureFields {
                    attention_head_count_kv: None,
                    present_keys: vec![ArchitectureKey::AttentionHeadCountKv],
                    ..base_fields()
                },
                KvUnavailableCode::UnusableKvHeadCount,
            ),
            (
                ArchitectureFields {
                    attention_head_count_kv: Some(Field::explicit(0)),
                    ..base_fields()
                },
                KvUnavailableCode::ZeroKvHeadCount,
            ),
            (
                ArchitectureFields {
                    attention_head_count_kv: Some(Field::explicit(64)),
                    ..base_fields()
                },
                KvUnavailableCode::KvHeadsExceedHeadCount,
            ),
            (
                ArchitectureFields {
                    attention_head_count: Some(Field::explicit(10)),
                    attention_head_count_kv: Some(Field::explicit(3)),
                    ..base_fields()
                },
                KvUnavailableCode::HeadCountNotDivisibleByKvHeads,
            ),
            (
                ArchitectureFields {
                    embedding_length: Some(Field::explicit(4095)),
                    ..base_fields()
                },
                KvUnavailableCode::HeadDimensionUnavailable,
            ),
            (
                ArchitectureFields {
                    attention_key_length: Some(Field::explicit(0)),
                    present_keys: vec![ArchitectureKey::AttentionKeyLength],
                    ..base_fields()
                },
                KvUnavailableCode::ZeroKeyDimension,
            ),
            (
                ArchitectureFields {
                    attention_value_length: Some(Field::explicit(0)),
                    present_keys: vec![ArchitectureKey::AttentionValueLength],
                    ..base_fields()
                },
                KvUnavailableCode::ZeroValueDimension,
            ),
        ];
        for (fields, expected) in cases {
            let component = estimate(&fields, 4096, 1);
            assert!(
                !component.is_available(),
                "expected unavailable for {expected:?}"
            );
            assert_eq!(component.unavailable.as_ref().unwrap().code, expected);
            assert!(component.estimate.value().is_none());
        }
    }

    #[test]
    fn rejects_unsupported_architectures_and_layouts() {
        let fields = base_fields();
        let unsupported = estimate_kv_cache(
            ParseStatus::Parsed,
            Some("qwen2"),
            &fields,
            4096,
            1,
            KvCacheType::F16,
        );
        assert_eq!(
            unsupported.unavailable.as_ref().unwrap().code,
            KvUnavailableCode::UnsupportedArchitecture
        );
        assert!(unsupported.profile.is_none());

        let mut sliding = base_fields();
        sliding.attention_layout_keys = vec![AttentionLayoutKey::SlidingWindow];
        let component = estimate(&sliding, 4096, 1);
        assert_eq!(
            component.unavailable.as_ref().unwrap().code,
            KvUnavailableCode::UnsupportedAttentionLayout
        );
        assert!(component.profile.is_some());
    }

    #[test]
    fn rejects_unparsed_models_and_zero_options() {
        let fields = base_fields();
        let unparsed = estimate_kv_cache(
            ParseStatus::Malformed,
            Some("llama"),
            &fields,
            4096,
            1,
            KvCacheType::F16,
        );
        assert_eq!(
            unparsed.unavailable.as_ref().unwrap().code,
            KvUnavailableCode::ModelNotParsed
        );

        let no_arch = estimate_kv_cache(
            ParseStatus::Parsed,
            None,
            &fields,
            4096,
            1,
            KvCacheType::F16,
        );
        assert_eq!(
            no_arch.unavailable.as_ref().unwrap().code,
            KvUnavailableCode::MissingArchitecture
        );

        assert_eq!(
            estimate(&fields, 0, 1).unavailable.as_ref().unwrap().code,
            KvUnavailableCode::ZeroContextTokens
        );
        assert_eq!(
            estimate(&fields, 1, 0).unavailable.as_ref().unwrap().code,
            KvUnavailableCode::ZeroSequences
        );
    }

    #[test]
    fn reports_arithmetic_overflow_without_panicking() {
        let mut fields = base_fields();
        fields.block_count = Some(Field::explicit(u64::MAX));
        let component = estimate(&fields, 4096, 1);
        assert_eq!(
            component.unavailable.as_ref().unwrap().code,
            KvUnavailableCode::ArithmeticOverflow
        );
    }

    #[test]
    fn component_round_trips_through_json() {
        let component = estimate(&base_fields(), 4096, 1);
        let encoded = serde_json::to_string(&component).unwrap();
        let decoded: super::KvCacheComponent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, component);
    }
}
