//! Real RMCP/JSON-line transport and original registries; no Git or OS child.
use super::*;
use optic_bridge_core::TaskLeaseId;
use optic_bridge_runtime::{OwnedSessionAdmissionPermit, TaskLeaseRegistryError};
use rmcp::{
    ClientHandler,
    model::{CallToolRequestParams, ClientCapabilities, ClientConfig, Implementation},
};
use std::fs;
use tokio::{io::DuplexStream, task::JoinHandle};

type ServiceResult = Result<(), Box<dyn Error + Send + Sync>>;

struct ProbeClient;

impl ClientHandler for ProbeClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            ClientCapabilities::builder().build(),
            Implementation::new("stdio-session-owner-proof", "1"),
        )
    }
}

struct Fixture {
    root: PathBuf,
    sessions: Arc<SessionRegistry>,
    leases: Arc<TaskLeaseRegistry>,
    processes: Arc<ProcessManager>,
    lifecycle: Arc<SessionLifecycleManager>,
    clock: Arc<dyn Clock>,
    session: SessionHandle,
    other_session: SessionHandle,
    lease: TaskLeaseId,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "optic-stdio-owner-{}",
            optic_bridge_core::ActionId::generate().unwrap().to_token()
        ));
        fs::create_dir(&root).unwrap();
        let sessions = Arc::new(
            SessionRegistry::from_hard_limits(HardLimits {
                max_sessions: 2,
                ..HardLimits::default()
            })
            .unwrap(),
        );
        let leases = Arc::new(TaskLeaseRegistry::new());
        let processes =
            Arc::new(ProcessManager::new(&root, HardLimits::default(), Vec::new()).unwrap());
        let lifecycle = Arc::new(SessionLifecycleManager::new(
            Arc::clone(&sessions),
            Arc::clone(&leases),
            Arc::clone(&processes),
        ));
        let clock: Arc<dyn Clock> = Arc::new(StdClock::new());
        let spec = || SessionGrantSpec {
            principal: PrincipalId::new("stdio-proof").unwrap(),
            project: ProjectId::new("synthetic").unwrap(),
            capabilities: BTreeSet::from([Capability::FileRead]),
            expires_at: clock.now().saturating_add_millis(60_000),
            policy_epoch: 1,
        };
        let session = lifecycle.provision(spec(), clock.now()).unwrap().handle;
        let other_session = lifecycle.provision(spec(), clock.now()).unwrap().handle;
        let lease = TaskLeaseId::generate().unwrap();
        leases
            .register(TaskLease {
                id: lease.clone(),
                session: session.clone(),
                capabilities: BTreeSet::from([Capability::FileRead]),
                scopes: BTreeSet::new(),
                resource_ceiling: read_authority_resource_budget(HardLimits::default()),
                workload_class: WorkloadClass::Standard,
                expires_at: clock.now().saturating_add_millis(60_000),
                policy_epoch: 1,
            })
            .unwrap();
        Self {
            root,
            sessions,
            leases,
            processes,
            lifecycle,
            clock,
            session,
            other_session,
            lease,
        }
    }

    fn start(
        &self,
        supervised: bool,
    ) -> (DuplexStream, JoinHandle<(Option<String>, ServiceResult)>) {
        let server = ReadonlyMcpServer::new_with_process_runtime(
            &self.root,
            Arc::clone(&self.sessions),
            self.session.clone(),
            Arc::clone(&self.clock),
            HardLimits::default(),
            Arc::clone(&self.processes),
            Arc::clone(&self.leases),
            BTreeMap::new(),
        )
        .unwrap();
        let (client, stream) = tokio::io::duplex(64 * 1024);
        let (read, write) = tokio::io::split(stream);
        let transport = BoundedJsonLineTransport::new(read, write, 64 * 1024, 64 * 1024).unwrap();
        let lifecycle = Arc::clone(&self.lifecycle);
        let session = self.session.clone();
        let clock = Arc::clone(&self.clock);
        let owner = tokio::spawn(async move {
            let result: ServiceResult = async {
                let service = server.serve(transport).await?;
                let renewal = supervised.then_some((session.clone(), 60_000));
                tokio::select! {
                    result = service.waiting() => result.map(|_| ()).map_err(Into::into),
                    result = supervise_session_expiry(Arc::clone(&lifecycle), clock, renewal) => {
                        result.and_then(|()| Err(std::io::Error::other("unexpected supervisor completion").into()))
                    }
                }
            }
            .await;
            let original_error = result.as_ref().err().map(ToString::to_string);
            let result = finish_original_stdio_session(result, lifecycle, session).await;
            (original_error, result)
        });
        (client, owner)
    }

    fn original_closed(&self) -> bool {
        matches!(
            self.sessions.get_active(&self.session, self.clock.now()),
            Err(SessionRegistryError::Revoked)
        ) && matches!(
            self.leases
                .get_active(&self.lease, &self.session, self.clock.now()),
            Err(TaskLeaseRegistryError::Revoked)
        )
    }

    fn other_active(&self) -> bool {
        self.sessions
            .get_active(&self.other_session, self.clock.now())
            .is_ok()
    }

    fn owned_admission(&self) -> OwnedSessionAdmissionPermit {
        self.sessions
            .begin_owned_admission(&self.session, self.clock.now())
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for session in [&self.session, &self.other_session] {
            match self.lifecycle.revoke(session) {
                Ok(_)
                | Err(SessionLifecycleError::SessionRegistry(
                    SessionRegistryError::UnknownSession,
                )) => {}
                Err(error) => panic!("fixture cleanup: {error}"),
            }
        }
        let canonical = fs::canonicalize(&self.root).unwrap();
        assert_eq!(
            canonical.parent().unwrap(),
            fs::canonicalize(std::env::temp_dir()).unwrap()
        );
        assert!(
            canonical
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("optic-stdio-owner-")
        );
        fs::remove_dir_all(canonical).unwrap();
    }
}

