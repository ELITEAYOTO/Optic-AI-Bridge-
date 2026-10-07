#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use optic_bridge_core::{
    Capability, HardLimits, LeaseScope, PrincipalId, ProcessExecutionClass, ProjectId, TaskLease,
    TaskLeaseId, WorkspacePath,
};
use optic_bridge_mcp::{BoundedJsonLineTransport, ReadonlyMcpServer};
use optic_bridge_runtime::{
    AuthorizedFileMutationService, AuthorizedGitIntegrationService, Clock,
    GitIntegrationAuthoritySet, GitIntegrationAuthoritySpec, GitIntegrationService, GitReadService,
    MutationAuthoritySet, MutationAuthoritySpec, ProcessManager, SessionGrantSpec,
    SessionLifecycleManager, SessionRegistry, StdClock, TaskLeaseRegistry,
    TransactionalFileService, git_integration_resource_budget, mutation_resource_budget,
};
use rmcp::ServiceExt;

const INITIAL_SESSION_TTL_MS: u64 = 30 * 60 * 1000;
const SESSION_REAP_INTERVAL_MS: u64 = 5_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = AppArgs::parse()?;
    let limits = HardLimits::default().validate_nonzero()?;
    let clock: Arc<dyn Clock> = Arc::new(StdClock::new());
    let now = clock.now();
    let expires_at = now.saturating_add_millis(INITIAL_SESSION_TTL_MS);
    let processes = Arc::new(ProcessManager::new(
        &args.workspace,
        limits,
        args.allowed_env.clone(),
    )?);
    let task_leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits)?);

    let canonical_process_executables =
        canonicalize_allowed_executables(&processes, &args.allowed_executables)?;

    let mutation_spec = MutationAuthoritySpec {
        write_scopes: args.write_scopes.clone(),
        delete_scopes: args.delete_scopes.clone(),
    };
    mutation_spec.validate()?;

    let git_service = args
        .git_executable
        .as_ref()
        .map(|git| GitReadService::from_hard_limits(&args.workspace, git, limits))
        .transpose()?
        .map(Arc::new);

    let git_integration_runtime = match (
        args.git_integration_executable.as_ref(),
        args.git_integration_root.as_ref(),
        args.git_integration_ref.as_ref(),
    ) {
        (Some(git), Some(integration_root), Some(target_ref)) => {
            let runtime = GitIntegrationService::from_hard_limits(
                &args.workspace,
                git,
                integration_root,
                target_ref.clone(),
                limits,
            )?;
            let report = runtime.recover_owned_worktrees()?;
            if report.removed_worktrees != 0 {
                eprintln!(
                    "Optic AI Bridge startup Git recovery removed {} owned worktree(s)",
                    report.removed_worktrees
                );
            }
            Some(runtime)
        }
        (None, None, None) => None,
        _ => {
            return Err(
                std::io::Error::other("invalid Git integration startup configuration").into(),
            );
        }
    };

    let mut capabilities = BTreeSet::from([Capability::FileRead, Capability::FileSearch]);
    if !canonical_process_executables.is_empty() {
        capabilities.insert(Capability::ProcessRun);
    }
    if !mutation_spec.write_scopes.is_empty() {
        capabilities.insert(Capability::FileWrite);
    }
    if !mutation_spec.delete_scopes.is_empty() {
        capabilities.insert(Capability::FileDelete);
    }
    if git_service.is_some() {
        capabilities.insert(Capability::GitRead);
    }
    if args.allow_git_integrate {
        capabilities.insert(Capability::GitIntegrate);
    }

    let sessions = Arc::new(SessionRegistry::from_hard_limits(limits)?);
    let lifecycle = Arc::new(SessionLifecycleManager::new(
        Arc::clone(&sessions),
        Arc::clone(&task_leases),
        Arc::clone(&processes),
    ));
    let grant = lifecycle.provision(
        SessionGrantSpec {
            principal: PrincipalId::new("local-stdio")
                .ok_or_else(|| std::io::Error::other("invalid local principal id"))?,
            project: ProjectId::new("local-workspace")
                .ok_or_else(|| std::io::Error::other("invalid local project id"))?,
            capabilities,
            expires_at,
            policy_epoch: 1,
        },
        now,
    )?;
    let session = grant.handle;

    let mut process_leases = BTreeMap::new();
    for (canonical, class) in canonical_process_executables {
        let id = TaskLeaseId::generate()
            .map_err(|_| std::io::Error::other("failed to create process task lease id"))?;
        task_leases.register(TaskLease {
            id: id.clone(),
            session: session.clone(),
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([LeaseScope::ProcessExecutable {
                executable: canonical.clone(),
                class,
            }]),
            resource_ceiling: limits.max_process_budget,
            expires_at,
            policy_epoch: 1,
        })?;
        process_leases.insert(canonical, id);
    }

    let mutation_authorities = MutationAuthoritySet::provision(
        &task_leases,
        &session,
        &mutation_spec,
        mutation_resource_budget(limits),
        expires_at,
        1,
    )?;

    let git_integration_authorities = GitIntegrationAuthoritySet::provision(
        &task_leases,
        &session,
        GitIntegrationAuthoritySpec {
            enabled: args.allow_git_integrate,
        },
        git_integration_resource_budget(limits),
        expires_at,
        1,
    )?;

    let git_integration_service = match (
        git_integration_runtime,
        git_integration_authorities.has_integrate(),
    ) {
        (Some(runtime), true) => Some(Arc::new(AuthorizedGitIntegrationService::from_runtime(
            runtime,
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            Arc::clone(&clock),
        ))),
        (Some(_), false) | (None, false) => None,
        (None, true) => {
            return Err(std::io::Error::other(
                "Git integration authority exists without an integration runtime",
            )
            .into());
        }
    };

    let mutation_service = if let Some(state_root) = &args.mutation_state_dir {
        let recovery =
            TransactionalFileService::from_hard_limits(&args.workspace, state_root, limits)?;
        let report = recovery.recover()?;
        if !report.is_empty() {
            eprintln!(
                "Optic AI Bridge startup recovery reconciled {} mutation record(s)",
                report.records.len()
            );
        }
        drop(recovery);
        Some(Arc::new(AuthorizedFileMutationService::from_hard_limits(
            &args.workspace,
            state_root,
            limits,
            Arc::clone(&sessions),
            Arc::clone(&task_leases),
            Arc::clone(&clock),
        )?))
    } else {
        None
    };

    let git_read_enabled = git_service.is_some();
    let server = ReadonlyMcpServer::new_with_git_integration_runtime(
        &args.workspace,
        sessions,
        session,
        Arc::clone(&clock),
        limits,
        processes,
        task_leases,
        process_leases,
        mutation_service,
        mutation_authorities,
        git_service,
        git_integration_service,
        git_integration_authorities,
    )?;
    let git_integrate_authority_ready = server.git_integration_authority_ready();

    let max_request_bytes = usize::try_from(limits.max_request_bytes)
        .map_err(|_| std::io::Error::other("request hard limit does not fit usize"))?;
    let max_response_bytes = usize::try_from(limits.max_response_bytes)
        .map_err(|_| std::io::Error::other("response hard limit does not fit usize"))?;
    let transport = BoundedJsonLineTransport::new(
        tokio::io::stdin(),
        tokio::io::stdout(),
        max_request_bytes,
        max_response_bytes,
    )?;

    eprintln!(
        "Optic AI Bridge {} — MCP stdio ({} process executable(s), {} write scope(s), {} delete scope(s), Git read {}, Git integrate authority {})",
        env!("CARGO_PKG_VERSION"),
        args.allowed_executables.len(),
        args.write_scopes.len(),
        args.delete_scopes.len(),
        if git_read_enabled {
            "enabled"
        } else {
            "disabled"
        },
        if git_integrate_authority_ready {
            "enabled"
        } else {
            "disabled"
        },
    );
    let service = server.serve(transport).await?;
    tokio::select! {
        result = service.waiting() => {
            result?;
        }
        result = supervise_session_expiry(Arc::clone(&lifecycle), Arc::clone(&clock)) => {
            result?;
            return Err(std::io::Error::other(
                "session expiry supervisor stopped unexpectedly",
            )
            .into());
        }
    }
    Ok(())
}

