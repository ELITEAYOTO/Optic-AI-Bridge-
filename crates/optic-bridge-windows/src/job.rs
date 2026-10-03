use std::{
    ffi::c_void,
    future::Future,
    io::{Error, ErrorKind, Result},
    mem::size_of,
    os::windows::io::{AsRawHandle, BorrowedHandle},
    pin::Pin,
    process::ExitStatus,
    time::Duration,
};

use process_wrap::tokio::{ChildWrapper, CommandWrap, CommandWrapper};
use tokio::process::Command;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_NO_MORE_FILES, HANDLE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
                JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
                QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
            },
            Threading::{
                CREATE_SUSPENDED, GetProcessId, OpenThread, PROCESS_CREATION_FLAGS, ResumeThread,
                THREAD_SUSPEND_RESUME,
            },
        },
    },
    core::HRESULT,
};

const JOB_EMPTY_POLL: Duration = Duration::from_millis(10);

/// Windows Job Object containment configured from an already-authorized process budget.
///
/// The wrapper forces temporary suspended creation, creates/configures the Job Object,
/// assigns the child, and only then resumes the child threads. This closes the ordinary
/// spawn-before-assignment window while keeping the public process API structured.
#[derive(Clone, Copy, Debug)]
pub struct LimitedJobObject {
    active_process_limit: u32,
    job_memory_limit: usize,
}

impl LimitedJobObject {
    pub fn new(active_process_limit: u32, job_memory_limit: u64) -> Result<Self> {
        if active_process_limit == 0 || job_memory_limit == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Windows Job Object limits must be non-zero",
            ));
        }
        let job_memory_limit = usize::try_from(job_memory_limit).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                "Windows Job Object memory limit does not fit the platform address size",
            )
        })?;
        Ok(Self {
            active_process_limit,
            job_memory_limit,
        })
    }
}

impl CommandWrapper for LimitedJobObject {
    fn pre_spawn(&mut self, command: &mut Command, _core: &CommandWrap) -> Result<()> {
        command.creation_flags(PROCESS_CREATION_FLAGS(CREATE_SUSPENDED.0).0);
        Ok(())
    }

    fn wrap_child(
        &mut self,
        mut inner: Box<dyn ChildWrapper>,
        _core: &CommandWrap,
    ) -> Result<Box<dyn ChildWrapper>> {
        let process = child_process_handle(inner.as_ref()).ok_or_else(|| {
            Error::new(
                ErrorKind::Unsupported,
                "child wrapper does not expose a Windows process handle",
            )
        })?;
        let process = HANDLE(process.as_raw_handle());

        let job = match OwnedJob::create(self.active_process_limit, self.job_memory_limit) {
            Ok(job) => job,
            Err(error) => {
                let _ = inner.start_kill();
                return Err(error);
            }
        };

        // SAFETY: `job.raw()` is an owned Job Object handle and `process` is the live
        // child process handle borrowed from `inner`. Both remain valid for this call.
        if let Err(error) = unsafe { AssignProcessToJobObject(job.raw(), process) } {
            let _ = inner.start_kill();
            return Err(Error::other(error));
        }

        if let Err(error) = resume_process_threads(process) {
            let _ = job.terminate(1);
            let _ = inner.start_kill();
            return Err(error);
        }

        Ok(Box::new(LimitedJobChild {
            inner,
            job,
            root_status: None,
        }))
    }
}

#[derive(Debug)]
struct OwnedJob(HANDLE);

// SAFETY: Win32 kernel object handles are process-wide values that may be used from
// different threads. Ownership remains unique in `OwnedJob`, and Drop closes it once.
unsafe impl Send for OwnedJob {}
// SAFETY: operations used through `&OwnedJob` do not mutate Rust memory and the kernel
// synchronizes access to the underlying Job Object.
unsafe impl Sync for OwnedJob {}

