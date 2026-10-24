//! Deadline result classification, not a claim that pipe joins are bounded.
use optic_bridge_runtime::{HardenedCommandError, HardenedCommandRunner, HardenedCommandSpec};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const INPUT: usize = 2 * 1024 * 1024;
const HOLD: Duration = Duration::from_millis(1600);
const DEADLINE: Duration = Duration::from_millis(200);

fn context() -> (PathBuf, String) {
    let root = PathBuf::from(env::var_os("OPTIC317_LATE_ROOT").expect("supervised fixture root"));
    let mode = env::var("OPTIC317_LATE_MODE").expect("fixed case");
    assert!(matches!(
        mode.as_str(),
        "stdout" | "stderr" | "both" | "stdin" | "fast" | "broken-pipe"
            | "late-output-limit" | "late-broken-pipe"
    ));
    (root, mode)
}

fn environment(root: &std::path::Path, mode: &str) -> BTreeMap<std::ffi::OsString, std::ffi::OsString> {
    let mut vars = BTreeMap::from([
        ("OPTIC317_LATE_ROOT".into(), root.as_os_str().to_owned()),
        ("OPTIC317_LATE_MODE".into(), mode.into()),
        ("TEMP".into(), root.as_os_str().to_owned()),
        ("TMP".into(), root.as_os_str().to_owned()),
    ]);
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = env::var_os(name) {
            vars.insert(name.into(), value);
        }
    }
    vars
}

fn fixed(name: &str) -> Command {
    let (root, mode) = context();
    let mut command = Command::new(env::current_exe().expect("fixed test executable"));
    command
        .args(["--exact", name, "--ignored", "--nocapture", "--test-threads=1"])
        .current_dir(&root)
        .env_clear()
        .envs(environment(&root, &mode));
    command
}

#[test]
#[ignore = "fixed child; invoke only inside the independently supervised fixture"]
fn fixture_descendant() {
    let (root, mode) = context();
    fs::write(root.join("descendant-start"), b"owns selected pipe ends").unwrap();
    thread::sleep(HOLD);
    if mode == "stdin" {
        let mut bytes = Vec::new();
        io::stdin().take((INPUT + 1) as u64).read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes.len(), INPUT);
        assert!(bytes.iter().all(|byte| *byte == b'x'));
    }
    if matches!(mode.as_str(), "stdout" | "both") {
        println!("fixed-stdout");
        io::stdout().flush().unwrap();
    }
    if matches!(mode.as_str(), "stderr" | "both") {
        eprintln!("fixed-stderr");
        io::stderr().flush().unwrap();
    }
    if mode == "late-output-limit" {
        io::stdout().write_all(&[b'x'; 8192]).unwrap();
        io::stdout().flush().unwrap();
    }
    fs::write(root.join("descendant-done"), b"finite fixture completed").unwrap();
}

#[test]
#[ignore = "fixed root; invoke only inside the independently supervised fixture"]
fn fixture_root() {
    let (root, mode) = context();
    fs::write(root.join("root-start"), b"fixed root started").unwrap();
    if !matches!(mode.as_str(), "fast" | "broken-pipe") {
        let mut command = fixed("fixture_descendant");
        command.stdin(if matches!(mode.as_str(), "stdin" | "late-broken-pipe") {
            Stdio::inherit()
        } else {
            Stdio::null()
        });
        command.stdout(if matches!(mode.as_str(), "stdout" | "both" | "late-output-limit") {
            Stdio::inherit()
        } else {
            Stdio::null()
        });
        command.stderr(if matches!(mode.as_str(), "stderr" | "both") {
            Stdio::inherit()
        } else {
            Stdio::null()
        });
        let _finite_child = command.spawn().unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while !root.join("descendant-start").exists() {
            assert!(Instant::now() < until, "fixed descendant failed to start");
            thread::sleep(Duration::from_millis(2));
        }
        // Deliberate synthetic parent exit; the independent outer Job/group
        // owns every fixture descendant. Production must not copy this pattern.
        fs::write(root.join("root-return"), b"root exits before descendant").unwrap();
        std::process::exit(0);
    }
    fs::write(root.join("root-return"), b"fixed root exits").unwrap();
    std::process::exit(0);
}

#[test]
#[ignore = "requires independent outer Job/group ownership before the effect gate"]
fn classifies_result() {
    let (root, mode) = context();
    let limit = if mode == "late-output-limit" { 1024 } else { 64 * 1024 };
    let spec = HardenedCommandSpec {
        executable: env::current_exe().unwrap(),
        cwd: root.clone(),
        args: ["--exact", "fixture_root", "--ignored", "--nocapture", "--test-threads=1"]
            .into_iter().map(Into::into).collect(),
        env: environment(&root, &mode),
        output_limit: limit,
    };
    let budget = if matches!(mode.as_str(), "fast" | "broken-pipe") {
        Duration::from_secs(5)
    } else {
        DEADLINE
    };
    let runner = HardenedCommandRunner::new(budget).unwrap();
    let started = Instant::now();
    let result = if matches!(mode.as_str(), "stdin" | "broken-pipe" | "late-broken-pipe") {
        runner.run_with_input(&spec, &vec![b'x'; INPUT], INPUT as u64)
    } else {
        runner.run(&spec)
    };
    let elapsed = started.elapsed();
    assert!(root.join("root-return").exists(), "root fixture did not reach its fixed exit");
    if !matches!(mode.as_str(), "fast" | "broken-pipe") {
        assert!(root.join("descendant-done").exists());
        assert!(elapsed > Duration::from_millis(800), "late join was not exercised");
    }
    match mode.as_str() {
        "fast" => assert!(result.unwrap().status.success()),
        "broken-pipe" | "late-broken-pipe" => assert!(
            matches!(result, Err(HardenedCommandError::Io(ref error)) if error.kind() == io::ErrorKind::BrokenPipe),
            "existing input I/O error precedence changed: {result:?}"
        ),
        "late-output-limit" => assert!(
            matches!(result, Err(HardenedCommandError::OutputLimitExceeded { limit: 1024 })),
            "existing output ceiling error precedence changed: {result:?}"
        ),
        _ => assert!(
            matches!(result, Err(HardenedCommandError::TimedOut)),
            "LATE_SUCCESS_MUST_BE_TIMEOUT mode={mode} elapsed_ms={} result={result:?}",
            elapsed.as_millis()
        ),
    }
    println!(
        "OPTIC317_CLASSIFICATION mode={mode} elapsed_ms={} deadline_ms={} expected_result_observed=true",
        elapsed.as_millis(), budget.as_millis()
    );
}