async fn supervise_session_expiry(
    lifecycle: Arc<SessionLifecycleManager>,
    clock: Arc<dyn Clock>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut interval = tokio::time::interval(Duration::from_millis(SESSION_REAP_INTERVAL_MS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let now = clock.now();
        let lifecycle = Arc::clone(&lifecycle);
        let removed_sessions = tokio::task::spawn_blocking(move || lifecycle.reap_inactive(now))
            .await
            .map_err(|error| {
                std::io::Error::other(format!("session expiry cleanup task failed: {error}"))
            })??;
        if removed_sessions != 0 {
            eprintln!(
                "Optic AI Bridge session expiry cleanup reaped {removed_sessions} session(s)"
            );
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AllowedExecutableSpec {
    class: ProcessExecutionClass,
    path: String,
}

fn canonicalize_allowed_executables(
    processes: &ProcessManager,
    executables: &[AllowedExecutableSpec],
) -> Result<BTreeMap<String, ProcessExecutionClass>, Box<dyn Error + Send + Sync>> {
    let mut canonical = BTreeMap::new();
    for executable in executables {
        let path = processes.canonicalize_executable(&executable.path)?;
        if canonical.insert(path, executable.class).is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "duplicate canonical --allow-executable path",
            )
            .into());
        }
    }
    Ok(canonical)
}

#[derive(Debug)]
struct AppArgs {
    workspace: PathBuf,
    allowed_executables: Vec<AllowedExecutableSpec>,
    allowed_env: Vec<String>,
    write_scopes: BTreeSet<LeaseScope>,
    delete_scopes: BTreeSet<LeaseScope>,
    mutation_state_dir: Option<PathBuf>,
    git_executable: Option<PathBuf>,
    git_integration_executable: Option<PathBuf>,
    allow_git_integrate: bool,
    git_integration_root: Option<PathBuf>,
    git_integration_ref: Option<String>,
}

impl AppArgs {
    fn parse() -> Result<Self, std::io::Error> {
        Self::parse_from(std::env::args_os().skip(1))
    }

    fn parse_from<I>(args: I) -> Result<Self, std::io::Error>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut workspace = None;
        let mut allowed_executables = Vec::new();
        let mut allowed_env = Vec::new();
        let mut write_scopes = BTreeSet::new();
        let mut delete_scopes = BTreeSet::new();
        let mut mutation_state_dir = None;
        let mut git_executable = None;
        let mut git_integration_executable = None;
        let mut allow_git_integrate = false;
        let mut git_integration_root = None;
        let mut git_integration_ref = None;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            if arg == "--allow-executable" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-executable requires <fixed-tool|interpreter|repository-code>:<absolute-path>",
                    )
                })?;
                allowed_executables.push(parse_allowed_executable(os_string_to_utf8(
                    value,
                    "classified executable authority",
                )?)?);
            } else if arg == "--allow-env" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-env requires a variable name",
                    )
                })?;
                allowed_env.push(os_string_to_utf8(value, "environment variable name")?);
            } else if arg == "--allow-write-scope" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-write-scope requires all or prefix:<workspace-path>",
                    )
                })?;
                write_scopes.insert(parse_mutation_scope(os_string_to_utf8(
                    value,
                    "write scope",
                )?)?);
            } else if arg == "--allow-delete-scope" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-delete-scope requires all or prefix:<workspace-path>",
                    )
                })?;
                delete_scopes.insert(parse_mutation_scope(os_string_to_utf8(
                    value,
                    "delete scope",
                )?)?);
            } else if arg == "--mutation-state-dir" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--mutation-state-dir requires an absolute path outside the workspace",
                    )
                })?;
                set_mutation_state_dir(&mut mutation_state_dir, PathBuf::from(value))?;
            } else if arg == "--git-executable" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--git-executable requires an absolute path",
                    )
                })?;
                set_git_executable(&mut git_executable, PathBuf::from(value))?;
            } else if arg == "--git-integration-executable" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--git-integration-executable requires an absolute path",
                    )
                })?;
                set_git_integration_executable(
                    &mut git_integration_executable,
                    PathBuf::from(value),
                )?;
            } else if arg == "--allow-git-integrate" {
                if allow_git_integrate {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-git-integrate may be supplied only once",
                    ));
                }
                allow_git_integrate = true;
            } else if arg == "--git-integration-root" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--git-integration-root requires an absolute path outside the repository",
                    )
                })?;
                set_git_integration_root(&mut git_integration_root, PathBuf::from(value))?;
            } else if arg == "--git-integration-ref" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--git-integration-ref requires refs/optic/integration/<name>",
                    )
                })?;
                set_git_integration_ref(
                    &mut git_integration_ref,
                    os_string_to_utf8(value, "Git integration ref")?,
                )?;
            } else if let Some(value) = option_value(&arg, "--allow-executable=")? {
                allowed_executables.push(parse_allowed_executable(value)?);
            } else if let Some(value) = option_value(&arg, "--allow-env=")? {
                allowed_env.push(value);
            } else if let Some(value) = option_value(&arg, "--allow-write-scope=")? {
                write_scopes.insert(parse_mutation_scope(value)?);
            } else if let Some(value) = option_value(&arg, "--allow-delete-scope=")? {
                delete_scopes.insert(parse_mutation_scope(value)?);
            } else if let Some(value) = option_value(&arg, "--mutation-state-dir=")? {
                set_mutation_state_dir(&mut mutation_state_dir, PathBuf::from(value))?;
            } else if let Some(value) = option_value(&arg, "--git-executable=")? {
                set_git_executable(&mut git_executable, PathBuf::from(value))?;
            } else if let Some(value) = option_value(&arg, "--git-integration-executable=")? {
                set_git_integration_executable(
                    &mut git_integration_executable,
                    PathBuf::from(value),
                )?;
            } else if let Some(value) = option_value(&arg, "--git-integration-root=")? {
                set_git_integration_root(&mut git_integration_root, PathBuf::from(value))?;
            } else if let Some(value) = option_value(&arg, "--git-integration-ref=")? {
                set_git_integration_ref(&mut git_integration_ref, value)?;
            } else if arg.to_string_lossy().starts_with('-') {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown option: {}", arg.to_string_lossy()),
                ));
            } else if workspace.replace(PathBuf::from(arg)).is_some() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "only one workspace path may be supplied",
                ));
            }
        }

        if (!write_scopes.is_empty() || !delete_scopes.is_empty()) && mutation_state_dir.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "mutation authority requires --mutation-state-dir",
            ));
        }
        let integration_parts = usize::from(git_integration_executable.is_some())
            + usize::from(git_integration_root.is_some())
            + usize::from(git_integration_ref.is_some());
        if integration_parts != 0 && integration_parts != 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Git integration recovery requires --git-integration-executable, --git-integration-root and --git-integration-ref together",
            ));
        }
        if allow_git_integrate && integration_parts != 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--allow-git-integrate requires --git-integration-executable, --git-integration-root and --git-integration-ref",
            ));
        }

        Ok(Self {
            workspace: workspace.unwrap_or(std::env::current_dir()?),
            allowed_executables,
            allowed_env,
            write_scopes,
            delete_scopes,
            mutation_state_dir,
            git_executable,
            git_integration_executable,
            allow_git_integrate,
            git_integration_root,
            git_integration_ref,
        })
    }
}

