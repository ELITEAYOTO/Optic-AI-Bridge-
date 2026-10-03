use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
};

use optic_bridge_core::{
    ContentVersion, ContentVersionReadError, ExpectedState, HardLimits, WorkspacePath,
    WorkspacePathError,
};
use thiserror::Error;

#[derive(Debug)]
pub struct BoundedFileSystem {
    root: PathBuf,
    max_read_bytes: u64,
    max_mutation_bytes: u64,
    max_list_page_entries: usize,
    max_directory_scan_entries: usize,
}

impl BoundedFileSystem {
    pub fn from_hard_limits(
        root: impl AsRef<Path>,
        limits: HardLimits,
    ) -> Result<Self, FileSystemError> {
        Self::new_with_mutation_limit(
            root,
            limits.max_fs_read_bytes,
            limits.max_fs_mutation_bytes,
            limits.max_fs_list_page_entries,
            limits.max_fs_directory_scan_entries,
        )
    }

    pub fn new(
        root: impl AsRef<Path>,
        max_read_bytes: u64,
        max_list_page_entries: u32,
        max_directory_scan_entries: u32,
    ) -> Result<Self, FileSystemError> {
        Self::new_with_mutation_limit(
            root,
            max_read_bytes,
            max_read_bytes,
            max_list_page_entries,
            max_directory_scan_entries,
        )
    }

    pub fn new_with_mutation_limit(
        root: impl AsRef<Path>,
        max_read_bytes: u64,
        max_mutation_bytes: u64,
        max_list_page_entries: u32,
        max_directory_scan_entries: u32,
    ) -> Result<Self, FileSystemError> {
        if max_read_bytes == 0
            || max_mutation_bytes == 0
            || max_list_page_entries == 0
            || max_directory_scan_entries == 0
        {
            return Err(FileSystemError::InvalidLimits);
        }
        if max_directory_scan_entries < max_list_page_entries {
            return Err(FileSystemError::InvalidLimits);
        }

        let root = fs::canonicalize(root).map_err(FileSystemError::Io)?;
        if !root.is_dir() {
            return Err(FileSystemError::RootNotDirectory);
        }

        Ok(Self {
            root,
            max_read_bytes,
            max_mutation_bytes,
            max_list_page_entries: usize::try_from(max_list_page_entries)
                .map_err(|_| FileSystemError::InvalidLimits)?,
            max_directory_scan_entries: usize::try_from(max_directory_scan_entries)
                .map_err(|_| FileSystemError::InvalidLimits)?,
        })
    }

    pub fn read(
        &self,
        path: &WorkspacePath,
        offset: u64,
        max_bytes: Option<u64>,
    ) -> Result<FsReadChunk, FileSystemError> {
        let requested = max_bytes.unwrap_or(self.max_read_bytes);
        if requested == 0 || requested > self.max_read_bytes {
            return Err(FileSystemError::ReadLimitExceeded);
        }
        let buffer_len =
            usize::try_from(requested).map_err(|_| FileSystemError::ReadLimitExceeded)?;

        let resolved = self.resolve_existing(path)?;
        let mut file = File::open(&resolved).map_err(FileSystemError::Io)?;
        let metadata = file.metadata().map_err(FileSystemError::Io)?;
        if !metadata.is_file() {
            return Err(FileSystemError::NotFile);
        }
        if offset > metadata.len() {
            return Err(FileSystemError::OffsetOutOfRange);
        }

        file.seek(SeekFrom::Start(offset))
            .map_err(FileSystemError::Io)?;
        let mut bytes = vec![0_u8; buffer_len];
        let read = file.read(&mut bytes).map_err(FileSystemError::Io)?;
        bytes.truncate(read);

        let read = u64::try_from(read).map_err(|_| FileSystemError::ReadLimitExceeded)?;
        let next_offset = offset.saturating_add(read);
        Ok(FsReadChunk {
            bytes,
            offset,
            next_offset,
            eof: next_offset >= metadata.len(),
        })
    }

