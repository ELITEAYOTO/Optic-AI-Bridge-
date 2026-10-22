//! Windows-only cross-boundary proof, not production Git runner wiring.
use std::{
    collections::BTreeSet,
    env, fs,
    process::Stdio,
    sync::{
        Arc,
        atomic::Ordering,
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use optic_bridge_core::{
    Capability, MonotonicTime, PrincipalId, ProjectId, SessionGrant, SessionHandle,
};
use optic_bridge_windows::LimitedJobObject;
use process_wrap::tokio::CommandWrap;
use tokio::{io::AsyncReadExt, process::Command};

use crate::{SessionRegistry, SessionRegistryError};

const POLL: Duration = Duration::from_millis(10);
const RUN_BOUND: Duration = Duration::from_secs(10);
const CLEANUP_BOUND: Duration = Duration::from_secs(4);
const OUTPUT_CEILING: usize = 16 * 1024;

fn grant(label: &str) -> SessionGrant {
    SessionGrant {
        handle: SessionHandle::generate().expect("synthetic session handle"),
        principal: PrincipalId::new(format!("fixture-{label}")).expect("valid principal"),
        project: ProjectId::new("fixture-project").expect("valid project"),
        capabilities: BTreeSet::from([Capability::FileRead]),
        expires_at: MonotonicTime::from_millis(10_000),
        policy_epoch: 1,
    }
}

async fn read_small_pipe_to_eof<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
) -> Result<(), std::io::Error> {
    tokio::time::timeout(CLEANUP_BOUND, async {
        let mut buffer = [0_u8; 512];
        let mut captured = 0_usize;
        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                return Ok(());
            }
            captured = captured
                .checked_add(n)
                .ok_or_else(|| std::io::Error::other("synthetic output overflow"))?;
            if captured > OUTPUT_CEILING {
                return Err(std::io::Error::other("synthetic output limit reached"));
            }
        }
    })
    .await
    .map_err(|_| std::io::Error::other("synthetic pipe EOF unconfirmed"))?
}

#[test]
fn owned_git_revoke_fixture_child() {
    let Some(root) = env::var_os("OPTIC317_JOINED_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    fs::write(root.join("started"), b"started").expect("child starts");
    thread::sleep(Duration::from_secs(8));
    fs::write(root.join("unexpected_survivor"), b"survived").expect("child survived");
}

#[test]
fn real_session_revoke_stops_a_contained_windows_child_after_caller_abandons_reply() {
    let registry = Arc::new(SessionRegistry::new());
    let session_a = grant("a");
    let session_b = grant("b");
    registry.register(session_a.clone()).expect("register A");
    registry.register(session_b.clone()).expect("register B");

    let now = MonotonicTime::from_millis(1);
    let (flag_a, validated_a) = registry
        .watch_active(&session_a.handle, now)
        .expect("validated session A");
    assert_eq!(validated_a.handle, session_a.handle);
    let (flag_b, _) = registry
        .watch_active(&session_b.handle, now)
        .expect("validated session B");

    let root = env::temp_dir().join(format!(
        "optic317-session-native-{}",
        SessionHandle::generate().expect("unique fixture root").to_token()
    ));
    fs::create_dir(&root).expect("synthetic fixture root");
    let (ready_tx, ready_rx) = mpsc::channel();
    let (observed_tx, observed_rx) = mpsc::channel();
    let (cleanup_tx, cleanup_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = mpsc::channel::<()>();
    let worker_registry = Arc::clone(&registry);
    let worker_handle = session_a.handle.clone();
    let worker_root = root.clone();

    let worker = thread::spawn(move || {
        // The server thread owns its Registry Arc and native child. A caller
        // dropping the result receiver cannot drop these owner resources.
        let _permit = worker_registry
            .begin_admission(&worker_handle, now)
            .expect("service-owned admission");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("local synthetic runtime");
        runtime.block_on(async move {
            let mut command = Command::new(env::current_exe().expect("test executable"));
            command
                .args([
                    "--exact",
                    "session_git_owner_integration::owned_git_revoke_fixture_child",
                    "--nocapture",
                ])
                .env_clear()
                .env("OPTIC317_JOINED_ROOT", &worker_root)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            for key in ["SystemRoot", "WINDIR", "TEMP"] {
                if let Some(value) = env::var_os(key) {
                    command.env(key, value);
                }
            }
            let mut command = CommandWrap::from(command);
            command.wrap(
                LimitedJobObject::new(1, 512 * 1024 * 1024, 25)
                    .expect("finite native Job limits"),
            );
            let mut child = command.spawn().expect("suspended assigned native child");
            let stdout = child.stdout().take().expect("stdout pipe");
            let stderr = child.stderr().take().expect("stderr pipe");
            let until = Instant::now() + RUN_BOUND;
            while !worker_root.join("started").exists() {
                assert!(Instant::now() < until, "native fixture never started");
                tokio::time::sleep(POLL).await;
            }
            ready_tx.send(()).expect("native owner registered");
            while !flag_a.load(Ordering::Acquire) {
                assert!(Instant::now() < until, "real revocation signal missing");
                tokio::time::sleep(POLL).await;
            }
            observed_tx.send(()).expect("native owner observed stop");
            cleanup_rx
                .recv_timeout(CLEANUP_BOUND)
                .expect("allow owned cleanup");

            // LimitedJobChild::start_kill terminates the entire owned Job.
            // Its try_wait reports terminality only when root exited AND
            // native Job accounting confirms the descendant tree is empty.
            child.start_kill().expect("terminate native owned Job");
            let cleanup_until = Instant::now() + CLEANUP_BOUND;
            loop {
                if child.try_wait().expect("native owner termination query").is_some() {
                    break;
                }
                assert!(
                    Instant::now() < cleanup_until,
                    "native process tree terminality unconfirmed"
                );
                tokio::time::sleep(POLL).await;
            }
            read_small_pipe_to_eof(stdout)
                .await
                .expect("stdout EOF with bounded capture");
            read_small_pipe_to_eof(stderr)
                .await
                .expect("stderr EOF with bounded capture");
            assert!(!worker_root.join("unexpected_survivor").exists());
        });
        // Permit drops only after both native proof and bounded EOF reads.
        assert!(reply_tx.send(()).is_err(), "requester reply already dropped");
    });

    drop(reply_rx);
    ready_rx
        .recv_timeout(RUN_BOUND)
        .expect("service owns started native child");

    let revoke_registry = Arc::clone(&registry);
    let revoke_handle = session_a.handle.clone();
    let (revoke_tx, revoke_rx) = mpsc::channel();
    let revoker = thread::spawn(move || {
        revoke_tx
            .send(revoke_registry.revoke(&revoke_handle))
            .expect("revoke result");
    });
    observed_rx
        .recv_timeout(RUN_BOUND)
        .expect("native worker sees real revoke signal");
    assert!(matches!(
        registry.begin_admission(&session_a.handle, now),
        Err(SessionRegistryError::Revoked)
    ));
    assert!(!flag_b.load(Ordering::Acquire));
    let other = registry
        .begin_admission(&session_b.handle, now)
        .expect("unrelated session remains active");
    drop(other);
    assert!(matches!(revoke_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    cleanup_tx.send(()).expect("finish owned native Job");
    worker.join().expect("service worker completed");
    assert!(
        revoke_rx
            .recv_timeout(RUN_BOUND)
            .expect("revoke released after native owner")
            .expect("revoke succeeded")
    );
    revoker.join().expect("revoker joined");
    assert!(!flag_b.load(Ordering::Acquire));
    fs::remove_dir_all(root).expect("synthetic fixture cleanup");
}
