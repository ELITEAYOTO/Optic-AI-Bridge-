use std::{
    collections::HashMap,
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};

use optic_bridge_core::{HardLimits, LimitError, MonotonicTime, SessionGrant, SessionHandle};
#[cfg(windows)]
use optic_bridge_core::JobId;
use thiserror::Error;

#[cfg(test)]
#[path = "registry_revoke_tests.rs"]
mod revoke_phase_tests;

#[derive(Debug)]
pub struct SessionRegistry {
    sessions: Mutex<HashMap<SessionHandle, SessionRecord>>,
    admissions_drained: Condvar,
    max_sessions: u32,
    max_renewal_horizon_ms: u64,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            admissions_drained: Condvar::new(),
            max_sessions: HardLimits::default().max_sessions,
            max_renewal_horizon_ms: HardLimits::default().max_session_renewal_horizon_ms,
        }
    }
}

#[derive(Clone, Debug)]
struct SessionRecord {
    grant: SessionGrant,
    revoked: bool,
    revocation_signal: Arc<AtomicBool>,
    active_admissions: u32,
}

impl SessionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_hard_limits(limits: HardLimits) -> Result<Self, LimitError> {
        let limits = limits.validate_nonzero()?;
        Ok(Self {
            sessions: Mutex::new(HashMap::new()),
            admissions_drained: Condvar::new(),
            max_sessions: limits.max_sessions,
            max_renewal_horizon_ms: limits.max_session_renewal_horizon_ms,
        })
    }

    pub fn register(&self, grant: SessionGrant) -> Result<(), SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        if sessions.contains_key(&grant.handle) {
            return Err(SessionRegistryError::AlreadyRegistered);
        }
        let max_sessions = usize::try_from(self.max_sessions)
            .map_err(|_| SessionRegistryError::CapacityExceeded)?;
        if sessions.len() >= max_sessions {
            return Err(SessionRegistryError::CapacityExceeded);
        }
        sessions.insert(
            grant.handle.clone(),
            SessionRecord {
                grant,
                revoked: false,
                revocation_signal: Arc::new(AtomicBool::new(false)),
                active_admissions: 0,
            },
        );
        Ok(())
    }

    pub fn get_active(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<SessionGrant, SessionRegistryError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let record = sessions
            .get(handle)
            .ok_or(SessionRegistryError::UnknownSession)?;
        if record.revoked {
            return Err(SessionRegistryError::Revoked);
        }
        if record.grant.is_expired_at(now) {
            return Err(SessionRegistryError::Expired);
        }
        Ok(record.grant.clone())
    }

    pub fn begin_admission(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<SessionAdmissionPermit<'_>, SessionRegistryError> {
        let (grant, revocation_watch) = self.admit_record(handle, now)?;
        Ok(SessionAdmissionPermit {
            registry: self,
            handle: handle.clone(),
            grant,
            revocation_watch,
        })
    }

    /// A counted admission whose lifetime is independent of the borrowing
    /// caller. Future service-owned workers can retain it across threads.
    ///
    /// It has exactly the same admission and revoke/drain semantics as
    /// `begin_admission`; this is not a new permission to execute an effect.
    /// Its owner must retain it until native quiescence has been *proved*.
    pub fn begin_owned_admission(
        self: &Arc<Self>,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<OwnedSessionAdmissionPermit, SessionRegistryError> {
        let (grant, revocation_watch) = self.admit_record(handle, now)?;
        Ok(OwnedSessionAdmissionPermit {
            registry: Arc::clone(self),
            handle: handle.clone(),
            grant,
            revocation_watch,
        })
    }

    fn admit_record(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<(SessionGrant, SessionRevocationWatch), SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let record = sessions
            .get_mut(handle)
            .ok_or(SessionRegistryError::UnknownSession)?;
        if record.revoked {
            return Err(SessionRegistryError::Revoked);
        }
        if record.grant.is_expired_at(now) {
            return Err(SessionRegistryError::Expired);
        }
        record.active_admissions = record
            .active_admissions
            .checked_add(1)
            .ok_or(SessionRegistryError::AdmissionCounterOverflow)?;
        let grant = record.grant.clone();
        // Both permit forms capture the same immutable record identity while
        // their admission is counted under the shared registry mutex.
        let revocation_watch = SessionRevocationWatch {
            signal: Arc::clone(&record.revocation_signal),
        };
        Ok((grant, revocation_watch))
    }

    /// Read the exact owner-held session generation under the same mutex as
    /// direct `revoke` and lifecycle `publish_revocation_before_drain`.
    ///
    /// This is an *inert* observation: the guard is released on return. An
    /// affirmative result is NOT a commit ticket or CreateProcess permission.
    /// TaskLease revocation, policy epochs and native stop remain independent.
    #[cfg(windows)]
    pub(crate) fn observe_original_owned_admission_under_close_lock(
        &self,
        admitted: &OwnedSessionAdmissionPermit,
        now: MonotonicTime,
    ) -> Result<(), SessionRegistryError> {
        if !std::ptr::eq(self, admitted.registry.as_ref()) {
            return Err(SessionRegistryError::ForeignAdmissionRegistry);
        }
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let record = sessions
            .get(&admitted.handle)
            .ok_or(SessionRegistryError::UnknownSession)?;
        Self::check_original_native_owner_record(record, admitted, now)
    }

    /// Commit *only the session authority boundary* for one already registered,
    /// precreated native owner. Direct and lifecycle close serialize against
    /// this short critical section; no Win32 call or JobStore lock is taken.
    ///
    /// The receipt remains bound to the original session generation and its
    /// real private Job ID. It is NOT a native launch permission: independent
    /// task lease, policy epoch and target fences must still be engineered.
    #[cfg(windows)]
    pub(crate) fn commit_registered_native_owner_session(
        &self,
        admitted: &OwnedSessionAdmissionPermit,
        registered_job: JobId,
        now: MonotonicTime,
    ) -> Result<SessionNativeOwnerCommit, SessionRegistryError> {
        if !std::ptr::eq(self, admitted.registry.as_ref()) {
            return Err(SessionRegistryError::ForeignAdmissionRegistry);
        }
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let record = sessions
            .get(&admitted.handle)
            .ok_or(SessionRegistryError::UnknownSession)?;
        Self::check_original_native_owner_record(record, admitted, now)?;
        // Receipt publication is ordered with the irreversible session close.
        // Neither the original admission nor the Job is dropped here.
        Ok(SessionNativeOwnerCommit {
            registered_job,
            original_stop: admitted.stop_watch(),
        })
    }

    #[cfg(windows)]
    fn check_original_native_owner_record(
        record: &SessionRecord,
        admitted: &OwnedSessionAdmissionPermit,
        now: MonotonicTime,
    ) -> Result<(), SessionRegistryError> {
        if !Arc::ptr_eq(&record.revocation_signal, &admitted.revocation_watch.signal) {
            return Err(SessionRegistryError::AdmissionGenerationMismatch);
        }
        if record.revoked {
            return Err(SessionRegistryError::Revoked);
        }
        // An admission-time expiry must not grow after a later renewal.
        if record.grant.is_expired_at(now) || admitted.grant.is_expired_at(now) {
            return Err(SessionRegistryError::Expired);
        }
        Ok(())
    }

    pub(crate) fn validate_renewal(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
        new_expires_at: MonotonicTime,
    ) -> Result<SessionGrant, SessionRenewalError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRenewalError::StateUnavailable)?;
        let record = sessions
            .get(handle)
            .ok_or(SessionRenewalError::UnknownSession)?;
        validate_renewal_record(record, now, new_expires_at, self.max_renewal_horizon_ms)?;
        Ok(record.grant.clone())
    }

    pub(crate) fn renew_active(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
        new_expires_at: MonotonicTime,
    ) -> Result<SessionGrant, SessionRenewalError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRenewalError::StateUnavailable)?;
        let record = sessions
            .get_mut(handle)
            .ok_or(SessionRenewalError::UnknownSession)?;
        validate_renewal_record(record, now, new_expires_at, self.max_renewal_horizon_ms)?;
        record.grant.expires_at = new_expires_at;
        Ok(record.grant.clone())
    }

    /// Publish the irreversible revoked bit and notify already-admitted owners,
    /// but do not wait for those owners here. The session lifecycle uses this
    /// first to request owned process cancellation before admission drainage.
    /// Authority/grant cleanup remains forbidden until the regular revoke
    /// barrier has finished. This does not grant a new admission capability.
    pub(crate) fn publish_revocation_before_drain(
        &self,
        handle: &SessionHandle,
    ) -> Result<bool, SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let Some(record) = sessions.get_mut(handle) else {
            return Ok(false);
        };
        Ok(Self::publish_revocation(record))
    }

    pub fn revoke(&self, handle: &SessionHandle) -> Result<bool, SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let Some(record) = sessions.get_mut(handle) else {
            return Ok(false);
        };
        let changed = Self::publish_revocation(record);
        // Keep the same guard across publication and the first predicate check.
        // The drain returns it so it remains held until this method returns.
        let _sessions = self.wait_for_admission_drain(handle, sessions)?;
        Ok(changed)
    }

    fn publish_revocation(record: &mut SessionRecord) -> bool {
        let changed = !record.revoked;
        record.revoked = true;
        record.revocation_signal.store(true, Ordering::Release);
        changed
    }

    fn wait_for_admission_drain<'a>(
        &self,
        handle: &SessionHandle,
        mut sessions: MutexGuard<'a, HashMap<SessionHandle, SessionRecord>>,
    ) -> Result<MutexGuard<'a, HashMap<SessionHandle, SessionRecord>>, SessionRegistryError> {
        loop {
            let active_admissions = sessions
                .get(handle)
                .ok_or(SessionRegistryError::UnknownSession)?
                .active_admissions;
            if active_admissions == 0 {
                return Ok(sessions);
            }
            sessions = self
                .admissions_drained
                .wait(sessions)
                .map_err(|_| SessionRegistryError::StateUnavailable)?;
        }
    }

    #[cfg(windows)]
    pub(crate) fn watch_active(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<(Arc<AtomicBool>, SessionGrant), SessionRegistryError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let record = sessions
            .get(handle)
            .ok_or(SessionRegistryError::UnknownSession)?;
        if record.revoked {
            return Err(SessionRegistryError::Revoked);
        }
        if record.grant.is_expired_at(now) {
            return Err(SessionRegistryError::Expired);
        }
        Ok((Arc::clone(&record.revocation_signal), record.grant.clone()))
    }

    pub(crate) fn inactive_handles(
        &self,
        now: MonotonicTime,
    ) -> Result<Vec<SessionHandle>, SessionRegistryError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        Ok(sessions
            .iter()
            .filter(|(_, record)| record.revoked || record.grant.is_expired_at(now))
            .map(|(handle, _)| handle.clone())
            .collect())
    }

    pub(crate) fn remove_quiescent_inactive(
        &self,
        handle: &SessionHandle,
        now: MonotonicTime,
    ) -> Result<bool, SessionRegistryError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        let Some(record) = sessions.get(handle) else {
            return Ok(false);
        };
        if !record.revoked && !record.grant.is_expired_at(now) {
            return Err(SessionRegistryError::StillActive);
        }
        if record.active_admissions != 0 {
            return Err(SessionRegistryError::AdmissionsInFlight);
        }
        sessions.remove(handle);
        Ok(true)
    }

    fn finish_admission(&self, handle: &SessionHandle) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(record) = sessions.get_mut(handle) else {
            return;
        };
        if record.active_admissions == 0 {
            debug_assert!(false, "session admission counter underflow");
            return;
        }
        record.active_admissions -= 1;
        if record.active_admissions == 0 {
            self.admissions_drained.notify_all();
        }
    }

    pub fn len(&self) -> Result<usize, SessionRegistryError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionRegistryError::StateUnavailable)?;
        Ok(sessions.len())
    }

    pub fn is_empty(&self) -> Result<bool, SessionRegistryError> {
        Ok(self.len()? == 0)
    }
}

