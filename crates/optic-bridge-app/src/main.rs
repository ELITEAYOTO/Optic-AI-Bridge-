#![forbid(unsafe_code)]

mod persistent_config;
mod tool_profile_config;

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use optic_bridge_core::{
    Capability, HardLimits, LeaseScope, MonotonicTime, PrincipalId, ProcessExecutionClass,
    ProjectId, ResourceBudget, SessionHandle, TaskLease, TaskLeaseId, ToolProfile, WorkloadClass,
    WorkspacePath,
};
use optic_bridge_mcp::{BoundedJsonLineTransport, ReadonlyMcpServer};
use optic_bridge_runtime::{
    ApprovalBroker, AuthorizedFileMutationService, AuthorizedGitIntegrationService, Clock,
    EnvironmentGrant, EnvironmentVariableClass, GitIntegrationAuthoritySet,
    GitIntegrationAuthoritySpec, GitIntegrationService, GitReadService, MutationAuthoritySet,
    MutationAuthoritySpec, ProcessManager, ReadAuthoritySet, ReadAuthoritySpec, SessionGrantSpec,
    SessionLifecycleManager, SessionRegistry, StdClock, TaskLeaseRegistry,
    TransactionalFileService, git_integration_resource_budget, mutation_resource_budget,
    read_authority_resource_budget,
};
use rmcp::ServiceExt;

use crate::{
    persistent_config::load_exclusive_config_args, tool_profile_config::load_tool_profiles,
};

const INITIAL_SESSION_TTL_MS: u64 = 30 * 60 * 1000;
const SESSION_REAP_INTERVAL_MS: u64 = 5_000;

fn build_process_manager(
    args: &AppArgs,
    limits: HardLimits,
) -> Result<ProcessManager, Box<dyn Error + Send + Sync>> {
    #[cfg(windows)]
    {
        let launcher = discover_isolation_launcher()?;
        if !args.isolated_node_executables.is_empty() && launcher.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "--allow-isolated-node requires the installed sibling optic-bridge-isolation-launcher.exe",
            )
            .into());
        }
        if let Some(launcher) = launcher {
            return Ok(
                ProcessManager::new_with_isolation_launcher_and_environment_grants(
                    &args.workspace,
                    limits,
                    args.environment_grants.clone(),
                    launcher,
                )?,
            );
        }
    }

    #[cfg(not(windows))]
    if !args.isolated_node_executables.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "--allow-isolated-node requires the Windows AppContainer runtime",
        )
        .into());
    }

    Ok(ProcessManager::new_with_environment_grants(
        &args.workspace,
        limits,
        args.environment_grants.clone(),
    )?)
}

