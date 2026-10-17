//! Host-owned crash evidence, never restored authorization or replay authority.
use optic_bridge_core::{ActionEnvelope, ActionId, ContentVersion, JobId, ProjectInputManifest};
use optic_bridge_windows::{
    EphemeralAppContainerPlan, OpenedWindowsFile, PinnedDirectoryChain, WindowsFileIdentity,
    inspect_directory_no_reparse, open_exclusive_state_file, open_file_for_delete_no_reparse,
    pin_directory_chain,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf, Prefix},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const DIRECTORY: &str = "private-tasks-v1";
const LOCK: &str = ".owner.lock";
const MAX_RECORDS: usize = 32;
const MAX_UNRESOLVED: usize = 4;
const MAX_BYTES: u64 = 16 * 1024;
const MAX_EVENTS: usize = 8;

#[cfg(test)]
struct RunningPause {
    entered: std::sync::mpsc::SyncSender<()>,
    release: std::sync::mpsc::Receiver<()>,
}

/// One operator-owned journal per selected workspace. Its exclusive native lock
/// and pinned directory live as long as any worker ticket. No source/output text,
/// credentials, raw session/task/job handles, PID or command line is persisted.
pub struct PrivateTaskJournal {
    root: PathBuf,
    workspace: WindowsFileIdentity,
    workspace_digest: String,
    gate: Mutex<()>,
    #[cfg(test)]
    running_pause: Mutex<Option<RunningPause>>,
    _directory: PinnedDirectoryChain,
    _lock: OpenedWindowsFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateTaskRecoveryPhase {
    Intent,
    Allocating,
    Prepared,
    Starting,
    Running,
    Finalizing,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrivateTaskOutcome {
    NoProcess,
    Exited { code: i32 },
    Stopped,
    TimedOut,
    OutputLimitExceeded,
    Failed,
}

#[derive(Clone, Debug)]
pub struct PrivateTaskRecoveryRecord {
    /// Diagnostic identity only; cannot grant permission or recreate a profile.
    pub run_id: String,
    pub phase: PrivateTaskRecoveryPhase,
    pub outcome: Option<PrivateTaskOutcome>,
    pub owner_fingerprint: ContentVersion,
    pub input_fingerprint: ContentVersion,
    /// Validated start-intent evidence only, never a live job or replay token.
    pub execution: Option<PrivateTaskRecoveryExecution>,
    /// Bounded event order is authoritative. Wall-clock observations are only
    /// diagnostic, may go backwards, and never restore authorization/deadlines.
    pub timeline: Vec<PrivateTaskRecoveryObservation>,
}

/// Fingerprints already present in a validated Starting event. No raw handle,
/// native PID, executable path or command is returned. Starting does not prove
/// that resume happened; even Closed does not restore any authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateTaskRecoveryExecution {
    pub action_fingerprint: ContentVersion,
    pub process_task_fingerprint: ContentVersion,
    pub runtime_fingerprint: ContentVersion,
    pub job_fingerprint: ContentVersion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateTaskRecoveryObservation {
    pub phase: PrivateTaskRecoveryPhase,
    /// None for legacy records or an unavailable/unrepresentable system clock.
    pub observed_unix_ms: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    version: u32,
    run: String,
    profile: String,
    workspace: String,
    owner: String,
    read_task: String,
    epoch: u64,
    inputs: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    Intent {
        header: Header,
    },
    Allocating,
    Prepared,
    Starting {
        action: String,
        task: String,
        runtime: String,
        job: String,
    },
    Running,
    Finalizing {
        outcome: PrivateTaskOutcome,
    },
    Closed,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimedEvent {
    entry: Event,
    observed_unix_ms: Option<u64>,
}

#[derive(Serialize)]
struct TimedEventRef<'a> {
    entry: &'a Event,
    observed_unix_ms: Option<u64>,
}

struct Parsed {
    header: Header,
    phase: PrivateTaskRecoveryPhase,
    outcome: Option<PrivateTaskOutcome>,
    events: usize,
    timeline: Vec<PrivateTaskRecoveryObservation>,
    execution: Option<PrivateTaskRecoveryExecution>,
}

pub(crate) struct PrivateTaskJournalTicket {
    journal: Arc<PrivateTaskJournal>,
    run: String,
}

impl PrivateTaskJournal {
    /// State parent must already exist, be outside the selected workspace, and
    /// resolve to a local Windows drive. No recursive directory creation.
    pub fn open(
        state_parent: &Path,
        workspace_root: &Path,
    ) -> Result<Arc<Self>, PrivateTaskJournalError> {
        let workspace_pin = pin_full_directory(workspace_root)?;
        let workspace = inspect_directory_no_reparse(workspace_pin.final_path())?.identity;
        let parent_pin = pin_full_directory(state_parent)?;
        if crate::windows_path::windows_path_is_within(
            workspace_pin.final_path(),
            parent_pin.final_path(),
        ) {
            return Err(PrivateTaskJournalError::StateInsideWorkspace);
        }
        let root = parent_pin.final_path().join(DIRECTORY);
        match fs::create_dir(&root) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        }
        inspect_directory_no_reparse(&root)?;
        let directory = pin_full_directory(&root)?;
        let root = directory.final_path().to_path_buf();
        if crate::windows_path::windows_path_is_within(workspace_pin.final_path(), &root) {
            return Err(PrivateTaskJournalError::StateInsideWorkspace);
        }
        let (mut lock, created) = match open_exclusive_state_file(&root.join(LOCK), true) {
            Ok(file) => (file, true),
            Err(optic_bridge_windows::WindowsFileError::Io(e))
                if e.kind() == std::io::ErrorKind::AlreadyExists =>
            {
                (open_exclusive_state_file(&root.join(LOCK), false)?, false)
            }
            Err(e) => return Err(e.into()),
        };
        if lock.final_path() != root.join(LOCK) {
            return Err(PrivateTaskJournalError::Boundary);
        }
        let workspace_digest = workspace_digest(workspace);
        let expected = format!("optic.private-tasks.v1:{workspace_digest}\n");
        if created {
            lock.file_mut().write_all(expected.as_bytes())?;
            lock.file_mut().sync_all()?;
        } else {
            let observed = read_bounded(lock.file_mut(), 128)?;
            if observed != expected.as_bytes() {
                return Err(PrivateTaskJournalError::WorkspaceMismatch);
            }
        }
        let journal = Arc::new(Self {
            root,
            workspace,
            workspace_digest,
            gate: Mutex::new(()),
            #[cfg(test)]
            running_pause: Mutex::new(None),
            _directory: directory,
            _lock: lock,
        });
        journal.recovery_report()?;
        Ok(journal)
    }

    pub(crate) fn require_workspace(&self, root: &Path) -> Result<(), PrivateTaskJournalError> {
        if inspect_directory_no_reparse(root)?.identity != self.workspace {
            return Err(PrivateTaskJournalError::WorkspaceMismatch);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn pause_next_running_for_test(
        &self,
    ) -> (
        std::sync::mpsc::Receiver<()>,
        std::sync::mpsc::SyncSender<()>,
    ) {
        let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
        assert!(
            self.running_pause
                .lock()
                .unwrap()
                .replace(RunningPause {
                    entered: entered_tx,
                    release: release_rx
                })
                .is_none()
        );
        (entered_rx, release_tx)
    }

    /// Diagnostic snapshot only. Incomplete entries remain unresolved; no native
    /// process is opened/killed, no profile is recreated/deleted, no effect replayed.
    pub fn recovery_report(
        &self,
    ) -> Result<Vec<PrivateTaskRecoveryRecord>, PrivateTaskJournalError> {
        let _gate = self
            .gate
            .lock()
            .map_err(|_| PrivateTaskJournalError::Unavailable)?;
        self.scan()?
            .into_iter()
            .map(|p| {
                Ok(PrivateTaskRecoveryRecord {
                    run_id: p.header.run,
                    phase: p.phase,
                    outcome: p.outcome,
                    owner_fingerprint: ContentVersion::from_hex(&p.header.owner)
                        .map_err(|_| PrivateTaskJournalError::Corrupt)?,
                    input_fingerprint: ContentVersion::from_hex(&p.header.inputs)
                        .map_err(|_| PrivateTaskJournalError::Corrupt)?,
                    timeline: p.timeline,
                    execution: p.execution,
                })
            })
            .collect()
    }

    pub(crate) fn begin(
        self: &Arc<Self>,
        manifest: &ProjectInputManifest,
        first: &ActionEnvelope,
        plan: &EphemeralAppContainerPlan,
    ) -> Result<PrivateTaskJournalTicket, PrivateTaskJournalError> {
        let task = first
            .task_lease
            .as_ref()
            .ok_or(PrivateTaskJournalError::Binding)?;
        manifest
            .require_binding(&first.session, task, first.policy_epoch)
            .map_err(|_| PrivateTaskJournalError::Binding)?;
        let _gate = self
            .gate
            .lock()
            .map_err(|_| PrivateTaskJournalError::Unavailable)?;
        let records = self.scan()?;
        if records
            .iter()
            .filter(|p| p.phase != PrivateTaskRecoveryPhase::Closed)
            .count()
            >= MAX_UNRESOLVED
        {
            return Err(PrivateTaskJournalError::Capacity);
        }
        if records.len() == MAX_RECORDS {
            let old = records
                .iter()
                .find(|p| p.phase == PrivateTaskRecoveryPhase::Closed)
                .ok_or(PrivateTaskJournalError::Capacity)?;
            let path = self.path(&old.header.run);
            let file = open_file_for_delete_no_reparse(&path)?;
            if file.final_path() != path {
                return Err(PrivateTaskJournalError::Boundary);
            }
            file.delete()?;
        }
        let header = Header {
            version: 2,
            run: ActionId::generate()
                .map_err(|_| PrivateTaskJournalError::Unavailable)?
                .to_token(),
            profile: plan.name().to_owned(),
            workspace: self.workspace_digest.clone(),
            owner: digest("owner", &first.session.to_token()),
            read_task: digest("read-task", &task.to_token()),
            epoch: first.policy_epoch,
            inputs: manifest.fingerprint().to_hex(),
        };
        validate_header(&header)?;
        let path = self.path(&header.run);
        let mut file = open_exclusive_state_file(&path, true)?;
        if file.final_path() != path {
            return Err(PrivateTaskJournalError::Boundary);
        }
        let run = header.run.clone();
        append(file.file_mut(), &Event::Intent { header }, 2)?;
        Ok(PrivateTaskJournalTicket {
            journal: Arc::clone(self),
            run,
        })
    }

    fn path(&self, run: &str) -> PathBuf {
        self.root.join(format!("{run}.tasklog"))
    }

    fn scan(&self) -> Result<Vec<Parsed>, PrivateTaskJournalError> {
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let name = entry?
                .file_name()
                .into_string()
                .map_err(|_| PrivateTaskJournalError::Corrupt)?;
            if name == LOCK {
                continue;
            }
            let run = name
                .strip_suffix(".tasklog")
                .filter(|v| is_hex(v, 64))
                .ok_or(PrivateTaskJournalError::Corrupt)?;
            if records.len() == MAX_RECORDS {
                return Err(PrivateTaskJournalError::Capacity);
            }
            let path = self.path(run);
            let mut file = open_exclusive_state_file(&path, false)?;
            if file.final_path() != path {
                return Err(PrivateTaskJournalError::Boundary);
            }
            let record = parse(&read_bounded(file.file_mut(), MAX_BYTES)?, run)?;
            if record.header.workspace != self.workspace_digest {
                return Err(PrivateTaskJournalError::WorkspaceMismatch);
            }
            records.push(record);
        }
        records.sort_by(|a, b| a.header.run.cmp(&b.header.run));
        Ok(records)
    }
}

impl PrivateTaskJournalTicket {
    pub(crate) fn allocating(&self) -> Result<(), PrivateTaskJournalError> {
        self.event(Event::Allocating)
    }
    pub(crate) fn prepared(&self) -> Result<(), PrivateTaskJournalError> {
        self.event(Event::Prepared)
    }
    pub(crate) fn starting(
        &self,
        action: &ActionEnvelope,
        runtime: ContentVersion,
        job: &JobId,
    ) -> Result<(), PrivateTaskJournalError> {
        let task = action
            .task_lease
            .as_ref()
            .ok_or(PrivateTaskJournalError::Binding)?;
        self.event(Event::Starting {
            action: digest("action", &action.action_id.to_token()),
            task: digest("process-task", &task.to_token()),
            runtime: runtime.to_hex(),
            job: digest("job", &job.to_token()),
        })
    }
    pub(crate) fn running(&self) -> Result<(), PrivateTaskJournalError> {
        // Native regression only: suspend the owner after resume, immediately
        // before post-resume journal I/O. No environment-controlled production hook.
        #[cfg(test)]
        if let Some(RunningPause { entered, release }) =
            self.journal.running_pause.lock().unwrap().take()
        {
            entered
                .send(())
                .map_err(|_| PrivateTaskJournalError::Unavailable)?;
            release
                .recv_timeout(std::time::Duration::from_secs(8))
                .map_err(|_| PrivateTaskJournalError::Unavailable)?;
        }
        self.event(Event::Running)
    }
    pub(crate) fn finalizing(
        &self,
        outcome: PrivateTaskOutcome,
    ) -> Result<(), PrivateTaskJournalError> {
        self.event(Event::Finalizing { outcome })
    }
    pub(crate) fn closed(&self) -> Result<(), PrivateTaskJournalError> {
        self.event(Event::Closed)
    }
    fn event(&self, event: Event) -> Result<(), PrivateTaskJournalError> {
        let _gate = self
            .journal
            .gate
            .lock()
            .map_err(|_| PrivateTaskJournalError::Unavailable)?;
        let path = self.journal.path(&self.run);
        let mut file = open_exclusive_state_file(&path, false)?;
        if file.final_path() != path {
            return Err(PrivateTaskJournalError::Boundary);
        }
        let mut parsed = parse(&read_bounded(file.file_mut(), MAX_BYTES)?, &self.run)?;
        if parsed.header.workspace != self.journal.workspace_digest {
            return Err(PrivateTaskJournalError::WorkspaceMismatch);
        }
        advance(&mut parsed, &event)?;
        append(file.file_mut(), &event, parsed.header.version)
    }
}

fn append(file: &mut File, event: &Event, version: u32) -> Result<(), PrivateTaskJournalError> {
    let mut bytes = match version {
        1 => serde_json::to_vec(event),
        2 => serde_json::to_vec(&TimedEventRef {
            entry: event,
            observed_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
        }),
        _ => return Err(PrivateTaskJournalError::Corrupt),
    }
    .map_err(|_| PrivateTaskJournalError::Corrupt)?;
    bytes.push(b'\n');
    if bytes.len() > 2048 || file.metadata()?.len().saturating_add(bytes.len() as u64) > MAX_BYTES {
        return Err(PrivateTaskJournalError::Capacity);
    }
    file.seek(SeekFrom::End(0))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn read_bounded(file: &mut File, limit: u64) -> Result<Vec<u8>, PrivateTaskJournalError> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(PrivateTaskJournalError::Capacity);
    }
    Ok(bytes)
}

fn parse(bytes: &[u8], run: &str) -> Result<Parsed, PrivateTaskJournalError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_BYTES || bytes.last() != Some(&b'\n') {
        return Err(PrivateTaskJournalError::Corrupt);
    }
    let mut lines = bytes[..bytes.len() - 1].split(|b| *b == b'\n');
    let first = lines.next().ok_or(PrivateTaskJournalError::Corrupt)?;
    if first.len() > 2048 {
        return Err(PrivateTaskJournalError::Capacity);
    }
    let (event, observed_unix_ms, version) = decode(first)?;
    let Event::Intent { header } = event else {
        return Err(PrivateTaskJournalError::Corrupt);
    };
    validate_header(&header)?;
    if header.run != run || header.version != version {
        return Err(PrivateTaskJournalError::Corrupt);
    }
    let mut parsed = Parsed {
        header,
        phase: PrivateTaskRecoveryPhase::Intent,
        outcome: None,
        execution: None,
        events: 1,
        timeline: vec![PrivateTaskRecoveryObservation {
            phase: PrivateTaskRecoveryPhase::Intent,
            observed_unix_ms,
        }],
    };
    for line in lines {
        if line.len() > 2048 {
            return Err(PrivateTaskJournalError::Capacity);
        }
        let (event, observed_unix_ms, version) = decode(line)?;
        if version != parsed.header.version {
            return Err(PrivateTaskJournalError::Corrupt);
        }
        advance(&mut parsed, &event)?;
        parsed.timeline.push(PrivateTaskRecoveryObservation {
            phase: parsed.phase,
            observed_unix_ms,
        });
    }
    Ok(parsed)
}