fn set_mutation_state_dir(
    slot: &mut Option<PathBuf>,
    value: PathBuf,
) -> Result<(), std::io::Error> {
    if !value.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--mutation-state-dir must be an absolute path",
        ));
    }
    if slot.replace(value).is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--mutation-state-dir may be supplied only once",
        ));
    }
    Ok(())
}

fn set_git_executable(slot: &mut Option<PathBuf>, value: PathBuf) -> Result<(), std::io::Error> {
    if !value.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--git-executable must be an absolute path",
        ));
    }
    if slot.replace(value).is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--git-executable may be supplied only once",
        ));
    }
    Ok(())
}

fn set_git_integration_executable(
    slot: &mut Option<PathBuf>,
    value: PathBuf,
) -> Result<(), std::io::Error> {
    if !value.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--git-integration-executable must be an absolute path",
        ));
    }
    if slot.replace(value).is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--git-integration-executable may be supplied only once",
        ));
    }
    Ok(())
}

fn set_git_integration_root(
    slot: &mut Option<PathBuf>,
    value: PathBuf,
) -> Result<(), std::io::Error> {
    if !value.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--git-integration-root must be an absolute path",
        ));
    }
    if slot.replace(value).is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--git-integration-root may be supplied only once",
        ));
    }
    Ok(())
}

