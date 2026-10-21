//! Bounded streaming SHA-256 support.

use std::io::Read;

use sha2::{Digest, Sha256};

/// Hashing reads at most this many bytes into memory at once.
pub const HASH_BUFFER_BYTES: usize = 64 * 1024;

/// Complete digest and the number of bytes consumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HashComputation {
    pub digest_hex: String,
    pub bytes_hashed: u64,
}

/// A failed hash read. No partial digest is exposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HashReadError {
    pub message: String,
    pub bytes_hashed: u64,
}

/// Streams a reader to EOF through SHA-256 with a fixed-size buffer.
///
/// A digest is returned only after a successful EOF. An interrupted read returns
/// an error with the number of bytes consumed but no partial digest.
pub(crate) fn hash_reader<R: Read>(reader: &mut R) -> Result<HashComputation, HashReadError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    let mut bytes_hashed = 0_u64;

    loop {
        let read = reader.read(&mut buffer).map_err(|error| HashReadError {
            message: error.to_string(),
            bytes_hashed,
        })?;
        if read == 0 {
            break;
        }
        bytes_hashed = bytes_hashed
            .checked_add(u64::try_from(read).expect("usize read length fits in u64"))
            .ok_or_else(|| HashReadError {
                message: "hashed byte count overflow".to_string(),
                bytes_hashed,
            })?;
        hasher.update(&buffer[..read]);
    }

    Ok(HashComputation {
        digest_hex: format!("{:x}", hasher.finalize()),
        bytes_hashed,
    })
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use super::hash_reader;

    #[test]
    fn matches_the_empty_sha256_vector() {
        let mut input = io::empty();
        let hash = hash_reader(&mut input).unwrap();
        assert_eq!(
            hash.digest_hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(hash.bytes_hashed, 0);
    }

    #[test]
    fn matches_the_abc_sha256_vector() {
        let mut input = b"abc".as_slice();
        let hash = hash_reader(&mut input).unwrap();
        assert_eq!(
            hash.digest_hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(hash.bytes_hashed, 3);
    }

    #[test]
    fn hashes_across_multiple_buffer_fills() {
        let data = vec![b'a'; 1_000_000];
        let mut input = data.as_slice();
        let hash = hash_reader(&mut input).unwrap();
        assert_eq!(
            hash.digest_hex,
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        assert_eq!(hash.bytes_hashed, 1_000_000);
    }

    struct FailingReader {
        remaining: usize,
    }

    impl Read for FailingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::other("synthetic read failure"));
            }
            let count = self.remaining.min(buffer.len()).min(8);
            buffer[..count].fill(b'x');
            self.remaining -= count;
            Ok(count)
        }
    }

    #[test]
    fn returns_no_digest_after_a_read_error() {
        let mut input = FailingReader { remaining: 8 };
        let error = hash_reader(&mut input).unwrap_err();
        assert!(error.message.contains("synthetic read failure"));
        assert_eq!(error.bytes_hashed, 8);
    }
}
