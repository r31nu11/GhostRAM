//! Typed errors for GGUF parsing and scan setup.

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::catalog::{Failure, FailureKind};

/// Stable parser error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgufErrorKind {
    /// Magic bytes were not `GGUF`.
    InvalidMagic,
    /// The version is not 2 or 3.
    UnsupportedVersion,
    /// The file appears to use a byte order other than little-endian.
    UnsupportedEndianness,
    /// A metadata type code is outside the documented range.
    UnsupportedMetadataType,
    /// `general.alignment` is unusable or not a multiple of 8.
    InvalidAlignment,
    /// A metadata key occurred more than once.
    DuplicateMetadataKey,
    /// A tensor name occurred more than once.
    DuplicateTensorName,
    /// A tensor descriptor is invalid or outside the declared file.
    InvalidTensor,
    /// The declared structure extends beyond the file.
    Truncated,
    /// Other structural inconsistency.
    Malformed,
    /// A documented resource limit was exceeded.
    LimitExceeded,
    /// Reading the file failed.
    Io,
}

impl GgufErrorKind {
    /// Returns the stable schema string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidMagic => "invalid_magic",
            Self::UnsupportedVersion => "unsupported_version",
            Self::UnsupportedEndianness => "unsupported_endianness",
            Self::UnsupportedMetadataType => "unsupported_metadata_type",
            Self::InvalidAlignment => "invalid_alignment",
            Self::DuplicateMetadataKey => "duplicate_metadata_key",
            Self::DuplicateTensorName => "duplicate_tensor_name",
            Self::InvalidTensor => "invalid_tensor",
            Self::Truncated => "truncated",
            Self::Malformed => "malformed",
            Self::LimitExceeded => "limit_exceeded",
            Self::Io => "io",
        }
    }

    pub(crate) const fn failure_kind(self) -> FailureKind {
        match self {
            Self::InvalidMagic => FailureKind::InvalidMagic,
            Self::UnsupportedVersion => FailureKind::UnsupportedVersion,
            Self::UnsupportedEndianness => FailureKind::UnsupportedEndianness,
            Self::UnsupportedMetadataType => FailureKind::UnsupportedMetadataType,
            Self::InvalidAlignment => FailureKind::InvalidAlignment,
            Self::DuplicateMetadataKey => FailureKind::DuplicateMetadataKey,
            Self::DuplicateTensorName => FailureKind::DuplicateTensorName,
            Self::InvalidTensor => FailureKind::InvalidTensor,
            Self::Truncated => FailureKind::Truncated,
            Self::Malformed => FailureKind::Malformed,
            Self::LimitExceeded => FailureKind::LimitExceeded,
            Self::Io => FailureKind::Io,
        }
    }
}

/// A bounded GGUF parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GgufError {
    kind: GgufErrorKind,
    message: String,
    offset: Option<u64>,
}

impl GgufError {
    pub(crate) fn new(
        kind: GgufErrorKind,
        message: impl Into<String>,
        offset: Option<u64>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            offset,
        }
    }

    /// Returns the stable error category.
    #[must_use]
    pub const fn kind(&self) -> GgufErrorKind {
        self.kind
    }

    /// Returns the error detail.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the byte offset associated with the failure, when known.
    #[must_use]
    pub const fn offset(&self) -> Option<u64> {
        self.offset
    }

    pub(crate) fn into_failure(self) -> Failure {
        Failure {
            kind: self.kind.failure_kind(),
            message: self.message,
            offset: self.offset,
        }
    }
}

impl fmt::Display for GgufError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(offset) = self.offset {
            write!(
                formatter,
                "{} at byte {offset}: {}",
                self.kind.as_str(),
                self.message
            )
        } else {
            write!(formatter, "{}: {}", self.kind.as_str(), self.message)
        }
    }
}

impl Error for GgufError {}

/// A fatal failure before or while opening the selected scan root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    path: String,
    message: String,
}

impl ScanError {
    pub(crate) fn new(path: &Path, message: impl Into<String>) -> Self {
        Self {
            path: path.to_string_lossy().into_owned(),
            message: message.into(),
        }
    }

    /// Returns the affected path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the error detail.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl Error for ScanError {}
