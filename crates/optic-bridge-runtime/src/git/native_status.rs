//! Service-owned, non-executing preparation for one fixed native git status.
//! The future Windows guardian must adopt this value before native creation.
use std::path::Path;

use optic_bridge_core::{ActionEnvelope, ActionId, MonotonicTime, ResourceBudget, TaskLeaseId};
use thiserror::Error;

#[cfg(windows)]
use crate::{Clock, NativeGitReadLeaseError, verify_registered_native_git_read_preflight};

use crate::{
    GitReadError, GitReadService, HardenedCommandSpec, NativeGitReadAdmissionError,
    NativeGitReadAdmittedPreflight, NativeGitReadOwnedPermits, NativeGitReadTrustedContext,
    finish_owned_native_git_read_preflight,
};

/// A fixed service command and its original counted authorization resources.
/// This value is neither cloneable nor a native spawn permit. A future
/// registered guardian must retain it through real terminal OS/pipe proof.
#[must_use = "native preparation must remain owned until proven terminal"]
pub struct PreparedNativeGitStatus {
    // Only the service-clock constructor can establish this immutable binding.
    // The legacy logical-time preparation remains inert and cannot arm.
    #[cfg(windows)]
    watchdog_clock: Option<(crate::StdClock, std::time::Instant)>,
    pub(crate) command: HardenedCommandSpec,
    pub(crate) admission: NativeGitReadAdmittedPreflight,
    pub(crate) envelope: ActionEnvelope,
}

impl PreparedNativeGitStatus {
    #[must_use]
    pub fn repository_root(&self) -> &Path {
        &self.command.cwd
    }

    #[must_use]
    pub fn git_executable(&self) -> &Path {
        &self.command.executable
    }

    #[must_use]
    pub fn output_limit(&self) -> u64 {
        self.command.output_limit
    }

    #[must_use]
    pub fn action_id(&self) -> &ActionId {
        &self.envelope.action_id
    }

    #[must_use]
    pub fn resources(&self) -> ResourceBudget {
        self.envelope.resources
    }

    #[must_use]
    pub fn lease_id(&self) -> &TaskLeaseId {
        self.admission.lease_id()
    }

    /// The original absolute horizon; native preparation must not restart it.
    #[must_use]
    pub fn deadline(&self) -> MonotonicTime {
        self.admission.deadline()
    }

    /// Stop request only: false is never independent permission to spawn.
    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.admission.is_stop_required_at(now)
    }
}