#[cfg(windows)]
fn discover_isolation_launcher() -> Result<Option<PathBuf>, std::io::Error> {
    let current_exe = std::env::current_exe()?;
    let launcher = current_exe.with_file_name("optic-bridge-isolation-launcher.exe");
    match std::fs::metadata(&launcher) {
        Ok(metadata) if metadata.is_file() => Ok(Some(launcher)),
        Ok(_) => Err(std::io::Error::other(
            "isolation launcher sibling exists but is not a regular file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = AppArgs::parse()?;
    let limits = HardLimits::default().validate_nonzero()?;
    let clock: Arc<dyn Clock> = Arc::new(StdClock::new());
    let now = clock.now();
    let expires_at = now.saturating_add_millis(INITIAL_SESSION_TTL_MS);
    let processes = Arc::new(build_process_manager(&args, limits)?);
    let task_leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits)?);
    let approvals = Arc::new(ApprovalBroker::from_hard_limits(limits)?);

    let canonical_process_executables =
        canonicalize_allowed_executables(&processes, &args.allowed_executables)?;
    let canonical_process_read_grants = canonicalize_process_read_grants(
        &processes,
        &canonical_process_executables,
        &args.process_read_grants,
    )?;
    let process_isolation_eligible = canonicalize_isolated_node_eligibility(
        &processes,
        &canonical_process_executables,
        &args.isolated_node_executables,
    )?;

    let loaded_tool_profiles = load_tool_profiles(
        args.tool_profile_file.as_deref(),
        &processes,
        &canonical_process_executables,
        &canonical_process_read_grants,
        &process_isolation_eligible,
        limits,
    )?;
    let configured_tool_profiles = loaded_tool_profiles
        .as_ref()
        .map(|loaded| loaded.profiles.as_slice())
        .unwrap_or(&[]);

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
    let lifecycle = Arc::new(SessionLifecycleManager::new_with_approval_broker(
        Arc::clone(&sessions),
        Arc::clone(&task_leases),
        Arc::clone(&processes),
        Arc::clone(&approvals),
    ));
    let grant = lifecycle.provision(
        SessionGrantSpec {
            principal: PrincipalId::new("local-stdio")
                .ok_or_else(|| std::io::Error::other("invalid local principal id"))?,
            project: ProjectId::new("local-workspace")
                .ok_or_else(|| std::io::Error::other("invalid local project id"))?,
            capabilities,
            expires_at,
            policy_epoch: args.policy_epoch,
        },
        now,
    )?;
    let session = grant.handle;

    let read_authorities = ReadAuthoritySet::provision(
        &task_leases,
        &session,
        &ReadAuthoritySpec::workspace_all_non_sensitive(),
        read_authority_resource_budget(limits),
        expires_at,
        args.policy_epoch,
    )?;

    // C5E keeps the first production minting path deliberately profile-specific:
    // only exact operator-selected Node interpreters may receive eligibility.
    let process_leases = provision_process_leases(
        &task_leases,
        &session,
        &canonical_process_executables,
        &canonical_process_read_grants,
        &process_isolation_eligible,
        configured_tool_profiles,
        ProcessLeaseTerms {
            workload_class: WorkloadClass::Heavy,
            resource_ceiling: limits.max_process_budget,
            expires_at,
            policy_epoch: args.policy_epoch,
        },
    )?;
    let tool_profile_count = configured_tool_profiles.len();
    let tool_profile_registry = loaded_tool_profiles.map(|loaded| Arc::new(loaded.registry));

    let mutation_authorities = MutationAuthoritySet::provision(
        &task_leases,
        &session,
        &mutation_spec,
        mutation_resource_budget(limits),
        expires_at,
        args.policy_epoch,
    )?;

    let git_integration_authorities = GitIntegrationAuthoritySet::provision(
        &task_leases,
        &session,
        GitIntegrationAuthoritySpec {
            enabled: args.allow_git_integrate,
        },
        git_integration_resource_budget(limits),
        expires_at,
        args.policy_epoch,
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
    let server = ReadonlyMcpServer::new_with_git_integration_runtime_and_approvals_and_profiles(
        &args.workspace,
        sessions,
        session,
        Arc::clone(&clock),
        limits,
        processes,
        task_leases,
        approvals,
        tool_profile_registry,
        process_leases,
        mutation_service,
        mutation_authorities,
        git_service,
        git_integration_service,
        git_integration_authorities,
    )?
    .with_read_authorities(read_authorities);
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
        "Optic AI Bridge {} — MCP stdio ({} process executable(s), {} tool profile(s), {} write scope(s), {} delete scope(s), Git read {}, Git integrate authority {})",
        env!("CARGO_PKG_VERSION"),
        args.allowed_executables.len(),
        tool_profile_count,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProcessReadGrantSpec {
    executable: String,
    path: WorkspacePath,
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

fn canonicalize_process_read_grants(
    processes: &ProcessManager,
    executables: &BTreeMap<String, ProcessExecutionClass>,
    grants: &[ProcessReadGrantSpec],
) -> Result<BTreeMap<String, BTreeSet<WorkspacePath>>, Box<dyn Error + Send + Sync>> {
    #[cfg(not(windows))]
    if !grants.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "--allow-process-read-file requires the Windows AppContainer runtime",
        )
        .into());
    }

    let mut canonical = BTreeMap::<String, BTreeSet<WorkspacePath>>::new();
    for grant in grants {
        let executable = processes.canonicalize_executable(&grant.executable)?;
        let class = executables.get(&executable).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--allow-process-read-file executable must also be authorized by --allow-executable",
            )
        })?;
        if *class == ProcessExecutionClass::FixedTool {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--allow-process-read-file requires interpreter or repository-code authority",
            )
            .into());
        }
        let paths = canonical.entry(executable).or_default();
        if !paths.insert(grant.path.clone()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "duplicate --allow-process-read-file grant",
            )
            .into());
        }
    }

    #[cfg(windows)]
    for paths in canonical.values() {
        let paths = paths.iter().cloned().collect::<Vec<_>>();
        processes.validate_workspace_read_files(&paths)?;
    }
    Ok(canonical)
}

fn canonicalize_isolated_node_eligibility(
    processes: &ProcessManager,
    executables: &BTreeMap<String, ProcessExecutionClass>,
    nodes: &[String],
) -> Result<BTreeSet<String>, Box<dyn Error + Send + Sync>> {
    #[cfg(not(windows))]
    if !nodes.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "--allow-isolated-node requires the Windows AppContainer runtime",
        )
        .into());
    }

    let mut eligible = BTreeSet::new();
    for node in nodes {
        let executable = processes.canonicalize_executable(node)?;
        match executables.get(&executable) {
            Some(ProcessExecutionClass::Interpreter) => {}
            Some(_) | None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "--allow-isolated-node requires the same executable to be authorized as --allow-executable=interpreter:<absolute-node-path>",
                )
                .into());
            }
        }

        #[cfg(windows)]
        {
            let executable_path = PathBuf::from(&executable);
            let file_name = executable_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if !file_name.eq_ignore_ascii_case("node.exe") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "--allow-isolated-node accepts only an exact node.exe interpreter path",
                )
                .into());
            }
        }

        if !eligible.insert(executable) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "duplicate canonical --allow-isolated-node executable",
            )
            .into());
        }
    }
    Ok(eligible)
}

