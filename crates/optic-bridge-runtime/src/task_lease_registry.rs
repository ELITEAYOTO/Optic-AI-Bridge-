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
    lease: TaskLease,
    revoked: bool,
    revocation_signal: Arc<AtomicBool>,
    process_authorities: HashMap<String, ProcessAuthority>,
}

/// Read-only observation bound to one concrete, verified TaskLease record.
/// This is cancellation evidence, not approval to create or resume a process.
/// Removal and even reuse of the opaque lease ID cannot retarget this signal.
#[derive(Clone, Debug)]
pub(crate) struct TaskLeaseRevocationWatch {
    signal: Arc<AtomicBool>,
}

impl TaskLeaseRevocationWatch {
    // The future native owner will consume this generation-bound stop watch.
    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn is_revoked(&self) -> bool {
        self.signal.load(Ordering::Acquire)
    }

    // Native preparation needs the original watched atomic as a stop source.
    // This is not permission to start an unowned process.
    #[cfg(windows)]
    pub(crate) fn signal(&self) -> &Arc<AtomicBool> {
        &self.signal
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
        // Reject known duplicates and exhausted capacity before opening and
        // hashing an executable. Never hold the registry lock during file IO.
        {
            let leases = self
                .leases
                .lock()
                .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
            self.validate_registration(&leases, &lease)?;
        }

        let process_authorities = capture_process_authorities(&lease)?;

        let mut leases = self
            .leases
            .lock()
            .map_err(|_| TaskLeaseRegistryError::StateUnavailable)?;
        // Concurrent registrations/removals may have changed the result since
        // preflight. Only this locked second check authorizes insertion.
        self.validate_registration(&leases, &lease)?;
        leases.insert(
            lease.id.clone(),
            LeaseRecord {
                lease,
                revoked: false,
                revocation_signal: Arc::new(AtomicBool::new(false)),
                process_authorities,
            },
        );
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
        leases.retain(|_, record| &record.lease.session != session);
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
}
