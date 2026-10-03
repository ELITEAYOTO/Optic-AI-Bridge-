#![cfg(windows)]

use std::{fs, path::PathBuf, time::Duration};

use optic_bridge_core::{ActionId, HardLimits, ResourceBudget, SessionHandle};
use optic_bridge_runtime::{
    ProcessManager, ProcessStartSpec, ProcessStatus, ProcessStream,
};

fn workspace(label: &str) -> PathBuf {
    let token = ActionId::generate().expect("test entropy").to_token();
    let root = std::env::temp_dir().join(format!("optic-win-job-{label}-{token}"));
    fs::create_dir_all(&root).expect("create test workspace");
    root
}

fn fixture() -> String {
    env!("CARGO_BIN_EXE_optic-process-fixture").to_owned()
}

fn budget(timeout_ms: u64, memory_bytes: u64, process_count: u32) -> ResourceBudget {
    ResourceBudget {
        timeout_ms,
        output_bytes: 128 * 1024,
        memory_bytes,
        process_count,
    }
}

fn spec(
    session: SessionHandle,
    args: impl IntoIterator<Item = String>,
    resources: ResourceBudget,
) -> ProcessStartSpec {
    ProcessStartSpec {
        session,
        executable: fixture(),
        args: args.into_iter().collect(),
        cwd: None,
        env_allowlist: Vec::new(),
        resources,
    }
}

async fn await_terminal(
    manager: &ProcessManager,
    session: &SessionHandle,
    job: &optic_bridge_core::JobId,
) -> optic_bridge_runtime::ProcessResult {
    for _ in 0..500 {
        let result = manager.result(session, job).expect("read process result");
        if result.status != ProcessStatus::Running {
            return result;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("process did not reach a terminal state");
}

fn stdout_text(
    manager: &ProcessManager,
    session: &SessionHandle,
    job: &optic_bridge_core::JobId,
) -> String {
    let chunk = manager
        .read(session, job, ProcessStream::Stdout, 0, 128 * 1024)
        .expect("read stdout");
    String::from_utf8_lossy(&chunk.bytes).into_owned()
}

#[tokio::test]
async fn active_process_limit_blocks_descendant_creation() {
    let root = workspace("process-limit");
    let manager = ProcessManager::new(&root, HardLimits::default(), Vec::new())
        .expect("process manager");
    let session = SessionHandle::generate().expect("session id");
    let job = manager
        .start(spec(
            session.clone(),
            ["spawn-once".to_owned()],
            budget(3_000, 512 * 1024 * 1024, 1),
        ))
        .expect("start fixture");

    let result = await_terminal(&manager, &session, &job).await;
    assert_eq!(result.status, ProcessStatus::Exited);
    let stdout = stdout_text(&manager, &session, &job);
    assert!(
        stdout.contains("spawn-blocked-"),
        "expected the kernel process limit to block/terminate the descendant, got: {stdout:?}"
    );
    assert!(!stdout.contains("spawned-ok"));

    fs::remove_dir_all(root).expect("remove test workspace");
}

#[tokio::test]
async fn job_memory_limit_prevents_large_allocation() {
    let root = workspace("memory-limit");
    let manager = ProcessManager::new(&root, HardLimits::default(), Vec::new())
        .expect("process manager");
    let session = SessionHandle::generate().expect("session id");
    let target = 384_u64 * 1024 * 1024;
    let job = manager
        .start(spec(
            session.clone(),
            ["allocate".to_owned(), target.to_string()],
            budget(5_000, 128 * 1024 * 1024, 1),
        ))
        .expect("start fixture");

    let result = await_terminal(&manager, &session, &job).await;
    let stdout = stdout_text(&manager, &session, &job);
    assert!(
        !stdout.contains("memory-allocated:"),
        "fixture reached a target larger than the Job Object memory ceiling: {stdout:?}"
    );
    assert!(
        result.exit_code != Some(0) || stdout.contains("memory-blocked:"),
        "memory-constrained process exited successfully without reporting allocation refusal: {stdout:?}"
    );

    fs::remove_dir_all(root).expect("remove test workspace");
}

#[tokio::test]
async fn timeout_kills_descendant_tree_before_it_can_survive() {
    let root = workspace("tree-timeout");
    let marker = root.join("descendant-survived.txt");
    let manager = ProcessManager::new(&root, HardLimits::default(), Vec::new())
        .expect("process manager");
    let session = SessionHandle::generate().expect("session id");
    let job = manager
        .start(spec(
            session.clone(),
            [
                "tree-parent".to_owned(),
                marker.to_string_lossy().into_owned(),
            ],
            budget(300, 512 * 1024 * 1024, 4),
        ))
        .expect("start fixture");

    let result = await_terminal(&manager, &session, &job).await;
    assert_eq!(result.status, ProcessStatus::TimedOut);
    let stdout = stdout_text(&manager, &session, &job);
    assert!(
        stdout.contains("tree-child-started:"),
        "fixture descendant was not observed before timeout: {stdout:?}"
    );

    tokio::time::sleep(Duration::from_millis(900)).await;
    assert!(
        !marker.exists(),
        "descendant escaped the Job Object and survived long enough to write its marker"
    );

    fs::remove_dir_all(root).expect("remove test workspace");
}