fn validate_renewal_record(
    record: &SessionRecord,
    now: MonotonicTime,
    new_expires_at: MonotonicTime,
    max_renewal_horizon_ms: u64,
) -> Result<(), SessionRenewalError> {
    if new_expires_at <= now {
        return Err(SessionRenewalError::ExpiryNotFuture);
    }
    if new_expires_at > now.saturating_add_millis(max_renewal_horizon_ms) {
        return Err(SessionRenewalError::BeyondHorizon);
    }
    if record.revoked {
        return Err(SessionRenewalError::Revoked);
    }
    if record.grant.is_expired_at(now) {
        return Err(SessionRenewalError::Expired);
    }
    if new_expires_at <= record.grant.expires_at {
        return Err(SessionRenewalError::MustExtend);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionRenewalError {
    #[error("session renewal state is unavailable")]
    StateUnavailable,
    #[error("session renewal target is unknown")]
    UnknownSession,
    #[error("revoked sessions cannot be renewed")]
    Revoked,
    #[error("expired sessions cannot be renewed")]
    Expired,
    #[error("session renewal expiry must be later than now")]
    ExpiryNotFuture,
    #[error("session renewal exceeds the configured future horizon")]
    BeyondHorizon,
    #[error("session renewal must strictly extend the current expiry")]
    MustExtend,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SessionRegistryError {
    #[error("session registry state is unavailable")]
    StateUnavailable,
    #[error("session handle is already registered")]
    AlreadyRegistered,
    #[error("session registry reached its hard session capacity")]
    CapacityExceeded,
    #[error("session handle is unknown")]
    UnknownSession,
    #[error("session has been revoked")]
    Revoked,
    #[error("session has expired")]
    Expired,
    #[cfg(windows)]
    #[error("owned admission originated in another session registry")]
    ForeignAdmissionRegistry,
    #[cfg(windows)]
    #[error("owned admission generation differs from the current session record")]
    AdmissionGenerationMismatch,
    #[error("session admission counter reached its representable limit")]
    AdmissionCounterOverflow,
    #[error("session still has admitted effects in flight")]
    AdmissionsInFlight,
    #[error("session is still active and cannot be removed")]
    StillActive,
}

/// Opaque, non-cloneable proof that the session close ordering point was
/// reached for this already registered private native owner. Does not contain
/// an OS handle, grant a native launch or certify process termination.
#[cfg(windows)]
#[must_use = "the original session stop signal must outlive native ownership"]
pub(crate) struct SessionNativeOwnerCommit {
    registered_job: JobId,
    original_stop: SessionAdmissionStopWatch,
}

#[cfg(windows)]
impl SessionNativeOwnerCommit {
    pub(crate) fn job_id(&self) -> &JobId {
        &self.registered_job
    }

    pub(crate) fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.original_stop.is_stop_required_at(now)
    }
}

/// Read-only observation of *explicit* revocation for one admitted session
/// record. It does not grant permission to start work, does not extend the
/// admitted deadline, and does not prove that any process has terminated.
///
/// A watcher continues to refer to its original session generation even
/// after its permit is dropped and the opaque handle is eventually reused.
#[derive(Clone, Debug)]
pub struct SessionRevocationWatch {
    signal: Arc<AtomicBool>,
}

impl SessionRevocationWatch {
    #[must_use]
    pub fn is_revoked(&self) -> bool {
        self.signal.load(Ordering::Acquire)
    }
}

/// Read-only stop condition captured by a *genuine* session admission.
///
/// Explicit revocation is observed from the original session record. Expiry is
/// evaluated against the session deadline captured at admission time, which a
/// later renewal cannot extend for an already-admitted operation.
///
/// `false` does NOT authorize starting, resuming or publishing an effect:
/// independent task leases, policy, operation deadlines and owner-registration
/// checks are still mandatory. `true` requests that the caller stop; it is
/// never a native process-termination or resource-release receipt.
#[derive(Clone, Debug)]
pub struct SessionAdmissionStopWatch {
    revocation: SessionRevocationWatch,
    captured_expires_at: MonotonicTime,
}

impl SessionAdmissionStopWatch {
    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.revocation.is_revoked() || now >= self.captured_expires_at
    }
}

pub struct SessionAdmissionPermit<'a> {
    registry: &'a SessionRegistry,
    handle: SessionHandle,
    grant: SessionGrant,
    revocation_watch: SessionRevocationWatch,
}

