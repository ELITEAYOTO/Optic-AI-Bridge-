#![cfg(windows)]

use std::{
    env, fs,
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use optic_bridge_core::{HardLimits, ProcessExecutionClass, ResourceBudget, SessionHandle};
use optic_bridge_runtime::{ProcessManager, ProcessStartSpec, ProcessStatus, ProcessStream};

const PROCESS_TIMEOUT_MS: u64 = 10_000;
const OUTPUT_BYTES: u64 = 32 * 1024;
const FIXED_TOOL_PROBE_PORT_FILE: &str = "fixed-tool-loopback-port";

fn launcher_path() -> &'static str {
    env!("CARGO_BIN_EXE_optic-bridge-isolation-launcher")
}

fn unique_workspace() -> PathBuf {
    std::env::temp_dir().join(format!(
        "optic-network-isolation-{}-{}",
        std::process::id(),
        SessionHandle::generate()
            .expect("workspace entropy")
            .to_token()
    ))
}

fn where_node() -> Option<PathBuf> {
    let output = Command::new("where.exe").arg("node.exe").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find_map(|path| fs::canonicalize(path).ok().filter(|value| value.is_file()))
}

fn accept_within(listener: &TcpListener, timeout: Duration) -> Option<SocketAddr> {
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, address)) => {
                drop(stream);
                return Some(address);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("loopback listener accept failed: {error}"),
        }
        if started.elapsed() >= timeout {
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

async fn await_terminal(
    manager: &ProcessManager,
    session: &SessionHandle,
    job: &optic_bridge_core::JobId,
) -> optic_bridge_runtime::ProcessResult {
    for _ in 0..600 {
        let result = manager
            .result(session, job)
            .expect("read network probe result");
        if result.status != ProcessStatus::Running {
            return result;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("network isolation probe did not reach a terminal state");
}

#[test]
fn fixed_tool_loopback_probe_child() {
    let cwd = env::current_dir().expect("fixed-tool probe cwd");
    let port_file = cwd.join(FIXED_TOOL_PROBE_PORT_FILE);
    let Ok(port) = fs::read_to_string(port_file) else {
        // Normal cargo test execution reaches this helper without a probe
        // fixture. Only the parent characterization test creates the marker.
        return;
    };
    let port = port
        .trim()
        .parse::<u16>()
        .expect("parse fixed-tool loopback port");

    println!("FIXED_TOOL_NETWORK_ATTEMPT");
    let stream = TcpStream::connect(("127.0.0.1", port))
        .expect("direct FixedTool loopback connection should characterize current host behavior");
    println!("FIXED_TOOL_NETWORK_CONNECTED");
    drop(stream);
}

#[tokio::test]
async fn direct_fixed_tool_path_can_open_tcp_ipv4_loopback_socket() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fixed-tool loopback listener");
    listener
        .set_nonblocking(true)
        .expect("set fixed-tool listener nonblocking");
    let address = listener
        .local_addr()
        .expect("read fixed-tool listener address");

    let baseline = TcpStream::connect(address).expect("host loopback baseline must connect");
    assert!(
        accept_within(&listener, Duration::from_secs(1)).is_some(),
        "host listener baseline must accept a normal host connection"
    );
    drop(baseline);

    let workspace = unique_workspace();
    fs::create_dir_all(&workspace).expect("create fixed-tool characterization workspace");
    let _cleanup = WorkspaceCleanup(workspace.clone());
    fs::write(
        workspace.join(FIXED_TOOL_PROBE_PORT_FILE),
        address.port().to_string(),
    )
    .expect("write fixed-tool loopback port fixture");

    let executable = env::current_exe()
        .expect("current network test executable")
        .canonicalize()
        .expect("canonical network test executable")
        .to_string_lossy()
        .into_owned();
    let manager = ProcessManager::new(&workspace, HardLimits::default(), Vec::new())
        .expect("direct fixed-tool process manager");
    let session = SessionHandle::generate().expect("fixed-tool session");

    // B-02 characterization only: FixedTool is intentionally the direct
    // ProcessManager path. No NetworkAccess capability or network authority is
    // introduced here. This locks the current TCP/IPv4 loopback fact before an
    // enforcement primitive is selected: this direct path can open that socket.
    let job = manager
        .start(ProcessStartSpec {
            session: session.clone(),
            class: ProcessExecutionClass::FixedTool,
            workload_class: optic_bridge_core::WorkloadClass::Standard,
            executable: executable.clone(),
            args: vec![
                "--exact".to_owned(),
                "fixed_tool_loopback_probe_child".to_owned(),
                "--nocapture".to_owned(),
            ],
            cwd: None,
            workspace_read_files: Vec::new(),
            env_allowlist: Vec::new(),
            resources: ResourceBudget {
                timeout_ms: PROCESS_TIMEOUT_MS,
                output_bytes: OUTPUT_BYTES,
                memory_bytes: 256 * 1024 * 1024,
                process_count: 1,
            },
        })
        .expect("start direct FixedTool network characterization probe");

    let result = await_terminal(&manager, &session, &job).await;
    let stdout = manager
        .read(&session, &job, ProcessStream::Stdout, 0, OUTPUT_BYTES)
        .expect("read fixed-tool probe stdout");
    let stderr = manager
        .read(&session, &job, ProcessStream::Stderr, 0, OUTPUT_BYTES)
        .expect("read fixed-tool probe stderr");
    let stdout = String::from_utf8_lossy(&stdout.bytes).into_owned();
    let stderr = String::from_utf8_lossy(&stderr.bytes).into_owned();
    let accepted = accept_within(&listener, Duration::from_secs(1));

    println!(
        "OPTIC_FIXED_TOOL_NETWORK_CHARACTERIZATION executable={executable:?} status={:?} exit={:?} accepted={accepted:?} stdout={stdout:?} stderr={stderr:?}",
        result.status, result.exit_code
    );

    assert_eq!(
        result.status,
        ProcessStatus::Exited,
        "direct FixedTool probe must reach a normal terminal state"
    );
    assert_eq!(
        result.exit_code,
        Some(0),
        "direct FixedTool TCP/IPv4 loopback probe currently succeeds on the unconstrained host path"
    );
    assert!(
        stdout.contains("FIXED_TOOL_NETWORK_ATTEMPT"),
        "FixedTool child must reach the network attempt"
    );
    assert!(
        stdout.contains("FIXED_TOOL_NETWORK_CONNECTED"),
        "FixedTool child must characterize the current successful loopback connection"
    );
    assert!(
        accepted.is_some(),
        "host listener must observe the direct FixedTool TCP/IPv4 loopback connection"
    );
}

#[tokio::test]
async fn capability_free_appcontainer_denies_node_loopback_connection() {
    let require_hosted = std::env::var_os("OPTIC_REQUIRE_HOSTED_TOOLCHAINS").is_some();
    let node = where_node();
    if require_hosted {
        assert!(node.is_some(), "Windows CI must provide node.exe");
    }
    let Some(node) = node else {
        return;
    };

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback listener");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");
    let address = listener.local_addr().expect("read listener address");

    let baseline = TcpStream::connect(address).expect("host loopback baseline must connect");
    assert!(
        accept_within(&listener, Duration::from_secs(1)).is_some(),
        "host listener baseline must accept a normal host connection"
    );
    drop(baseline);

    let workspace = unique_workspace();
    fs::create_dir_all(&workspace).expect("create network-isolation workspace");
    let _cleanup = WorkspaceCleanup(workspace.clone());
    let manager = ProcessManager::new_with_isolation_launcher(
        &workspace,
        HardLimits::default(),
        Vec::new(),
        launcher_path(),
    )
    .expect("process manager with isolation launcher");
    let session = SessionHandle::generate().expect("session");

    let script = format!(
        "const net=require('net');\n\
         console.log('NETWORK_ATTEMPT');\n\
         const socket=net.createConnection({{host:'127.0.0.1',port:{}}});\n\
         const timer=setTimeout(()=>{{console.error('NETWORK_TIMEOUT');socket.destroy();process.exit(43);}},1500);\n\
         socket.once('connect',()=>{{clearTimeout(timer);console.log('NETWORK_CONNECTED');socket.end();process.exit(0);}});\n\
         socket.once('error',(err)=>{{clearTimeout(timer);console.error('NETWORK_DENIED:'+(err.code||err.message));process.exit(42);}});",
        address.port()
    );

    let job = manager
        .start(ProcessStartSpec {
            session: session.clone(),
            class: ProcessExecutionClass::Interpreter,
            workload_class: optic_bridge_core::WorkloadClass::Standard,
            executable: node.to_string_lossy().into_owned(),
            args: vec!["-e".to_owned(), script],
            cwd: None,
            workspace_read_files: Vec::new(),
            env_allowlist: Vec::new(),
            resources: ResourceBudget {
                timeout_ms: PROCESS_TIMEOUT_MS,
                output_bytes: OUTPUT_BYTES,
                memory_bytes: 256 * 1024 * 1024,
                process_count: 1,
            },
        })
        .expect("start isolated Node network probe");

    let result = await_terminal(&manager, &session, &job).await;
    let stdout = manager
        .read(&session, &job, ProcessStream::Stdout, 0, OUTPUT_BYTES)
        .expect("read network probe stdout");
    let stderr = manager
        .read(&session, &job, ProcessStream::Stderr, 0, OUTPUT_BYTES)
        .expect("read network probe stderr");
    let stdout = String::from_utf8_lossy(&stdout.bytes).into_owned();
    let stderr = String::from_utf8_lossy(&stderr.bytes).into_owned();
    let accepted = accept_within(&listener, Duration::from_millis(500));

    println!(
        "OPTIC_NETWORK_ISOLATION executable={node:?} status={:?} exit={:?} accepted={accepted:?} stdout={stdout:?} stderr={stderr:?}",
        result.status, result.exit_code
    );

    assert_eq!(
        result.status,
        ProcessStatus::Exited,
        "Node must execute the probe to a normal exit"
    );
    assert!(
        stdout.contains("NETWORK_ATTEMPT"),
        "Node script must reach the network attempt"
    );
    assert!(
        !stdout.contains("NETWORK_CONNECTED"),
        "capability-free AppContainer must not report a successful loopback connection"
    );
    assert_ne!(
        result.exit_code,
        Some(0),
        "capability-free AppContainer network attempt must not succeed"
    );
    assert!(
        matches!(result.exit_code, Some(42 | 43)),
        "probe must fail by socket error or bounded connection timeout, not by unrelated runtime failure"
    );
    assert!(
        accepted.is_none(),
        "host listener must not accept a connection from the capability-free AppContainer"
    );
}

struct WorkspaceCleanup(PathBuf);

impl Drop for WorkspaceCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
