//! Data-only input contract for a future private build stage. A manifest is not
//! a read grant: adapters must authorize every input before collecting bytes.
use std::{collections::BTreeMap, fmt};

use thiserror::Error;

use crate::{ContentVersion, SessionHandle, TaskLeaseId, WorkspacePath};

const MAX_FILES: usize = 256;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PATH_BYTES: usize = 240;
const MAX_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectInputSpec {
    pub path: WorkspacePath,
    pub version: ContentVersion,
    pub size_bytes: u64,
}

/// Owns immutable, bounded expectations, bound to one task authorization epoch.
/// It neither checks live authorization nor grants filesystem/process access.
#[derive(Debug)]
pub struct ProjectInputManifest {
    session: SessionHandle,
    task: TaskLeaseId,
    policy_epoch: u64,
    files: Vec<ProjectInputSpec>,
    total_bytes: u64,
}

impl ProjectInputManifest {
    pub fn new(
        session: SessionHandle,
        task: TaskLeaseId,
        policy_epoch: u64,
        inputs: impl IntoIterator<Item = ProjectInputSpec>,
    ) -> Result<Self, ProjectInputError> {
        if policy_epoch == 0 {
            return Err(ProjectInputError::Binding);
        }
        let mut files = Vec::new();
        let mut names = BTreeMap::<String, bool>::new();
        let mut total_bytes = 0_u64;
        for input in inputs {
            if files.len() == MAX_FILES || input.size_bytes > MAX_FILE_BYTES {
                return Err(ProjectInputError::Capacity);
            }
            total_bytes = total_bytes
                .checked_add(input.size_bytes)
                .filter(|total| *total <= MAX_TOTAL_BYTES)
                .ok_or(ProjectInputError::Capacity)?;
            let key = portable_key(&input.path)?;
            // Track file and implied-directory names, including case, so inputs
            // cannot collide during materialization on a Windows filesystem.
            if names.contains_key(&key) {
                return Err(ProjectInputError::Collision);
            }
            let mut parent = key.as_str();
            while let Some((prefix, _)) = parent.rsplit_once('/') {
                if names.get(prefix) == Some(&true) {
                    return Err(ProjectInputError::Collision);
                }
                names.entry(prefix.to_owned()).or_insert(false);
                parent = prefix;
            }
            names.insert(key, true);
            files.push(input);
        }
        if files.is_empty() {
            return Err(ProjectInputError::Capacity);
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self {
            session,
            task,
            policy_epoch,
            files,
            total_bytes,
        })
    }

    pub fn require_binding(
        &self,
        session: &SessionHandle,
        task: &TaskLeaseId,
        policy_epoch: u64,
    ) -> Result<(), ProjectInputError> {
        if &self.session != session || &self.task != task || self.policy_epoch != policy_epoch {
            return Err(ProjectInputError::Binding);
        }
        Ok(())
    }

    #[must_use]
    pub fn files(&self) -> &[ProjectInputSpec] {
        &self.files
    }

    #[must_use]
    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Consumes already-authorized complete reads. No live paths are opened and
    /// no serialized/model-supplied snapshot can bypass this verification.
    pub fn seal(
        self,
        inputs: impl IntoIterator<Item = (WorkspacePath, Vec<u8>)>,
    ) -> Result<ProjectInputSnapshot, ProjectInputError> {
        let mut collected = BTreeMap::new();
        for (path, bytes) in inputs {
            let expected = self
                .files
                .binary_search_by(|spec| spec.path.cmp(&path))
                .map(|index| &self.files[index])
                .map_err(|_| ProjectInputError::UnexpectedInput)?;
            if bytes.len() as u64 != expected.size_bytes
                || ContentVersion::from_bytes(&bytes) != expected.version
            {
                return Err(ProjectInputError::ContentMismatch);
            }
            if collected.insert(path, bytes).is_some() {
                return Err(ProjectInputError::Collision);
            }
        }
        if collected.len() != self.files.len() {
            return Err(ProjectInputError::MissingInput);
        }
        Ok(ProjectInputSnapshot {
            manifest: self,
            contents: collected,
        })
    }
}

