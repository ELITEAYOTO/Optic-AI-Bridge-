//! Bounded same-workspace publication; no process write or new MCP authority.
use std::{collections::HashSet, path::Path, sync::Arc};

use optic_bridge_core::{
    ActionEnvelope, Effect, HardLimits, ProjectInputManifest, ProjectInputSpec, WorkspacePath,
};
use thiserror::Error;

use crate::{
    AuthorizedFileMutationError, AuthorizedFileMutationService, Clock, JournaledMutationCommit,
    ProjectInputCaptureError, ProjectInputCaptureService, SessionRegistry, TaskLeaseRegistry,
};

const MAX_PUBLICATIONS: usize = 32;

/// Trusted host composes authorized capture and journaled writes against the
/// same enrolled workspace. This does not accept a private-output root or confer
/// build/process permissions. Keep transport and blocking-I/O permits throughout.
pub struct AuthorizedArtifactPublicationService {
    capture: ProjectInputCaptureService,
    mutations: AuthorizedFileMutationService,
    max_write_bytes: u64,
}

#[derive(Debug)]
pub struct ArtifactPublicationReceipt {
    pub source: WorkspacePath,
    pub destination: WorkspacePath,
    pub mutation: JournaledMutationCommit,
}

/// A batch is deliberately not a multi-file transaction. Earlier receipts are
/// confirmed writes. At failure_index the write may require journal recovery;
/// never infer absence of an effect or retry the batch automatically.
#[derive(Debug)]
pub struct ArtifactPublicationProgress {
    pub receipts: Vec<ArtifactPublicationReceipt>,
    pub interruption: Option<ArtifactPublicationInterruption>,
}

#[derive(Debug)]
pub struct ArtifactPublicationInterruption {
    pub failure_index: usize,
    pub error: ArtifactPublicationError,
}

