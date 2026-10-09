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
        self.run_inner(spec, None)
    }

    pub fn run_with_input(
        &self,
        spec: &HardenedCommandSpec,
        input: &[u8],
        input_limit: u64,
    ) -> Result<HardenedCommandOutput, HardenedCommandError> {
        let input_len = u64::try_from(input.len()).unwrap_or(u64::MAX);
        if input_limit == 0 || input_len > input_limit {
            return Err(HardenedCommandError::InputLimitExceeded { limit: input_limit });
        }
        self.run_inner(spec, Some(input.to_vec()))
    }

    fn run_inner(
        &self,
        spec: &HardenedCommandSpec,
        input: Option<Vec<u8>>,
    ) -> Result<HardenedCommandOutput, HardenedCommandError> {
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

        let started = Instant::now();
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

        if timed_out {
            return Err(HardenedCommandError::TimedOut);
        }
        if exceeded.load(Ordering::Acquire) {
            return Err(HardenedCommandError::OutputLimitExceeded {
                limit: spec.output_limit,
            });
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
