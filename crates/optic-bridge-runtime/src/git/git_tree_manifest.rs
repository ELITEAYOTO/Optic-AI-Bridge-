use optic_bridge_core::{GitObjectId, GitObjectIdError, WorkspacePath, WorkspacePathError};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitTreeEntryKind {
    RegularFile,
    ExecutableFile,
    Symlink,
    Gitlink,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitTreeEntry {
    pub path: WorkspacePath,
    pub kind: GitTreeEntryKind,
    pub object: GitObjectId,
    pub blob_size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitTreeManifest {
    pub head: GitObjectId,
    pub entries: Vec<GitTreeEntry>,
    pub total_blob_bytes: u64,
}

pub(crate) fn parse_git_tree_manifest(
    head: GitObjectId,
    bytes: &[u8],
    entry_limit: u32,
) -> Result<GitTreeManifest, GitTreeManifestError> {
    if !bytes.is_empty() && !bytes.ends_with(&[0]) {
        return Err(GitTreeManifestError::Malformed);
    }
    let mut entries = Vec::new();
    let mut total_blob_bytes = 0_u64;

    for record in bytes.split(|byte| *byte == 0) {
        if record.is_empty() {
            continue;
        }
        if entries.len() >= usize::try_from(entry_limit).unwrap_or(usize::MAX) {
            return Err(GitTreeManifestError::EntryLimitExceeded);
        }
        let entry = parse_record(record)?;
        if let Some(size) = entry.blob_size {
            total_blob_bytes = total_blob_bytes
                .checked_add(size)
                .ok_or(GitTreeManifestError::TotalSizeOverflow)?;
        }
        entries.push(entry);
    }

    entries.sort_by(|left, right| left.path.as_str().cmp(right.path.as_str()));
    if entries.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err(GitTreeManifestError::DuplicatePath);
    }

    Ok(GitTreeManifest {
        head,
        entries,
        total_blob_bytes,
    })
}

fn parse_record(record: &[u8]) -> Result<GitTreeEntry, GitTreeManifestError> {
    let tab = record
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or(GitTreeManifestError::Malformed)?;
    let (metadata, path_with_tab) = record.split_at(tab);
    let raw_path = &path_with_tab[1..];
    if raw_path.is_empty() {
        return Err(GitTreeManifestError::Malformed);
    }
    let path = std::str::from_utf8(raw_path).map_err(|_| GitTreeManifestError::NonUtf8Path)?;
    let path = WorkspacePath::parse(path).map_err(GitTreeManifestError::Path)?;

    let metadata = std::str::from_utf8(metadata).map_err(|_| GitTreeManifestError::Malformed)?;
    let fields = metadata.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() != 4 {
        return Err(GitTreeManifestError::Malformed);
    }
    let mode = fields[0];
    let object_type = fields[1];
    let object =
        GitObjectId::parse(fields[2].to_owned()).map_err(GitTreeManifestError::ObjectId)?;
    let size_field = fields[3];

    let (kind, blob_size) = match (mode, object_type) {
        ("100644", "blob") => (
            GitTreeEntryKind::RegularFile,
            Some(parse_blob_size(size_field)?),
        ),
        ("100755", "blob") => (
            GitTreeEntryKind::ExecutableFile,
            Some(parse_blob_size(size_field)?),
        ),
        ("120000", "blob") => (
            GitTreeEntryKind::Symlink,
            Some(parse_blob_size(size_field)?),
        ),
        ("160000", "commit") if size_field == "-" => (GitTreeEntryKind::Gitlink, None),
        _ => return Err(GitTreeManifestError::UnsupportedEntry),
    };

    Ok(GitTreeEntry {
        path,
        kind,
        object,
        blob_size,
    })
}

fn parse_blob_size(value: &str) -> Result<u64, GitTreeManifestError> {
    if value == "-" {
        return Err(GitTreeManifestError::Malformed);
    }
    value
        .parse::<u64>()
        .map_err(|_| GitTreeManifestError::Malformed)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GitTreeManifestError {
    #[error("Git tree manifest output is malformed")]
    Malformed,
    #[error("Git tree manifest contains a non-UTF-8 path")]
    NonUtf8Path,
    #[error("Git tree manifest path is invalid: {0}")]
    Path(WorkspacePathError),
    #[error("Git tree manifest object id is invalid: {0}")]
    ObjectId(GitObjectIdError),
    #[error("Git tree manifest contains an unsupported entry mode/type")]
    UnsupportedEntry,
    #[error("Git tree manifest entry count exceeded its hard ceiling")]
    EntryLimitExceeded,
    #[error("Git tree manifest contains a duplicate logical path")]
    DuplicatePath,
    #[error("Git tree manifest total blob size overflowed")]
    TotalSizeOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: char) -> String {
        std::iter::repeat_n(byte, 40).collect()
    }

    #[test]
    fn parser_is_deterministic_and_classifies_tree_entries() {
        let a = oid('1');
        let b = oid('2');
        let c = oid('3');
        let d = oid('4');
        let bytes = format!(
            "100755 blob {b} 7\tbin/run\0\
             100644 blob {a} 5\tsrc/lib.rs\0\
             120000 blob {c} 9\tlink\0\
             160000 commit {d} -\tvendor/submodule\0"
        );
        let head = GitObjectId::parse(oid('f')).expect("head");
        let manifest =
            parse_git_tree_manifest(head.clone(), bytes.as_bytes(), 4).expect("manifest");
        assert_eq!(manifest.head, head);
        assert_eq!(manifest.entries.len(), 4);
        assert_eq!(manifest.total_blob_bytes, 21);
        assert_eq!(manifest.entries[0].path.as_str(), "bin/run");
        assert_eq!(manifest.entries[0].kind, GitTreeEntryKind::ExecutableFile);
        assert_eq!(manifest.entries[1].path.as_str(), "link");
        assert_eq!(manifest.entries[1].kind, GitTreeEntryKind::Symlink);
        assert_eq!(manifest.entries[2].path.as_str(), "src/lib.rs");
        assert_eq!(manifest.entries[3].kind, GitTreeEntryKind::Gitlink);
    }

    #[test]
    fn parser_rejects_entry_limit_and_unsupported_mode() {
        let object = oid('a');
        let one = format!("100644 blob {object} 1\ta.txt\0");
        assert_eq!(
            parse_git_tree_manifest(
                GitObjectId::parse(oid('f')).expect("head"),
                one.as_bytes(),
                0,
            )
            .expect_err("entry bound"),
            GitTreeManifestError::EntryLimitExceeded
        );

        let unsupported = format!("040000 tree {object} -\tdir\0");
        assert_eq!(
            parse_git_tree_manifest(
                GitObjectId::parse(oid('f')).expect("head"),
                unsupported.as_bytes(),
                4,
            )
            .expect_err("tree records are not expected with recursive leaf listing"),
            GitTreeManifestError::UnsupportedEntry
        );
    }
}
