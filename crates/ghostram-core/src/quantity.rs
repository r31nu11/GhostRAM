//! Byte-based quantities used by GhostRAM.
//!
//! Memory and storage values are stored as exact `u64` byte counts. Human-readable
//! units are produced only for display. The JSON representation of [`Bytes`] is a
//! plain integer number of bytes.

use std::fmt;

use serde::{Deserialize, Serialize};

/// An exact byte count.
///
/// Zero is a valid measured value; it is never used as a substitute for missing
/// data. Optional measurements use `Option<Bytes>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bytes(u64);

impl Bytes {
    /// The zero byte count.
    pub const ZERO: Self = Self(0);

    /// Creates a byte count.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact byte count.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Converts KiB to bytes, returning `None` on overflow.
    #[must_use]
    pub fn from_kib(kib: u64) -> Option<Self> {
        kib.checked_mul(1024).map(Self)
    }

    /// Converts MiB to bytes, returning `None` on overflow.
    #[must_use]
    pub fn from_mib(mib: u64) -> Option<Self> {
        mib.checked_mul(1024 * 1024).map(Self)
    }

    /// Converts GiB to bytes, returning `None` on overflow.
    #[must_use]
    pub fn from_gib(gib: u64) -> Option<Self> {
        gib.checked_mul(1024 * 1024 * 1024).map(Self)
    }
}

impl fmt::Display for Bytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        const KIB: u64 = 1024;
        const MIB: u64 = 1024 * KIB;
        const GIB: u64 = 1024 * MIB;
        const TIB: u64 = 1024 * GIB;

        let bytes = self.0;
        if bytes >= TIB {
            write!(formatter, "{:.1} TiB", bytes as f64 / TIB as f64)
        } else if bytes >= GIB {
            write!(formatter, "{:.1} GiB", bytes as f64 / GIB as f64)
        } else if bytes >= MIB {
            write!(formatter, "{:.1} MiB", bytes as f64 / MIB as f64)
        } else if bytes >= KIB {
            write!(formatter, "{:.1} KiB", bytes as f64 / KIB as f64)
        } else {
            write!(formatter, "{bytes} B")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bytes;

    #[test]
    fn preserves_exact_byte_counts() {
        assert_eq!(Bytes::new(7_802_212).as_u64(), 7_802_212);
        assert_eq!(Bytes::ZERO.as_u64(), 0);
    }

    #[test]
    fn converts_binary_units() {
        assert_eq!(Bytes::from_kib(1), Some(Bytes::new(1024)));
        assert_eq!(Bytes::from_mib(1), Some(Bytes::new(1024 * 1024)));
        assert_eq!(Bytes::from_gib(1), Some(Bytes::new(1024 * 1024 * 1024)));
    }

    #[test]
    fn reports_conversion_overflow() {
        assert_eq!(Bytes::from_kib(u64::MAX), None);
        assert_eq!(Bytes::from_mib(u64::MAX), None);
        assert_eq!(Bytes::from_gib(u64::MAX), None);
    }

    #[test]
    fn formats_human_readable_units() {
        assert_eq!(Bytes::new(0).to_string(), "0 B");
        assert_eq!(Bytes::new(1023).to_string(), "1023 B");
        assert_eq!(Bytes::new(1024).to_string(), "1.0 KiB");
        assert_eq!(Bytes::from_kib(1536).unwrap().to_string(), "1.5 MiB");
        assert_eq!(Bytes::from_gib(7).unwrap().to_string(), "7.0 GiB");
    }

    #[test]
    fn serializes_as_an_integer() {
        let encoded = serde_json::to_string(&Bytes::new(4096)).unwrap();
        assert_eq!(encoded, "4096");
        let decoded: Bytes = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, Bytes::new(4096));
    }
}
