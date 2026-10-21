//! Typed evidence wrappers for planner quantities and options.
//!
//! A value is never presented without its evidence class. Unknown quantities
//! carry a reason and no value, so missing data cannot be mistaken for zero.

use serde::{Deserialize, Serialize};

/// How a planner value was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    /// A current system or file observation.
    Observed,
    /// A value supplied by the caller or the command line.
    ExplicitInput,
    /// A value calculated exactly from other known values.
    Derived,
    /// A value based on a proxy, assumption, or architecture-specific model.
    Estimated,
    /// A documented product-policy default chosen by this build.
    ProductDefault,
    /// No usable value is available.
    Unknown,
}

impl EvidenceClass {
    /// Returns the stable schema name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::ExplicitInput => "explicit_input",
            Self::Derived => "derived",
            Self::Estimated => "estimated",
            Self::ProductDefault => "product_default",
            Self::Unknown => "unknown",
        }
    }
}

/// A value together with the evidence that justifies it.
///
/// The `Unknown` variant has no value field, which makes a missing quantity
/// structurally distinct from a measured or calculated zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "evidence", rename_all = "snake_case")]
pub enum Evidence<T> {
    /// A current observation.
    Observed {
        /// Observed value.
        value: T,
    },
    /// A caller-supplied value.
    ExplicitInput {
        /// Supplied value.
        value: T,
    },
    /// A value calculated from other known values.
    Derived {
        /// Calculated value.
        value: T,
        /// Calculation or derivation context.
        reason: String,
    },
    /// A proxy or assumption-based value.
    Estimated {
        /// Estimated value.
        value: T,
        /// Why the value is an estimate and what it excludes.
        reason: String,
    },
    /// A documented product-policy default.
    ProductDefault {
        /// Default value.
        value: T,
        /// Why this build chose the default.
        reason: String,
    },
    /// An unavailable value with no numeric substitute.
    Unknown {
        /// Why no usable value is available.
        reason: String,
    },
}

impl<T> Evidence<T> {
    /// Creates an observed value.
    #[must_use]
    pub const fn observed(value: T) -> Self {
        Self::Observed { value }
    }

    /// Creates a caller-supplied value.
    #[must_use]
    pub const fn explicit_input(value: T) -> Self {
        Self::ExplicitInput { value }
    }

    /// Creates a calculated value with an explanation.
    #[must_use]
    pub fn derived(value: T, reason: impl Into<String>) -> Self {
        Self::Derived {
            value,
            reason: reason.into(),
        }
    }

    /// Creates an estimated value with an explanation.
    #[must_use]
    pub fn estimated(value: T, reason: impl Into<String>) -> Self {
        Self::Estimated {
            value,
            reason: reason.into(),
        }
    }

    /// Creates a product-policy default with an explanation.
    #[must_use]
    pub fn product_default(value: T, reason: impl Into<String>) -> Self {
        Self::ProductDefault {
            value,
            reason: reason.into(),
        }
    }

    /// Creates an unknown value with an explanation.
    #[must_use]
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self::Unknown {
            reason: reason.into(),
        }
    }

    /// Returns the evidence class.
    #[must_use]
    pub const fn class(&self) -> EvidenceClass {
        match self {
            Self::Observed { .. } => EvidenceClass::Observed,
            Self::ExplicitInput { .. } => EvidenceClass::ExplicitInput,
            Self::Derived { .. } => EvidenceClass::Derived,
            Self::Estimated { .. } => EvidenceClass::Estimated,
            Self::ProductDefault { .. } => EvidenceClass::ProductDefault,
            Self::Unknown { .. } => EvidenceClass::Unknown,
        }
    }

    /// Returns the value, or `None` when the quantity is unknown.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Observed { value }
            | Self::ExplicitInput { value }
            | Self::Derived { value, .. }
            | Self::Estimated { value, .. }
            | Self::ProductDefault { value, .. } => Some(value),
            Self::Unknown { .. } => None,
        }
    }

    /// Returns the explanation, when present.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Derived { reason, .. }
            | Self::Estimated { reason, .. }
            | Self::ProductDefault { reason, .. }
            | Self::Unknown { reason } => Some(reason),
            Self::Observed { .. } | Self::ExplicitInput { .. } => None,
        }
    }

    /// Returns whether a value is available.
    #[must_use]
    pub const fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::{Evidence, EvidenceClass};
    use ghostram_core::quantity::Bytes;

    #[test]
    fn serializes_each_evidence_class_with_a_typed_tag() {
        let observed = Evidence::observed(Bytes::new(10));
        assert_eq!(
            serde_json::to_value(&observed).unwrap(),
            serde_json::json!({"evidence": "observed", "value": 10})
        );

        let explicit = Evidence::explicit_input(7_u64);
        assert_eq!(
            serde_json::to_value(&explicit).unwrap(),
            serde_json::json!({"evidence": "explicit_input", "value": 7})
        );

        let derived = Evidence::derived(5_u64, "5 = 10 - 5");
        assert_eq!(
            serde_json::to_value(&derived).unwrap(),
            serde_json::json!({"evidence": "derived", "value": 5, "reason": "5 = 10 - 5"})
        );

        let estimated = Evidence::estimated(3_u64, "proxy");
        assert_eq!(
            serde_json::to_value(&estimated).unwrap(),
            serde_json::json!({"evidence": "estimated", "value": 3, "reason": "proxy"})
        );

        let default = Evidence::product_default(1_u64, "policy");
        assert_eq!(
            serde_json::to_value(&default).unwrap(),
            serde_json::json!({"evidence": "product_default", "value": 1, "reason": "policy"})
        );

        let unknown: Evidence<u64> = Evidence::unknown("not observed");
        assert_eq!(
            serde_json::to_value(&unknown).unwrap(),
            serde_json::json!({"evidence": "unknown", "reason": "not observed"})
        );
    }

    #[test]
    fn unknown_has_no_value_and_preserves_its_reason() {
        let unknown: Evidence<Bytes> = Evidence::unknown("MemAvailable was not reported");
        assert_eq!(unknown.class(), EvidenceClass::Unknown);
        assert!(unknown.value().is_none());
        assert_eq!(unknown.reason(), Some("MemAvailable was not reported"));
        assert!(!unknown.is_known());
    }

    #[test]
    fn evidence_round_trips_through_json() {
        let values = [
            Evidence::observed(1_u64),
            Evidence::explicit_input(2_u64),
            Evidence::derived(3_u64, "derived"),
            Evidence::estimated(4_u64, "estimated"),
            Evidence::product_default(5_u64, "default"),
            Evidence::unknown("unknown"),
        ];
        for value in values {
            let encoded = serde_json::to_string(&value).unwrap();
            let decoded: Evidence<u64> = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, value);
        }
    }
}
