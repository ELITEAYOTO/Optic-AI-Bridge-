//! Research fixture only. Run through scripts/research/issue317_supervisor.py.
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use optic_bridge_runtime::{HardenedCommandRunner, HardenedCommandSpec};

fn await_file(root: &Path, name: &str, seconds: u64) {
    let started = Instant::now();
    while !root.join(name).is_file() {
        assert!(
            started.elapsed() < Duration::from_secs(seconds),
            "fixture guard"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    assert_eq!(args.len(), 3);
    let mode = args[0].to_str().expect("mode");
    let root = PathBuf::from(&args[1]);
    let case = args[2].to_str().expect("case");
    assert!(root.is_absolute() && root.is_dir());
    match mode {
        "runner" => {
            fs::write(root.join("runner.pid"), std::process::id().to_string()).expect("pid");
            // Independent OS containment is assigned before this gate opens.
            await_file(&root, "go", 8);
            let spec = HardenedCommandSpec {
                executable: env::current_exe().expect("fixture exe"),
                cwd: root.clone(),
                args: vec!["parent".into(), root.clone().into_os_string(), case.into()],
                env: ["SystemRoot", "SystemDrive"]
                    .into_iter()
                    .filter_map(|name| env::var_os(name).map(|value| (name.into(), value)))
                    .collect::<BTreeMap<_, _>>(),
                output_limit: 4096,
            };
            let runner = HardenedCommandRunner::new(Duration::from_millis(200)).expect("runner");
            let started = Instant::now();
            let result = if case == "stdin" {
                runner.run_with_input(&spec, &vec![0_u8; 256 * 1024], 256 * 1024)
            } else {
                runner.run(&spec)
            };
            let value = serde_json::json!({
                "runner_ms": started.elapsed().as_millis(),
                "ok": result.is_ok(),
                "error": result.as_ref().err().map(|error| format!("{error:?}")),
            });
            fs::write(root.join("result.json"), value.to_string()).expect("result");
        }
        "parent" => {
            fs::write(root.join("parent.pid"), std::process::id().to_string()).expect("pid");
            if case == "control" {
                fs::write(root.join("parent.exiting"), b"synthetic").expect("parent marker");
                return;
            }
            let stdout_held = case == "stdout" || case == "both";
            let stderr_held = case == "stderr" || case == "both";
            let mut command = Command::new(env::current_exe().expect("fixture exe"));
            command.args(["holder".as_ref(), root.as_os_str(), args[2].as_os_str()]);
            command.stdin(if case == "stdin" {
                Stdio::inherit()
            } else {
                Stdio::null()
            });
            command.stdout(if stdout_held {
                Stdio::inherit()
            } else {
                Stdio::null()
            });
            command.stderr(if stderr_held {
                Stdio::inherit()
            } else {
                Stdio::null()
            });
            // Intentional orphan: the external Job/subreaper supervisor owns and reaps it.
            #[expect(clippy::zombie_processes)]
            let _owned_by_outer_guard = command.spawn().expect("holder");
            await_file(&root, "holder.ready", 3);
            fs::write(root.join("parent.exiting"), b"synthetic").expect("parent marker");
        }
        "holder" => {
            fs::write(root.join("holder.pid"), std::process::id().to_string()).expect("pid");
            fs::write(root.join("holder.ready"), b"synthetic").expect("ready");
            // Secondary autonomous ceiling even if the outer supervisor fails.
            await_file(&root, "release", 8);
        }
        _ => panic!("unknown fixture mode"),
    }
}