fn set_git_integration_ref(slot: &mut Option<String>, value: String) -> Result<(), std::io::Error> {
    if value.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--git-integration-ref must not be empty",
        ));
    }
    if slot.replace(value).is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--git-integration-ref may be supplied only once",
        ));
    }
    Ok(())
}

fn parse_allowed_executable(value: String) -> Result<AllowedExecutableSpec, std::io::Error> {
    let Some((class, path)) = value.split_once(':') else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--allow-executable must use <fixed-tool|interpreter|repository-code>:<absolute-path>",
        ));
    };
    let class = match class {
        "fixed-tool" => ProcessExecutionClass::FixedTool,
        "interpreter" => ProcessExecutionClass::Interpreter,
        "repository-code" => ProcessExecutionClass::RepositoryCode,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "unknown --allow-executable class; expected fixed-tool, interpreter, or repository-code",
            ));
        }
    };
    if path.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--allow-executable path must not be empty",
        ));
    }
    Ok(AllowedExecutableSpec {
        class,
        path: path.to_owned(),
    })
}

fn parse_mutation_scope(value: String) -> Result<LeaseScope, std::io::Error> {
    if value == "all" {
        return Ok(LeaseScope::WorkspaceAll);
    }
    let Some(prefix) = value.strip_prefix("prefix:") else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "mutation scope must be all or prefix:<workspace-path>",
        ));
    };
    let path = WorkspacePath::parse(prefix).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "mutation scope prefix must be a safe project-relative workspace path",
        )
    })?;
    Ok(LeaseScope::WorkspacePrefix(path))
}