#[derive(Debug, Error)]
pub enum NativeGitStatusPreparationError {
    #[error("native Git status requires UTF-8 service-owned targets")]
    NonUtf8Target,
    #[cfg(windows)]
    #[error("native Git status transport permit has missing or foreign clock provenance")]
    TransportClockMismatch,
    #[error(transparent)]
    Command(#[from] GitReadError),
    #[error(transparent)]
    Admission(#[from] NativeGitReadAdmissionError),
}

impl GitReadService {
    /// Prepare with the actual service clock used for the original worker
    /// permits. No caller-supplied logical now or arm-time clock is accepted.
    /// This retains the clock origin, but is still not launch authority.
    #[cfg(windows)]
    #[allow(dead_code)] // Non-executing until the native guardian is accepted.
    pub(crate) fn prepare_native_status_on_clock(
        &self,
        trusted: &NativeGitReadTrustedContext<'_>,
        owned: NativeGitReadOwnedPermits,
        envelope: &ActionEnvelope,
        clock: &crate::StdClock,
    ) -> Result<PreparedNativeGitStatus, NativeGitStatusPreparationError> {
        if !owned.transport_belongs_to_clock(clock) {
            return Err(NativeGitStatusPreparationError::TransportClockMismatch);
        }
        let mut prepared = self.prepare_native_status(trusted, owned, envelope, clock.now())?;
        let until = clock
            .deadline_instant(prepared.deadline())
            .filter(|until| *until > std::time::Instant::now())
            .ok_or(NativeGitReadAdmissionError::StopRequired)?;
        prepared.watchdog_clock = Some((clock.clone(), until));
        Ok(prepared)
    }

    /// Bind the fixed status spec to the service's configured canonical
    /// repository/executable and the worker's original owned permits.
    ///
    /// No caller-provided target, command arguments or separate output budget
    /// crosses this boundary. It does not create a process, reserve native
    /// quotas or approve sensitive metadata publication. Those controls and
    /// final synchronized identity/policy/revocation checks remain mandatory.
    pub fn prepare_native_status(
        &self,
        trusted: &NativeGitReadTrustedContext<'_>,
        owned: NativeGitReadOwnedPermits,
        envelope: &ActionEnvelope,
        now: MonotonicTime,
    ) -> Result<PreparedNativeGitStatus, NativeGitStatusPreparationError> {
        let command = self.status_command_spec(envelope.resources.output_bytes)?;
        let repository = command
            .cwd
            .to_str()
            .ok_or(NativeGitStatusPreparationError::NonUtf8Target)?;
        let executable = command
            .executable
            .to_str()
            .ok_or(NativeGitStatusPreparationError::NonUtf8Target)?;
        let admission = finish_owned_native_git_read_preflight(
            trusted, owned, envelope, repository, executable, now,
        )?;
        Ok(PreparedNativeGitStatus {
            #[cfg(windows)]
            watchdog_clock: None,
            command,
            admission,
            envelope: envelope.clone(),
        })
    }
}

/// Windows-only accounting stage between admitted status preparation and a
/// future OS-native Git guardian. This is NOT a Win32 Job handle or spawn permit:
/// NativeProcessReservation records capacity and configures limits, but does
/// not create a kernel Job, process, or pipes. Only later owner code may do so.
#[cfg(windows)]
#[must_use = "keep the original permits and process accounting until safe disposal"]
pub struct RegisteredNativeGitStatus {
    // Drop the no-child record before the original session/transport permits.
    record: crate::process::NativeProcessReservation,
    prepared: PreparedNativeGitStatus,
}

#[cfg(windows)]
impl RegisteredNativeGitStatus {
    #[must_use]
    pub fn job_id(&self) -> optic_bridge_core::JobId {
        self.record.id()
    }

    #[must_use]
    pub fn deadline(&self) -> MonotonicTime {
        self.prepared.deadline()
    }

    /// A stop request, not approval to create a process or proof of cleanup.
    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.prepared.is_stop_required_at(now) || self.record.stop_requested()
    }
}

#[cfg(windows)]
#[derive(Debug, Error)]
pub enum NativeGitStatusRegistrationError {
    #[error("the original native GitRead admission has expired or been revoked")]
    StopRequired,
    #[error("GitRead service repository differs from the configured process manager root")]
    WrongProcessRoot,
    #[error("native GitRead requires UTF-8 executable and fixed arguments")]
    NonUtf8Command,
    #[error("native GitRead preparation and action output budgets differ")]
    OutputBudgetMismatch,
    #[error(transparent)]
    Process(#[from] crate::ProcessError),
}

#[cfg(windows)]
impl crate::ProcessManager {
    /// Register a private, owner-scoped action and reserve actual shared
    /// global/session process, CPU, memory and output capacity before any
    /// potential native creation. The original owned admission, transport
    /// permit, task lease witness and action horizon remain inside the result.
    ///
    /// No child, OS Job handle or pipe is created by this method. In
    /// particular, successful registration grants NO CreateProcess authority:
    /// an owner must add true native Job/pipe custody, pinned Git identity and
    /// synchronous last-boundary revocation/policy checks before any launch.
    pub fn reserve_prepared_native_status(
        &self,
        prepared: PreparedNativeGitStatus,
        now: MonotonicTime,
    ) -> Result<RegisteredNativeGitStatus, NativeGitStatusRegistrationError> {
        if prepared.is_stop_required_at(now) {
            return Err(NativeGitStatusRegistrationError::StopRequired);
        }
        if self.native_registration_root() != prepared.repository_root() {
            return Err(NativeGitStatusRegistrationError::WrongProcessRoot);
        }
        if prepared.command.output_limit != prepared.envelope.resources.output_bytes {
            return Err(NativeGitStatusRegistrationError::OutputBudgetMismatch);
        }
        let executable = prepared
            .git_executable()
            .to_str()
            .ok_or(NativeGitStatusRegistrationError::NonUtf8Command)?
            .to_owned();
        let args = prepared
            .command
            .args
            .iter()
            .map(|arg| {
                arg.to_str()
                    .map(str::to_owned)
                    .ok_or(NativeGitStatusRegistrationError::NonUtf8Command)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let spec = crate::ProcessStartSpec {
            session: prepared.envelope.session.clone(),
            class: optic_bridge_core::ProcessExecutionClass::FixedTool,
            workload_class: prepared.admission.workload_class(),
            network: optic_bridge_core::NetworkAccess::Denied,
            executable,
            args,
            cwd: None,
            workspace_read_files: Vec::new(),
            env_allowlist: Vec::new(),
            resources: prepared.envelope.resources,
        };
        let record = self.reserve_native_private(&spec, prepared.action_id())?;
        Ok(RegisteredNativeGitStatus { record, prepared })
    }
}

/// Inert, unstarted Windows kernel Job and stdio custody tied to a single
/// registered private action. Never exposes CreateProcess or child ownership.
#[cfg(windows)]
#[must_use = "hold original MCP permits and OS handles through native ownership"]
pub struct PrecreatedNativeGitStatusCustody {
    // Drop the real unstarted Job/pipes before the original counted permits.
    native: optic_bridge_windows::PrecreatedNativeReadCustody,
    // Target handles outlive the Job; prevent service identity/path substitution.
    _executable_pin: optic_bridge_windows::PinnedExecutableFile,
    _repository_pin: optic_bridge_windows::PinnedDirectoryChain,
    registered: RegisteredNativeGitStatus,
}

#[cfg(windows)]
impl PrecreatedNativeGitStatusCustody {
    #[must_use]
    pub fn job_id(&self) -> optic_bridge_core::JobId {
        self.registered.job_id()
    }

    #[must_use]
    pub fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.registered.is_stop_required_at(now)
    }

    /// This queries a real kernel Job with no process started by this type.
    /// An empty Job during precreation is NOT terminal proof for a future run.
    pub fn native_active_processes(&self) -> std::io::Result<u32> {
        self.native.active_processes()
    }

    /// Observe current policy, captured Git executable identity and the
    /// *same original* TaskLease registry record while holding the original
    /// session/transport permits and unopened Job. An independent registry or
    /// a different generation with a reused opaque lease ID is rejected.
    ///
    /// This is a **snapshot**, not a synchronized CreateProcessW gate and
    /// never a permission to launch, resume, publish or release resources.
    /// A later native owner must recheck under a final synchronized boundary.
    pub fn observe_original_prelaunch_snapshot(
        &self,
        trusted: &NativeGitReadTrustedContext<'_>,
        now: MonotonicTime,
    ) -> Result<(), NativeGitStatusSnapshotError> {
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusSnapshotError::StopRequired);
        }
        let prepared = &self.registered.prepared;
        if &prepared.envelope.session != trusted.trusted_session {
            return Err(NativeGitStatusSnapshotError::WrongTrustedSession);
        }
        if !prepared
            .admission
            .belongs_to_session_registry(trusted.sessions)
        {
            return Err(NativeGitStatusSnapshotError::ForeignSessionRegistry);
        }
        // Close and this read share SessionRegistry's mutex. This alone is
        // NOT a cross-registry fence or a permission to create a process.
        prepared
            .admission
            .observe_original_session_under_close_lock(trusted.sessions, now)?;
        let repository = prepared
            .repository_root()
            .to_str()
            .ok_or(NativeGitStatusSnapshotError::NonUtf8Target)?;
        let executable = prepared
            .git_executable()
            .to_str()
            .ok_or(NativeGitStatusSnapshotError::NonUtf8Target)?;
        let observed = verify_registered_native_git_read_preflight(
            trusted.task_leases,
            trusted.policy,
            &prepared.envelope,
            prepared.admission.original_session_grant(),
            repository,
            executable,
            now,
        )?;
        if !prepared
            .admission
            .matches_original_lease_registration(&observed)
        {
            return Err(NativeGitStatusSnapshotError::LeaseGenerationMismatch);
        }
        // Re-observe the exact session registration after potentially expensive
        // lease/policy checks. A later revoke may still win before OS creation.
        prepared
            .admission
            .observe_original_session_under_close_lock(trusted.sessions, now)?;
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusSnapshotError::StopRequired);
        }
        Ok(())
    }

    /// Consume the uniquely owned, already registered and kernel-precreated
    /// candidate into a session-only commitment. Session close and this short
    /// receipt publication share the original registry mutex.
    ///
    /// This is deliberately NOT the final native GitRead commitment: the
    /// independent lease/policy issuer fence, CreateProcessW, durable guardian,
    /// completed stdio EOF and terminal OS proof are still absent.
    // Not yet wired into the MCP owner: intentionally inert research precursor.
    #[allow(dead_code)]
    pub(crate) fn commit_original_session(
        self,
        trusted: &NativeGitReadTrustedContext<'_>,
        now: MonotonicTime,
    ) -> Result<SessionCommittedNativeGitStatusCustody, NativeGitStatusSnapshotError> {
        self.observe_original_prelaunch_snapshot(trusted, now)?;
        let commit = self
            .registered
            .prepared
            .admission
            .commit_original_session_for_registered_owner(
                trusted.sessions,
                self.registered.job_id(),
                now,
            )?;
        // A close after the session-only commitment must remain observable.
        // There is still NO child, so a rejected transition safely disposes
        // the no-child owner and its original counted permits.
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusSnapshotError::StopRequired);
        }
        Ok(SessionCommittedNativeGitStatusCustody {
            unstarted: self,
            session_commit: commit,
        })
    }
}

