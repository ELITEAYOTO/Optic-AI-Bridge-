#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, path::PathBuf, sync::Arc};

use optic_bridge_core::{
    Capability, HardLimits, PrincipalId, ProjectId, SessionGrant, SessionHandle,
};
use optic_bridge_mcp::{BoundedJsonLineTransport, ReadonlyMcpServer};
use optic_bridge_runtime::{Clock, SessionRegistry, StdClock};
use rmcp::ServiceExt;

const INITIAL_SESSION_TTL_MS: u64 = 30 * 60 * 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let root = workspace_root()?;
    let limits = HardLimits::default().validate_nonzero()?;
    let clock = Arc::new(StdClock::new());
    let now = clock.now();
    let session = SessionHandle::generate()
        .map_err(|_| std::io::Error::other("failed to create application session handle"))?;
    let grant = SessionGrant {
        handle: session.clone(),
        principal: PrincipalId::new("local-stdio")
            .ok_or_else(|| std::io::Error::other("invalid local principal id"))?,
        project: ProjectId::new("local-workspace")
            .ok_or_else(|| std::io::Error::other("invalid local project id"))?,
        capabilities: BTreeSet::from([Capability::FileRead, Capability::FileSearch]),
        expires_at: now.saturating_add_millis(INITIAL_SESSION_TTL_MS),
        policy_epoch: 1,
    };

    let sessions = Arc::new(SessionRegistry::new());
    sessions.register(grant)?;
    let server = ReadonlyMcpServer::new(root, sessions, session, clock, limits)?;

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
        "Optic AI Bridge {} — Phase 1B read-only MCP stdio",
        env!("CARGO_PKG_VERSION")
    );
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

fn workspace_root() -> Result<PathBuf, std::io::Error> {
    match std::env::args_os().nth(1) {
        Some(path) => Ok(PathBuf::from(path)),
        None => std::env::current_dir(),
    }
}
