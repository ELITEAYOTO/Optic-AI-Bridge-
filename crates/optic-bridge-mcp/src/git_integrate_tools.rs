use std::sync::Arc;

use optic_bridge_core::{ActionEnvelope, ActionId, Effect, GitObjectId};
use optic_bridge_runtime::{
    AuthorizedGitIntegrationError, AuthorizedGitIntegrationService, GitIntegrationError,
    GitIntegrationMode, GitIntegrationResult, git_integration_resource_budget,
};
use rmcp::{ErrorData, Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::server::ReadonlyMcpServer;

#[tool_router(router = git_integrate_tool_router, vis = "pub")]
impl ReadonlyMcpServer {
    #[tool(
        name = "git_integrate",
        description = "Fast-forward the operator-owned internal Optic integration ref from an exact expected target commit to an exact source commit. Repository, target ref, Git executable, worktree path, lease and ActionId are application-owned and cannot be supplied by the caller."
    )]
    pub async fn git_integrate(
        &self,
        params: Parameters<GitIntegrateRequest>,
    ) -> Result<Json<GitIntegrateResponse>, ErrorData> {
        let source_head = parse_object_id(&params.0.source_head)?;
        let expected_target_head = parse_object_id(&params.0.expected_target_head)?;
        let effect = Effect::GitIntegrate {
            source_head,
            expected_target_head,
        };
        let envelope = self.git_integration_envelope(effect)?;
        let result = self
            .run_git_integration(move |service| service.integrate_fast_forward(&envelope))
            .await?;
        let response = integration_response(result);
        self.ensure_structured_payload_fits(&response)?;
        Ok(Json(response))
    }

    fn git_integration_envelope(&self, effect: Effect) -> Result<ActionEnvelope, ErrorData> {
        let now = self.clock.now();
        let grant = self.active_grant(now)?;
        let lease_id = self
            .git_integration_authorities
            .lease()
            .cloned()
            .ok_or_else(|| {
                ErrorData::invalid_request("optic.git_integration_authority_unavailable", None)
            })?;
        let action_id = ActionId::generate()
            .map_err(|_| ErrorData::internal_error("optic.action_id_unavailable", None))?;
        Ok(ActionEnvelope {
            action_id,
            session: self.session.clone(),
            task_lease: Some(lease_id),
            effect,
            resources: git_integration_resource_budget(self.limits),
            policy_epoch: grant.policy_epoch,
        })
    }

    async fn run_git_integration<T, F>(&self, operation: F) -> Result<T, ErrorData>
    where
        T: Send + 'static,
        F: FnOnce(Arc<AuthorizedGitIntegrationService>) -> Result<T, AuthorizedGitIntegrationError>
            + Send
            + 'static,
    {
        let permit = self
            .transport_guard
            .begin_execution(self.clock.now())
            .map_err(super::server::map_transport_error)?;
        let service = self
            .git_integration_service
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                ErrorData::invalid_request("optic.git_integration_authority_unavailable", None)
            })?;

        // Once a blocking Git mutation has started it cannot be safely cancelled
        // by dropping its JoinHandle. Keep the execution permit and follow the
        // operation to a known result. The Git runtime itself owns bounded command
        // deadlines and exact-head/cleanup semantics.
        let result = tokio::task::spawn_blocking(move || operation(service))
            .await
            .map_err(|_| ErrorData::internal_error("optic.runtime_join_failed", None))?
            .map_err(map_authorized_git_integration_error)?;
        drop(permit);
        Ok(result)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitIntegrateRequest {
    pub source_head: String,
    pub expected_target_head: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitIntegrateResponse {
    pub action_id: String,
    pub previous_target_head: String,
    pub new_target_head: String,
    pub mode: String,
}

fn parse_object_id(value: &str) -> Result<GitObjectId, ErrorData> {
    GitObjectId::parse(value.to_owned())
        .map_err(|_| ErrorData::invalid_params("optic.invalid_git_object_id", None))
}

fn integration_response(result: GitIntegrationResult) -> GitIntegrateResponse {
    let mode = match result.mode {
        GitIntegrationMode::FastForward => "fast_forward",
    };
    GitIntegrateResponse {
        action_id: result.action_id.to_token(),
        previous_target_head: result.previous_target_head.as_str().to_owned(),
        new_target_head: result.new_target_head.as_str().to_owned(),
        mode: mode.to_owned(),
    }
}

fn map_authorized_git_integration_error(error: AuthorizedGitIntegrationError) -> ErrorData {
    match error {
        AuthorizedGitIntegrationError::Session(_) => {
            ErrorData::invalid_request("optic.session_inactive", None)
        }
        AuthorizedGitIntegrationError::TaskLease(_)
        | AuthorizedGitIntegrationError::MissingTaskLease => {
            ErrorData::invalid_request("optic.git_integration_authority_inactive", None)
        }
        AuthorizedGitIntegrationError::PolicyDenied(_) => {
            ErrorData::invalid_request("optic.policy_denied", None)
        }
        AuthorizedGitIntegrationError::EffectMismatch => {
            ErrorData::internal_error("optic.git_integration_effect_mismatch", None)
        }
        AuthorizedGitIntegrationError::Integration(error) => map_git_integration_error(error),
    }
}

fn map_git_integration_error(error: GitIntegrationError) -> ErrorData {
    match error {
        GitIntegrationError::ObjectNotFound => {
            ErrorData::invalid_params("optic.git_integration_source_not_commit", None)
        }
        GitIntegrationError::StaleTarget { .. } => {
            ErrorData::invalid_request("optic.precondition_failed", None)
        }
        GitIntegrationError::NonFastForward => {
            ErrorData::invalid_request("optic.git_non_fast_forward", None)
        }
        GitIntegrationError::CommandTimedOut => {
            ErrorData::internal_error("optic.request_timeout", None)
        }
        GitIntegrationError::OperationPathAlreadyExists => {
            ErrorData::internal_error("optic.git_integration_action_active", None)
        }
        GitIntegrationError::WorktreeCleanupFailed
        | GitIntegrationError::WorktreeEscapedIntegrationRoot
        | GitIntegrationError::WorktreeHeadMismatch => {
            ErrorData::internal_error("optic.git_integration_recovery_required", None)
        }
        GitIntegrationError::PostUpdateVerificationUncertain
        | GitIntegrationError::PostUpdateVerificationFailed => {
            ErrorData::internal_error("optic.git_integration_outcome_uncertain", None)
        }
        GitIntegrationError::CommandOutputTooLarge => {
            ErrorData::internal_error("optic.git_output_too_large", None)
        }
        GitIntegrationError::InvalidLimits
        | GitIntegrationError::ReadBoundary(_)
        | GitIntegrationError::IntegrationRootMustBeAbsolute
        | GitIntegrationError::IntegrationRootNotDirectory
        | GitIntegrationError::IntegrationRootOverlapsRepository
        | GitIntegrationError::DisabledHooksDirectoryNotEmpty
        | GitIntegrationError::TargetRefOutsideOpticNamespace
        | GitIntegrationError::InvalidTargetRef
        | GitIntegrationError::TargetRefMissing
        | GitIntegrationError::SymbolicTargetRef
        | GitIntegrationError::InvalidObjectId
        | GitIntegrationError::WorktreeCreateFailed
        | GitIntegrationError::RecoveryWorktreeListMalformed
        | GitIntegrationError::RecoveryEntryLimitExceeded
        | GitIntegrationError::RecoveryOwnedLimitExceeded
        | GitIntegrationError::RecoveryOwnedPathMalformed
        | GitIntegrationError::RecoveryOwnedWorktreeMissing
        | GitIntegrationError::RecoveryOwnedWorktreeUnlocked
        | GitIntegrationError::RecoveryOwnedPathUnsafe
        | GitIntegrationError::RecoveryUnexpectedEntry
        | GitIntegrationError::RecoveryUnregisteredOwnedPath
        | GitIntegrationError::MissingChildPipe
        | GitIntegrationError::ReaderThreadPanicked
        | GitIntegrationError::GitCommandFailed
        | GitIntegrationError::EntropyUnavailable
        | GitIntegrationError::Io(_) => {
            ErrorData::internal_error("optic.git_integration_failed", None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        collections::{BTreeMap, BTreeSet},
        env,
        ffi::OsStr,
        fs,
        path::{Path, PathBuf},
        process::Command,
    };

    use optic_bridge_core::{
        Capability, HardLimits, MonotonicTime, PrincipalId, ProjectId, SessionGrant, SessionHandle,
    };
    use optic_bridge_runtime::{
        Clock, GitIntegrationAuthoritySet, GitIntegrationAuthoritySpec, GitIntegrationService,
        MutationAuthoritySet, ProcessManager, SessionRegistry, TaskLeaseRegistry,
        git_integration_resource_budget,
    };

    const TARGET_REF: &str = "refs/optic/integration/mcp";

    #[derive(Debug)]
    struct FixedClock(MonotonicTime);

    impl Clock for FixedClock {
        fn now(&self) -> MonotonicTime {
            self.0
        }
    }

    struct RepoFixture {
        base: PathBuf,
        repo: PathBuf,
        integration_root: PathBuf,
        git: PathBuf,
        initial: GitObjectId,
        source: GitObjectId,
    }

    impl RepoFixture {
        fn new(label: &str) -> Self {
            let git = find_git_executable().expect("git on CI PATH");
            let token = ActionId::generate().expect("test entropy").to_token();
            let base = env::temp_dir().join(format!("optic-mcp-integrate-{label}-{token}"));
            let repo = base.join("repo");
            let integration_root = base.join("integration");
            fs::create_dir_all(&repo).expect("repo dir");
            fs::create_dir_all(&integration_root).expect("integration dir");
            run_git(
                &git,
                None,
                [OsStr::new("init"), OsStr::new("--quiet"), repo.as_os_str()],
            );
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("config"),
                    OsStr::new("user.email"),
                    OsStr::new("optic@example.invalid"),
                ],
            );
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("config"),
                    OsStr::new("user.name"),
                    OsStr::new("Optic Test"),
                ],
            );
            fs::write(repo.join("tracked.txt"), b"alpha\n").expect("initial bytes");
            commit_all(&git, &repo, "initial");
            let initial = rev_parse(&git, &repo, "HEAD");
            run_git_owned(&git, &repo, ["update-ref", TARGET_REF, initial.as_str()]);

            fs::write(repo.join("tracked.txt"), b"beta\n").expect("source bytes");
            commit_all(&git, &repo, "source");
            let source = rev_parse(&git, &repo, "HEAD");

            Self {
                base,
                repo,
                integration_root,
                git,
                initial,
                source,
            }
        }

        fn server(&self) -> ReadonlyMcpServer {
            let limits = HardLimits::default();
            let now = MonotonicTime::from_millis(10);
            let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
            let session = SessionHandle::generate().expect("session entropy");
            let grant = SessionGrant {
                handle: session.clone(),
                principal: PrincipalId::new("test-principal").expect("principal"),
                project: ProjectId::new("test-project").expect("project"),
                capabilities: BTreeSet::from([Capability::GitIntegrate]),
                expires_at: MonotonicTime::from_millis(10_000),
                policy_epoch: 1,
            };
            let sessions = Arc::new(SessionRegistry::new());
            sessions.register(grant).expect("register session");
            let leases = Arc::new(TaskLeaseRegistry::new());
            let authority = GitIntegrationAuthoritySet::provision(
                &leases,
                &session,
                GitIntegrationAuthoritySpec { enabled: true },
                git_integration_resource_budget(limits),
                MonotonicTime::from_millis(10_000),
                1,
            )
            .expect("integration authority");
            let integration = GitIntegrationService::from_hard_limits(
                &self.repo,
                &self.git,
                &self.integration_root,
                TARGET_REF,
                limits,
            )
            .expect("integration runtime");
            let service = Arc::new(AuthorizedGitIntegrationService::from_runtime(
                integration,
                Arc::clone(&sessions),
                Arc::clone(&leases),
                Arc::clone(&clock),
            ));
            let processes = Arc::new(
                ProcessManager::new(&self.repo, limits, Vec::new()).expect("process runtime"),
            );
            ReadonlyMcpServer::new_with_git_integration_runtime(
                &self.repo,
                sessions,
                session,
                clock,
                limits,
                processes,
                leases,
                BTreeMap::new(),
                None,
                MutationAuthoritySet::default(),
                None,
                Some(service),
                authority,
            )
            .expect("server")
        }
    }

    impl Drop for RepoFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn find_git_executable() -> Option<PathBuf> {
        let path = env::var_os("PATH")?;
        for directory in env::split_paths(&path) {
            #[cfg(windows)]
            let candidate = directory.join("git.exe");
            #[cfg(not(windows))]
            let candidate = directory.join("git");
            if candidate.is_file() {
                return fs::canonicalize(candidate).ok();
            }
        }
        None
    }

    fn run_git<const N: usize>(git: &Path, repo: Option<&Path>, args: [&OsStr; N]) {
        let mut command = Command::new(git);
        if let Some(repo) = repo {
            command.arg("-C").arg(repo);
        }
        let status = command
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("Git fixture command");
        assert!(status.success(), "Git fixture command failed");
    }

    fn run_git_owned<const N: usize>(git: &Path, repo: &Path, args: [&str; N]) {
        let status = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("Git fixture command");
        assert!(status.success(), "Git fixture command failed");
    }

    fn commit_all(git: &Path, repo: &Path, message: &str) {
        run_git(git, Some(repo), [OsStr::new("add"), OsStr::new(".")]);
        run_git(
            git,
            Some(repo),
            [
                OsStr::new("commit"),
                OsStr::new("--quiet"),
                OsStr::new("-m"),
                OsStr::new(message),
            ],
        );
    }

    fn rev_parse(git: &Path, repo: &Path, object: &str) -> GitObjectId {
        let output = Command::new(git)
            .arg("-C")
            .arg(repo)
            .args(["rev-parse", "--verify", object])
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("rev-parse");
        assert!(output.status.success());
        GitObjectId::parse(
            std::str::from_utf8(&output.stdout)
                .expect("utf8 oid")
                .trim()
                .to_owned(),
        )
        .expect("oid")
    }

    #[test]
    fn integration_authority_adds_only_git_integrate_not_git_read_tools() {
        let fixture = RepoFixture::new("surface");
        let server = fixture.server();
        let names = server.registered_tool_names();
        assert!(names.iter().any(|name| name == "git_integrate"));
        assert!(!names.iter().any(|name| name == "git_status"));
        assert!(!names.iter().any(|name| name == "git_diff"));
        assert!(!names.iter().any(|name| name == "git_log"));
    }

    #[tokio::test]
    async fn git_integrate_fast_forwards_internal_ref_with_server_owned_action() {
        let fixture = RepoFixture::new("success");
        let server = fixture.server();
        let response = server
            .git_integrate(Parameters(GitIntegrateRequest {
                source_head: fixture.source.as_str().to_owned(),
                expected_target_head: fixture.initial.as_str().to_owned(),
            }))
            .await
            .expect("integration")
            .0;
        assert_eq!(response.previous_target_head, fixture.initial.as_str());
        assert_eq!(response.new_target_head, fixture.source.as_str());
        assert_eq!(response.mode, "fast_forward");
        assert!(ActionId::from_token(&response.action_id).is_ok());
        assert_eq!(
            rev_parse(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.source
        );
    }

    #[tokio::test]
    async fn stale_expected_target_fails_without_overwriting_current_target() {
        let fixture = RepoFixture::new("stale");
        run_git_owned(
            &fixture.git,
            &fixture.repo,
            ["update-ref", TARGET_REF, fixture.source.as_str()],
        );
        let server = fixture.server();
        let error = match server
            .git_integrate(Parameters(GitIntegrateRequest {
                source_head: fixture.source.as_str().to_owned(),
                expected_target_head: fixture.initial.as_str().to_owned(),
            }))
            .await
        {
            Ok(_) => panic!("stale target must fail"),
            Err(error) => error,
        };
        assert_eq!(error.message.as_ref(), "optic.precondition_failed");
        assert_eq!(
            rev_parse(&fixture.git, &fixture.repo, TARGET_REF),
            fixture.source
        );
    }

    #[test]
    fn request_rejects_caller_controlled_extra_authority_fields() {
        let value = serde_json::json!({
            "source_head": "a".repeat(40),
            "expected_target_head": "b".repeat(40),
            "target_ref": "refs/heads/main",
            "task_lease": "caller-controlled"
        });
        assert!(serde_json::from_value::<GitIntegrateRequest>(value).is_err());
    }

    #[test]
    fn public_request_accepts_only_exact_git_object_ids() {
        assert!(parse_object_id(&"a".repeat(40)).is_ok());
        assert!(parse_object_id(&"b".repeat(64)).is_ok());
        assert!(parse_object_id("HEAD").is_err());
        assert!(parse_object_id(&"g".repeat(40)).is_err());
    }

    #[test]
    fn stale_and_non_fast_forward_are_public_precondition_errors() {
        let expected = GitObjectId::parse("1".repeat(40)).expect("oid");
        let observed = GitObjectId::parse("2".repeat(40)).expect("oid");
        let stale =
            map_git_integration_error(GitIntegrationError::StaleTarget { expected, observed });
        assert_eq!(stale.message.as_ref(), "optic.precondition_failed");

        let non_ff = map_git_integration_error(GitIntegrationError::NonFastForward);
        assert_eq!(non_ff.message.as_ref(), "optic.git_non_fast_forward");
    }

    #[test]
    fn post_update_verification_never_looks_like_safe_retry() {
        for error in [
            GitIntegrationError::PostUpdateVerificationUncertain,
            GitIntegrationError::PostUpdateVerificationFailed,
        ] {
            let mapped = map_git_integration_error(error);
            assert_eq!(
                mapped.message.as_ref(),
                "optic.git_integration_outcome_uncertain"
            );
        }
    }
}