    pub fn list(
        &self,
        path: Option<&WorkspacePath>,
        cursor: usize,
        limit: u32,
    ) -> Result<FsListPage, FileSystemError> {
        let limit = usize::try_from(limit).map_err(|_| FileSystemError::ListLimitExceeded)?;
        if limit == 0 || limit > self.max_list_page_entries {
            return Err(FileSystemError::ListLimitExceeded);
        }

        let resolved = match path {
            Some(path) => self.resolve_existing(path)?,
            None => self.root.clone(),
        };
        if !resolved.is_dir() {
            return Err(FileSystemError::NotDirectory);
        }

        let mut entries = Vec::new();
        for result in fs::read_dir(&resolved).map_err(FileSystemError::Io)? {
            if entries.len() >= self.max_directory_scan_entries {
                return Err(FileSystemError::DirectoryScanLimitExceeded);
            }
            let entry = result.map_err(FileSystemError::Io)?;
            let file_name = entry
                .file_name()
                .into_string()
                .map_err(|_| FileSystemError::NonUtf8Name)?;
            let relative = match path {
                Some(parent) => format!("{}/{}", parent.as_str(), file_name),
                None => file_name.clone(),
            };
            let workspace_path = WorkspacePath::parse(&relative).map_err(FileSystemError::Path)?;
            let file_type = entry.file_type().map_err(FileSystemError::Io)?;
            let kind = if file_type.is_file() {
                EntryKind::File
            } else if file_type.is_dir() {
                EntryKind::Directory
            } else if file_type.is_symlink() {
                EntryKind::Symlink
            } else {
                EntryKind::Other
            };
            entries.push(DirectoryEntry {
                name: file_name,
                path: workspace_path,
                kind,
            });
        }

        entries.sort_by(|left, right| left.path.cmp(&right.path));
        let total_entries = entries.len();
        let page = entries
            .into_iter()
            .skip(cursor)
            .take(limit)
            .collect::<Vec<_>>();
        let consumed = cursor.saturating_add(page.len());
        let next_cursor = (consumed < total_entries).then_some(consumed);

        Ok(FsListPage {
            entries: page,
            next_cursor,
        })
    }

    /// Resolve a future mutation target to the canonical workspace location and
    /// observe the current optimistic-concurrency state without mutating anything.
    ///
    /// Existing leaf symlinks are rejected instead of followed. For an absent
    /// target, the parent directory is canonicalized so a symlinked parent cannot
    /// hide the actual authorization target. Callers should authorize the returned
    /// `canonical_path`, not the raw requested path.
    pub fn observe_mutation_target(
        &self,
        path: &WorkspacePath,
    ) -> Result<MutationObservation, MutationError> {
        let candidate = self.join_workspace_path(path);

        match fs::symlink_metadata(&candidate) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(MutationError::SymlinkDenied);
                }
                if !metadata.is_file() {
                    return Err(MutationError::NotFile);
                }
                if metadata.len() > self.max_mutation_bytes {
                    return Err(MutationError::TargetTooLarge {
                        limit: self.max_mutation_bytes,
                    });
                }

                let canonical = fs::canonicalize(&candidate).map_err(MutationError::Io)?;
                self.ensure_inside_workspace(&canonical)
                    .map_err(MutationError::FileSystem)?;
                let canonical_path = self.workspace_path_from_absolute(&canonical)?;
                let mut file = File::open(&canonical).map_err(MutationError::Io)?;
                let opened_metadata = file.metadata().map_err(MutationError::Io)?;
                if !opened_metadata.is_file() {
                    return Err(MutationError::NotFile);
                }
                if opened_metadata.len() > self.max_mutation_bytes {
                    return Err(MutationError::TargetTooLarge {
                        limit: self.max_mutation_bytes,
                    });
                }

                let version =
                    match ContentVersion::from_reader_bounded(&mut file, self.max_mutation_bytes) {
                        Ok(version) => version,
                        Err(ContentVersionReadError::LimitExceeded) => {
                            return Err(MutationError::TargetTooLarge {
                                limit: self.max_mutation_bytes,
                            });
                        }
                        Err(ContentVersionReadError::Io(error)) => {
                            return Err(MutationError::Io(error));
                        }
                    };

