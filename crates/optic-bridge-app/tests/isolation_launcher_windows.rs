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

fn run_launcher_command(request: Value, minimal_windows_environment: bool) -> Output {
    let mut command = Command::new(launcher_path());
    if minimal_windows_environment {
        command.env_clear();
        for name in ["SystemRoot", "LOCALAPPDATA", "TEMP", "TMP"] {
            command.env(
                name,
                std::env::var_os(name).unwrap_or_else(|| panic!("{name}")),
            );
        }
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
        "version": 2,
        "executable": executable.to_string_lossy(),
        "args": args,
        "cwd": cwd.to_string_lossy(),
        "timeout_ms": TEST_TIMEOUT_MS,
        "workspace_root": cwd.to_string_lossy(),
        "workspace_read_files": [],
    })
}

fn launcher_request_with_read_files(
    executable: &Path,
    args: &[&str],
    cwd: &Path,
    read_files: &[&Path],
    workspace_root: &Path,
) -> Value {
    json!({
        "version": 2,
        "executable": executable.to_string_lossy(),
        "args": args,
        "cwd": cwd.to_string_lossy(),
        "timeout_ms": TEST_TIMEOUT_MS,
        "workspace_root": workspace_root.to_string_lossy(),
        "workspace_read_files": read_files.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>(),
    })
}

#[test]
fn launcher_runs_with_minimal_windows_environment() {
    let cmd = system32_executable("cmd.exe");
    let cwd = cmd.parent().expect("System32 parent").to_path_buf();
    let output = run_launcher_command(
        launcher_request(
            &cmd,
            &["/d", "/s", "/c", "echo optic-minimal-windows-baseline"],
            &cwd,
        ),
        true,
    );

    assert!(
        output.status.success(),
        "launcher failed with minimal Windows environment: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("optic-minimal-windows-baseline"),
        "AppContainer stdout was not forwarded under minimal Windows environment"
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
fn launcher_exact_file_grant_supports_command_interpreter_read() {
    let granted = unique_temp_path("interpreter-granted.txt");
    let ungranted = unique_temp_path("interpreter-ungranted.txt");
    fs::write(&granted, b"optic interpreter granted sentinel").expect("write granted sentinel");
    fs::write(&ungranted, b"optic interpreter ungranted sentinel")
        .expect("write ungranted sentinel");
    let _granted_cleanup = Cleanup(granted.clone());
    let _ungranted_cleanup = Cleanup(ungranted.clone());

    let cmd = system32_executable("cmd.exe");
    let cwd = cmd.parent().expect("System32 parent").to_path_buf();
    let workspace_root = granted.parent().expect("temporary workspace root");
    let granted_command = format!("type {}", granted.to_string_lossy());
    let output = run_launcher(launcher_request_with_read_files(
        &cmd,
        &["/d", "/c", &granted_command],
        &cwd,
        &[granted.as_path()],
        workspace_root,
    ));

    assert!(
        output.status.success(),
        "command interpreter could not read its exact grant: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("optic interpreter granted sentinel"),
        "granted exact file was not readable through the isolated interpreter"
    );

    let ungranted_command = format!("type {}", ungranted.to_string_lossy());
    let denied = run_launcher(launcher_request_with_read_files(
        &cmd,
        &["/d", "/c", &ungranted_command],
        &cwd,
        &[granted.as_path()],
        workspace_root,
    ));
    assert!(
        !denied.status.success(),
        "isolated interpreter unexpectedly read an ungranted sibling file"
    );
    assert_ne!(
        denied.status.code(),
        Some(LAUNCHER_FAILURE_EXIT),
        "launcher failed internally instead of returning the target access denial"
    );
}

#[test]
fn launcher_rejects_read_grant_outside_workspace_root() {
    let sentinel = unique_temp_path("outside-workspace-sentinel.txt");
    fs::write(&sentinel, b"optic outside workspace sentinel").expect("write sentinel");
    let _cleanup = Cleanup(sentinel.clone());
    let findstr = system32_executable("findstr.exe");
    let cwd = findstr.parent().expect("System32 parent").to_path_buf();
    let sentinel_arg = sentinel.to_string_lossy().into_owned();

    let output = run_launcher(launcher_request_with_read_files(
        &findstr,
        &["/c:optic outside workspace sentinel", &sentinel_arg],
        &cwd,
        &[sentinel.as_path()],
        &cwd,
    ));

    assert_eq!(output.status.code(), Some(LAUNCHER_FAILURE_EXIT));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("workspace grant target resolves outside the configured workspace"),
        "out-of-workspace grant did not fail at the helper boundary: {}",
        String::from_utf8_lossy(&output.stderr)
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
