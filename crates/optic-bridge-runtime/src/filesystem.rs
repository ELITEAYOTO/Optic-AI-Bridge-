use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use optic_bridge_core::{HardLimits, WorkspacePath, WorkspacePathError};
use thiserror::Error;

#[derive(Debug)]
pub struct BoundedFileSystem {
    root: PathBuf,
    max_read_bytes: u64,
    max_list_page_entries: usize,
    max_directory_scan_entries: usize,
}

impl BoundedFileSystem {
    pub fn from_hard_limits(root: impl AsRef<Path>, limits: HardLimits) -> Result<Self, FileSystemError> {
        Self::new(
            root,
            limits.max_fs_read_bytes,
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
        if max_read_bytes == 0
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
        let buffer_len = usize::try_from(requested).map_err(|_| FileSystemError::ReadLimitExceeded)?;

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
        let page = entries.into_iter().skip(cursor).take(limit).collect::<Vec<_>>();
        let consumed = cursor.saturating_add(page.len());
        let next_cursor = (consumed < total_entries).then_some(consumed);

        Ok(FsListPage {
            entries: page,
            next_cursor,
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn resolve_existing(&self, path: &WorkspacePath) -> Result<PathBuf, FileSystemError> {
        let mut candidate = self.root.clone();
        for segment in path.as_str().split('/') {
            candidate.push(segment);
        }

        let canonical = fs::canonicalize(candidate).map_err(FileSystemError::Io)?;
        if !canonical.starts_with(&self.root) {
            return Err(FileSystemError::OutsideWorkspace);
        }
        Ok(canonical)
    }
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
}
