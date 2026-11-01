use std::{fmt, io::Read};

use thiserror::Error;

const VERSION_READ_BUFFER_BYTES: usize = 64 * 1024;
// Reserved for the pinned executable identity, not ordinary mutation reads.
const IDENTITY_VERSION_READ_BUFFER_BYTES: usize = 1024 * 1024;
const CONTENT_VERSION_HEX_BYTES: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentVersion([u8; 32]);

impl ContentVersion {
    #[must_use]
    pub fn from_bytes(content: &[u8]) -> Self {
        Self(*blake3::hash(content).as_bytes())
    }

    pub fn from_hex(value: &str) -> Result<Self, ContentVersionParseError> {
        if value.len() != CONTENT_VERSION_HEX_BYTES {
            return Err(ContentVersionParseError::InvalidLength);
        }

        let source = value.as_bytes();
        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let high = decode_hex_nibble(source[index * 2])?;
            let low = decode_hex_nibble(source[index * 2 + 1])?;
            *byte = (high << 4) | low;
        }
        Ok(Self(bytes))
    }

    /// Hash a reader while enforcing an exact upper bound on bytes consumed.
    ///
    /// At most `max_bytes + 1` bytes are read: the extra byte is used only to
    /// prove that the source exceeds the authorized observation ceiling.
    pub fn from_reader_bounded(
        reader: impl Read,
        max_bytes: u64,
    ) -> Result<Self, ContentVersionReadError> {
        Self::hash_bounded_with_buffer(reader, max_bytes, VERSION_READ_BUFFER_BYTES)
    }

    /// Hashes ALL bytes with the same BLAKE3 and exact max+1 ceiling, with a
    /// larger, fixed read request for originally pinned executable identity.
    /// Fewer OS calls are an evidence-based hypothesis for #467's slow
    /// 64-KiB Windows reads; this does not promise bounded disk latency.
    pub fn from_reader_bounded_large_chunks(
        reader: impl Read,
        max_bytes: u64,
    ) -> Result<Self, ContentVersionReadError> {
        Self::hash_bounded_with_buffer(reader, max_bytes, IDENTITY_VERSION_READ_BUFFER_BYTES)
    }

    fn hash_bounded_with_buffer(
        mut reader: impl Read,
        max_bytes: u64,
        capacity: usize,
    ) -> Result<Self, ContentVersionReadError> {
        if max_bytes == 0 {
            return Err(ContentVersionReadError::LimitExceeded);
        }

        // The identity chunk must live on the heap: a 1-MiB stack array can
        // overflow Windows worker stacks. Both capacities are fixed constants.
        let mut buffer = vec![0_u8; capacity];
        let mut hasher = blake3::Hasher::new();
        let mut total = 0_u64;

        loop {
            let remaining = max_bytes.saturating_sub(total);
            let request = if remaining >= capacity as u64 {
                capacity
            } else {
                usize::try_from(remaining)
                    .expect("bounded remainder fits the fixed buffer")
                    .saturating_add(1)
            };
            // No prefetch, cache, skipped hash, unpinned replacement or
            // observation beyond the original max_bytes+1 security ceiling.
            let read = reader.read(&mut buffer[..request])?;
            if read == 0 {
                break;
            }
            total = total
                .checked_add(
                    u64::try_from(read).map_err(|_| ContentVersionReadError::LimitExceeded)?,
                )
                .ok_or(ContentVersionReadError::LimitExceeded)?;
            if total > max_bytes {
                return Err(ContentVersionReadError::LimitExceeded);
            }
            hasher.update(&buffer[..read]);
        }
        Ok(Self(*hasher.finalize().as_bytes()))
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        blake3::Hash::from_bytes(self.0).to_hex().to_string()
    }
}

fn decode_hex_nibble(byte: u8) -> Result<u8, ContentVersionParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ContentVersionParseError::InvalidHex),
    }
}

impl fmt::Debug for ContentVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ContentVersion")
            .field(&self.to_hex())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ContentVersionParseError {
    #[error("content version must contain exactly 64 hexadecimal characters")]
    InvalidLength,
    #[error("content version contains a non-hexadecimal character")]
    InvalidHex,
}

