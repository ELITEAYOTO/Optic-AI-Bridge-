use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use optic_bridge_core::{
    ActionId, ContentVersion, ExpectedState, HardLimits, IdError, TokenParseError, WorkspacePath,
    WorkspacePathError,
};
#[cfg(not(windows))]
use std::fs::File;
#[cfg(windows)]
use std::io::{Seek, SeekFrom};

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

        let workspace_root = fs::canonicalize(workspace_root)?;
        // Refuse a new state directory under the workspace before creating
        // any of its missing ancestors. A post-creation canonical check alone
        // can reject the path only after writing directories into the project.
        // This is a fail-closed preflight, not a directory-handle pin against
        // concurrent namespace changes.
        let mut existing_ancestor = state_root.as_ref().to_path_buf();
        loop {
            match fs::canonicalize(&existing_ancestor) {
                Ok(canonical) => {
                    if canonical == workspace_root || canonical.starts_with(&workspace_root) {
                        return Err(RecoveryJournalError::StateDirectoryInsideWorkspace);
                    }
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if !existing_ancestor.pop() {
                        return Err(RecoveryJournalError::Io(error));
                    }
                    if existing_ancestor.as_os_str().is_empty() {
                        existing_ancestor = PathBuf::from(".");
                    }
                }
                Err(error) => return Err(RecoveryJournalError::Io(error)),
            }
        }

        fs::create_dir_all(state_root.as_ref())?;
        let state_root = fs::canonicalize(state_root.as_ref())?;
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
        if let Err(error) = self.publish_prepared_no_replace(&temp_path, &journal_path) {
            // The destination is never replaced on either hosted OS. The
            // create-new temp may still need cleanup after a failed attempt.
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
        Ok(ticket)
    }

    fn publish_prepared_no_replace(
        &self,
        temp_path: &Path,
        journal_path: &Path,
    ) -> Result<(), RecoveryJournalError> {
        #[cfg(windows)]
        {
            optic_bridge_windows::move_file_no_replace(temp_path, journal_path)
                .map_err(RecoveryJournalError::NativeJournalFile)
        }
        #[cfg(all(target_os = "linux", target_env = "gnu"))]
        {
            self.publish_prepared_linux_with_sync(temp_path, journal_path, File::sync_all)
        }
        #[cfg(not(any(windows, all(target_os = "linux", target_env = "gnu"))))]
        {
            let _ = (temp_path, journal_path);
            Err(RecoveryJournalError::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "atomic no-replace journal publication is unsupported on this platform",
            )))
        }
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    fn publish_prepared_linux_with_sync(
        &self,
        temp_path: &Path,
        journal_path: &Path,
        sync_directory: impl FnOnce(&File) -> std::io::Result<()>,
    ) -> Result<(), RecoveryJournalError> {
        use nix::fcntl::{RenameFlags, renameat2};

        // Both names are resolved relative to the same open recovery root.
        // Atomic no-replace publication is only a namespace change: the
        // prepared file sync does not persist the new directory entry.
        let dir = File::open(&self.root)?;
        let temp_name = temp_path.strip_prefix(&self.root).map_err(|_| {
            RecoveryJournalError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "prepared journal path must be within the recovery root",
            ))
        })?;
        let journal_name = journal_path.strip_prefix(&self.root).map_err(|_| {
            RecoveryJournalError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "published journal path must be within the recovery root",
            ))
        })?;
        renameat2(
            &dir,
            temp_name,
            &dir,
            journal_name,
            RenameFlags::RENAME_NOREPLACE,
        )
        .map_err(|errno| {
            RecoveryJournalError::Io(std::io::Error::from_raw_os_error(errno as i32))
        })?;

        // Treat a failed directory sync as an uncertain *published* state:
        // never remove the stable name or claim the operation was admitted.
        // Recovery can inspect the durable intent on subsequent startup.
        sync_directory(&dir)?;
        Ok(())
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
        let scan = self.scan_pending(filesystem)?;
        self.discard_unpublished(&scan)?;
        for record in &scan.report.records {
            self.retire_action(&record.action_id)?;
        }
        Ok(scan.report)
    }

    // Validation only: callers decide when cleanup can safely start.
    // The journal-only reconciler may clean immediately after this scan,
    // while full recovery must also validate staged operation artifacts.
    pub(crate) fn scan_pending(
        &self,
        filesystem: &BoundedFileSystem,
    ) -> Result<PendingRecoveryScan, RecoveryJournalError> {
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
        // Delay cleanup until all stable records have been parsed and their
        // outcomes classified. An unrelated corrupt journal must not erase
        // unpublished evidence just because its name sorts later.
        let mut unpublished = Vec::new();
        for path in paths {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(RecoveryJournalError::UnexpectedJournalEntry)?;

            if let Some(token) = file_name.strip_suffix(PREPARED_TEMP_SUFFIX) {
                ActionId::from_token(token)?;
                unpublished.push(path);
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

        Ok(PendingRecoveryScan {
            report: RecoveryReport { records },
            unpublished,
        })
    }

    pub(crate) fn discard_unpublished(
        &self,
        scan: &PendingRecoveryScan,
    ) -> Result<(), RecoveryJournalError> {
        // Unpublished files cannot represent committed effects. Their
        // deletion must follow all relevant validation phases, not just
        // stable-journal classification. Deletion remains non-atomic.
        for path in &scan.unpublished {
            self.remove_regular_file(path)?;
        }
        Ok(())
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

        // The pathname-only check above is not evidence that the subsequent
        // opener targets that same single-link journal. Validate the opened
        // object before the first append byte can reach durable evidence.
        #[cfg(windows)]
        let mut opened = optic_bridge_windows::open_exclusive_state_file(&path, false)
            .map_err(RecoveryJournalError::NativeJournalFile)?;
        #[cfg(windows)]
        let file = opened.file_mut();
        #[cfg(not(windows))]
        let mut file = OpenOptions::new().append(true).open(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let actual = file.metadata()?;
            if !actual.is_file()
                || actual.nlink() != 1
                || actual.dev() != metadata.dev()
                || actual.ino() != metadata.ino()
            {
                return Err(RecoveryJournalError::UnexpectedJournalEntry);
            }
        }
        // Use the actual open handle's length, not just the earlier pathname
        // observation. Windows' exclusive handle pins this file through sync.
        self.ensure_file_size(file.metadata()?.len(), bytes.len())?;
        #[cfg(windows)]
        file.seek(SeekFrom::End(0))?;
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

        // Inspect the object opened by the OS, not only the pathname checked
        // above: a hard link (or a replacement between lstat and open) must
        // never make unrelated data acceptable as recovery evidence.
        #[cfg(windows)]
        let mut opened = optic_bridge_windows::open_exclusive_read_file(path)
            .map_err(RecoveryJournalError::NativeJournalFile)?;
        #[cfg(windows)]
        let mut file = opened.file_mut();
        #[cfg(not(windows))]
        let mut file = File::open(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let actual = file.metadata()?;
            if !actual.is_file()
                || actual.nlink() != 1
                || actual.dev() != metadata.dev()
                || actual.ino() != metadata.ino()
            {
                return Err(RecoveryJournalError::UnexpectedJournalEntry);
            }
        }
        // Bound the actual opened object, including after a pathname race.
        if file.metadata()?.len() > self.max_file_bytes {
            return Err(RecoveryJournalError::JournalFileTooLarge {
                limit: self.max_file_bytes,
            });
        }
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
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(RecoveryJournalError::Io(error)),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(RecoveryJournalError::UnexpectedJournalEntry);
        }

        #[cfg(windows)]
        {
            // Delete the exact exclusively opened regular file, not a path
            // independently resolved after validation. The native primitive
            // also rejects reparse points and hard-link aliases.
            let opened = optic_bridge_windows::open_exclusive_file_for_delete_no_reparse(path)
                .map_err(RecoveryJournalError::NativeJournalFile)?;
            opened
                .delete()
                .map_err(RecoveryJournalError::NativeJournalFile)
        }
        #[cfg(not(windows))]
        {
            let opened = File::open(path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;

                let actual = opened.metadata()?;
                if !actual.is_file()
                    || actual.nlink() != 1
                    || actual.dev() != metadata.dev()
                    || actual.ino() != metadata.ino()
                {
                    return Err(RecoveryJournalError::UnexpectedJournalEntry);
                }
            }
            // Unix does not offer the native Windows handle-based delete
            // primitive here; a concurrent pathname replacement remains
            // outside this bounded alias-rejection improvement.
            fs::remove_file(path)?;
            Ok(())
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

// Internal scan result retains the exact unpublished paths observed during
// validation; callers must not rescan and accidentally clean newer entries.
pub(crate) struct PendingRecoveryScan {
    pub(crate) report: RecoveryReport,
    unpublished: Vec<PathBuf>,
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
            // split() yields exactly one empty final element for the normal
            // newline terminator. Earlier empty elements represent complete
            // blank records, which the append-only writer never produces.
            if has_complete_tail && index + 1 == chunks.len() {
                continue;
            }
            return Err(RecoveryJournalError::CorruptJournal);
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
    #[cfg(windows)]
    #[error("native recovery journal open/identity validation failed: {0}")]
    NativeJournalFile(#[source] optic_bridge_windows::WindowsFileError),
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
    fn missing_state_directory_inside_workspace_is_rejected_before_creation() {
        let (workspace, state) = fixture("preflight-inside");
        let requested = workspace.join("should-not-exist").join("nested");
        let atomic =
            crate::AtomicMutationService::from_hard_limits(&workspace, HardLimits::default())
                .expect("atomic");
        assert!(matches!(
            MutationRecoveryJournal::open(
                &requested,
                atomic.filesystem().root(),
                HardLimits::default()
            ),
            Err(RecoveryJournalError::StateDirectoryInsideWorkspace)
        ));
        assert!(
            !workspace.join("should-not-exist").exists(),
            "rejected state root must not create any workspace directory"
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
        let _ = state;
    }

    #[test]
    fn missing_state_directory_outside_workspace_can_still_be_created() {
        let (workspace, state) = fixture("preflight-outside");
        let requested = state.join("new").join("nested");
        let atomic =
            crate::AtomicMutationService::from_hard_limits(&workspace, HardLimits::default())
                .expect("atomic");
        assert!(!requested.exists());
        let journal = MutationRecoveryJournal::open(
            &requested,
            atomic.filesystem().root(),
            HardLimits::default(),
        )
        .expect("outside root");
        assert!(journal.root().is_dir());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(any(windows, all(target_os = "linux", target_env = "gnu")))]
    #[test]
    fn journal_publication_does_not_replace_preexisting_evidence() {
        let (workspace, state) = fixture("publish-existing");
        let (_, journal) = service(&workspace, &state, HardLimits::default());
        let action_id = ActionId::generate().expect("action id");
        let temp_path = journal.prepared_temp_path(&action_id);
        let journal_path = journal.journal_path(&action_id);
        fs::write(&temp_path, b"new prepared journal").expect("create temp");
        fs::write(&journal_path, b"durable previous evidence").expect("create stable");

        assert!(
            journal
                .publish_prepared_no_replace(&temp_path, &journal_path)
                .is_err(),
            "publication must not overwrite even after a prior absence check"
        );
        assert_eq!(
            fs::read(&journal_path).expect("stable retained"),
            b"durable previous evidence"
        );
        assert_eq!(
            fs::read(&temp_path).expect("temporary retained"),
            b"new prepared journal"
        );
        fs::remove_file(&journal_path).expect("remove test destination");
        journal
            .publish_prepared_no_replace(&temp_path, &journal_path)
            .expect("publish when absent");
        assert!(!temp_path.exists());
        assert_eq!(
            fs::read(&journal_path).expect("published content"),
            b"new prepared journal"
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(any(windows, all(target_os = "linux", target_env = "gnu")))]
    #[test]
    fn competing_journal_publishers_never_overwrite_the_first_winner() {
        use std::sync::{Arc, Barrier};

        let (workspace, state) = fixture("publish-contended");
        let (_, journal) = service(&workspace, &state, HardLimits::default());
        let dest = journal.root().join("contended.journal");
        let first = journal.root().join("one.prepared.tmp");
        let second = journal.root().join("two.prepared.tmp");
        fs::write(&first, b"writer-one").expect("first");
        fs::write(&second, b"writer-two").expect("second");

        let gate = Arc::new(Barrier::new(3));
        let (a, b) = std::thread::scope(|scope| {
            let g1 = Arc::clone(&gate);
            let g2 = Arc::clone(&gate);
            let journal_ref = &journal;
            let dest_ref = &dest;
            let first_ref = &first;
            let second_ref = &second;
            let a = scope.spawn(move || {
                g1.wait();
                journal_ref.publish_prepared_no_replace(first_ref, dest_ref)
            });
            let b = scope.spawn(move || {
                g2.wait();
                journal_ref.publish_prepared_no_replace(second_ref, dest_ref)
            });
            gate.wait();
            (
                a.join().expect("first thread"),
                b.join().expect("second thread"),
            )
        });
        assert_ne!(a.is_ok(), b.is_ok(), "exactly one publisher must win");
        let expected = if a.is_ok() {
            b"writer-one"
        } else {
            b"writer-two"
        };
        assert_eq!(fs::read(&dest).expect("one stable winner"), expected);
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn failed_directory_sync_keeps_published_journal_for_reconciliation() {
        let (workspace, state) = fixture("publish-dir-sync-error");
        let (atomic, journal) = service(&workspace, &state, HardLimits::default());
        let action_id = ActionId::generate().expect("action id");
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let ticket = JournalTicket {
            action_id: action_id.clone(),
            canonical_path: plan.canonical_path().clone(),
            previous: plan.expected_state(),
            intended: ExpectedState::Content(ContentVersion::from_bytes(b"new")),
        };
        let prepared =
            serialize_line(&ticket.entry(JournalState::Prepared)).expect("prepared journal bytes");
        let temp_path = journal.prepared_temp_path(&action_id);
        let stable_path = journal.journal_path(&action_id);
        fs::write(&temp_path, &prepared).expect("write prepared fixture");

        let error = journal
            .publish_prepared_linux_with_sync(&temp_path, &stable_path, |_| {
                Err(std::io::Error::other("injected directory sync failure"))
            })
            .expect_err("directory sync failure must not be accepted");
        assert!(matches!(error, RecoveryJournalError::Io(_)));
        assert!(!temp_path.exists(), "rename already published the journal");
        assert_eq!(
            fs::read(&stable_path).expect("stable intent retained"),
            prepared
        );
        let pending = journal
            .scan_pending(atomic.filesystem())
            .expect("recover intent")
            .report;
        assert_eq!(pending.records.len(), 1);
        assert_eq!(pending.records[0].action_id, action_id);
        assert_eq!(
            pending.records[0].outcome,
            RecoveryOutcome::PreparedNotCommitted
        );
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn corrupt_stable_journal_does_not_delete_earlier_unpublished_evidence() {
        let (workspace, state) = fixture("preserve-unpublished");
        let (atomic, journal) = service(&workspace, &state, HardLimits::default());
        let first = ActionId::generate().expect("first id");
        let second = ActionId::generate().expect("second id");
        assert_ne!(first.to_token(), second.to_token());
        let (unpublished_id, corrupt_id) = if first.to_token() < second.to_token() {
            (first, second)
        } else {
            (second, first)
        };
        let unpublished_path = journal.prepared_temp_path(&unpublished_id);
        let corrupt_path = journal.journal_path(&corrupt_id);
        assert!(
            unpublished_path < corrupt_path,
            "unpublished entry must be visited before corrupt stable entry"
        );
        let unpublished_bytes = b"unpublished but retained after failed inspection";
        let corrupt_bytes = b"not-json\n";
        fs::write(&unpublished_path, unpublished_bytes).expect("unpublished fixture");
        fs::write(&corrupt_path, corrupt_bytes).expect("corrupt stable fixture");

        assert!(matches!(
            journal.reconcile(atomic.filesystem()),
            Err(RecoveryJournalError::Serde(_))
        ));
        assert_eq!(
            fs::read(&unpublished_path).expect("unpublished evidence retained"),
            unpublished_bytes
        );
        assert_eq!(
            fs::read(&corrupt_path).expect("corrupt stable evidence retained"),
            corrupt_bytes
        );

        fs::remove_file(&corrupt_path).expect("resolve corrupted fixture");
        assert!(
            journal
                .reconcile(atomic.filesystem())
                .expect("reconcile")
                .is_empty(),
            "unpublished evidence is discarded only after a clean scan"
        );
        assert!(!unpublished_path.exists());
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
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
    fn hard_linked_journal_is_not_trusted_as_recovery_evidence() {
        let (workspace, state) = fixture("hardlink-evidence");
        let (atomic, journal) = service(&workspace, &state, HardLimits::default());
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let ticket = journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        let journal_path = journal.journal_path(ticket.action_id());
        let outside = state
            .parent()
            .expect("fixture base")
            .join("external-evidence");
        let original = fs::read(&journal_path).expect("journal contents");

        fs::rename(&journal_path, &outside).expect("move valid journal outside state");
        fs::hard_link(&outside, &journal_path).expect("link external file into journal");

        assert!(
            journal.scan_pending(atomic.filesystem()).is_err(),
            "a linked external file must not be accepted as journal evidence"
        );
        assert_eq!(fs::read(&outside).expect("external evidence"), original);
        assert_eq!(fs::read(&journal_path).expect("journal evidence"), original);

        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn append_rejects_hard_linked_journal_without_changing_external_evidence() {
        let (workspace, state) = fixture("append-hardlink");
        let (atomic, journal) = service(&workspace, &state, HardLimits::default());
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let ticket = journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        let journal_path = journal.journal_path(ticket.action_id());
        let outside = state
            .parent()
            .expect("fixture base")
            .join("external-append-evidence");
        let before = fs::read(&journal_path).expect("original journal");

        fs::rename(&journal_path, &outside).expect("move journal outside state");
        fs::hard_link(&outside, &journal_path).expect("link external journal");

        assert!(
            journal.mark_committing(&ticket).is_err(),
            "aliased recovery journal must not be appended"
        );
        assert_eq!(fs::read(&outside).expect("external evidence"), before);
        assert_eq!(fs::read(&journal_path).expect("journal evidence"), before);

        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn retirement_rejects_hard_linked_evidence_without_deleting_it() {
        let (workspace, state) = fixture("retire-hardlink");
        let (atomic, journal) = service(&workspace, &state, HardLimits::default());
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let ticket = journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        let journal_path = journal.journal_path(ticket.action_id());
        let outside = state
            .parent()
            .expect("fixture base")
            .join("retained-evidence");
        let bytes = fs::read(&journal_path).expect("original journal");

        fs::rename(&journal_path, &outside).expect("move journal outside state");
        fs::hard_link(&outside, &journal_path).expect("add linked journal entry");

        assert!(
            journal.retire(&ticket).is_err(),
            "do not retire aliased evidence"
        );
        assert_eq!(fs::read(&outside).expect("external evidence"), bytes);
        assert_eq!(fs::read(&journal_path).expect("journal evidence"), bytes);
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn complete_empty_journal_lines_fail_closed_and_preserve_evidence() {
        let (workspace, state) = fixture("blank-journal-records");
        let (atomic, journal) = service(&workspace, &state, HardLimits::default());
        let path = WorkspacePath::parse("target.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let ticket = journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        journal.mark_committing(&ticket).expect("committing");
        let journal_path = journal.journal_path(ticket.action_id());
        let original = fs::read(&journal_path).expect("original journal");
        let first_terminator = original
            .iter()
            .position(|byte| *byte == b'\n')
            .expect("first journal record");

        let mut interior_blank = original.clone();
        interior_blank.insert(first_terminator + 1, b'\n');
        fs::write(&journal_path, &interior_blank).expect("corrupt interior record");
        assert!(matches!(
            journal.reconcile(atomic.filesystem()),
            Err(RecoveryJournalError::CorruptJournal)
        ));
        assert!(journal_path.exists(), "retain corrupted journal evidence");

        let mut trailing_blank = original;
        trailing_blank.push(b'\n');
        fs::write(&journal_path, &trailing_blank).expect("corrupt trailing record");
        assert!(matches!(
            journal.reconcile(atomic.filesystem()),
            Err(RecoveryJournalError::CorruptJournal)
        ));
        assert!(journal_path.exists(), "retain corrupted journal evidence");
        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");
    }

    #[test]
    fn incomplete_torn_tail_still_uses_last_complete_journal_state() {
        let (workspace, state) = fixture("torn-journal-tail");
        let (atomic, journal) = service(&workspace, &state, HardLimits::default());
        let path = WorkspacePath::parse("new.txt").expect("path");
        let plan = atomic
            .prepare_write(&path, ExpectedState::Absent)
            .expect("plan");
        let ticket = journal
            .begin(
                &plan,
                ExpectedState::Content(ContentVersion::from_bytes(b"new")),
            )
            .expect("begin");
        journal.mark_committing(&ticket).expect("committing");
        let journal_path = journal.journal_path(ticket.action_id());
        let mut contents = fs::read(&journal_path).expect("journal bytes");
        contents.extend_from_slice(b"{\"version\":");
        fs::write(&journal_path, contents).expect("torn append");

        let report = journal
            .scan_pending(atomic.filesystem())
            .expect("inspect")
            .report;
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].outcome,
            RecoveryOutcome::ObservedNotCommitted
        );
        assert!(journal_path.exists(), "inspection retains journal evidence");
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