/// An already registered, pinned and still unstarted native candidate with a
/// session-only original-generation commitment. Non-cloneable and never a
/// process launch token: no CreateProcess handle or launch API is exposed.
#[cfg(windows)]
#[must_use = "retain original custody and session close receipt together"]
#[allow(dead_code)] // Research-only until the native guardian is implemented.
pub(crate) struct SessionCommittedNativeGitStatusCustody {
    unstarted: PrecreatedNativeGitStatusCustody,
    session_commit: crate::session_registry::SessionNativeOwnerCommit,
}

#[cfg(windows)]
#[allow(dead_code)] // No production native launch path consumes this stage yet.
impl SessionCommittedNativeGitStatusCustody {
    pub(crate) fn job_id(&self) -> optic_bridge_core::JobId {
        self.session_commit.job_id().clone()
    }

    pub(crate) fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.session_commit.is_stop_required_at(now) || self.unstarted.is_stop_required_at(now)
    }

    pub(crate) fn native_active_processes(&self) -> std::io::Result<u32> {
        self.unstarted.native_active_processes()
    }

    /// Consume this already session-committed owner into a second,
    /// **lease-only** ordered observation. The session and TaskLease mutexes
    /// are entered separately and are NEVER represented as an atomic fence.
    ///
    /// No child is created. On rejection the already owned empty Job/pipes
    /// and counted original admission/transport are dropped as no-child work.
    #[allow(dead_code)] // Native launch guardian and policy issuer are absent.
    pub(crate) fn commit_original_lease(
        self,
        trusted: &NativeGitReadTrustedContext<'_>,
        now: MonotonicTime,
    ) -> Result<LeaseObservedNativeGitStatusCustody, NativeGitStatusSnapshotError> {
        // Repeat typed policy and original session/lease identity checks after
        // session receipt issuance; these are snapshots, not launch grants.
        self.unstarted
            .observe_original_prelaunch_snapshot(trusted, now)?;
        let original = &self.unstarted.registered.prepared;
        let lease_commit = original
            .admission
            .commit_original_lease_for_registered_owner(
                trusted.task_leases,
                &original.envelope.session,
                now,
            )?;
        if self.is_stop_required_at(now) || lease_commit.is_stop_required_at(now) {
            return Err(NativeGitStatusSnapshotError::StopRequired);
        }
        Ok(LeaseObservedNativeGitStatusCustody {
            session_owner: self,
            lease_commit,
        })
    }
}