impl AuthorizedArtifactPublicationService {
    pub fn from_hard_limits(
        workspace: impl AsRef<Path>,
        state: impl AsRef<Path>,
        limits: HardLimits,
        sessions: Arc<SessionRegistry>,
        leases: Arc<TaskLeaseRegistry>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, ArtifactPublicationError> {
        Ok(Self {
            capture: ProjectInputCaptureService::new(
                &workspace,
                limits,
                sessions.clone(),
                leases.clone(),
                clock.clone(),
            )?,
            mutations: AuthorizedFileMutationService::from_hard_limits(
                workspace, state, limits, sessions, leases, clock,
            )?,
            max_write_bytes: limits.max_fs_mutation_bytes,
        })
    }

    /// Reads and writes are aligned with the sorted manifest. Validate every
    /// declared effect/owner/epoch/scope before reading a single source. Sources
    /// are sealed once; actual writes independently recheck their live grants.
    /// Err means no publication write was attempted. Once writing starts, return
    /// explicit progress even on interruption, without rollback or blind retry.
    pub fn publish(
        &self,
        manifest: ProjectInputManifest,
        reads: &[ActionEnvelope],
        writes: &[ActionEnvelope],
    ) -> Result<ArtifactPublicationProgress, ArtifactPublicationError> {
        let first = reads.first().ok_or(ArtifactPublicationError::Mapping)?;
        let (_admission, started) = self.capture.capture_admission(first)?;
        self.capture.preflight(&manifest, reads, started)?;
        let destinations = validate_mapping(&manifest, writes, self.max_write_bytes)?;
        for write in writes {
            if write.session != first.session || write.policy_epoch != first.policy_epoch {
                return Err(ArtifactPublicationError::Binding);
            }
            self.mutations.preflight_write(write)?;
        }
        let snapshot = self.capture.capture(manifest, reads)?;
        let mut progress = ArtifactPublicationProgress {
            receipts: Vec::new(),
            interruption: None,
        };
        for (index, ((input, write), destination)) in snapshot
            .manifest()
            .files()
            .iter()
            .zip(writes)
            .zip(destinations)
            .enumerate()
        {
            // Retain both sides of the consent: capture is not a permanent read
            // authorization after lease revocation/expiry during publication.
            let attempt = (|| {
                self.capture.authorize(&reads[index], started)?;
                let bytes = snapshot
                    .content(&input.path)
                    .expect("sealed exact manifest");
                Ok::<_, ArtifactPublicationError>(self.mutations.write(write, bytes)?)
            })();
            match attempt {
                Ok(mutation) => progress.receipts.push(ArtifactPublicationReceipt {
                    source: input.path.clone(),
                    destination,
                    mutation,
                }),
                Err(error) => {
                    progress.interruption = Some(ArtifactPublicationInterruption {
                        failure_index: index,
                        error,
                    });
                    break;
                }
            }
        }
        Ok(progress)
    }
}

fn validate_mapping(
    manifest: &ProjectInputManifest,
    writes: &[ActionEnvelope],
    max_write_bytes: u64,
) -> Result<Vec<WorkspacePath>, ArtifactPublicationError> {
    if writes.len() != manifest.files().len() || writes.len() > MAX_PUBLICATIONS {
        return Err(ArtifactPublicationError::Capacity);
    }
    let mut ids = HashSet::new();
    let mut destinations = Vec::with_capacity(writes.len());
    let mut target_specs = Vec::with_capacity(writes.len());
    for (input, write) in manifest.files().iter().zip(writes) {
        if input.size_bytes > max_write_bytes || !ids.insert(write.action_id.clone()) {
            return Err(ArtifactPublicationError::Capacity);
        }
        let Effect::FileWrite { path, .. } = &write.effect else {
            return Err(ArtifactPublicationError::Mapping);
        };
        let destination = path.as_str().to_ascii_lowercase();
        if manifest.files().iter().any(|source| {
            let source = source.path.as_str().to_ascii_lowercase();
            source == destination
                || source.starts_with(&(destination.clone() + "/"))
                || destination.starts_with(&(source + "/"))
        }) {
            return Err(ArtifactPublicationError::SourceOverlap);
        }
        destinations.push(path.clone());
        target_specs.push(ProjectInputSpec {
            path: path.clone(),
            version: input.version,
            size_bytes: input.size_bytes,
        });
    }
    // Reuse the portable bounded namespace rules (case/file-directory aliases,
    // device/control names). This data-only manifest is not a permission grant.
    ProjectInputManifest::new(
        writes[0].session.clone(),
        writes[0]
            .task_lease
            .clone()
            .ok_or(ArtifactPublicationError::Binding)?,
        writes[0].policy_epoch,
        target_specs,
    )
    .map_err(|_| ArtifactPublicationError::Mapping)?;
    Ok(destinations)
}

#[derive(Debug, Error)]
pub enum ArtifactPublicationError {
    #[error("artifact publication count, bytes or action identity bound exceeded")]
    Capacity,
    #[error("artifact publication requires distinct portable destination FileWrite effects")]
    Mapping,
    #[error("artifact publication owner/task/epoch mismatch")]
    Binding,
    #[error("artifact publication cannot overlap captured sources")]
    SourceOverlap,
    #[error("artifact capture authorization or version check failed: {0}")]
    Capture(#[from] ProjectInputCaptureError),
    #[error("artifact destination authorization or journaled write failed: {0}")]
    Mutation(#[from] AuthorizedFileMutationError),
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use optic_bridge_core::{
        ActionId, Capability, ContentVersion, ExpectedState, LeaseScope, MonotonicTime,
        PrincipalId, ProjectId, ReadContentScope, ResourceBudget, SessionGrant, SessionHandle,
        TaskLease, TaskLeaseId, WorkloadClass,
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicBool, Ordering},
    };

    struct Fixture {
        base: PathBuf,
        root: PathBuf,
        state: PathBuf,
        sessions: Arc<SessionRegistry>,
        leases: Arc<TaskLeaseRegistry>,
        session: SessionGrant,
        read: TaskLease,
        write: TaskLease,
    }
    fn path(p: &str) -> WorkspacePath {
        WorkspacePath::parse(p).unwrap()
    }
    fn budget() -> ResourceBudget {
        ResourceBudget {
            timeout_ms: 10_000,
            output_bytes: 64 * 1024,
            memory_bytes: 64 * 1024 * 1024,
            process_count: 1,
        }
    }
    struct FixedClock;
    impl Clock for FixedClock {
        fn now(&self) -> MonotonicTime {
            MonotonicTime::from_millis(100)
        }
    }
    impl Fixture {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!(
                "optic-artifact-publication-{}",
                ActionId::generate().unwrap().to_token()
            ));
            let root = base.join("workspace");
            let state = base.join("state");
            fs::create_dir_all(root.join("build")).unwrap();
            fs::create_dir(root.join("export")).unwrap();
            fs::create_dir(&state).unwrap();
            let sessions = Arc::new(SessionRegistry::new());
            let leases = Arc::new(TaskLeaseRegistry::new());
            let session = SessionGrant {
                handle: SessionHandle::generate().unwrap(),
                principal: PrincipalId::new("artifact-test").unwrap(),
                project: ProjectId::new("generated").unwrap(),
                capabilities: [Capability::FileRead, Capability::FileWrite]
                    .into_iter()
                    .collect(),
                expires_at: MonotonicTime::from_millis(60_000),
                policy_epoch: 1,
            };
            sessions.register(session.clone()).unwrap();
            let read = TaskLease {
                id: TaskLeaseId::generate().unwrap(),
                session: session.handle.clone(),
                capabilities: [Capability::FileRead].into_iter().collect(),
                scopes: [LeaseScope::ReadContent(ReadContentScope::all())]
                    .into_iter()
                    .collect(),
                workload_class: WorkloadClass::Standard,
                resource_ceiling: budget(),
                expires_at: session.expires_at,
                policy_epoch: 1,
            };
            let write = TaskLease {
                id: TaskLeaseId::generate().unwrap(),
                session: session.handle.clone(),
                capabilities: [Capability::FileWrite].into_iter().collect(),
                scopes: [LeaseScope::WorkspacePrefix(path("export"))]
                    .into_iter()
                    .collect(),
                workload_class: WorkloadClass::Standard,
                resource_ceiling: budget(),
                expires_at: session.expires_at,
                policy_epoch: 1,
            };
            leases.register(read.clone()).unwrap();
            leases.register(write.clone()).unwrap();
            Self {
                base,
                root,
                state,
                sessions,
                leases,
                session,
                read,
                write,
            }
        }
        fn action(&self, lease: &TaskLease, effect: Effect) -> ActionEnvelope {
            ActionEnvelope {
                action_id: ActionId::generate().unwrap(),
                session: self.session.handle.clone(),
                task_lease: Some(lease.id.clone()),
                policy_epoch: 1,
                effect,
                resources: budget(),
            }
        }
        fn plan(
            &self,
        ) -> (
            ProjectInputManifest,
            Vec<ActionEnvelope>,
            Vec<ActionEnvelope>,
        ) {
            let mut specs = Vec::new();
            let mut reads = Vec::new();
            let mut writes = Vec::new();
            for name in ["a.js", "b.js", "c.js"] {
                let source = path(&format!("build/{name}"));
                let bytes = format!("generated {name}\n");
                specs.push(ProjectInputSpec {
                    path: source.clone(),
                    version: ContentVersion::from_bytes(bytes.as_bytes()),
                    size_bytes: bytes.len() as u64,
                });
                reads.push(self.action(&self.read, Effect::FileRead { path: source }));
                writes.push(self.action(
                    &self.write,
                    Effect::FileWrite {
                        path: path(&format!("export/{name}")),
                        expected: ExpectedState::Absent,
                    },
                ));
            }
            (
                ProjectInputManifest::new(
                    self.session.handle.clone(),
                    self.read.id.clone(),
                    1,
                    specs,
                )
                .unwrap(),
                reads,
                writes,
            )
        }
        fn sources(&self) {
            for name in ["a.js", "b.js", "c.js"] {
                fs::write(
                    self.root.join("build").join(name),
                    format!("generated {name}\n"),
                )
                .unwrap();
            }
        }
        fn service(&self, clock: Arc<dyn Clock>) -> AuthorizedArtifactPublicationService {
            AuthorizedArtifactPublicationService::from_hard_limits(
                &self.root,
                &self.state,
                HardLimits::default(),
                self.sessions.clone(),
                self.leases.clone(),
                clock,
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let resolved = self.base.canonicalize().unwrap();
            assert_eq!(
                resolved.parent().unwrap(),
                std::env::temp_dir().canonicalize().unwrap()
            );
            assert!(
                resolved
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("optic-artifact-publication-")
            );
            fs::remove_dir_all(resolved).unwrap();
        }
    }

