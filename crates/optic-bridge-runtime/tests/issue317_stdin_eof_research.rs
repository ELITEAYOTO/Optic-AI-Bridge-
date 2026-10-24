//! Diagnostic only: a descendant retains stdin after its direct parent exits.
//! This accepts the current deadline violation as evidence, not as correctness.
use optic_bridge_runtime::{HardenedCommandRunner, HardenedCommandSpec};
use std::{
    collections::BTreeMap,
    env,
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const INPUT_BYTES: usize = 2 * 1024 * 1024;
const RUNNER_DEADLINE: Duration = Duration::from_millis(200);
const DESCENDANT_DELAY: Duration = Duration::from_millis(1600);
const OUTER_DEADLINE: Duration = Duration::from_secs(15);

fn synthetic_test(name: &str) -> Command {
    let mut child = Command::new(env::current_exe().expect("fixed test executable"));
    child.args(["--exact", name, "--nocapture"]);
    child.env("OPTIC_317_STDIN_SYNTHETIC", "1");
    child
}

#[test]
fn fixture_descendant_delays_stdin_eof() {
    if env::var_os("OPTIC_317_STDIN_SYNTHETIC").is_none() {
        return;
    }
    thread::sleep(DESCENDANT_DELAY);
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .expect("read inherited stdin to real EOF");
    assert_eq!(bytes.len(), INPUT_BYTES, "synthetic input must be complete");
    assert!(bytes.iter().all(|byte| *byte == b'x'));
}

#[test]
fn fixture_root_exits_after_forwarding_stdin() {
    if env::var_os("OPTIC_317_STDIN_SYNTHETIC").is_none() {
        return;
    }
    let mut descendant = synthetic_test("fixture_descendant_delays_stdin_eof");
    descendant
        .stdin(Stdio::inherit())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _owned_fixture = descendant
        .spawn()
        .expect("create finite synthetic stdin descendant");
    // Terminate the disposable root only; its descendant keeps the read end
    // while it sleeps for a fixed bounded duration. No real Git process.
    std::process::exit(0);
}

#[test]
fn fixture_observes_late_writer_join() {
    if env::var_os("OPTIC_317_STDIN_SYNTHETIC").is_none() {
        return;
    }
    let runner = HardenedCommandRunner::new(RUNNER_DEADLINE).expect("runner");
    let spec = HardenedCommandSpec {
        executable: env::current_exe().expect("fixed test executable"),
        cwd: env::current_dir().expect("fixture cwd"),
        args: [
            "--exact",
            "fixture_root_exits_after_forwarding_stdin",
            "--nocapture",
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
        env: BTreeMap::from([("OPTIC_317_STDIN_SYNTHETIC".into(), "1".into())]),
        output_limit: 64 * 1024,
    };
    let input = vec![b'x'; INPUT_BYTES];
    let started = Instant::now();
    let output = runner
        .run_with_input(&spec, &input, INPUT_BYTES as u64)
        .expect("existing writer join waits for delayed reader and EOF");
    let elapsed = started.elapsed();
    assert!(output.status.success(), "direct root exited successfully");
    assert!(
        elapsed > Duration::from_millis(800),
        "expected writer join beyond 200ms deadline; elapsed={elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "bounded synthetic reader did not complete promptly"
    );
    println!(
        "OPTIC_317_STDIN_LATE_EOF deadline_ms=200 input_bytes={} elapsed_ms={} root_success=true",
        input.len(),
        elapsed.as_millis()
    );
}

#[test]
fn externally_supervised_stdin_eof_deadline_reproduction() {
    let mut fixture = synthetic_test("fixture_observes_late_writer_join");
    fixture.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut host = fixture.spawn().expect("start isolated diagnostic host");
    let started = Instant::now();
    loop {
        if let Some(status) = host.try_wait().expect("poll fixture host") {
            let output = host.wait_with_output().expect("capture bounded host output");
            assert!(
                status.success(),
                "synthetic stdin fixture failed: stdout={} stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("OPTIC_317_STDIN_LATE_EOF"),
                "missing measured late EOF witness: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            return;
        }
        if started.elapsed() >= OUTER_DEADLINE {
            // Only the exact process handle spawned by this test is killed.
            // The descendant has a finite 1.6s delay; do not claim a native
            // process-tree containment or independently observed empty Job.
            let _ = host.kill();
            let _ = host.wait();
            panic!("isolated diagnostic host exceeded its 15s outer deadline");
        }
        thread::sleep(Duration::from_millis(20));
    }
}