/// Non-executing owner that retains independent session and TaskLease ordering
/// receipts. These were issued under DIFFERENT mutexes; a revocation may win
/// between them or immediately afterwards. This is NOT a native launch grant.
#[cfg(windows)]
#[must_use = "retain both original stop witnesses and native custody"]
#[allow(dead_code)] // No CreateProcessW path can consume this research stage.
pub(crate) struct LeaseObservedNativeGitStatusCustody {
    session_owner: SessionCommittedNativeGitStatusCustody,
    lease_commit: crate::task_lease_registry::TaskLeaseNativeOwnerCommit,
}

#[cfg(windows)]
#[allow(dead_code)] // Only hosted non-executing owner tests consume this stage.
impl LeaseObservedNativeGitStatusCustody {
    pub(crate) fn job_id(&self) -> optic_bridge_core::JobId {
        self.session_owner.job_id()
    }

    /// Arm once using ONLY retained original authorities and the original
    /// absolute horizon. These independent receipts are not a launch fence.
    /// No registry lock is held while the kernel watchdog is created.
    pub(crate) fn arm_original_watchdog(
        &mut self,
        trusted: &NativeGitReadTrustedContext<'_>,
    ) -> Result<(), NativeGitStatusSnapshotError> {
        let (clock, until) = self
            .session_owner
            .unstarted
            .registered
            .prepared
            .watchdog_clock
            .as_ref()
            .ok_or(NativeGitStatusSnapshotError::ClockRequired)?;
        let now = clock.now();
        let until = *until;
        self.session_owner
            .unstarted
            .observe_original_prelaunch_snapshot(trusted, now)?;
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusSnapshotError::StopRequired);
        }
        let unstarted = &mut self.session_owner.unstarted;
        let mut stops = unstarted
            .registered
            .prepared
            .admission
            .native_stop_signals();
        stops.push(unstarted.registered.record.stop_signal());
        unstarted.native.arm_watchdog_until(until, stops)?;
        // Revocation racing arm remains attached to the independent worker.
        // Failure is still no-child custody; callers must dispose it.
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusSnapshotError::StopRequired);
        }
        Ok(())
    }

    pub(crate) fn watchdog_stop_observed(&self) -> bool {
        self.session_owner.unstarted.native.watchdog_stop_observed()
    }

    pub(crate) fn watchdog_deadline_expired(&self) -> bool {
        self.session_owner
            .unstarted
            .native
            .watchdog_deadline_expired()
    }

    pub(crate) fn is_stop_required_at(&self, now: MonotonicTime) -> bool {
        self.session_owner.is_stop_required_at(now) || self.lease_commit.is_stop_required_at(now)
    }

    pub(crate) fn native_active_processes(&self) -> std::io::Result<u32> {
        self.session_owner.native_active_processes()
    }
}

