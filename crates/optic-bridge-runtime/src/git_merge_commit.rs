use optic_bridge_core::{GitObjectId, GitObjectIdError};
use thiserror::Error;

pub(crate) const MERGE_AUTHOR_NAME: &str = "Optic AI Bridge";
pub(crate) const MERGE_AUTHOR_EMAIL: &str = "optic@localhost.invalid";
const MERGE_MESSAGE: &[u8] = b"Optic deterministic session merge\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionMergeCommit {
    pub commit: GitObjectId,
    pub tree: GitObjectId,
    pub base_head: GitObjectId,
    pub left_head: GitObjectId,
    pub right_head: GitObjectId,
    pub commit_time_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObservedMergeCommit {
    pub(crate) tree: GitObjectId,
    pub(crate) parents: Vec<GitObjectId>,
    pub(crate) commit_time_unix_seconds: u64,
}

pub(crate) fn canonical_merge_message() -> &'static [u8] {
    MERGE_MESSAGE
}

pub(crate) fn deterministic_merge_time(
    left: u64,
    right: u64,
) -> Result<u64, SessionMergeCommitError> {
    left.max(right)
        .checked_add(1)
        .ok_or(SessionMergeCommitError::TimestampOverflow)
}

pub(crate) fn parse_commit_time(bytes: &[u8]) -> Result<u64, SessionMergeCommitError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| SessionMergeCommitError::MalformedTimestamp)?;
    let trimmed = trim_line_ending(text);
    if trimmed.is_empty() || trimmed.bytes().any(|byte| !byte.is_ascii_digit()) {
        return Err(SessionMergeCommitError::MalformedTimestamp);
    }
    trimmed
        .parse::<u64>()
        .map_err(|_| SessionMergeCommitError::MalformedTimestamp)
}

pub(crate) fn parse_commit_output(bytes: &[u8]) -> Result<GitObjectId, SessionMergeCommitError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| SessionMergeCommitError::MalformedCommitOutput)?;
    let trimmed = trim_line_ending(text);
    if trimmed.is_empty() || trimmed.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return Err(SessionMergeCommitError::MalformedCommitOutput);
    }
    GitObjectId::parse(trimmed.to_owned()).map_err(SessionMergeCommitError::ObjectId)
}

pub(crate) fn parse_commit_metadata(
    bytes: &[u8],
) -> Result<ObservedMergeCommit, SessionMergeCommitError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| SessionMergeCommitError::MalformedCommitMetadata)?;
    let trimmed = trim_line_ending(text);
    if trimmed.contains(['\r', '\n']) {
        return Err(SessionMergeCommitError::MalformedCommitMetadata);
    }
    let mut fields = trimmed.split('\t');
    let tree = fields
        .next()
        .ok_or(SessionMergeCommitError::MalformedCommitMetadata)?;
    let parents = fields
        .next()
        .ok_or(SessionMergeCommitError::MalformedCommitMetadata)?;
    let timestamp = fields
        .next()
        .ok_or(SessionMergeCommitError::MalformedCommitMetadata)?;
    if fields.next().is_some() || tree.is_empty() || timestamp.is_empty() {
        return Err(SessionMergeCommitError::MalformedCommitMetadata);
    }
    let tree = GitObjectId::parse(tree.to_owned()).map_err(SessionMergeCommitError::ObjectId)?;
    let parents = if parents.is_empty() {
        Vec::new()
    } else {
        parents
            .split_ascii_whitespace()
            .map(|value| {
                GitObjectId::parse(value.to_owned()).map_err(SessionMergeCommitError::ObjectId)
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let commit_time_unix_seconds = timestamp
        .parse::<u64>()
        .map_err(|_| SessionMergeCommitError::MalformedCommitMetadata)?;
    Ok(ObservedMergeCommit {
        tree,
        parents,
        commit_time_unix_seconds,
    })
}

fn trim_line_ending(text: &str) -> &str {
    text.trim_end_matches(['\r', '\n'])
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionMergeCommitError {
    #[error("deterministic merge commit timestamp overflowed")]
    TimestampOverflow,
    #[error("Git commit timestamp output was malformed")]
    MalformedTimestamp,
    #[error("git commit-tree returned malformed output")]
    MalformedCommitOutput,
    #[error("Git merge commit metadata was malformed")]
    MalformedCommitMetadata,
    #[error("Git merge commit message exceeds byte ceiling {limit}")]
    MessageLimitExceeded { limit: u64 },
    #[error("Git merge commit command produced unexpected stderr")]
    UnexpectedStderr,
    #[error("created merge commit does not match its deterministic contract")]
    VerificationMismatch,
    #[error("Git merge commit contained an invalid object id: {0}")]
    ObjectId(GitObjectIdError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: char) -> GitObjectId {
        GitObjectId::parse(std::iter::repeat_n(byte, 40).collect::<String>()).expect("oid")
    }

    #[test]
    fn deterministic_time_is_parent_derived_and_checked() {
        assert_eq!(deterministic_merge_time(10, 14).expect("time"), 15);
        assert_eq!(
            deterministic_merge_time(u64::MAX, 1).expect_err("overflow"),
            SessionMergeCommitError::TimestampOverflow
        );
    }

    #[test]
    fn commit_output_and_metadata_parsers_are_strict() {
        let commit = oid('a');
        assert_eq!(
            parse_commit_output(format!("{}\n", commit.as_str()).as_bytes()).expect("commit"),
            commit
        );
        assert_eq!(
            parse_commit_output(format!("{} extra\n", commit.as_str()).as_bytes())
                .expect_err("extra output"),
            SessionMergeCommitError::MalformedCommitOutput
        );

        let tree = oid('b');
        let left = oid('c');
        let right = oid('d');
        let metadata = format!(
            "{}\t{} {}\t42\n",
            tree.as_str(),
            left.as_str(),
            right.as_str()
        );
        let parsed = parse_commit_metadata(metadata.as_bytes()).expect("metadata");
        assert_eq!(parsed.tree, tree);
        assert_eq!(parsed.parents, vec![left, right]);
        assert_eq!(parsed.commit_time_unix_seconds, 42);
        assert_eq!(parse_commit_time(b"42\n").expect("time"), 42);
    }
}