fn decode(line: &[u8]) -> Result<(Event, Option<u64>, u32), PrivateTaskJournalError> {
    if let Ok(event) = serde_json::from_slice::<Event>(line) {
        return Ok((event, None, 1));
    }
    let timed: TimedEvent =
        serde_json::from_slice(line).map_err(|_| PrivateTaskJournalError::Corrupt)?;
    Ok((timed.entry, timed.observed_unix_ms, 2))
}

fn advance(parsed: &mut Parsed, event: &Event) -> Result<(), PrivateTaskJournalError> {
    use PrivateTaskRecoveryPhase as P;
    if parsed.events >= MAX_EVENTS {
        return Err(PrivateTaskJournalError::Capacity);
    }
    parsed.phase = match (parsed.phase, event) {
        (P::Intent, Event::Allocating) => P::Allocating,
        (P::Allocating, Event::Prepared) => P::Prepared,
        (
            P::Prepared,
            Event::Starting {
                action,
                task,
                runtime,
                job,
            },
        ) if [action, task, runtime, job].iter().all(|v| is_hex(v, 64)) => {
            let fingerprint = |value: &str| {
                ContentVersion::from_hex(value).map_err(|_| PrivateTaskJournalError::Corrupt)
            };
            parsed.execution = Some(PrivateTaskRecoveryExecution {
                action_fingerprint: fingerprint(action)?,
                process_task_fingerprint: fingerprint(task)?,
                runtime_fingerprint: fingerprint(runtime)?,
                job_fingerprint: fingerprint(job)?,
            });
            P::Starting
        }
        (P::Starting, Event::Running) => P::Running,
        (P::Allocating | P::Prepared | P::Starting | P::Running, Event::Finalizing { outcome })
            if parsed.phase != P::Running || *outcome != PrivateTaskOutcome::NoProcess =>
        {
            parsed.outcome = Some(*outcome);
            P::Finalizing
        }
        (P::Intent, Event::Closed) => {
            parsed.outcome = Some(PrivateTaskOutcome::NoProcess);
            P::Closed
        }
        (P::Finalizing, Event::Closed) => P::Closed,
        _ => return Err(PrivateTaskJournalError::Transition),
    };
    parsed.events += 1;
    Ok(())
}

