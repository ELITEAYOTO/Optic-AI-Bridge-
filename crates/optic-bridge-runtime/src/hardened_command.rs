use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use thiserror::Error;

const PIPE_READ_CHUNK_BYTES: usize = 8 * 1024;
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Clone, Debug)]
pub struct HardenedCommandSpec {
    pub executable: PathBuf,
    pub cwd: PathBuf,
    pub args: Vec<OsString>,
    pub env: BTreeMap<OsString, OsString>,
    pub output_limit: u64,
}

impl HardenedCommandSpec {
    fn validate(&self) -> Result<(), HardenedCommandError> {
        if !self.executable.is_absolute() {
            return Err(HardenedCommandError::ExecutableMustBeAbsolute);
        }
        if !self.cwd.is_absolute() {
            return Err(HardenedCommandError::WorkingDirectoryMustBeAbsolute);
        }
        if self.output_limit == 0 {
            return Err(HardenedCommandError::ZeroOutputLimit);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HardenedCommandRunner {
    timeout: Duration,
}

impl HardenedCommandRunner {
    pub fn new(timeout: Duration) -> Result<Self, HardenedCommandError> {
        if timeout.is_zero() {
            return Err(HardenedCommandError::ZeroTimeout);
        }
        Ok(Self { timeout })
    }

    pub fn run(
        &self,
        spec: &HardenedCommandSpec,
    ) -> Result<HardenedCommandOutput, HardenedCommandError> {
        // Include spec preparation, process creation and pipe setup in the
        // same non-renewable deadline as child execution and pipe drainage.
        self.run_inner(spec, None, Instant::now())
    }

    pub fn run_with_input(
        &self,
        spec: &HardenedCommandSpec,
        input: &[u8],
        input_limit: u64,
    ) -> Result<HardenedCommandOutput, HardenedCommandError> {
        let started = Instant::now();
        let input_len = u64::try_from(input.len()).unwrap_or(u64::MAX);
        if input_limit == 0 || input_len > input_limit {
            return Err(HardenedCommandError::InputLimitExceeded { limit: input_limit });
        }
        self.run_inner(spec, Some(input.to_vec()), started)
    }

    fn run_inner(
        &self,
        spec: &HardenedCommandSpec,
        input: Option<Vec<u8>>,
        started: Instant,
    ) -> Result<HardenedCommandOutput, HardenedCommandError> {
        // This start point is owned by the caller before any potentially
        // blocking Command::spawn or setup. It is never refreshed after spawn.
        // It cannot interrupt an OS call or join that never returns (#317).
        spec.validate()?;

        let mut command = Command::new(&spec.executable);
        command
            .current_dir(&spec.cwd)
            .args(&spec.args)
            .env_clear()
            .envs(&spec.env);
        if input.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Fail closed if spec/input preparation has already exhausted the
        // original nonrenewable call deadline. This avoids entering OS
        // process creation after known expiry; it is not an atomic
        // create/revoke fence and cannot interrupt spawn once entered.
        if started.elapsed() >= self.timeout {
            return Err(HardenedCommandError::TimedOut);
        }
        let mut child = command.spawn()?;
        let input_writer = if let Some(input) = input {
            let mut stdin = child
                .stdin
                .take()
                .ok_or(HardenedCommandError::MissingChildPipe)?;
            Some(thread::spawn(move || -> Result<(), io::Error> {
                stdin.write_all(&input)?;
                stdin.flush()?;
                Ok(())
            }))
        } else {
            None
        };
        let stdout = child
            .stdout
            .take()
            .ok_or(HardenedCommandError::MissingChildPipe)?;
        let stderr = child
            .stderr
            .take()
            .ok_or(HardenedCommandError::MissingChildPipe)?;
        let total = Arc::new(AtomicU64::new(0));
        let exceeded = Arc::new(AtomicBool::new(false));
        let stdout_reader = spawn_bounded_reader(
            stdout,
            Arc::clone(&total),
            Arc::clone(&exceeded),
            spec.output_limit,
        );
        let stderr_reader =
            spawn_bounded_reader(stderr, total, Arc::clone(&exceeded), spec.output_limit);

        let mut timed_out = false;
        let status = loop {
            if exceeded.load(Ordering::Acquire) {
                let _ = child.kill();
                break child.wait()?;
            }
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() >= self.timeout {
                timed_out = true;
                let _ = child.kill();
                break child.wait()?;
            }
            thread::sleep(CHILD_POLL_INTERVAL);
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| HardenedCommandError::ReaderThreadPanicked)??;
        let stderr = stderr_reader
            .join()
            .map_err(|_| HardenedCommandError::ReaderThreadPanicked)??;
        let input_result = input_writer
            .map(|writer| {
                writer
                    .join()
                    .map_err(|_| HardenedCommandError::WriterThreadPanicked)
            })
            .transpose()?;

        // Preserve the reason that actually stopped the direct child: a
        // previously detected output limit is not reclassified as a timeout
        // merely because descendants kept inherited pipes open afterwards.
        if timed_out {
            return Err(HardenedCommandError::TimedOut);
        }
        if exceeded.load(Ordering::Acquire) {
            return Err(HardenedCommandError::OutputLimitExceeded {
                limit: spec.output_limit,
            });
        }
        // Even a direct child that succeeds after a slow spawn/pipe setup is
        // beyond the original call deadline. A descendant may then keep pipes
        // open after that child exits. Neither case can be reported as success.
        // This corrects the eventual result only: bounding spawn, waits and
        // joins still requires owned process-tree/pipe lifetimes (issue #317).
        if started.elapsed() >= self.timeout {
            return Err(HardenedCommandError::TimedOut);
        }
        if let Some(result) = input_result {
            result?;
        }

        Ok(HardenedCommandOutput {
            status,
            stdout,
            stderr,
        })
    }
}

#[derive(Debug)]
pub struct HardenedCommandOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum HardenedCommandError {
    #[error("command executable path must be absolute")]
    ExecutableMustBeAbsolute,
    #[error("command working directory must be absolute")]
    WorkingDirectoryMustBeAbsolute,
    #[error("command output limit must be non-zero")]
    ZeroOutputLimit,
    #[error("command timeout must be non-zero")]
    ZeroTimeout,
    #[error("command exceeded its hard deadline")]
    TimedOut,
    #[error("command output exceeded byte ceiling {limit}")]
    OutputLimitExceeded { limit: u64 },
    #[error("command input exceeded byte ceiling {limit}")]
    InputLimitExceeded { limit: u64 },
    #[error("command child pipe was unavailable")]
    MissingChildPipe,
    #[error("command output reader thread panicked")]
    ReaderThreadPanicked,
    #[error("command input writer thread panicked")]
    WriterThreadPanicked,
    #[error("command process operation failed: {0}")]
    Io(#[from] io::Error),
}

fn spawn_bounded_reader<R: Read + Send + 'static>(
    mut reader: R,
    total: Arc<AtomicU64>,
    exceeded: Arc<AtomicBool>,
    limit: u64,
) -> thread::JoinHandle<Result<Vec<u8>, io::Error>> {
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0_u8; PIPE_READ_CHUNK_BYTES];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let read_u64 = u64::try_from(read).unwrap_or(u64::MAX);
            let previous = total.fetch_add(read_u64, Ordering::AcqRel);
            let remaining = limit.saturating_sub(previous);
            let accepted = remaining.min(read_u64) as usize;
            captured.extend_from_slice(&buffer[..accepted]);
            if read_u64 > remaining {
                exceeded.store(true, Ordering::Release);
            }
        }
        Ok(captured)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_consumed_before_spawn_cannot_be_refreshed_after_child_exit() {
        // Model time already spent before Command::spawn using an old
        // original start. This now proves early expiry rejection; it does
        // not prove post-spawn joins or Windows calls are interruptible.
        let runner = HardenedCommandRunner::new(Duration::from_millis(200)).unwrap();
        let spec = HardenedCommandSpec {
            executable: std::env::current_exe().expect("harness executable"),
            cwd: std::env::current_dir().expect("working directory"),
            args: vec![
                "--exact".into(),
                "hardened_command::tests::invalid_specs_fail_before_spawn".into(),
            ],
            env: BTreeMap::new(),
            output_limit: 16 * 1024,
        };
        let started = Instant::now()
            .checked_sub(Duration::from_secs(5))
            .expect("subtract bounded elapsed fixture time");
        assert!(
            matches!(
                runner.run_inner(&spec, None, started),
                Err(HardenedCommandError::TimedOut)
            ),
            "the original call deadline must not restart after OS spawn"
        );
    }

    #[test]
    fn expired_original_deadline_is_rejected_before_os_spawn_attempt() {
        // This real runner seam receives the nonrenewable start from the
        // public API. A working directory is an absolute but non-executable
        // target on both platforms: attempting spawn would return an I/O
        // error instead of the expected deadline error.
        let runner = HardenedCommandRunner::new(Duration::from_millis(200)).unwrap();
        let cwd = std::env::current_dir().expect("working directory");
        let spec = HardenedCommandSpec {
            executable: cwd.clone(),
            cwd,
            args: Vec::new(),
            env: BTreeMap::new(),
            output_limit: 1,
        };
        let started = Instant::now()
            .checked_sub(Duration::from_secs(2))
            .expect("subtract bounded elapsed fixture time");
        assert!(
            matches!(
                runner.run_inner(&spec, None, started),
                Err(HardenedCommandError::TimedOut)
            ),
            "an already expired caller must not attempt OS process creation"
        );
    }

    #[test]
    fn invalid_specs_fail_before_spawn() {
        let runner = HardenedCommandRunner::new(Duration::from_millis(1)).expect("runner");
        let relative_executable = HardenedCommandSpec {
            executable: PathBuf::from("tool"),
            cwd: std::env::current_dir().expect("cwd"),
            args: Vec::new(),
            env: BTreeMap::new(),
            output_limit: 1,
        };
        assert!(matches!(
            runner.run(&relative_executable),
            Err(HardenedCommandError::ExecutableMustBeAbsolute)
        ));

        assert!(matches!(
            HardenedCommandRunner::new(Duration::ZERO),
            Err(HardenedCommandError::ZeroTimeout)
        ));

        assert!(matches!(
            runner.run_with_input(&relative_executable, b"ab", 1),
            Err(HardenedCommandError::InputLimitExceeded { limit: 1 })
        ));
        assert!(matches!(
            runner.run_with_input(&relative_executable, b"", 0),
            Err(HardenedCommandError::InputLimitExceeded { limit: 0 })
        ));
    }
}
