#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    path::PathBuf,
    sync::Arc,
};

use optic_bridge_core::{
    Capability, HardLimits, LeaseScope, PrincipalId, ProjectId, ResourceBudget, SessionGrant,
    SessionHandle, TaskLease, TaskLeaseId, WorkspacePath,
};
use optic_bridge_mcp::{BoundedJsonLineTransport, ReadonlyMcpServer};
use optic_bridge_runtime::{
    Clock, MutationAuthoritySet, MutationAuthoritySpec, ProcessManager, SessionRegistry, StdClock,
    TaskLeaseRegistry,
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
    let server = ReadonlyMcpServer::new_with_process_runtime(
        &args.workspace,
        sessions,
        session,
        clock,
        limits,
        processes,
        task_leases,
        process_leases,
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
        "Optic AI Bridge {} — MCP stdio ({} process executable(s), {} write scope(s), {} delete scope(s) authorized)",
        env!("CARGO_PKG_VERSION"),
        args.allowed_executables.len(),
        args.write_scopes.len(),
        args.delete_scopes.len(),
    );
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

fn mutation_resource_budget(limits: HardLimits) -> ResourceBudget {
    ResourceBudget {
        timeout_ms: limits.max_request_duration_ms,
        output_bytes: limits.max_response_bytes,
        memory_bytes: limits.max_active_output_ram_bytes,
        process_count: 1,
    }
}

#[derive(Debug)]
struct AppArgs {
    workspace: PathBuf,
    allowed_executables: Vec<String>,
    allowed_env: Vec<String>,
    write_scopes: BTreeSet<LeaseScope>,
    delete_scopes: BTreeSet<LeaseScope>,
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
            } else if let Some(value) = option_value(&arg, "--allow-executable=")? {
                allowed_executables.push(value);
            } else if let Some(value) = option_value(&arg, "--allow-env=")? {
                allowed_env.push(value);
            } else if let Some(value) = option_value(&arg, "--allow-write-scope=")? {
                write_scopes.insert(parse_mutation_scope(value)?);
            } else if let Some(value) = option_value(&arg, "--allow-delete-scope=")? {
                delete_scopes.insert(parse_mutation_scope(value)?);
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

        Ok(Self {
            workspace: workspace.unwrap_or(std::env::current_dir()?),
            allowed_executables,
            allowed_env,
            write_scopes,
            delete_scopes,
        })
    }
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
    }

    #[test]
    fn mutation_scope_flags_are_explicit_and_repeatable() {
        let parsed = AppArgs::parse_from(args(&[
            "--allow-write-scope=prefix:src",
            "--allow-write-scope",
            "prefix:generated",
            "--allow-delete-scope=all",
            "workspace",
        ]))
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
                AppArgs::parse_from(args(&[&format!("--allow-write-scope={value}"), "workspace"]))
                    .is_err(),
                "scope should fail: {value}"
            );
        }
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
