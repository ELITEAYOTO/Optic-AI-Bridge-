//! Authorized native capture for private build inputs; no staging or execution.
use std::{path::Path, sync::Arc};

use optic_bridge_core::{
    ActionEnvelope, Effect, HardLimits, MonotonicTime, ProjectInputError, ProjectInputManifest,
    ProjectInputSnapshot,
};
use optic_bridge_policy::{PolicyDecision, PolicyEngine, PolicyReason};
use thiserror::Error;

use crate::{
    BoundedFileSystem, Clock, FileSystemError, SessionRegistry, SessionRegistryError,
    TaskLeaseRegistry, TaskLeaseRegistryError,
};
#[cfg(windows)]
use optic_bridge_windows::{
    WindowsFileIdentity, inspect_directory_no_reparse, pin_directory_chain,
};

/// Synchronous bounded service. A transport must retain its own execution and
/// blocking-I/O permits through this call, including when its caller disconnects.
pub struct ProjectInputCaptureService {
    #[cfg_attr(not(windows), allow(dead_code))]
    filesystem: BoundedFileSystem,
    sessions: Arc<SessionRegistry>,
    leases: Arc<TaskLeaseRegistry>,
    clock: Arc<dyn Clock>,
    max_duration_ms: u64,
    #[cfg(windows)]
    root_identity: WindowsFileIdentity,
}

