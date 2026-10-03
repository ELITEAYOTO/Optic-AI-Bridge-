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
    TaskLease, TaskLeaseId,
};
use optic_bridge_mcp::{BoundedJsonLineTransport, ReadonlyMcpServer};
use optic_bridge_runtime::{Clock, ProcessManager, SessionRegistry, StdClock, TaskLeaseRegistry};
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

    let mut capabilities = BTreeSet::from([Capability::FileRead, Capability::FileSearch]);
    if !process_leases.is_empty() {
        capabilities.insert(Capability::ProcessRun);
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
        "Optic AI Bridge {} — Phase 1C MCP stdio ({} process executable(s) authorized)",
        env!("CARGO_PKG_VERSION"),
        args.allowed_executables.len()
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
}

impl AppArgs {
    fn parse() -> Result<Self, std::io::Error> {
        let mut workspace = None;
        let mut allowed_executables = Vec::new();
        let mut allowed_env = Vec::new();
        let mut args = std::env::args_os().skip(1);

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
            } else if let Some(value) = option_value(&arg, "--allow-executable=")? {
                allowed_executables.push(value);
            } else if let Some(value) = option_value(&arg, "--allow-env=")? {
                allowed_env.push(value);
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
        })
    }
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