fn validate_process_isolation_eligibility(
    executables: &BTreeMap<String, ProcessExecutionClass>,
    eligible_executables: &BTreeSet<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(not(windows))]
    if !eligible_executables.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "process isolation eligibility requires the Windows AppContainer runtime",
        )
        .into());
    }

    for executable in eligible_executables {
        match executables.get(executable) {
            Some(ProcessExecutionClass::Interpreter | ProcessExecutionClass::RepositoryCode) => {}
            Some(ProcessExecutionClass::FixedTool) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "process isolation eligibility cannot be attached to fixed-tool authority",
                )
                .into());
            }
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "process isolation eligibility requires an already-authorized exact executable",
                )
                .into());
            }
        }
    }
    Ok(())
}
struct ProcessLeaseTerms {
    workload_class: WorkloadClass,
    resource_ceiling: ResourceBudget,
    expires_at: MonotonicTime,
    policy_epoch: u64,
}

fn provision_process_leases(
    task_leases: &TaskLeaseRegistry,
    session: &SessionHandle,
    executables: &BTreeMap<String, ProcessExecutionClass>,
    read_grants: &BTreeMap<String, BTreeSet<WorkspacePath>>,
    isolation_eligible: &BTreeSet<String>,
    tool_profiles: &[ToolProfile],
    terms: ProcessLeaseTerms,
) -> Result<BTreeMap<String, TaskLeaseId>, Box<dyn Error + Send + Sync>> {
    // Validate every server-owned eligibility selection before registering any
    // lease so a bad marker can never leave partial process authority behind.
    validate_process_isolation_eligibility(executables, isolation_eligible)?;

    #[cfg(not(windows))]
    if read_grants.values().any(|paths| !paths.is_empty()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "process read-grant leases require the Windows AppContainer runtime",
        )
        .into());
    }

    let mut process_leases = BTreeMap::new();
    for (canonical, class) in executables {
        let id = TaskLeaseId::generate()
            .map_err(|_| std::io::Error::other("failed to create process task lease id"))?;
        let mut capabilities = BTreeSet::from([Capability::ProcessRun]);
        let mut scopes = BTreeSet::from([LeaseScope::ProcessExecutable {
            executable: canonical.clone(),
            class: *class,
        }]);
        if isolation_eligible.contains(canonical) {
            scopes.insert(LeaseScope::ProcessIsolationEligible {
                executable: canonical.clone(),
                class: *class,
            });
        }
        for profile in tool_profiles
            .iter()
            .filter(|profile| profile.spec().executable == *canonical)
        {
            scopes.insert(LeaseScope::ToolProfile {
                name: profile.name().clone(),
                approval: profile.approval_requirement(),
            });
        }
        if let Some(paths) = read_grants.get(canonical)
            && !paths.is_empty()
        {
            capabilities.insert(Capability::FileRead);
            scopes.extend(paths.iter().cloned().map(LeaseScope::WorkspacePrefix));
        }
        let resource_ceiling = if isolation_eligible.contains(canonical) {
            ResourceBudget {
                process_count: 1,
                ..terms.resource_ceiling
            }
        } else {
            terms.resource_ceiling
        };
        task_leases.register(TaskLease {
            id: id.clone(),
            session: session.clone(),
            capabilities,
            scopes,
            workload_class: terms.workload_class,
            resource_ceiling,
            expires_at: terms.expires_at,
            policy_epoch: terms.policy_epoch,
        })?;
        process_leases.insert(canonical.clone(), id);
    }
    Ok(process_leases)
}

#[derive(Debug)]
struct AppArgs {
    workspace: PathBuf,
    policy_epoch: u64,
    allowed_executables: Vec<AllowedExecutableSpec>,
    process_read_grants: Vec<ProcessReadGrantSpec>,
    isolated_node_executables: Vec<String>,
    tool_profile_file: Option<PathBuf>,
    environment_grants: Vec<EnvironmentGrant>,
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
        let args = args.into_iter().collect::<Vec<_>>();
        if let Some(config) = load_exclusive_config_args(&args)? {
            let mut parsed = Self::parse_from(config.args)?;
            parsed.policy_epoch = config.policy_epoch;
            return Ok(parsed);
        }