/// Fail-closed read-only authority observations; none of these variants is a
/// native termination receipt, spawn permit or reason to release live owners.
#[cfg(windows)]
#[derive(Debug, Error)]
pub enum NativeGitStatusSnapshotError {
    #[error("native watchdog requires preparation on the original service clock")]
    ClockRequired,
    #[error("native watchdog could not be armed: {0}")]
    Watchdog(#[from] std::io::Error),
    #[error("original native Git status admission is revoked or expired")]
    StopRequired,
    #[error("prelaunch snapshot requires original trusted session")]
    WrongTrustedSession,
    #[error("prelaunch snapshot must use original session registry")]
    ForeignSessionRegistry,
    #[error("task lease lookup returned a different registration generation")]
    LeaseGenerationMismatch,
    #[error("original service-owned GitRead target is not UTF-8")]
    NonUtf8Target,
    #[error(transparent)]
    Session(#[from] crate::SessionRegistryError),
    #[error(transparent)]
    LeaseRecord(#[from] crate::TaskLeaseRegistryError),
    #[error(transparent)]
    Lease(#[from] NativeGitReadLeaseError),
}

#[cfg(windows)]
#[derive(Debug, Error)]
pub enum NativeGitStatusCustodyError {
    #[error("native Git status admission stopped before Job precreation")]
    StopRequired,
    #[error("native Git repository pin failed: {0}")]
    RepositoryPin(#[from] optic_bridge_windows::PinnedDirectoryError),
    #[error("native Git targets differ from the original canonical service identity")]
    TargetIdentityChanged,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(windows)]
impl RegisteredNativeGitStatus {
    /// Pins the *original* service-owned executable and repository by opened
    /// Windows handles, verifies their kernel-resolved canonical paths, and
    /// opens a real bounded Job, stdout/stderr pipes and closed stdin under
    /// the already counted admission. Any error has no child and releases
    /// the unstarted resources and original permits.
    ///
    /// This is path pin custody, NOT an independently revalidated original
    /// TaskLease content hash, synchronized launch gate, native execution,
    /// terminal/EOF proof or permission to wire MCP GitRead.
    pub fn precreate_native_custody(
        self,
        now: MonotonicTime,
    ) -> Result<PrecreatedNativeGitStatusCustody, NativeGitStatusCustodyError> {
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusCustodyError::StopRequired);
        }
        let repository = self.prepared.repository_root();
        let executable = self.prepared.git_executable();
        // Open handles that deny directory replacement and executable write/
        // delete sharing. A real post-open final path must match the trusted
        // canonical service spec, not just the original lexical path.
        let repository_pin = optic_bridge_windows::pin_native_git_read_repository_root(repository)?;
        if repository_pin.final_path() != repository {
            return Err(NativeGitStatusCustodyError::TargetIdentityChanged);
        }
        let executable_pin = optic_bridge_windows::open_pinned_executable(executable)?;
        if executable_pin.final_path() != executable {
            return Err(NativeGitStatusCustodyError::TargetIdentityChanged);
        }
        // Do not allocate another resource or establish any future custody
        // after already-observed revocation, even though no spawn exists.
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusCustodyError::StopRequired);
        }
        let native = optic_bridge_windows::PrecreatedNativeReadCustody::new(self.record.limits)?;
        // A revocation racing OS handle creation still cannot turn this
        // unstarted owner into spawn authority or fresh deadline.
        if self.is_stop_required_at(now) {
            return Err(NativeGitStatusCustodyError::StopRequired);
        }
        Ok(PrecreatedNativeGitStatusCustody {
            native,
            _executable_pin: executable_pin,
            _repository_pin: repository_pin,
            registered: self,
        })
    }
}
