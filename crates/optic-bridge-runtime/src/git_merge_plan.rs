use std::collections::BTreeMap;

use optic_bridge_core::{GitObjectId, SessionHandle};
use thiserror::Error;

use crate::{
    GitTreeEntry, GitTreeManifest, SessionChange, SessionChangeKind, SessionChangeSet,
    SessionConflictError, SessionConflictReport, git_change_set::detect_conflicts,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionMergePlan {
    pub base_head: GitObjectId,
    pub left_owner: SessionHandle,
    pub right_owner: SessionHandle,
    pub left_head: GitObjectId,
    pub right_head: GitObjectId,
    pub entries: Vec<GitTreeEntry>,
    pub total_blob_bytes: u64,
}

pub(crate) fn build_merge_plan(
    base: &GitTreeManifest,
    left: &SessionChangeSet,
    right: &SessionChangeSet,
    max_conflicts: u32,
) -> Result<SessionMergePlan, SessionMergePlanError> {
    if base.head != left.base_head || base.head != right.base_head {
        return Err(SessionMergePlanError::BaseManifestMismatch);
    }
    let conflicts = detect_conflicts(left, right, max_conflicts)?;
    if conflicts.truncated || !conflicts.conflicts.is_empty() {
        return Err(SessionMergePlanError::Conflicts(conflicts));
    }

    let mut entries = base
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.as_str().to_owned(), entry))
        .collect::<BTreeMap<_, _>>();
    apply_changes(&mut entries, &left.changes)?;
    apply_changes(&mut entries, &right.changes)?;

    let mut total_blob_bytes = 0_u64;
    for entry in entries.values() {
        if let Some(size) = entry.blob_size {
            total_blob_bytes = total_blob_bytes
                .checked_add(size)
                .ok_or(SessionMergePlanError::SizeOverflow)?;
        }
    }

    Ok(SessionMergePlan {
        base_head: base.head.clone(),
        left_owner: left.owner.clone(),
        right_owner: right.owner.clone(),
        left_head: left.current_head.clone(),
        right_head: right.current_head.clone(),
        entries: entries.into_values().collect(),
        total_blob_bytes,
    })
}