    #[test]
    fn artifact_publication_creates_and_replaces_only_expected_destinations() {
        let f = Fixture::new();
        f.sources();
        let service = f.service(Arc::new(FixedClock));
        let (manifest, reads, writes) = f.plan();
        let result = service.publish(manifest, &reads, &writes).unwrap();
        assert!(result.interruption.is_none());
        assert_eq!(result.receipts.len(), 3);
        for receipt in &result.receipts {
            assert_eq!(
                fs::read(f.root.join(receipt.destination.as_str())).unwrap(),
                fs::read(f.root.join(receipt.source.as_str())).unwrap()
            );
        }
        let (manifest, reads, mut writes) = f.plan();
        for write in &mut writes {
            let Effect::FileWrite { path, expected } = &mut write.effect else {
                unreachable!()
            };
            *expected = ExpectedState::Content(ContentVersion::from_bytes(
                &fs::read(f.root.join(path.as_str())).unwrap(),
            ));
        }
        assert!(
            service
                .publish(manifest, &reads, &writes)
                .unwrap()
                .interruption
                .is_none()
        );
    }

    #[test]
    fn artifact_publication_denies_all_declared_writes_before_missing_source_io() {
        let f = Fixture::new();
        let service = f.service(Arc::new(FixedClock));
        let (manifest, reads, mut writes) = f.plan();
        writes[2].effect = Effect::FileWrite {
            path: path("outside.js"),
            expected: ExpectedState::Absent,
        };
        assert!(matches!(
            service.publish(manifest, &reads, &writes),
            Err(ArtifactPublicationError::Mutation(_))
        ));
        assert!(!f.root.join("export/a.js").exists());
        let (manifest, reads, mut writes) = f.plan();
        writes[1].session = SessionHandle::generate().unwrap();
        assert!(matches!(
            service.publish(manifest, &reads, &writes),
            Err(ArtifactPublicationError::Binding)
        ));
        assert!(!f.root.join("export/a.js").exists());
    }