#[tokio::test]
async fn stdio_eof_revokes_original_session_independent_of_renewal_mode() {
    for supervised in [false, true] {
        let fixture = Fixture::new();
        let (client, owner) = fixture.start(supervised);
        let (read, write) = tokio::io::split(client);
        let peer = ProbeClient.serve((read, write)).await.unwrap();
        let request: CallToolRequestParams =
            serde_json::from_value(serde_json::json!({"name":"session_info", "arguments":{}}))
                .unwrap();
        let info = peer.call_tool(request).await.unwrap();
        assert_ne!(info.is_error, Some(true));
        peer.cancel().await.unwrap();
        let (before, result) = tokio::time::timeout(Duration::from_secs(5), owner)
            .await
            .unwrap()
            .unwrap();
        assert!(before.is_none());
        result.unwrap();
        assert!(
            fixture.original_closed(),
            "EOF left original session/lease active; supervised={supervised}"
        );
        assert!(fixture.other_active());
    }
}

#[tokio::test]
async fn stdio_handshake_error_closes_original_session_and_preserves_error() {
    for supervised in [false, true] {
        let fixture = Fixture::new();
        let (client, owner) = fixture.start(supervised);
        drop(client); // Actual transport EOF before initialize, not an injected result.
        let (before, result) = tokio::time::timeout(Duration::from_secs(5), owner)
            .await
            .unwrap()
            .unwrap();
        let error = result.expect_err("initialize EOF must remain an error");
        assert_eq!(Some(error.to_string()), before);
        assert!(
            fixture.original_closed(),
            "failed initialize retained original authority"
        );
        assert!(fixture.other_active());
    }
}

#[tokio::test]
async fn stdio_eof_publishes_stop_but_holds_cleanup_until_original_admission_drains() {
    let fixture = Fixture::new();
    let admitted = fixture.owned_admission();
    let (client, owner) = fixture.start(false);
    let (read, write) = tokio::io::split(client);
    let peer = ProbeClient.serve((read, write)).await.unwrap();
    peer.cancel().await.unwrap();
    let stopped = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if fixture.original_closed() {
                break true;
            }
            if owner.is_finished() {
                break false;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    let waiting = !owner.is_finished();
    let mut replacement = fixture
        .sessions
        .get_active(&fixture.other_session, fixture.clock.now())
        .unwrap();
    replacement.handle = SessionHandle::generate().unwrap();
    let counted = matches!(
        fixture.sessions.register(replacement.clone()),
        Err(SessionRegistryError::CapacityExceeded)
    );
    drop(admitted); // No native process exists; this is synthetic admission cleanup.
    let (_, result) = tokio::time::timeout(Duration::from_secs(5), owner)
        .await
        .unwrap()
        .unwrap();
    result.unwrap();
    assert!(stopped, "transport close did not publish original stop");
    assert!(
        waiting && counted,
        "cleanup released original admission prematurely"
    );
    assert!(fixture.other_active());
    assert!(
        fixture
            .lifecycle
            .try_reap(&fixture.session, fixture.clock.now())
            .unwrap()
            .session_removed
    );
    fixture.sessions.register(replacement.clone()).unwrap();
    fixture.lifecycle.revoke(&replacement.handle).unwrap();
}