fn apply_changes(
    entries: &mut BTreeMap<String, GitTreeEntry>,
    changes: &[SessionChange],
) -> Result<(), SessionMergePlanError> {
    for change in changes {
        let key = change.path.as_str().to_owned();
        match change.kind {
            SessionChangeKind::Added => {
                if change.base.is_some() || entries.contains_key(&key) {
                    return Err(SessionMergePlanError::InvalidChangeSet);
                }
                let current = change
                    .current
                    .as_ref()
                    .ok_or(SessionMergePlanError::InvalidChangeSet)?;
                if current.path != change.path {
                    return Err(SessionMergePlanError::InvalidChangeSet);
                }
                entries.insert(key, current.clone());
            }
            SessionChangeKind::Modified => {
                let base = change
                    .base
                    .as_ref()
                    .ok_or(SessionMergePlanError::InvalidChangeSet)?;
                let current = change
                    .current
                    .as_ref()
                    .ok_or(SessionMergePlanError::InvalidChangeSet)?;
                if base.path != change.path || current.path != change.path {
                    return Err(SessionMergePlanError::InvalidChangeSet);
                }
                let Some(observed) = entries.get(&key) else {
                    return Err(SessionMergePlanError::InvalidChangeSet);
                };
                if observed != base {
                    return Err(SessionMergePlanError::InvalidChangeSet);
                }
                entries.insert(key, current.clone());
            }
            SessionChangeKind::Deleted => {
                let base = change
                    .base
                    .as_ref()
                    .ok_or(SessionMergePlanError::InvalidChangeSet)?;
                if base.path != change.path || change.current.is_some() {
                    return Err(SessionMergePlanError::InvalidChangeSet);
                }
                let Some(observed) = entries.remove(&key) else {
                    return Err(SessionMergePlanError::InvalidChangeSet);
                };
                if observed != *base {
                    return Err(SessionMergePlanError::InvalidChangeSet);
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionMergePlanError {
    #[error("merge-plan base manifest does not match both session base HEADs")]
    BaseManifestMismatch,
    #[error("session change set is internally inconsistent with its exact base manifest")]
    InvalidChangeSet,
    #[error("session current HEAD is not a descendant of its exact base HEAD")]
    HeadNotDescendant,
    #[error("session merge plan contains conflicts")]
    Conflicts(SessionConflictReport),
    #[error("session merge-plan aggregate blob size overflowed")]
    SizeOverflow,
    #[error(transparent)]
    ConflictGate(#[from] SessionConflictError),
}

#[cfg(test)]
mod tests {
    use optic_bridge_core::WorkspacePath;

    use super::*;
    use crate::{GitTreeEntryKind, SessionChangeKind};

    fn oid(byte: char) -> GitObjectId {
        GitObjectId::parse(std::iter::repeat_n(byte, 40).collect::<String>()).expect("oid")
    }

    fn entry(path: &str, object: char, size: u64) -> GitTreeEntry {
        GitTreeEntry {
            path: WorkspacePath::parse(path).expect("path"),
            kind: GitTreeEntryKind::RegularFile,
            object: oid(object),
            blob_size: Some(size),
        }
    }

    #[test]
    fn disjoint_session_changes_produce_deterministic_merged_tree() {
        let base_a = entry("a.txt", '1', 1);
        let base_b = entry("b.txt", '2', 2);
        let base = GitTreeManifest {
            head: oid('a'),
            entries: vec![base_a.clone(), base_b.clone()],
            total_blob_bytes: 3,
        };
        let left_a = entry("a.txt", '3', 3);
        let right_c = entry("c.txt", '4', 4);
        let left = SessionChangeSet {
            owner: SessionHandle::generate().expect("left"),
            base_head: base.head.clone(),
            current_head: oid('b'),
            changes: vec![SessionChange {
                path: base_a.path.clone(),
                kind: SessionChangeKind::Modified,
                base: Some(base_a),
                current: Some(left_a.clone()),
            }],
        };
        let right = SessionChangeSet {
            owner: SessionHandle::generate().expect("right"),
            base_head: base.head.clone(),
            current_head: oid('c'),
            changes: vec![SessionChange {
                path: right_c.path.clone(),
                kind: SessionChangeKind::Added,
                base: None,
                current: Some(right_c.clone()),
            }],
        };

        let plan = build_merge_plan(&base, &left, &right, 8).expect("merge plan");
        assert_eq!(
            plan.entries
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["a.txt", "b.txt", "c.txt"]
        );
        assert_eq!(plan.entries[0], left_a);
        assert_eq!(plan.entries[1], base_b);
        assert_eq!(plan.entries[2], right_c);
        assert_eq!(plan.total_blob_bytes, 9);
    }

    #[test]
    fn conflicting_sessions_never_produce_a_merge_plan() {
        let base_entry = entry("same.txt", '1', 1);
        let base = GitTreeManifest {
            head: oid('a'),
            entries: vec![base_entry.clone()],
            total_blob_bytes: 1,
        };
        let change = |owner: SessionHandle, head: char, object: char| SessionChangeSet {
            owner,
            base_head: base.head.clone(),
            current_head: oid(head),
            changes: vec![SessionChange {
                path: base_entry.path.clone(),
                kind: SessionChangeKind::Modified,
                base: Some(base_entry.clone()),
                current: Some(entry("same.txt", object, 1)),
            }],
        };
        let left = change(SessionHandle::generate().expect("left"), 'b', '2');
        let right = change(SessionHandle::generate().expect("right"), 'c', '3');
        assert!(matches!(
            build_merge_plan(&base, &left, &right, 8),
            Err(SessionMergePlanError::Conflicts(_))
        ));
    }
}