    #[test]
    fn artifact_publication_rejects_aliases_overlap_and_duplicate_action_ids() {
        let f = Fixture::new();
        let (manifest, _, mut writes) = f.plan();
        writes[1].effect = Effect::FileWrite {
            path: path("EXPORT/A.JS"),
            expected: ExpectedState::Absent,
        };
        assert!(matches!(
            validate_mapping(&manifest, &writes, 1024),
            Err(ArtifactPublicationError::Mapping)
        ));
        let (manifest, _, mut writes) = f.plan();
        writes[0].effect = Effect::FileWrite {
            path: path("build/a.js"),
            expected: ExpectedState::Absent,
        };
        assert!(matches!(
            validate_mapping(&manifest, &writes, 1024),
            Err(ArtifactPublicationError::SourceOverlap)
        ));
        let (manifest, _, mut writes) = f.plan();
        writes[1].action_id = writes[0].action_id.clone();
        assert!(matches!(
            validate_mapping(&manifest, &writes, 1024),
            Err(ArtifactPublicationError::Capacity)
        ));
        let (manifest, _, writes) = f.plan();
        assert!(matches!(
            validate_mapping(&manifest, &writes, 1),
            Err(ArtifactPublicationError::Capacity)
        ));
    }

    #[test]
    fn artifact_publication_stale_second_write_reports_partial_progress_without_rollback() {
        let f = Fixture::new();
        f.sources();
        fs::write(f.root.join("export/b.js"), b"operator edit").unwrap();
        let service = f.service(Arc::new(FixedClock));
        let (manifest, reads, writes) = f.plan();
        let result = service.publish(manifest, &reads, &writes).unwrap();
        assert_eq!(result.receipts.len(), 1);
        assert_eq!(result.interruption.unwrap().failure_index, 1);
        assert_eq!(
            fs::read(f.root.join("export/a.js")).unwrap(),
            b"generated a.js\n"
        );
        assert_eq!(
            fs::read(f.root.join("export/b.js")).unwrap(),
            b"operator edit"
        );
        assert!(!f.root.join("export/c.js").exists());
    }