                Ok(MutationObservation {
                    canonical_path,
                    state: ExpectedState::Content(version),
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let parent = candidate.parent().ok_or(MutationError::InvalidTarget)?;
                let parent = fs::canonicalize(parent).map_err(MutationError::Io)?;
                self.ensure_inside_workspace(&parent)
                    .map_err(MutationError::FileSystem)?;
                if !parent.is_dir() {
                    return Err(MutationError::NotDirectory);
                }
                let file_name = candidate.file_name().ok_or(MutationError::InvalidTarget)?;
                let canonical_candidate = parent.join(file_name);
                let canonical_path = self.workspace_path_from_absolute(&canonical_candidate)?;

                Ok(MutationObservation {
                    canonical_path,
                    state: ExpectedState::Absent,
                })
            }
            Err(error) => Err(MutationError::Io(error)),
        }
    }

    /// Observe the canonical mutation target and require an exact caller-supplied
    /// expected state. This is a planning/revalidation primitive only; it performs
    /// no durable mutation.
    pub fn require_expected_state(
        &self,
        path: &WorkspacePath,
        expected: ExpectedState,
    ) -> Result<MutationObservation, MutationError> {
        let observation = self.observe_mutation_target(path)?;
        if observation.state != expected {
            return Err(MutationError::PreconditionFailed {
                expected,
                observed: observation.state,
            });
        }
        Ok(observation)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn join_workspace_path(&self, path: &WorkspacePath) -> PathBuf {
        let mut candidate = self.root.clone();
        for segment in path.as_str().split('/') {
            candidate.push(segment);
        }
        candidate
    }

    fn resolve_existing(&self, path: &WorkspacePath) -> Result<PathBuf, FileSystemError> {
        let candidate = self.join_workspace_path(path);
        let canonical = fs::canonicalize(candidate).map_err(FileSystemError::Io)?;
        self.ensure_inside_workspace(&canonical)?;
        Ok(canonical)
    }

    fn ensure_inside_workspace(&self, path: &Path) -> Result<(), FileSystemError> {
        if path.starts_with(&self.root) {
            Ok(())
        } else {
            Err(FileSystemError::OutsideWorkspace)
        }
    }

    fn workspace_path_from_absolute(&self, path: &Path) -> Result<WorkspacePath, MutationError> {
        self.ensure_inside_workspace(path)
            .map_err(MutationError::FileSystem)?;
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| MutationError::FileSystem(FileSystemError::OutsideWorkspace))?;
        let mut segments = Vec::new();
        for component in relative.components() {
            match component {
                Component::Normal(segment) => segments.push(
                    segment
                        .to_str()
                        .ok_or(MutationError::NonUtf8Name)?
                        .to_owned(),
                ),
                _ => return Err(MutationError::InvalidTarget),
            }
        }
        if segments.is_empty() {
            return Err(MutationError::InvalidTarget);
        }
        WorkspacePath::parse(&segments.join("/")).map_err(MutationError::Path)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationObservation {
    pub canonical_path: WorkspacePath,
    pub state: ExpectedState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FsReadChunk {
    pub bytes: Vec<u8>,
    pub offset: u64,
    pub next_offset: u64,
    pub eof: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FsListPage {
    pub entries: Vec<DirectoryEntry>,
    pub next_cursor: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: WorkspacePath,
    pub kind: EntryKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Error)]
pub enum MutationError {
    #[error("mutation target is invalid")]
    InvalidTarget,
    #[error("mutation of a leaf symlink is denied")]
    SymlinkDenied,
    #[error("mutation target is not a regular file")]
    NotFile,
    #[error("mutation target parent is not a directory")]
    NotDirectory,
    #[error("mutation target contains a non-UTF-8 path segment")]
    NonUtf8Name,
    #[error("mutation path is invalid: {0}")]
    Path(WorkspacePathError),
    #[error("mutation target exceeds the hard observation byte limit")]
    TargetTooLarge { limit: u64 },
    #[error("mutation precondition does not match the current canonical target state")]
    PreconditionFailed {
        expected: ExpectedState,
        observed: ExpectedState,
    },
    #[error("canonical mutation target validation failed: {0}")]
    FileSystem(FileSystemError),
    #[error("mutation observation failed: {0}")]
    Io(std::io::Error),
}

#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("filesystem limits are invalid")]
    InvalidLimits,
    #[error("workspace root must be a directory")]
    RootNotDirectory,
    #[error("filesystem path is invalid: {0}")]
    Path(WorkspacePathError),
    #[error("resolved path escapes the workspace root")]
    OutsideWorkspace,
    #[error("target is not a regular file")]
    NotFile,
    #[error("target is not a directory")]
    NotDirectory,
    #[error("requested read exceeds the hard per-call byte limit")]
    ReadLimitExceeded,
    #[error("requested list page exceeds the hard entry limit")]
    ListLimitExceeded,
    #[error("directory exceeds the hard deterministic scan ceiling")]
    DirectoryScanLimitExceeded,
    #[error("read offset is beyond the observed file length")]
    OffsetOutOfRange,
    #[error("directory entry cannot be represented as UTF-8")]
    NonUtf8Name,
    #[error("filesystem operation failed: {0}")]
    Io(std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use optic_bridge_core::ActionId;

    use super::*;

    fn workspace(label: &str) -> PathBuf {
        let token = ActionId::generate().expect("test entropy").to_token();
        let root = env::temp_dir().join(format!("optic-bridge-{label}-{token}"));
        fs::create_dir_all(&root).expect("create temp workspace");
        root
    }

    #[test]
    fn read_is_chunked_and_bounded() {
        let root = workspace("read");
        fs::write(root.join("hello.txt"), b"abcdefghij").expect("write fixture");
        let service = BoundedFileSystem::new(&root, 4, 4, 16).expect("filesystem service");
        let path = WorkspacePath::parse("hello.txt").expect("safe path");

        let first = service.read(&path, 0, Some(4)).expect("first chunk");
        assert_eq!(first.bytes, b"abcd");
        assert_eq!(first.next_offset, 4);
        assert!(!first.eof);

        let last = service.read(&path, 8, Some(4)).expect("last chunk");
        assert_eq!(last.bytes, b"ij");
        assert!(last.eof);
        assert_eq!(
            service
                .read(&path, 0, Some(5))
                .expect_err("limit must fail")
                .to_string(),
            FileSystemError::ReadLimitExceeded.to_string()
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn list_is_sorted_and_paginated() {
        let root = workspace("list");
        fs::write(root.join("z.txt"), b"z").expect("write z");
        fs::write(root.join("a.txt"), b"a").expect("write a");
        fs::create_dir(root.join("middle")).expect("create dir");
        let service = BoundedFileSystem::new(&root, 16, 2, 16).expect("filesystem service");

        let first = service.list(None, 0, 2).expect("first page");
        assert_eq!(
            first
                .entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a.txt", "middle"]
        );
        assert_eq!(first.next_cursor, Some(2));

        let second = service.list(None, 2, 2).expect("second page");
        assert_eq!(second.entries[0].name, "z.txt");
        assert_eq!(second.next_cursor, None);

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn deterministic_scan_has_a_hard_ceiling() {
        let root = workspace("scan");
        for index in 0..3 {
            fs::write(root.join(format!("{index}.txt")), b"x").expect("write fixture");
        }
        let service = BoundedFileSystem::new(&root, 16, 2, 2).expect("filesystem service");
        assert!(matches!(
            service.list(None, 0, 2),
            Err(FileSystemError::DirectoryScanLimitExceeded)
        ));
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn mutation_observation_distinguishes_absent_and_content() {
        let root = workspace("mutation-observe");
        fs::create_dir(root.join("src")).expect("create src");
        fs::write(root.join("src/existing.txt"), b"alpha").expect("write fixture");
        let service = BoundedFileSystem::new(&root, 16, 2, 16).expect("filesystem service");

        let existing = WorkspacePath::parse("src/existing.txt").expect("existing path");
        let observed = service
            .observe_mutation_target(&existing)
            .expect("observe existing target");
        assert_eq!(observed.canonical_path, existing);
        assert_eq!(
            observed.state,
            ExpectedState::Content(ContentVersion::from_bytes(b"alpha"))
        );

        let absent = WorkspacePath::parse("src/new.txt").expect("absent path");
        let observed = service
            .observe_mutation_target(&absent)
            .expect("observe absent target");
        assert_eq!(observed.canonical_path, absent);
        assert_eq!(observed.state, ExpectedState::Absent);

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn mutation_observation_is_hard_bounded() {
        let root = workspace("mutation-limit");
        fs::write(root.join("large.txt"), vec![0x41; 17]).expect("write fixture");
        let service = BoundedFileSystem::new_with_mutation_limit(&root, 64, 16, 2, 16)
            .expect("filesystem service");
        let path = WorkspacePath::parse("large.txt").expect("safe path");

        assert!(matches!(
            service.observe_mutation_target(&path),
            Err(MutationError::TargetTooLarge { limit: 16 })
        ));
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn stale_and_blind_overwrite_preconditions_fail_closed() {
        let root = workspace("mutation-stale");
        fs::write(root.join("target.txt"), b"alpha").expect("write fixture");
        let service = BoundedFileSystem::new(&root, 16, 2, 16).expect("filesystem service");
        let path = WorkspacePath::parse("target.txt").expect("safe path");
        let alpha = ContentVersion::from_bytes(b"alpha");

        service
            .require_expected_state(&path, ExpectedState::Content(alpha))
            .expect("matching version");
        assert!(matches!(
            service.require_expected_state(&path, ExpectedState::Absent),
            Err(MutationError::PreconditionFailed { .. })
        ));

        fs::write(root.join("target.txt"), b"beta").expect("mutate fixture");
        match service.require_expected_state(&path, ExpectedState::Content(alpha)) {
            Err(MutationError::PreconditionFailed { expected, observed }) => {
                assert_eq!(expected, ExpectedState::Content(alpha));
                assert_eq!(
                    observed,
                    ExpectedState::Content(ContentVersion::from_bytes(b"beta"))
                );
            }
            other => panic!("expected stale-state conflict, got {other:?}"),
        }

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn mutation_observation_rejects_leaf_symlink() {
        use std::os::unix::fs::symlink;

        let root = workspace("mutation-leaf-symlink");
        fs::write(root.join("real.txt"), b"alpha").expect("write real file");
        symlink(root.join("real.txt"), root.join("alias.txt")).expect("create symlink");
        let service = BoundedFileSystem::new(&root, 16, 2, 16).expect("filesystem service");
        let alias = WorkspacePath::parse("alias.txt").expect("alias path");

        assert!(matches!(
            service.observe_mutation_target(&alias),
            Err(MutationError::SymlinkDenied)
        ));

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn mutation_observation_authorizes_canonical_parent_target() {
        use std::os::unix::fs::symlink;

        let root = workspace("mutation-parent-symlink");
        fs::create_dir(root.join("real")).expect("create real dir");
        symlink(root.join("real"), root.join("alias")).expect("create parent symlink");
        let service = BoundedFileSystem::new(&root, 16, 2, 16).expect("filesystem service");
        let requested = WorkspacePath::parse("alias/new.txt").expect("requested path");

        let observed = service
            .observe_mutation_target(&requested)
            .expect("observe canonical target");
        assert_eq!(
            observed.canonical_path,
            WorkspacePath::parse("real/new.txt").expect("canonical path")
        );
        assert_eq!(observed.state, ExpectedState::Absent);

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn mutation_observation_rejects_parent_escape() {
        use std::os::unix::fs::symlink;

        let root = workspace("mutation-parent-escape");
        let outside = workspace("mutation-outside");
        symlink(&outside, root.join("escape")).expect("create escaping symlink");
        let service = BoundedFileSystem::new(&root, 16, 2, 16).expect("filesystem service");
        let requested = WorkspacePath::parse("escape/new.txt").expect("requested path");

        assert!(matches!(
            service.observe_mutation_target(&requested),
            Err(MutationError::FileSystem(FileSystemError::OutsideWorkspace))
        ));

        fs::remove_dir_all(root).expect("remove fixture");
        fs::remove_dir_all(outside).expect("remove outside fixture");
    }
}
