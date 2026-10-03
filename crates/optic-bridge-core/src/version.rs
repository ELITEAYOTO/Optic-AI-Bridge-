use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentVersion([u8; 32]);

impl ContentVersion {
    #[must_use]
    pub fn from_bytes(content: &[u8]) -> Self {
        Self(*blake3::hash(content).as_bytes())
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
    use super::*;

    #[test]
    fn versions_change_when_content_changes() {
        let first = ContentVersion::from_bytes(b"alpha");
        let second = ContentVersion::from_bytes(b"beta");
        assert_ne!(first, second);
        assert_eq!(first, ContentVersion::from_bytes(b"alpha"));
    }
}
