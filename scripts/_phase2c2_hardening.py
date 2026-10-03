from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected exactly one match in {path}: got {count}\n{old[:160]}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


mutation = Path("crates/optic-bridge-runtime/src/mutation.rs")
replace_once(
    mutation,
    '''use std::path::Path;\n\n#[cfg(all(test, not(windows)))]\nuse std::path::PathBuf;\n#[cfg(windows)]\nuse std::{\n    fs::{self, OpenOptions},\n    io::Write,\n    path::PathBuf,\n};''',
    '''use std::path::Path;\n\n#[cfg(any(windows, all(test, not(windows))))]\nuse std::path::PathBuf;\n#[cfg(windows)]\nuse std::{\n    fs::{self, OpenOptions},\n    io::Write,\n};''',
)
replace_once(
    mutation,
    '''const STAGING_PREFIX: &str = ".optic-";\nconst STAGING_SUFFIX: &str = ".staged";''',
    '''#[cfg(windows)]\nconst STAGING_PREFIX: &str = ".optic-";\n#[cfg(windows)]\nconst STAGING_SUFFIX: &str = ".staged";''',
)
replace_once(
    mutation,
    '''pub(crate) fn staging_file_name(action_id: &ActionId) -> String {''',
    '''#[cfg(windows)]\npub(crate) fn staging_file_name(action_id: &ActionId) -> String {''',
)
replace_once(
    mutation,
    '''pub(crate) fn staging_path_for(\n    filesystem: &BoundedFileSystem,''',
    '''#[cfg(windows)]\npub(crate) fn staging_path_for(\n    filesystem: &BoundedFileSystem,''',
)
replace_once(
    mutation,
    '''fn absolute_workspace_path(root: &Path, path: &WorkspacePath) -> PathBuf {''',
    '''#[cfg(windows)]\nfn absolute_workspace_path(root: &Path, path: &WorkspacePath) -> PathBuf {''',
)

recovery = Path("crates/optic-bridge-runtime/src/recovery.rs")
text = recovery.read_text(encoding="utf-8")
start = text.index("    pub fn reconcile(\n")
end = text.index("    fn append(\n", start)
replacement = '''    pub fn reconcile(\n        &self,\n        filesystem: &BoundedFileSystem,\n    ) -> Result<RecoveryReport, RecoveryJournalError> {\n        let report = self.inspect_pending(filesystem)?;\n        for record in &report.records {\n            self.retire_action(&record.action_id)?;\n        }\n        Ok(report)\n    }\n\n    pub(crate) fn inspect_pending(\n        &self,\n        filesystem: &BoundedFileSystem,\n    ) -> Result<RecoveryReport, RecoveryJournalError> {\n        let mut paths = Vec::new();\n        for entry in fs::read_dir(&self.root)? {\n            if paths.len() >= self.max_records {\n                return Err(RecoveryJournalError::RecoveryRecordLimitExceeded {\n                    limit: self.max_records,\n                });\n            }\n            paths.push(entry?.path());\n        }\n        paths.sort();\n\n        let mut records = Vec::new();\n        for path in paths {\n            let file_name = path\n                .file_name()\n                .and_then(|name| name.to_str())\n                .ok_or(RecoveryJournalError::UnexpectedJournalEntry)?;\n\n            if let Some(token) = file_name.strip_suffix(PREPARED_TEMP_SUFFIX) {\n                ActionId::from_token(token)?;\n                self.remove_regular_file(&path)?;\n                continue;\n            }\n\n            let token = file_name\n                .strip_suffix(JOURNAL_SUFFIX)\n                .ok_or(RecoveryJournalError::UnexpectedJournalEntry)?;\n            let action_id = ActionId::from_token(token)?;\n            let parsed = self.read_journal(&path, &action_id)?;\n            let outcome = match parsed.latest_state {\n                JournalState::Prepared => RecoveryOutcome::PreparedNotCommitted,\n                JournalState::Verified => RecoveryOutcome::VerifiedTerminal,\n                JournalState::Committing | JournalState::Ambiguous => {\n                    let observation = filesystem\n                        .observe_mutation_target(&parsed.ticket.canonical_path)\n                        .map_err(RecoveryJournalError::RecoveryObservation)?;\n                    if observation.canonical_path != parsed.ticket.canonical_path {\n                        return Err(RecoveryJournalError::UnresolvedRecoveryConflict {\n                            path: parsed.ticket.canonical_path.as_str().to_owned(),\n                        });\n                    }\n                    if parsed.ticket.matches_intended(observation.state) {\n                        RecoveryOutcome::ObservedCommitted\n                    } else if parsed.ticket.previous.matches(observation.state) {\n                        RecoveryOutcome::ObservedNotCommitted\n                    } else {\n                        return Err(RecoveryJournalError::UnresolvedRecoveryConflict {\n                            path: parsed.ticket.canonical_path.as_str().to_owned(),\n                        });\n                    }\n                }\n            };\n\n            let intended_version_hex = parsed.ticket.intended_version_hex.clone();\n            records.push(RecoveryRecord {\n                action_id: parsed.ticket.action_id,\n                canonical_path: parsed.ticket.canonical_path,\n                intended_version_hex,\n                outcome,\n            });\n        }\n\n        Ok(RecoveryReport { records })\n    }\n\n    pub(crate) fn retire_action(\n        &self,\n        action_id: &ActionId,\n    ) -> Result<(), RecoveryJournalError> {\n        self.remove_regular_file(&self.journal_path(action_id))\n    }\n\n'''
recovery.write_text(text[:start] + replacement + text[end:], encoding="utf-8")
replace_once(
    recovery,
    '''pub struct RecoveryRecord {\n    pub action_id: ActionId,\n    pub canonical_path: WorkspacePath,\n    pub outcome: RecoveryOutcome,\n}''',
    '''pub struct RecoveryRecord {\n    pub action_id: ActionId,\n    pub canonical_path: WorkspacePath,\n    pub intended_version_hex: String,\n    pub outcome: RecoveryOutcome,\n}''',
)