    #[test]
    fn artifact_publication_stale_capture_never_starts_destination_writes() {
        let f = Fixture::new();
        f.sources();
        fs::write(f.root.join("build/b.js"), b"new build generation").unwrap();
        let service = f.service(Arc::new(FixedClock));
        let (manifest, reads, writes) = f.plan();
        assert!(matches!(
            service.publish(manifest, &reads, &writes),
            Err(ArtifactPublicationError::Capture(_))
        ));
        assert!(!f.root.join("export/a.js").exists());
    }

    #[test]
    fn artifact_publication_redirected_destination_cannot_escape_workspace() {
        let f = Fixture::new();
        f.sources();
        let outside = f.base.join("other");
        fs::create_dir(&outside).unwrap();
        fs::remove_dir(f.root.join("export")).unwrap();
        std::os::windows::fs::symlink_dir(&outside, f.root.join("export")).unwrap();
        let service = f.service(Arc::new(FixedClock));
        let (manifest, reads, writes) = f.plan();
        let progress = service.publish(manifest, &reads, &writes).unwrap();
        assert!(progress.receipts.is_empty());
        assert_eq!(progress.interruption.unwrap().failure_index, 0);
        assert!(!outside.join("a.js").exists());
    }

    struct RevokeAfterFirst {
        first: PathBuf,
        leases: Arc<TaskLeaseRegistry>,
        read: TaskLeaseId,
        done: AtomicBool,
    }
    impl Clock for RevokeAfterFirst {
        fn now(&self) -> MonotonicTime {
            if self.first.exists() && !self.done.swap(true, Ordering::AcqRel) {
                self.leases.revoke(&self.read).unwrap();
            }
            MonotonicTime::from_millis(100)
        }
    }
    #[test]
    fn artifact_publication_rechecks_read_consent_between_destination_effects() {
        let f = Fixture::new();
        f.sources();
        let service = f.service(Arc::new(RevokeAfterFirst {
            first: f.root.join("export/a.js"),
            leases: f.leases.clone(),
            read: f.read.id.clone(),
            done: AtomicBool::new(false),
        }));
        let (manifest, reads, writes) = f.plan();
        let result = service.publish(manifest, &reads, &writes).unwrap();
        assert_eq!(result.receipts.len(), 1);
        let failure = result.interruption.unwrap();
        assert_eq!(failure.failure_index, 1);
        assert!(matches!(
            failure.error,
            ArtifactPublicationError::Capture(_)
        ));
        assert!(!f.root.join("export/b.js").exists());
    }
}
