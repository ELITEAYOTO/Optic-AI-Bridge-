//! #317 research only: a real temporary Git CAS can complete before the
//! existing HardenedCommandRunner returns TimedOut. No production integration
//! flow, user repository or private data is touched.
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use optic_bridge_runtime::{HardenedCommandError, HardenedCommandRunner, HardenedCommandSpec};

const TARGET: &str = "refs/optic/integration/research-timeout";
const RUNNER_DEADLINE: Duration = Duration::from_millis(2800);
const SHIM_SLEEP: Duration = Duration::from_secs(8);

struct FixtureRoot(PathBuf);

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git_executable() -> PathBuf {
    let path = env::var_os("PATH").expect("hosted runner PATH");
    for dir in env::split_paths(&path) {
        let candidate = dir.join(if cfg!(windows) { "git.exe" } else { "git" });
        if candidate.is_file() {
            return fs::canonicalize(candidate).expect("canonical git executable");
        }
    }
    panic!("hosted runner Git executable not found");
}

fn git_output(git: &Path, repo: &Path, args: &[&str]) -> String {
    let output = Command::new(git)
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("run isolated Git fixture command");
    assert!(
        output.status.success(),
        "isolated Git fixture command failed: {:?}; status={:?}",
        args,
        output.status.code()
    );
    String::from_utf8(output.stdout)
        .expect("fixture Git stdout UTF-8")
        .trim()
        .to_owned()
}

fn run_shim(args: &[String]) {
    assert_eq!(args.len(), 8, "shim argument count");
    let case = &args[1];
    let git = PathBuf::from(&args[2]);
    let repo = PathBuf::from(&args[3]);
    let target = &args[4];
    let new = &args[5];
    let expected = &args[6];
    let marker = PathBuf::from(&args[7]);
    assert!(repo.is_absolute() && repo.is_dir());
    assert!(git.is_absolute() && git.is_file());
    assert_eq!(target, TARGET);
    match case.as_str() {
        "cas-then-hold" => {
            git_output(
                &git,
                &repo,
                &["update-ref", "--no-deref", target, new, expected],
            );
            fs::write(&marker, b"cas-executed").expect("write CAS-completion marker");
        }
        "no-cas-then-hold" => {
            // Control: no mutation, same successful child startup and timeout.
            fs::write(&marker, b"no-cas").expect("write no-effect marker");
        }
        _ => panic!("unknown synthetic case"),
    }
    // Keep the *direct child* alive past the runner deadline. The runner
    // kills and reaps it; no retained descendants or shared files are used.
    thread::sleep(SHIM_SLEEP);
}

fn run_driver() {
    let base = env::temp_dir().join(format!("optic-317-git-cas-timeout-{}", std::process::id()));
    fs::create_dir(&base).expect("unique fixture root");
    let _owned_root = FixtureRoot(base.clone());
    let git = git_executable();
    git_output(&git, &base, &["init", "--quiet"]);
    fs::write(base.join("old.txt"), b"synthetic old\n").expect("old blob fixture");
    fs::write(base.join("new.txt"), b"synthetic new\n").expect("new blob fixture");
    let old = git_output(&git, &base, &["hash-object", "-w", "old.txt"]);
    let new = git_output(&git, &base, &["hash-object", "-w", "new.txt"]);
    assert_ne!(old, new, "distinct Git objects");

    let runner = HardenedCommandRunner::new(RUNNER_DEADLINE).expect("bounded runner");
    for (case, expect_mutation) in [("cas-then-hold", true), ("no-cas-then-hold", false)] {
        git_output(&git, &base, &["update-ref", TARGET, &old]);
        let marker = base.join(format!("{case}.ready"));
        let args = [
            "shim".into(),
            case.into(),
            git.as_os_str().to_os_string(),
            base.as_os_str().to_os_string(),
            TARGET.into(),
            new.clone().into(),
            old.clone().into(),
            marker.as_os_str().to_os_string(),
        ];
        let env_vars: BTreeMap<_, _> = ["SystemRoot", "SystemDrive", "WINDIR", "TMP", "TEMP"]
            .into_iter()
            .filter_map(|name| env::var_os(name).map(|value| (name.into(), value)))
            .collect();
        let spec = HardenedCommandSpec {
            executable: env::current_exe().expect("fixture binary"),
            cwd: base.clone(),
            args: args.into(),
            env: env_vars,
            output_limit: 4096,
        };
        let result = runner.run(&spec);
        assert!(
            matches!(result, Err(HardenedCommandError::TimedOut)),
            "expected native runner timeout after child startup, observed {result:?}"
        );
        assert!(
            marker.is_file(),
            "fixture did not reach its intended phase before timeout: {case}"
        );
        let observed = git_output(&git, &base, &["rev-parse", "--verify", TARGET]);
        assert_eq!(
            observed.as_str(),
            if expect_mutation {
                new.as_str()
            } else {
                old.as_str()
            },
            "Git ref state mismatched expected synthetic scenario {case}"
        );
        println!(
            "OPTIC_317_GIT_CAS_TIMEOUT case={case} error=TimedOut actual_ref={} marker=observed",
            if expect_mutation {
                "advanced"
            } else {
                "unchanged"
            }
        );
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("driver") if args.len() == 1 => run_driver(),
        Some("shim") => run_shim(&args),
        _ => panic!("usage: issue317_git_ref_timeout driver|shim ..."),
    }
}
