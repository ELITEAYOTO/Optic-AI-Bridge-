//! Unwired read-only Node task controller; no dependency/build/GUI authority.
use crate::process::NativeProcessReservation;
use crate::{
    BlockingIoGovernor, PreparedPrivateProject, PrivateProjectStageError as Error,
    ProcessAuthority, ProcessManager, ProcessStartSpec, ProcessStatus, ProcessStream,
    ProjectInputCaptureService, RequestPermit,
};
use optic_bridge_core::{
    ActionEnvelope, Effect, JobId, NetworkAccess, ProcessExecutionClass, WorkspacePath,
};
use optic_bridge_windows::{
    AppContainerStdio, CapturedOutputPipe, ContainedAppContainerProcess,
    spawn_appcontainer_with_environment_suspended,
};
use std::{
    ffi::OsString,
    fs::OpenOptions,
    os::windows::io::AsHandle,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

/// Operator-derived identity/action plus an entry already present in the sealed
/// manifest. There are no arbitrary argv, environment or filesystem-write flags.
pub struct PrivateNodeTaskStart {
    pub action: ActionEnvelope,
    pub runtime: Arc<ProcessAuthority>,
    pub processes: Arc<ProcessManager>,
    pub entry: WorkspacePath,
}

pub(crate) struct PrivateNodeTaskCommand {
    pub(crate) start: PrivateNodeTaskStart,
    pub(crate) request: RequestPermit,
    pub(crate) io: tokio::sync::OwnedSemaphorePermit,
    pub(crate) reply: tokio::sync::oneshot::Sender<Result<JobId, Error>>,
}

struct Running {
    record: NativeProcessReservation,
    process: ContainedAppContainerProcess,
    stdout: CapturedOutputPipe,
    stderr: CapturedOutputPipe,
}

fn drain_once(
    pipe: &mut CapturedOutputPipe,
    record: &NativeProcessReservation,
    stream: ProcessStream,
) -> Result<bool, Error> {
    let chunk = pipe.try_read(8192)?;
    record.append(stream, &chunk.bytes);
    Ok(chunk.eof)
}

pub(crate) fn execute(
    stage: PreparedPrivateProject,
    capture: &ProjectInputCaptureService,
    reads: &[ActionEnvelope],
    command: PrivateNodeTaskCommand,
    io: &BlockingIoGovernor,
    runtime: &tokio::runtime::Handle,
) -> Result<(), Error> {
    // An unwinding worker must never reclaim a profile while its tree may live.
    // Only the explicit known-empty paths recover ownership for cleanup.
    let mut stage = std::mem::ManuallyDrop::new(stage);
    let PrivateNodeTaskCommand {
        start,
        request,
        io: start_io,
        reply,
    } = command;
    let mut reply = Some(reply);
    let mut record = None;
    let mut native = None;
    let mut stdout = None;
    let mut stderr = None;
    let mut root_pin = None;
    let mut native_created = false;
    let initialization = (|| -> Result<(), Error> {
        let first_read = reads.first().ok_or(Error::InvalidPrivateTask)?;
        stage.require_binding(
            &start.action.session,
            first_read
                .task_lease
                .as_ref()
                .ok_or(Error::InvalidPrivateTask)?,
            start.action.policy_epoch,
        )?;
        let (_admission, now) = capture.capture_admission(&start.action)?;
        if request.is_expired_at(now) || reply.as_ref().unwrap().is_closed() {
            return Err(Error::WorkerUnavailable);
        }
        for read in reads {
            capture.live_lease(read)?;
        }
        let lease = capture.live_lease(&start.action)?;
        let (executable, class) = match &start.action.effect {
            Effect::ProcessRun {
                executable,
                class,
                network: NetworkAccess::Denied,
            } if matches!(
                class,
                ProcessExecutionClass::Interpreter | ProcessExecutionClass::RepositoryCode
            ) =>
            {
                (executable, *class)
            }
            _ => return Err(Error::InvalidPrivateTask),
        };
        if start.action.task_lease.as_ref() != Some(start.runtime.lease_id())
            || start.runtime.execution_class() != class
            || start.runtime.canonical_path() != executable
            || !Path::new(executable)
                .file_name()
                .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("node.exe"))
            || !stage
                .manifest()
                .files()
                .iter()
                .any(|input| input.path == start.entry)
        {
            return Err(Error::InvalidPrivateTask);
        }
        start.runtime.verify()?;
        root_pin = Some(stage.verify_inputs_and_pin()?);
        let root = node_local_path(stage.input_root())?;
        let entry = node_local_path(&stage.input_root().join(start.entry.as_str()))?;
        let args = vec![
            "--permission".into(),
            format!("--allow-fs-read={root}"),
            "--preserve-symlinks".into(),
            "--preserve-symlinks-main".into(),
            entry,
        ];
        let spec = ProcessStartSpec {
            session: start.action.session.clone(),
            class,
            workload_class: lease.workload_class,
            network: NetworkAccess::Denied,
            executable: executable.clone(),
            args: args.clone(),
            cwd: None,
            workspace_read_files: Vec::new(),
            env_allowlist: Vec::new(),
            resources: start.action.resources,
        };
        record = Some(start.processes.reserve_native_private(&spec)?);
        stdout = Some(CapturedOutputPipe::new()?);
        stderr = Some(CapturedOutputPipe::new()?);
        let environment = stage.node_environment()?;
        let input = OpenOptions::new().read(true).open("NUL")?;
        // Recheck after identity hashing/internal preparation and before the
        // first process effect. No source paths/ambient credentials are granted.
        let (_last_admission, now) = capture.capture_admission(&start.action)?;
        if request.is_expired_at(now) {
            return Err(Error::WorkerUnavailable);
        }
        capture.live_lease(&start.action)?;
        for read in reads {
            capture.live_lease(read)?;
        }
        let child = spawn_appcontainer_with_environment_suspended(
            stage.execution_profile(),
            Path::new(executable),
            &args.into_iter().map(OsString::from).collect::<Vec<_>>(),
            stage.input_root(),
            AppContainerStdio {
                stdin: input.as_handle(),
                stdout: stdout.as_ref().unwrap().writer_handle()?,
                stderr: stderr.as_ref().unwrap().writer_handle()?,
            },
            &environment,
        )?;
        native_created = true;
        record.as_mut().unwrap().mark_native_created();
        native = Some(ContainedAppContainerProcess::contain(
            child,
            record.as_ref().unwrap().limits,
        )?);
        stdout.as_mut().unwrap().close_parent_writer();
        stderr.as_mut().unwrap().close_parent_writer();
        native.as_ref().unwrap().resume()?;
        Ok(())
    })();
    // The start effect has a definite outcome or an owned record now. A lost
    // response never retries start; the shared manager retains owner-scoped IDs.
    drop(request);
    drop(start_io);
    if let Err(error) = initialization {
        let known_empty = match native.as_mut() {
            Some(process) => process.terminate_and_confirm_empty(2000).is_ok(),
            None => !native_created,
        };
        drop(native);
        drop(root_pin);
        if known_empty {
            let cleanup = cleanup_stage(std::mem::ManuallyDrop::into_inner(stage), io, runtime);
            if let Some(record) = record.as_mut() {
                record.finish(ProcessStatus::Failed, None);
            }
            let _ = reply.take().unwrap().send(Err(error));
            return cleanup;
        }
        std::mem::forget(stage);
        if let Some(record) = record.as_mut() {
            record.finish(ProcessStatus::TerminationUncertain, None);
        }
        let _ = reply.take().unwrap().send(Err(error));
        return Err(Error::WorkerUnavailable);
    }
    let mut running = Running {
        record: record.take().unwrap(),
        process: native.take().unwrap(),
        stdout: stdout.take().unwrap(),
        stderr: stderr.take().unwrap(),
    };
    let _ = reply.take().unwrap().send(Ok(running.record.id()));
    let deadline = Instant::now() + Duration::from_millis(start.action.resources.timeout_ms);
    let (mut status, mut exit_code) = loop {
        let pipe_error = drain_once(&mut running.stdout, &running.record, ProcessStream::Stdout)
            .is_err()
            || drain_once(&mut running.stderr, &running.record, ProcessStream::Stderr).is_err();
        let reason = if pipe_error {
            Some(ProcessStatus::Failed)
        } else if running.record.stop_requested() {
            Some(ProcessStatus::Stopped)
        } else if running.record.overflowed() {
            Some(ProcessStatus::OutputLimitExceeded)
        } else if Instant::now() >= deadline {
            Some(ProcessStatus::TimedOut)
        } else if capture.live_lease(&start.action).is_err()
            || reads.iter().any(|read| capture.live_lease(read).is_err())
        {
            Some(ProcessStatus::Stopped)
        } else {
            None
        };
        if let Some(reason) = reason {
            if running.process.terminate_and_confirm_empty(2000).is_ok() {
                break (reason, None);
            }
            break (ProcessStatus::TerminationUncertain, None);
        }
        match running.process.try_wait() {
            Ok(Some(code)) => break (ProcessStatus::Exited, Some(code as i32)),
            Ok(None) => (),
            Err(_) => {
                if running.process.terminate_and_confirm_empty(2000).is_ok() {
                    break (ProcessStatus::Failed, None);
                }
                break (ProcessStatus::TerminationUncertain, None);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if status == ProcessStatus::TerminationUncertain {
        running.record.truncate_output();
        running.record.finish(status, None);
        std::mem::forget(stage);
        return Err(Error::WorkerUnavailable);
    }
    // Bound final draining even if an unexpected foreign handle retained a pipe.
    let mut eof = false;
    for _ in 0..32 {
        match (
            drain_once(&mut running.stdout, &running.record, ProcessStream::Stdout),
            drain_once(&mut running.stderr, &running.record, ProcessStream::Stderr),
        ) {
            (Ok(true), Ok(true)) => {
                eof = true;
                break;
            }
            (Err(_), _) | (_, Err(_)) => {
                status = ProcessStatus::Failed;
                break;
            }
            _ => (),
        }
    }
    if !eof {
        running.record.truncate_output();
    }
    if status == ProcessStatus::Exited && running.record.overflowed() {
        status = ProcessStatus::OutputLimitExceeded;
    }
    let Running {
        mut record,
        process,
        stdout,
        stderr,
    } = running;
    drop(process);
    drop(stdout);
    drop(stderr);
    drop(root_pin);
    let cleanup = cleanup_stage(std::mem::ManuallyDrop::into_inner(stage), io, runtime);
    if cleanup.is_err() {
        record.append(
            ProcessStream::Stderr,
            b"optic.private_stage_cleanup_uncertain\n",
        );
        status = ProcessStatus::Failed;
        exit_code = None;
    }
    record.finish(status, exit_code);
    cleanup
}

fn cleanup_stage(
    stage: PreparedPrivateProject,
    io: &BlockingIoGovernor,
    runtime: &tokio::runtime::Handle,
) -> Result<(), Error> {
    match runtime.block_on(io.acquire_owned()) {
        Ok(_permit) => stage.discard(),
        Err(error) => {
            std::mem::forget(stage);
            Err(error.into())
        }
    }
}

// Node's permission matcher and CJS loader must receive the same DOS spelling.
// Native identity checks continue to use the pinned, canonical host path. Only
// a local drive prefix is translated; UNC/device namespaces fail closed.
fn node_local_path(path: &Path) -> Result<String, Error> {
    let text = path.to_str().ok_or(Error::InvalidPrivateTask)?;
    let local = text.strip_prefix(r"\\?\").unwrap_or(text);
    let bytes = local.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || &bytes[1..3] != b":\\" {
        return Err(Error::InvalidPrivateTask);
    }
    Ok(local.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_paths_reject_remote_and_device_namespaces() {
        assert_eq!(
            node_local_path(Path::new(r"\\?\C:\private\src\main.cjs")).unwrap(),
            r"C:\private\src\main.cjs"
        );
        for denied in [
            r"\\server\share\main.cjs",
            r"\\?\UNC\server\share\main.cjs",
            r"\\.\pipe\x",
            r"C:relative.cjs",
            "relative.cjs",
        ] {
            assert!(node_local_path(Path::new(denied)).is_err());
        }
    }
}
