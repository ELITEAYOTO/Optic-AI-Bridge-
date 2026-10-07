#![cfg(windows)]

use std::{
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

const LAUNCHER_FAILURE_EXIT: i32 = 126;
const REQUEST_LIMIT_BYTES: usize = 64 * 1024;
const TEST_TIMEOUT_MS: u64 = 10_000;

fn launcher_path() -> &'static str {
    env!("CARGO_BIN_EXE_optic-bridge-isolation-launcher")
}

fn system32_executable(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"))
        .join("System32")
        .join(name)
}

fn run_launcher(request: Value) -> Output {
    run_launcher_command(request, false)
}

fn run_launcher_command(request: Value, system_root_only: bool) -> Output {
    let mut command = Command::new(launcher_path());
    if system_root_only {
        command.env_clear().env(
            "SystemRoot",
            std::env::var_os("SystemRoot").expect("SystemRoot"),
        );
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn isolation launcher");
    let payload = serde_json::to_vec(&request).expect("serialize launcher request");
    child
        .stdin
        .take()
        .expect("launcher stdin")
        .write_all(&payload)
        .expect("write launcher request");
    child.wait_with_output().expect("wait isolation launcher")
}

fn launcher_request(executable: &Path, args: &[&str], cwd: &Path) -> Value {
    json!({
        "version": 1,
        "executable": executable.to_string_lossy(),
        "args": args,
        "cwd": cwd.to_string_lossy(),
        "timeout_ms": TEST_TIMEOUT_MS,
    })
}

#[test]
fn launcher_runs_with_systemroot_only_environment() {
    let cmd = system32_executable("cmd.exe");
    let cwd = cmd.parent().expect("System32 parent").to_path_buf();
    let output = run_launcher_command(
        launcher_request(
            &cmd,
            &["/d", "/s", "/c", "echo optic-systemroot-baseline"],
            &cwd,
        ),
        true,
    );

    assert!(
        output.status.success(),
        "launcher failed with SystemRoot-only environment: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("optic-systemroot-baseline"),
        "AppContainer stdout was not forwarded under SystemRoot-only environment"
    );
}

#[test]
fn launcher_captures_appcontainer_stdout() {
    let cmd = system32_executable("cmd.exe");
    let cwd = cmd.parent().expect("System32 parent").to_path_buf();
    let output = run_launcher(launcher_request(
        &cmd,
        &["/d", "/s", "/c", "echo optic-internal-launcher"],
        &cwd,
    ));

    assert!(
        output.status.success(),
        "launcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("optic-internal-launcher"),
        "AppContainer stdout was not forwarded"
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("optic isolation launcher failed"),
        "launcher reported an internal failure"
    );
}

#[test]
fn launcher_captures_appcontainer_stderr() {
    let cmd = system32_executable("cmd.exe");
    let cwd = cmd.parent().expect("System32 parent").to_path_buf();
    let output = run_launcher(launcher_request(
        &cmd,
        &["/d", "/s", "/c", "echo optic-internal-launcher-stderr 1>&2"],
        &cwd,
    ));

    assert!(
        output.status.success(),
        "launcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("optic-internal-launcher-stderr"),
        "AppContainer stderr was not forwarded"
    );
}

#[test]
fn launcher_preserves_ungranted_file_denial() {
    let sentinel = unique_temp_path("sentinel.txt");
    fs::write(&sentinel, b"optic launcher sentinel").expect("write sentinel");
    let _cleanup = Cleanup(sentinel.clone());

    let findstr = system32_executable("findstr.exe");
    let control = Command::new(&findstr)
        .args([
            OsStr::new("/c:optic launcher sentinel"),
            sentinel.as_os_str(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run control findstr");
    assert!(control.success(), "control process must read sentinel");

    let cwd = findstr.parent().expect("System32 parent").to_path_buf();
    let sentinel_arg = sentinel.to_string_lossy().into_owned();
    let output = run_launcher(launcher_request(
        &findstr,
        &["/c:optic launcher sentinel", &sentinel_arg],
        &cwd,
    ));

    assert!(
        !output.status.success(),
        "no-capability AppContainer unexpectedly read the sentinel"
    );
    assert_ne!(
        output.status.code(),
        Some(LAUNCHER_FAILURE_EXIT),
        "launcher failed internally instead of returning the target denial"
    );
}

#[test]
fn launcher_rejects_oversized_internal_request() {
    let mut child = Command::new(launcher_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn isolation launcher");
    let oversized = "x".repeat(REQUEST_LIMIT_BYTES + 1);
    child
        .stdin
        .take()
        .expect("launcher stdin")
        .write_all(oversized.as_bytes())
        .expect("write oversized request");
    let output = child.wait_with_output().expect("wait isolation launcher");

    assert_eq!(output.status.code(), Some(LAUNCHER_FAILURE_EXIT));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("request exceeds byte limit"),
        "oversized request was not rejected by the bounded parser"
    );
}

fn unique_temp_path(suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "optic-isolation-launcher-{}-{nanos}-{suffix}",
        std::process::id()
    ))
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
