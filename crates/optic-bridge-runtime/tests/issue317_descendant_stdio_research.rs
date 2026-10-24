//! Research-only, time-bounded reproduction of issue 317. This is an observation
//! of current behavior, NOT acceptance of the production runner.
use std::{
    collections::BTreeMap,
    env,
    io::{self, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use optic_bridge_runtime::{HardenedCommandRunner, HardenedCommandSpec};

const RUNNER_DEADLINE: Duration = Duration::from_millis(200);
const DESCENDANT_HOLD: Duration = Duration::from_millis(1600);
const SUPERVISOR_CEILING: Duration = Duration::from_secs(15);

fn fixture_command(mode: &str) -> Command {
    let mut command = Command::new(env::current_exe().expect("current fixture binary"));
    command.args(["--exact", mode, "--nocapture"]);
    command.env("OPTIC_317_SYNTHETIC_ONLY", "1");
    command
}

#[test]
fn fixture_descendant_holds_stdio() {
    if env::var_os("OPTIC_317_SYNTHETIC_ONLY").is_none() {
        return;
    }
    println!("descendant-retains-stdout");
    eprintln!("descendant-retains-stderr");
    io::stdout().flush().expect("stdout flush");
    io::stderr().flush().expect("stderr flush");
    thread::sleep(DESCENDANT_HOLD);
}

#[test]
fn fixture_parent_exits_before_descendant() {
    if env::var_os("OPTIC_317_SYNTHETIC_ONLY").is_none() {
        return;
    }
    let mut child = fixture_command("fixture_descendant_holds_stdio");
    // The descendant inherits *both* handles, as it would from a program
    // that forwards its stdout/stderr to its children.
    child.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let _child = child.spawn().expect("spawn synthetic descendant");
    // Deliberately exit without waiting: only the test fixture may do this.
    // The descendant is guaranteed to exit after DESCENDANT_HOLD.
}

#[test]
fn fixture_runner_observes_pipe_join() {
    if env::var_os("OPTIC_317_SYNTHETIC_ONLY").is_none() {
        return;
    }
    let spec = HardenedCommandSpec {
        executable: env::current_exe().expect("test binary"),
        cwd: env::current_dir().expect("test cwd"),
        args: ["--exact", "fixture_parent_exits_before_descendant", "--nocapture"]
            .into_iter().map(Into::into).collect(),
        env: BTreeMap::from([("OPTIC_317_SYNTHETIC_ONLY".into(), "1".into())]),
        output_limit: 64 * 1024,
    };
    let runner = HardenedCommandRunner::new(RUNNER_DEADLINE).expect("runner");
    let started = Instant::now();
    let result = runner.run(&spec).expect("root exits successfully; existing join waits");
    let elapsed = started.elapsed();
    assert!(result.status.success(), "root outcome");
    assert!(elapsed > Duration::from_millis(800),
        "expected old join to outlive its advertised 200ms deadline; elapsed={elapsed:?}");
    assert!(elapsed < Duration::from_secs(10), "synthetic descendant did not exit promptly");
    assert!(String::from_utf8_lossy(&result.stdout).contains("descendant-retains-stdout"));
    assert!(String::from_utf8_lossy(&result.stderr).contains("descendant-retains-stderr"));
    println!("OBSERVED_ISSUE317 deadline_ms=200 elapsed_ms={} outcome=successful_root_and_late_eof", elapsed.as_millis());
}

#[test]
fn externally_supervised_descendant_stdio_deadline_reproduction() {
    let mut supervisor = fixture_command("fixture_runner_observes_pipe_join");
    supervisor.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = supervisor.spawn().expect("start isolated fixture runner");
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("poll fixture runner") {
            let output = child.wait_with_output().expect("collect supervisor evidence");
            assert!(status.success(),
                "fixture failed: stdout={} stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr));
            assert!(String::from_utf8_lossy(&output.stdout).contains("OBSERVED_ISSUE317"),
                "missing measured observation: {}", String::from_utf8_lossy(&output.stdout));
            return;
        }
        if start.elapsed() >= SUPERVISOR_CEILING {
            // Kill only the specifically created test host, never an arbitrary PID.
            // Synthetic descendants have their own finite sleep; no native Job
            // containment, process-tree guarantee, or successful cleanup is claimed.
            let _ = child.kill();
            let _ = child.wait();
            panic!("isolated fixture host exceeded 15s ceiling; outcome not accepted");
        }
        thread::sleep(Duration::from_millis(20));
    }
}