impl SessionAdmissionPermit<'_> {
    #[must_use]
    pub fn grant(&self) -> &SessionGrant {
        &self.grant
    }

    /// Obtain a read-only revocation watch bound to the grant's original
    /// session record; unlike relooking up an opaque handle, it survives
    /// close/removal without accidentally observing a replacement session.
    ///
    /// `false` means only that explicit revocation has not been published.
    /// Callers must *also* enforce monotonic deadlines, task leases and
    /// effect authorization independently.
    #[must_use]
    pub fn revocation_watch(&self) -> SessionRevocationWatch {
        self.revocation_watch.clone()
    }

    /// Capture the original admission generation's revoke signal and absolute
    /// monotonic session expiry. Dropping the permit does not retarget this
    /// observation to another session record using the same opaque handle.
    ///
    /// This is cancellation observation only, not a new launch permission or
    /// a guarantee of successful cleanup.
    #[must_use]
    pub fn stop_watch(&self) -> SessionAdmissionStopWatch {
        SessionAdmissionStopWatch {
            revocation: self.revocation_watch(),
            captured_expires_at: self.grant.expires_at,
        }
    }
}

impl Drop for SessionAdmissionPermit<'_> {
    fn drop(&mut self) {
        self.registry.finish_admission(&self.handle);
    }
}