fn option_value(arg: &OsString, prefix: &str) -> Result<Option<String>, std::io::Error> {
    let value = arg.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "option must be valid UTF-8",
        )
    })?;
    Ok(value
        .strip_prefix(prefix)
        .map(str::to_owned)
        .filter(|value| !value.is_empty()))
}

fn os_string_to_utf8(value: OsString, label: &str) -> Result<String, std::io::Error> {
    value.into_string().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{label} must be valid UTF-8"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn mutation_authority_is_absent_without_operator_flags() {
        let parsed = AppArgs::parse_from(args(&["workspace"])).expect("args");
        assert!(parsed.write_scopes.is_empty());
        assert!(parsed.delete_scopes.is_empty());
        assert!(parsed.mutation_state_dir.is_none());
        assert!(parsed.git_executable.is_none());
        assert!(parsed.git_integration_executable.is_none());
        assert!(!parsed.allow_git_integrate);
        assert!(parsed.git_integration_root.is_none());
        assert!(parsed.git_integration_ref.is_none());
    }

    #[test]
    fn classified_process_authority_is_explicit_and_repeatable() {
        let parsed = AppArgs::parse_from(args(&[
            "--allow-executable=fixed-tool:/opt/tool",
            "--allow-executable",
            "interpreter:C:\\Tools\\python.exe",
            "--allow-executable=repository-code:/opt/cargo",
            "workspace",
        ]))
        .expect("classified process authority");

        assert_eq!(
            parsed.allowed_executables,
            vec![
                AllowedExecutableSpec {
                    class: ProcessExecutionClass::FixedTool,
                    path: "/opt/tool".to_owned(),
                },
                AllowedExecutableSpec {
                    class: ProcessExecutionClass::Interpreter,
                    path: "C:\\Tools\\python.exe".to_owned(),
                },
                AllowedExecutableSpec {
                    class: ProcessExecutionClass::RepositoryCode,
                    path: "/opt/cargo".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn unclassified_or_unknown_process_authority_fails_closed() {
        for value in [
            "C:\\Windows\\System32\\cmd.exe",
            "/usr/bin/python",
            "unknown:/opt/tool",
            "fixed-tool:",
        ] {
            assert!(
                AppArgs::parse_from(args(&[&format!("--allow-executable={value}"), "workspace"]))
                    .is_err(),
                "authority should fail: {value}"
            );
        }
    }

    #[test]
    fn duplicate_canonical_executable_cannot_receive_competing_classes() {
        let processes = ProcessManager::new(
            std::env::current_dir().expect("current directory"),
            HardLimits::default(),
            Vec::new(),
        )
        .expect("process manager");
        let executable = std::env::current_exe()
            .expect("current test executable")
            .to_string_lossy()
            .into_owned();
        let specs = vec![
            AllowedExecutableSpec {
                class: ProcessExecutionClass::FixedTool,
                path: executable.clone(),
            },
            AllowedExecutableSpec {
                class: ProcessExecutionClass::Interpreter,
                path: executable,
            },
        ];

        assert!(canonicalize_allowed_executables(&processes, &specs).is_err());
    }

    #[test]
    fn mutation_scope_flags_are_explicit_repeatable_and_require_state_dir() {
        let state = std::env::temp_dir().join("optic-mutation-state-test");
        let parsed = AppArgs::parse_from(vec![
            OsString::from("--allow-write-scope=prefix:src"),
            OsString::from("--allow-write-scope"),
            OsString::from("prefix:generated"),
            OsString::from("--allow-delete-scope=all"),
            OsString::from("--mutation-state-dir"),
            state.clone().into_os_string(),
            OsString::from("workspace"),
        ])
        .expect("args");

        assert_eq!(
            parsed.write_scopes,
            BTreeSet::from([
                LeaseScope::WorkspacePrefix(WorkspacePath::parse("generated").expect("path")),
                LeaseScope::WorkspacePrefix(WorkspacePath::parse("src").expect("path")),
            ])
        );
        assert_eq!(
            parsed.delete_scopes,
            BTreeSet::from([LeaseScope::WorkspaceAll])
        );
        assert_eq!(parsed.mutation_state_dir, Some(state));
    }

    #[test]
    fn mutation_scope_without_state_dir_fails_closed() {
        assert!(
            AppArgs::parse_from(args(&["--allow-write-scope=prefix:src", "workspace"])).is_err()
        );
    }

    #[test]
    fn mutation_state_dir_must_be_absolute_but_may_be_recovery_only() {
        assert!(
            AppArgs::parse_from(args(&["--mutation-state-dir=relative-state", "workspace"]))
                .is_err()
        );

        let state = std::env::temp_dir().join("optic-recovery-only-state");
        let parsed = AppArgs::parse_from(vec![
            OsString::from("--mutation-state-dir"),
            state.clone().into_os_string(),
            OsString::from("workspace"),
        ])
        .expect("recovery-only state dir");
        assert_eq!(parsed.mutation_state_dir, Some(state));
        assert!(parsed.write_scopes.is_empty());
        assert!(parsed.delete_scopes.is_empty());
    }

    #[test]
    fn mutation_scope_rejects_escape_absolute_and_unknown_forms() {
        for value in [
            "prefix:../secret",
            "prefix:/absolute",
            "prefix:C:\\Windows",
            "workspace",
            "*",
        ] {
            assert!(
                AppArgs::parse_from(args(&[
                    &format!("--allow-write-scope={value}"),
                    "workspace"
                ]))
                .is_err(),
                "scope should fail: {value}"
            );
        }
    }

    #[test]
    fn git_read_authority_is_absent_by_default() {
        let parsed = AppArgs::parse_from(args(&["workspace"])).expect("args");
        assert!(parsed.git_executable.is_none());
    }

    #[test]
    fn git_executable_must_be_absolute_and_unique() {
        assert!(AppArgs::parse_from(args(&["--git-executable=git", "workspace"])).is_err());

        let git = std::env::temp_dir().join("optic-git-placeholder");
        let parsed = AppArgs::parse_from(vec![
            OsString::from("--git-executable"),
            git.clone().into_os_string(),
            OsString::from("workspace"),
        ])
        .expect("absolute Git path is accepted at argument normalization");
        assert_eq!(parsed.git_executable, Some(git.clone()));

        assert!(
            AppArgs::parse_from(vec![
                OsString::from("--git-executable"),
                git.clone().into_os_string(),
                OsString::from("--git-executable"),
                git.into_os_string(),
                OsString::from("workspace"),
            ])
            .is_err()
        );
    }

    #[test]
    fn git_integration_recovery_only_requires_complete_explicit_config() {
        let git = std::env::temp_dir().join("optic-git-integration-placeholder");
        let root = std::env::temp_dir().join("optic-git-integration-state");
        let parsed = AppArgs::parse_from(vec![
            OsString::from("--git-integration-executable"),
            git.clone().into_os_string(),
            OsString::from("--git-integration-root"),
            root.clone().into_os_string(),
            OsString::from("--git-integration-ref=refs/optic/integration/default"),
            OsString::from("workspace"),
        ])
        .expect("recovery-only Git integration config");
        assert!(!parsed.allow_git_integrate);
        assert!(parsed.git_executable.is_none());
        assert!(parsed.git_integration_executable.is_some());
        assert_eq!(parsed.git_integration_root, Some(root));
        assert_eq!(
            parsed.git_integration_ref.as_deref(),
            Some("refs/optic/integration/default")
        );

        assert!(
            AppArgs::parse_from(vec![
                OsString::from("--git-integration-executable"),
                git.clone().into_os_string(),
                OsString::from("--git-integration-root"),
                std::env::temp_dir()
                    .join("optic-root-only")
                    .into_os_string(),
                OsString::from("workspace"),
            ])
            .is_err()
        );
        assert!(
            AppArgs::parse_from(vec![
                OsString::from("--git-integration-root"),
                std::env::temp_dir().join("optic-no-git").into_os_string(),
                OsString::from("--git-integration-ref=refs/optic/integration/default"),
                OsString::from("workspace"),
            ])
            .is_err()
        );
        assert!(
            AppArgs::parse_from(args(&[
                "--git-integration-executable=/absolute-placeholder",
                "--git-integration-root=relative-root",
                "--git-integration-ref=refs/optic/integration/default",
                "workspace",
            ]))
            .is_err()
        );
    }

    #[test]
    fn git_integrate_authority_requires_explicit_allow_and_recovery_config() {
        assert!(AppArgs::parse_from(args(&["--allow-git-integrate", "workspace"])).is_err());

        let git = std::env::temp_dir().join("optic-git-integrate-placeholder");
        let root = std::env::temp_dir().join("optic-git-integrate-root");
        let parsed = AppArgs::parse_from(vec![
            OsString::from("--allow-git-integrate"),
            OsString::from("--git-integration-executable"),
            git.into_os_string(),
            OsString::from("--git-integration-root"),
            root.clone().into_os_string(),
            OsString::from("--git-integration-ref"),
            OsString::from("refs/optic/integration/default"),
            OsString::from("workspace"),
        ])
        .expect("explicit integrate authority");
        assert!(parsed.allow_git_integrate);
        assert_eq!(parsed.git_integration_root, Some(root));
        assert_eq!(
            parsed.git_integration_ref.as_deref(),
            Some("refs/optic/integration/default")
        );
    }

    #[test]
    fn git_integrate_allow_flag_is_not_repeatable() {
        let git = std::env::temp_dir().join("optic-git-integrate-repeat");
        let root = std::env::temp_dir().join("optic-git-integrate-repeat-root");
        assert!(
            AppArgs::parse_from(vec![
                OsString::from("--allow-git-integrate"),
                OsString::from("--allow-git-integrate"),
                OsString::from("--git-integration-executable"),
                git.into_os_string(),
                OsString::from("--git-integration-root"),
                root.into_os_string(),
                OsString::from("--git-integration-ref=refs/optic/integration/default"),
                OsString::from("workspace"),
            ])
            .is_err()
        );
    }

    #[test]
    fn mutation_budget_is_bounded_by_transport_and_runtime_limits() {
        let limits = HardLimits::default();
        let budget = mutation_resource_budget(limits);
        assert_eq!(budget.timeout_ms, limits.max_request_duration_ms);
        assert_eq!(budget.output_bytes, limits.max_response_bytes);
        assert_eq!(budget.memory_bytes, limits.max_active_output_ram_bytes);
        assert_eq!(budget.process_count, 1);
    }
}
