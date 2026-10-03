use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use optic_bridge_core::{
    ActionId, ContentVersion, ExpectedState, HardLimits, IdError, TokenParseError, WorkspacePath,
    WorkspacePathError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{BoundedFileSystem, MutationError, PreparedMutation};

const JOURNAL_VERSION_V1: u32 = 1;
const JOURNAL_VERSION_V2: u32 = 2;
const JOURNAL_VERSION_CURRENT: u32 = JOURNAL_VERSION_V2;
// This directory name is the stable storage-layout version. Record schemas
// inside it are independently versioned and currently accept v1 + v2.
const JOURNAL_DIR_NAME: &str = "mutation-journal-v1";
const JOURNAL_SUFFIX: &str = ".journal";
const PREPARED_TEMP_SUFFIX: &str = ".prepared.tmp";

#[derive(Debug)]
pub struct MutationRecoveryJournal {
    root: PathBuf,
    max_file_bytes: u64,
    max_records: usize,
}

impl MutationRecoveryJournal {
    pub fn open(
        state_root: impl AsRef<Path>,
        workspace_root: &Path,
        limits: HardLimits,
    ) -> Result<Self, RecoveryJournalError> {
        if limits.max_mutation_journal_file_bytes == 0 || limits.max_mutation_recovery_records == 0
        {
            return Err(RecoveryJournalError::InvalidLimits);
        }

        fs::create_dir_all(state_root.as_ref())?;
        let state_root = fs::canonicalize(state_root.as_ref())?;
        let workspace_root = fs::canonicalize(workspace_root)?;
        if state_root == workspace_root || state_root.starts_with(&workspace_root) {
            return Err(RecoveryJournalError::StateDirectoryInsideWorkspace);
        }

        let root = state_root.join(JOURNAL_DIR_NAME);
        fs::create_dir_all(&root)?;
        let root = fs::canonicalize(root)?;
        if root == workspace_root || root.starts_with(&workspace_root) {
            return Err(RecoveryJournalError::StateDirectoryInsideWorkspace);
        }

        Ok(Self {
            root,
            max_file_bytes: limits.max_mutation_journal_file_bytes,
            max_records: usize::try_from(limits.max_mutation_recovery_records)
                .map_err(|_| RecoveryJournalError::InvalidLimits)?,
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn begin(
        &self,
        plan: &PreparedMutation,
        intended: ExpectedState,
    ) -> Result<JournalTicket, RecoveryJournalError> {
        let action_id = ActionId::generate()?;
        self.begin_for_action(&action_id, plan, intended)
    }

    pub fn begin_for_action(
        &self,
        action_id: &ActionId,
        plan: &PreparedMutation,
        intended: ExpectedState,
    ) -> Result<JournalTicket, RecoveryJournalError> {
        self.ensure_action_path_absent(&self.prepared_temp_path(action_id), action_id)?;
        self.ensure_action_path_absent(&self.journal_path(action_id), action_id)?;
        self.ensure_record_capacity()?;
        let ticket = JournalTicket {
            action_id: action_id.clone(),
            canonical_path: plan.canonical_path().clone(),
            previous: plan.expected_state(),
            intended,
        };
        let bytes = serialize_line(&ticket.entry(JournalState::Prepared))?;
        self.ensure_file_size(0, bytes.len())?;

        let temp_path = self.prepared_temp_path(&ticket.action_id);
        let journal_path = self.journal_path(&ticket.action_id);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = options.open(&temp_path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                RecoveryJournalError::ActionAlreadyActive {
                    action_id: ticket.action_id.clone(),
                }
            } else {
                RecoveryJournalError::Io(error)
            }
        })?;
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&temp_path);
            return Err(RecoveryJournalError::Io(error));
        }
        drop(file);

        if let Err(error) = self.ensure_action_path_absent(&journal_path, &ticket.action_id) {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
        if let Err(error) = fs::rename(&temp_path, &journal_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(RecoveryJournalError::Io(error));
        }
        Ok(ticket)
    }

    pub fn mark_committing(&self, ticket: &JournalTicket) -> Result<(), RecoveryJournalError> {
        self.append(ticket, JournalState::Committing)
    }

    pub fn mark_verified(&self, ticket: &JournalTicket) -> Result<(), RecoveryJournalError> {
        self.append(ticket, JournalState::Verified)
    }

    pub fn mark_ambiguous(&self, ticket: &JournalTicket) -> Result<(), RecoveryJournalError> {
        self.append(ticket, JournalState::Ambiguous)
    }

    pub fn retire(&self, ticket: &JournalTicket) -> Result<(), RecoveryJournalError> {
        self.remove_regular_file(&self.journal_path(&ticket.action_id))
    }

    pub fn reconcile(
        &self,
        filesystem: &BoundedFileSystem,
    ) -> Result<RecoveryReport, RecoveryJournalError> {
        let report = self.inspect_pending(filesystem)?;
        for record in &report.records {
            self.retire_action(&record.action_id)?;
        }
        Ok(report)
    }

    pub(crate) fn inspect_pending(
        &self,
        filesystem: &BoundedFileSystem,
    ) -> Result<RecoveryReport, RecoveryJournalError> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            if paths.len() >= self.max_records {
                return Err(RecoveryJournalError::RecoveryRecordLimitExceeded {
                    limit: self.max_records,
                });
            }
            paths.push(entry?.path());
        }
        paths.sort();

        let mut records = Vec::new();
        for path in paths {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(RecoveryJournalError::UnexpectedJournalEntry)?;

            if let Some(token) = file_name.strip_suffix(PREPARED_TEMP_SUFFIX) {
                ActionId::from_token(token)?;
                self.remove_regular_file(&path)?;
                continue;
            }

            let token = file_name
                .strip_suffix(JOURNAL_SUFFIX)
                .ok_or(RecoveryJournalError::UnexpectedJournalEntry)?;
            let action_id = ActionId::from_token(token)?;
            let parsed = self.read_journal(&path, &action_id)?;
            let outcome = match parsed.latest_state {
                JournalState::Prepared => RecoveryOutcome::PreparedNotCommitted,
                JournalState::Verified => RecoveryOutcome::VerifiedTerminal,
                JournalState::Committing | JournalState::Ambiguous => {
                    let observation = filesystem
                        .observe_mutation_target(&parsed.canonical_path)
                        .map_err(RecoveryJournalError::RecoveryObservation)?;
                    if observation.canonical_path != parsed.canonical_path {
                        return Err(RecoveryJournalError::UnresolvedRecoveryConflict {
                            path: parsed.canonical_path.as_str().to_owned(),
                        });
                    }
                    if observation.state == parsed.intended {
                        RecoveryOutcome::ObservedCommitted
                    } else if observation.state == parsed.previous {
                        RecoveryOutcome::ObservedNotCommitted
                    } else {
                        return Err(RecoveryJournalError::UnresolvedRecoveryConflict {
                            path: parsed.canonical_path.as_str().to_owned(),
                        });
                    }
                }
            };

            records.push(RecoveryRecord {
                action_id: parsed.action_id,
                canonical_path: parsed.canonical_path,
                intended: parsed.intended,
                outcome,
            });
        }

        Ok(RecoveryReport { records })
    }

    pub(crate) fn retire_action(&self, action_id: &ActionId) -> Result<(), RecoveryJournalError> {
        self.remove_regular_file(&self.journal_path(action_id))
    }

    fn ensure_action_path_absent(
        &self,
        path: &Path,
        action_id: &ActionId,
    ) -> Result<(), RecoveryJournalError> {
        match fs::symlink_metadata(path) {
            Ok(_) => Err(RecoveryJournalError::ActionAlreadyActive {
                action_id: action_id.clone(),
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(RecoveryJournalError::Io(error)),
        }
    }

    fn append(
        &self,
        ticket: &JournalTicket,
        state: JournalState,
    ) -> Result<(), RecoveryJournalError> {
        let path = self.journal_path(&ticket.action_id);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(RecoveryJournalError::UnexpectedJournalEntry);
        }

        let bytes = serialize_line(&ticket.entry(state))?;
        self.ensure_file_size(metadata.len(), bytes.len())?;
        let mut file = OpenOptions::new().append(true).open(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(())
    }

    fn read_journal(
        &self,
        path: &Path,
        expected_action_id: &ActionId,
    ) -> Result<ParsedJournal, RecoveryJournalError> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(RecoveryJournalError::UnexpectedJournalEntry);
        }
        if metadata.len() > self.max_file_bytes {
            return Err(RecoveryJournalError::JournalFileTooLarge {
                limit: self.max_file_bytes,
            });
        }

        let mut file = File::open(path)?;
        let read_limit = self
            .max_file_bytes
            .checked_add(1)
            .ok_or(RecoveryJournalError::InvalidLimits)?;
        let mut bytes = Vec::new();
        std::io::Read::by_ref(&mut file)
            .take(read_limit)
            .read_to_end(&mut bytes)?;
        if u64::try_from(bytes.len()).map_err(|_| RecoveryJournalError::InvalidLimits)?
            > self.max_file_bytes
        {
            return Err(RecoveryJournalError::JournalFileTooLarge {
                limit: self.max_file_bytes,
            });
        }

        parse_journal(&bytes, expected_action_id)
    }

    fn ensure_record_capacity(&self) -> Result<(), RecoveryJournalError> {
        let mut count = 0_usize;
        for entry in fs::read_dir(&self.root)? {
            let _ = entry?;
            count = count.saturating_add(1);
            if count >= self.max_records {
                return Err(RecoveryJournalError::RecoveryRecordLimitExceeded {
                    limit: self.max_records,
                });
            }
        }
        Ok(())
    }

    fn ensure_file_size(
        &self,
        current_bytes: u64,
        additional_bytes: usize,
    ) -> Result<(), RecoveryJournalError> {
        let additional_bytes =
            u64::try_from(additional_bytes).map_err(|_| RecoveryJournalError::InvalidLimits)?;
        let resulting = current_bytes.checked_add(additional_bytes).ok_or(
            RecoveryJournalError::JournalFileTooLarge {
                limit: self.max_file_bytes,
            },
        )?;
        if resulting > self.max_file_bytes {
            return Err(RecoveryJournalError::JournalFileTooLarge {
                limit: self.max_file_bytes,
            });
        }
        Ok(())
    }

    fn journal_path(&self, action_id: &ActionId) -> PathBuf {
        self.root
            .join(format!("{}{}", action_id.to_token(), JOURNAL_SUFFIX))
    }

    fn prepared_temp_path(&self, action_id: &ActionId) -> PathBuf {
        self.root
            .join(format!("{}{}", action_id.to_token(), PREPARED_TEMP_SUFFIX))
    }

    fn remove_regular_file(&self, path: &Path) -> Result<(), RecoveryJournalError> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(RecoveryJournalError::UnexpectedJournalEntry);
                }
                fs::remove_file(path)?;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(RecoveryJournalError::Io(error)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct JournalTicket {
    action_id: ActionId,
    canonical_path: WorkspacePath,
    previous: ExpectedState,
    intended: ExpectedState,
}

impl JournalTicket {
    #[must_use]
    pub fn action_id(&self) -> &ActionId {
        &self.action_id
    }

    #[must_use]
    pub fn canonical_path(&self) -> &WorkspacePath {
        &self.canonical_path
    }

    #[must_use]
    pub const fn intended_state(&self) -> ExpectedState {
        self.intended
    }

    fn entry(&self, state: JournalState) -> JournalEntryV2 {
        JournalEntryV2 {
            version: JOURNAL_VERSION_CURRENT,
            action_id: self.action_id.to_token(),
            state,
            path: self.canonical_path.as_str().to_owned(),
            previous: JournalExpectedState::from_expected(self.previous),
            intended: JournalExpectedState::from_expected(self.intended),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryReport {
    pub records: Vec<RecoveryRecord>,
}

impl RecoveryReport {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryRecord {
    pub action_id: ActionId,
    pub canonical_path: WorkspacePath,
    pub intended: ExpectedState,
    pub outcome: RecoveryOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryOutcome {
    PreparedNotCommitted,
    ObservedNotCommitted,
    ObservedCommitted,
    VerifiedTerminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum JournalState {
    Prepared,
    Committing,
    Verified,
    Ambiguous,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum JournalExpectedState {
    Absent,
    Content { version_hex: String },
}

impl JournalExpectedState {
    fn from_expected(expected: ExpectedState) -> Self {
        match expected {
            ExpectedState::Absent => Self::Absent,
            ExpectedState::Content(version) => Self::Content {
                version_hex: version.to_hex(),
            },
        }
    }

    fn to_expected(&self) -> Result<ExpectedState, RecoveryJournalError> {
        match self {
            Self::Absent => Ok(ExpectedState::Absent),
            Self::Content { version_hex } => ContentVersion::from_hex(version_hex)
                .map(ExpectedState::Content)
                .map_err(|_| RecoveryJournalError::CorruptJournal),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalEntryV1 {
    version: u32,
    action_id: String,
    state: JournalState,
    path: String,
    previous: JournalExpectedState,
    intended_version_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalEntryV2 {
    version: u32,
    action_id: String,
    state: JournalState,
    path: String,
    previous: JournalExpectedState,
    intended: JournalExpectedState,
}

#[derive(Deserialize)]
struct JournalVersionHeader {
    version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NormalizedJournalEntry {
    version: u32,
    action_id: String,
    state: JournalState,
    path: String,
    previous: JournalExpectedState,
    intended: JournalExpectedState,
}

#[derive(Debug)]
struct ParsedJournal {
    action_id: ActionId,
    canonical_path: WorkspacePath,
    previous: ExpectedState,
    intended: ExpectedState,
    latest_state: JournalState,
}

fn serialize_line(entry: &impl Serialize) -> Result<Vec<u8>, RecoveryJournalError> {
    let mut bytes = serde_json::to_vec(entry)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn parse_entry(bytes: &[u8]) -> Result<NormalizedJournalEntry, RecoveryJournalError> {
    let header = serde_json::from_slice::<JournalVersionHeader>(bytes)?;
    match header.version {
        JOURNAL_VERSION_V1 => {
            let entry = serde_json::from_slice::<JournalEntryV1>(bytes)?;
            Ok(NormalizedJournalEntry {
                version: entry.version,
                action_id: entry.action_id,
                state: entry.state,
                path: entry.path,
                previous: entry.previous,
                intended: JournalExpectedState::Content {
                    version_hex: entry.intended_version_hex,
                },
            })
        }
        JOURNAL_VERSION_V2 => {
            let entry = serde_json::from_slice::<JournalEntryV2>(bytes)?;
            Ok(NormalizedJournalEntry {
                version: entry.version,
                action_id: entry.action_id,
                state: entry.state,
                path: entry.path,
                previous: entry.previous,
                intended: entry.intended,
            })
        }
        _ => Err(RecoveryJournalError::CorruptJournal),
    }
}

fn parse_journal(
    bytes: &[u8],
    expected_action_id: &ActionId,
) -> Result<ParsedJournal, RecoveryJournalError> {
    if bytes.is_empty() {
        return Err(RecoveryJournalError::CorruptJournal);
    }

    let has_complete_tail = bytes.last() == Some(&b'\n');
    let mut entries = Vec::new();
    let chunks = bytes.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    for (index, chunk) in chunks.iter().enumerate() {
        if chunk.is_empty() {
            continue;
        }
        let is_last = index + 1 == chunks.len();
        if is_last && !has_complete_tail {
            break;
        }
        entries.push(parse_entry(chunk)?);
    }

    let first = entries
        .first()
        .ok_or(RecoveryJournalError::CorruptJournal)?;
    if first.state != JournalState::Prepared || first.action_id != expected_action_id.to_token() {
        return Err(RecoveryJournalError::CorruptJournal);
    }

    let canonical_path = WorkspacePath::parse(&first.path)?;
    let previous = first.previous.to_expected()?;
    let intended = first.intended.to_expected()?;

    let mut latest = JournalState::Prepared;
    for (index, entry) in entries.iter().enumerate() {
        if entry.version != first.version
            || entry.action_id != first.action_id
            || entry.path != first.path
            || entry.previous != first.previous
            || entry.intended != first.intended
        {
            return Err(RecoveryJournalError::CorruptJournal);
        }

        let expected_state = match index {
            0 => JournalState::Prepared,
            1 => JournalState::Committing,
            2 => entry.state,
            _ => return Err(RecoveryJournalError::InvalidStateTransition),
        };
        if entry.state != expected_state {
            return Err(RecoveryJournalError::InvalidStateTransition);
        }
        if index == 2
            && !matches!(
                entry.state,
                JournalState::Verified | JournalState::Ambiguous
            )
        {
            return Err(RecoveryJournalError::InvalidStateTransition);
        }
        latest = entry.state;
    }

    Ok(ParsedJournal {
        action_id: expected_action_id.clone(),
        canonical_path,
        previous,
        intended,
        latest_state: latest,
    })
}

#[derive(Debug, Error)]
pub enum RecoveryJournalError {
    #[error("mutation recovery journal limits are invalid")]
    InvalidLimits,
    #[error("mutation recovery state directory must be outside the workspace")]
    StateDirectoryInsideWorkspace,
    #[error("mutation recovery already has active state for operation {action_id:?}")]
    ActionAlreadyActive { action_id: ActionId },
    #[error("mutation recovery journal contains an unexpected entry")]
    UnexpectedJournalEntry,
    #[error("mutation recovery journal is corrupt or incomplete before its first durable state")]
    CorruptJournal,
    #[error("mutation recovery journal contains an invalid state transition")]
    InvalidStateTransition,
    #[error("mutation recovery journal file exceeds hard byte ceiling {limit}")]
    JournalFileTooLarge { limit: u64 },
    #[error("mutation recovery journal exceeds hard record ceiling {limit}")]
    RecoveryRecordLimitExceeded { limit: usize },
    #[error("mutation recovery cannot resolve current state for {path}")]
    UnresolvedRecoveryConflict { path: String },
    #[error("mutation recovery target observation failed: {0}")]
    RecoveryObservation(#[source] MutationError),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal serialization failed: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("workspace path in journal is invalid: {0}")]
    WorkspacePath(#[from] WorkspacePathError),
    #[error("journal action token is invalid: {0}")]
    Token(#[from] TokenParseError),
    #[error("operating-system entropy source is unavailable")]
    Entropy(#[from] IdError),
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    fn fixture(label: &str) -> (PathBuf, PathBuf) {
        let token = ActionId::generate().expect("test entropy").to_token();
        let base = env::temp_dir().join(format!("optic-recovery-{label}-{token}"));
        let workspace = base.join("workspace");
        let state = base.join("state");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&state).expect("state");
        (workspace, state)
    }

    fn service(
        workspace: &Path,
        state: &Path,
        limits: HardLimits,
    ) -> (crate::AtomicMutationService, MutationRecoveryJournal) {
        let atomic = crate::AtomicMutationService::from_hard_limits(workspace, limits)
            .expect("atomic service");
        let journal = MutationRecoveryJournal::open(state, atomic.filesystem().root(), limits)
            .expect("journal");
        (atomic, journal)
    }

    fn v1_entry(
        action_id: &ActionId,
        state: JournalState,
        path: &WorkspacePath,
        previous: ExpectedState,
        intended: ContentVersion,
    ) -> JournalEntryV1 {
        JournalEntryV1 {
            version: JOURNAL_VERSION_V1,
            action_id: action_id.to_token(),
            state,
            path: path.as_str().to_owned(),
            previous: JournalExpectedState::from_expected(previous),
            intended_version_hex: intended.to_hex(),
        }
    }

    fn write_lines(path: &Path, lines: &[Vec<u8>]) {
        let bytes = lines.concat();
        fs::write(path, bytes).expect("write journal fixture");
    }

    #[test]
    fn state_directory_inside_workspace_is_rejected() {
        let (workspace, state) = fixture("inside-state");
        let inside = workspace.join("state");
        fs::create_dir_all(&inside).expect("inside state");
        let atomic =
            crate::AtomicMutationService::from_hard_limits(&workspace, HardLimits::default())
                .expect("atomic");
        assert!(matches!(
            MutationRecoveryJournal::open(
                &inside,
                atomic.filesystem().root(),
                HardLimits::default()
            ),
            Err(RecoveryJournalError::StateDirectoryInsideWorkspace)
        ));
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
        let _ = state;
    }

    #[test]
    fn prepared_only_is_recovered_as_not_committed() {
        let (workspace, state) = fixture("prepared");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");

        let report = journal.reconcile(atomic.filesystem()).expect("reconcile");
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::PreparedNotCommitted
        );
        assert!(
            fs::read_dir(journal.root())
                .expect("journal dir")
                .next()
                .is_none()
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn new_journal_records_use_v2_explicit_intended_state() {
        let (workspace, state) = fixture("v2-format");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let intended = ExpectedState::Content(ContentVersion::from_bytes(b"new"));
        let ticket = journal.begin(&plan, intended).expect("begin");

        let bytes = fs::read(journal.journal_path(ticket.action_id())).expect("journal bytes");
        let first = bytes
            .split(|byte| *byte == b'\n')
            .find(|line| !line.is_empty())
            .expect("first line");
        let entry = serde_json::from_slice::<JournalEntryV2>(first).expect("v2 entry");
        assert_eq!(entry.version, JOURNAL_VERSION_V2);
        assert_eq!(entry.intended.to_expected().expect("intended"), intended);

        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn caller_supplied_action_id_is_the_journal_key_and_active_reuse_fails_closed() {
        let (workspace, state) = fixture("caller-action-id");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let intended = ExpectedState::Content(ContentVersion::from_bytes(b"new"));
        let action_id = ActionId::generate().expect("action entropy");

        let ticket = journal
            .begin_for_action(&action_id, &plan, intended)
            .expect("begin for action");
        assert_eq!(ticket.action_id(), &action_id);
        assert!(journal.journal_path(&action_id).is_file());
        assert!(matches!(
            journal.begin_for_action(&action_id, &plan, intended),
            Err(RecoveryJournalError::ActionAlreadyActive { .. })
        ));

        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn committing_state_recovers_by_observing_intended_content() {
        let (workspace, state) = fixture("committed");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let plan = atomic
            .prepare_write(
                &path,
                ExpectedState::Content(ContentVersion::from_bytes(b"old")),
            )
            .expect("plan");
        let intended = ExpectedState::Content(ContentVersion::from_bytes(b"new"));
        let ticket = journal.begin(&plan, intended).expect("begin");
        journal.mark_committing(&ticket).expect("committing");
        fs::write(workspace.join("target.txt"), b"new").expect("new");

        let report = journal.reconcile(atomic.filesystem()).expect("reconcile");
        assert_eq!(report.records[0].intended, intended);
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::ObservedCommitted
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn committing_state_recovers_by_observing_previous_content() {
        let (workspace, state) = fixture("not-committed");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let previous = ExpectedState::Content(ContentVersion::from_bytes(b"old"));
        let plan = atomic.prepare_write(&path, previous).expect("plan");
        let ticket = journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        journal.mark_committing(&ticket).expect("committing");

        let report = journal.reconcile(atomic.filesystem()).expect("reconcile");
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::ObservedNotCommitted
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn intended_absent_can_classify_a_committed_delete_shape() {
        let (workspace, state) = fixture("intended-absent-committed");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let previous = ExpectedState::Content(ContentVersion::from_bytes(b"old"));
        let plan = atomic.prepare_write(&path, previous).expect("plan");
        let ticket = journal
            .begin(&plan, ExpectedState::Absent)
            .expect("begin absent intent");
        journal.mark_committing(&ticket).expect("committing");
        fs::remove_file(workspace.join("target.txt")).expect("simulate delete effect");

        let report = journal.reconcile(atomic.filesystem()).expect("reconcile");
        assert_eq!(report.records[0].intended, ExpectedState::Absent);
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::ObservedCommitted
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn intended_absent_can_classify_a_not_committed_delete_shape() {
        let (workspace, state) = fixture("intended-absent-not-committed");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let previous = ExpectedState::Content(ContentVersion::from_bytes(b"old"));
        let plan = atomic.prepare_write(&path, previous).expect("plan");
        let ticket = journal
            .begin(&plan, ExpectedState::Absent)
            .expect("begin absent intent");
        journal.mark_committing(&ticket).expect("committing");

        let report = journal.reconcile(atomic.filesystem()).expect("reconcile");
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::ObservedNotCommitted
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn v1_content_journal_remains_recoverable() {
        let (workspace, state) = fixture("v1-compatible");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        let action_id = ActionId::generate().expect("action id");
        let previous = ExpectedState::Content(ContentVersion::from_bytes(b"old"));
        let intended = ContentVersion::from_bytes(b"new");
        fs::write(workspace.join("target.txt"), b"new").expect("committed content");

        write_lines(
            &journal.journal_path(&action_id),
            &[
                serialize_line(&v1_entry(
                    &action_id,
                    JournalState::Prepared,
                    &path,
                    previous,
                    intended,
                ))
                .expect("v1 prepared"),
                serialize_line(&v1_entry(
                    &action_id,
                    JournalState::Committing,
                    &path,
                    previous,
                    intended,
                ))
                .expect("v1 committing"),
            ],
        );

        let report = journal
            .reconcile(atomic.filesystem())
            .expect("reconcile v1");
        assert_eq!(report.records[0].intended, ExpectedState::Content(intended));
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::ObservedCommitted
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn mixed_schema_journal_fails_closed_and_keeps_evidence() {
        let (workspace, state) = fixture("mixed-schema");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        let action_id = ActionId::generate().expect("action id");
        let previous = ExpectedState::Content(ContentVersion::from_bytes(b"old"));
        let intended = ContentVersion::from_bytes(b"new");
        fs::write(workspace.join("target.txt"), b"old").expect("old");

        let v2 = JournalEntryV2 {
            version: JOURNAL_VERSION_V2,
            action_id: action_id.to_token(),
            state: JournalState::Committing,
            path: path.as_str().to_owned(),
            previous: JournalExpectedState::from_expected(previous),
            intended: JournalExpectedState::from_expected(ExpectedState::Content(intended)),
        };
        write_lines(
            &journal.journal_path(&action_id),
            &[
                serialize_line(&v1_entry(
                    &action_id,
                    JournalState::Prepared,
                    &path,
                    previous,
                    intended,
                ))
                .expect("v1 prepared"),
                serialize_line(&v2).expect("v2 committing"),
            ],
        );

        assert!(matches!(
            journal.reconcile(atomic.filesystem()),
            Err(RecoveryJournalError::CorruptJournal)
        ));
        assert!(journal.journal_path(&action_id).exists());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn invalid_persisted_content_version_fails_closed_and_keeps_evidence() {
        let (workspace, state) = fixture("invalid-version");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        let action_id = ActionId::generate().expect("action id");
        let entry = JournalEntryV1 {
            version: JOURNAL_VERSION_V1,
            action_id: action_id.to_token(),
            state: JournalState::Prepared,
            path: path.as_str().to_owned(),
            previous: JournalExpectedState::Absent,
            intended_version_hex: "z".repeat(64),
        };
        write_lines(
            &journal.journal_path(&action_id),
            &[serialize_line(&entry).expect("invalid fixture")],
        );

        assert!(matches!(
            journal.reconcile(atomic.filesystem()),
            Err(RecoveryJournalError::CorruptJournal)
        ));
        assert!(journal.journal_path(&action_id).exists());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn conflicting_recovery_state_fails_closed_and_keeps_journal() {
        let (workspace, state) = fixture("conflict");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let previous = ExpectedState::Content(ContentVersion::from_bytes(b"old"));
        let plan = atomic.prepare_write(&path, previous).expect("plan");
        let ticket = journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        journal.mark_committing(&ticket).expect("committing");
        fs::write(workspace.join("target.txt"), b"third-party").expect("third party");

        assert!(matches!(
            journal.reconcile(atomic.filesystem()),
            Err(RecoveryJournalError::UnresolvedRecoveryConflict { .. })
        ));
        assert!(journal.journal_path(ticket.action_id()).exists());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn verified_terminal_record_is_retired_without_reinterpreting_current_target() {
        let (workspace, state) = fixture("verified");
        let limits = HardLimits::default();
        let (atomic, journal) = service(&workspace, &state, limits);
        let path = WorkspacePath::parse("target.txt").expect("path");
        fs::write(workspace.join("target.txt"), b"old").expect("old");
        let previous = ExpectedState::Content(ContentVersion::from_bytes(b"old"));
        let plan = atomic.prepare_write(&path, previous).expect("plan");
        let ticket = journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        journal.mark_committing(&ticket).expect("committing");
        journal.mark_verified(&ticket).expect("verified");
        fs::write(workspace.join("target.txt"), b"later-change").expect("later change");

        let report = journal.reconcile(atomic.filesystem()).expect("reconcile");
        assert_eq!(report.records[0].outcome, RecoveryOutcome::VerifiedTerminal);
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }
}