/// A genuine, non-cloneable session admission owned by an `Arc<SessionRegistry>`.
///
/// It is a lifetime/stop-observation primitive, never a launch, task-lease,
/// policy or result-publication authority. The real native owner must keep this
/// object until termination is independently proven; dropping it early can
/// let synchronous revoke finish while a process remains alive.
pub struct OwnedSessionAdmissionPermit {
    registry: Arc<SessionRegistry>,
    handle: SessionHandle,
    grant: SessionGrant,
    revocation_watch: SessionRevocationWatch,
}

impl OwnedSessionAdmissionPermit {
    #[must_use]
    pub fn grant(&self) -> &SessionGrant {
        &self.grant
    }

    /// A transferred native admission must belong to the exact server-owned
    /// registry, not another registry that reused the same opaque handle.
    #[must_use]
    pub(crate) fn belongs_to_registry(&self, registry: &Arc<SessionRegistry>) -> bool {
        Arc::ptr_eq(&self.registry, registry)
    }

    #[must_use]
    pub fn stop_watch(&self) -> SessionAdmissionStopWatch {
        SessionAdmissionStopWatch {
            revocation: self.revocation_watch.clone(),
            captured_expires_at: self.grant.expires_at,
        }
    }
}

impl Drop for OwnedSessionAdmissionPermit {
    fn drop(&mut self) {
        self.registry.finish_admission(&self.handle);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        sync::{Arc, mpsc},
        thread,
        time::Duration,
    };

