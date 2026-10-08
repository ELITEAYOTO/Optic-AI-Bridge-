#![cfg(windows)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use optic_bridge_core::{HardLimits, ProcessExecutionClass, ResourceBudget, SessionHandle};
use optic_bridge_runtime::{ProcessManager, ProcessStartSpec, ProcessStatus, ProcessStream};

const TEST_TIMEOUT_MS: u64 = 15_000;
const OUTPUT_BYTES: u64 = 64 * 1024;

fn launcher_path() -> &'static str {
    env!("CARGO_BIN_EXE_optic-bridge-isolation-launcher")
}

fn unique_workspace() -> PathBuf {
    std::env::temp_dir().join(format!(
        "optic-runtime-deps-{}-{}",
        std::process::id(),
        SessionHandle::generate()
            .expect("workspace entropy")
            .to_token()
    ))
}

fn canonical_file(path: impl AsRef<Path>) -> Option<PathBuf> {
    fs::canonicalize(path).ok().filter(|value| value.is_file())
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
        .find_map(canonical_file)
}

fn rustup_which(name: &str) -> Option<PathBuf> {
    let output = Command::new("rustup").args(["which", name]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    canonical_file(String::from_utf8_lossy(&output.stdout).trim())
}

fn adjacent_python_runtime_dlls(python: &Path) -> Vec<PathBuf> {
    let Some(parent) = python.parent() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut dlls = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("dll"))
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.to_ascii_lowercase().starts_with("python"))
        })
        .filter_map(canonical_file)
        .collect::<Vec<_>>();
    dlls.sort();
    dlls.truncate(16);
    dlls
}

fn java_runtime_candidates(java: &Path) -> Vec<PathBuf> {
    let Some(bin) = java.parent() else {
        return Vec::new();
    };
    [bin.join("jli.dll"), bin.join("server").join("jvm.dll")]
        .into_iter()
        .filter_map(canonical_file)
        .collect()
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
    panic!("isolated runtime-dependency probe did not reach a terminal state");
}

