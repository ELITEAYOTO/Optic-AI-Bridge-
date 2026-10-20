use std::collections::{BTreeMap, BTreeSet};

use optic_bridge_core::{GitObjectId, SessionHandle, WorkspacePath};
use thiserror::Error;

use crate::{GitTreeEntry, GitTreeManifest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionChangeKind {
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionChange {
    pub path: WorkspacePath,
    pub kind: SessionChangeKind,
    pub base: Option<GitTreeEntry>,
    pub current: Option<GitTreeEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionChangeSet {
    pub owner: SessionHandle,
    pub base_head: GitObjectId,
    pub current_head: GitObjectId,
    pub changes: Vec<SessionChange>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionConflictKind {
    ExactPath,
    PathHierarchy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionConflict {
    pub left_path: WorkspacePath,
    pub right_path: WorkspacePath,
    pub kind: SessionConflictKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionConflictReport {
    pub base_head: GitObjectId,
    pub left_head: GitObjectId,
    pub right_head: GitObjectId,
    pub conflicts: Vec<SessionConflict>,
    pub truncated: bool,
}

pub(crate) fn build_change_set(
    owner: SessionHandle,
    base: &GitTreeManifest,
    current: &GitTreeManifest,
) -> SessionChangeSet {
    let base_by_path = base
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let current_by_path = current
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let paths = base_by_path
        .keys()
        .chain(current_by_path.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    let mut changes = Vec::new();
    for path in paths {
        let before = base_by_path.get(path).copied();
        let after = current_by_path.get(path).copied();
        let kind = match (before, after) {
            (None, Some(_)) => Some(SessionChangeKind::Added),
            (Some(_), None) => Some(SessionChangeKind::Deleted),
            (Some(before), Some(after)) if before != after => Some(SessionChangeKind::Modified),
            _ => None,
        };
        if let Some(kind) = kind {
            let workspace_path = before
                .map(|entry| entry.path.clone())
                .or_else(|| after.map(|entry| entry.path.clone()))
                .expect("a changed path exists in at least one manifest");
            changes.push(SessionChange {
                path: workspace_path,
                kind,
                base: before.cloned(),
                current: after.cloned(),
            });
        }
    }

    SessionChangeSet {
        owner,
        base_head: base.head.clone(),
        current_head: current.head.clone(),
        changes,
    }
}

pub(crate) fn detect_conflicts(
    left: &SessionChangeSet,
    right: &SessionChangeSet,
    max_conflicts: u32,
) -> Result<SessionConflictReport, SessionConflictError> {
    if left.owner == right.owner {
        return Err(SessionConflictError::SameOwner);
    }
    if left.base_head != right.base_head {
        return Err(SessionConflictError::DifferentBaseHeads);
    }
    if max_conflicts == 0 {
        return Err(SessionConflictError::InvalidLimit);
    }

    let right_paths = right
        .changes
        .iter()
        .map(|change| change.path.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let mut conflicts = Vec::new();
    let mut truncated = false;
    let limit = usize::try_from(max_conflicts).unwrap_or(usize::MAX);

    for left_change in &left.changes {
        let left_path = left_change.path.as_str();
        if right_paths.contains(left_path) {
            push_conflict(
                &mut conflicts,
                &mut truncated,
                limit,
                &left_change.path,
                WorkspacePath::parse(left_path).expect("change-set paths are already validated"),
                SessionConflictKind::ExactPath,
            );
        }

        for (index, byte) in left_path.bytes().enumerate() {
            if byte == b'/' {
                let ancestor = &left_path[..index];
                if right_paths.contains(ancestor) {
                    push_conflict(
                        &mut conflicts,
                        &mut truncated,
                        limit,
                        &left_change.path,
                        WorkspacePath::parse(ancestor)
                            .expect("ancestor of a workspace path is valid"),
                        SessionConflictKind::PathHierarchy,
                    );
                }
            }
        }

        let descendant_prefix = format!("{left_path}/");
        for right_path in right_paths.range(descendant_prefix.clone()..) {
            if !right_path.starts_with(&descendant_prefix) {
                break;
            }
            push_conflict(
                &mut conflicts,
                &mut truncated,
                limit,
                &left_change.path,
                WorkspacePath::parse(right_path.as_str())
                    .expect("change-set paths are already validated"),
                SessionConflictKind::PathHierarchy,
            );
        }
    }

    Ok(SessionConflictReport {
        base_head: left.base_head.clone(),
        left_head: left.current_head.clone(),
        right_head: right.current_head.clone(),
        conflicts,
        truncated,
    })
}

fn push_conflict(
    conflicts: &mut Vec<SessionConflict>,
    truncated: &mut bool,
    limit: usize,
    left_path: &WorkspacePath,
    right_path: WorkspacePath,
    kind: SessionConflictKind,
) {
    if conflicts.len() < limit {
        conflicts.push(SessionConflict {
            left_path: left_path.clone(),
            right_path,
            kind,
        });
    } else {
        *truncated = true;
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SessionConflictError {
    #[error("a session cannot be conflict-compared with itself")]
    SameOwner,
    #[error("session conflict comparison requires the same exact base HEAD")]
    DifferentBaseHeads,
    #[error("session conflict report limit must be non-zero")]
    InvalidLimit,
}

#[cfg(test)]
mod tests {
    use optic_bridge_core::GitObjectId;

    use super::*;
    use crate::GitTreeEntryKind;

    fn oid(byte: char) -> GitObjectId {
        GitObjectId::parse(std::iter::repeat_n(byte, 40).collect::<String>()).expect("oid")
    }

    fn entry(path: &str, object: char) -> GitTreeEntry {
        GitTreeEntry {
            path: WorkspacePath::parse(path).expect("path"),
            kind: GitTreeEntryKind::RegularFile,
            object: oid(object),
            blob_size: Some(1),
        }
    }

    fn manifest(head: char, entries: Vec<GitTreeEntry>) -> GitTreeManifest {
        GitTreeManifest {
            head: oid(head),
            entries,
            total_blob_bytes: 0,
        }
    }

    #[test]
    fn change_set_detects_add_modify_delete_in_sorted_order() {
        let owner = SessionHandle::generate().expect("owner");
        let base = manifest(
            '1',
            vec![
                entry("b.txt", 'b'),
                entry("keep.txt", 'd'),
                entry("z.txt", 'e'),
            ],
        );
        let current = manifest(
            '2',
            vec![
                entry("a.txt", 'a'),
                entry("b.txt", 'c'),
                entry("keep.txt", 'd'),
            ],
        );
        let changes = build_change_set(owner, &base, &current);
        assert_eq!(changes.changes.len(), 3);
        assert_eq!(changes.changes[0].path.as_str(), "a.txt");
        assert_eq!(changes.changes[0].kind, SessionChangeKind::Added);
        assert_eq!(changes.changes[1].path.as_str(), "b.txt");
        assert_eq!(changes.changes[1].kind, SessionChangeKind::Modified);
        assert_eq!(changes.changes[2].path.as_str(), "z.txt");
        assert_eq!(changes.changes[2].kind, SessionChangeKind::Deleted);
    }

    #[test]
    fn conflict_gate_detects_exact_and_parent_child_paths() {
        let base = oid('1');
        let left = SessionChangeSet {
            owner: SessionHandle::generate().expect("left"),
            base_head: base.clone(),
            current_head: oid('2'),
            changes: vec![
                SessionChange {
                    path: WorkspacePath::parse("same.txt").expect("path"),
                    kind: SessionChangeKind::Modified,
                    base: None,
                    current: None,
                },
                SessionChange {
                    path: WorkspacePath::parse("src").expect("path"),
                    kind: SessionChangeKind::Added,
                    base: None,
                    current: None,
                },
            ],
        };
        let right = SessionChangeSet {
            owner: SessionHandle::generate().expect("right"),
            base_head: base,
            current_head: oid('3'),
            changes: vec![
                SessionChange {
                    path: WorkspacePath::parse("same.txt").expect("path"),
                    kind: SessionChangeKind::Modified,
                    base: None,
                    current: None,
                },
                SessionChange {
                    path: WorkspacePath::parse("src/lib.rs").expect("path"),
                    kind: SessionChangeKind::Added,
                    base: None,
                    current: None,
                },
            ],
        };
        let report = detect_conflicts(&left, &right, 8).expect("report");
        assert_eq!(report.conflicts.len(), 2);
        assert_eq!(report.conflicts[0].kind, SessionConflictKind::ExactPath);
        assert_eq!(report.conflicts[1].kind, SessionConflictKind::PathHierarchy);
        assert!(!report.truncated);
    }

    #[test]
    fn conflict_gate_rejects_different_bases_and_bounds_report() {
        let left = SessionChangeSet {
            owner: SessionHandle::generate().expect("left"),
            base_head: oid('1'),
            current_head: oid('2'),
            changes: Vec::new(),
        };
        let mut right = SessionChangeSet {
            owner: SessionHandle::generate().expect("right"),
            base_head: oid('9'),
            current_head: oid('3'),
            changes: Vec::new(),
        };
        assert_eq!(
            detect_conflicts(&left, &right, 1).expect_err("different bases"),
            SessionConflictError::DifferentBaseHeads
        );

        right.base_head = left.base_head.clone();
        right.changes = vec![
            SessionChange {
                path: WorkspacePath::parse("same.txt").expect("path"),
                kind: SessionChangeKind::Modified,
                base: None,
                current: None,
            },
            SessionChange {
                path: WorkspacePath::parse("src/lib.rs").expect("path"),
                kind: SessionChangeKind::Modified,
                base: None,
                current: None,
            },
        ];
        let mut left = left;
        left.changes = vec![
            right.changes[0].clone(),
            SessionChange {
                path: WorkspacePath::parse("src").expect("path"),
                kind: SessionChangeKind::Added,
                base: None,
                current: None,
            },
        ];
        let report = detect_conflicts(&left, &right, 1).expect("bounded report");
        assert_eq!(report.conflicts.len(), 1);
        assert!(report.truncated);
    }
}