        let mut workspace = None;
        let mut allowed_executables = Vec::new();
        let mut process_read_grants = Vec::new();
        let mut isolated_node_executables = Vec::new();
        let mut tool_profile_file = None;
        let mut environment_grants = Vec::new();
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
            } else if arg == "--allow-process-read-file" {
                let executable = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-process-read-file requires <absolute-executable> <workspace-file>",
                    )
                })?;
                let path = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-process-read-file requires <absolute-executable> <workspace-file>",
                    )
                })?;
                process_read_grants.push(parse_process_read_grant(executable, path)?);
            } else if arg == "--allow-isolated-node" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-isolated-node requires an absolute node.exe path",
                    )
                })?;
                isolated_node_executables.push(parse_isolated_node_executable(os_string_to_utf8(
                    value,
                    "isolated Node executable",
                )?)?);
            } else if arg == "--tool-profile-file" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--tool-profile-file requires an absolute JSON file path",
                    )
                })?;
                set_tool_profile_file(&mut tool_profile_file, PathBuf::from(value))?;
            } else if arg == "--allow-env" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-env requires a variable name",
                    )
                })?;
                environment_grants.push(EnvironmentGrant::benign(os_string_to_utf8(
                    value,
                    "environment variable name",
                )?));
            } else if arg == "--env-grant" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--env-grant requires <benign|sensitive|forbidden>:<name>",
                    )
                })?;
                environment_grants.push(parse_environment_grant(os_string_to_utf8(
                    value,
                    "classified environment grant",
                )?)?);
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
            } else if let Some(executable) = option_value(&arg, "--allow-process-read-file=")? {
                let path = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-process-read-file=<absolute-executable> requires <workspace-file>",
                    )
                })?;
                process_read_grants
                    .push(parse_process_read_grant(OsString::from(executable), path)?);
            } else if let Some(value) = option_value(&arg, "--allow-isolated-node=")? {
                isolated_node_executables.push(parse_isolated_node_executable(value)?);
            } else if let Some(value) = option_value(&arg, "--tool-profile-file=")? {
                set_tool_profile_file(&mut tool_profile_file, PathBuf::from(value))?;
            } else if let Some(value) = option_value(&arg, "--allow-env=")? {
                environment_grants.push(EnvironmentGrant::benign(value));
            } else if let Some(value) = option_value(&arg, "--env-grant=")? {
                environment_grants.push(parse_environment_grant(value)?);
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
            policy_epoch: 1,
            allowed_executables,
            process_read_grants,
            isolated_node_executables,
            tool_profile_file,
            environment_grants,
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

fn parse_environment_grant(value: String) -> Result<EnvironmentGrant, std::io::Error> {
    let (class, name) = value.split_once(':').ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--env-grant requires <benign|sensitive|forbidden>:<name>",
        )
    })?;
    if name.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--env-grant variable name must not be empty",
        ));
    }
    let class = match class {
        "benign" => EnvironmentVariableClass::Benign,
        "sensitive" => EnvironmentVariableClass::Sensitive,
        "forbidden" => EnvironmentVariableClass::Forbidden,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--env-grant class must be benign, sensitive, or forbidden",
            ));
        }
    };
    Ok(EnvironmentGrant {
        name: name.to_owned(),
        class,
    })
}

fn set_tool_profile_file(slot: &mut Option<PathBuf>, value: PathBuf) -> Result<(), std::io::Error> {
    if !value.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--tool-profile-file must be an absolute path",
        ));
    }
    if slot.replace(value).is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--tool-profile-file may be supplied only once",
        ));
    }
    Ok(())
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

fn parse_isolated_node_executable(value: String) -> Result<String, std::io::Error> {
    if value.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--allow-isolated-node path must not be empty",
        ));
    }
    Ok(value)
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

