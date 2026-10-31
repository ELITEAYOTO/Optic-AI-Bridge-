//! Issue #317: a direct child can exit before inherited stdio reaches EOF.
//! These supervised synthetic tests prove that an operation cannot report
//! success *after* its advertised deadline. They deliberately do not claim
//! that the current blocking joins complete within that deadline.
use optic_bridge_runtime::{HardenedCommandError, HardenedCommandRunner, HardenedCommandSpec};
use std::{
    collections::BTreeMap,
    env,
    io::{self, Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const RUNNER_TIMEOUT: Duration = Duration::from_millis(200);
const DESCENDANT_DELAY: Duration = Duration::from_millis(1600);
const SUPERVISOR_TIMEOUT: Duration = Duration::from_secs(15);
const INPUT_BYTES: usize = 2 * 1024 * 1024;
const FIXTURE_ENV: &str = "OPTIC_317_LATE_PIPE_SYNTHETIC";

fn synthetic_command(test_name: &str) -> Command {
    let mut command = Command::new(env::current_exe().expect("known test binary"));
    command.args(["--exact", test_name, "--nocapture"]);
    command.env(FIXTURE_ENV, "1");
    command
}

fn spec(test_name: &str) -> HardenedCommandSpec {
    HardenedCommandSpec {
        executable: env::current_exe().expect("test binary"),
        cwd: env::current_dir().expect("test cwd"),
        args: ["--exact", test_name, "--nocapture"]
            .into_iter()
            .map(Into::into)
            .collect(),
        env: BTreeMap::from([(FIXTURE_ENV.into(), "1".into())]),
        output_limit: 64 * 1024,
    }
}

#[test]
fn fixture_stdout_descendant() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    println!("retained-stdout");
    eprintln!("retained-stderr");
    io::stdout().flush().expect("flush stdout");
    io::stderr().flush().expect("flush stderr");
    thread::sleep(DESCENDANT_DELAY);
}

#[test]
fn fixture_quiet_stdout_holder() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    // Keep the inherited stdout handle open until the known finite expiry.
    // Do not write after the runner deliberately closes an over-limit reader.
    thread::sleep(DESCENDANT_DELAY);
}

#[test]
fn fixture_stdout_root_exits() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    let mut descendant = synthetic_command("fixture_stdout_descendant");
    descendant.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let _synthetic_descendant = descendant.spawn().expect("spawn synthetic descendant");
    // Exit the disposable root, leaving only its bounded synthetic descendant.
    std::process::exit(0);
}

#[test]
fn fixture_output_limit_root_exits() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    let mut descendant = synthetic_command("fixture_quiet_stdout_holder");
    // Retain inherited stdout beyond the root, but not stderr: a peer reader
    // already blocked on an unrelated inherited pipe cannot be interrupted
    // by the shared overflow flag and would make this timing test racy.
    descendant.stdout(Stdio::inherit()).stderr(Stdio::null());
    let _synthetic_descendant = descendant.spawn().expect("spawn synthetic pipe holder");
    // Emit past the byte ceiling *after* opening the descendant's inherited
    // handles. An over-limit reader must now stop without waiting for EOF.
    println!("{}", "x".repeat(512));
    io::stdout().flush().expect("flush over-budget output");
    std::process::exit(0);
}

#[test]
fn fixture_stdin_descendant() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    thread::sleep(DESCENDANT_DELAY);
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .expect("read inherited input to EOF");
    assert_eq!(bytes.len(), INPUT_BYTES);
    assert!(bytes.iter().all(|byte| *byte == b'x'));
}

#[test]
fn fixture_stdin_root_exits() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    let mut descendant = synthetic_command("fixture_stdin_descendant");
    descendant
        .stdin(Stdio::inherit())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _synthetic_descendant = descendant.spawn().expect("spawn synthetic stdin reader");
    std::process::exit(0);
}