    use optic_bridge_core::{Capability, PrincipalId, ProjectId};

    use super::*;

    fn grant(expires_at: u64) -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().expect("test entropy"),
            principal: PrincipalId::new("principal").expect("valid principal"),
            project: ProjectId::new("project").expect("valid project"),
            capabilities: BTreeSet::from([Capability::FileRead]),
            expires_at: MonotonicTime::from_millis(expires_at),
            policy_epoch: 1,
        }
    }

    #[test]
    fn active_session_round_trips() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        let loaded = registry
            .get_active(&grant.handle, MonotonicTime::from_millis(99))
            .expect("session should be active");
        assert_eq!(loaded.handle, grant.handle);
    }

    #[test]
    fn revoke_fails_closed_immediately() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        assert!(registry.revoke(&grant.handle).expect("revoke session"));
        assert_eq!(
            registry
                .get_active(&grant.handle, MonotonicTime::from_millis(1))
                .expect_err("revoked session must fail"),
            SessionRegistryError::Revoked
        );
    }

    #[test]
    fn expiry_is_checked_against_monotonic_time() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        assert_eq!(
            registry
                .get_active(&grant.handle, MonotonicTime::from_millis(100))
                .expect_err("expired session must fail"),
            SessionRegistryError::Expired
        );
    }

    #[test]
    fn inactive_handles_include_only_revoked_and_expired_sessions() {
        let registry = SessionRegistry::new();
        let active = grant(1_000);
        let expired = grant(10);
        let revoked = grant(1_000);
        registry.register(active.clone()).expect("active session");
        registry.register(expired.clone()).expect("expired session");
        registry.register(revoked.clone()).expect("revoked session");
        registry.revoke(&revoked.handle).expect("revoke session");

        let inactive = registry
            .inactive_handles(MonotonicTime::from_millis(10))
            .expect("inactive handles");
        assert_eq!(inactive.len(), 2);
        assert!(inactive.contains(&expired.handle));
        assert!(inactive.contains(&revoked.handle));
        assert!(!inactive.contains(&active.handle));
    }

    #[test]
    fn duplicate_handle_is_rejected() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("first insert");
        assert_eq!(
            registry.register(grant).expect_err("duplicate must fail"),
            SessionRegistryError::AlreadyRegistered
        );
    }

    #[test]
    fn revoke_closes_new_admissions_and_waits_for_existing_permit() {
        let registry = Arc::new(SessionRegistry::new());
        let grant = grant(1_000);
        registry.register(grant.clone()).expect("register session");
        let now = MonotonicTime::from_millis(1);
        #[cfg(windows)]
        let (stop_signal, _) = registry.watch_active(&grant.handle, now).unwrap();
        let permit = registry
            .begin_admission(&grant.handle, now)
            .expect("admit effect");

        let worker_registry = Arc::clone(&registry);
        let worker_handle = grant.handle.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let result = worker_registry.revoke(&worker_handle);
            done_tx.send(result).expect("send revoke result");
        });

        let mut observed_revoked = false;
        for _ in 0..100 {
            if matches!(
                registry.get_active(&grant.handle, now),
                Err(SessionRegistryError::Revoked)
            ) {
                observed_revoked = true;
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            observed_revoked,
            "revoke must close admission before waiting"
        );
        assert!(matches!(
            registry.begin_admission(&grant.handle, now),
            Err(SessionRegistryError::Revoked)
        ));
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        // Running jobs observe revocation before the synchronous admission
        // fence has drained, without taking this registry's mutex themselves.
        #[cfg(windows)]
        assert!(stop_signal.load(Ordering::Acquire));

        drop(permit);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("revoke must drain after permit drop")
                .expect("revoke result")
        );
        worker.join().expect("revoke thread");
    }

    #[test]
    fn renewal_extends_only_active_session_expiry() {
        let registry = SessionRegistry::new();
        let grant = grant(100);
        registry.register(grant.clone()).expect("register session");
        let renewed = registry
            .renew_active(
                &grant.handle,
                MonotonicTime::from_millis(10),
                MonotonicTime::from_millis(200),
            )
            .expect("renew session");
        assert_eq!(renewed.expires_at, MonotonicTime::from_millis(200));
        assert_eq!(renewed.capabilities, grant.capabilities);
        assert_eq!(renewed.policy_epoch, grant.policy_epoch);
    }

    #[test]
    fn renewal_cannot_exceed_configured_future_horizon() {
        let limits = HardLimits {
            max_session_renewal_horizon_ms: 50,
            ..HardLimits::default()
        };
        let registry = SessionRegistry::from_hard_limits(limits).expect("limits");
        let grant = grant(100);
        registry.register(grant.clone()).expect("session");
        assert_eq!(
            registry
                .renew_active(
                    &grant.handle,
                    MonotonicTime::from_millis(10),
                    MonotonicTime::from_millis(61),
                )
                .expect_err("horizon must fail"),
            SessionRenewalError::BeyondHorizon
        );
    }

    #[test]
    fn renewal_cannot_shorten_revoke_or_resurrect_expired_session() {
        let registry = SessionRegistry::new();
        let active = grant(100);
        registry.register(active.clone()).expect("active session");
        assert_eq!(
            registry
                .renew_active(
                    &active.handle,
                    MonotonicTime::from_millis(10),
                    MonotonicTime::from_millis(100),
                )
                .expect_err("equal expiry must fail"),
            SessionRenewalError::MustExtend
        );

        let revoked = grant(100);
        registry.register(revoked.clone()).expect("revoked session");
        registry.revoke(&revoked.handle).expect("revoke");
        assert_eq!(
            registry
                .renew_active(
                    &revoked.handle,
                    MonotonicTime::from_millis(10),
                    MonotonicTime::from_millis(200),
                )
                .expect_err("revoked must fail"),
            SessionRenewalError::Revoked
        );

        let expired = grant(10);
        registry.register(expired.clone()).expect("expired session");
        assert_eq!(
            registry
                .renew_active(
                    &expired.handle,
                    MonotonicTime::from_millis(10),
                    MonotonicTime::from_millis(200),
                )
                .expect_err("expired must fail"),
            SessionRenewalError::Expired
        );
    }

    #[test]
    fn registration_is_bounded_by_hard_session_capacity() {
        let limits = HardLimits {
            max_sessions: 2,
            ..HardLimits::default()
        };
        let registry = SessionRegistry::from_hard_limits(limits).expect("valid limits");
        registry.register(grant(100)).expect("first session");
        registry.register(grant(100)).expect("second session");
        assert_eq!(
            registry
                .register(grant(100))
                .expect_err("capacity must fail closed"),
            SessionRegistryError::CapacityExceeded
        );
        assert_eq!(registry.len().expect("registry length"), 2);
    }
}
