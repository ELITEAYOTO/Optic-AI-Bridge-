use std::{fmt, io::Read};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentVersion([u8; 32]);

impl ContentVersion {
    #[must_use]
    pub fn from_bytes(content: &[u8]) -> Self {
        Self(*blake3::hash(content).as_bytes())
    }

    pub fn from_reader(mut reader: impl Read) -> std::io::Result<Self> {
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
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
    fn streaming_version_matches_in_memory_version() {
        let content = vec![0x5a; 192 * 1024 + 17];
        let streamed = ContentVersion::from_reader(Cursor::new(&content)).expect("stream hash");
        assert_eq!(streamed, ContentVersion::from_bytes(&content));
    }
}
