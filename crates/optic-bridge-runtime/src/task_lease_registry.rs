use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use optic_bridge_core::{
    HardLimits, LeaseScope, LimitError, MonotonicTime, SessionHandle, TaskLease, TaskLeaseId,
};
use thiserror::Error;

use crate::process_authority::{ProcessAuthority, ProcessAuthorityError};

#[derive(Debug)]
pub struct TaskLeaseRegistry {
    leases: Mutex<HashMap<TaskLeaseId, LeaseRecord>>,
    max_leases: u32,
    max_leases_per_session: u32,
}

impl Default for TaskLeaseRegistry {
    fn default() -> Self {
        let limits = HardLimits::default();
        Self {
            leases: Mutex::new(HashMap::new()),
            max_leases: limits.max_task_leases,
            max_leases_per_session: limits.max_task_leases_per_session,
        }
    }
}

#[derive(Debug)]
struct LeaseRecord {
    #[cfg(any(windows, test))]
    clock_identity: Option<crate::clock::ClockIdentity>,
    lease: TaskLease,
    revoked: bool,
    revocation_signal: Arc<AtomicBool>,
    process_authorities: HashMap<String, ProcessAuthority>,
}

impl LeaseRecord {
    fn new(lease: TaskLease) -> Self {
        Self {
            #[cfg(any(windows, test))]
            clock_identity: None,
            lease,
            revoked: false,
            revocation_signal: Arc::new(AtomicBool::new(false)),
            process_authorities: HashMap::new(),
        }
    }
}

#[cfg(any(windows, test))]
#[derive(Debug, Error)]
pub(crate) enum TaskLeaseClockError {
    #[error("task lease clock lifetime must be positive and representable")]
    InvalidDeadline,
    #[error(transparent)]
    Registry(#[from] TaskLeaseRegistryError),
}

/// Read-only observation bound to one concrete, verified TaskLease record.
/// This is cancellation evidence, not approval to create or resume a process.
/// Removal and even reuse of the opaque lease ID cannot retarget this signal.
#[derive(Clone, Debug)]
pub(crate) struct TaskLeaseRevocationWatch {
    #[cfg(any(windows, test))]
    clock_identity: Option<crate::clock::ClockIdentity>,
    signal: Arc<AtomicBool>,
}

impl TaskLeaseRevocationWatch {
    #[cfg(any(windows, test))]
    #[allow(dead_code)] // Required by the inert native clock boundary.
    pub(crate) fn belongs_to_clock(&self, clock: &crate::StdClock) -> bool {
        self.clock_identity
            .as_ref()
            .is_some_and(|identity| identity.matches(clock))
    }

    /// Retain the exact original registration; no lookup by reusable public ID.
    #[cfg(windows)]
    pub(crate) fn native_stop_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.signal)
    }

    /// Compare the actual retained registry generation, not an opaque lease ID
    /// that could be reused by a new or foreign registry entry. This is a
    /// same-record *observation*, never process-creation authority.
    #[cfg(windows)]
    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.signal, &other.signal)
    }

    // The future native owner will consume this generation-bound stop watch.
    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn is_revoked(&self) -> bool {
        self.signal.load(Ordering::Acquire)
    }
}

/// Non-cloneable observation of the *original* TaskLease revocation order,
/// with its immutable admitted deadline. No process launch authority.
#[cfg(windows)]
#[must_use = "retain the original TaskLease stop watch through owner cleanup"]
#[allow(dead_code)] // Used only by the inert native owner stage.
pub(crate) struct TaskLeaseNativeOwnerCommit {
    original_stop: TaskLeaseRevocationWatch,
    original_expires_at: MonotonicTime,
}

#[cfg(windows)]
#[allow(dead_code)] // No production native guardian consumes the receipt yet.
impl TaskLeaseNativeOwnerCommit {
    pub(crate) fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.original_stop.is_revoked() || now >= self.original_expires_at
    }
}

