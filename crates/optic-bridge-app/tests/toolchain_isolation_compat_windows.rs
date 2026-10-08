#![cfg(windows)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use optic_bridge_core::{
    HardLimits, ProcessExecutionClass, ResourceBudget, SessionHandle, WorkspacePath,
};
use optic_bridge_runtime::{ProcessManager, ProcessStartSpec, ProcessStatus, ProcessStream};

const TEST_TIMEOUT_MS: u64 = 15_000;
const OUTPUT_BYTES: u64 = 64 * 1024;
const SENTINEL: &str = "optic-toolchain-exact-read-sentinel\n";

fn launcher_path() -> &'static str {
    env!("CARGO_BIN_EXE_optic-bridge-isolation-launcher")
}

fn unique_workspace() -> PathBuf {
    std::env::temp_dir().join(format!(
        "optic-toolchain-compat-{}-{}",
        std::process::id(),
        SessionHandle::generate()
            .expect("workspace entropy")
            .to_token()
    ))
}

fn where_executable(name: &str) -> Option<PathBuf> {
    let output = Command::new("where.exe").arg(name).output().ok()?;
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

async fn await_terminal(
    manager: &ProcessManager,
    session: &SessionHandle,
    job: &optic_bridge_core::JobId,
) -> optic_bridge_runtime::ProcessResult {
    for _ in 0..750 {
        let result = manager.result(session, job).expect("read process result");
        if result.status != ProcessStatus::Running {
            return result;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("isolated compatibility probe did not reach a terminal state");
}

#[derive(Debug)]
struct ProbeOutcome {
    status: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl ProbeOutcome {
    fn start_error(error: impl std::fmt::Display) -> Self {
        Self {
            status: "start_error".to_owned(),
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        }
    }

    fn succeeded(&self) -> bool {
        self.status == "exited" && self.exit_code == Some(0)
    }
}

async fn run_probe(
    manager: &ProcessManager,
    session: &SessionHandle,
    class: ProcessExecutionClass,
    executable: &Path,
    args: Vec<String>,
    workspace_read_files: Vec<WorkspacePath>,
) -> ProbeOutcome {
    let job = match manager.start(ProcessStartSpec {
        session: session.clone(),
        class,
        workload_class: optic_bridge_core::WorkloadClass::Standard,
        executable: executable.to_string_lossy().into_owned(),
        args,
        cwd: None,
        workspace_read_files,
        env_allowlist: Vec::new(),
        resources: ResourceBudget {
            timeout_ms: TEST_TIMEOUT_MS,
            output_bytes: OUTPUT_BYTES,
            memory_bytes: 512 * 1024 * 1024,
            process_count: 1,
        },
    }) {
        Ok(job) => job,
        Err(error) => return ProbeOutcome::start_error(error),
    };

    let result = await_terminal(manager, session, &job).await;
    let stdout = manager
        .read(session, &job, ProcessStream::Stdout, 0, OUTPUT_BYTES)
        .expect("read compatibility stdout");
    let stderr = manager
        .read(session, &job, ProcessStream::Stderr, 0, OUTPUT_BYTES)
        .expect("read compatibility stderr");
    ProbeOutcome {
        status: match result.status {
            ProcessStatus::Running => "running",
            ProcessStatus::Exited => "exited",
            ProcessStatus::Stopped => "stopped",
            ProcessStatus::TimedOut => "timed_out",
            ProcessStatus::OutputLimitExceeded => "output_limit_exceeded",
            ProcessStatus::TerminationUncertain => "termination_uncertain",
            ProcessStatus::Failed => "failed",
        }
        .to_owned(),
        exit_code: result.exit_code,
        stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(),
        stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(),
    }
}

fn print_outcome(tool: &str, probe: &str, executable: &Path, outcome: &ProbeOutcome) {
    let compact = |value: &str| {
        let mut value = value.replace(['\r', '\n'], " ");
        value.truncate(value.len().min(500));
        value
    };
    println!(
        "OPTIC_TOOLCHAIN_COMPAT tool={tool} probe={probe} executable={:?} status={} exit={:?} stdout={:?} stderr={:?}",
        executable,
        outcome.status,
        outcome.exit_code,
        compact(&outcome.stdout),
        compact(&outcome.stderr)
    );
}

async fn assert_denied_file_is_not_read(
    manager: &ProcessManager,
    session: &SessionHandle,
    tool: &str,
    class: ProcessExecutionClass,
    executable: &Path,
    args: Vec<String>,
    grants: Vec<WorkspacePath>,
) {
    let denied = run_probe(manager, session, class, executable, args, grants).await;
    print_outcome(tool, "ungranted_read", executable, &denied);
    assert!(
        !(denied.succeeded() && denied.stdout.contains(SENTINEL)),
        "{tool} escaped exact-file authority and read the ungranted sentinel"
    );
}

#[tokio::test]
async fn representative_toolchains_are_characterized_without_widening_authority() {
    let workspace = unique_workspace();
    fs::create_dir_all(workspace.join("src")).expect("create compatibility workspace");
    let _cleanup = WorkspaceCleanup(workspace.clone());

    fs::write(workspace.join("allowed.txt"), SENTINEL).expect("write allowed sentinel");
    fs::write(workspace.join("denied.txt"), SENTINEL).expect("write denied sentinel");
    fs::write(
        workspace.join("OpticCompat.java"),
        r#"import java.nio.file.*; class OpticCompat { public static void main(String[] args) throws Exception { System.out.print(Files.readString(Path.of(args[0]))); } }"#,
    )
    .expect("write Java source probe");
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"optic_compat\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo manifest");
    fs::write(workspace.join("src/main.rs"), "fn main() {}\n").expect("write Rust source");

    let manager = ProcessManager::new_with_isolation_launcher(
        &workspace,
        HardLimits::default(),
        Vec::new(),
        launcher_path(),
    )
    .expect("process manager with isolation launcher");
    let session = SessionHandle::generate().expect("session");
    let require_hosted = std::env::var_os("OPTIC_REQUIRE_HOSTED_TOOLCHAINS").is_some();

    let node = where_executable("node.exe");
    let python = where_executable("python.exe");
    let java = where_executable("java.exe");
    let cargo = where_executable("cargo.exe");
    let rustc = where_executable("rustc.exe");

    if require_hosted {
        assert!(node.is_some(), "Windows CI must provide node.exe");
        assert!(python.is_some(), "Windows CI must provide python.exe");
        assert!(java.is_some(), "Windows CI must provide java.exe");
        assert!(cargo.is_some(), "Windows CI must provide cargo.exe");
        assert!(rustc.is_some(), "Windows CI must provide rustc.exe");
    }

    if let Some(executable) = node {
        let startup = run_probe(
            &manager,
            &session,
            ProcessExecutionClass::Interpreter,
            &executable,
            vec!["--version".to_owned()],
            Vec::new(),
        )
        .await;
        print_outcome("node", "startup", &executable, &startup);

        let allowed = run_probe(
            &manager,
            &session,
            ProcessExecutionClass::Interpreter,
            &executable,
            vec![
                "-e".to_owned(),
                "const fs=require('fs'); process.stdout.write(fs.readFileSync(process.argv[1], 'utf8'))".to_owned(),
                workspace.join("allowed.txt").to_string_lossy().into_owned(),
            ],
            vec![WorkspacePath::parse("allowed.txt").expect("allowed workspace path")],
        )
        .await;
        print_outcome("node", "granted_read", &executable, &allowed);

        assert_denied_file_is_not_read(
            &manager,
            &session,
            "node",
            ProcessExecutionClass::Interpreter,
            &executable,
            vec![
                "-e".to_owned(),
                "const fs=require('fs'); process.stdout.write(fs.readFileSync(process.argv[1], 'utf8'))".to_owned(),
                workspace.join("denied.txt").to_string_lossy().into_owned(),
            ],
            vec![WorkspacePath::parse("allowed.txt").expect("allowed workspace path")],
        )
        .await;
    }

    if let Some(executable) = python {
        let startup = run_probe(
            &manager,
            &session,
            ProcessExecutionClass::Interpreter,
            &executable,
            vec!["--version".to_owned()],
            Vec::new(),
        )
        .await;
        print_outcome("python", "startup", &executable, &startup);

        let allowed = run_probe(
            &manager,
            &session,
            ProcessExecutionClass::Interpreter,
            &executable,
            vec![
                "-c".to_owned(),
                "import sys; sys.stdout.write(open(sys.argv[1], encoding='utf-8').read())"
                    .to_owned(),
                workspace.join("allowed.txt").to_string_lossy().into_owned(),
            ],
            vec![WorkspacePath::parse("allowed.txt").expect("allowed workspace path")],
        )
        .await;
        print_outcome("python", "granted_read", &executable, &allowed);

        assert_denied_file_is_not_read(
            &manager,
            &session,
            "python",
            ProcessExecutionClass::Interpreter,
            &executable,
            vec![
                "-c".to_owned(),
                "import sys; sys.stdout.write(open(sys.argv[1], encoding='utf-8').read())"
                    .to_owned(),
                workspace.join("denied.txt").to_string_lossy().into_owned(),
            ],
            vec![WorkspacePath::parse("allowed.txt").expect("allowed workspace path")],
        )
        .await;
    }

    if let Some(executable) = java {
        let startup = run_probe(
            &manager,
            &session,
            ProcessExecutionClass::Interpreter,
            &executable,
            vec!["-version".to_owned()],
            Vec::new(),
        )
        .await;
        print_outcome("java", "startup", &executable, &startup);

        let allowed = run_probe(
            &manager,
            &session,
            ProcessExecutionClass::Interpreter,
            &executable,
            vec![
                workspace
                    .join("OpticCompat.java")
                    .to_string_lossy()
                    .into_owned(),
                workspace.join("allowed.txt").to_string_lossy().into_owned(),
            ],
            vec![
                WorkspacePath::parse("OpticCompat.java").expect("Java probe path"),
                WorkspacePath::parse("allowed.txt").expect("allowed workspace path"),
            ],
        )
        .await;
        print_outcome("java", "granted_read", &executable, &allowed);

        assert_denied_file_is_not_read(
            &manager,
            &session,
            "java",
            ProcessExecutionClass::Interpreter,
            &executable,
            vec![
                workspace
                    .join("OpticCompat.java")
                    .to_string_lossy()
                    .into_owned(),
                workspace.join("denied.txt").to_string_lossy().into_owned(),
            ],
            vec![WorkspacePath::parse("OpticCompat.java").expect("Java probe path")],
        )
        .await;
    }

    if let Some(executable) = cargo {
        let startup = run_probe(
            &manager,
            &session,
            ProcessExecutionClass::RepositoryCode,
            &executable,
            vec!["--version".to_owned()],
            Vec::new(),
        )
        .await;
        print_outcome("cargo", "startup", &executable, &startup);
    }

    if let Some(executable) = rustc {
        let startup = run_probe(
            &manager,
            &session,
            ProcessExecutionClass::RepositoryCode,
            &executable,
            vec!["--version".to_owned()],
            Vec::new(),
        )
        .await;
        print_outcome("rustc", "startup", &executable, &startup);
    }
}

struct WorkspaceCleanup(PathBuf);

impl Drop for WorkspaceCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
