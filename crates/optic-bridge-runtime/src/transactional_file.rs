use std::path::Path;

use optic_bridge_core::{ActionId, ContentVersion, ExpectedState, HardLimits, WorkspacePath};
use thiserror::Error;

use crate::{
    FileSystemError, JournaledDeleteCommit, JournaledMutationCommit, JournaledMutationError,
    JournaledMutationService, RecoveryReport,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytePatch {
    pub offset: u64,
    pub remove_bytes: u64,
    pub insert: Vec<u8>,
}

#[derive(Debug)]
pub struct TransactionalFileService {
    mutations: JournaledMutationService,
    max_mutation_bytes: u64,
    read_chunk_bytes: u64,
}

impl TransactionalFileService {
    pub fn from_hard_limits(
        workspace_root: impl AsRef<Path>,
        state_root: impl AsRef<Path>,
        limits: HardLimits,
    ) -> Result<Self, TransactionalFileError> {
        let mutations =
            JournaledMutationService::from_hard_limits(workspace_root, state_root, limits)?;
        Ok(Self {
            mutations,
            max_mutation_bytes: limits.max_fs_mutation_bytes,
            read_chunk_bytes: limits.max_fs_read_bytes.min(limits.max_fs_mutation_bytes),
        })
    }

    pub fn recover(&self) -> Result<RecoveryReport, TransactionalFileError> {
        Ok(self.mutations.recover()?)
    }

    pub fn write(
        &self,
        path: &WorkspacePath,
        expected: ExpectedState,
        content: &[u8],
    ) -> Result<JournaledMutationCommit, TransactionalFileError> {
        self.ensure_content_fits(content.len())?;
        let plan = self.mutations.prepare_write(path, expected)?;
        Ok(self.mutations.commit_write(&plan, content)?)
    }

    pub fn write_for_action(
        &self,
        action_id: &ActionId,
        path: &WorkspacePath,
        expected: ExpectedState,
        content: &[u8],
    ) -> Result<JournaledMutationCommit, TransactionalFileError> {
        self.ensure_content_fits(content.len())?;
        let plan = self.mutations.prepare_write(path, expected)?;
        Ok(self
            .mutations
            .commit_write_for_action(action_id, &plan, content)?)
    }

    pub fn delete(
        &self,
        path: &WorkspacePath,
        expected: ContentVersion,
    ) -> Result<JournaledDeleteCommit, TransactionalFileError> {
        let plan = self.mutations.prepare_delete(path, expected)?;
        Ok(self.mutations.commit_delete(&plan)?)
    }

    pub fn delete_for_action(
        &self,
        action_id: &ActionId,
        path: &WorkspacePath,
        expected: ContentVersion,
    ) -> Result<JournaledDeleteCommit, TransactionalFileError> {
        let plan = self.mutations.prepare_delete(path, expected)?;
        Ok(self.mutations.commit_delete_for_action(action_id, &plan)?)
    }

    pub fn apply_patch(
        &self,
        path: &WorkspacePath,
        expected: ContentVersion,
        patch: &BytePatch,
    ) -> Result<JournaledMutationCommit, TransactionalFileError> {
        let expected_state = ExpectedState::Content(expected);
        let plan = self.mutations.prepare_write(path, expected_state)?;
        let base = self.read_exact_snapshot(plan.canonical_path(), expected)?;
        let content = apply_byte_patch(&base, patch, self.max_mutation_bytes)?;
        Ok(self.mutations.commit_write(&plan, &content)?)
    }

    pub fn apply_patch_for_action(
        &self,
        action_id: &ActionId,
        path: &WorkspacePath,
        expected: ContentVersion,
        patch: &BytePatch,
    ) -> Result<JournaledMutationCommit, TransactionalFileError> {
        let expected_state = ExpectedState::Content(expected);
        let plan = self.mutations.prepare_write(path, expected_state)?;
        let base = self.read_exact_snapshot(plan.canonical_path(), expected)?;
        let content = apply_byte_patch(&base, patch, self.max_mutation_bytes)?;
        Ok(self
            .mutations
            .commit_write_for_action(action_id, &plan, &content)?)
    }

    fn read_exact_snapshot(
        &self,
        path: &WorkspacePath,
        expected: ContentVersion,
    ) -> Result<Vec<u8>, TransactionalFileError> {
        let mut bytes = Vec::new();
        let mut offset = 0_u64;

        loop {
            let chunk = self.mutations.atomic().filesystem().read(
                path,
                offset,
                Some(self.read_chunk_bytes),
            )?;
            if chunk.bytes.is_empty() && !chunk.eof {
                return Err(TransactionalFileError::SnapshotDidNotProgress);
            }

            let resulting_len = bytes.len().checked_add(chunk.bytes.len()).ok_or(
                TransactionalFileError::SnapshotTooLarge {
                    limit: self.max_mutation_bytes,
                },
            )?;
            if u64::try_from(resulting_len).map_err(|_| {
                TransactionalFileError::SnapshotTooLarge {
                    limit: self.max_mutation_bytes,
                }
            })? > self.max_mutation_bytes
            {
                return Err(TransactionalFileError::SnapshotTooLarge {
                    limit: self.max_mutation_bytes,
                });
            }

            bytes.extend_from_slice(&chunk.bytes);
            offset = chunk.next_offset;
            if chunk.eof {
                break;
            }
        }

        if ContentVersion::from_bytes(&bytes) != expected {
            return Err(TransactionalFileError::PatchBaseChanged);
        }
        Ok(bytes)
    }

    fn ensure_content_fits(&self, len: usize) -> Result<(), TransactionalFileError> {
        let len = u64::try_from(len).map_err(|_| TransactionalFileError::NewContentTooLarge {
            limit: self.max_mutation_bytes,
        })?;
        if len > self.max_mutation_bytes {
            return Err(TransactionalFileError::NewContentTooLarge {
                limit: self.max_mutation_bytes,
            });
        }
        Ok(())
    }
}

fn apply_byte_patch(
    base: &[u8],
    patch: &BytePatch,
    max_mutation_bytes: u64,
) -> Result<Vec<u8>, TransactionalFileError> {
    let offset =
        usize::try_from(patch.offset).map_err(|_| TransactionalFileError::PatchOutOfRange)?;
    let remove_bytes =
        usize::try_from(patch.remove_bytes).map_err(|_| TransactionalFileError::PatchOutOfRange)?;
    let end = offset
        .checked_add(remove_bytes)
        .ok_or(TransactionalFileError::PatchOutOfRange)?;
    if offset > base.len() || end > base.len() {
        return Err(TransactionalFileError::PatchOutOfRange);
    }

    let resulting_len = base
        .len()
        .checked_sub(remove_bytes)
        .and_then(|len| len.checked_add(patch.insert.len()))
        .ok_or(TransactionalFileError::NewContentTooLarge {
            limit: max_mutation_bytes,
        })?;
    if u64::try_from(resulting_len).map_err(|_| TransactionalFileError::NewContentTooLarge {
        limit: max_mutation_bytes,
    })? > max_mutation_bytes
    {
        return Err(TransactionalFileError::NewContentTooLarge {
            limit: max_mutation_bytes,
        });
    }

    let mut result = Vec::with_capacity(resulting_len);
    result.extend_from_slice(&base[..offset]);
    result.extend_from_slice(&patch.insert);
    result.extend_from_slice(&base[end..]);
    Ok(result)
}

#[derive(Debug, Error)]
pub enum TransactionalFileError {
    #[error("journaled mutation failed: {0}")]
    Mutation(#[from] JournaledMutationError),
    #[error("bounded snapshot read failed: {0}")]
    FileSystem(#[from] FileSystemError),
    #[error("new mutation content exceeds hard byte ceiling {limit}")]
    NewContentTooLarge { limit: u64 },
    #[error("patch base snapshot exceeds hard byte ceiling {limit}")]
    SnapshotTooLarge { limit: u64 },
    #[error("patch range is outside the exact expected base content")]
    PatchOutOfRange,
    #[error("patch base changed while the bounded snapshot was being read")]
    PatchBaseChanged,
    #[error("bounded patch snapshot read made no forward progress")]
    SnapshotDidNotProgress,
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use optic_bridge_core::ActionId;

    use super::*;

    fn fixture(label: &str) -> (PathBuf, PathBuf, PathBuf) {
        let token = ActionId::generate().expect("test entropy").to_token();
        let base = env::temp_dir().join(format!("optic-transactional-file-{label}-{token}"));
        let workspace = base.join("workspace");
        let state = base.join("state");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&state).expect("state");
        (base, workspace, state)
    }

    #[test]
    fn byte_patch_replaces_one_exact_range() {
        let patch = BytePatch {
            offset: 1,
            remove_bytes: 3,
            insert: b"XYZ".to_vec(),
        };
        let result = apply_byte_patch(b"alpha", &patch, 16).expect("patch");
        assert_eq!(result, b"aXYZa");
    }

    #[test]
    fn byte_patch_rejects_out_of_range_edits() {
        let patch = BytePatch {
            offset: 4,
            remove_bytes: 2,
            insert: Vec::new(),
        };
        assert!(matches!(
            apply_byte_patch(b"alpha", &patch, 16),
            Err(TransactionalFileError::PatchOutOfRange)
        ));
    }

    #[test]
    fn byte_patch_rejects_oversized_result() {
        let patch = BytePatch {
            offset: 1,
            remove_bytes: 1,
            insert: b"WXYZ".to_vec(),
        };
        assert!(matches!(
            apply_byte_patch(b"abc", &patch, 4),
            Err(TransactionalFileError::NewContentTooLarge { limit: 4 })
        ));
    }

    #[cfg(windows)]
    #[test]
    fn transactional_write_and_patch_reuse_journaled_commit() {
        let (base, workspace, state) = fixture("write-patch");
        let service =
            TransactionalFileService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");

        service
            .write(&path, ExpectedState::Absent, b"alpha")
            .expect("write");
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("read"),
            b"alpha"
        );

        let expected = ContentVersion::from_bytes(b"alpha");
        service
            .apply_patch(
                &path,
                expected,
                &BytePatch {
                    offset: 1,
                    remove_bytes: 3,
                    insert: b"XYZ".to_vec(),
                },
            )
            .expect("patch");
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("read patched"),
            b"aXYZa"
        );
        assert!(service.recover().expect("recovery").is_empty());

        fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn transactional_delete_reuses_journaled_commit() {
        let (base, workspace, state) = fixture("delete");
        fs::write(workspace.join("target.txt"), b"alpha").expect("fixture");
        let service =
            TransactionalFileService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");

        let result = service
            .delete(&path, ContentVersion::from_bytes(b"alpha"))
            .expect("delete");
        assert!(result.journal_retired);
        assert!(!workspace.join("target.txt").exists());
        assert!(service.recover().expect("recovery").is_empty());

        fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn patch_reads_and_verifies_base_across_multiple_chunks() {
        let (base, workspace, state) = fixture("chunked-patch");
        fs::write(workspace.join("target.txt"), b"abcdef").expect("fixture");
        let limits = HardLimits {
            max_fs_read_bytes: 2,
            ..HardLimits::default()
        };
        let service = TransactionalFileService::from_hard_limits(&workspace, &state, limits)
            .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");

        service
            .apply_patch(
                &path,
                ContentVersion::from_bytes(b"abcdef"),
                &BytePatch {
                    offset: 2,
                    remove_bytes: 2,
                    insert: b"XY".to_vec(),
                },
            )
            .expect("chunked patch");
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("read patched"),
            b"abXYef"
        );
        assert!(service.recover().expect("recovery").is_empty());

        fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn stale_patch_base_is_rejected_without_modifying_target() {
        let (base, workspace, state) = fixture("stale-patch");
        fs::write(workspace.join("target.txt"), b"alpha").expect("fixture");
        let service =
            TransactionalFileService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");

        assert!(
            service
                .apply_patch(
                    &path,
                    ContentVersion::from_bytes(b"stale"),
                    &BytePatch {
                        offset: 0,
                        remove_bytes: 1,
                        insert: b"A".to_vec(),
                    },
                )
                .is_err()
        );
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("read unchanged"),
            b"alpha"
        );
        assert!(service.recover().expect("recovery").is_empty());

        fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(not(windows))]
    #[test]
    fn transactional_write_remains_fail_closed_without_platform_commit() {
        let (base, workspace, state) = fixture("unsupported");
        let service =
            TransactionalFileService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");

        assert!(matches!(
            service.write(&path, ExpectedState::Absent, b"alpha"),
            Err(TransactionalFileError::Mutation(
                JournaledMutationError::UnsupportedPlatform
            ))
        ));
        assert!(!workspace.join("target.txt").exists());
        assert!(service.recover().expect("recovery").is_empty());

        fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(not(windows))]
    #[test]
    fn transactional_delete_remains_fail_closed_without_platform_commit() {
        let (base, workspace, state) = fixture("unsupported-delete");
        fs::write(workspace.join("target.txt"), b"alpha").expect("fixture");
        let service =
            TransactionalFileService::from_hard_limits(&workspace, &state, HardLimits::default())
                .expect("service");
        let path = WorkspacePath::parse("target.txt").expect("path");

        assert!(matches!(
            service.delete(&path, ContentVersion::from_bytes(b"alpha")),
            Err(TransactionalFileError::Mutation(
                JournaledMutationError::UnsupportedPlatform
            ))
        ));
        assert_eq!(
            fs::read(workspace.join("target.txt")).expect("target"),
            b"alpha"
        );
        assert!(service.recover().expect("recovery").is_empty());

        fs::remove_dir_all(base).expect("cleanup");
    }
}