/// A bounded in-memory copy, never a promise of a globally atomic directory
/// snapshot. Consumers still need live session/task admission at execution.
pub struct ProjectInputSnapshot {
    manifest: ProjectInputManifest,
    contents: BTreeMap<WorkspacePath, Vec<u8>>,
}

impl ProjectInputSnapshot {
    #[must_use]
    pub fn manifest(&self) -> &ProjectInputManifest {
        &self.manifest
    }

    #[must_use]
    pub fn content(&self, path: &WorkspacePath) -> Option<&[u8]> {
        self.contents.get(path).map(Vec::as_slice)
    }
}

impl fmt::Debug for ProjectInputSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProjectInputSnapshot")
            .field("files", &self.manifest.files.len())
            .field("total_bytes", &self.manifest.total_bytes)
            .finish_non_exhaustive()
    }
}

fn portable_key(path: &WorkspacePath) -> Result<String, ProjectInputError> {
    let name = path.as_str();
    if name.len() > MAX_PATH_BYTES || name.split('/').count() > MAX_DEPTH {
        return Err(ProjectInputError::Path);
    }
    for segment in name.split('/') {
        if !segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            || segment.ends_with('.')
        {
            return Err(ProjectInputError::Path);
        }
        let lower = segment.to_ascii_lowercase();
        let stem = lower.split('.').next().unwrap_or_default();
        if matches!(
            lower.as_str(),
            ".git" | ".optic" | "node_modules" | "target"
        ) || matches!(stem, "con" | "prn" | "aux" | "nul")
            || ((stem.starts_with("com") || stem.starts_with("lpt"))
                && stem.len() == 4
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(ProjectInputError::Path);
        }
    }
    Ok(name.to_ascii_lowercase())
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ProjectInputError {
    #[error("project input ownership or epoch does not match")]
    Binding,
    #[error("project inputs exceed the finite snapshot budget")]
    Capacity,
    #[error("project input path cannot be safely materialized")]
    Path,
    #[error("project inputs have a duplicate or file/directory collision")]
    Collision,
    #[error("project input bytes do not match the expected size and version")]
    ContentMismatch,
    #[error("project snapshot contains an undeclared input")]
    UnexpectedInput,
    #[error("project snapshot is missing a declared input")]
    MissingInput,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> SessionHandle {
        SessionHandle::from_token(&"11".repeat(32)).unwrap()
    }
    fn task() -> TaskLeaseId {
        TaskLeaseId::from_token(&"22".repeat(32)).unwrap()
    }
    fn spec(path: &str, bytes: &[u8]) -> ProjectInputSpec {
        ProjectInputSpec {
            path: WorkspacePath::parse(path).unwrap(),
            version: ContentVersion::from_bytes(bytes),
            size_bytes: bytes.len() as u64,
        }
    }
    fn manifest(inputs: Vec<ProjectInputSpec>) -> Result<ProjectInputManifest, ProjectInputError> {
        ProjectInputManifest::new(session(), task(), 1, inputs)
    }

    #[test]
    fn seals_nested_modules_in_deterministic_order_without_debug_content() {
        let snapshot = manifest(vec![
            spec("src/main.cjs", b"SECRET"),
            spec("package.json", b"{}"),
        ])
        .unwrap()
        .seal([
            (
                WorkspacePath::parse("package.json").unwrap(),
                b"{}".to_vec(),
            ),
            (
                WorkspacePath::parse("src/main.cjs").unwrap(),
                b"SECRET".to_vec(),
            ),
        ])
        .unwrap();
        assert_eq!(snapshot.manifest().files()[0].path.as_str(), "package.json");
        assert_eq!(snapshot.manifest().total_bytes(), 8);
        assert_eq!(
            snapshot.content(&WorkspacePath::parse("src/main.cjs").unwrap()),
            Some(b"SECRET".as_slice())
        );
        assert!(!format!("{snapshot:?}").contains("SECRET"));
    }

    #[test]
    fn binding_does_not_transfer_between_sessions_tasks_or_epochs() {
        let value = manifest(vec![spec("main.cjs", b"")]).unwrap();
        value.require_binding(&session(), &task(), 1).unwrap();
        assert_eq!(
            value.require_binding(&session(), &task(), 2),
            Err(ProjectInputError::Binding)
        );
        assert!(
            value
                .require_binding(
                    &SessionHandle::from_token(&"33".repeat(32)).unwrap(),
                    &task(),
                    1
                )
                .is_err()
        );
        assert!(
            value
                .require_binding(
                    &session(),
                    &TaskLeaseId::from_token(&"33".repeat(32)).unwrap(),
                    1
                )
                .is_err()
        );
        assert!(ProjectInputManifest::new(session(), task(), 0, [spec("main.cjs", b"")]).is_err());
    }

    #[test]
    fn windows_aliases_control_names_and_prefix_collisions_are_denied() {
        for path in [
            "src/NUL.js",
            "con",
            "LPT9.txt",
            "aux.cjs",
            "src/trailing.",
            "src/a b",
            "src/é.js",
            ".git/config",
            "node_modules/a.js",
            ".optic/policy",
            "target/release/a",
        ] {
            assert!(manifest(vec![spec(path, b"")]).is_err(), "{path}");
        }
        for pair in [
            ["src/A.js", "src/a.js"],
            ["src", "src/a.js"],
            ["src/a.js", "src"],
        ] {
            assert!(matches!(
                manifest(pair.map(|path| spec(path, b"")).into()),
                Err(ProjectInputError::Collision)
            ));
        }
        assert!(manifest(vec![spec(&format!("{}a", "d/".repeat(16)), b"")]).is_err());
    }

    #[test]
    fn finite_limits_apply_before_retaining_inputs() {
        assert!(manifest(vec![]).is_err());
        let mut too_large = spec("a", b"");
        too_large.size_bytes = MAX_FILE_BYTES + 1;
        assert!(matches!(
            manifest(vec![too_large]),
            Err(ProjectInputError::Capacity)
        ));
        let inputs = (0..9)
            .map(|i| {
                let mut value = spec(&format!("f{i}"), b"");
                value.size_bytes = MAX_FILE_BYTES;
                value
            })
            .collect();
        assert!(matches!(manifest(inputs), Err(ProjectInputError::Capacity)));
        assert!(matches!(
            manifest((0..257).map(|i| spec(&format!("f{i}"), b"")).collect()),
            Err(ProjectInputError::Capacity)
        ));
    }

    #[test]
    fn exact_versions_missing_extra_and_duplicate_contents_fail_closed() {
        let path = WorkspacePath::parse("main.cjs").unwrap();
        assert!(matches!(
            manifest(vec![spec("main.cjs", b"ab")])
                .unwrap()
                .seal([(path.clone(), b"ac".to_vec())]),
            Err(ProjectInputError::ContentMismatch)
        ));
        assert!(matches!(
            manifest(vec![spec("main.cjs", b"ab")])
                .unwrap()
                .seal([(path.clone(), b"a".to_vec())]),
            Err(ProjectInputError::ContentMismatch)
        ));
        assert!(matches!(
            manifest(vec![spec("main.cjs", b"ab")]).unwrap().seal([]),
            Err(ProjectInputError::MissingInput)
        ));
        assert!(matches!(
            manifest(vec![spec("other", b"ab")])
                .unwrap()
                .seal([(path.clone(), b"ab".to_vec())]),
            Err(ProjectInputError::UnexpectedInput)
        ));
        assert!(matches!(
            manifest(vec![spec("main.cjs", b"ab")])
                .unwrap()
                .seal([(path.clone(), b"ab".to_vec()), (path, b"ab".to_vec())]),
            Err(ProjectInputError::Collision)
        ));
    }
}