#[test]
fn fixture_assert_late_stdout_is_not_success() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    let runner = HardenedCommandRunner::new(RUNNER_TIMEOUT).expect("runner");
    let started = Instant::now();
    let result = runner.run(&spec("fixture_stdout_root_exits"));
    let elapsed = started.elapsed();
    assert!(
        elapsed > Duration::from_millis(800) && elapsed < Duration::from_secs(10),
        "fixture must exhibit bounded late EOF; observed {elapsed:?}"
    );
    assert!(
        matches!(result, Err(HardenedCommandError::TimedOut)),
        "a successful root is not proof of deadline compliance; got {result:?}"
    );
    println!(
        "ISSUE317_LATE_STDOUT_REJECTED elapsed_ms={}",
        elapsed.as_millis()
    );
}

#[test]
fn fixture_assert_late_stdin_is_not_success() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    let runner = HardenedCommandRunner::new(RUNNER_TIMEOUT).expect("runner");
    let input = vec![b'x'; INPUT_BYTES];
    let started = Instant::now();
    let result = runner.run_with_input(
        &spec("fixture_stdin_root_exits"),
        &input,
        INPUT_BYTES as u64,
    );
    let elapsed = started.elapsed();
    assert!(
        elapsed > Duration::from_millis(800) && elapsed < Duration::from_secs(10),
        "fixture must exhibit bounded late writer completion; observed {elapsed:?}"
    );
    assert!(
        matches!(result, Err(HardenedCommandError::TimedOut)),
        "a successful root is not proof of deadline compliance; got {result:?}"
    );
    println!(
        "ISSUE317_LATE_STDIN_REJECTED elapsed_ms={}",
        elapsed.as_millis()
    );
}

#[test]
fn fixture_assert_overflow_does_not_wait_for_descendant_eof() {
    if env::var_os(FIXTURE_ENV).is_none() {
        return;
    }
    let runner = HardenedCommandRunner::new(RUNNER_TIMEOUT).expect("runner");
    let mut command_spec = spec("fixture_output_limit_root_exits");
    command_spec.output_limit = 128;
    let started = Instant::now();
    let result = runner.run(&command_spec);
    let elapsed = started.elapsed();
    // The quiet synthetic descendant holds inherited stdout for 1.6s;
    // stderr is isolated so a separate blocked read cannot skew this check.
    // Crossing the output ceiling must now reject before that late EOF,
    // rather than continuing to drain an already unacceptable stream.
    assert!(
        elapsed < Duration::from_millis(800),
        "over-limit output must be rejected without waiting for descendant EOF; elapsed={elapsed:?}"
    );
    assert!(
        matches!(
            result,
            Err(HardenedCommandError::OutputLimitExceeded { limit: 128 })
        ),
        "output overflow must retain its specific error even with inherited pipes; got {result:?}"
    );
    println!(
        "ISSUE317_OVERFLOW_EARLY_REJECTED elapsed_ms={}",
        elapsed.as_millis()
    );
}

fn supervise(inner: &str, evidence: &str) {
    let mut command = synthetic_command(inner);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut host = command.spawn().expect("start isolated fixture host");
    let started = Instant::now();
    loop {
        if let Some(status) = host.try_wait().expect("poll exact fixture host") {
            let output = host
                .wait_with_output()
                .expect("capture bounded host output");
            assert!(
                status.success(),
                "fixture failed: stdout={} stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains(evidence),
                "missing evidence marker: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            return;
        }
        if started.elapsed() >= SUPERVISOR_TIMEOUT {
            // Terminate only the exact disposable host this test launched.
            // The descendant's finite sleep is not native containment proof.
            let _ = host.kill();
            let _ = host.wait();
            panic!("isolated synthetic fixture exceeded 15s supervisor ceiling");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn late_stdout_completion_must_not_be_success() {
    supervise(
        "fixture_assert_late_stdout_is_not_success",
        "ISSUE317_LATE_STDOUT_REJECTED",
    );
}

#[test]
fn output_limit_stops_early_and_remains_specific() {
    supervise(
        "fixture_assert_overflow_does_not_wait_for_descendant_eof",
        "ISSUE317_OVERFLOW_EARLY_REJECTED",
    );
}

#[test]
fn late_stdin_completion_must_not_be_success() {
    supervise(
        "fixture_assert_late_stdin_is_not_success",
        "ISSUE317_LATE_STDIN_REJECTED",
    );
}
