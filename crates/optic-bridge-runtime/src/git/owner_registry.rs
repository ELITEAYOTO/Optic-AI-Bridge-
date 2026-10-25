//! #317 research-only: session-owned native GitRead resource reservations.
//! No GitRead production caller yet. An owner must be adopted BEFORE native spawn.
//! This ledger is not a grant to execute commands or release uncertain children.
#![allow(dead_code)]

use crate::{
    GitReadError, GitReadService, OwnedSessionAdmissionPermit, SessionRegistry, SessionRegistryError,
    TaskLeaseRegistry, TaskLeaseRegistryError,
};
use optic_bridge_core::{
    Capability, LeaseScope, MonotonicTime, ProcessExecutionClass, SessionHandle, TaskLeaseId,
};
use optic_bridge_windows::{LimitedJobObject, PreparedReadCommand, ReadCommandOutcome};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use thiserror::Error;

#[derive(Clone, Copy)]
struct Budget {
    memory: u64,
    output: u64,
    cpu: u32,
}

#[derive(Clone, Copy)]
struct Ceiling {
    count: usize,
    memory: u64,
    output: u64,
    cpu: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct OwnerId(u64);

/// An exact, live task-lease registry reference; never model-controlled.
/// The future service adapter must provide its configured, canonical Git
/// executable, not a caller-selectable command or path.
struct GitReadLeaseRef<'a> {
    registry: &'a TaskLeaseRegistry,
    id: &'a TaskLeaseId,
    git_executable: &'a Path,
    repository_root: &'a Path,
}