fn parse_process_read_grant(
    executable: OsString,
    path: OsString,
) -> Result<ProcessReadGrantSpec, std::io::Error> {
    let executable = os_string_to_utf8(executable, "process read executable")?;
    if executable.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--allow-process-read-file executable must not be empty",
        ));
    }
    let path = WorkspacePath::parse(&os_string_to_utf8(path, "process read workspace file")?)
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--allow-process-read-file path must be a safe project-relative workspace path",
            )
        })?;
    Ok(ProcessReadGrantSpec { executable, path })
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

    #[test]
    fn classified_environment_grant_parser_preserves_operator_class() {
        assert_eq!(
            parse_environment_grant("benign:PATH_HINT".to_owned()).expect("benign"),
            EnvironmentGrant {
                name: "PATH_HINT".to_owned(),
                class: EnvironmentVariableClass::Benign,
            }
        );
        assert_eq!(
            parse_environment_grant("sensitive:API_KEY".to_owned()).expect("sensitive"),
            EnvironmentGrant {
                name: "API_KEY".to_owned(),
                class: EnvironmentVariableClass::Sensitive,
            }
        );
        assert!(parse_environment_grant("unknown:VALUE".to_owned()).is_err());
        assert!(parse_environment_grant("forbidden:".to_owned()).is_err());
    }

    use super::*;
    use optic_bridge_core::{
        NetworkAccess, ToolApprovalRequirement, ToolProfileName, ToolProfileSpec,
    };

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn persistent_v1_config_reuses_cli_validation_and_sets_policy_epoch() {
        let token = SessionHandle::generate().expect("test entropy").to_token();
        let path = std::env::temp_dir().join(format!("optic-config-{token}.json"));
        std::fs::write(
            &path,
            r#"{
                "version": 1,
                "policy_epoch": 9,
                "workspace": "workspace-from-config",
                "environment_grants": [{"class":"benign","name":"PATH_HINT"}]
            }"#,
        )
        .expect("write config");

        let parsed = AppArgs::parse_from(vec![
            OsString::from("--config"),
            path.clone().into_os_string(),
        ])
        .expect("persistent config");
        assert_eq!(parsed.policy_epoch, 9);
        assert_eq!(parsed.workspace, PathBuf::from("workspace-from-config"));
        assert_eq!(
            parsed.environment_grants,
            vec![EnvironmentGrant::benign("PATH_HINT")]
        );
        std::fs::remove_file(path).expect("remove config");
    }

    #[test]
    fn mutation_authority_is_absent_without_operator_flags() {
        let parsed = AppArgs::parse_from(args(&["workspace"])).expect("args");
        assert!(parsed.process_read_grants.is_empty());
        assert!(parsed.isolated_node_executables.is_empty());
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
    fn tool_profile_file_is_explicit_absolute_and_unique() {
        let absolute = if cfg!(windows) {
            r"C:\profiles.json"
        } else {
            "/tmp/profiles.json"
        };
        let parsed = AppArgs::parse_from([
            OsString::from("--tool-profile-file"),
            OsString::from(absolute),
        ])
        .expect("profile file flag");
        assert_eq!(parsed.tool_profile_file, Some(PathBuf::from(absolute)));
        assert!(
            AppArgs::parse_from([
                OsString::from("--tool-profile-file"),
                OsString::from(absolute),
                OsString::from("--tool-profile-file"),
                OsString::from(absolute),
            ])
            .is_err()
        );
        assert!(
            AppArgs::parse_from([
                OsString::from("--tool-profile-file"),
                OsString::from("relative.json"),
            ])
            .is_err()
        );
    }

    #[test]
    fn isolated_node_profile_is_explicit_and_repeatable() {
        let parsed = AppArgs::parse_from(args(&[
            "--allow-isolated-node=C:\\Program Files\\nodejs\\node.exe",
            "--allow-isolated-node",
            "D:\\Tools\\node.exe",
            "workspace",
        ]))
        .expect("isolated Node profile");
        assert_eq!(
            parsed.isolated_node_executables,
            vec![
                "C:\\Program Files\\nodejs\\node.exe".to_owned(),
                "D:\\Tools\\node.exe".to_owned(),
            ]
        );
        assert!(
            AppArgs::parse_from(args(&["--allow-isolated-node=", "workspace"])).is_err(),
            "empty isolated Node profile must fail closed"
        );
    }

    #[test]
    fn process_read_grants_are_explicit_exact_workspace_paths() {
        let parsed = AppArgs::parse_from(args(&[
            "--allow-process-read-file",
            "/opt/python",
            "input.txt",
            "--allow-process-read-file=/opt/cargo",
            "Cargo.toml",
            "workspace",
        ]))
        .expect("process read grants");
        assert_eq!(
            parsed.process_read_grants,
            vec![
                ProcessReadGrantSpec {
                    executable: "/opt/python".to_owned(),
                    path: WorkspacePath::parse("input.txt").expect("path"),
                },
                ProcessReadGrantSpec {
                    executable: "/opt/cargo".to_owned(),
                    path: WorkspacePath::parse("Cargo.toml").expect("path"),
                },
            ]
        );

        for invalid in ["../secret", "/absolute", "C:\\Windows\\win.ini"] {
            assert!(
                AppArgs::parse_from(args(&[
                    "--allow-process-read-file=/opt/python",
                    invalid,
                    "workspace",
                ]))
                .is_err(),
                "unsafe process read path should fail: {invalid}"
            );
        }
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

    #[cfg(windows)]
    #[test]
    fn operator_process_read_grants_are_bounded_to_authorized_high_risk_executables() {
        let token = SessionHandle::generate().expect("test entropy").to_token();
        let root = std::env::temp_dir().join(format!("optic-process-read-grants-{token}"));
        std::fs::create_dir_all(&root).expect("create workspace");
        std::fs::write(root.join("input.txt"), b"input").expect("write input");
        std::fs::create_dir(root.join("directory")).expect("create directory");
        let processes =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let executable = std::env::current_exe()
            .expect("current executable")
            .canonicalize()
            .expect("canonical executable")
            .to_string_lossy()
            .into_owned();
        let grants = vec![ProcessReadGrantSpec {
            executable: executable.clone(),
            path: WorkspacePath::parse("input.txt").expect("path"),
        }];
        let interpreter =
            BTreeMap::from([(executable.clone(), ProcessExecutionClass::Interpreter)]);
        let canonical = canonicalize_process_read_grants(&processes, &interpreter, &grants)
            .expect("canonical exact-file grant");
        assert_eq!(
            canonical.get(&executable),
            Some(&BTreeSet::from([
                WorkspacePath::parse("input.txt").expect("path")
            ]))
        );

        let fixed = BTreeMap::from([(executable.clone(), ProcessExecutionClass::FixedTool)]);
        assert!(canonicalize_process_read_grants(&processes, &fixed, &grants).is_err());
        assert!(canonicalize_process_read_grants(&processes, &BTreeMap::new(), &grants).is_err());
        assert!(
            canonicalize_process_read_grants(
                &processes,
                &interpreter,
                &[grants[0].clone(), grants[0].clone()],
            )
            .is_err()
        );
        assert!(
            canonicalize_process_read_grants(
                &processes,
                &interpreter,
                &[ProcessReadGrantSpec {
                    executable: executable.clone(),
                    path: WorkspacePath::parse("directory").expect("directory path"),
                }],
            )
            .is_err()
        );
        std::fs::remove_dir_all(root).expect("remove workspace");
    }

    #[cfg(windows)]
    #[test]
    fn process_leases_mint_file_read_only_for_explicit_operator_grants() {
        let executable = std::env::current_exe()
            .expect("current executable")
            .canonicalize()
            .expect("canonical executable")
            .to_string_lossy()
            .into_owned();
        let executables =
            BTreeMap::from([(executable.clone(), ProcessExecutionClass::Interpreter)]);
        let exact = WorkspacePath::parse("input.txt").expect("path");
        let read_grants = BTreeMap::from([(executable.clone(), BTreeSet::from([exact.clone()]))]);
        let limits = HardLimits::default();
        let expires_at = MonotonicTime::from_millis(1_000);
        let now = MonotonicTime::from_millis(1);
        let task_leases = TaskLeaseRegistry::from_hard_limits(limits).expect("registry");
        let session_a = SessionHandle::generate().expect("session A");
        let session_b = SessionHandle::generate().expect("session B");

        let a = provision_process_leases(
            &task_leases,
            &session_a,
            &executables,
            &read_grants,
            &BTreeSet::new(),
            &[],
            ProcessLeaseTerms {
                workload_class: WorkloadClass::Heavy,
                resource_ceiling: limits.max_process_budget,
                expires_at,
                policy_epoch: 1,
            },
        )
        .expect("A process authority");
        let a_id = a.get(&executable).expect("A lease id");
        let a_lease = task_leases
            .get_active(a_id, &session_a, now)
            .expect("A active lease");
        assert!(a_lease.allows(Capability::ProcessRun));
        assert_eq!(a_lease.workload_class, WorkloadClass::Heavy);
        assert!(a_lease.allows(Capability::FileRead));
        assert!(a_lease.has_scope(&LeaseScope::WorkspacePrefix(exact.clone())));
        assert!(!a_lease.has_scope(&LeaseScope::WorkspaceAll));
        assert_eq!(
            task_leases
                .get_active(a_id, &session_b, now)
                .expect_err("A lease must not cross sessions"),
            optic_bridge_runtime::TaskLeaseRegistryError::WrongSession
        );

        let b = provision_process_leases(
            &task_leases,
            &session_b,
            &executables,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &[],
            ProcessLeaseTerms {
                workload_class: WorkloadClass::Heavy,
                resource_ceiling: limits.max_process_budget,
                expires_at,
                policy_epoch: 1,
            },
        )
        .expect("B process authority without read grants");
        let b_lease = task_leases
            .get_active(b.get(&executable).expect("B lease id"), &session_b, now)
            .expect("B active lease");
        assert!(b_lease.allows(Capability::ProcessRun));
        assert!(!b_lease.allows(Capability::FileRead));
        assert!(
            b_lease
                .scopes
                .iter()
                .all(|scope| !matches!(scope, LeaseScope::WorkspacePrefix(_)))
        );

        assert!(task_leases.revoke(a_id).expect("revoke A lease"));
        assert_eq!(
            task_leases
                .get_active(a_id, &session_a, now)
                .expect_err("revoked read authority must be inactive"),
            optic_bridge_runtime::TaskLeaseRegistryError::Revoked
        );
    }

    #[test]
    fn process_profile_scope_is_application_owned_and_exact() {
        let executable = std::env::current_exe()
            .expect("current executable")
            .canonicalize()
            .expect("canonical executable")
            .to_string_lossy()
            .into_owned();
        let limits = HardLimits::default();
        let profile = ToolProfile::from_spec(ToolProfileSpec {
            name: ToolProfileName::parse("fixed-check").expect("profile name"),
            executable: executable.clone(),
            class: ProcessExecutionClass::FixedTool,
            workload_class: WorkloadClass::Heavy,
            exact_args: vec!["--check".to_owned()],
            cwd: None,
            workspace_read_files: BTreeSet::new(),
            env_allowlist: BTreeSet::new(),
            network: NetworkAccess::Denied,
            resource_ceiling: limits.max_process_budget,
            approval: ToolApprovalRequirement::HumanRequired,
        })
        .expect("profile");
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("registry");
        let session = SessionHandle::generate().expect("session");
        let leases = provision_process_leases(
            &registry,
            &session,
            &BTreeMap::from([(executable.clone(), ProcessExecutionClass::FixedTool)]),
            &BTreeMap::new(),
            &BTreeSet::new(),
            std::slice::from_ref(&profile),
            ProcessLeaseTerms {
                workload_class: WorkloadClass::Heavy,
                resource_ceiling: limits.max_process_budget,
                expires_at: MonotonicTime::from_millis(1_000),
                policy_epoch: 1,
            },
        )
        .expect("process lease");
        let lease = registry
            .get_active(
                leases.get(&executable).expect("lease id"),
                &session,
                MonotonicTime::from_millis(1),
            )
            .expect("active lease");
        assert!(lease.has_scope(&LeaseScope::ToolProfile {
            name: profile.name().clone(),
            approval: ToolApprovalRequirement::HumanRequired,
        }));
        assert!(!lease.allows(Capability::NetworkAccess));
        assert_eq!(lease.workload_class, WorkloadClass::Heavy);
    }

    #[cfg(windows)]
    #[test]
    fn internal_isolation_eligibility_is_exact_optional_and_high_risk_only() {
        let executable = std::env::current_exe()
            .expect("current executable")
            .canonicalize()
            .expect("canonical executable")
            .to_string_lossy()
            .into_owned();
        let limits = HardLimits::default();
        let expires_at = MonotonicTime::from_millis(1_000);
        let now = MonotonicTime::from_millis(1);

        for class in [
            ProcessExecutionClass::Interpreter,
            ProcessExecutionClass::RepositoryCode,
        ] {
            let executables = BTreeMap::from([(executable.clone(), class)]);
            let eligible = BTreeSet::from([executable.clone()]);
            let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("registry");
            let session = SessionHandle::generate().expect("session");
            let leases = provision_process_leases(
                &registry,
                &session,
                &executables,
                &BTreeMap::new(),
                &eligible,
                &[],
                ProcessLeaseTerms {
                    workload_class: WorkloadClass::Heavy,
                    resource_ceiling: limits.max_process_budget,
                    expires_at,
                    policy_epoch: 1,
                },
            )
            .expect("eligible process lease");
            let lease = registry
                .get_active(leases.get(&executable).expect("lease id"), &session, now)
                .expect("active eligible lease");
            assert!(lease.process_isolation_eligible(&executable, class));
            assert_eq!(lease.workload_class, WorkloadClass::Heavy);
            assert!(lease.has_scope(&LeaseScope::ProcessExecutable {
                executable: executable.clone(),
                class,
            }));
            assert!(!lease.allows(Capability::NetworkAccess));
        }

        let executables =
            BTreeMap::from([(executable.clone(), ProcessExecutionClass::Interpreter)]);
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("registry");
        let session = SessionHandle::generate().expect("session");
        let leases = provision_process_leases(
            &registry,
            &session,
            &executables,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &[],
            ProcessLeaseTerms {
                workload_class: WorkloadClass::Heavy,
                resource_ceiling: limits.max_process_budget,
                expires_at,
                policy_epoch: 1,
            },
        )
        .expect("ordinary process lease");
        let lease = registry
            .get_active(leases.get(&executable).expect("lease id"), &session, now)
            .expect("active ordinary lease");
        assert!(!lease.process_isolation_eligible(&executable, ProcessExecutionClass::Interpreter));

        let fixed = BTreeMap::from([(executable.clone(), ProcessExecutionClass::FixedTool)]);
        assert!(
            validate_process_isolation_eligibility(&fixed, &BTreeSet::from([executable.clone()]))
                .is_err(),
            "fixed tools must not receive the high-risk isolation marker"
        );
        assert!(
            validate_process_isolation_eligibility(
                &executables,
                &BTreeSet::from(["C:\\missing\\unknown.exe".to_owned()])
            )
            .is_err(),
            "unknown executables must not receive the isolation marker"
        );
    }

    #[cfg(windows)]
    #[test]
    fn invalid_isolation_eligibility_fails_before_any_process_lease_is_registered() {
        let executable = std::env::current_exe()
            .expect("current executable")
            .canonicalize()
            .expect("canonical executable")
            .to_string_lossy()
            .into_owned();
        let executables =
            BTreeMap::from([(executable.clone(), ProcessExecutionClass::Interpreter)]);
        let limits = HardLimits {
            max_task_leases: 1,
            max_task_leases_per_session: 1,
            ..HardLimits::default()
        };
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("bounded registry");
        let session = SessionHandle::generate().expect("session");
        let invalid = BTreeSet::from([
            executable.clone(),
            "Z:\\unknown-isolation-target.exe".to_owned(),
        ]);

        assert!(
            provision_process_leases(
                &registry,
                &session,
                &executables,
                &BTreeMap::new(),
                &invalid,
                &[],
                ProcessLeaseTerms {
                    workload_class: WorkloadClass::Heavy,
                    resource_ceiling: limits.max_process_budget,
                    expires_at: MonotonicTime::from_millis(1_000),
                    policy_epoch: 1,
                },
            )
            .is_err()
        );

        let valid = provision_process_leases(
            &registry,
            &session,
            &executables,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &[],
            ProcessLeaseTerms {
                workload_class: WorkloadClass::Heavy,
                resource_ceiling: limits.max_process_budget,
                expires_at: MonotonicTime::from_millis(1_000),
                policy_epoch: 1,
            },
        )
        .expect("capacity must remain untouched after invalid eligibility");
        assert_eq!(valid.len(), 1);
    }
    #[cfg(not(windows))]
    #[test]
    fn process_read_authority_fails_closed_without_windows_appcontainer() {
        let root = std::env::temp_dir().join(format!(
            "optic-nonwindows-process-read-{}",
            SessionHandle::generate().expect("test entropy").to_token()
        ));
        std::fs::create_dir_all(&root).expect("create workspace");
        std::fs::write(root.join("input.txt"), b"input").expect("write input");
        let processes =
            ProcessManager::new(&root, HardLimits::default(), Vec::new()).expect("process manager");
        let executable = std::env::current_exe()
            .expect("current executable")
            .canonicalize()
            .expect("canonical executable")
            .to_string_lossy()
            .into_owned();
        let executables =
            BTreeMap::from([(executable.clone(), ProcessExecutionClass::Interpreter)]);
        let grant = ProcessReadGrantSpec {
            executable: executable.clone(),
            path: WorkspacePath::parse("input.txt").expect("path"),
        };
        assert!(
            canonicalize_process_read_grants(&processes, &executables, &[grant]).is_err(),
            "non-Windows startup must reject AppContainer read authority"
        );

        let task_leases = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session");
        let read_grants = BTreeMap::from([(
            executable,
            BTreeSet::from([WorkspacePath::parse("input.txt").expect("path")]),
        )]);
        assert!(
            provision_process_leases(
                &task_leases,
                &session,
                &executables,
                &read_grants,
                &BTreeSet::new(),
                &[],
                ProcessLeaseTerms {
                    workload_class: WorkloadClass::Heavy,
                    resource_ceiling: HardLimits::default().max_process_budget,
                    expires_at: MonotonicTime::from_millis(1_000),
                    policy_epoch: 1,
                },
            )
            .is_err(),
            "direct provisioning must not mint Windows-only read authority"
        );
        let eligibility = BTreeSet::from([executables.keys().next().expect("executable").clone()]);
        assert!(
            provision_process_leases(
                &task_leases,
                &session,
                &executables,
                &BTreeMap::new(),
                &eligibility,
                &[],
                ProcessLeaseTerms {
                    workload_class: WorkloadClass::Heavy,
                    resource_ceiling: HardLimits::default().max_process_budget,
                    expires_at: MonotonicTime::from_millis(1_000),
                    policy_epoch: 1,
                },
            )
            .is_err(),
            "non-Windows provisioning must reject AppContainer isolation eligibility"
        );
        std::fs::remove_dir_all(root).expect("remove workspace");
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
