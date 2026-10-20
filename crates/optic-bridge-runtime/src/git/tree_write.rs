use std::collections::BTreeMap;

use optic_bridge_core::{GitObjectId, GitObjectIdError};
use thiserror::Error;

use crate::{GitTreeEntry, GitTreeEntryKind, SessionMergePlan};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionMergeTree {
    pub tree: GitObjectId,
    pub base_head: GitObjectId,
    pub left_head: GitObjectId,
    pub right_head: GitObjectId,
    pub entry_count: u32,
    pub total_blob_bytes: u64,
}

#[derive(Debug, Default)]
pub(crate) struct MergeTreeDirectory {
    pub(crate) children: BTreeMap<String, MergeTreeNode>,
}

#[derive(Debug)]
pub(crate) enum MergeTreeNode {
    Directory(MergeTreeDirectory),
    Entry(GitTreeEntry),
}

pub(crate) fn build_directory_tree(
    plan: &SessionMergePlan,
) -> Result<MergeTreeDirectory, SessionMergeTreeError> {
    let mut root = MergeTreeDirectory::default();
    for entry in &plan.entries {
        if entry.kind == GitTreeEntryKind::Gitlink {
            return Err(SessionMergeTreeError::GitlinkUnsupported);
        }
        insert_entry(&mut root, entry.clone())?;
    }
    Ok(root)
}

fn insert_entry(
    root: &mut MergeTreeDirectory,
    entry: GitTreeEntry,
) -> Result<(), SessionMergeTreeError> {
    let segments = entry.path.as_str().split('/').collect::<Vec<_>>();
    let Some((last, parents)) = segments.split_last() else {
        return Err(SessionMergeTreeError::StructuralCollision);
    };
    let mut directory = root;
    for segment in parents {
        let node = directory
            .children
            .entry((*segment).to_owned())
            .or_insert_with(|| MergeTreeNode::Directory(MergeTreeDirectory::default()));
        match node {
            MergeTreeNode::Directory(child) => directory = child,
            MergeTreeNode::Entry(_) => return Err(SessionMergeTreeError::StructuralCollision),
        }
    }
    if directory.children.contains_key(*last) {
        return Err(SessionMergeTreeError::StructuralCollision);
    }
    directory
        .children
        .insert((*last).to_owned(), MergeTreeNode::Entry(entry));
    Ok(())
}

pub(crate) fn encode_mktree_directory(
    directory: &MergeTreeDirectory,
    child_trees: &BTreeMap<String, GitObjectId>,
    max_bytes: u64,
) -> Result<Vec<u8>, SessionMergeTreeError> {
    if max_bytes == 0 {
        return Err(SessionMergeTreeError::InputLimitExceeded { limit: 0 });
    }
    let capacity = usize::try_from(max_bytes.min(64 * 1024)).unwrap_or(64 * 1024);
    let mut input = Vec::with_capacity(capacity);

    for (name, node) in &directory.children {
        let (mode, object_type, object) = match node {
            MergeTreeNode::Directory(_) => {
                let object = child_trees
                    .get(name)
                    .ok_or(SessionMergeTreeError::MissingChildTree)?;
                ("040000", "tree", object)
            }
            MergeTreeNode::Entry(entry) => match entry.kind {
                GitTreeEntryKind::RegularFile => ("100644", "blob", &entry.object),
                GitTreeEntryKind::ExecutableFile => ("100755", "blob", &entry.object),
                GitTreeEntryKind::Symlink => ("120000", "blob", &entry.object),
                GitTreeEntryKind::Gitlink => return Err(SessionMergeTreeError::GitlinkUnsupported),
            },
        };
        let line = format!("{mode} {object_type} {}\t{name}", object.as_str());
        let required = input
            .len()
            .checked_add(line.len())
            .and_then(|value| value.checked_add(1))
            .ok_or(SessionMergeTreeError::InputSizeOverflow)?;
        if u64::try_from(required).unwrap_or(u64::MAX) > max_bytes {
            return Err(SessionMergeTreeError::InputLimitExceeded { limit: max_bytes });
        }
        input.extend_from_slice(line.as_bytes());
        input.push(0);
    }
    Ok(input)
}