impl TaskLeaseRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_hard_limits(limits: HardLimits) -> Result<Self, LimitError> {
        let limits = limits.validate_nonzero()?;
        Ok(Self {
            leases: Mutex::new(HashMap::new()),
            max_leases: limits.max_task_leases,
            max_leases_per_session: limits.max_task_leases_per_session,
        })
    }

    pub fn register(&self, lease: TaskLease) -> Result<(), TaskLeaseRegistryError> {
        self.register_record(LeaseRecord::new(lease))
    }

    /// Issue a new registration on the retained service clock. The input lease
    /// supplies scope/capability metadata, never an existing expiry to attest.
    #[cfg(any(windows, test))]
    #[allow(dead_code)] // No production native launcher consumes this issuer yet.
    pub(crate) fn register_on_clock(
        &self,
        mut lease: TaskLease,
        lifetime_ms: u64,
        clock: &crate::StdClock,
    ) -> Result<TaskLease, TaskLeaseClockError> {
        if lifetime_ms == 0 {
            return Err(TaskLeaseClockError::InvalidDeadline);
        }
        lease.expires_at = crate::Clock::now(clock)
            .as_millis()
            .checked_add(lifetime_ms)
            .map(MonotonicTime::from_millis)
            .ok_or(TaskLeaseClockError::InvalidDeadline)?;
        let mut record = LeaseRecord::new(lease.clone());
        record.clock_identity = Some(clock.identity());
        self.register_record(record)?;
        Ok(lease)
    }

    fn register_record(&self, mut record: LeaseRecord) -> Result<(), TaskLeaseRegistryError> {
        let lease = &record.lease;
        // Reject known duplicates and exhausted capacity before opening and
        // hashing an executable. Never hold the registry lock during file IO.
        {
            let leases = self
                .leases
                .lock()
                .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
            self.validate_registration(&leases, lease)?;
        }

        record.process_authorities = capture_process_authorities(lease)?;

        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        // Concurrent registrations/removals may have changed the result since
        // preflight. Only this locked second check authorizes insertion.
        self.validate_registration(&leases, lease)?;
        leases.insert(lease.id.clone(), record);
        Ok(())
    }

    fn validate_registration(
        &self,
        leases: &HashMap<TaskLeaseId, LeaseRecord>,
        lease: &TaskLease,
    ) -> Result<(), TaskLeaseRegistryError> {
        if leases.contains_key(&lease.id) {
            return Err(TaskLeaseRegistryError::AlreadyRegistered);
        }
        let max_leases = usize::try_from(self.max_leases)
            .map_err(|_| TaskLeaseRegistryError::CapacityExceeded)?;
        if leases.len() >= max_leases {
            return Err(TaskLeaseRegistryError::CapacityExceeded);
        }
        let session_count = leases
            .values()
            .filter(|record| record.lease.session == lease.session)
            .count();
        let max_leases_per_session = usize::try_from(self.max_leases_per_session)
            .map_err(|_| TaskLeaseRegistryError::SessionCapacityExceeded)?;
        if session_count >= max_leases_per_session {
            return Err(TaskLeaseRegistryError::SessionCapacityExceeded);
        }
        Ok(())
    }

    pub fn get_active(
        &self,
        id: &TaskLeaseId,
        session: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<TaskLease, TaskLeaseRegistryError> {
        self.get_active_with_revocation_watch(id, session, now)
            .map(|(lease, _watch)| lease)
    }

    /// Verify the exact registered lease, including the pinned executable
    /// identity, and capture its original revocation signal under ONE lock.
    /// Consumers must still enforce session admission, typed policy, the
    /// native watchdog deadline and final pre-spawn revocation validation.
    /// No native child or owner is created by this observation.
    pub(crate) fn get_active_with_revocation_watch(
        &self,
        id: &TaskLeaseId,
        session: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<(TaskLease, TaskLeaseRevocationWatch), TaskLeaseRegistryError> {
        let leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let record = leases.get(id).ok_or(TaskLeaseRegistryError::UnknownLease)?;
        validate_active_record(record, session, now)?;
        validate_process_authorities(record)?;
        Ok((
            record.lease.clone(),
            TaskLeaseRevocationWatch {
                #[cfg(any(windows, test))]
                clock_identity: record.clock_identity.clone(),
                signal: Arc::clone(&record.revocation_signal),
            },
        ))
    }

    pub(crate) fn renew_active_session(
        &self,
        session: &SessionHandle,
        now: MonotonicTime,
        expected_policy_epoch: u64,
        new_expires_at: MonotonicTime,
    ) -> Result<usize, TaskLeaseRenewalError> {
        if new_expires_at <= now {
            return Err(TaskLeaseRenewalError::ExpiryNotFuture);
        }
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRenewalError::StateUnavailable)?;

        for record in leases.values() {
            if &record.lease.session != session || record.revoked {
                continue;
            }
            if record.lease.is_expired_at(now) {
                return Err(TaskLeaseRenewalError::ExpiredLease);
            }
            if record.lease.policy_epoch != expected_policy_epoch {
                return Err(TaskLeaseRenewalError::PolicyEpochMismatch);
            }
            validate_process_authorities(record)
                .map_err(|_| TaskLeaseRenewalError::AuthorityInvalid)?;
        }

        let mut changed = 0;
        for record in leases.values_mut() {
            if &record.lease.session == session
                && !record.revoked
                && record.lease.expires_at < new_expires_at
            {
                record.lease.expires_at = new_expires_at;
                #[cfg(any(windows, test))]
                {
                    // Untagged renewal cannot attest a new expiry's origin.
                    record.clock_identity = None;
                }
                changed += 1;
            }
        }
        Ok(changed)
    }

    pub fn revoke(&self, id: &TaskLeaseId) -> Result<bool, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let Some(record) = leases.get_mut(id) else {
            return Ok(false);
        };
        let changed = !record.revoked;
        record.revoked = true;
        record.revocation_signal.store(true, Ordering::Release);
        Ok(changed)
    }

    pub fn revoke_session(&self, session: &SessionHandle) -> Result<usize, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let mut changed = 0;
        for record in leases.values_mut() {
            if &record.lease.session == session && !record.revoked {
                record.revoked = true;
                record.revocation_signal.store(true, Ordering::Release);
                changed += 1;
            }
        }
        Ok(changed)
    }

    /// Mint a *lease-only* commit observation for a previously registered,
    /// non-executing native owner, ordered under the SAME mutex as direct
    /// `revoke`, `revoke_session` and `remove_session`.
    ///
    /// The original witness's Arc identity is mandatory. An opaque TaskLeaseId
    /// can be reused after revocation and removal, but may never revive an
    /// older owner. This critical section does NO file IO, native calls,
    /// session locking or policy callbacks.
    ///
    /// The result is NOT a launch permission: session close and policy
    /// authorities are independent; their observations are not atomic.
    #[cfg(windows)]
    #[allow(dead_code)] // Inert research stage; no live native launch consumes it.
    pub(crate) fn commit_original_native_read_lease(
        &self,
        id: &TaskLeaseId,
        original: &TaskLeaseRevocationWatch,
        original_expires_at: MonotonicTime,
        session: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<TaskLeaseNativeOwnerCommit, TaskLeaseRegistryError> {
        let leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let record = leases.get(id).ok_or(TaskLeaseRegistryError::UnknownLease)?;
        if !Arc::ptr_eq(&record.revocation_signal, &original.signal) {
            // Fail closed as Revoked: changing this public error enum would
            // broaden the MCP contract for a private, non-executing stage.
            // A foreign or reused generation is never an active authority.
            return Err(TaskLeaseRegistryError::Revoked);
        }
        if record.revoked {
            return Err(TaskLeaseRegistryError::Revoked);
        }
        if &record.lease.session != session {
            return Err(TaskLeaseRegistryError::WrongSession);
        }
        // Renewal can extend a registry entry, never the retained owner.
        if record.lease.is_expired_at(now) || now >= original_expires_at {
            return Err(TaskLeaseRegistryError::Expired);
        }
        Ok(TaskLeaseNativeOwnerCommit {
            original_stop: original.clone(),
            original_expires_at,
        })
    }

    /// Cheap lifetime observation only. Callers must already have verified and
    /// pinned executable authority before launch; this never replaces admission.
    #[cfg(windows)]
    pub(crate) fn watch_active(
        &self,
        id: &TaskLeaseId,
        session: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<(Arc<AtomicBool>, TaskLease), TaskLeaseRegistryError> {
        let leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let record = leases.get(id).ok_or(TaskLeaseRegistryError::UnknownLease)?;
        validate_active_record(record, session, now)?;
        Ok((Arc::clone(&record.revocation_signal), record.lease.clone()))
    }

    pub(crate) fn remove_revoked(&self, id: &TaskLeaseId) -> Result<bool, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let Some(record) = leases.get(id) else {
            return Ok(false);
        };
        if !record.revoked {
            return Err(TaskLeaseRegistryError::LeaseStillActive);
        }
        leases.remove(id);
        Ok(true)
    }

    pub(crate) fn remove_session(
        &self,
        session: &SessionHandle,
    ) -> Result<usize, TaskLeaseRegistryError> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        let before = leases.len();
        leases.retain(|_, record| {
            if &record.lease.session != session {
                return true;
            }
            // Even an isolated physical cleanup must stop an old generation.
            // Publish while holding this registry lock and before losing the
            // record; a later lease with the same ID gets its own signal.
            // This is only a stop observation, never termination evidence.
            record.revoked = true;
            record.revocation_signal.store(true, Ordering::Release);
            false
        });
        Ok(before - leases.len())
    }
}