#[derive(Debug, Error)]
pub enum ContentVersionReadError {
    #[error("content observation exceeds its hard byte limit")]
    LimitExceeded,
    #[error("content observation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn versions_change_when_content_changes() {
        let first = ContentVersion::from_bytes(b"alpha");
        let second = ContentVersion::from_bytes(b"beta");
        assert_ne!(first, second);
        assert_eq!(first, ContentVersion::from_bytes(b"alpha"));
    }

    #[test]
    fn persisted_hex_round_trips_case_insensitively() {
        let version = ContentVersion::from_bytes(b"persisted");
        let encoded = version.to_hex();
        assert_eq!(
            ContentVersion::from_hex(&encoded).expect("lowercase"),
            version
        );
        assert_eq!(
            ContentVersion::from_hex(&encoded.to_uppercase()).expect("uppercase"),
            version
        );
    }

    #[test]
    fn persisted_hex_rejects_wrong_length_or_non_hex() {
        assert_eq!(
            ContentVersion::from_hex("00"),
            Err(ContentVersionParseError::InvalidLength)
        );
        let mut invalid = "0".repeat(CONTENT_VERSION_HEX_BYTES);
        invalid.replace_range(7..8, "z");
        assert_eq!(
            ContentVersion::from_hex(&invalid),
            Err(ContentVersionParseError::InvalidHex)
        );
    }

    #[test]
    fn bounded_streaming_version_matches_in_memory_version() {
        let content = vec![0x5a; 192 * 1024 + 17];
        let streamed = ContentVersion::from_reader_bounded(
            Cursor::new(&content),
            u64::try_from(content.len()).expect("fixture length fits u64"),
        )
        .expect("stream hash");
        assert_eq!(streamed, ContentVersion::from_bytes(&content));
    }

    #[test]
    fn large_identity_chunks_keep_full_hash_with_far_fewer_reads() {
        use std::cell::Cell;
        struct Count<'a> {
            source: Cursor<Vec<u8>>,
            calls: &'a Cell<usize>,
        }
        impl Read for Count<'_> {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                self.calls.set(self.calls.get() + 1);
                self.source.read(output)
            }
        }
        let input = vec![0x5a; 4 * 1024 * 1024 + 17];
        let large_calls = Cell::new(0);
        let old_calls = Cell::new(0);
        let old = ContentVersion::from_reader_bounded(
            Count { source: Cursor::new(input.clone()), calls: &old_calls },
            input.len() as u64,
        ).expect("legacy full digest");
        let large = ContentVersion::from_reader_bounded_large_chunks(
            Count { source: Cursor::new(input.clone()), calls: &large_calls },
            input.len() as u64,
        ).expect("large-chunk full digest");
        assert_eq!(old, ContentVersion::from_bytes(&input));
        assert_eq!(large, old);
        assert_eq!(large_calls.get(), 6, "four full reads, short tail and EOF");
        assert!(old_calls.get() > large_calls.get() * 10);
    }

    #[test]
    fn large_identity_chunks_keep_max_plus_one_and_short_read_guarantees() {
        struct OneByte {
            next: u8,
            end: u8,
        }
        impl Read for OneByte {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                if self.next == self.end {
                    return Ok(0);
                }
                output[0] = self.next;
                self.next += 1;
                Ok(1)
            }
        }
        let input = vec![0x45; 2 * 1024 * 1024 + 13];
        let limit = (input.len() - 1) as u64;
        assert!(matches!(
            ContentVersion::from_reader_bounded_large_chunks(Cursor::new(&input), limit),
            Err(ContentVersionReadError::LimitExceeded)
        ));
        let one = ContentVersion::from_reader_bounded_large_chunks(
            OneByte { next: 0, end: 47 }, 47,
        ).expect("short reads and exact EOF");
        assert_eq!(one, ContentVersion::from_bytes(&(0_u8..47).collect::<Vec<_>>()));
        assert!(matches!(
            ContentVersion::from_reader_bounded_large_chunks(Cursor::new(b"anything"), 0),
            Err(ContentVersionReadError::LimitExceeded)
        ));
    }

    #[test]
    fn bounded_streaming_version_detects_one_byte_overflow() {
        let content = vec![0x42; 65 * 1024];
        assert!(matches!(
            ContentVersion::from_reader_bounded(Cursor::new(content), 64 * 1024),
            Err(ContentVersionReadError::LimitExceeded)
        ));
    }
}