pub(crate) fn parse_mktree_output(bytes: &[u8]) -> Result<GitObjectId, SessionMergeTreeError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| SessionMergeTreeError::MalformedTreeOutput)?;
    let trimmed = text.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() || trimmed.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return Err(SessionMergeTreeError::MalformedTreeOutput);
    }
    if !text[trimmed.len()..]
        .bytes()
        .all(|byte| byte == b'\r' || byte == b'\n')
    {
        return Err(SessionMergeTreeError::MalformedTreeOutput);
    }
    GitObjectId::parse(trimmed.to_owned()).map_err(SessionMergeTreeError::ObjectId)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionMergeTreeError {
    #[error("session merge-tree mktree input exceeds byte ceiling {limit}")]
    InputLimitExceeded { limit: u64 },
    #[error("session merge-tree mktree input size overflowed")]
    InputSizeOverflow,
    #[error("session merge-tree does not admit gitlink entries in this gate")]
    GitlinkUnsupported,
    #[error("session merge-tree contains a file/directory structural collision")]
    StructuralCollision,
    #[error("session merge-tree child tree object is missing during bottom-up construction")]
    MissingChildTree,
    #[error("session merge-tree object count exceeded its hard ceiling")]
    TreeObjectLimitExceeded,
    #[error("git mktree returned malformed output")]
    MalformedTreeOutput,
    #[error("git mktree returned an invalid object id: {0}")]
    ObjectId(GitObjectIdError),
    #[error("git mktree produced unexpected stderr")]
    MktreeUnexpectedStderr,
    #[error("written merge tree does not match the deterministic merge plan")]
    VerificationMismatch,
}

#[cfg(test)]
mod tests {
    use optic_bridge_core::{SessionHandle, WorkspacePath};

    use super::*;

    fn oid(byte: char) -> GitObjectId {
        GitObjectId::parse(std::iter::repeat_n(byte, 40).collect::<String>()).expect("oid")
    }

    fn entry(path: &str, kind: GitTreeEntryKind, object: char) -> GitTreeEntry {
        GitTreeEntry {
            path: WorkspacePath::parse(path).expect("path"),
            kind,
            object: oid(object),
            blob_size: Some(1),
        }
    }

    fn plan(entries: Vec<GitTreeEntry>) -> SessionMergePlan {
        SessionMergePlan {
            base_head: oid('1'),
            left_owner: SessionHandle::generate().expect("left"),
            right_owner: SessionHandle::generate().expect("right"),
            left_head: oid('2'),
            right_head: oid('3'),
            total_blob_bytes: entries.iter().filter_map(|entry| entry.blob_size).sum(),
            entries,
        }
    }

    #[test]
    fn directory_tree_is_deterministic_and_encodes_exact_modes() {
        let plan = plan(vec![
            entry("src/lib.rs", GitTreeEntryKind::RegularFile, 'a'),
            entry("bin/run", GitTreeEntryKind::ExecutableFile, 'b'),
            entry("link", GitTreeEntryKind::Symlink, 'c'),
        ]);
        let tree = build_directory_tree(&plan).expect("tree");
        let mut child_trees = BTreeMap::new();
        child_trees.insert("bin".to_owned(), oid('d'));
        child_trees.insert("src".to_owned(), oid('e'));
        let root = encode_mktree_directory(&tree, &child_trees, 1024).expect("root input");
        let root = String::from_utf8(root).expect("utf8");
        assert!(root.contains(&format!("040000 tree {}\tbin\0", oid('d').as_str())));
        assert!(root.contains(&format!("120000 blob {}\tlink\0", oid('c').as_str())));
        assert!(root.contains(&format!("040000 tree {}\tsrc\0", oid('e').as_str())));
    }

    #[test]
    fn gitlinks_and_structural_collisions_fail_closed() {
        assert_eq!(
            build_directory_tree(&plan(vec![entry(
                "vendor/submodule",
                GitTreeEntryKind::Gitlink,
                'a'
            )]))
            .expect_err("gitlink"),
            SessionMergeTreeError::GitlinkUnsupported
        );
        let collision = plan(vec![
            entry("src", GitTreeEntryKind::RegularFile, 'a'),
            entry("src/lib.rs", GitTreeEntryKind::RegularFile, 'b'),
        ]);
        assert_eq!(
            build_directory_tree(&collision).expect_err("collision"),
            SessionMergeTreeError::StructuralCollision
        );
    }

    #[test]
    fn mktree_output_parser_is_strict() {
        let object = oid('f');
        assert_eq!(
            parse_mktree_output(format!("{}\n", object.as_str()).as_bytes()).expect("tree oid"),
            object
        );
        assert_eq!(
            parse_mktree_output(format!("{}\nextra\n", oid('f').as_str()).as_bytes())
                .expect_err("extra output"),
            SessionMergeTreeError::MalformedTreeOutput
        );
    }
}
