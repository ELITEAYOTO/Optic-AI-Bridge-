#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    path::PathBuf,
    sync::Arc,
};

use optic_bridge_core::{
    Capability, HardLimits, LeaseScope, PrincipalId, ProjectId, SessionGrant, SessionHandle,
    TaskLease, TaskLeaseId, WorkspacePath,
};
use optic_bridge_mcp::{BoundedJsonLineTransport, ReadonlyMcpServer};
use optic_bridge_runtime::{
    AuthorizedFileMutationService, Clock, GitReadService, MutationAuthoritySet,
    MutationAuthoritySpec, ProcessManager, SessionRegistry, StdClock, TaskLeaseRegistry,
    TransactionalFileService, mutation_resource_budget,
};
use rmcp::ServiceExt;

const INITIAL_SESSION_TTL_MS: u64 = 30 * 60 * 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = AppArgs::parse()?;
    let limits = HardLimits::default().validate_nonzero()?;
    let clock = Arc::new(StdClock::new());
    let now = clock.now();
    let expires_at = now.saturating_add_millis(INITIAL_SESSION_TTL_MS);
    let session = SessionHandle::generate()
        .map_err(|_| std::io::Error::other("failed to create application session handle"))?;

    let processes = Arc::new(ProcessManager::new(
        &args.workspace,
        limits,
        args.allowed_env.clone(),
    )?);
    let task_leases = Arc::new(TaskLeaseRegistry::new());
    let mut process_leases = BTreeMap::new();
    for executable in &args.allowed_executables {
        let canonical = processes.canonicalize_executable(executable)?;
        if process_leases.contains_key(&canonical) {
            continue;
        }
        let id = TaskLeaseId::generate()
            .map_err(|_| std::io::Error::other("failed to create process task lease id"))?;
        task_leases.register(TaskLease {
            id: id.clone(),
            session: session.clone(),
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([LeaseScope::ProcessExecutable(canonical.clone())]),
            resource_ceiling: limits.max_process_budget,
            expires_at,
            policy_epoch: 1,
        })?;
        process_leases.insert(canonical, id);
    }

    let mutation_authorities = MutationAuthoritySet::provision(
        &task_leases,
        &session,
        &MutationAuthoritySpec {
            write_scopes: args.write_scopes.clone(),
            delete_scopes: args.delete_scopes.clone(),
        },
        mutation_resource_budget(limits),
        expires_at,
        1,
    )?;

    let git_service = args
        .git_executable
        .as_ref()
        .map(|git| GitReadService::from_hard_limits(&args.workspace, git, limits))
        .transpose()?
        .map(Arc::new);

    let mut capabilities = BTreeSet::from([Capability::FileRead, Capability::FileSearch]);
    if !process_leases.is_empty() {
        capabilities.insert(Capability::ProcessRun);
    }
    if mutation_authorities.has_write() {
        capabilities.insert(Capability::FileWrite);
    }
    if mutation_authorities.has_delete() {
        capabilities.insert(Capability::FileDelete);
    }
    if git_service.is_some() {
        capabilities.insert(Capability::GitRead);
    }
    let grant = SessionGrant {
        handle: session.clone(),
        principal: PrincipalId::new("local-stdio")
            .ok_or_else(|| std::io::Error::other("invalid local principal id"))?,
        project: ProjectId::new("local-workspace")
            .ok_or_else(|| std::io::Error::other("invalid local project id"))?,
        capabilities,
        expires_at,
        policy_epoch: 1,
    };

    let sessions = Arc::new(SessionRegistry::new());
    sessions.register(grant)?;

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
            Arc::clone(&clock) as Arc<dyn Clock>,
        )?))
    } else {
        None
    };

    let git_read_enabled = git_service.is_some();
    let server = ReadonlyMcpServer::new_with_git_runtime(
        &args.workspace,
        sessions,
        session,
        clock,
        limits,
        processes,
        task_leases,
        process_leases,
        mutation_service,
        mutation_authorities,
        git_service,
    )?;

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
        "Optic AI Bridge {} — MCP stdio ({} process executable(s), {} write scope(s), {} delete scope(s), Git read {})",
        env!("CARGO_PKG_VERSION"),
        args.allowed_executables.len(),
        args.write_scopes.len(),
        args.delete_scopes.len(),
        if git_read_enabled { "enabled" } else { "disabled" },
    );
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

#[derive(Debug)]
struct AppArgs {
    workspace: PathBuf,
    allowed_executables: Vec<String>,
    allowed_env: Vec<String>,
    write_scopes: BTreeSet<LeaseScope>,
    delete_scopes: BTreeSet<LeaseScope>,
    mutation_state_dir: Option<PathBuf>,
    git_executable: Option<PathBuf>,
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
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            if arg == "--allow-executable" {
                let value = args.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--allow-executable requires a path",
                    )
                })?;
                allowed_executables.push(os_string_to_utf8(value, "executable path")?);
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
            } else if let Some(value) = option_value(&arg, "--allow-executable=")? {
                allowed_executables.push(value);
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

        Ok(Self {
            workspace: workspace.unwrap_or(std::env::current_dir()?),
            allowed_executables,
            allowed_env,
            write_scopes,
            delete_scopes,
            mutation_state_dir,
            git_executable,
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

fn set_git_executable(
    slot: &mut Option<PathBuf>,
    value: PathBuf,
) -> Result<(), std::io::Error> {
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
        assert!(
            AppArgs::parse_from(args(&["--git-executable=git", "workspace"])).is_err()
        );

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
    fn mutation_budget_is_bounded_by_transport_and_runtime_limits() {
        let limits = HardLimits::default();
        let budget = mutation_resource_budget(limits);
        assert_eq!(budget.timeout_ms, limits.max_request_duration_ms);
        assert_eq!(budget.output_bytes, limits.max_response_bytes);
        assert_eq!(budget.memory_bytes, limits.max_active_output_ram_bytes);
        assert_eq!(budget.process_count, 1);
    }
}
