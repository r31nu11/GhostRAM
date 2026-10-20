//! Typed errors with stable machine-readable codes.

use std::error::Error;
use std::fmt;

/// Stable error codes for library boundaries and future machine-readable errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// An input value is invalid or cannot be represented.
    InvalidInput,
    /// A system or tool output could not be parsed.
    ParseFailure,
    /// A read-only filesystem operation failed.
    IoFailure,
    /// A subprocess could not be started or returned an unexpected failure.
    CommandFailure,
    /// A bounded subprocess exceeded its timeout.
    Timeout,
    /// The current platform is not supported by the requested probe.
    UnsupportedPlatform,
    /// A report could not be serialized or deserialized.
    SerializationFailure,
}

impl ErrorCode {
    /// Returns the stable string code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::ParseFailure => "parse_failure",
            Self::IoFailure => "io_failure",
            Self::CommandFailure => "command_failure",
            Self::Timeout => "timeout",
            Self::UnsupportedPlatform => "unsupported_platform",
            Self::SerializationFailure => "serialization_failure",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A typed GhostRAM error.
///
/// Messages are intended for humans and must not contain secrets, prompts, model
/// contents, or full environment dumps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GhostramError {
    /// Invalid input or a quantity that cannot be represented.
    InvalidInput { message: String },
    /// A parser rejected malformed or incomplete input.
    Parse { source: String, message: String },
    /// A read-only filesystem access failed.
    Io { path: String, message: String },
    /// A subprocess failed to start or exited unexpectedly for the operation.
    Command { program: String, message: String },
    /// A subprocess exceeded its bounded execution time.
    Timeout { program: String, timeout_ms: u64 },
    /// The requested operation is not supported on this platform.
    UnsupportedPlatform { platform: String },
    /// A report could not be serialized or deserialized.
    Serialization { message: String },
}

impl GhostramError {
    /// Returns the stable error code.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidInput { .. } => ErrorCode::InvalidInput,
            Self::Parse { .. } => ErrorCode::ParseFailure,
            Self::Io { .. } => ErrorCode::IoFailure,
            Self::Command { .. } => ErrorCode::CommandFailure,
            Self::Timeout { .. } => ErrorCode::Timeout,
            Self::UnsupportedPlatform { .. } => ErrorCode::UnsupportedPlatform,
            Self::Serialization { .. } => ErrorCode::SerializationFailure,
        }
    }
}

impl fmt::Display for GhostramError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput { message } => write!(formatter, "invalid input: {message}"),
            Self::Parse { source, message } => {
                write!(formatter, "failed to parse {source}: {message}")
            }
            Self::Io { path, message } => write!(formatter, "I/O error at {path}: {message}"),
            Self::Command { program, message } => {
                write!(formatter, "command {program} failed: {message}")
            }
            Self::Timeout {
                program,
                timeout_ms,
            } => write!(
                formatter,
                "command {program} exceeded the {timeout_ms} ms timeout"
            ),
            Self::UnsupportedPlatform { platform } => {
                write!(formatter, "unsupported platform: {platform}")
            }
            Self::Serialization { message } => {
                write!(formatter, "serialization error: {message}")
            }
        }
    }
}

impl Error for GhostramError {}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, GhostramError};

    #[test]
    fn exposes_stable_codes() {
        assert_eq!(ErrorCode::InvalidInput.as_str(), "invalid_input");
        assert_eq!(ErrorCode::ParseFailure.as_str(), "parse_failure");
        assert_eq!(ErrorCode::IoFailure.as_str(), "io_failure");
        assert_eq!(ErrorCode::CommandFailure.as_str(), "command_failure");
        assert_eq!(ErrorCode::Timeout.as_str(), "timeout");
        assert_eq!(
            ErrorCode::UnsupportedPlatform.as_str(),
            "unsupported_platform"
        );
        assert_eq!(
            ErrorCode::SerializationFailure.as_str(),
            "serialization_failure"
        );
    }

    #[test]
    fn maps_variants_to_codes() {
        let error = GhostramError::Timeout {
            program: "example".to_string(),
            timeout_ms: 250,
        };
        assert_eq!(error.code(), ErrorCode::Timeout);
        assert_eq!(
            error.to_string(),
            "command example exceeded the 250 ms timeout"
        );
    }

    #[test]
    fn formats_parse_errors_with_source() {
        let error = GhostramError::Parse {
            source: "/proc/meminfo".to_string(),
            message: "missing MemTotal".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "failed to parse /proc/meminfo: missing MemTotal"
        );
    }
}