impl OwnedJob {
    fn create(active_process_limit: u32, job_memory_limit: usize) -> Result<Self> {
        // SAFETY: null security attributes/name request a new unnamed Job Object.
        let handle = unsafe { CreateJobObjectW(None, None) }.map_err(Error::other)?;
        let job = Self(handle);

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_JOB_MEMORY;
        limits.BasicLimitInformation.ActiveProcessLimit = active_process_limit;
        limits.JobMemoryLimit = job_memory_limit;

        // SAFETY: the information class matches the concrete structure and its exact size;
        // `job` owns a valid Job Object handle for the duration of the call.
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(Error::other)?;

        Ok(job)
    }

    const fn raw(&self) -> HANDLE {
        self.0
    }

    fn terminate(&self, exit_code: u32) -> Result<()> {
        // SAFETY: `self.0` is the valid Job Object handle uniquely owned by this value.
        unsafe { TerminateJobObject(self.0, exit_code) }.map_err(Error::other)
    }

    fn active_processes(&self) -> Result<u32> {
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: the requested information class matches `accounting` and the buffer
        // remains valid for the entire synchronous call.
        unsafe {
            QueryInformationJobObject(
                Some(self.0),
                JobObjectBasicAccountingInformation,
                (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                None,
            )
        }
        .map_err(Error::other)?;
        Ok(accounting.ActiveProcesses)
    }
}

impl Drop for OwnedJob {
    fn drop(&mut self) {
        // SAFETY: this value owns the handle and Drop runs once. KILL_ON_JOB_CLOSE makes
        // closing the final Job Object handle the containment backstop for descendants.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[derive(Debug)]
struct LimitedJobChild {
    inner: Box<dyn ChildWrapper>,
    job: OwnedJob,
    root_status: Option<ExitStatus>,
}

impl ChildWrapper for LimitedJobChild {
    fn inner(&self) -> &dyn ChildWrapper {
        self.inner.as_ref()
    }

    fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
        self.inner.as_mut()
    }

    fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
        let LimitedJobChild {
            mut inner, job, ..
        } = *self;
        // Removing the containment wrapper must not silently detach a live process tree.
        // Fail closed: terminate the whole job, ask the root child to terminate as a
        // fallback, then drop the owned Job Object handle instead of leaking it.
        let _ = job.terminate(1);
        let _ = inner.start_kill();
        drop(job);
        inner
    }

    fn process_handle(&self) -> Option<BorrowedHandle<'_>> {
        child_process_handle(self.inner.as_ref())
    }

    fn start_kill(&mut self) -> Result<()> {
        self.job.terminate(1)
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        if self.root_status.is_none() {
            self.root_status = self.inner.try_wait()?;
        }
        match self.root_status {
            Some(status) if self.job.active_processes()? == 0 => Ok(Some(status)),
            _ => Ok(None),
        }
    }

    fn wait(&mut self) -> Pin<Box<dyn Future<Output = Result<ExitStatus>> + Send + '_>> {
        Box::pin(async move {
            let status = match self.root_status {
                Some(status) => status,
                None => {
                    let status = self.inner.wait().await?;
                    self.root_status = Some(status);
                    status
                }
            };

            while self.job.active_processes()? != 0 {
                tokio::time::sleep(JOB_EMPTY_POLL).await;
            }
            Ok(status)
        })
    }
}

fn child_process_handle(child: &dyn ChildWrapper) -> Option<BorrowedHandle<'_>> {
    child.process_handle().or_else(|| {
        child
            .try_inner_child()
            .and_then(|native| native.process_handle())
    })
}

