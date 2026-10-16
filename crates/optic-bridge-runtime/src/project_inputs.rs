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
        let session = self.sessions.get_active(&action.session, now)?;
        let id = action
            .task_lease
            .as_ref()
            .ok_or(ProjectInputCaptureError::EffectMismatch)?;
        let lease = self.leases.get_active(id, &action.session, now)?;
        match PolicyEngine.evaluate(action, &session, Some(&lease), now) {
            PolicyDecision::Allow => Ok(()),
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
        let (manifest, actions) = fixture.inputs("src/main.cjs", b"x");
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
