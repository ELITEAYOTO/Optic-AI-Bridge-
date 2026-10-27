use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use optic_bridge_core::{
    ActionEnvelope, ActionId, Effect, GitObjectId, ResourceBudget, SessionGrant, WorkspacePath,
};
use optic_bridge_policy::PolicyDecision;
use optic_bridge_runtime::{GitLogCursor, GitReadError, GitReadService};
use rmcp::{ErrorData, Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::server::ReadonlyMcpServer;

const DEFAULT_GIT_DIFF_BYTES: u64 = 64 * 1024;
const DEFAULT_GIT_LOG_ENTRIES: u32 = 32;

#[tool_router(router = git_read_tool_router, vis = "pub")]
impl ReadonlyMcpServer {
    #[tool(
        name = "git_status",
        description = "Return bounded Git porcelain-v2 status for the operator-owned repository. Raw porcelain bytes are base64; the caller cannot choose a repository or Git executable."
    )]
    pub async fn git_status(&self) -> Result<Json<GitStatusResponse>, ErrorData> {
        let max_bytes = self.max_mcp_git_read_bytes();
        let snapshot = self
            .run_git_read_checked(
                move |service| service.status_with_max_bytes(max_bytes),
                |server, grant, snapshot| {
                    for path in &snapshot.sensitive_paths {
                        server.ensure_sensitive_metadata_authorized(
                            grant,
                            path,
                            server.clock.now(),
                        )?;
                    }
                    Ok(())
                },
            )
            .await?;
        let response = GitStatusResponse {
            head: snapshot.head.map(|value| value.as_str().to_owned()),
            encoding: "base64".to_owned(),
            porcelain_v2_base64: STANDARD.encode(snapshot.porcelain_v2),
        };
        self.ensure_structured_payload_fits(&response)?;
        Ok(Json(response))
    }

    #[tool(
        name = "git_diff",
        description = "Return a bounded literal-path Git diff from the operator-owned repository. External diff/textconv and submodule traversal are disabled."
    )]
    pub async fn git_diff(
        &self,
        params: Parameters<GitDiffRequest>,
    ) -> Result<Json<GitDiffResponse>, ErrorData> {
        let path = params
            .0
            .path
            .as_deref()
            .map(parse_workspace_path)
            .transpose()?;
        let staged = params.0.staged.unwrap_or(false);
        let max_mcp_bytes = self.max_mcp_git_read_bytes();
        let max_bytes = params
            .0
            .max_bytes
            .unwrap_or(DEFAULT_GIT_DIFF_BYTES.min(max_mcp_bytes));
        if max_bytes == 0 || max_bytes > max_mcp_bytes {
            return Err(ErrorData::invalid_params(
                "optic.git_read_limit_exceeded",
                None,
            ));
        }

        let diff = self
            .run_git_read_checked(
                move |service| service.diff(path.as_ref(), staged, Some(max_bytes)),
                |server, grant, diff| {
                    for path in &diff.sensitive_paths {
                        // A patch discloses content, not only a filename.
                        // Do not let GitRead substitute for either FileRead
                        // or its matching ReadContent + ReadSensitive lease.
                        server.authorize(
                            grant,
                            Effect::FileRead { path: path.clone() },
                            server.clock.now(),
                        )?;
                    }
                    Ok(())
                },
            )
            .await?;
        let response = GitDiffResponse {
            encoding: "base64".to_owned(),
            data_base64: STANDARD.encode(diff.bytes),
        };
        self.ensure_structured_payload_fits(&response)?;
        Ok(Json(response))
    }

    #[tool(
        name = "git_log",
        description = "Return a bounded page of commit metadata. Cursors remain pinned to their original reachable HEAD and fail closed after incompatible history rewrites."
    )]
    pub async fn git_log(
        &self,
        params: Parameters<GitLogRequest>,
    ) -> Result<Json<GitLogResponse>, ErrorData> {
        let cursor = params.0.cursor.map(parse_log_cursor).transpose()?;
        let start_offset = cursor.as_ref().map(|value| value.offset).unwrap_or(0);
        let limit = params.0.limit.unwrap_or(DEFAULT_GIT_LOG_ENTRIES);
        if limit == 0 || limit > self.limits.max_git_log_entries {
            return Err(ErrorData::invalid_params(
                "optic.git_log_limit_exceeded",
                None,
            ));
        }

        let page = self
            .run_git_read(move |service| service.log(cursor.as_ref(), limit))
            .await?;
        let mut response = GitLogResponse {
            snapshot_head: page
                .snapshot_head
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            entries: page
                .entries
                .into_iter()
                .map(|entry| GitLogEntryResponse {
                    id: entry.id.as_str().to_owned(),
                    parents: entry
                        .parents
                        .into_iter()
                        .map(|parent| parent.as_str().to_owned())
                        .collect(),
                    commit_time_unix_seconds: entry.commit_time_unix_seconds,
                })
                .collect(),
            next_cursor: page.next_cursor.map(cursor_response),
        };
        self.fit_git_log_response(start_offset, &mut response)?;
        Ok(Json(response))
    }

    async fn run_git_read<T, F>(&self, operation: F) -> Result<T, ErrorData>
    where
        T: Send + 'static,
        F: FnOnce(Arc<GitReadService>) -> Result<T, GitReadError> + Send + 'static,
    {
        self.run_git_read_checked(operation, |_, _, _| Ok(())).await
    }

    /// Validate sensitive metadata before dropping admission and returning
    /// bytes to the caller. Revocation must drain this entire decision.
    async fn run_git_read_checked<T, F, V>(&self, operation: F, verify: V) -> Result<T, ErrorData>
    where
        T: Send + 'static,
        F: FnOnce(Arc<GitReadService>) -> Result<T, GitReadError> + Send + 'static,
        V: FnOnce(&Self, &SessionGrant, &T) -> Result<(), ErrorData> + Send + 'static,
    {
        self.run_owned_admitted_read(move |server, admission, permit| {
            let grant = admission.grant();
            server.ensure_read_deadline(&permit)?;
            server.authorize_git_read(grant, server.clock.now())?;
            let service = server.git_service.as_ref().cloned().ok_or_else(|| {
                ErrorData::invalid_request("optic.git_read_authority_unavailable", None)
            })?;
            let output = operation(service).map_err(map_git_read_error)?;
            server.ensure_read_deadline(&permit)?;
            verify(server, grant, &output)?;
            Ok(output)
        })
        .await
    }

    fn authorize_git_read(
        &self,
        grant: &SessionGrant,
        now: optic_bridge_core::MonotonicTime,
    ) -> Result<(), ErrorData> {
        let action_id = ActionId::generate()
            .map_err(|_| ErrorData::internal_error("optic.action_id_unavailable", None))?;
        let envelope = ActionEnvelope {
            action_id,
            session: self.session.clone(),
            task_lease: None,
            effect: Effect::GitRead,
            resources: ResourceBudget {
                timeout_ms: self.limits.max_request_duration_ms,
                output_bytes: self.limits.max_response_bytes,
                memory_bytes: self.limits.max_active_output_ram_bytes,
                process_count: 1,
            },
            policy_epoch: grant.policy_epoch,
        };
        match self.policy.evaluate(&envelope, grant, None, now) {
            PolicyDecision::Allow => Ok(()),
            PolicyDecision::RequireApproval(_) | PolicyDecision::Deny(_) => {
                Err(ErrorData::invalid_request("optic.policy_denied", None))
            }
        }
    }

    fn max_mcp_git_read_bytes(&self) -> u64 {
        let encoded_budget = self
            .structured_payload_budget()
            .saturating_sub(super::server::STRUCTURED_VALUE_RESERVE_BYTES);
        let binary_budget = encoded_budget.saturating_div(4).saturating_mul(3);
        self.limits.max_git_read_bytes.min(binary_budget)
    }

    fn fit_git_log_response(
        &self,
        start_offset: u64,
        response: &mut GitLogResponse,
    ) -> Result<(), ErrorData> {
        let original_len = response.entries.len();
        while self.ensure_structured_payload_fits(response).is_err() && !response.entries.is_empty()
        {
            response.entries.pop();
        }
        self.ensure_structured_payload_fits(response)?;

        if response.entries.len() < original_len {
            let head = response.snapshot_head.clone().ok_or_else(|| {
                ErrorData::internal_error("optic.git_log_cursor_missing_head", None)
            })?;
            // The original last page can have no cursor. Adding one after
            // trimming increases the response size, so fit the final cursor
            // and entries together. A page must retain forward progress.
            loop {
                if response.entries.is_empty() {
                    return Err(ErrorData::internal_error("optic.response_too_large", None));
                }
                let kept = u64::try_from(response.entries.len())
                    .map_err(|_| ErrorData::internal_error("optic.response_too_large", None))?;
                let offset = start_offset
                    .checked_add(kept)
                    .ok_or_else(|| ErrorData::internal_error("optic.response_too_large", None))?;
                response.next_cursor = Some(GitLogCursorResponse {
                    head: head.clone(),
                    offset,
                });
                if self.ensure_structured_payload_fits(response).is_ok() {
                    break;
                }
                response.entries.pop();
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitDiffRequest {
    pub path: Option<String>,
    pub staged: Option<bool>,
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitStatusResponse {
    pub head: Option<String>,
    pub encoding: String,
    pub porcelain_v2_base64: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitDiffResponse {
    pub encoding: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitLogRequest {
    pub cursor: Option<GitLogCursorRequest>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitLogCursorRequest {
    pub head: String,
    pub offset: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitLogResponse {
    pub snapshot_head: Option<String>,
    pub entries: Vec<GitLogEntryResponse>,
    pub next_cursor: Option<GitLogCursorResponse>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitLogEntryResponse {
    pub id: String,
    pub parents: Vec<String>,
    pub commit_time_unix_seconds: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitLogCursorResponse {
    pub head: String,
    pub offset: u64,
}

fn parse_workspace_path(value: &str) -> Result<WorkspacePath, ErrorData> {
    WorkspacePath::parse(value)
        .map_err(|_| ErrorData::invalid_params("optic.invalid_workspace_path", None))
}

fn parse_log_cursor(value: GitLogCursorRequest) -> Result<GitLogCursor, ErrorData> {
    let head = GitObjectId::parse(value.head)
        .map_err(|_| ErrorData::invalid_params("optic.git_log_cursor_invalid", None))?;
    Ok(GitLogCursor {
        head,
        offset: value.offset,
    })
}

fn cursor_response(value: GitLogCursor) -> GitLogCursorResponse {
    GitLogCursorResponse {
        head: value.head.as_str().to_owned(),
        offset: value.offset,
    }
}

fn map_git_read_error(error: GitReadError) -> ErrorData {
    match error {
        GitReadError::ZeroRequestLimit
        | GitReadError::ReadByteLimitExceeded { .. }
        | GitReadError::LogEntryLimitExceeded { .. } => {
            ErrorData::invalid_params("optic.git_read_limit_exceeded", None)
        }
        GitReadError::LogCursorNotReachable | GitReadError::ObjectId(_) => {
            ErrorData::invalid_params("optic.git_log_cursor_invalid", None)
        }
        GitReadError::OutputLimitExceeded { .. } => {
            ErrorData::internal_error("optic.git_output_too_large", None)
        }
        GitReadError::CommandTimedOut => ErrorData::internal_error("optic.request_timeout", None),
        GitReadError::UnverifiableStatus => ErrorData::invalid_request("optic.policy_denied", None),
        GitReadError::UnverifiableDiff => {
            ErrorData::invalid_request("optic.git_diff_unverifiable", None)
        }
        GitReadError::InvalidLimits
        | GitReadError::GitExecutableMustBeAbsolute
        | GitReadError::GitExecutableNotFile
        | GitReadError::RepositoryRootNotDirectory
        | GitReadError::RepositoryRootMismatch
        | GitReadError::NonUtf8RepositoryRoot
        | GitReadError::CommandFailed { .. }
        | GitReadError::MissingChildPipe
        | GitReadError::ReaderThreadPanicked
        | GitReadError::InvalidObjectId
        | GitReadError::InvalidLogRecord
        | GitReadError::Io(_) => ErrorData::internal_error("optic.git_read_error", None),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet},
        env,
        ffi::OsStr,
        fs,
        path::{Path, PathBuf},
        process::Command,
        sync::Arc,
    };

    use optic_bridge_core::{
        Capability, HardLimits, MonotonicTime, PrincipalId, ProjectId, ReadContentScope,
        ReadSensitiveScope, SessionGrant, SessionHandle,
    };
    use optic_bridge_runtime::{
        Clock, MutationAuthoritySet, ProcessManager, ReadAuthoritySet, ReadAuthoritySpec,
        SessionRegistry, TaskLeaseRegistry, read_authority_resource_budget,
    };

    use super::*;

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
        git: PathBuf,
    }

    impl RepoFixture {
        fn new(label: &str) -> Self {
            let git = find_git_executable().expect("git on CI PATH");
            let token = ActionId::generate().expect("test entropy").to_token();
            let base = env::temp_dir().join(format!("optic-mcp-git-{label}-{token}"));
            let repo = base.join("repo");
            fs::create_dir_all(&repo).expect("repo dir");
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
            fs::write(repo.join("tracked.txt"), b"alpha\n").expect("fixture");
            run_git(&git, Some(&repo), [OsStr::new("add"), OsStr::new(".")]);
            run_git(
                &git,
                Some(&repo),
                [
                    OsStr::new("commit"),
                    OsStr::new("--quiet"),
                    OsStr::new("-m"),
                    OsStr::new("initial"),
                ],
            );
            Self { base, repo, git }
        }

        fn server(&self, capabilities: &[Capability]) -> ReadonlyMcpServer {
            let limits = HardLimits::default();
            let clock: Arc<dyn Clock> = Arc::new(FixedClock(MonotonicTime::from_millis(10)));
            let session = SessionHandle::generate().expect("session entropy");
            let grant = SessionGrant {
                handle: session.clone(),
                principal: PrincipalId::new("test-principal").expect("principal"),
                project: ProjectId::new("test-project").expect("project"),
                capabilities: capabilities.iter().copied().collect::<BTreeSet<_>>(),
                expires_at: MonotonicTime::from_millis(10_000),
                policy_epoch: 1,
            };
            let sessions = Arc::new(SessionRegistry::new());
            sessions.register(grant).expect("register session");
            let processes = Arc::new(
                ProcessManager::new(&self.repo, limits, Vec::new()).expect("process manager"),
            );
            let git_service = Arc::new(
                GitReadService::from_hard_limits(&self.repo, &self.git, limits)
                    .expect("git read service"),
            );
            ReadonlyMcpServer::new_with_git_runtime(
                &self.repo,
                sessions,
                session,
                clock,
                limits,
                processes,
                Arc::new(TaskLeaseRegistry::new()),
                BTreeMap::new(),
                None,
                MutationAuthoritySet::default(),
                Some(git_service),
            )
            .expect("server")
        }

        fn server_with_sensitive_read(&self, scope: ReadSensitiveScope) -> ReadonlyMcpServer {
            let server = self.server(&[Capability::FileRead, Capability::GitRead]);
            let limits = HardLimits::default();
            let spec = ReadAuthoritySpec {
                search_metadata: BTreeSet::new(),
                read_content: BTreeSet::from([ReadContentScope::all()]),
                read_sensitive: BTreeSet::from([scope]),
            };
            let read = ReadAuthoritySet::provision(
                &server.task_leases,
                &server.session,
                &spec,
                read_authority_resource_budget(limits),
                MonotonicTime::from_millis(10_000),
                1,
            )
            .expect("explicit scoped authority");
            server.with_read_authorities(read)
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
            .status()
            .expect("run Git fixture command");
        assert!(status.success(), "Git fixture command failed");
    }

    #[test]
    fn trimmed_git_log_refits_the_new_cursor_before_returning() {
        let fixture = RepoFixture::new("log-cursor-budget");
        let mut server = fixture.server(&[Capability::GitRead]);
        let head = "a".repeat(40);
        let entry = || GitLogEntryResponse {
            id: "b".repeat(40),
            parents: vec!["c".repeat(40), "d".repeat(40)],
            commit_time_unix_seconds: 1,
        };
        let one_without_cursor = GitLogResponse {
            snapshot_head: Some(head.clone()),
            entries: vec![entry()],
            next_cursor: None,
        };
        let one_with_cursor = GitLogResponse {
            snapshot_head: Some(head.clone()),
            entries: vec![entry()],
            next_cursor: Some(GitLogCursorResponse {
                head: head.clone(),
                offset: 1,
            }),
        };
        let without_bytes = serde_json::to_vec(&one_without_cursor)
            .expect("one entry without cursor")
            .len() as u64;
        let with_bytes = serde_json::to_vec(&one_with_cursor)
            .expect("one entry and continuation cursor")
            .len() as u64;
        assert!(with_bytes > without_bytes, "new cursor consumes more bytes");

        let overhead = server.limits.max_response_bytes - server.structured_payload_budget();
        let two_entries = || GitLogResponse {
            snapshot_head: Some(head.clone()),
            entries: vec![entry(), entry()],
            next_cursor: None,
        };
        assert!(
            serde_json::to_vec(&two_entries())
                .expect("two entries")
                .len() as u64
                > with_bytes,
            "two entries must exceed even the cursor-aware budget"
        );

        server.limits.max_response_bytes = overhead + without_bytes;
        let mut insufficient = two_entries();
        let error = server
            .fit_git_log_response(0, &mut insufficient)
            .expect_err("a cursor must not push an otherwise fitting page over budget");
        assert_eq!(error.message, "optic.response_too_large");

        server.limits.max_response_bytes = overhead + with_bytes;
        let mut sufficient = two_entries();
        server
            .fit_git_log_response(0, &mut sufficient)
            .expect("one entry with its cursor fits");
        assert_eq!(sufficient.entries.len(), 1);
        assert_eq!(
            sufficient
                .next_cursor
                .as_ref()
                .expect("continuation")
                .offset,
            1
        );
        assert_eq!(
            sufficient.next_cursor.as_ref().expect("continuation").head,
            head
        );
        server
            .ensure_structured_payload_fits(&sufficient)
            .expect("final serialized response respects the budget");

        let mut complete = one_without_cursor;
        server
            .fit_git_log_response(0, &mut complete)
            .expect("complete last page remains cursor-free");
        assert!(complete.next_cursor.is_none());
    }

    #[tokio::test]
    async fn git_status_routes_through_operator_owned_runtime() {
        let fixture = RepoFixture::new("status");
        fs::write(fixture.repo.join("tracked.txt"), b"changed\n").expect("modify fixture");
        let server = fixture.server(&[Capability::GitRead]);

        let response = server.git_status().await.expect("Git status").0;
        assert!(response.head.is_some());
        assert_eq!(response.encoding, "base64");
        let raw = STANDARD
            .decode(response.porcelain_v2_base64)
            .expect("base64 status");
        assert!(
            raw.windows(b"tracked.txt".len())
                .any(|window| window == b"tracked.txt")
        );
    }

    #[tokio::test]
    async fn git_status_does_not_expose_sensitive_staged_or_untracked_names() {
        let fixture = RepoFixture::new("sensitive-status");
        fs::write(fixture.repo.join(".env"), b"synthetic-only").expect("staged fixture");
        run_git(
            &fixture.git,
            Some(&fixture.repo),
            [OsStr::new("add"), OsStr::new(".env")],
        );
        fs::write(fixture.repo.join("access_token.txt"), b"synthetic-only")
            .expect("untracked fixture");
        let ordinary = fixture.server(&[Capability::GitRead]);
        assert_eq!(
            ordinary
                .git_status()
                .await
                .err()
                .expect("sensitive names must be denied")
                .message,
            "optic.policy_denied"
        );
        let mismatched = fixture.server_with_sensitive_read(ReadSensitiveScope::prefix(
            WorkspacePath::parse("src").expect("prefix"),
        ));
        assert_eq!(
            mismatched
                .git_status()
                .await
                .err()
                .expect("incorrect sensitive prefix must not authorize names")
                .message,
            "optic.policy_denied"
        );
        let permitted = fixture.server_with_sensitive_read(ReadSensitiveScope::all());
        let response = permitted
            .git_status()
            .await
            .expect("explicit sensitive authority")
            .0;
        let raw = STANDARD
            .decode(response.porcelain_v2_base64)
            .expect("status base64");
        assert!(raw.windows(b".env".len()).any(|value| value == b".env"));
        assert!(
            raw.windows(b"access_token.txt".len())
                .any(|value| value == b"access_token.txt")
        );
    }

    #[tokio::test]
    async fn direnv_private_filename_requires_extra_authority_for_status_and_diff() {
        let fixture = RepoFixture::new("direnv-sensitive-read");
        fs::write(fixture.repo.join(".envrc.private"), b"fixture first\n").expect("fixture");
        let ordinary = fixture.server(&[Capability::GitRead]);
        assert_eq!(
            ordinary
                .git_status()
                .await
                .err()
                .expect("direnv filename is sensitive metadata")
                .message,
            "optic.policy_denied"
        );
        let granted = fixture.server_with_sensitive_read(ReadSensitiveScope::all());
        let status = granted.git_status().await.expect("authorized status").0;
        let raw = STANDARD.decode(status.porcelain_v2_base64).expect("base64");
        assert!(
            raw.windows(b".envrc.private".len())
                .any(|part| part == b".envrc.private")
        );

        run_git(
            &fixture.git,
            Some(&fixture.repo),
            [OsStr::new("add"), OsStr::new(".envrc.private")],
        );
        run_git(
            &fixture.git,
            Some(&fixture.repo),
            [
                OsStr::new("commit"),
                OsStr::new("--quiet"),
                OsStr::new("-m"),
                OsStr::new("synthetic direnv"),
            ],
        );
        fs::write(fixture.repo.join(".envrc.private"), b"fixture second\n")
            .expect("modified fixture");
        let request = || GitDiffRequest {
            path: Some(".envrc.private".to_owned()),
            staged: Some(false),
            max_bytes: Some(16 * 1024),
        };
        assert_eq!(
            ordinary
                .git_diff(Parameters(request()))
                .await
                .err()
                .expect("direnv patch requires content authority")
                .message,
            "optic.policy_denied"
        );
        let patch = granted
            .git_diff(Parameters(request()))
            .await
            .expect("authorized direnv patch")
            .0;
        assert!(
            STANDARD
                .decode(patch.data_base64)
                .expect("base64 patch")
                .windows(b"fixture second".len())
                .any(|part| part == b"fixture second")
        );
    }

    #[tokio::test]
    async fn git_diff_requires_sensitive_content_authority_but_preserves_ordinary_diff() {
        let fixture = RepoFixture::new("sensitive-diff");
        fs::write(fixture.repo.join(".env"), b"first dummy only\n").expect("fixture");
        run_git(
            &fixture.git,
            Some(&fixture.repo),
            [OsStr::new("add"), OsStr::new(".env")],
        );
        run_git(
            &fixture.git,
            Some(&fixture.repo),
            [
                OsStr::new("commit"),
                OsStr::new("--quiet"),
                OsStr::new("-m"),
                OsStr::new("fixture env"),
            ],
        );
        fs::write(fixture.repo.join(".env"), b"second dummy only\n").expect("change");
        fs::write(fixture.repo.join("tracked.txt"), b"ordinary change\n").expect("change");
        let request = |path: Option<&str>, staged: bool| GitDiffRequest {
            path: path.map(str::to_owned),
            staged: Some(staged),
            max_bytes: Some(16 * 1024),
        };
        let normal = fixture.server(&[Capability::GitRead]);
        assert_eq!(
            normal
                .git_diff(Parameters(request(None, false)))
                .await
                .err()
                .expect("whole repo with secret denied")
                .message,
            "optic.policy_denied"
        );
        assert_eq!(
            normal
                .git_diff(Parameters(request(Some(".env"), false)))
                .await
                .err()
                .expect("literal protected path denied")
                .message,
            "optic.policy_denied"
        );
        let source = normal
            .git_diff(Parameters(request(Some("tracked.txt"), false)))
            .await
            .expect("ordinary source diff remains accessible")
            .0;
        assert!(
            String::from_utf8_lossy(&STANDARD.decode(source.data_base64).expect("base64"))
                .contains("ordinary change")
        );
        let wrong_scope = fixture.server_with_sensitive_read(ReadSensitiveScope::prefix(
            WorkspacePath::parse("src").expect("prefix"),
        ));
        assert_eq!(
            wrong_scope
                .git_diff(Parameters(request(None, false)))
                .await
                .err()
                .expect("wrong prefix does not authorize")
                .message,
            "optic.policy_denied"
        );
        let granted = fixture.server_with_sensitive_read(ReadSensitiveScope::all());
        let patch = granted
            .git_diff(Parameters(request(None, false)))
            .await
            .expect("explicit protected content authority")
            .0;
        assert!(
            String::from_utf8_lossy(&STANDARD.decode(patch.data_base64).expect("base64"))
                .contains("second dummy only")
        );
    }

    #[tokio::test]
    async fn git_diff_staged_secret_is_denied_without_extra_lease() {
        let fixture = RepoFixture::new("staged-diff");
        fs::write(fixture.repo.join("credentials.key"), b"fixture key only\n").expect("fixture");
        run_git(
            &fixture.git,
            Some(&fixture.repo),
            [OsStr::new("add"), OsStr::new("credentials.key")],
        );
        let request = || GitDiffRequest {
            path: None,
            staged: Some(true),
            max_bytes: Some(16 * 1024),
        };
        let only_git = fixture.server(&[Capability::GitRead]);
        assert_eq!(
            only_git
                .git_diff(Parameters(request()))
                .await
                .err()
                .expect("staged protected addition refused")
                .message,
            "optic.policy_denied"
        );
        let authorized = fixture.server_with_sensitive_read(ReadSensitiveScope::all());
        let result = authorized
            .git_diff(Parameters(request()))
            .await
            .expect("explicit sensitive authority")
            .0;
        assert!(
            String::from_utf8_lossy(&STANDARD.decode(result.data_base64).expect("base64"))
                .contains("fixture key only")
        );
    }

    #[tokio::test]
    async fn git_runtime_does_not_bypass_missing_session_capability() {
        let fixture = RepoFixture::new("deny");
        let server = fixture.server(&[Capability::FileRead]);
        assert!(server.git_status().await.is_err());
    }

    #[tokio::test]
    async fn git_read_revoke_waits_for_the_authorized_worker() {
        use std::{sync::mpsc, time::Duration};

        let fixture = RepoFixture::new("read-drain");
        let server = fixture.server(&[Capability::GitRead]);
        let reader = server.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let request = tokio::spawn(async move {
            reader
                .run_git_read(move |service| {
                    entered_tx
                        .send(())
                        .expect("entered authorized Git operation");
                    release_rx
                        .recv_timeout(Duration::from_secs(5))
                        .expect("release Git");
                    service.status_with_max_bytes(4096)
                })
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), entered_rx)
            .await
            .expect("start deadline")
            .expect("start");
        // Dropping the caller must not drop admission in a still-running Git worker.
        request.abort();
        assert!(request.await.expect_err("cancelled").is_cancelled());
        let sessions = Arc::clone(&server.sessions);
        let session = server.session.clone();
        let revoke = tokio::task::spawn_blocking(move || sessions.revoke(&session));
        tokio::time::timeout(Duration::from_secs(5), async {
            while server
                .sessions
                .get_active(&server.session, server.clock.now())
                .is_ok()
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("marked revoked");
        assert!(
            !revoke.is_finished(),
            "Git worker participates in revoke quiescence"
        );
        release_tx.send(()).expect("release");
        tokio::time::timeout(Duration::from_secs(5), revoke)
            .await
            .expect("revoke deadline")
            .expect("revoke task")
            .expect("revoke");
        assert_eq!(
            server
                .git_status()
                .await
                .err()
                .expect("subsequent Git read denied")
                .message,
            "optic.session_inactive"
        );
    }
}