journaled = Path("crates/optic-bridge-runtime/src/journaled_mutation.rs")
replace_once(
    journaled,
    '''#[cfg(windows)]\nuse std::fs;\n\nuse optic_bridge_core::{ActionId, ContentVersion, ExpectedState, HardLimits, WorkspacePath};''',
    '''#[cfg(windows)]\nuse std::fs::{self, File};\n\nuse optic_bridge_core::{ActionId, ExpectedState, HardLimits, WorkspacePath};\n#[cfg(windows)]\nuse optic_bridge_core::{ContentVersion, ContentVersionReadError};''',
)
replace_once(
    journaled,
    '''use crate::{\n    AtomicMutationError, AtomicMutationService, CommitVerification, MutationCommit,\n    MutationRecoveryJournal, PreparedMutation, RecoveryJournalError, RecoveryOutcome,\n    RecoveryReport,\n};''',
    '''use crate::{\n    AtomicMutationError, AtomicMutationService, MutationCommit, MutationRecoveryJournal,\n    PreparedMutation, RecoveryJournalError, RecoveryReport,\n};\n#[cfg(windows)]\nuse crate::{CommitVerification, RecoveryOutcome};''',
)
replace_once(
    journaled,
    '''pub struct JournaledMutationService {\n    atomic: AtomicMutationService,\n    journal: MutationRecoveryJournal,\n}''',
    '''pub struct JournaledMutationService {\n    atomic: AtomicMutationService,\n    journal: MutationRecoveryJournal,\n    max_mutation_bytes: u64,\n}''',
)
replace_once(
    journaled,
    '''        let journal =\n            MutationRecoveryJournal::open(state_root, atomic.filesystem().root(), limits)?;\n        Ok(Self { atomic, journal })''',
    '''        let journal =\n            MutationRecoveryJournal::open(state_root, atomic.filesystem().root(), limits)?;\n        Ok(Self {\n            atomic,\n            journal,\n            max_mutation_bytes: limits.max_fs_mutation_bytes,\n        })''',
)
replace_once(
    journaled,
    '''    pub fn recover(&self) -> Result<RecoveryReport, JournaledMutationError> {\n        let report = self.journal.reconcile(self.atomic.filesystem())?;\n        #[cfg(windows)]\n        self.cleanup_recovered_staging(&report)?;\n        Ok(report)\n    }''',
    '''    pub fn recover(&self) -> Result<RecoveryReport, JournaledMutationError> {\n        let report = self.journal.inspect_pending(self.atomic.filesystem())?;\n        #[cfg(windows)]\n        self.cleanup_recovered_staging(&report)?;\n        for record in &report.records {\n            self.journal.retire_action(&record.action_id)?;\n        }\n        Ok(report)\n    }''',
)
replace_once(
    journaled,
    '''            if record.outcome == RecoveryOutcome::PreparedNotCommitted {\n                return Err(JournaledMutationError::UnexpectedPreparedStagingArtifact {\n                    action_id: record.action_id.clone(),\n                });\n            }\n\n            fs::remove_file(&staging).map_err(|source| {''',
    '''            if record.outcome == RecoveryOutcome::PreparedNotCommitted {\n                return Err(JournaledMutationError::UnexpectedPreparedStagingArtifact {\n                    action_id: record.action_id.clone(),\n                });\n            }\n            if metadata.len() > self.max_mutation_bytes {\n                return Err(JournaledMutationError::StagingArtifactTooLarge {\n                    action_id: record.action_id.clone(),\n                    limit: self.max_mutation_bytes,\n                });\n            }\n\n            let mut file = File::open(&staging).map_err(|source| {\n                JournaledMutationError::StagingCleanupFailed {\n                    action_id: record.action_id.clone(),\n                    source,\n                }\n            })?;\n            let observed = ContentVersion::from_reader_bounded(&mut file, self.max_mutation_bytes)\n                .map_err(|source| JournaledMutationError::StagingObservationFailed {\n                    action_id: record.action_id.clone(),\n                    source,\n                })?;\n            if observed.to_hex() != record.intended_version_hex {\n                return Err(JournaledMutationError::StagingContentMismatch {\n                    action_id: record.action_id.clone(),\n                });\n            }\n            drop(file);\n\n            fs::remove_file(&staging).map_err(|source| {''',
)
replace_once(
    journaled,
    '''    #[error("operation {action_id:?} staging cleanup failed: {source}")]\n    StagingCleanupFailed {\n        action_id: ActionId,\n        #[source]\n        source: std::io::Error,\n    },''',
    '''    #[error("operation {action_id:?} staging artifact exceeds hard byte ceiling {limit}")]\n    StagingArtifactTooLarge { action_id: ActionId, limit: u64 },\n    #[error("operation {action_id:?} staging content observation failed: {source}")]\n    StagingObservationFailed {\n        action_id: ActionId,\n        #[source]\n        source: ContentVersionReadError,\n    },\n    #[error("operation {action_id:?} staging content no longer matches its journaled intent")]\n    StagingContentMismatch { action_id: ActionId },\n    #[error("operation {action_id:?} staging cleanup failed: {source}")]\n    StagingCleanupFailed {\n        action_id: ActionId,\n        #[source]\n        source: std::io::Error,\n    },''',
)
insert_after = '''    #[cfg(windows)]\n    #[test]\n    fn recovery_removes_regular_operation_owned_staging_after_committing() {'''
pos = journaled.read_text(encoding="utf-8").index(insert_after)
# Add a mismatch-retention test immediately before the existing cleanup test.
text = journaled.read_text(encoding="utf-8")
test = '''    #[cfg(windows)]\n    #[test]\n    fn staging_mismatch_fails_closed_and_retains_journal_evidence() {\n        let (workspace, state) = fixture("staging-mismatch");\n        let service =\n            JournaledMutationService::from_hard_limits(&workspace, &state, HardLimits::default())\n                .expect("service");\n        let path = WorkspacePath::parse("target.txt").expect("path");\n        let plan = service\n            .prepare_write(&path, ExpectedState::Absent)\n            .expect("plan");\n        let ticket = service\n            .journal()\n            .begin(&plan, ContentVersion::from_bytes(b"new"))\n            .expect("begin");\n        service\n            .journal()\n            .mark_committing(&ticket)\n            .expect("committing");\n\n        let staging = staging_path_for(service.atomic().filesystem(), &path, ticket.action_id());\n        fs::write(&staging, b"tampered").expect("staging fixture");\n        assert!(matches!(\n            service.recover(),\n            Err(JournaledMutationError::StagingContentMismatch { .. })\n        ));\n        assert!(staging.exists());\n        assert_eq!(\n            fs::read_dir(service.journal().root())\n                .expect("journal root")\n                .count(),\n            1\n        );\n        fs::remove_dir_all(workspace.parent().expect("base")).expect("cleanup");\n    }\n\n'''
journaled.write_text(text[:pos] + test + text[pos:], encoding="utf-8")

print("phase2c2 hardening patch applied")