fn validate_active_record(
    record: &LeaseRecord,
    session: &SessionHandle,
    now: MonotonicTime,
) -> Result<(), TaskLeaseRegistryError> {
    if record.revoked {
        return Err(TaskLeaseRegistryError::Revoked);
    }
    if &record.lease.session != session {
        return Err(TaskLeaseRegistryError::WrongSession);
    }
    if record.lease.is_expired_at(now) {
        return Err(TaskLeaseRegistryError::Expired);
    }
    Ok(())
}

fn validate_process_authorities(record: &LeaseRecord) -> Result<(), TaskLeaseRegistryError> {
    for authority in record.process_authorities.values() {
        authority.verify().map_err(|error| match error {
            ProcessAuthorityError::ExecutablePathChanged
            | ProcessAuthorityError::ExecutableIdentityChanged => {
                TaskLeaseRegistryError::ProcessIdentityChanged
            }
            ProcessAuthorityError::InvalidExecutablePath
            | ProcessAuthorityError::NonUtf8ExecutablePath
            | ProcessAuthorityError::IdentityRead(_)
            | ProcessAuthorityError::Io(_) => TaskLeaseRegistryError::ProcessIdentityUnavailable,
        })?;
    }
    Ok(())
}

fn capture_process_authorities(
    lease: &TaskLease,
) -> Result<HashMap<String, ProcessAuthority>, TaskLeaseRegistryError> {
    let mut authorities = HashMap::new();
    for scope in &lease.scopes {
        let LeaseScope::ProcessExecutable { executable, class } = scope else {
            continue;
        };
        if !Path::new(executable).is_absolute() {
            continue;
        }
        let authority = ProcessAuthority::capture(lease.id.clone(), executable, *class)
            .map_err(|_| TaskLeaseRegistryError::ProcessIdentityUnavailable)?;
        authorities.insert(authority.canonical_path().to_owned(), authority);
    }
    Ok(authorities)
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskLeaseRenewalError {
    #[error("task lease renewal state is unavailable")]
    StateUnavailable,
    #[error("task lease renewal expiry must be later than now")]
    ExpiryNotFuture,
    #[error("expired task leases cannot be renewed")]
    ExpiredLease,
    #[error("task lease policy epoch does not match the active session")]
    PolicyEpochMismatch,
    #[error("task lease process authority is no longer valid")]
    AuthorityInvalid,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum TaskLeaseRegistryError {
    #[error("task lease registry state is unavailable")]
    StateUnavailable,
    #[error("task lease is already registered")]
    AlreadyRegistered,
    #[error("task lease registry reached its hard global capacity")]
    CapacityExceeded,
    #[error("session reached its hard task lease capacity")]
    SessionCapacityExceeded,
    #[error("task lease is unknown")]
    UnknownLease,
    #[error("task lease has been revoked")]
    Revoked,
    #[error("task lease belongs to another session")]
    WrongSession,
    #[error("task lease has expired")]
    Expired,
    #[error("process executable identity changed since authorization")]
    ProcessIdentityChanged,
    #[error("process executable identity is unavailable for this lease")]
    ProcessIdentityUnavailable,
    #[error("active task lease cannot be physically removed")]
    LeaseStillActive,
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, env, fs, path::PathBuf};

    use optic_bridge_core::{
        Capability, LeaseScope, ProcessExecutionClass, ResourceBudget, SessionHandle, TaskLease,
        TaskLeaseId,
    };

    use super::*;

    fn lease(session: SessionHandle, expires_at: u64) -> TaskLease {
        TaskLease {
            id: TaskLeaseId::generate().expect("test entropy"),
            session,
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([LeaseScope::ProcessExecutable {
                executable: "test".to_owned(),
                class: ProcessExecutionClass::FixedTool,
            }]),
            resource_ceiling: ResourceBudget {
                timeout_ms: 1000,
                output_bytes: 1024,
                memory_bytes: 1024,
                process_count: 1,
            },
            workload_class: optic_bridge_core::WorkloadClass::Standard,
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    fn absolute_process_lease(
        session: SessionHandle,
        executable: &Path,
        expires_at: u64,
    ) -> TaskLease {
        TaskLease {
            id: TaskLeaseId::generate().expect("test entropy"),
            session,
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([LeaseScope::ProcessExecutable {
                executable: executable.to_string_lossy().into_owned(),
                class: ProcessExecutionClass::FixedTool,
            }]),
            resource_ceiling: ResourceBudget {
                timeout_ms: 1000,
                output_bytes: 1024,
                memory_bytes: 1024,
                process_count: 1,
            },
            workload_class: optic_bridge_core::WorkloadClass::Standard,
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    fn workspace(label: &str) -> PathBuf {
        let root = env::temp_dir().join(format!("optic-task-lease-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    #[test]
    fn clocked_lease_watch_requires_original_registration_origin() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().unwrap();
        let clock = crate::StdClock::new();
        let foreign = crate::StdClock::new();
        let issued = registry
            .register_on_clock(lease(session.clone(), 1), 10_000, &clock)
            .unwrap();
        let (_, original) = registry
            .get_active_with_revocation_watch(&issued.id, &session, crate::Clock::now(&clock))
            .unwrap();
        assert!(original.belongs_to_clock(&clock));
        assert!(original.belongs_to_clock(&clock.clone()));
        assert!(!original.belongs_to_clock(&foreign));
        assert!(matches!(
            registry.register_on_clock(issued.clone(), 10_000, &foreign),
            Err(TaskLeaseClockError::Registry(
                TaskLeaseRegistryError::AlreadyRegistered
            ))
        ));
        registry.revoke(&issued.id).unwrap();
        registry.remove_revoked(&issued.id).unwrap();
        registry.register(issued.clone()).unwrap();
        let (_, raw) = registry
            .get_active_with_revocation_watch(&issued.id, &session, crate::Clock::now(&clock))
            .unwrap();
        assert!(!raw.belongs_to_clock(&clock));
        assert!(original.is_revoked());
    }

    #[test]
    fn clocked_lease_retains_old_horizon_and_origin_after_untagged_renewal() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().unwrap();
        let clock = crate::StdClock::new();
        let issued = registry
            .register_on_clock(lease(session.clone(), 1), 10_000, &clock)
            .unwrap();
        let (original, watch) = registry
            .get_active_with_revocation_watch(&issued.id, &session, crate::Clock::now(&clock))
            .unwrap();
        let renewed = issued.expires_at.saturating_add_millis(1_000);
        registry
            .renew_active_session(&session, crate::Clock::now(&clock), 1, renewed)
            .unwrap();
        let (current, current_watch) = registry
            .get_active_with_revocation_watch(&issued.id, &session, crate::Clock::now(&clock))
            .unwrap();
        assert_eq!(original.expires_at, issued.expires_at);
        assert_eq!(current.expires_at, renewed);
        assert!(watch.belongs_to_clock(&clock));
        assert!(!current_watch.belongs_to_clock(&clock));
        registry.revoke(&issued.id).unwrap();
        registry.remove_revoked(&issued.id).unwrap();
        let foreign = crate::StdClock::new();
        registry
            .register_on_clock(issued.clone(), 10_000, &foreign)
            .unwrap();
        assert!(watch.is_revoked());
        let (_, replacement) = registry
            .get_active_with_revocation_watch(&issued.id, &session, crate::Clock::now(&foreign))
            .unwrap();
        assert!(replacement.belongs_to_clock(&foreign));
        assert!(!replacement.belongs_to_clock(&clock));
    }

    #[test]
    fn clocked_lease_rejects_invalid_deadline_expiry_and_revocation() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().unwrap();
        let clock = crate::StdClock::new();
        assert!(matches!(
            registry.register_on_clock(lease(session.clone(), 1), 0, &clock),
            Err(TaskLeaseClockError::InvalidDeadline)
        ));
        let issued = registry
            .register_on_clock(lease(session.clone(), 1), 1, &clock)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(matches!(
            registry.register_on_clock(lease(session.clone(), 1), u64::MAX, &clock),
            Err(TaskLeaseClockError::InvalidDeadline)
        ));
        assert!(matches!(
            registry.get_active_with_revocation_watch(
                &issued.id,
                &session,
                crate::Clock::now(&clock)
            ),
            Err(TaskLeaseRegistryError::Expired)
        ));
        registry.revoke(&issued.id).unwrap();
        assert!(matches!(
            registry.get_active_with_revocation_watch(
                &issued.id,
                &session,
                crate::Clock::now(&clock)
            ),
            Err(TaskLeaseRegistryError::Revoked)
        ));
    }

    #[test]
    fn verified_lease_watch_binds_original_registry_record() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session");
        let original = lease(session.clone(), 100);
        registry
            .register(original.clone())
            .expect("register original");
        let (observed, watch) = registry
            .get_active_with_revocation_watch(&original.id, &session, MonotonicTime::from_millis(1))
            .expect("verified record and watch together");
        assert_eq!(observed.id, original.id);
        assert_eq!(observed.policy_epoch, original.policy_epoch);
        assert!(!watch.is_revoked(), "verified watch begins active");

        assert!(registry.revoke(&original.id).expect("publish revocation"));
        assert!(
            watch.is_revoked(),
            "watch must observe same record revocation"
        );
        assert!(
            registry
                .remove_revoked(&original.id)
                .expect("remove original")
        );

        // Reusing even the same opaque lease ID cannot retarget an old watch.
        let mut replacement = lease(session.clone(), 200);
        replacement.id = original.id.clone();
        registry
            .register(replacement.clone())
            .expect("replacement record");
        let (current, new_watch) = registry
            .get_active_with_revocation_watch(
                &replacement.id,
                &session,
                MonotonicTime::from_millis(2),
            )
            .expect("new verified record");
        assert_eq!(current.expires_at, replacement.expires_at);
        assert!(watch.is_revoked(), "old watch must not observe replacement");
        assert!(!new_watch.is_revoked(), "new record has own revocation");
    }

    #[test]
    fn verified_lease_watch_rejects_untrusted_or_inactive_record() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner");
        let stranger = SessionHandle::generate().expect("other session");
        let owned = lease(owner.clone(), 10);
        registry.register(owned.clone()).expect("register original");

        assert!(matches!(
            registry.get_active_with_revocation_watch(
                &owned.id,
                &stranger,
                MonotonicTime::from_millis(1),
            ),
            Err(TaskLeaseRegistryError::WrongSession)
        ));
        assert!(matches!(
            registry.get_active_with_revocation_watch(
                &owned.id,
                &owner,
                MonotonicTime::from_millis(10),
            ),
            Err(TaskLeaseRegistryError::Expired)
        ));
        assert_eq!(registry.revoke_session(&owner).expect("publish revoke"), 1);
        assert!(matches!(
            registry.get_active_with_revocation_watch(
                &owned.id,
                &owner,
                MonotonicTime::from_millis(1),
            ),
            Err(TaskLeaseRegistryError::Revoked)
        ));
    }

    #[cfg(not(windows))]
    #[test]
    fn verified_lease_watch_rejects_changed_executable_before_issuing_witness() {
        let root = workspace("watch-identity-drift");
        let executable = root.join("tool.bin");
        fs::write(&executable, b"original-binary").expect("fixture");
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("session");
        let owned = absolute_process_lease(owner.clone(), &executable, 100);
        registry
            .register(owned.clone())
            .expect("pin executable identity");
        fs::write(&executable, b"changed-binary").expect("modify fixture");
        assert!(matches!(
            registry.get_active_with_revocation_watch(
                &owned.id,
                &owner,
                MonotonicTime::from_millis(1),
            ),
            Err(TaskLeaseRegistryError::ProcessIdentityChanged)
        ));
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn active_process_lease_preserves_execution_class() {
        let root = workspace("class");
        let executable = root.join("tool.bin");
        fs::write(&executable, b"classified-tool").expect("write fixture");
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session");
        let lease = absolute_process_lease(session.clone(), &executable, 100);
        let id = lease.id.clone();
        registry.register(lease).expect("register lease");

        let active = registry
            .get_active(&id, &session, MonotonicTime::from_millis(1))
            .expect("active lease");
        assert_eq!(
            active.process_execution_class(&executable.to_string_lossy()),
            Some(ProcessExecutionClass::FixedTool)
        );
        drop(registry);
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn registration_is_bounded_globally_and_per_session() {
        let limits = HardLimits {
            max_task_leases: 3,
            max_task_leases_per_session: 2,
            ..HardLimits::default()
        };
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("valid limits");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");

        registry
            .register(lease(session_a.clone(), 100))
            .expect("A1");
        registry
            .register(lease(session_a.clone(), 100))
            .expect("A2");
        assert_eq!(
            registry
                .register(lease(session_a, 100))
                .expect_err("A must hit its per-session ceiling"),
            TaskLeaseRegistryError::SessionCapacityExceeded
        );

        registry
            .register(lease(session_b.clone(), 100))
            .expect("B1");
        assert_eq!(
            registry
                .register(lease(session_b, 100))
                .expect_err("global ceiling must fail closed"),
            TaskLeaseRegistryError::CapacityExceeded
        );
    }

    #[test]
    fn denied_registration_does_not_observe_an_unavailable_executable() {
        let root = workspace("preflight");
        let absent_executable = root.join("missing-tool.bin");
        let owner = SessionHandle::generate().expect("owner");
        let foreign = SessionHandle::generate().expect("foreign");
        let limits = HardLimits {
            max_task_leases: 2,
            max_task_leases_per_session: 1,
            ..HardLimits::default()
        };
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("limits");
        let first = lease(owner.clone(), 100);
        registry.register(first.clone()).expect("first lease");

        let mut duplicate = absolute_process_lease(owner.clone(), &absent_executable, 100);
        duplicate.id = first.id.clone();
        assert_eq!(
            registry
                .register(duplicate)
                .expect_err("duplicate without file IO"),
            TaskLeaseRegistryError::AlreadyRegistered
        );

        let exhausted_session = absolute_process_lease(owner, &absent_executable, 100);
        assert_eq!(
            registry
                .register(exhausted_session)
                .expect_err("session capacity before file IO"),
            TaskLeaseRegistryError::SessionCapacityExceeded
        );

        registry
            .register(lease(foreign.clone(), 100))
            .expect("second lease");
        let exhausted_global = absolute_process_lease(foreign, &absent_executable, 100);
        assert_eq!(
            registry
                .register(exhausted_global)
                .expect_err("global capacity before file IO"),
            TaskLeaseRegistryError::CapacityExceeded
        );

        assert_eq!(
            registry
                .register(absolute_process_lease(
                    SessionHandle::generate().expect("unoccupied session"),
                    &absent_executable,
                    100,
                ))
                .expect_err("global capacity still applies"),
            TaskLeaseRegistryError::CapacityExceeded
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn concurrent_registration_cannot_bypass_final_capacity_check() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        const WORKERS: usize = 8;
        let limits = HardLimits {
            max_task_leases: 2,
            max_task_leases_per_session: 2,
            ..HardLimits::default()
        };
        let registry = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("limits"));
        let start = Arc::new(Barrier::new(WORKERS));
        let mut workers = Vec::new();
        for _ in 0..WORKERS {
            let registry = Arc::clone(&registry);
            let start = Arc::clone(&start);
            workers.push(thread::spawn(move || {
                let owned = lease(SessionHandle::generate().expect("session"), 100);
                start.wait();
                registry.register(owned)
            }));
        }

        let mut accepted = 0;
        for worker in workers {
            match worker.join().expect("worker thread") {
                Ok(()) => accepted += 1,
                Err(TaskLeaseRegistryError::CapacityExceeded) => {}
                Err(other) => panic!("unexpected admission error: {other}"),
            }
        }
        assert_eq!(accepted, 2, "at most the configured global limit may enter");
        assert_eq!(
            registry.leases.lock().expect("registry state").len(),
            2,
            "failed racing admissions must not modify the registry"
        );
    }

    #[test]
    fn relative_fixture_scope_remains_registry_compatible() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner session");
        let owned = lease(owner.clone(), 100);
        registry
            .register(owned.clone())
            .expect("register fixture lease");
        registry
            .get_active(&owned.id, &owner, MonotonicTime::from_millis(1))
            .expect("relative fixture scope remains active");
    }

    #[cfg(not(windows))]
    #[test]
    fn absolute_process_scope_detects_content_drift() {
        let root = workspace("identity-drift");
        let executable = root.join("tool.bin");
        fs::write(&executable, b"first-tool").expect("write fixture");
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner session");
        let owned = absolute_process_lease(owner.clone(), &executable, 100);
        registry.register(owned.clone()).expect("register lease");
        fs::write(&executable, b"second-tool").expect("replace fixture");
        assert_eq!(
            registry
                .get_active(&owned.id, &owner, MonotonicTime::from_millis(1))
                .expect_err("identity drift must fail"),
            TaskLeaseRegistryError::ProcessIdentityChanged
        );
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[cfg(windows)]
    #[test]
    fn absolute_process_scope_stays_pinned_until_physical_removal() {
        let root = workspace("identity-pin");
        let executable = root.join("tool.exe");
        fs::write(&executable, b"pinned-tool").expect("write fixture");
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner session");
        let owned = absolute_process_lease(owner.clone(), &executable, 100);
        registry.register(owned.clone()).expect("register lease");
        registry
            .get_active(&owned.id, &owner, MonotonicTime::from_millis(1))
            .expect("pinned identity remains active");
        assert!(fs::write(&executable, b"replacement").is_err());
        assert!(registry.revoke(&owned.id).expect("revoke lease"));
        assert!(registry.remove_revoked(&owned.id).expect("remove lease"));
        fs::write(&executable, b"replacement").expect("rewrite after physical removal");
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn renewal_extends_all_active_session_leases_without_changing_authority() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner");
        let first = lease(owner.clone(), 100);
        let second = lease(owner.clone(), 150);
        registry.register(first.clone()).expect("first lease");
        registry.register(second.clone()).expect("second lease");

        let changed = registry
            .renew_active_session(
                &owner,
                MonotonicTime::from_millis(10),
                1,
                MonotonicTime::from_millis(200),
            )
            .expect("renew leases");
        assert_eq!(changed, 2);
        for original in [&first, &second] {
            let renewed = registry
                .get_active(&original.id, &owner, MonotonicTime::from_millis(160))
                .expect("renewed lease");
            assert_eq!(renewed.expires_at, MonotonicTime::from_millis(200));
            assert_eq!(renewed.capabilities, original.capabilities);
            assert_eq!(renewed.scopes, original.scopes);
            assert_eq!(renewed.resource_ceiling, original.resource_ceiling);
            assert_eq!(renewed.policy_epoch, original.policy_epoch);
        }
    }

    #[test]
    fn lease_renewal_is_all_or_nothing_when_one_active_lease_is_expired() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner");
        let expired = lease(owner.clone(), 10);
        let active = lease(owner.clone(), 100);
        registry.register(expired.clone()).expect("expired lease");
        registry.register(active.clone()).expect("active lease");
        assert_eq!(
            registry
                .renew_active_session(
                    &owner,
                    MonotonicTime::from_millis(10),
                    1,
                    MonotonicTime::from_millis(200),
                )
                .expect_err("expired member must fail whole renewal"),
            TaskLeaseRenewalError::ExpiredLease
        );
        let still_active = registry
            .get_active(&active.id, &owner, MonotonicTime::from_millis(50))
            .expect("active lease unchanged");
        assert_eq!(still_active.expires_at, MonotonicTime::from_millis(100));
    }

    #[test]
    fn revoked_leases_are_never_resurrected_by_session_renewal() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner");
        let revoked = lease(owner.clone(), 100);
        registry.register(revoked.clone()).expect("lease");
        registry.revoke(&revoked.id).expect("revoke");
        assert_eq!(
            registry
                .renew_active_session(
                    &owner,
                    MonotonicTime::from_millis(10),
                    1,
                    MonotonicTime::from_millis(200),
                )
                .expect("renew remaining active leases"),
            0
        );
        assert_eq!(
            registry
                .get_active(&revoked.id, &owner, MonotonicTime::from_millis(20))
                .expect_err("revoked stays revoked"),
            TaskLeaseRegistryError::Revoked
        );
    }

    #[test]
    fn revoked_leases_hold_capacity_until_owner_scoped_removal() {
        let limits = HardLimits {
            max_task_leases: 1,
            max_task_leases_per_session: 1,
            ..HardLimits::default()
        };
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("valid limits");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");
        let owned = lease(session_a.clone(), 100);
        registry.register(owned.clone()).expect("A lease");
        assert!(registry.revoke(&owned.id).expect("revoke A lease"));
        assert_eq!(
            registry
                .register(lease(session_b.clone(), 100))
                .expect_err("revocation alone must not reclaim storage"),
            TaskLeaseRegistryError::CapacityExceeded
        );

        assert_eq!(registry.remove_session(&session_a).expect("remove A"), 1);
        let foreign = lease(session_b.clone(), 100);
        registry
            .register(foreign.clone())
            .expect("B lease after reap");
        registry
            .get_active(&foreign.id, &session_b, MonotonicTime::from_millis(1))
            .expect("B lease remains active");
    }

    #[test]
    fn cross_session_access_fails_closed() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("test entropy");
        let other = SessionHandle::generate().expect("test entropy");
        let lease = lease(owner, 100);
        registry.register(lease.clone()).expect("register lease");
        assert_eq!(
            registry
                .get_active(&lease.id, &other, MonotonicTime::from_millis(1))
                .expect_err("cross-session access must fail"),
            TaskLeaseRegistryError::WrongSession
        );
    }

    #[test]
    fn only_revoked_lease_can_be_physically_removed_individually() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner session");
        let owned = lease(owner.clone(), 100);
        registry.register(owned.clone()).expect("owned lease");
        assert_eq!(
            registry
                .remove_revoked(&owned.id)
                .expect_err("active lease removal must fail"),
            TaskLeaseRegistryError::LeaseStillActive
        );
        assert!(registry.revoke(&owned.id).expect("revoke lease"));
        assert!(registry.remove_revoked(&owned.id).expect("remove revoked"));
        assert_eq!(
            registry
                .get_active(&owned.id, &owner, MonotonicTime::from_millis(1))
                .expect_err("removed lease must be unknown"),
            TaskLeaseRegistryError::UnknownLease
        );
    }

    #[test]
    fn remove_session_only_removes_owned_leases() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner session");
        let other = SessionHandle::generate().expect("other session");
        let owned = lease(owner.clone(), 100);
        let foreign = lease(other.clone(), 100);
        registry.register(owned.clone()).expect("owned lease");
        registry.register(foreign.clone()).expect("foreign lease");

        assert_eq!(registry.remove_session(&owner).expect("remove leases"), 1);
        assert_eq!(
            registry
                .get_active(&owned.id, &owner, MonotonicTime::from_millis(1))
                .expect_err("owned lease must be removed"),
            TaskLeaseRegistryError::UnknownLease
        );
        registry
            .get_active(&foreign.id, &other, MonotonicTime::from_millis(1))
            .expect("foreign lease must remain");
    }

    #[test]
    fn removed_session_revokes_original_lease_watch_without_affecting_other_sessions() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("owner");
        let other = SessionHandle::generate().expect("other");
        let owned = lease(owner.clone(), 100);
        let foreign = lease(other.clone(), 100);
        registry.register(owned.clone()).expect("owned lease");
        registry.register(foreign.clone()).expect("foreign lease");

        let (_, old_watch) = registry
            .get_active_with_revocation_watch(&owned.id, &owner, MonotonicTime::from_millis(1))
            .expect("original owned witness");
        let (_, foreign_watch) = registry
            .get_active_with_revocation_watch(&foreign.id, &other, MonotonicTime::from_millis(1))
            .expect("foreign witness");
        assert!(!old_watch.is_revoked());
        assert!(!foreign_watch.is_revoked());

        // Exercise isolated cleanup, without an earlier revoke_session call.
        // Any still-observed old generation must stop as it is removed.
        assert_eq!(registry.remove_session(&owner).expect("remove owner"), 1);
        assert!(
            old_watch.is_revoked(),
            "physical cleanup must publish original-generation stop"
        );
        assert!(
            !foreign_watch.is_revoked(),
            "other session stays authorized"
        );

        let mut replacement = lease(owner.clone(), 100);
        replacement.id = owned.id.clone();
        registry.register(replacement).expect("reuse opaque ID");
        let (_, replacement_watch) = registry
            .get_active_with_revocation_watch(&owned.id, &owner, MonotonicTime::from_millis(1))
            .expect("new generation");
        assert!(
            old_watch.is_revoked(),
            "old watch may never recover authority"
        );
        assert!(!replacement_watch.is_revoked());
    }

    #[test]
    fn revoke_session_invalidates_its_leases() {
        let registry = TaskLeaseRegistry::new();
        let owner = SessionHandle::generate().expect("test entropy");
        let lease = lease(owner.clone(), 100);
        registry.register(lease.clone()).expect("register lease");
        assert_eq!(registry.revoke_session(&owner).expect("revoke session"), 1);
        assert_eq!(
            registry
                .get_active(&lease.id, &owner, MonotonicTime::from_millis(1))
                .expect_err("revoked lease must fail"),
            TaskLeaseRegistryError::Revoked
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_original_lease_receipt_survives_id_reuse_and_foreign_registry() {
        let registry = TaskLeaseRegistry::new();
        let foreign = TaskLeaseRegistry::new();
        let grant = lease(SessionHandle::generate().expect("session"), 100);
        registry
            .register(grant.clone())
            .expect("original generation");
        foreign.register(grant.clone()).expect("separate registry");
        let now = MonotonicTime::from_millis(1);
        let (_, original_watch) = registry
            .get_active_with_revocation_watch(&grant.id, &grant.session, now)
            .expect("original witness");
        let original_commit = registry
            .commit_original_native_read_lease(
                &grant.id,
                &original_watch,
                grant.expires_at,
                &grant.session,
                now,
            )
            .expect("commit original lease under the revoke mutex");
        assert!(!original_commit.is_stop_required_at(now));

        assert!(matches!(
            foreign.commit_original_native_read_lease(
                &grant.id,
                &original_watch,
                grant.expires_at,
                &grant.session,
                now,
            ),
            Err(TaskLeaseRegistryError::Revoked)
        ));
        assert!(registry.revoke(&grant.id).expect("revoke original"));
        assert!(original_commit.is_stop_required_at(now));
        assert!(registry.remove_revoked(&grant.id).expect("remove original"));
        registry.register(grant.clone()).expect("reuse opaque ID");
        let (_, replacement_watch) = registry
            .get_active_with_revocation_watch(&grant.id, &grant.session, now)
            .expect("new generation");
        assert!(!replacement_watch.is_revoked());
        assert!(matches!(
            registry.commit_original_native_read_lease(
                &grant.id,
                &original_watch,
                grant.expires_at,
                &grant.session,
                now,
            ),
            Err(TaskLeaseRegistryError::Revoked)
        ));
        assert!(
            original_commit.is_stop_required_at(now),
            "old generation stays stopped even when same opaque ID is active"
        );
        assert!(
            registry
                .commit_original_native_read_lease(
                    &grant.id,
                    &replacement_watch,
                    grant.expires_at,
                    &grant.session,
                    now,
                )
                .is_ok()
        );
    }

    #[cfg(windows)]
    #[test]
    fn original_lease_gate_rejects_all_revoke_forms_and_renewed_deadline() {
        for remove_session in [false, true] {
            let registry = TaskLeaseRegistry::new();
            let grant = lease(SessionHandle::generate().expect("session"), 100);
            registry.register(grant.clone()).unwrap();
            let now = MonotonicTime::from_millis(1);
            let (_, witness) = registry
                .get_active_with_revocation_watch(&grant.id, &grant.session, now)
                .unwrap();
            let original = registry
                .commit_original_native_read_lease(
                    &grant.id,
                    &witness,
                    grant.expires_at,
                    &grant.session,
                    now,
                )
                .unwrap();
            if remove_session {
                assert_eq!(registry.remove_session(&grant.session).unwrap(), 1);
                assert!(matches!(
                    registry.commit_original_native_read_lease(
                        &grant.id,
                        &witness,
                        grant.expires_at,
                        &grant.session,
                        now,
                    ),
                    Err(TaskLeaseRegistryError::UnknownLease)
                ));
            } else {
                assert_eq!(registry.revoke_session(&grant.session).unwrap(), 1);
                assert!(matches!(
                    registry.commit_original_native_read_lease(
                        &grant.id,
                        &witness,
                        grant.expires_at,
                        &grant.session,
                        now,
                    ),
                    Err(TaskLeaseRegistryError::Revoked)
                ));
            }
            assert!(original.is_stop_required_at(now));
        }

        let registry = TaskLeaseRegistry::new();
        let grant = lease(SessionHandle::generate().expect("session"), 100);
        registry.register(grant.clone()).unwrap();
        let now = MonotonicTime::from_millis(1);
        let (_, witness) = registry
            .get_active_with_revocation_watch(&grant.id, &grant.session, now)
            .unwrap();
        registry
            .renew_active_session(
                &grant.session,
                now,
                grant.policy_epoch,
                MonotonicTime::from_millis(200),
            )
            .expect("active session lease can be renewed");
        assert!(matches!(
            registry.commit_original_native_read_lease(
                &grant.id,
                &witness,
                grant.expires_at,
                &grant.session,
                MonotonicTime::from_millis(100),
            ),
            Err(TaskLeaseRegistryError::Expired)
        ));
        assert!(
            registry
                .commit_original_native_read_lease(
                    &grant.id,
                    &witness,
                    grant.expires_at,
                    &grant.session,
                    MonotonicTime::from_millis(99),
                )
                .is_ok()
        );
    }
}