#[derive(Debug, Error)]
enum OwnerError {
    #[error("owner reservation capacity exceeded")]
    Capacity,
    #[error("owner registry state unavailable")]
    Unavailable,
    #[error("owner identity exhausted")]
    IdExhausted,
    #[error("native owner absent or wrong session")]
    WrongOwner,
    #[error("native owner already adopted")]
    AlreadyAdopted,
    #[error("native owner payload missing")]
    MissingPayload,
    #[error("native owner observation in progress")]
    ObservationBusy,
    #[error("prepared native owner does not match session stop signals or was already started")]
    UnboundNativeOwner,
    #[error("session grant does not include GitRead capability")]
    MissingGitReadCapability,
    #[error("session was revoked before owner adoption")]
    Revoked,
    #[error("GitRead task lease has missing capability, repository scope, policy epoch or budget")]
    InvalidGitReadLease,
    #[error("exact canonical Git executable is not pinned by this GitRead lease")]
    GitExecutableNotAuthorized,
    #[error("GitRead repository must be the configured canonical directory")]
    GitRepositoryNotAuthorized,
    #[error("GitRead command spec identity or budget is not bound to this native owner")]
    UnboundGitReadSpec,
    #[error(transparent)]
    GitRead(#[from] GitReadError),
    #[error(transparent)]
    Lease(#[from] TaskLeaseRegistryError),
    #[error(transparent)]
    Session(#[from] SessionRegistryError),
    #[error(transparent)]
    Native(#[from] io::Error),
}

struct Record {
    session: SessionHandle,
    // The counted admission is owned by the service, never the request thread.
    // Dropping it while the Job is uncertain would let revoke drain too early.
    admission: OwnedSessionAdmissionPermit,
    budget: Budget,
    stop: Arc<AtomicBool>,
    revoked: Arc<AtomicBool>,
    lease_revoked: Option<Arc<AtomicBool>>,
    // Application-owned identities; low-level research reservations lack these.
    git_executable: Option<PathBuf>,
    repository_root: Option<PathBuf>,
    native: Option<Arc<Mutex<PreparedReadCommand>>>,
    observing: Arc<AtomicBool>,
}

struct State {
    next: u64,
    memory: u64,
    output: u64,
    cpu: u32,
    records: BTreeMap<OwnerId, Record>,
}

struct Observation(Arc<AtomicBool>);

impl Drop for Observation {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

struct NativeGitOwners {
    ceiling: Ceiling,
    state: Mutex<State>,
}

impl NativeGitOwners {
    fn new(ceiling: Ceiling) -> Result<Self, OwnerError> {
        if ceiling.count == 0
            || ceiling.memory == 0
            || ceiling.output == 0
            || !(1..=100).contains(&ceiling.cpu)
        {
            return Err(OwnerError::Capacity);
        }
        Ok(Self {
            ceiling,
            state: Mutex::new(State {
                next: 0,
                memory: 0,
                output: 0,
                cpu: 0,
                records: BTreeMap::new(),
            }),
        })
    }

    // The admission and all three ceilings are committed BEFORE a process can
    // exist. GitRead policy/task leases and executable pins remain separate
    // mandatory checks for a future production adapter.
    fn reserve(
        &self,
        sessions: &Arc<SessionRegistry>,
        session: &SessionHandle,
        now: MonotonicTime,
        budget: Budget,
    ) -> Result<OwnerId, OwnerError> {
        let admission = sessions.begin_owned_admission(session, now)?;
        let (revoked, current) = sessions.watch_active(session, now)?;
        let grant = admission.grant();
        // The owner may eventually spawn a Git process. A FileRead or
        // ProcessRun grant is never a substitute for explicit GitRead.
        // Reject before any ledger reservation or native preparation.
        if !grant.capabilities.contains(&Capability::GitRead) {
            return Err(OwnerError::MissingGitReadCapability);
        }
        if current.principal != grant.principal
            || current.project != grant.project
            || current.policy_epoch != grant.policy_epoch
        {
            return Err(OwnerError::Revoked);
        }
        if budget.memory == 0 || budget.output == 0 || !(1..=100).contains(&budget.cpu) {
            return Err(OwnerError::Capacity);
        }
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        if revoked.load(Ordering::Acquire) {
            return Err(OwnerError::Revoked);
        }
        let memory = state
            .memory
            .checked_add(budget.memory)
            .ok_or(OwnerError::Capacity)?;
        let output = state
            .output
            .checked_add(budget.output)
            .ok_or(OwnerError::Capacity)?;
        let cpu = state
            .cpu
            .checked_add(budget.cpu)
            .ok_or(OwnerError::Capacity)?;
        if state.records.len() >= self.ceiling.count
            || memory > self.ceiling.memory
            || output > self.ceiling.output
            || cpu > self.ceiling.cpu
        {
            return Err(OwnerError::Capacity);
        }
        let id = OwnerId(state.next.checked_add(1).ok_or(OwnerError::IdExhausted)?);
        state.next = id.0;
        state.memory = memory;
        state.output = output;
        state.cpu = cpu;
        state.records.insert(
            id,
            Record {
                session: session.clone(),
                admission,
                budget,
                stop: Arc::new(AtomicBool::new(false)),
                revoked,
                lease_revoked: None,
                git_executable: None,
                repository_root: None,
                native: None,
                observing: Arc::new(AtomicBool::new(false)),
            },
        );
        Ok(id)
    }

    /// The only failure-reclamation path for a reservation whose identifier
    /// has never been returned to a caller. Called solely if the trusted
    /// PreparedReadCommand::prepare path failed: it constructs Job/pipes/watchdog
    /// but has no process-spawn operation. Never expose this as release(id).
    fn reclaim_failed_unexposed_preparation(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<(), OwnerError> {
        let released = {
            let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
            let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
            if &record.session != session {
                return Err(OwnerError::WrongOwner);
            }
            if record.native.is_some() || record.observing.load(Ordering::Acquire) {
                return Err(OwnerError::ObservationBusy);
            }
            let record = state
                .records
                .remove(&id)
                .expect("unexposed reservation exists");
            state.memory -= record.budget.memory;
            state.output -= record.budget.output;
            state.cpu -= record.budget.cpu;
            record
        };
        // Session revoke may be waiting for this counted admission. Keep
        // the ledger unlocked while dropping it.
        drop(released);
        Ok(())
    }

    /// Trusted atomic service entry: reserve admission and all capacities
    /// first, construct a never-started native Job/pipe owner, and adopt it.
    /// The reservation id is not returned or otherwise exposed until adoption.
    /// Only a *pre-spawn preparation failure* can reclaim the still-unexposed
    /// record. Once an owner exists, errors retain it for native recovery.
    fn reserve_prepared(
        &self,
        sessions: &Arc<SessionRegistry>,
        authority: GitReadLeaseRef<'_>,
        session: &SessionHandle,
        now: MonotonicTime,
        budget: Budget,
        timeout_ms: u32,
    ) -> Result<OwnerId, OwnerError> {
        // Necessary research-only preflight, not a complete production GitRead
        // authorization. Exact repository, executable and typed policy
        // envelope must still be verified by a future service adapter.
        let grant = sessions.get_active(session, now)?;
        if !grant.allows(Capability::GitRead) {
            return Err(OwnerError::MissingGitReadCapability);
        }
        // A canonical configured executable and an exact FixedTool scope are
        // required. The registry verifies the original opened-and-hashed
        // executable pin before any counted reservation. GitRead does not
        // implicitly grant arbitrary ProcessRun.
        let canonical_git = std::fs::canonicalize(authority.git_executable)
            .map_err(|_| OwnerError::GitExecutableNotAuthorized)?;
        if canonical_git != authority.git_executable {
            return Err(OwnerError::GitExecutableNotAuthorized);
        }
        let git_path = canonical_git
            .to_str()
            .ok_or(OwnerError::GitExecutableNotAuthorized)?;
        let canonical_repository = std::fs::canonicalize(authority.repository_root)
            .map_err(|_| OwnerError::GitRepositoryNotAuthorized)?;
        if canonical_repository != authority.repository_root || !canonical_repository.is_dir() {
            return Err(OwnerError::GitRepositoryNotAuthorized);
        }
        let lease = authority.registry.get_active(authority.id, session, now)?;
        if !lease.has_scope(&LeaseScope::ProcessExecutable {
            executable: git_path.to_owned(),
            class: ProcessExecutionClass::FixedTool,
        }) {
            return Err(OwnerError::GitExecutableNotAuthorized);
        }
        // No command may outlive either the session grant or its task lease.
        // The native watchdog does not independently observe monotonic lease
        // expiry, so require both lifetimes to cover the finite command horizon.
        let until = now.saturating_add_millis(u64::from(timeout_ms));
        if grant.expires_at < until
            || lease.expires_at < until
            || !lease.allows(Capability::GitRead)
            || !lease.has_scope(&LeaseScope::Repository)
            || lease.policy_epoch != grant.policy_epoch
            || lease.resource_ceiling.process_count < 1
            || lease.resource_ceiling.memory_bytes < budget.memory
            || lease.resource_ceiling.output_bytes < budget.output
            || lease.resource_ceiling.timeout_ms < u64::from(timeout_ms)
        {
            return Err(OwnerError::InvalidGitReadLease);
        }
        let (lease_revoked, current_lease) =
            authority
                .registry
                .watch_active(authority.id, session, now)?;
        if current_lease.id != lease.id || current_lease.policy_epoch != lease.policy_epoch {
            return Err(OwnerError::InvalidGitReadLease);
        }
        let id = self.reserve(sessions, session, now, budget)?;
        {
            let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
            let record = state.records.get_mut(&id).ok_or(OwnerError::WrongOwner)?;
            if &record.session != session {
                return Err(OwnerError::WrongOwner);
            }
            // Atomic revocation signal remains owned across caller abandonment.
            record.lease_revoked = Some(lease_revoked);
            record.git_executable = Some(canonical_git);
            record.repository_root = Some(canonical_repository);
        }
        let signals = self.stop_signals(id, session)?;
        // None of these operations can create a child. Native preparation
        // only establishes Job limits, pipes and watchdog ownership.
        let prepared = (|| -> Result<PreparedReadCommand, OwnerError> {
            let output_limit = usize::try_from(budget.output).map_err(|_| OwnerError::Capacity)?;
            let limits = LimitedJobObject::new(1, budget.memory, budget.cpu)?;
            Ok(PreparedReadCommand::prepare(
                limits,
                output_limit,
                timeout_ms,
                signals,
            )?)
        })();
        let native = match prepared {
            Ok(native) => native,
            Err(error) => {
                // A trusted preparation error (including invalid timeout)
                // is positive proof that this record never owned a child.
                // On any ledger failure we conservatively retain capacity.
                self.reclaim_failed_unexposed_preparation(id, session)?;
                return Err(error);
            }
        };
        let mut native = Some(native);
        // A raced revoke is compatible with *custody* of this still-unstarted
        // owner; it does not grant spawn rights. An adoption error retains
        // reservations rather than guessing about OS resources.
        self.adopt(id, session, &mut native)?;
        Ok(id)
    }

    // Session stop/revoke and optional exact TaskLease revoke are independent
    // stop requests, none of which is a terminal native receipt.
    fn stop_signals(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<Vec<Arc<AtomicBool>>, OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.session != session {
            return Err(OwnerError::WrongOwner);
        }
        let mut signals = vec![Arc::clone(&record.stop), Arc::clone(&record.revoked)];
        if let Some(lease_revoked) = &record.lease_revoked {
            signals.push(Arc::clone(lease_revoked));
        }
        Ok(signals)
    }

    fn adopt(
        &self,
        id: OwnerId,
        session: &SessionHandle,
        owner: &mut Option<PreparedReadCommand>,
    ) -> Result<(), OwnerError> {
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get_mut(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.session != session {
            return Err(OwnerError::WrongOwner);
        }
        if record.native.is_some() {
            return Err(OwnerError::AlreadyAdopted);
        }
        // Adopting an *unstarted* native owner is cleanup ownership, NOT
        // authorization to launch. Even after revoke, it must be possible to
        // retain and prove the no-spawn Job/EOF receipt before releasing the
        // counted service admission. Accept ONLY the exact pre-reserved stop
        // atomics so a substituted owner cannot ignore revocation.
        let native = owner.as_ref().ok_or(OwnerError::MissingPayload)?;
        if !native.matches_unstarted_owner_signals(
            &record.stop,
            &record.revoked,
            record.lease_revoked.as_ref(),
        ) {
            return Err(OwnerError::UnboundNativeOwner);
        }
        if record.revoked.load(Ordering::Acquire) {
            record.stop.store(true, Ordering::Release);
        }
        // Check all fallible conditions before taking the caller-owned value.
        let native = owner.take().expect("prepared owner already validated");
        record.native = Some(Arc::new(Mutex::new(native)));
        Ok(())
    }

    fn stop_session(&self, session: &SessionHandle) -> Result<usize, OwnerError> {
        let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let mut count = 0;
        for record in state.records.values_mut() {
            if &record.session == session && !record.stop.swap(true, Ordering::AcqRel) {
                count += 1;
            }
        }
        Ok(count)
    }

    fn outstanding(&self, session: &SessionHandle) -> Result<usize, OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        Ok(state
            .records
            .values()
            .filter(|r| &r.session == session)
            .count())
    }

    // Does not hold the global budget mutex during potentially blocking native
    // operations; an external cancellation can still set the stop signal.
    fn with_owner<R>(
        &self,
        id: OwnerId,
        session: &SessionHandle,
        visit: impl FnOnce(&mut PreparedReadCommand) -> io::Result<R>,
    ) -> Result<R, OwnerError> {
        let (owner, _observation) = self.claim_owner(id, session)?;
        let mut native = owner.lock().map_err(|_| OwnerError::Unavailable)?;
        Ok(visit(&mut native)?)
    }

    // Both native start and terminal proof share ONE per-owner observer gate.
    // Neither operation keeps the global ledger locked while using the OS.
    fn claim_owner(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<(Arc<Mutex<PreparedReadCommand>>, Observation), OwnerError> {
        let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
        let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
        if &record.session != session {
            return Err(OwnerError::WrongOwner);
        }
        let owner = record.native.as_ref().ok_or(OwnerError::MissingPayload)?;
        if record
            .observing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(OwnerError::ObservationBusy);
        }
        Ok((
            Arc::clone(owner),
            Observation(Arc::clone(&record.observing)),
        ))
    }

    /// Research-only adapter: launch exactly the structured spec generated by
    /// application-owned GitReadService, under this reserved native Job owner.
    /// Refuse a different executable/root or an output ceiling increase BEFORE
    /// touching native start. Policy approval remains external and mandatory.
    fn start_git_read(
        &self,
        id: OwnerId,
        session: &SessionHandle,
        service: &GitReadService,
        args: &[OsString],
        output_limit: u64,
    ) -> Result<(), OwnerError> {
        let spec = service.command_spec(args, output_limit)?;
        let (owner, _observation) = self.claim_owner(id, session)?;
        {
            let state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
            let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
            if &record.session != session
                || !record.native.as_ref().is_some_and(|r| Arc::ptr_eq(r, &owner))
                || record.git_executable.as_deref() != Some(spec.executable.as_path())
                || record.repository_root.as_deref() != Some(spec.cwd.as_path())
                || spec.output_limit > record.budget.output
            {
                return Err(OwnerError::UnboundGitReadSpec);
            }
        }
        let mut command = Command::new(&spec.executable);
        command
            .current_dir(&spec.cwd)
            .args(&spec.args)
            .env_clear()
            .envs(&spec.env);
        let mut native = owner.lock().map_err(|_| OwnerError::Unavailable)?;
        native.start(&mut command)?;
        Ok(())
    }

    // No caller-supplied success boolean: the only release path requires the
    // staged native owner's own root+Job-empty+both-EOF terminal receipt and a
    // successfully claimed outcome. Any native error retains reservations.
    fn reap_terminal(
        &self,
        id: OwnerId,
        session: &SessionHandle,
    ) -> Result<Option<ReadCommandOutcome>, OwnerError> {
        let (owner, _observation) = self.claim_owner(id, session)?;
        let outcome = {
            let mut native = owner.lock().map_err(|_| OwnerError::Unavailable)?;
            if !native.poll_terminal()? {
                return Ok(None);
            }
            native.take_outcome()?
        };
        // Only an identical proven native payload may release its admission.
        let released = {
            let mut state = self.state.lock().map_err(|_| OwnerError::Unavailable)?;
            let record = state.records.get(&id).ok_or(OwnerError::WrongOwner)?;
            if &record.session != session
                || !record
                    .native
                    .as_ref()
                    .is_some_and(|p| Arc::ptr_eq(p, &owner))
            {
                return Err(OwnerError::WrongOwner);
            }
            let record = state.records.remove(&id).expect("proven record exists");
            state.memory -= record.budget.memory;
            state.output -= record.budget.output;
            state.cpu -= record.budget.cpu;
            record
        };
        // Releasing the counted session admission follows the kernel receipt,
        // and occurs OUTSIDE the resource ledger mutex.
        drop(released);
        Ok(Some(outcome))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_bridge_core::{
        PrincipalId, ProjectId, ResourceBudget, SessionGrant, TaskLease, WorkloadClass,
    };
    use optic_bridge_windows::LimitedJobObject;
    use std::{
        collections::BTreeSet,
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    fn grant(name: &str) -> SessionGrant {
        SessionGrant {
            handle: SessionHandle::generate().expect("session entropy"),
            principal: PrincipalId::new(name).expect("principal"),
            project: ProjectId::new("native-ledger-test").expect("project"),
            capabilities: BTreeSet::from([Capability::GitRead]),
            expires_at: MonotonicTime::from_millis(20000),
            policy_epoch: 1,
        }
    }

    fn budget() -> Budget {
        Budget {
            memory: 512 * 1024 * 1024,
            output: 16384,
            cpu: 25,
        }
    }

    fn ledger() -> NativeGitOwners {
        let b = budget();
        NativeGitOwners::new(Ceiling {
            count: 1,
            memory: b.memory,
            output: b.output,
            cpu: b.cpu,
        })
        .expect("limits")
    }

    fn fixture_git_executable() -> std::path::PathBuf {
        std::fs::canonicalize(std::env::current_exe().expect("synthetic native test executable"))
            .expect("canonical synthetic executable")
    }

    fn fixture_repository_root() -> std::path::PathBuf {
        std::fs::canonicalize(std::env::current_dir().expect("fixture working directory"))
            .expect("canonical fixture working directory")
    }

    fn task_lease(grant: &SessionGrant) -> TaskLease {
        TaskLease {
            id: TaskLeaseId::generate().expect("test lease identity"),
            session: grant.handle.clone(),
            capabilities: BTreeSet::from([Capability::GitRead]),
            scopes: BTreeSet::from([
                LeaseScope::Repository,
                LeaseScope::ProcessExecutable {
                    executable: fixture_git_executable()
                        .to_str()
                        .expect("fixture executable UTF-8")
                        .to_owned(),
                    class: ProcessExecutionClass::FixedTool,
                },
            ]),
            workload_class: WorkloadClass::Standard,
            resource_ceiling: ResourceBudget {
                timeout_ms: 10_000,
                output_bytes: budget().output,
                memory_bytes: budget().memory,
                process_count: 1,
            },
            expires_at: MonotonicTime::from_millis(10_000),
            policy_epoch: grant.policy_epoch,
        }
    }

    fn prepared_with_fixture_lease(
        owners: &NativeGitOwners,
        sessions: &Arc<SessionRegistry>,
        session: &SessionHandle,
        now: MonotonicTime,
        budget: Budget,
        timeout_ms: u32,
    ) -> Result<OwnerId, OwnerError> {
        let grant = sessions.get_active(session, now)?;
        let leases = TaskLeaseRegistry::new();
        let lease = task_lease(&grant);
        let id = lease.id.clone();
        leases.register(lease)?;
        owners.reserve_prepared(
            sessions,
            GitReadLeaseRef {
                registry: &leases,
                id: &id,
                git_executable: &fixture_git_executable(),
                    repository_root: &fixture_repository_root(),
            },
            session,
            now,
            budget,
            timeout_ms,
        )
    }

    #[test]
    fn non_git_read_grants_cannot_reserve_native_git_execution_capacity() {
        let sessions = Arc::new(SessionRegistry::new());
        let owners = ledger();
        let now = MonotonicTime::from_millis(1);

        // None of these independently useful capabilities authorizes GitRead.
        // In particular FileRead does not imply GitRead, nor does ProcessRun.
        for (name, capability) in [
            ("file-read-only", Capability::FileRead),
            ("process-run-only", Capability::ProcessRun),
            ("git-integrate-only", Capability::GitIntegrate),
        ] {
            let mut attempted = grant(name);
            attempted.capabilities = BTreeSet::from([capability]);
            sessions
                .register(attempted.clone())
                .expect("register limited session");
            assert!(matches!(
                prepared_with_fixture_lease(
                    &owners,
                    &sessions,
                    &attempted.handle,
                    now,
                    budget(),
                    3000
                ),
                Err(OwnerError::MissingGitReadCapability)
            ));
            assert_eq!(
                owners
                    .outstanding(&attempted.handle)
                    .expect("no owner allocated"),
                0
            );
            // A rejected attempt must drop its admission, not keep a revoke
            // barrier open or subtract from a different session's ceilings.
            assert!(
                sessions
                    .revoke(&attempted.handle)
                    .expect("denied reservation must not retain admission")
            );
        }

        let permitted = grant("explicit-git-read");
        sessions
            .register(permitted.clone())
            .expect("authorized GitRead session");
        let id =
            prepared_with_fixture_lease(&owners, &sessions, &permitted.handle, now, budget(), 3000)
                .expect("GitRead-capable grant may own a prepared native Job");
        assert_eq!(owners.outstanding(&permitted.handle).expect("reserved"), 1);
        assert_eq!(
            owners
                .stop_session(&permitted.handle)
                .expect("request stop"),
            1
        );
        let receipt = owners
            .reap_terminal(id, &permitted.handle)
            .expect("OS no-spawn terminal evidence")
            .expect("prepared Job stopped with native EOF");
        assert!(receipt.cancelled && receipt.status.is_none());
        assert_eq!(owners.outstanding(&permitted.handle).expect("released"), 0);
    }

    #[test]
    fn failed_native_preparation_reclaims_only_unexposed_never_spawned_owner() {
        use std::process::Command;

        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("preparation-failure-no-spawn");
        sessions.register(grant.clone()).expect("session grant");
        let owners = ledger();
        let now = MonotonicTime::from_millis(1);

        // The 0-ms value is rejected inside the real native prepare path,
        // strictly AFTER the service has reserved admission and budgets.
        // No fixture command, shell or native child is launched.
        let error =
            prepared_with_fixture_lease(&owners, &sessions, &grant.handle, now, budget(), 0)
                .expect_err("invalid native timeout must fail preparation");
        assert!(
            matches!(error, OwnerError::Native(ref native) if native.kind() == io::ErrorKind::InvalidInput),
            "native invalid-input proof required: {error:?}"
        );
        assert_eq!(
            owners
                .outstanding(&grant.handle)
                .expect("failed preparation released"),
            0,
            "trusted pre-spawn failure must not strand a counted admission"
        );

        // Capacity is actually available for a second authorized reservation.
        // A successful preparation remains owned until independently proven
        // no-spawn terminal after explicitly requesting session-scoped stop.
        let id =
            prepared_with_fixture_lease(&owners, &sessions, &grant.handle, now, budget(), 3000)
                .expect("fresh preparation after trusted failed no-spawn path");
        assert_eq!(id, OwnerId(2), "reclaimed IDs must not be reused");
        assert_eq!(owners.outstanding(&grant.handle).expect("owned"), 1);
        assert_eq!(owners.stop_session(&grant.handle).expect("owned stop"), 1);
        let mut command = Command::new(std::env::current_exe().expect("synthetic binary"));
        assert!(matches!(
            owners.with_owner(id, &grant.handle, |native| native.start(&mut command)),
            Err(OwnerError::Native(ref native)) if native.kind() == io::ErrorKind::Interrupted
        ));
        let outcome = owners
            .reap_terminal(id, &grant.handle)
            .expect("OS no-child/Job-empty/EOF evidence")
            .expect("native no-spawn receipt");
        assert!(outcome.cancelled && outcome.status.is_none());
        assert!(!outcome.is_success());
        assert_eq!(owners.outstanding(&grant.handle).expect("final release"), 0);
    }

    #[test]
    fn failed_preparation_does_not_steal_another_sessions_capacity() {
        let sessions = Arc::new(SessionRegistry::new());
        let a = grant("fail-prepare-a");
        let b = grant("normal-prepare-b");
        sessions.register(a.clone()).expect("register A");
        sessions.register(b.clone()).expect("register B");
        let owners = ledger();
        let now = MonotonicTime::from_millis(1);
        assert!(matches!(
            prepared_with_fixture_lease(&owners, &sessions, &a.handle, now, budget(), 0),
            Err(OwnerError::Native(ref e)) if e.kind() == io::ErrorKind::InvalidInput
        ));
        assert_eq!(owners.outstanding(&a.handle).expect("A release"), 0);
        let id = prepared_with_fixture_lease(&owners, &sessions, &b.handle, now, budget(), 3000)
            .expect("independent session B uses recovered global capacity");
        assert!(matches!(
            owners.stop_signals(id, &a.handle),
            Err(OwnerError::WrongOwner)
        ));
        assert_eq!(owners.outstanding(&b.handle).expect("B admitted"), 1);
        assert_eq!(owners.stop_session(&b.handle).expect("stop B"), 1);
        let receipt = owners
            .reap_terminal(id, &b.handle)
            .expect("native B terminal")
            .expect("never-spawned owner B stopped and proven");
        assert!(receipt.cancelled && receipt.status.is_none());
        assert_eq!(owners.outstanding(&b.handle).expect("B released"), 0);
    }

    #[test]
    fn revoked_before_adoption_is_owned_and_proven_no_spawn_before_releasing_admission() {
        use std::process::Command;

        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("revoked-before-adoption");
        sessions.register(grant.clone()).expect("register");
        let owners = ledger();
        let id = owners
            .reserve(
                &sessions,
                &grant.handle,
                MonotonicTime::from_millis(1),
                budget(),
            )
            .expect("reserve counted admission before native preparation");
        let signals = owners
            .stop_signals(id, &grant.handle)
            .expect("stop/revoke watch");
        let mut prepared = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("job"),
                budget().output as usize,
                3000,
                signals.clone(),
            )
            .expect("prepare native job and streams without launching"),
        );

        let sessions_for_revoke = Arc::clone(&sessions);
        let handle = grant.handle.clone();
        let (tx, rx) = mpsc::channel();
        let revoker = thread::spawn(move || {
            tx.send(sessions_for_revoke.revoke(&handle))
                .expect("send revoke result");
        });
        let started = Instant::now();
        while !signals[1].load(Ordering::Acquire) {
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "revoke bit never published"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let held_during_revoke = owners.outstanding(&grant.handle).expect("reservation");
        let drain_pending = matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty));
        let not_adopted = matches!(
            owners.reap_terminal(id, &grant.handle),
            Err(OwnerError::MissingPayload)
        );
        // Revoke does NOT grant permission to spawn, but must not strand the
        // counted reservation with no OS owner capable of proving no-spawn.
        owners
            .adopt(id, &grant.handle, &mut prepared)
            .expect("adopt stopped preparation solely for native cleanup");
        let transferred = prepared.is_none();
        let mut command = Command::new(std::env::current_exe().expect("test binary"));
        let launch_error = owners
            .with_owner(id, &grant.handle, |native| native.start(&mut command))
            .expect_err("revoked native process MUST NOT be spawned");
        let interrupted = matches!(
            launch_error,
            OwnerError::Native(ref e) if e.kind() == io::ErrorKind::Interrupted
        );
        let job_empty = owners
            .with_owner(id, &grant.handle, |native| native.native_active_processes())
            .expect("native Job accounting");
        let receipt = owners
            .reap_terminal(id, &grant.handle)
            .expect("full Job and EOF no-spawn proof")
            .expect("terminal native evidence");
        let capacity_released = owners
            .outstanding(&grant.handle)
            .expect("released reservation");
        let revoke_result = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("revoke must drain after native proof")
            .expect("revoke result");
        revoker.join().expect("revoke thread");

        assert_eq!(held_during_revoke, 1);
        assert!(drain_pending && not_adopted && transferred && interrupted);
        assert_eq!(job_empty, 0);
        assert!(
            receipt.status.is_none(),
            "no native root can have been created"
        );
        assert!(receipt.cancelled && !receipt.is_success());
        assert!(receipt.stdout.is_empty() && receipt.stderr.is_empty());
        assert_eq!(capacity_released, 0);
        assert!(revoke_result);
    }

    #[test]
    fn mismatched_native_stop_signals_cannot_be_adopted_or_release_capacity() {
        use std::process::Command;

        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("native-signals-bound-to-session");
        sessions.register(grant.clone()).expect("register");
        let owners = ledger();
        let id = owners
            .reserve(
                &sessions,
                &grant.handle,
                MonotonicTime::from_millis(1),
                budget(),
            )
            .expect("reserve");
        let signals = owners
            .stop_signals(id, &grant.handle)
            .expect("owner signals");
        let unrelated = vec![
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ];
        let mut foreign = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("foreign job"),
                budget().output as usize,
                3000,
                unrelated,
            )
            .expect("prepare unrelated native owner"),
        );
        let rejected = matches!(
            owners.adopt(id, &grant.handle, &mut foreign),
            Err(OwnerError::UnboundNativeOwner)
        );
        let caller_kept_foreign = foreign.is_some();
        let capacity_still_held = owners.outstanding(&grant.handle).expect("reservation");
        let no_owner_adopted = matches!(
            owners.reap_terminal(id, &grant.handle),
            Err(OwnerError::MissingPayload)
        );
        // Complete this valid no-spawn case using the actual bound atomics.
        let mut correctly_bound = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("job"),
                budget().output as usize,
                3000,
                signals,
            )
            .expect("prepare actual session owner"),
        );
        owners
            .adopt(id, &grant.handle, &mut correctly_bound)
            .expect("adopt only exact stop/revoke identity");
        let stopped = owners
            .stop_session(&grant.handle)
            .expect("session-scoped stop");
        let mut command = Command::new(std::env::current_exe().expect("test binary"));
        let start_rejected = matches!(
            owners.with_owner(id, &grant.handle, |native| native.start(&mut command)),
            Err(OwnerError::Native(ref e)) if e.kind() == io::ErrorKind::Interrupted
        );
        let receipt = owners
            .reap_terminal(id, &grant.handle)
            .expect("prove no native child and EOF")
            .expect("terminal receipt");
        let released = owners
            .outstanding(&grant.handle)
            .expect("capacity released");

        assert!(rejected && caller_kept_foreign && no_owner_adopted);
        assert_eq!(capacity_still_held, 1);
        assert_eq!(stopped, 1);
        assert!(start_rejected);
        assert!(receipt.cancelled && receipt.status.is_none());
        assert!(!receipt.is_success());
        assert_eq!(released, 0);
    }

    #[test]
    fn reserved_owner_retains_admission_until_native_quiescence() {
        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("owner-a");
        sessions.register(grant.clone()).expect("register");
        let now = MonotonicTime::from_millis(1);
        let owners = ledger();
        let id = owners
            .reserve(&sessions, &grant.handle, now, budget())
            .expect("reserve before spawn");
        let signals = owners
            .stop_signals(id, &grant.handle)
            .expect("native watchdog signals");
        let mut native = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("job"),
                budget().output as usize,
                3000,
                signals,
            )
            .expect("native preparation before child creation"),
        );
        owners
            .adopt(id, &grant.handle, &mut native)
            .expect("adopt BEFORE spawn");
        assert!(native.is_none());