impl ProjectInputCaptureService {
    #[cfg(windows)]
    pub(crate) fn workspace_root(&self) -> &Path {
        self.filesystem.root()
    }
    pub fn new(
        root: impl AsRef<Path>,
        limits: HardLimits,
        sessions: Arc<SessionRegistry>,
        leases: Arc<TaskLeaseRegistry>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, ProjectInputCaptureError> {
        limits
            .validate_nonzero()
            .map_err(|_| ProjectInputCaptureError::InvalidLimits)?;
        let filesystem = BoundedFileSystem::from_hard_limits(root, limits)?;
        #[cfg(windows)]
        let root_identity = inspect_directory_no_reparse(filesystem.root())
            .map_err(|_| ProjectInputCaptureError::NativeBoundary)?
            .identity;
        Ok(Self {
            filesystem,
            sessions,
            leases,
            clock,
            max_duration_ms: limits.max_request_duration_ms,
            #[cfg(windows)]
            root_identity,
        })
    }

    pub fn capture(
        &self,
        manifest: ProjectInputManifest,
        actions: &[ActionEnvelope],
    ) -> Result<ProjectInputSnapshot, ProjectInputCaptureError> {
        if actions.len() != manifest.files().len() {
            return Err(ProjectInputCaptureError::EffectMismatch);
        }
        let first = actions
            .first()
            .ok_or(ProjectInputCaptureError::EffectMismatch)?;
        let started = self.clock.now();
        let admission = self.sessions.begin_admission(&first.session, started)?;
        self.preflight(&manifest, actions, started)?;
        #[cfg(not(windows))]
        {
            drop(admission);
            Err(ProjectInputCaptureError::UnsupportedPlatform)
        }
        #[cfg(windows)]
        {
            // Keep the selected native root and ancestors pinned throughout.
            let _root = pin_directory_chain(self.filesystem.root(), None)
                .map_err(|_| ProjectInputCaptureError::NativeBoundary)?;
            if inspect_directory_no_reparse(self.filesystem.root())
                .map_err(|_| ProjectInputCaptureError::NativeBoundary)?
                .identity
                != self.root_identity
            {
                return Err(ProjectInputCaptureError::NativeBoundary);
            }
            let mut contents = Vec::with_capacity(manifest.files().len());
            for (input, action) in manifest.files().iter().zip(actions) {
                self.authorize(action, started)?;
                // Respect existing read hard limits; the manifest's larger
                // ceiling is not permission to increase them silently.
                let prepared =
                    self.filesystem
                        .prepare_read(&input.path, 0, Some(input.size_bytes.max(1)))?;
                if prepared.observation().canonical_path != input.path {
                    return Err(ProjectInputCaptureError::NativeBoundary);
                }
                self.authorize(action, started)?;
                let chunk = prepared.read()?;
                if !chunk.eof || chunk.next_offset != input.size_bytes {
                    return Err(ProjectInputError::ContentMismatch.into());
                }
                contents.push((input.path.clone(), chunk.bytes));
            }
            let snapshot = manifest.seal(contents)?;
            // Revocation/expiry while bytes were being copied discards them.
            for action in actions {
                self.authorize(action, started)?;
            }
            drop(admission);
            Ok(snapshot)
        }
    }

    pub(crate) fn preflight(
        &self,
        manifest: &ProjectInputManifest,
        actions: &[ActionEnvelope],
        started: MonotonicTime,
    ) -> Result<(), ProjectInputCaptureError> {
        if actions.len() != manifest.files().len() {
            return Err(ProjectInputCaptureError::EffectMismatch);
        }
        // All declared ownership/effects/policy must pass before any source is
        // opened. A manifest does not convert process authority into read rights.
        for (input, action) in manifest.files().iter().zip(actions) {
            if manifest.total_bytes() > action.resources.memory_bytes {
                return Err(ProjectInputCaptureError::PolicyDenied(
                    PolicyReason::ResourceBudgetExceeded,
                ));
            }
            let task = action
                .task_lease
                .as_ref()
                .ok_or(ProjectInputCaptureError::EffectMismatch)?;
            manifest.require_binding(&action.session, task, action.policy_epoch)?;
            if action.effect
                != (Effect::FileRead {
                    path: input.path.clone(),
                })
            {
                return Err(ProjectInputCaptureError::EffectMismatch);
            }
            self.authorize(action, started)?;
        }
        Ok(())
    }

    pub(crate) fn capture_admission(
        &self,
        action: &ActionEnvelope,
    ) -> Result<(crate::SessionAdmissionPermit<'_>, MonotonicTime), ProjectInputCaptureError> {
        let started = self.clock.now();
        Ok((
            self.sessions.begin_admission(&action.session, started)?,
            started,
        ))
    }

    pub(crate) fn authorize(
        &self,
        action: &ActionEnvelope,
        started: MonotonicTime,
    ) -> Result<(), ProjectInputCaptureError> {
        let now = self.clock.now();
        if now < started
            || now.as_millis() - started.as_millis()
                >= self.max_duration_ms.min(action.resources.timeout_ms)
        {
            return Err(ProjectInputCaptureError::Deadline);
        }
        self.authorize_at(action, now).map(|_| ())
    }

    #[cfg(windows)]
    pub(crate) fn live_lease(
        &self,
        action: &ActionEnvelope,
    ) -> Result<optic_bridge_core::TaskLease, ProjectInputCaptureError> {
        self.authorize_at(action, self.clock.now())
    }

    fn authorize_at(
        &self,
        action: &ActionEnvelope,
        now: MonotonicTime,
    ) -> Result<optic_bridge_core::TaskLease, ProjectInputCaptureError> {
        let session = self.sessions.get_active(&action.session, now)?;
        let id = action
            .task_lease
            .as_ref()
            .ok_or(ProjectInputCaptureError::EffectMismatch)?;
        let lease = self.leases.get_active(id, &action.session, now)?;
        match PolicyEngine.evaluate(action, &session, Some(&lease), now) {
            PolicyDecision::Allow => Ok(lease),
            PolicyDecision::Deny(reason) | PolicyDecision::RequireApproval(reason) => {
                Err(ProjectInputCaptureError::PolicyDenied(reason))
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ProjectInputCaptureError {
    #[error("project input capture has invalid hard limits")]
    InvalidLimits,
    #[error("project input capture requires the validated Windows filesystem boundary")]
    UnsupportedPlatform,
    #[error("project input capture native identity validation failed")]
    NativeBoundary,
    #[error("project input capture exceeded its monotonic deadline")]
    Deadline,
    #[error("project input capture requires one bound FileRead action per manifest entry")]
    EffectMismatch,
    #[error("project input read denied: {0:?}")]
    PolicyDenied(PolicyReason),
    #[error("project input contract failed: {0}")]
    Input(#[from] ProjectInputError),
    #[error("project input filesystem read failed: {0}")]
    FileSystem(#[from] FileSystemError),
    #[error("project input session is inactive: {0}")]
    Session(#[from] SessionRegistryError),
    #[error("project input task lease is inactive: {0}")]
    Lease(#[from] TaskLeaseRegistryError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_bridge_core::{
        ActionId, Capability, ContentVersion, LeaseScope, PrincipalId, ProjectId, ProjectInputSpec,
        ReadContentScope, ResourceBudget, SessionGrant, SessionHandle, TaskLease, TaskLeaseId,
        WorkloadClass, WorkspacePath,
    };
    use std::{collections::BTreeSet, fs, path::PathBuf};

    struct FixedClock;
    impl Clock for FixedClock {
        fn now(&self) -> MonotonicTime {
            MonotonicTime::from_millis(100)
        }
    }

    struct Fixture {
        root: PathBuf,
        sessions: Arc<SessionRegistry>,
        leases: Arc<TaskLeaseRegistry>,
        session: SessionGrant,
        task: TaskLease,
        state_root: Option<PathBuf>,
    }
    impl Fixture {
        fn new(capability: Capability, scope: ReadContentScope) -> Self {
            let root = std::env::temp_dir().join(format!(
                "optic-project-inputs-{}",
                ActionId::generate().unwrap().to_token()
            ));
            fs::create_dir(&root).unwrap();
            fs::create_dir(root.join("src")).unwrap();
            let sessions = Arc::new(SessionRegistry::new());
            let leases = Arc::new(TaskLeaseRegistry::new());
            let session = SessionGrant {
                handle: SessionHandle::generate().unwrap(),
                principal: PrincipalId::new("capture-test").unwrap(),
                project: ProjectId::new("project").unwrap(),
                capabilities: BTreeSet::from([capability]),
                expires_at: MonotonicTime::from_millis(100_000),
                policy_epoch: 1,
            };
            let task = TaskLease {
                id: TaskLeaseId::generate().unwrap(),
                session: session.handle.clone(),
                capabilities: BTreeSet::from([capability]),
                scopes: BTreeSet::from([LeaseScope::ReadContent(scope)]),
                resource_ceiling: budget(),
                workload_class: WorkloadClass::Standard,
                expires_at: session.expires_at,
                policy_epoch: 1,
            };
            sessions.register(session.clone()).unwrap();
            leases.register(task.clone()).unwrap();
            Self {
                root,
                sessions,
                leases,
                session,
                task,
                state_root: None,
            }
        }
        fn service(&self) -> ProjectInputCaptureService {
            ProjectInputCaptureService::new(
                &self.root,
                HardLimits::default(),
                self.sessions.clone(),
                self.leases.clone(),
                Arc::new(FixedClock),
            )
            .unwrap()
        }
        fn inputs(&self, path: &str, bytes: &[u8]) -> (ProjectInputManifest, Vec<ActionEnvelope>) {
            let path = WorkspacePath::parse(path).unwrap();
            let manifest = ProjectInputManifest::new(
                self.session.handle.clone(),
                self.task.id.clone(),
                1,
                [ProjectInputSpec {
                    path: path.clone(),
                    version: ContentVersion::from_bytes(bytes),
                    size_bytes: bytes.len() as u64,
                }],
            )
            .unwrap();
            let actions = vec![ActionEnvelope {
                action_id: ActionId::generate().unwrap(),
                session: self.session.handle.clone(),
                task_lease: Some(self.task.id.clone()),
                policy_epoch: 1,
                effect: Effect::FileRead { path },
                resources: budget(),
            }];
            (manifest, actions)
        }

        #[cfg(windows)]
        fn journaled_service(
            &mut self,
        ) -> (
            Arc<crate::PrivateProjectStageService>,
            Arc<crate::PrivateTaskJournal>,
        ) {
            assert!(self.state_root.is_none());
            let state_root = self.root.with_extension("private-state");
            fs::create_dir(&state_root).unwrap();
            self.state_root = Some(state_root.clone());
            let journal = crate::PrivateTaskJournal::open(&state_root, &self.root).unwrap();
            let service = Arc::new(
                crate::PrivateProjectStageService::with_journal(self.service(), journal.clone())
                    .unwrap(),
            );
            (service, journal)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // Unique test-owned root, never a caller-selected recursive target.
            assert!(
                self.root.parent().unwrap().canonicalize().unwrap()
                    == std::env::temp_dir().canonicalize().unwrap()
            );
            assert_eq!(
                self.root.canonicalize().unwrap().parent().unwrap(),
                std::env::temp_dir().canonicalize().unwrap()
            );
            fs::remove_dir_all(&self.root).unwrap();
            if let Some(state) = &self.state_root {
                assert_eq!(state, &self.root.with_extension("private-state"));
                assert_eq!(
                    state.canonicalize().unwrap().parent().unwrap(),
                    std::env::temp_dir().canonicalize().unwrap()
                );
                let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
                loop {
                    match fs::remove_dir_all(state) {
                        Ok(()) => break,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                        Err(_) if std::time::Instant::now() < until => {
                            std::thread::sleep(std::time::Duration::from_millis(10))
                        }
                        Err(error) => panic!("owned test journal cleanup failed: {error}"),
                    }
                }
            }
        }
    }
    fn budget() -> ResourceBudget {
        ResourceBudget {
            timeout_ms: 10_000,
            output_bytes: 1024,
            memory_bytes: 16 * 1024 * 1024,
            process_count: 1,
        }
    }

    #[test]
    fn scope_capability_binding_and_effect_denial_precede_missing_file_io() {
        let fixture = Fixture::new(
            Capability::FileRead,
            ReadContentScope::prefix(WorkspacePath::parse("src").unwrap()),
        );
        let service = fixture.service();
        let (manifest, actions) = fixture.inputs("outside.txt", b"x");
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::PolicyDenied(
                PolicyReason::ScopeNotAuthorized
            ))
        ));
        let (manifest, mut actions) = fixture.inputs("src/missing", b"x");
        actions[0].policy_epoch = 2;
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::Input(ProjectInputError::Binding))
        ));
        let (manifest, mut actions) = fixture.inputs("src/missing", b"x");
        actions[0].effect = Effect::FileSearch {
            root: Some(WorkspacePath::parse("src/missing").unwrap()),
        };
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::EffectMismatch)
        ));
        let other = Fixture::new(Capability::ProcessRun, ReadContentScope::all());
        let (manifest, actions) = other.inputs("missing", b"x");
        assert!(matches!(
            other.service().capture(manifest, &actions),
            Err(ProjectInputCaptureError::PolicyDenied(
                PolicyReason::MissingCapability
            ))
        ));
    }

    #[test]
    fn revoked_task_and_insufficient_memory_cannot_capture() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        let service = fixture.service();
        let (manifest, mut actions) = fixture.inputs("src/missing", b"ab");
        actions[0].resources.memory_bytes = 1;
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::PolicyDenied(
                PolicyReason::ResourceBudgetExceeded
            ))
        ));
        fixture.leases.revoke(&fixture.task.id).unwrap();
        let (manifest, actions) = fixture.inputs("src/missing", b"ab");
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::Lease(
                TaskLeaseRegistryError::Revoked
            ))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn captured_bytes_are_exact_and_unchanged_after_source_edit() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"original").unwrap();
        let service = fixture.service();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"original");
        let snapshot = service.capture(manifest, &actions).unwrap();
        fs::write(fixture.root.join("src/main.cjs"), b"modified").unwrap();
        assert_eq!(
            snapshot.content(&WorkspacePath::parse("src/main.cjs").unwrap()),
            Some(b"original".as_slice())
        );
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"original");
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::Input(
                ProjectInputError::ContentMismatch
            ))
        ));
        fs::write(fixture.root.join("src/empty"), b"").unwrap();
        let (manifest, actions) = fixture.inputs("src/empty", b"");
        assert!(service.capture(manifest, &actions).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn private_preparation_copies_sealed_bytes_and_releases_only_owned_inputs() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"module.exports=42;").unwrap();
        let service = crate::PrivateProjectStageService::new(fixture.service());
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"module.exports=42;");
        let stage = service.prepare(manifest, &actions).unwrap();
        assert!(
            stage
                .require_binding(&fixture.session.handle, &fixture.task.id, 1)
                .is_ok()
        );
        assert!(
            stage
                .require_binding(&SessionHandle::generate().unwrap(), &fixture.task.id, 1)
                .is_err()
        );
        let private_root = stage.input_root().to_path_buf();
        assert_ne!(private_root, fixture.root);
        assert_eq!(service.occupied_slots(), 1);
        fs::write(fixture.root.join("src/main.cjs"), b"changed").unwrap();
        assert_eq!(
            fs::read(private_root.join("src/main.cjs")).unwrap(),
            b"module.exports=42;"
        );
        assert!(!format!("{stage:?}").contains("module.exports"));
        stage.discard().unwrap();
        assert_eq!(service.occupied_slots(), 0);
        assert!(!private_root.exists());
        assert_eq!(
            fs::read(fixture.root.join("src/main.cjs")).unwrap(),
            b"changed"
        );
    }

    #[cfg(windows)]
    #[test]
    fn private_preparation_is_bounded_and_rejects_stale_inputs_without_a_slot_leak() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let service = crate::PrivateProjectStageService::new(fixture.service());
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"stale");
        assert!(service.prepare(manifest, &actions).is_err());
        assert_eq!(service.occupied_slots(), 0);
        let mut stages = Vec::new();
        for _ in 0..4 {
            let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
            stages.push(service.prepare(manifest, &actions).unwrap());
        }
        // The fifth source deliberately does not exist: capacity must be
        // decided before opening it, without evicting a prepared input set.
        let (manifest, actions) = fixture.inputs("src/missing.cjs", b"x");
        assert!(matches!(
            service.prepare(manifest, &actions),
            Err(crate::PrivateProjectStageError::Capacity)
        ));
        assert_eq!(service.occupied_slots(), 4);
        drop(stages);
        assert_eq!(service.occupied_slots(), 0);
    }

    #[cfg(windows)]
    #[test]
    fn private_preparation_uncertain_cleanup_retains_accounting_without_recursive_delete() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let service = crate::PrivateProjectStageService::new(fixture.service());
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
        let stage = service.prepare(manifest, &actions).unwrap();
        let root = stage.input_root().to_path_buf();
        let identity = inspect_directory_no_reparse(&root).unwrap().identity;
        fs::write(root.join("untracked.txt"), b"not declared").unwrap();
        assert!(stage.discard().is_err());
        assert_eq!(service.occupied_slots(), 1);
        assert_eq!(fs::read(fixture.root.join("src/main.cjs")).unwrap(), b"x");
        // Host cleans only this test-created extra file, after checking identity.
        // The library intentionally refuses recursive removal of unknown data.
        if root.try_exists().unwrap() {
            assert_eq!(
                inspect_directory_no_reparse(&root).unwrap().identity,
                identity
            );
            assert_eq!(
                fs::read(root.join("untracked.txt")).unwrap(),
                b"not declared"
            );
            fs::remove_file(root.join("untracked.txt")).unwrap();
            fs::remove_dir(root).unwrap();
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn private_preparation_worker_owns_profile_until_acknowledged_cleanup() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let service = Arc::new(crate::PrivateProjectStageService::new(fixture.service()));
        let guard = crate::TransportGuard::new(HardLimits::default().into()).unwrap();
        let io = crate::BlockingIoGovernor::new(1).unwrap();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
        let lease = service
            .clone()
            .prepare_background(
                manifest,
                actions,
                guard
                    .begin_execution(MonotonicTime::from_millis(100))
                    .unwrap(),
                io.clone(),
            )
            .await
            .unwrap();
        let root = lease.input_root().to_path_buf();
        assert_eq!(fs::read(root.join("src/main.cjs")).unwrap(), b"x");
        assert!(
            lease
                .require_binding(&fixture.session.handle, &fixture.task.id, 1)
                .is_ok()
        );
        assert!(
            lease
                .require_binding(&fixture.session.handle, &fixture.task.id, 2)
                .is_err()
        );
        assert_eq!(guard.active_requests(), 0);
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(2), io.run(|| 42))
                .await
                .unwrap()
                .unwrap(),
            42
        );
        lease.discard().await.unwrap();
        assert!(!root.exists());
        assert_eq!(service.occupied_slots(), 0);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn private_preparation_dropped_worker_lease_completes_owned_cleanup() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let service = Arc::new(crate::PrivateProjectStageService::new(fixture.service()));
        let guard = crate::TransportGuard::new(HardLimits::default().into()).unwrap();
        let io = crate::BlockingIoGovernor::new(1).unwrap();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
        let lease = service
            .clone()
            .prepare_background(
                manifest,
                actions,
                guard
                    .begin_execution(MonotonicTime::from_millis(100))
                    .unwrap(),
                io,
            )
            .await
            .unwrap();
        let root = lease.input_root().to_path_buf();
        drop(lease);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while service.occupied_slots() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(!root.exists());
        assert_eq!(fs::read(fixture.root.join("src/main.cjs")).unwrap(), b"x");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn private_preparation_cancelled_caller_retains_effect_permits_until_worker_finishes() {
        use std::sync::{
            Condvar, Mutex,
            atomic::{AtomicBool, Ordering},
        };
        struct WorkerClock {
            entered: AtomicBool,
            released: Mutex<bool>,
            wake: Condvar,
        }
        impl Clock for WorkerClock {
            fn now(&self) -> MonotonicTime {
                if std::thread::current().name() == Some("optic-private-inputs")
                    && !self.entered.swap(true, Ordering::SeqCst)
                {
                    let mut released = self.released.lock().unwrap();
                    while !*released {
                        released = self.wake.wait(released).unwrap();
                    }
                }
                MonotonicTime::from_millis(100)
            }
        }
        struct Release(Arc<WorkerClock>);
        impl Drop for Release {
            fn drop(&mut self) {
                *self.0.released.lock().unwrap() = true;
                self.0.wake.notify_all();
            }
        }
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let clock = Arc::new(WorkerClock {
            entered: AtomicBool::new(false),
            released: Mutex::new(false),
            wake: Condvar::new(),
        });
        let release = Release(clock.clone());
        let mut capture = fixture.service();
        capture.clock = clock.clone();
        let service = Arc::new(crate::PrivateProjectStageService::new(capture));
        let guard = crate::TransportGuard::new(HardLimits::default().into()).unwrap();
        let io = crate::BlockingIoGovernor::new(1).unwrap();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
        let task = tokio::spawn(
            service.clone().prepare_background(
                manifest,
                actions,
                guard
                    .begin_execution(MonotonicTime::from_millis(100))
                    .unwrap(),
                io.clone(),
            ),
        );
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !clock.entered.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(guard.active_requests(), 1);
        assert_eq!(service.occupied_slots(), 1);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), io.run(|| 42))
                .await
                .is_err()
        );
        drop(release);
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(2), io.run(|| 42))
                .await
                .unwrap()
                .unwrap(),
            42
        );
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while guard.active_requests() != 0 || service.occupied_slots() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn existing_read_limit_is_not_increased_by_manifest_capacity() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"ab").unwrap();
        let limits = HardLimits {
            max_fs_read_bytes: 1,
            ..HardLimits::default()
        };
        let service = ProjectInputCaptureService::new(
            &fixture.root,
            limits,
            fixture.sessions.clone(),
            fixture.leases.clone(),
            Arc::new(FixedClock),
        )
        .unwrap();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"ab");
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::FileSystem(
                FileSystemError::ReadLimitExceeded
            ))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn private_preparation_expiry_after_allocation_closes_profile_and_intent() {
        struct AfterAllocation(Arc<crate::PrivateTaskJournal>);
        impl Clock for AfterAllocation {
            fn now(&self) -> MonotonicTime {
                let allocated = self
                    .0
                    .recovery_report()
                    .unwrap()
                    .iter()
                    .any(|record| record.phase == crate::PrivateTaskRecoveryPhase::Allocating);
                MonotonicTime::from_millis(if allocated { 100_000 } else { 100 })
            }
        }
        let mut fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let state_root = fixture.root.with_extension("private-state");
        fs::create_dir(&state_root).unwrap();
        fixture.state_root = Some(state_root.clone());
        let journal = crate::PrivateTaskJournal::open(&state_root, &fixture.root).unwrap();
        let mut capture = fixture.service();
        capture.clock = Arc::new(AfterAllocation(journal.clone()));
        let service =
            crate::PrivateProjectStageService::with_journal(capture, journal.clone()).unwrap();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
        assert!(service.prepare(manifest, &actions).is_err());
        assert_eq!(service.occupied_slots(), 0);
        let report = journal.recovery_report().unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].phase, crate::PrivateTaskRecoveryPhase::Closed);
        assert_eq!(
            report[0].outcome,
            Some(crate::PrivateTaskOutcome::NoProcess)
        );
    }

    #[cfg(windows)]
    #[test]
    fn private_preparation_journal_capacity_precedes_missing_source_io() {
        let mut fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        let (service, journal) = fixture.journaled_service();
        for _ in 0..4 {
            let (manifest, actions) = fixture.inputs("src/missing.cjs", b"x");
            let _ticket = journal
                .begin(
                    &manifest,
                    &actions[0],
                    &optic_bridge_windows::EphemeralAppContainerPlan::generate().unwrap(),
                )
                .unwrap();
        }
        let (manifest, actions) = fixture.inputs("src/missing.cjs", b"x");
        assert!(matches!(
            service.prepare(manifest, &actions),
            Err(crate::PrivateProjectStageError::Journal(
                crate::PrivateTaskJournalError::Capacity
            ))
        ));
        assert_eq!(service.occupied_slots(), 0);
        assert!(!fixture.root.join("src/missing.cjs").exists());
    }

    #[cfg(windows)]
    #[test]
    fn private_preparation_stale_capture_closes_only_the_pre_native_intent() {
        let mut fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"changed").unwrap();
        let (service, journal) = fixture.journaled_service();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"old");
        assert!(service.prepare(manifest, &actions).is_err());
        let records = journal.recovery_report().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].phase, crate::PrivateTaskRecoveryPhase::Closed);
        assert_eq!(
            records[0].outcome,
            Some(crate::PrivateTaskOutcome::NoProcess)
        );
        assert_eq!(service.occupied_slots(), 0);
        assert_eq!(
            fs::read(fixture.root.join("src/main.cjs")).unwrap(),
            b"changed"
        );
    }

    #[cfg(windows)]
    #[test]
    fn private_preparation_corrupt_finalization_does_not_release_accounting() {
        use std::io::Write;
        let mut fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let (service, journal) = fixture.journaled_service();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
        let prepared = service.prepare(manifest, &actions).unwrap();
        let record = journal.recovery_report().unwrap().remove(0);
        let path = fixture
            .state_root
            .as_ref()
            .unwrap()
            .join("private-tasks-v1")
            .join(format!("{}.tasklog", record.run_id));
        fs::OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap()
            .write_all(b"{interrupted")
            .unwrap();
        assert!(prepared.discard().is_err());
        assert_eq!(service.occupied_slots(), 1);
        assert!(journal.recovery_report().is_err());
        assert_eq!(fs::read(fixture.root.join("src/main.cjs")).unwrap(), b"x");
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires an explicitly selected installed Node runtime"]
    async fn private_preparation_node_task_roundtrip() {
        private_node_task_case("roundtrip").await;
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires an explicitly selected installed Node runtime"]
    async fn private_preparation_node_task_stop() {
        private_node_task_case("stop").await;
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires an explicitly selected installed Node runtime"]
    async fn private_preparation_node_task_output_bound() {
        private_node_task_case("output").await;
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires an explicitly selected installed Node runtime"]
    async fn private_preparation_node_task_revocation() {
        private_node_task_case("revoke").await;
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires an explicitly selected installed Node runtime"]
    async fn private_preparation_node_task_timeout() {
        private_node_task_case("timeout").await;
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn private_task_crash_child() {
        if std::env::var_os("OPTIC_PRIVATE_NATIVE_CRASH_HANDOFF").is_none() {
            return;
        }
        assert_eq!(
            std::env::var("OPTIC_PRIVATE_NATIVE_CRASH_PROOF").as_deref(),
            Ok("1")
        );
        private_node_task_case("host-crash").await;
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "CI-only deliberate host exit retains one quarantined native profile"]
    fn private_preparation_abrupt_native_owner_exit_is_quarantined() {
        use std::io::Read;
        assert_eq!(
            std::env::var("OPTIC_PRIVATE_NATIVE_CRASH_PROOF").as_deref(),
            Ok("1")
        );
        let control = Fixture::new(Capability::FileRead, ReadContentScope::all());
        let handoff = control.root.join("handoff.txt");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "project_inputs::tests::private_task_crash_child",
                "--nocapture",
            ])
            .env("OPTIC_PRIVATE_NATIVE_CRASH_HANDOFF", &handoff)
            .spawn()
            .unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= until {
                let _ = child.kill();
                let _ = child.wait();
                panic!("owned crash fixture exceeded deadline");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(72));
        let mut bytes = Vec::new();
        fs::File::open(&handoff)
            .unwrap()
            .take(8193)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= 8192);
        let text = String::from_utf8(bytes).unwrap();
        let paths = text.lines().map(PathBuf::from).collect::<Vec<_>>();
        assert_eq!(paths.len(), 2);
        let project = &paths[0];
        let state = &paths[1];
        assert_eq!(
            project.canonicalize().unwrap().parent().unwrap(),
            std::env::temp_dir().canonicalize().unwrap()
        );
        assert!(
            project
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("optic-project-inputs-")
        );
        assert_eq!(state, &project.with_extension("private-state"));
        let journal = crate::PrivateTaskJournal::open(state, project).unwrap();
        let report = journal.recovery_report().unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].phase, crate::PrivateTaskRecoveryPhase::Running);
        assert!(report[0].outcome.is_none());
        // No PID reopening, task replay or profile cleanup is attempted. The
        // disposable CI runner retains this small owned quarantine as evidence.
        assert!(project.join("src/main.cjs").exists());
    }

    #[cfg(windows)]
    async fn private_node_task_case(case: &str) {
        use crate::{ProcessAuthority, ProcessManager, ProcessStatus, ProcessStream};
        let selected = std::env::var("OPTIC_PRIVATE_TASK_NODE").expect("select Node explicitly");
        let selected = fs::canonicalize(selected)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let mut fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fixture.session.capabilities.insert(Capability::ProcessRun);
        fixture.sessions = Arc::new(SessionRegistry::new());
        fixture.sessions.register(fixture.session.clone()).unwrap();
        let class = optic_bridge_core::ProcessExecutionClass::RepositoryCode;
        let process_task = TaskLease {
            id: TaskLeaseId::generate().unwrap(),
            session: fixture.session.handle.clone(),
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([
                LeaseScope::ProcessExecutable {
                    executable: selected.clone(),
                    class,
                },
                LeaseScope::ProcessIsolationEligible {
                    executable: selected.clone(),
                    class,
                },
            ]),
            resource_ceiling: ResourceBudget {
                timeout_ms: if case == "timeout" { 1000 } else { 10000 },
                output_bytes: 4096,
                memory_bytes: 512 * 1024 * 1024,
                process_count: 4,
            },
            workload_class: WorkloadClass::Standard,
            expires_at: fixture.session.expires_at,
            policy_epoch: 1,
        };
        fixture.leases.register(process_task.clone()).unwrap();
        let roundtrip = br#"const fs=require('node:fs'),assert=require('node:assert/strict');
assert.equal(require('./math.cjs').add(20,22),42);
let denied=false;try{fs.writeFileSync(__dirname+'/forbidden.txt','x');}catch(e){denied=['ERR_ACCESS_DENIED','EACCES','EPERM'].includes(e.code);}
assert.equal(denied,true);console.log('PRIVATE_NODE_TASK_PASS');"#;
        let main: &[u8] = match case {
            "roundtrip" => roundtrip,
            "stop" | "revoke" | "timeout" | "host-crash" => {
                b"console.log('PRIVATE_NODE_READY');setInterval(()=>{},1000);"
            }
            "output" => b"while(true){process.stdout.write('x'.repeat(16384));}",
            _ => panic!("unknown test case"),
        };
        let math = b"exports.add=(a,b)=>a+b;";
        fs::write(fixture.root.join("src/main.cjs"), main).unwrap();
        fs::write(fixture.root.join("src/math.cjs"), math).unwrap();
        let mut actions = fixture.inputs("src/main.cjs", main).1;
        actions.extend(fixture.inputs("src/math.cjs", math).1);
        let manifest = ProjectInputManifest::new(
            fixture.session.handle.clone(),
            fixture.task.id.clone(),
            1,
            [("src/main.cjs", main), ("src/math.cjs", math.as_slice())]
                .into_iter()
                .map(|(path, bytes)| ProjectInputSpec {
                    path: WorkspacePath::parse(path).unwrap(),
                    version: ContentVersion::from_bytes(bytes),
                    size_bytes: bytes.len() as u64,
                }),
        )
        .unwrap();
        let state_root = fixture.root.with_extension("private-state");
        let (service, journal) = fixture.journaled_service();
        let transport = crate::TransportGuard::new(HardLimits::default().into()).unwrap();
        let io = crate::BlockingIoGovernor::new(2).unwrap();
        let prepared = service
            .clone()
            .prepare_background(
                manifest,
                actions,
                transport
                    .begin_execution(MonotonicTime::from_millis(100))
                    .unwrap(),
                io,
            )
            .await
            .unwrap();
        let root = prepared.input_root().to_path_buf();
        let processes = Arc::new(
            ProcessManager::new(&fixture.root, HardLimits::default(), Vec::new()).unwrap(),
        );
        let authority =
            Arc::new(ProcessAuthority::capture(process_task.id.clone(), &selected, class).unwrap());
        let action = ActionEnvelope {
            action_id: ActionId::generate().unwrap(),
            session: fixture.session.handle.clone(),
            task_lease: Some(process_task.id.clone()),
            policy_epoch: 1,
            effect: Effect::ProcessRun {
                executable: selected,
                class,
                network: optic_bridge_core::NetworkAccess::Denied,
            },
            resources: process_task.resource_ceiling,
        };
        let action_id = action.action_id.clone();
        let replay_spec = crate::ProcessStartSpec {
            session: action.session.clone(),
            class,
            workload_class: process_task.workload_class,
            network: optic_bridge_core::NetworkAccess::Denied,
            executable: authority.canonical_path().to_owned(),
            args: Vec::new(),
            cwd: None,
            workspace_read_files: Vec::new(),
            env_allowlist: Vec::new(),
            resources: action.resources,
        };
        let job = prepared
            .start_node(
                crate::PrivateNodeTaskStart {
                    action,
                    runtime: authority,
                    processes: processes.clone(),
                    entry: WorkspacePath::parse("src/main.cjs").unwrap(),
                },
                transport
                    .begin_execution(MonotonicTime::from_millis(100))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            processes
                .find_recorded_private_action(&fixture.session.handle, &action_id)
                .unwrap(),
            Some(job.clone())
        );
        assert!(
            processes
                .find_recorded_private_action(&SessionHandle::generate().unwrap(), &action_id)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            processes.reserve_native_private(&replay_spec, &action_id),
            Err(crate::ProcessError::PrivateActionAlreadyRecorded)
        ));
        if matches!(case, "stop" | "revoke" | "host-crash") {
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    let output = processes
                        .read(
                            &fixture.session.handle,
                            &job,
                            ProcessStream::Stdout,
                            0,
                            4096,
                        )
                        .unwrap();
                    if output.bytes.starts_with(b"PRIVATE_NODE_READY") {
                        break;
                    }
                    assert_eq!(
                        processes
                            .result(&fixture.session.handle, &job)
                            .unwrap()
                            .status,
                        ProcessStatus::Running
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            if case == "stop" {
                assert!(processes.stop(&fixture.session.handle, &job).unwrap());
            } else if case == "revoke" {
                assert!(fixture.sessions.revoke(&fixture.session.handle).unwrap());
            } else {
                let handoff = std::env::var_os("OPTIC_PRIVATE_NATIVE_CRASH_HANDOFF").unwrap();
                fs::write(
                    handoff,
                    format!("{}\n{}\n", fixture.root.display(), state_root.display()),
                )
                .unwrap();
                std::process::exit(72);
            }
        }
        let result = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            loop {
                let result = processes.result(&fixture.session.handle, &job).unwrap();
                if result.status != ProcessStatus::Running {
                    break result;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let stderr = processes
            .read(
                &fixture.session.handle,
                &job,
                ProcessStream::Stderr,
                0,
                4096,
            )
            .unwrap();
        assert_eq!(
            result.status,
            match case {
                "stop" | "revoke" => ProcessStatus::Stopped,
                "timeout" => ProcessStatus::TimedOut,
                "output" => ProcessStatus::OutputLimitExceeded,
                _ => ProcessStatus::Exited,
            },
            "stderr={}",
            String::from_utf8_lossy(&stderr.bytes)
        );
        assert_eq!(
            result.exit_code,
            if case == "roundtrip" { Some(0) } else { None },
            "stderr={}",
            String::from_utf8_lossy(&stderr.bytes)
        );
        let stdout = processes
            .read(
                &fixture.session.handle,
                &job,
                ProcessStream::Stdout,
                0,
                4096,
            )
            .unwrap();
        assert!(stdout.eof);
        if case == "roundtrip" {
            assert_eq!(stdout.bytes, b"PRIVATE_NODE_TASK_PASS\n");
        }
        if case == "output" {
            assert!(stdout.truncated);
            assert!(stdout.bytes.len() <= 4096);
        }
        assert_eq!(service.occupied_slots(), 0);
        assert_eq!(transport.active_requests(), 0);
        assert!(!root.exists());
        assert_eq!(fs::read(fixture.root.join("src/main.cjs")).unwrap(), main);
        assert!(!fixture.root.join("src/forbidden.txt").exists());
        let report = journal.recovery_report().unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].phase, crate::PrivateTaskRecoveryPhase::Closed);
        assert_eq!(
            report[0].outcome,
            Some(match case {
                "stop" | "revoke" => crate::PrivateTaskOutcome::Stopped,
                "timeout" => crate::PrivateTaskOutcome::TimedOut,
                "output" => crate::PrivateTaskOutcome::OutputLimitExceeded,
                _ => crate::PrivateTaskOutcome::Exited { code: 0 },
            })
        );
    }

    #[cfg(windows)]
    #[test]
    fn deadline_after_source_copy_discards_snapshot() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct DeadlineClock(AtomicUsize);
        impl Clock for DeadlineClock {
            fn now(&self) -> MonotonicTime {
                MonotonicTime::from_millis(if self.0.fetch_add(1, Ordering::SeqCst) < 4 {
                    100
                } else {
                    10_100
                })
            }
        }
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let mut service = fixture.service();
        service.clock = Arc::new(DeadlineClock(AtomicUsize::new(0)));
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::Deadline)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn selected_root_replacement_and_reparse_ancestors_are_rejected() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        let service = fixture.service();
        let original = fixture.root.with_extension("original");
        fs::rename(&fixture.root, &original).unwrap();
        fs::create_dir(&fixture.root).unwrap();
        fs::write(fixture.root.join("main.cjs"), b"x").unwrap();
        let (manifest, actions) = fixture.inputs("main.cjs", b"x");
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::NativeBoundary)
        ));
        fs::remove_file(fixture.root.join("main.cjs")).unwrap();
        fs::remove_dir(&fixture.root).unwrap();
        fs::rename(&original, &fixture.root).unwrap();
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        std::os::windows::fs::symlink_dir(fixture.root.join("src"), fixture.root.join("alias"))
            .unwrap();
        let (manifest, actions) = fixture.inputs("alias/main.cjs", b"x");
        assert!(service.capture(manifest, &actions).is_err());
        fs::remove_dir(fixture.root.join("alias")).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn revocation_after_source_copy_discards_snapshot() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct RevokeClock {
            calls: AtomicUsize,
            sessions: Arc<SessionRegistry>,
            handle: SessionHandle,
            worker: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
        }
        impl Clock for RevokeClock {
            fn now(&self) -> MonotonicTime {
                if self.calls.fetch_add(1, Ordering::SeqCst) == 4 {
                    // Session revoke fences admission immediately, then waits
                    // for this capture's permit to drain. Run it concurrently;
                    // calling it synchronously here would wait for ourselves.
                    let sessions = self.sessions.clone();
                    let handle = self.handle.clone();
                    *self.worker.lock().unwrap() = Some(std::thread::spawn(move || {
                        sessions.revoke(&handle).unwrap();
                    }));
                    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
                    while !matches!(
                        self.sessions
                            .get_active(&self.handle, MonotonicTime::from_millis(100)),
                        Err(SessionRegistryError::Revoked)
                    ) {
                        assert!(
                            std::time::Instant::now() < until,
                            "concurrent revoke must fence admission"
                        );
                        std::thread::yield_now();
                    }
                }
                MonotonicTime::from_millis(100)
            }
        }
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        fs::write(fixture.root.join("src/main.cjs"), b"x").unwrap();
        let mut service = fixture.service();
        let clock = Arc::new(RevokeClock {
            calls: AtomicUsize::new(0),
            sessions: fixture.sessions.clone(),
            handle: fixture.session.handle.clone(),
            worker: std::sync::Mutex::new(None),
        });
        service.clock = clock.clone();
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
        assert!(matches!(
            service.capture(manifest, &actions),
            Err(ProjectInputCaptureError::Session(
                SessionRegistryError::Revoked
            ))
        ));
        clock.worker.lock().unwrap().take().unwrap().join().unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn unproven_platform_does_not_open_sources() {
        let fixture = Fixture::new(Capability::FileRead, ReadContentScope::all());
        let (manifest, actions) = fixture.inputs("src/missing", b"x");
        assert!(matches!(
            fixture.service().capture(manifest, &actions),
            Err(ProjectInputCaptureError::UnsupportedPlatform)
        ));
    }
}
