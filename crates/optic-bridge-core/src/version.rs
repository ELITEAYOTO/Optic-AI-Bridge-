use std::{fmt, io::Read};

use thiserror::Error;

const VERSION_READ_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentVersion([u8; 32]);

impl ContentVersion {
    #[must_use]
    pub fn from_bytes(content: &[u8]) -> Self {
        Self(*blake3::hash(content).as_bytes())
    }

    /// Hash a reader while enforcing an exact upper bound on bytes consumed.
    ///
    /// At most `max_bytes + 1` bytes are read: the extra byte is used only to
    /// prove that the source exceeds the authorized observation ceiling.
    pub fn from_reader_bounded(
        mut reader: impl Read,
        max_bytes: u64,
    ) -> Result<Self, ContentVersionReadError> {
        if max_bytes == 0 {
            return Err(ContentVersionReadError::LimitExceeded);
        }

        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; VERSION_READ_BUFFER_BYTES];
        let mut total = 0_u64;

        loop {
            let remaining = max_bytes.saturating_sub(total);
            let request = if remaining >= VERSION_READ_BUFFER_BYTES as u64 {
                VERSION_READ_BUFFER_BYTES
            } else {
                usize::try_from(remaining)
                    .expect("remaining bounded read bytes must fit the fixed buffer")
                    .saturating_add(1)
            };

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

impl fmt::Debug for ContentVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ContentVersion")
            .field(&self.to_hex())
            .finish()
    }
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
    fn bounded_streaming_version_detects_one_byte_overflow() {
        let content = vec![0x42; 65 * 1024];
        assert!(matches!(
            ContentVersion::from_reader_bounded(Cursor::new(content), 64 * 1024),
            Err(ContentVersionReadError::LimitExceeded)
        ));
    }
}