fn validate_header(h: &Header) -> Result<(), PrivateTaskJournalError> {
    if !matches!(h.version, 1 | 2)
        || h.epoch == 0
        || ![&h.run, &h.workspace, &h.owner, &h.read_task, &h.inputs]
            .iter()
            .all(|v| is_hex(v, 64))
        || !h
            .profile
            .strip_prefix("Optic.Isolation.")
            .is_some_and(|v| is_hex(v, 32))
    {
        return Err(PrivateTaskJournalError::Corrupt);
    }
    Ok(())
}
fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn digest(domain: &str, value: &str) -> String {
    ContentVersion::from_bytes(format!("optic.private-task.v1:{domain}\0{value}").as_bytes())
        .to_hex()
}
fn workspace_digest(id: WindowsFileIdentity) -> String {
    let mut bytes = b"optic.private-workspace.v1\0".to_vec();
    bytes.extend_from_slice(&id.volume_serial_number.to_le_bytes());
    bytes.extend_from_slice(&id.file_id);
    ContentVersion::from_bytes(&bytes).to_hex()
}

fn pin_full_directory(path: &Path) -> Result<PinnedDirectoryChain, PrivateTaskJournalError> {
    validate_local_directory_shape(path)?;
    let canonical = fs::canonicalize(path)?;
    if canonical.as_os_str().len() > 4096 {
        return Err(PrivateTaskJournalError::Boundary);
    }
    let mut parts = canonical.components();
    let Some(Component::Prefix(prefix)) = parts.next() else {
        return Err(PrivateTaskJournalError::Boundary);
    };
    if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        || parts.next() != Some(Component::RootDir)
    {
        return Err(PrivateTaskJournalError::Boundary);
    }
    let mut anchor = PathBuf::from(prefix.as_os_str());
    anchor.push(r"\");
    let mut relative = PathBuf::new();
    for (n, part) in parts.enumerate() {
        if n == 64 || !matches!(part, Component::Normal(_)) {
            return Err(PrivateTaskJournalError::Boundary);
        }
        relative.push(part.as_os_str());
    }
    Ok(pin_directory_chain(&anchor, Some(&relative))?)
}

fn validate_local_directory_shape(path: &Path) -> Result<(), PrivateTaskJournalError> {
    if path.as_os_str().len() > 4096 {
        return Err(PrivateTaskJournalError::Boundary);
    }
    let mut parts = path.components();
    let Some(Component::Prefix(prefix)) = parts.next() else {
        return Err(PrivateTaskJournalError::Boundary);
    };
    if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        || parts.next() != Some(Component::RootDir)
    {
        return Err(PrivateTaskJournalError::Boundary);
    }
    for (n, part) in parts.enumerate() {
        if n == 64 || !matches!(part, Component::Normal(_)) {
            return Err(PrivateTaskJournalError::Boundary);
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum PrivateTaskJournalError {
    #[error("private task state must be outside the workspace")]
    StateInsideWorkspace,
    #[error("private task state belongs to another workspace")]
    WorkspaceMismatch,
    #[error("private task journal native boundary mismatch")]
    Boundary,
    #[error("private task journal capacity includes unresolved crash records")]
    Capacity,
    #[error("private task journal is corrupt or interrupted; do not replay")]
    Corrupt,
    #[error("private task journal transition is invalid")]
    Transition,
    #[error("private task journal input ownership mismatch")]
    Binding,
    #[error("private task journal owner unavailable")]
    Unavailable,
    #[error("private task journal I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("private task journal native file failed: {0}")]
    Native(#[from] optic_bridge_windows::WindowsFileError),
    #[error("private task journal native directory failed: {0}")]
    Directory(#[from] optic_bridge_windows::PinnedDirectoryError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_bridge_core::{
        Effect, ProjectInputSpec, ResourceBudget, SessionHandle, TaskLeaseId, WorkspacePath,
    };

    struct Fixture {
        root: PathBuf,
        workspace: PathBuf,
        state: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "optic-private-task-journal-{}",
                ActionId::generate().unwrap().to_token()
            ));
            fs::create_dir(&root).unwrap();
            let workspace = root.join("workspace");
            let state = root.join("state");
            fs::create_dir(&workspace).unwrap();
            fs::create_dir(&state).unwrap();
            Self {
                root,
                workspace,
                state,
            }
        }
        fn open(&self) -> Arc<PrivateTaskJournal> {
            PrivateTaskJournal::open(&self.state, &self.workspace).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let root = fs::canonicalize(&self.root).unwrap();
            assert_eq!(
                root.parent().unwrap(),
                fs::canonicalize(std::env::temp_dir()).unwrap()
            );
            assert!(
                root.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("optic-private-task-journal-")
            );
            fs::remove_dir_all(root).unwrap();
        }
    }
    fn input() -> (ProjectInputManifest, ActionEnvelope) {
        let action = ActionEnvelope {
            action_id: ActionId::generate().unwrap(),
            session: SessionHandle::generate().unwrap(),
            task_lease: Some(TaskLeaseId::generate().unwrap()),
            policy_epoch: 1,
            effect: Effect::FileRead {
                path: WorkspacePath::parse("src/main.cjs").unwrap(),
            },
            resources: ResourceBudget {
                timeout_ms: 10000,
                output_bytes: 1024,
                memory_bytes: 16 * 1024 * 1024,
                process_count: 1,
            },
        };
        let manifest = ProjectInputManifest::new(
            action.session.clone(),
            action.task_lease.clone().unwrap(),
            1,
            [ProjectInputSpec {
                path: WorkspacePath::parse("src/main.cjs").unwrap(),
                version: ContentVersion::from_bytes(b"private source must not be recorded"),
                size_bytes: 35,
            }],
        )
        .unwrap();
        (manifest, action)
    }

    #[test]
    fn private_journal_rejects_uninitialized_lock_and_device_paths() {
        let fixture = Fixture::new();
        let root = fixture.state.join(DIRECTORY);
        fs::create_dir(&root).unwrap();
        fs::write(root.join(LOCK), b"").unwrap();
        assert!(PrivateTaskJournal::open(&fixture.state, &fixture.workspace).is_err());
        assert!(fs::read(root.join(LOCK)).unwrap().is_empty());
        for path in [
            r"\\server\share",
            r"\\?\UNC\server\share",
            r"\\.\C:\state",
            r"C:relative",
            r"C:\a\..\state",
        ] {
            assert!(validate_local_directory_shape(Path::new(path)).is_err());
        }
    }

    #[test]
    fn private_journal_reparse_child_cannot_redirect_state_into_source() {
        let fixture = Fixture::new();
        std::os::windows::fs::symlink_dir(&fixture.workspace, fixture.state.join(DIRECTORY))
            .expect("native reparse fixture");
        assert!(PrivateTaskJournal::open(&fixture.state, &fixture.workspace).is_err());
        assert!(!fixture.workspace.join(LOCK).exists());
    }

    #[test]
    fn private_journal_transitions_are_bounded_and_do_not_restore_authority() {
        let fixture = Fixture::new();
        let journal = fixture.open();
        assert!(PrivateTaskJournal::open(&fixture.state, &fixture.workspace).is_err());
        assert!(matches!(
            PrivateTaskJournal::open(&fixture.workspace, &fixture.workspace),
            Err(PrivateTaskJournalError::StateInsideWorkspace)
        ));
        let (manifest, action) = input();
        let plan = EphemeralAppContainerPlan::generate().unwrap();
        let ticket = journal.begin(&manifest, &action, &plan).unwrap();
        assert!(ticket.running().is_err());
        ticket.allocating().unwrap();
        ticket.prepared().unwrap();
        let job = JobId::generate().unwrap();
        ticket
            .starting(&action, ContentVersion::from_bytes(b"runtime"), &job)
            .unwrap();
        ticket.running().unwrap();
        assert!(ticket.finalizing(PrivateTaskOutcome::NoProcess).is_err());
        ticket
            .finalizing(PrivateTaskOutcome::Exited { code: 0 })
            .unwrap();
        ticket.closed().unwrap();
        assert!(ticket.running().is_err());
        let bytes = fs::read(journal.path(&ticket.run)).unwrap();
        for secret in [
            action.session.to_token(),
            action.task_lease.as_ref().unwrap().to_token(),
            action.action_id.to_token(),
            job.to_token(),
            "private source must not be recorded".to_owned(),
        ] {
            assert!(!String::from_utf8_lossy(&bytes).contains(&secret));
        }
        drop(ticket);
        drop(journal);
        let restarted = fixture.open();
        let report = restarted.recovery_report().unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].phase, PrivateTaskRecoveryPhase::Closed);
        assert_eq!(
            report[0].outcome,
            Some(PrivateTaskOutcome::Exited { code: 0 })
        );
        assert_eq!(report[0].input_fingerprint, manifest.fingerprint());
        let execution = report[0].execution.as_ref().unwrap();
        assert_eq!(
            execution.action_fingerprint.to_hex(),
            digest("action", &action.action_id.to_token())
        );
        assert_eq!(
            execution.process_task_fingerprint.to_hex(),
            digest(
                "process-task",
                &action.task_lease.as_ref().unwrap().to_token()
            )
        );
        assert_eq!(
            execution.job_fingerprint.to_hex(),
            digest("job", &job.to_token())
        );
        assert_eq!(
            execution.runtime_fingerprint,
            ContentVersion::from_bytes(b"runtime")
        );
        assert_eq!(report[0].timeline.len(), 7);
        assert_eq!(
            report[0].timeline[0].phase,
            PrivateTaskRecoveryPhase::Intent
        );
        assert_eq!(
            report[0].timeline[6].phase,
            PrivateTaskRecoveryPhase::Closed
        );
        assert!(
            report[0]
                .timeline
                .iter()
                .all(|event| event.observed_unix_ms.is_some())
        );
        drop(restarted);
        let other = fixture.root.join("other");
        fs::create_dir(&other).unwrap();
        assert!(matches!(
            PrivateTaskJournal::open(&fixture.state, &other),
            Err(PrivateTaskJournalError::WorkspaceMismatch)
        ));
    }

    #[test]
    fn private_journal_unresolved_records_survive_restart_and_hold_capacity() {
        let fixture = Fixture::new();
        let journal = fixture.open();
        let (manifest, action) = input();
        for _ in 0..4 {
            let plan = EphemeralAppContainerPlan::generate().unwrap();
            let _unresolved = journal.begin(&manifest, &action, &plan).unwrap();
        }
        drop(journal);
        let restarted = fixture.open();
        assert_eq!(restarted.recovery_report().unwrap().len(), 4);
        assert!(matches!(
            restarted.begin(
                &manifest,
                &action,
                &EphemeralAppContainerPlan::generate().unwrap()
            ),
            Err(PrivateTaskJournalError::Capacity)
        ));
    }

    #[test]
    fn private_journal_legacy_unresolved_records_keep_the_same_capacity_domain() {
        let fixture = Fixture::new();
        let journal = fixture.open();
        let (manifest, action) = input();
        for _ in 0..MAX_UNRESOLVED {
            let ticket = journal
                .begin(
                    &manifest,
                    &action,
                    &EphemeralAppContainerPlan::generate().unwrap(),
                )
                .unwrap();
            let path = journal.path(&ticket.run);
            let mut legacy: TimedEvent = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            let Event::Intent { header } = &mut legacy.entry else {
                panic!("intent")
            };
            header.version = 1;
            let mut bytes = serde_json::to_vec(&legacy.entry).unwrap();
            bytes.push(b'\n');
            fs::write(&path, bytes).unwrap();
            // A retained legacy ticket continues its original encoding.
            ticket.allocating().unwrap();
        }
        drop(journal);
        let restarted = fixture.open();
        let report = restarted.recovery_report().unwrap();
        assert_eq!(report.len(), MAX_UNRESOLVED);
        assert!(report.iter().all(|record| {
            record.phase == PrivateTaskRecoveryPhase::Allocating
                && record.execution.is_none()
                && record.timeline.len() == 2
                && record
                    .timeline
                    .iter()
                    .all(|event| event.observed_unix_ms.is_none())
        }));
        assert!(matches!(
            restarted.begin(
                &manifest,
                &action,
                &EphemeralAppContainerPlan::generate().unwrap()
            ),
            Err(PrivateTaskJournalError::Capacity)
        ));
    }

    #[test]
    fn private_journal_wall_clock_rollback_is_diagnostic_not_transition_authority() {
        let fixture = Fixture::new();
        let journal = fixture.open();
        let (manifest, action) = input();
        let ticket = journal
            .begin(
                &manifest,
                &action,
                &EphemeralAppContainerPlan::generate().unwrap(),
            )
            .unwrap();
        let mut first: TimedEvent =
            serde_json::from_slice(&fs::read(journal.path(&ticket.run)).unwrap()).unwrap();
        first.observed_unix_ms = Some(2000);
        let mut bytes = serde_json::to_vec(&first).unwrap();
        bytes.push(b'\n');
        for (event, time) in [(Event::Allocating, Some(1000)), (Event::Prepared, None)] {
            bytes.extend(
                serde_json::to_vec(&TimedEvent {
                    entry: event,
                    observed_unix_ms: time,
                })
                .unwrap(),
            );
            bytes.push(b'\n');
        }
        let parsed = parse(&bytes, &ticket.run).unwrap();
        assert_eq!(parsed.phase, PrivateTaskRecoveryPhase::Prepared);
        assert_eq!(
            parsed
                .timeline
                .iter()
                .map(|event| event.observed_unix_ms)
                .collect::<Vec<_>>(),
            vec![Some(2000), Some(1000), None]
        );
        // A timestamp cannot make an invalid phase transition acceptable.
        bytes.extend_from_slice(
            b"{\"entry\":{\"event\":\"running\"},\"observed_unix_ms\":999999999}\n",
        );
        assert!(matches!(
            parse(&bytes, &ticket.run),
            Err(PrivateTaskJournalError::Transition)
        ));
    }

    #[test]
    fn private_journal_rejects_mixed_versions_and_malformed_observations() {
        let fixture = Fixture::new();
        let journal = fixture.open();
        let (manifest, action) = input();
        let ticket = journal
            .begin(
                &manifest,
                &action,
                &EphemeralAppContainerPlan::generate().unwrap(),
            )
            .unwrap();
        let original = fs::read(journal.path(&ticket.run)).unwrap();
        for suffix in [
            "{\"event\":\"allocating\"}\n",
            "{\"entry\":{\"event\":\"allocating\"},\"observed_unix_ms\":-1}\n",
            "{\"entry\":{\"event\":\"allocating\"},\"observed_unix_ms\":1.5}\n",
            "{\"entry\":{\"event\":\"allocating\"},\"observed_unix_ms\":null,\"extra\":1}\n",
        ] {
            let mut bytes = original.clone();
            bytes.extend_from_slice(suffix.as_bytes());
            assert!(matches!(
                parse(&bytes, &ticket.run),
                Err(PrivateTaskJournalError::Corrupt)
            ));
        }
        let mut timed: TimedEvent = serde_json::from_slice(&original).unwrap();
        let Event::Intent { header } = &mut timed.entry else {
            panic!("intent")
        };
        header.version = 1;
        let mut mismatched = serde_json::to_vec(&timed).unwrap();
        mismatched.push(b'\n');
        assert!(matches!(
            parse(&mismatched, &ticket.run),
            Err(PrivateTaskJournalError::Corrupt)
        ));
    }

    #[test]
    fn private_journal_starting_correlation_survives_legacy_and_timed_restart() {
        for legacy in [false, true] {
            let fixture = Fixture::new();
            let journal = fixture.open();
            let (manifest, action) = input();
            let ticket = journal
                .begin(
                    &manifest,
                    &action,
                    &EphemeralAppContainerPlan::generate().unwrap(),
                )
                .unwrap();
            assert!(journal.recovery_report().unwrap()[0].execution.is_none());
            if legacy {
                let path = journal.path(&ticket.run);
                let mut timed: TimedEvent =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                let Event::Intent { header } = &mut timed.entry else {
                    panic!("intent")
                };
                header.version = 1;
                let mut bytes = serde_json::to_vec(&timed.entry).unwrap();
                bytes.push(b'\n');
                fs::write(path, bytes).unwrap();
            }
            ticket.allocating().unwrap();
            ticket.prepared().unwrap();
            let job = JobId::generate().unwrap();
            let runtime = ContentVersion::from_bytes(b"selected runtime");
            ticket.starting(&action, runtime, &job).unwrap();
            let path = journal.path(&ticket.run);
            drop(ticket);
            drop(journal);
            let restarted = fixture.open();
            let report = restarted.recovery_report().unwrap();
            assert_eq!(report[0].phase, PrivateTaskRecoveryPhase::Starting);
            assert!(report[0].outcome.is_none());
            let execution = report[0].execution.as_ref().unwrap();
            assert_eq!(
                execution.action_fingerprint.to_hex(),
                digest("action", &action.action_id.to_token())
            );
            assert_eq!(
                execution.process_task_fingerprint.to_hex(),
                digest(
                    "process-task",
                    &action.task_lease.as_ref().unwrap().to_token()
                )
            );
            assert_eq!(execution.runtime_fingerprint, runtime);
            assert_eq!(
                execution.job_fingerprint.to_hex(),
                digest("job", &job.to_token())
            );
            let bytes = fs::read(&path).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            for secret in [
                action.action_id.to_token(),
                action.task_lease.as_ref().unwrap().to_token(),
                job.to_token(),
            ] {
                assert!(!text.contains(&secret));
            }
            // Malformed start evidence never becomes a partially trusted report.
            let corrupt = text.replace(&digest("job", &job.to_token()), &"z".repeat(64));
            fs::write(&path, corrupt).unwrap();
            assert!(restarted.recovery_report().is_err());
        }
    }

    #[test]
    fn private_journal_terminal_history_is_bounded() {
        let fixture = Fixture::new();
        let journal = fixture.open();
        let (manifest, action) = input();
        for _ in 0..40 {
            let plan = EphemeralAppContainerPlan::generate().unwrap();
            let ticket = journal.begin(&manifest, &action, &plan).unwrap();
            ticket.closed().unwrap();
        }
        assert_eq!(journal.recovery_report().unwrap().len(), MAX_RECORDS);
        assert_eq!(
            fs::read_dir(&journal.root).unwrap().count(),
            MAX_RECORDS + 1
        );
    }

    #[test]
    fn private_journal_rejects_interrupted_unknown_and_reordered_data() {
        let fixture = Fixture::new();
        let journal = fixture.open();
        let (manifest, action) = input();
        let ticket = journal
            .begin(
                &manifest,
                &action,
                &EphemeralAppContainerPlan::generate().unwrap(),
            )
            .unwrap();
        let path = journal.path(&ticket.run);
        let original = fs::read(&path).unwrap();
        let mut interrupted = original.clone();
        interrupted.extend_from_slice(b"{\"event\":\"alloc");
        fs::write(&path, interrupted).unwrap();
        assert!(matches!(
            journal.recovery_report(),
            Err(PrivateTaskJournalError::Corrupt)
        ));
        assert!(ticket.allocating().is_err());
        let mut reordered = original.clone();
        reordered
            .extend_from_slice(b"{\"entry\":{\"event\":\"running\"},\"observed_unix_ms\":1}\n");
        fs::write(&path, reordered).unwrap();
        assert!(matches!(
            journal.recovery_report(),
            Err(PrivateTaskJournalError::Transition)
        ));
        fs::write(&path, original).unwrap();
        fs::write(journal.root.join("unknown"), b"x").unwrap();
        assert!(matches!(
            journal.recovery_report(),
            Err(PrivateTaskJournalError::Corrupt)
        ));
    }

    #[test]
    fn private_journal_crash_child() {
        let Some(root) = std::env::var_os("OPTIC_PRIVATE_JOURNAL_CRASH_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        assert_eq!(
            fs::canonicalize(&root).unwrap().parent().unwrap(),
            fs::canonicalize(std::env::temp_dir()).unwrap()
        );
        let journal =
            PrivateTaskJournal::open(&root.join("state"), &root.join("workspace")).unwrap();
        let (manifest, action) = input();
        let plan = EphemeralAppContainerPlan::generate().unwrap();
        let ticket = journal.begin(&manifest, &action, &plan).unwrap();
        ticket.allocating().unwrap();
        // Abrupt process exit bypasses Drop, before any native profile is created.
        std::process::exit(71);
    }

    #[test]
    fn private_journal_recovers_evidence_after_abrupt_owner_exit() {
        let fixture = Fixture::new();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "private_task_journal::tests::private_journal_crash_child",
                "--nocapture",
            ])
            .env("OPTIC_PRIVATE_JOURNAL_CRASH_ROOT", &fixture.root)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(71));
        let journal = fixture.open();
        let report = journal.recovery_report().unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].phase, PrivateTaskRecoveryPhase::Allocating);
        assert!(report[0].outcome.is_none());
    }
}