        let sessions_for_revoke = Arc::clone(&sessions);
        let handle_for_revoke = grant.handle.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let revoker = thread::spawn(move || {
            done_tx
                .send(sessions_for_revoke.revoke(&handle_for_revoke))
                .expect("send revoke");
        });
        let start = Instant::now();
        while !owners.stop_signals(id, &grant.handle).expect("signals")[1].load(Ordering::Acquire) {
            assert!(
                start.elapsed() < Duration::from_secs(2),
                "revocation signal absent"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert_eq!(owners.outstanding(&grant.handle).expect("reserved"), 1);

        let result = owners
            .reap_terminal(id, &grant.handle)
            .expect("native observation");
        let receipt = result.expect("no-spawn revoked Job and EOF must be proven");
        assert!(receipt.cancelled && receipt.status.is_none());
        assert!(!receipt.is_success());
        assert_eq!(owners.outstanding(&grant.handle).expect("released"), 0);
        assert!(
            done_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("drained session")
                .expect("revoke")
        );
        revoker.join().expect("join revoke");
    }

    // Spawned only inside the synthetic, service-owned Windows Job. This is
    // not a GitRead or arbitrary executable path, and it performs no I/O to
    // user projects, network, secrets or external process IDs.
    #[test]
    fn owned_native_running_child_fixture() {
        if std::env::var_os("OPTIC317_LEDGER_CHILD").is_none() {
            return;
        }
        use std::io::Write;
        println!("OPTIC317_LEDGER_CHILD_ACTUALLY_RUNNING");
        std::io::stdout().flush().expect("synthetic child flush");
        let path = std::path::PathBuf::from(
            std::env::var_os("OPTIC317_LEDGER_STARTED").expect("synthetic startup marker"),
        );
        std::fs::write(path, b"alive before revoke").expect("mark synthetic child running");
        thread::sleep(Duration::from_secs(5));
    }

    #[test]
    fn live_native_owner_survives_revoke_until_job_empty_and_pipe_eof() {
        use std::{env, process::Command};

        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("live-owned-process");
        sessions
            .register(grant.clone())
            .expect("register synthetic session");
        let now = MonotonicTime::from_millis(1);
        let owners = ledger();
        let id = owners
            .reserve(&sessions, &grant.handle, now, budget())
            .expect("reserve counted owner before native creation");
        let signals = owners
            .stop_signals(id, &grant.handle)
            .expect("obtain independent stop signals");

        let mut native = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget().memory, budget().cpu).expect("native limits"),
                budget().output as usize,
                8_000,
                signals,
            )
            .expect("allocate native Job and pipes before child creation"),
        );
        owners
            .adopt(id, &grant.handle, &mut native)
            .expect("adopt prepared native resources before spawn");
        assert!(native.is_none());

        let gate = env::temp_dir().join(format!("optic317-service-ledger-{}", std::process::id()));
        let _ = std::fs::remove_file(&gate);
        let mut command = Command::new(env::current_exe().expect("synthetic test executable"));
        command
            .args([
                "--exact",
                "git_owner_registry::tests::owned_native_running_child_fixture",
                "--nocapture",
            ])
            .env_clear()
            .env("OPTIC317_LEDGER_CHILD", "1")
            .env("OPTIC317_LEDGER_STARTED", &gate);
        if let Some(system_root) = env::var_os("SystemRoot") {
            command.env("SystemRoot", system_root);
        }
        owners
            .with_owner(id, &grant.handle, |owner| owner.start(&mut command))
            .expect("registered owner for native spawn");

        let started = Instant::now();
        while !gate.is_file() {
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "owned synthetic child did not confirm its live execution"
            );
            thread::sleep(Duration::from_millis(5));
        }

        let active_before_revoke = owners
            .with_owner(id, &grant.handle, |owner| owner.native_active_processes())
            .expect("native Job accounting before revoke");
        let pending_before_revoke = owners
            .reap_terminal(id, &grant.handle)
            .expect("native owner still running")
            .is_none();
        let reservation_before_revoke = owners
            .outstanding(&grant.handle)
            .expect("counted budget retained");

        let (tx, rx) = mpsc::channel();
        let revoked_sessions = Arc::clone(&sessions);
        let revoked_handle = grant.handle.clone();
        let revoker = thread::spawn(move || {
            tx.send(revoked_sessions.revoke(&revoked_handle))
                .expect("send completed revoke");
        });

        let observing = Instant::now();
        while !owners
            .stop_signals(id, &grant.handle)
            .expect("stop signals")[1]
            .load(Ordering::Acquire)
        {
            assert!(
                observing.elapsed() < Duration::from_secs(2),
                "native owner did not see session revoke"
            );
            thread::sleep(Duration::from_millis(5));
        }

        let admission_still_blocks_revoke = matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty));
        let reservation_during_revoke = owners
            .outstanding(&grant.handle)
            .expect("hold reservation until native receipt");

        let stopping = Instant::now();
        let outcome = loop {
            if let Some(receipt) = owners
                .reap_terminal(id, &grant.handle)
                .expect("native terminal observation")
            {
                break receipt;
            }
            assert!(
                stopping.elapsed() < Duration::from_secs(3),
                "owned native termination and EOF not confirmed in finite time"
            );
            thread::sleep(Duration::from_millis(5));
        };

        let reservation_after_proof = owners
            .outstanding(&grant.handle)
            .expect("native confirmed release");
        let revoke_result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("session revoke must complete after native receipt")
            .expect("session registry revoke");
        revoker.join().expect("join revoke worker");
        let _ = std::fs::remove_file(&gate);

        // Assert collected evidence after the revoker has been joined.
        assert_eq!(active_before_revoke, 1, "actual native Job member required");
        assert!(
            pending_before_revoke,
            "running native child is not terminal"
        );
        assert_eq!(reservation_before_revoke, 1);
        assert!(admission_still_blocks_revoke);
        assert_eq!(reservation_during_revoke, 1);
        assert!(
            stopping.elapsed() < Duration::from_secs(5),
            "native cancellation must precede synthetic 5-second natural exit"
        );
        assert!(outcome.cancelled && !outcome.is_success());
        assert!(
            !outcome.deadline_expired,
            "external revoke, not 8s deadline"
        );
        assert!(
            outcome.status.is_some(),
            "owned native root must be observed"
        );
        assert!(
            String::from_utf8_lossy(&outcome.stdout)
                .contains("OPTIC317_LEDGER_CHILD_ACTUALLY_RUNNING")
        );
        assert_eq!(reservation_after_proof, 0);
        assert!(revoke_result);
    }

    #[test]
    fn reservation_rejects_cross_session_access_and_preserves_capacity() {
        let sessions = Arc::new(SessionRegistry::new());
        let a = grant("owner-a");
        let b = grant("owner-b");
        sessions.register(a.clone()).expect("register A");
        sessions.register(b.clone()).expect("register B");
        let owners = ledger();
        let now = MonotonicTime::from_millis(1);
        let id = owners
            .reserve(&sessions, &a.handle, now, budget())
            .expect("reserve A");
        assert!(matches!(
            owners.stop_signals(id, &b.handle),
            Err(OwnerError::WrongOwner)
        ));
        assert!(matches!(
            owners.reap_terminal(id, &b.handle),
            Err(OwnerError::WrongOwner)
        ));
        assert!(matches!(
            owners.reserve(&sessions, &b.handle, now, budget()),
            Err(OwnerError::Capacity)
        ));
        assert_eq!(owners.stop_session(&b.handle).expect("foreign stop"), 0);
        assert_eq!(owners.stop_session(&a.handle).expect("owned stop"), 1);
        assert_eq!(
            owners.outstanding(&a.handle).expect("retained reservation"),
            1
        );
    }

    #[test]
    fn git_read_native_reservation_denies_invalid_task_lease_before_capacity() {
        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("lease-authority");
        sessions.register(grant.clone()).expect("register session");
        let owners = ledger();
        let leases = TaskLeaseRegistry::new();
        let now = MonotonicTime::from_millis(1);

        let absent = TaskLeaseId::generate().expect("unknown lease");
        assert!(matches!(
            owners.reserve_prepared(
                &sessions,
                GitReadLeaseRef {
                    registry: &leases,
                    id: &absent,
                    git_executable: &fixture_git_executable(),
                    repository_root: &fixture_repository_root(),
                },
                &grant.handle,
                now,
                budget(),
                3000
            ),
            Err(OwnerError::Lease(TaskLeaseRegistryError::UnknownLease))
        ));

        for variant in 0..10 {
            let mut lease = task_lease(&grant);
            match variant {
                0 => lease.session = SessionHandle::generate().expect("other session"),
                1 => lease.capabilities = BTreeSet::from([Capability::FileRead]),
                2 => lease.scopes = BTreeSet::from([LeaseScope::WorkspaceAll]),
                3 => lease.policy_epoch += 1,
                4 => lease.expires_at = now,
                5 => lease.resource_ceiling.output_bytes = 1,
                6 => lease.resource_ceiling.timeout_ms = 1,
                7 => lease.expires_at = now.saturating_add_millis(100),
                8 => lease
                    .scopes
                    .retain(|scope| !matches!(scope, LeaseScope::ProcessExecutable { .. })),
                9 => {
                    let expected = LeaseScope::ProcessExecutable {
                        executable: fixture_git_executable()
                            .to_str()
                            .expect("UTF-8 fixture path")
                            .to_owned(),
                        class: ProcessExecutionClass::FixedTool,
                    };
                    assert!(lease.scopes.remove(&expected));
                    lease.scopes.insert(LeaseScope::ProcessExecutable {
                        executable: fixture_git_executable()
                            .to_str()
                            .expect("UTF-8 fixture path")
                            .to_owned(),
                        class: ProcessExecutionClass::Interpreter,
                    });
                }
                _ => unreachable!(),
            }
            let id = lease.id.clone();
            leases.register(lease).expect("register synthetic lease");
            let attempt = owners.reserve_prepared(
                &sessions,
                GitReadLeaseRef {
                    registry: &leases,
                    id: &id,
                    git_executable: &fixture_git_executable(),
                    repository_root: &fixture_repository_root(),
                },
                &grant.handle,
                now,
                budget(),
                3000,
            );
            assert!(
                matches!(
                    attempt,
                    Err(OwnerError::InvalidGitReadLease
                        | OwnerError::Lease(_)
                        | OwnerError::GitExecutableNotAuthorized)
                ),
                "variant {variant} must fail before owner allocation: {attempt:?}"
            );
            assert_eq!(
                owners.outstanding(&grant.handle).expect("no reservation"),
                0
            );
        }
        let mut short_session = grant.clone();
        short_session.handle = SessionHandle::generate().expect("short session");
        short_session.expires_at = now.saturating_add_millis(100);
        sessions
            .register(short_session.clone())
            .expect("short grant");
        let short_lease = task_lease(&short_session);
        let short_id = short_lease.id.clone();
        leases.register(short_lease).expect("short task lease");
        assert!(matches!(
            owners.reserve_prepared(
                &sessions,
                GitReadLeaseRef {
                    registry: &leases,
                    id: &short_id,
                    git_executable: &fixture_git_executable(),
                    repository_root: &fixture_repository_root(),
                },
                &short_session.handle,
                now,
                budget(),
                3000,
            ),
            Err(OwnerError::InvalidGitReadLease)
        ));
        assert_eq!(
            owners
                .outstanding(&short_session.handle)
                .expect("no short owner"),
            0
        );
        assert!(
            sessions
                .revoke(&short_session.handle)
                .expect("no short admission")
        );

        let valid = task_lease(&grant);
        let revoked_id = valid.id.clone();
        leases.register(valid).expect("revocable lease");
        assert!(
            leases
                .revoke(&revoked_id)
                .expect("revoke before reservation")
        );
        assert!(matches!(
            owners.reserve_prepared(
                &sessions,
                GitReadLeaseRef {
                    registry: &leases,
                    id: &revoked_id,
                    git_executable: &fixture_git_executable(),
                    repository_root: &fixture_repository_root(),
                },
                &grant.handle,
                now,
                budget(),
                3000,
            ),
            Err(OwnerError::Lease(TaskLeaseRegistryError::Revoked))
        ));
        assert_eq!(owners.outstanding(&grant.handle).expect("still empty"), 0);
        assert!(
            sessions
                .revoke(&grant.handle)
                .expect("no stranded admission")
        );
    }

    #[test]
    fn git_read_native_rejects_noncanonical_or_unpinned_executable() {
        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("exact-executable-scope");
        sessions.register(grant.clone()).expect("register session");
        let leases = TaskLeaseRegistry::new();
        let lease = task_lease(&grant);
        let id = lease.id.clone();
        leases
            .register(lease)
            .expect("capture exact test executable pin");
        let owners = ledger();
        let now = MonotonicTime::from_millis(1);

        // The registry has captured the real synthetic test binary. A caller
        // cannot point the identical GitRead lease at some different absolute
        // executable, even one backed by a valid regular file.
        let foreign =
            std::env::temp_dir().join(format!("optic317-unpinned-git-{}", std::process::id()));
        std::fs::write(&foreign, b"not a process or credential").expect("synthetic foreign file");
        let foreign = std::fs::canonicalize(&foreign).expect("canonical foreign fixture");
        assert!(matches!(
            owners.reserve_prepared(
                &sessions,
                GitReadLeaseRef {
                    registry: &leases,
                    id: &id,
                    git_executable: &foreign,
                    repository_root: &fixture_repository_root(),
                },
                &grant.handle,
                now,
                budget(),
                3000,
            ),
            Err(OwnerError::GitExecutableNotAuthorized)
        ));
        assert_eq!(
            owners.outstanding(&grant.handle).expect("no foreign owner"),
            0
        );
        std::fs::remove_file(&foreign).expect("remove synthetic foreign file");

        // A relative spelling cannot stand in for the pinned canonical
        // Git executable, even if it points to a valid file at runtime.
        assert!(matches!(
            owners.reserve_prepared(
                &sessions,
                GitReadLeaseRef {
                    registry: &leases,
                    id: &id,
                    git_executable: Path::new("."),
                },
                &grant.handle,
                now,
                budget(),
                3000,
            ),
            Err(OwnerError::GitExecutableNotAuthorized)
        ));
        assert_eq!(
            owners
                .outstanding(&grant.handle)
                .expect("no relative owner"),
            0
        );
        assert!(sessions.revoke(&grant.handle).expect("no admission held"));
    }

    #[test]
    fn real_gitread_spec_runs_under_native_owner_with_exact_repo_and_budget() {
        use optic_bridge_core::{ActionId, HardLimits};
        use std::{env, fs};

        let git = env::var_os("PATH")
            .and_then(|path| {
                env::split_paths(&path).find_map(|dir| {
                    let candidate = dir.join("git.exe");
                    candidate.is_file().then_some(candidate)
                })
            })
            .and_then(|path| fs::canonicalize(path).ok())
            .expect("real Git executable on Windows CI PATH");
        let token = ActionId::generate().expect("fixture token").to_token();
        let root = env::temp_dir().join(format!("optic317-real-git-native-{token}"));
        let repo = root.join("repo");
        let other = root.join("other-repo");
        fs::create_dir_all(&root).expect("synthetic fixture root");
        for path in [&repo, &other] {
            assert!(
                Command::new(&git)
                    .arg("init")
                    .arg("--quiet")
                    .arg(path)
                    .status()
                    .expect("initialize Git fixture")
                    .success(),
                "synthetic Git repository initialization"
            );
        }
        let service = GitReadService::from_hard_limits(&repo, &git, HardLimits::default())
            .expect("validated configured Git service");
        let other_service = GitReadService::from_hard_limits(&other, &git, HardLimits::default())
            .expect("second configured Git service");
        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("real-GitRead-native-service-spec");
        sessions.register(grant.clone()).expect("register authorized fixture session");
        let leases = TaskLeaseRegistry::new();
        let mut lease = task_lease(&grant);
        let pinned = LeaseScope::ProcessExecutable {
            executable: service
                .git_executable()
                .to_str()
                .expect("Windows fixture canonical UTF-8 Git path")
                .to_owned(),
            class: ProcessExecutionClass::FixedTool,
        };
        lease.scopes.retain(|scope| !matches!(scope, LeaseScope::ProcessExecutable { .. }));
        lease.scopes.insert(pinned);
        let lease_id = lease.id.clone();
        leases.register(lease).expect("pin real Git executable identity");
        let owners = ledger();
        let owner_id = owners
            .reserve_prepared(
                &sessions,
                GitReadLeaseRef {
                    registry: &leases,
                    id: &lease_id,
                    git_executable: service.git_executable(),
                    repository_root: service.repository_root(),
                },
                &grant.handle,
                MonotonicTime::from_millis(1),
                budget(),
                8000,
            )
            .expect("reserved exact Git executable/root and native Job first");
        let args = [
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
            "--ignore-submodules=all",
        ]
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>();

        // A valid Git service targeting a DIFFERENT canonical repository
        // cannot spend this already-authorized owner reservation.
        assert!(matches!(
            owners.start_git_read(
                owner_id,
                &grant.handle,
                &other_service,
                &args,
                8192,
            ),
            Err(OwnerError::UnboundGitReadSpec)
        ));
        assert!(matches!(
            owners.start_git_read(
                owner_id,
                &grant.handle,
                &service,
                &args,
                budget().output + 1,
            ),
            Err(OwnerError::UnboundGitReadSpec)
        ));
        assert_eq!(
            owners.outstanding(&grant.handle).expect("still held"),
            1,
            "denied starts must preserve service ownership"
        );
        owners
            .start_git_read(owner_id, &grant.handle, &service, &args, 8192)
            .expect("native suspended Git launched through production spec");
        let started = Instant::now();
        let receipt = loop {
            if let Some(receipt) = owners
                .reap_terminal(owner_id, &grant.handle)
                .expect("kernel Job/EOF check")
            {
                break receipt;
            }
            assert!(
                started.elapsed() < Duration::from_secs(6),
                "real Git did not provide native job and pipe terminal proof"
            );
            thread::sleep(Duration::from_millis(5));
        };
        assert!(receipt.is_success(), "real native Git status: {receipt:?}");
        assert!(
            receipt.stdout.windows(b"# branch.oid".len()).any(|v| v == b"# branch.oid"),
            "Git must actually execute status with the exact structured spec"
        );
        assert_eq!(owners.outstanding(&grant.handle).expect("fully reaped"), 0);
        assert!(sessions.revoke(&grant.handle).expect("session admission drained"));
        fs::remove_dir_all(root).expect("remove only synthetic Git repositories");
    }

    #[test]
    fn revoking_exact_task_lease_stops_prepared_native_owner_before_spawn() {
        use std::process::Command;

        let sessions = Arc::new(SessionRegistry::new());
        let grant = grant("lease-revoke-no-spawn");
        sessions.register(grant.clone()).expect("session");
        let leases = TaskLeaseRegistry::new();
        let lease = task_lease(&grant);
        let lease_id = lease.id.clone();
        leases.register(lease).expect("lease");
        let owners = ledger();
        let owner_id = owners
            .reserve_prepared(
                &sessions,
                GitReadLeaseRef {
                    registry: &leases,
                    id: &lease_id,
                    git_executable: &fixture_git_executable(),
                    repository_root: &fixture_repository_root(),
                },
                &grant.handle,
                MonotonicTime::from_millis(1),
                budget(),
                3000,
            )
            .expect("preparation held under exact lease");
        assert_eq!(owners.outstanding(&grant.handle).expect("reserved"), 1);
        assert!(leases.revoke(&lease_id).expect("publish task lease stop"));
        let mut command = Command::new(std::env::current_exe().expect("synthetic binary"));
        assert!(matches!(
            owners.with_owner(owner_id, &grant.handle, |native| native.start(&mut command)),
            Err(OwnerError::Native(ref e)) if e.kind() == io::ErrorKind::Interrupted
        ));
        assert_eq!(owners.outstanding(&grant.handle).expect("still counted"), 1);
        let receipt = owners
            .reap_terminal(owner_id, &grant.handle)
            .expect("native Job and two pipe EOF observations")
            .expect("never spawned child proved terminal");
        assert!(receipt.status.is_none());
        assert!(receipt.cancelled && !receipt.is_success());
        assert_eq!(
            owners
                .outstanding(&grant.handle)
                .expect("capacity released"),
            0
        );
        assert!(sessions.revoke(&grant.handle).expect("admission drained"));
    }
}
