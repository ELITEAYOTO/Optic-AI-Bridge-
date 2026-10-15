use std::path::{Component, Path};

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
    #[error("native path contains a non-UTF-8 segment")]
    NonUtf8NativeSegment,
    #[error("native path segment cannot be represented without changing its identity")]
    UnrepresentableNativeSegment,
}

impl WorkspacePath {
    /// Convert an OS-relative path without applying caller-input normalization
    /// to a native filename. In particular, a Unix literal backslash must never
    /// become a logical separator in an authorization target.
    pub fn from_native_relative(path: &Path) -> Result<Self, WorkspacePathError> {
        let mut value = String::new();
        for component in path.components() {
            let Component::Normal(segment) = component else {
                return Err(WorkspacePathError::UnsafeSegment);
            };
            let segment = segment
                .to_str()
                .ok_or(WorkspacePathError::NonUtf8NativeSegment)?;
            let parsed = Self::parse(segment)?;
            if parsed.as_str() != segment || segment.contains('/') {
                return Err(WorkspacePathError::UnrepresentableNativeSegment);
            }
            if !value.is_empty() {
                value.push('/');
            }
            value.push_str(segment);
            if value.len() > MAX_WORKSPACE_PATH_BYTES {
                return Err(WorkspacePathError::TooLong);
            }
        }
        Self::parse(&value)
    }

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

    #[must_use]
    pub fn is_within(&self, prefix: &Self) -> bool {
        self == prefix
            || self
                .0
                .strip_prefix(&prefix.0)
                .is_some_and(|suffix| suffix.starts_with('/'))
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

    #[test]
    fn prefix_matching_is_segment_aware() {
        let prefix = WorkspacePath::parse("src").expect("safe prefix");
        let nested = WorkspacePath::parse("src/core/lib.rs").expect("safe nested path");
        let sibling = WorkspacePath::parse("src2/lib.rs").expect("safe sibling path");
        assert!(nested.is_within(&prefix));
        assert!(!sibling.is_within(&prefix));
    }

    #[test]
    fn native_relative_paths_preserve_normal_segments_and_bounds() {
        let native = Path::new("src").join("core").join("lib.rs");
        assert_eq!(
            WorkspacePath::from_native_relative(&native)
                .expect("native path")
                .as_str(),
            "src/core/lib.rs"
        );
        for path in [
            Path::new(""),
            Path::new("../escape"),
            Path::new("src/../escape"),
            Path::new("/absolute"),
        ] {
            assert!(WorkspacePath::from_native_relative(path).is_err());
        }
        let too_long = "x".repeat(MAX_WORKSPACE_PATH_BYTES + 1);
        assert_eq!(
            WorkspacePath::from_native_relative(Path::new(&too_long)),
            Err(WorkspacePathError::TooLong)
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_unix_names_are_never_normalized_into_other_authority() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        assert_eq!(
            WorkspacePath::from_native_relative(Path::new(r"literal\name")),
            Err(WorkspacePathError::UnrepresentableNativeSegment)
        );
        assert_eq!(
            WorkspacePath::from_native_relative(Path::new(OsStr::from_bytes(b"bad\xff"))),
            Err(WorkspacePathError::NonUtf8NativeSegment)
        );
        assert!(WorkspacePath::from_native_relative(Path::new("literal:name")).is_err());
        assert_eq!(
            WorkspacePath::parse(r"literal\name")
                .expect("caller input remains compatible")
                .as_str(),
            "literal/name"
        );
    }
}
