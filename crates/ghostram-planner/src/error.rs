//! Typed errors for planner option validation.

use std::error::Error;
use std::fmt;

/// Stable planner error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanErrorKind {
    /// A required option has no value.
    MissingOption,
    /// An option was supplied with evidence that is not valid for an input.
    InvalidOptionEvidence,
    /// The requested context is outside the documented planner range.
    ContextOutOfRange,
    /// The requested sequence count is outside the documented planner range.
    SequencesOutOfRange,
}

impl PlanErrorKind {
    /// Returns the stable machine-readable code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingOption => "missing_option",
            Self::InvalidOptionEvidence => "invalid_option_evidence",
            Self::ContextOutOfRange => "context_out_of_range",
            Self::SequencesOutOfRange => "sequences_out_of_range",
        }
    }
}

/// A planner input error with a stable category and a human-readable detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanError {
    kind: PlanErrorKind,
    message: String,
}

impl PlanError {
    pub(crate) fn new(kind: PlanErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns the stable error category.
    #[must_use]
    pub const fn kind(&self) -> PlanErrorKind {
        self.kind
    }

    /// Returns the error detail.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl Error for PlanError {}
