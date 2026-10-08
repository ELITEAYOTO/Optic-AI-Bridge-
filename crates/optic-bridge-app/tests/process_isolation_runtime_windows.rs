#![cfg(windows)]

use std::{path::PathBuf, time::Duration};

use optic_bridge_core::{HardLimits, ProcessExecutionClass, ResourceBudget, SessionHandle};
use optic_bridge_runtime::{ProcessManager, ProcessStartSpec, ProcessStatus, ProcessStream};

const TEST_TIMEOUT_MS: u64 = 10_000;
const OUTPUT_BYTES: u64 = 64 * 1024;

fn launcher_path() -> &'static str {
    env!("CARGO_BIN_EXE_optic-bridge-isolation-launcher")
}

fn system32_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"))
        .join("System32")
        .join(name)
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
    panic!("isolated process did not reach a terminal state");
}

#[tokio::test]
async fn process_manager_routes_interpreter_through_pinned_launcher() {
    let cmd = system32_path("cmd.exe");
    let root = cmd.parent().expect("System32 parent");
    let manager = ProcessManager::new_with_isolation_launcher(
        root,
        HardLimits::default(),
        Vec::new(),
        launcher_path(),
    )
    .expect("process manager with isolation launcher");
    let session = SessionHandle::generate().expect("session");
    let job = manager
        .start(ProcessStartSpec {
            session: session.clone(),
            class: ProcessExecutionClass::Interpreter,
            workload_class: optic_bridge_core::WorkloadClass::Standard,
            network: optic_bridge_core::NetworkAccess::Denied,
            executable: cmd.to_string_lossy().into_owned(),
            args: vec![
                "/d".to_owned(),
                "/s".to_owned(),
                "/c".to_owned(),
                "if defined PATH (exit /b 9) else echo optic-runtime-isolation".to_owned(),
            ],
            cwd: None,
            workspace_read_files: Vec::new(),
            env_allowlist: Vec::new(),
            resources: ResourceBudget {
                timeout_ms: TEST_TIMEOUT_MS,
                output_bytes: OUTPUT_BYTES,
                memory_bytes: 256 * 1024 * 1024,
                process_count: 1,
            },
        })
        .expect("start isolated process through runtime");

    let result = await_terminal(&manager, &session, &job).await;
    let stderr = manager
        .read(&session, &job, ProcessStream::Stderr, 0, OUTPUT_BYTES)
        .expect("read isolated stderr");
    let stderr_text = String::from_utf8_lossy(&stderr.bytes);
    assert_eq!(
        result.status,
        ProcessStatus::Exited,
        "isolated runtime status mismatch: exit_code={:?}, stderr={stderr_text}",
        result.exit_code
    );
    assert_eq!(result.exit_code, Some(0));

    let stdout = manager
        .read(&session, &job, ProcessStream::Stdout, 0, OUTPUT_BYTES)
        .expect("read isolated stdout");
    assert!(
        stdout.eof,
        "isolated stdout must be closed at terminal state"
    );
    assert!(
        String::from_utf8_lossy(&stdout.bytes).contains("optic-runtime-isolation"),
        "target stdout did not traverse the runtime launcher path"
    );

    assert!(
        stderr.eof,
        "isolated stderr must be closed at terminal state"
    );
    assert!(
        !String::from_utf8_lossy(&stderr.bytes).contains("optic isolation launcher failed"),
        "launcher reported an internal failure"
    );
}