#[derive(Debug)]
struct ProbeOutcome {
    status: &'static str,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

async fn run_startup_probe(
    manager: &ProcessManager,
    session: &SessionHandle,
    executable: &Path,
    args: Vec<String>,
) -> ProbeOutcome {
    let job = match manager.start(ProcessStartSpec {
        session: session.clone(),
        class: ProcessExecutionClass::RepositoryCode,
        workload_class: optic_bridge_core::WorkloadClass::Standard,
        executable: executable.to_string_lossy().into_owned(),
        args,
        cwd: None,
        workspace_read_files: Vec::new(),
        env_allowlist: Vec::new(),
        resources: ResourceBudget {
            timeout_ms: TEST_TIMEOUT_MS,
            output_bytes: OUTPUT_BYTES,
            memory_bytes: 512 * 1024 * 1024,
            process_count: 1,
        },
    }) {
        Ok(job) => job,
        Err(error) => {
            return ProbeOutcome {
                status: "start_error",
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
            };
        }
    };

    let result = await_terminal(manager, session, &job).await;
    let stdout = manager
        .read(session, &job, ProcessStream::Stdout, 0, OUTPUT_BYTES)
        .expect("read runtime probe stdout");
    let stderr = manager
        .read(session, &job, ProcessStream::Stderr, 0, OUTPUT_BYTES)
        .expect("read runtime probe stderr");
    ProbeOutcome {
        status: match result.status {
            ProcessStatus::Running => "running",
            ProcessStatus::Exited => "exited",
            ProcessStatus::Stopped => "stopped",
            ProcessStatus::TimedOut => "timed_out",
            ProcessStatus::OutputLimitExceeded => "output_limit_exceeded",
            ProcessStatus::TerminationUncertain => "termination_uncertain",
            ProcessStatus::Failed => "failed",
        },
        exit_code: result.exit_code,
        stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(),
        stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(),
    }
}

fn compact(value: &str) -> String {
    let mut value = value.replace(['\r', '\n'], " ");
    value.truncate(value.len().min(500));
    value
}

fn print_probe(label: &str, executable: &Path, outcome: &ProbeOutcome) {
    println!(
        "OPTIC_RUNTIME_DEP probe={label} executable={:?} status={} exit={:?} stdout={:?} stderr={:?}",
        executable,
        outcome.status,
        outcome.exit_code,
        compact(&outcome.stdout),
        compact(&outcome.stderr)
    );
}

#[tokio::test]
async fn runtime_dependencies_are_characterized_without_new_authority() {
    let workspace = unique_workspace();
    fs::create_dir_all(&workspace).expect("create runtime dependency workspace");
    let _cleanup = WorkspaceCleanup(workspace.clone());
    let manager = ProcessManager::new_with_isolation_launcher(
        &workspace,
        HardLimits::default(),
        Vec::new(),
        launcher_path(),
    )
    .expect("process manager with isolation launcher");
    let session = SessionHandle::generate().expect("session");
    let require_hosted = std::env::var_os("OPTIC_REQUIRE_HOSTED_TOOLCHAINS").is_some();

    let cargo_proxy = where_executable("cargo.exe");
    let rustc_proxy = where_executable("rustc.exe");
    let cargo_direct = rustup_which("cargo.exe").or_else(|| rustup_which("cargo"));
    let rustc_direct = rustup_which("rustc.exe").or_else(|| rustup_which("rustc"));

    if require_hosted {
        assert!(cargo_proxy.is_some(), "Windows CI must provide cargo proxy");
        assert!(rustc_proxy.is_some(), "Windows CI must provide rustc proxy");
        assert!(
            cargo_direct.is_some(),
            "rustup must resolve the active cargo binary"
        );
        assert!(
            rustc_direct.is_some(),
            "rustup must resolve the active rustc binary"
        );
    }

    if let (Some(proxy), Some(direct)) = (&cargo_proxy, &cargo_direct) {
        println!(
            "OPTIC_RUNTIME_DEP rust_tool=cargo proxy={:?} direct={:?} same_file={}",
            proxy,
            direct,
            proxy == direct
        );
        let outcome =
            run_startup_probe(&manager, &session, direct, vec!["--version".to_owned()]).await;
        print_probe("cargo_direct_startup", direct, &outcome);
    }

    if let (Some(proxy), Some(direct)) = (&rustc_proxy, &rustc_direct) {
        println!(
            "OPTIC_RUNTIME_DEP rust_tool=rustc proxy={:?} direct={:?} same_file={}",
            proxy,
            direct,
            proxy == direct
        );
        let outcome =
            run_startup_probe(&manager, &session, direct, vec!["--version".to_owned()]).await;
        print_probe("rustc_direct_startup", direct, &outcome);
    }

    let python = where_executable("python.exe");
    if require_hosted {
        assert!(python.is_some(), "Windows CI must provide python.exe");
    }
    if let Some(python) = python {
        let dlls = adjacent_python_runtime_dlls(&python);
        println!(
            "OPTIC_RUNTIME_DEP tool=python executable={:?} adjacent_python_dll_count={}",
            python,
            dlls.len()
        );
        for dll in &dlls {
            println!("OPTIC_RUNTIME_DEP tool=python runtime_candidate={dll:?}");
        }
        if require_hosted {
            assert!(
                !dlls.is_empty(),
                "hosted Python should expose at least one adjacent python*.dll runtime candidate"
            );
        }
    }

    let java = where_executable("java.exe");
    if require_hosted {
        assert!(java.is_some(), "Windows CI must provide java.exe");
    }
    if let Some(java) = java {
        let candidates = java_runtime_candidates(&java);
        println!(
            "OPTIC_RUNTIME_DEP tool=java executable={:?} runtime_candidate_count={}",
            java,
            candidates.len()
        );
        for candidate in &candidates {
            println!("OPTIC_RUNTIME_DEP tool=java runtime_candidate={candidate:?}");
        }
        if require_hosted {
            assert!(
                !candidates.is_empty(),
                "hosted Java should expose at least one known JLI/JVM runtime candidate"
            );
        }
    }
}

struct WorkspaceCleanup(PathBuf);

impl Drop for WorkspaceCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
