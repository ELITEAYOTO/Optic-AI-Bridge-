use thiserror::Error;

const MAX_WORKSPACE_PATH_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspacePath(String);

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum WorkspacePathError {
    #[error("workspace path is empty")]
    Empty,
    #[error("workspace path exceeds the maximum accepted length")]
    TooLong,
    #[error("workspace path must be relative")]
    Absolute,
    #[error("workspace path contains a forbidden prefix")]
    Prefix,
    #[error("workspace path contains an empty, current-directory, or parent-directory segment")]
    UnsafeSegment,
    #[error("workspace path contains a NUL byte")]
    Nul,
}

impl WorkspacePath {
    pub fn parse(input: &str) -> Result<Self, WorkspacePathError> {
        if input.is_empty() {
            return Err(WorkspacePathError::Empty);
        }
        if input.len() > MAX_WORKSPACE_PATH_BYTES {
            return Err(WorkspacePathError::TooLong);
        }
        if input.contains('\0') {
            return Err(WorkspacePathError::Nul);
        }
        if input.starts_with('/') || input.starts_with('\\') {
            return Err(WorkspacePathError::Absolute);
        }

        let normalized = input.replace('\\', "/");
        if normalized.contains(':') {
            return Err(WorkspacePathError::Prefix);
        }

        for segment in normalized.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(WorkspacePathError::UnsafeSegment);
            }
        }

        Ok(Self(normalized))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_windows_separators() {
        let path = WorkspacePath::parse(r"src\core\lib.rs").expect("safe path");
        assert_eq!(path.as_str(), "src/core/lib.rs");
    }

    #[test]
    fn rejects_escape_and_absolute_forms() {
        for candidate in [
            "../secret.txt",
            "src/../secret.txt",
            "/windows/system32",
            r"\server\share",
            r"C:\Windows\System32",
            "src//main.rs",
            "./src/main.rs",
        ] {
            assert!(
                WorkspacePath::parse(candidate).is_err(),
                "candidate should be rejected: {candidate}"
            );
        }
    }
}