fn resume_process_threads(process: HANDLE) -> Result<()> {
    // SAFETY: `process` is the live child process handle supplied by the process wrapper.
    let pid = unsafe { GetProcessId(process) };
    if pid == 0 {
        return Err(Error::last_os_error());
    }

    // SAFETY: documented snapshot call with no caller-owned pointer arguments.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }?;
    let snapshot = ScopedHandle(snapshot);
    let mut entry = THREADENTRY32 {
        dwSize: size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };

    // SAFETY: `snapshot` is valid and `entry` has the required size initialized.
    unsafe { Thread32First(snapshot.0, &mut entry) }.map_err(Error::other)?;
    let mut resumed = false;

    loop {
        if entry.th32OwnerProcessID == pid {
            // SAFETY: opening a thread by enumerated thread id with the minimal resume right.
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID) }
                .map_err(Error::other)?;
            let thread = ScopedHandle(thread);
            // SAFETY: `thread.0` is a valid thread handle with THREAD_SUSPEND_RESUME access.
            let previous = unsafe { ResumeThread(thread.0) };
            if previous == u32::MAX {
                return Err(Error::last_os_error());
            }
            resumed |= previous > 0;
        }

        // SAFETY: same valid snapshot and output buffer as above.
        match unsafe { Thread32Next(snapshot.0, &mut entry) } {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(ERROR_NO_MORE_FILES.0) => break,
            Err(error) => return Err(Error::other(error)),
        }
    }

    if resumed {
        Ok(())
    } else {
        Err(Error::other(
            "no suspended child thread was found to resume",
        ))
    }
}

#[derive(Debug)]
struct ScopedHandle(HANDLE);

impl Drop for ScopedHandle {
    fn drop(&mut self) {
        // SAFETY: each ScopedHandle owns a temporary Win32 handle exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_limits() {
        assert!(LimitedJobObject::new(0, 1).is_err());
        assert!(LimitedJobObject::new(1, 0).is_err());
    }

    #[test]
    fn accepts_nonzero_limits() {
        assert!(LimitedJobObject::new(1, 64 * 1024 * 1024).is_ok());
    }

    #[test]
    fn drop_fixture_child() {
        let Some(started) = std::env::var_os("OPTIC_JOB_DROP_STARTED") else {
            return;
        };
        let survived = std::env::var_os("OPTIC_JOB_DROP_SURVIVED")
            .expect("survived marker must accompany started marker");
        std::fs::write(started, b"started").expect("write started marker");
        std::thread::sleep(Duration::from_millis(700));
        std::fs::write(survived, b"survived").expect("write survived marker");
    }

    async fn spawn_delayed_marker_fixture(
        label: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf, Box<dyn ChildWrapper>) {
        use process_wrap::tokio::CommandWrap;
        use std::time::{SystemTime, UNIX_EPOCH};

        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("optic-job-{label}-{token}"));
        std::fs::create_dir_all(&root).expect("create fixture dir");
        let started = root.join("started");
        let survived = root.join("survived");

        let mut command = Command::new(std::env::current_exe().expect("current test executable"));
        command
            .args(["--exact", "job::tests::drop_fixture_child", "--nocapture"])
            .env("OPTIC_JOB_DROP_STARTED", &started)
            .env("OPTIC_JOB_DROP_SURVIVED", &survived);
        let mut command = CommandWrap::from(command);
        command.wrap(LimitedJobObject::new(1, 512 * 1024 * 1024).expect("valid Job Object limits"));
        let child = command.spawn().expect("spawn wrapped fixture");

        for _ in 0..200 {
            if started.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(started.exists(), "fixture child did not start");
        (root, survived, child)
    }

    #[tokio::test]
    async fn dropping_job_handle_kills_running_child() {
        let (root, survived, child) = spawn_delayed_marker_fixture("drop").await;
        drop(child);
        tokio::time::sleep(Duration::from_millis(900)).await;
        assert!(
            !survived.exists(),
            "closing the kill-on-close Job Object did not terminate the child"
        );
        std::fs::remove_dir_all(root).expect("remove fixture dir");
    }

    #[tokio::test]
    async fn unwrapping_job_containment_fails_closed() {
        let (root, survived, child) = spawn_delayed_marker_fixture("unwrap").await;
        let inner = child.into_inner();
        drop(inner);
        tokio::time::sleep(Duration::from_millis(900)).await;
        assert!(
            !survived.exists(),
            "unwrapping containment allowed the child to survive"
        );
        std::fs::remove_dir_all(root).expect("remove fixture dir");
    }
}
