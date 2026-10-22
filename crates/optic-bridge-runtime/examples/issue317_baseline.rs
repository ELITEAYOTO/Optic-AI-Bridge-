//! Synthetic baseline only. Run through the independent Windows supervisor.
use optic_bridge_runtime::{HardenedCommandRunner, HardenedCommandSpec};
use std::{
    collections::BTreeMap,
    env,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn main() {
    let args: Vec<_> = env::args_os().collect();
    assert_eq!(args.len(), 5);
    let gate = PathBuf::from(env::var_os("OPTIC317_BASELINE_GATE").expect("outer Job gate"));
    let deadline = Instant::now() + Duration::from_secs(5);
    while !gate.exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    let mut child_env = BTreeMap::new();
    for key in ["SystemRoot", "WINDIR"] {
        if let Some(value) = env::var_os(key) {
            child_env.insert(key.into(), value);
        }
    }
    let mode = args[3].to_str().unwrap();
    let spec = HardenedCommandSpec {
        executable: PathBuf::from(&args[1]),
        cwd: PathBuf::from(&args[4]),
        args: vec![args[2].clone(), args[3].clone()],
        env: child_env,
        output_limit: 65536,
    };
    let runner = HardenedCommandRunner::new(Duration::from_millis(200)).unwrap();
    let start = Instant::now();
    let result = if mode == "stdin" {
        runner.run_with_input(&spec, &vec![b'x'; 2 * 1024 * 1024], 2 * 1024 * 1024)
    } else {
        runner.run(&spec)
    };
    match result {
        Ok(out) => println!(
            "OPTIC317_BASELINE result=exit code={} elapsed_ms={}",
            out.status.code().unwrap_or(-1),
            start.elapsed().as_millis()
        ),
        Err(error) => println!(
            "OPTIC317_BASELINE result={error} elapsed_ms={}",
            start.elapsed().as_millis()
        ),
    }
}
