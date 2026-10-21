//! Research fixture only. Run through scripts/research/issue317_supervisor.py.
use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
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
                env: ["SystemRoot", "SystemDrive", "OPTIC_RESEARCH_PYTHON"]
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
                "exit_code": result.as_ref().ok().and_then(|output| output.status.code()),
            });
            fs::write(root.join("result.json"), value.to_string()).expect("result");
        }
        "parent" => {
            fs::write(root.join("parent.pid"), std::process::id().to_string()).expect("pid");
            if case == "control" || case == "nonzero" {
                fs::write(root.join("parent.exiting"), b"synthetic").expect("parent marker");
                if case == "nonzero" {
                    std::process::exit(7);
                }
                return;
            }
            if case == "selective-null" || case == "escape-both" {
                // Contrast: Python explicitly closes non-standard inherited
                // handles. escape-both deliberately creates a separate Unix session;
                // only the research subreaper/watchdog may recover that fixture.
                let python = env::var_os("OPTIC_RESEARCH_PYTHON").expect("fixture Python");
                let script = "import subprocess,sys; subprocess.Popen([sys.argv[1],'holder',sys.argv[2],sys.argv[3]],stdin=subprocess.DEVNULL,stdout=None if sys.argv[3]=='escape-both' else subprocess.DEVNULL,stderr=None if sys.argv[3]=='escape-both' else subprocess.DEVNULL,close_fds=True,start_new_session=sys.argv[3]=='escape-both')";
                // The escape case MUST carry the parent's actual pipe fds
                // into Python, or stdout=None/stderr=None inherit /dev/null
                // and cannot exercise a retained pipe across setsid().
                let escape_pipes = case == "escape-both";
                let status = Command::new(python)
                    .args(["-I", "-S", "-c", script])
                    .arg(env::current_exe().expect("fixture exe"))
                    .arg(&root)
                    .arg(case)
                    .stdin(Stdio::null())
                    .stdout(if escape_pipes {
                        Stdio::inherit()
                    } else {
                        Stdio::null()
                    })
                    .stderr(if escape_pipes {
                        Stdio::inherit()
                    } else {
                        Stdio::null()
                    })
                    .status()
                    .expect("selective inheritance helper");
                assert!(status.success());
                await_file(&root, "holder.ready", 3);
                fs::write(root.join("parent.exiting"), b"synthetic").expect("parent marker");
                return;
            }
            let stdout_held = ["stdout", "both", "timeout-both", "limit-both"].contains(&case);
            let stderr_held = ["stderr", "both", "timeout-both", "limit-both"].contains(&case);
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
            if case == "limit-both" {
                std::io::stdout()
                    .write_all(&[b'X'; 4097])
                    .expect("bounded synthetic overflow");
            }
            if case == "timeout-both" {
                await_file(&root, "release", 8);
            }
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
