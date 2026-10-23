use std::{
    env, fs,
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};
#[test]
#[ignore = "synthetic child, only launched suspended inside supervised Jobs"]
fn synthetic_root() {
    let root = std::path::PathBuf::from(env::var_os("OPTIC317_OWNER_ROOT").unwrap());
    fs::write(root.join("executed"), b"fixed child").unwrap();
    println!("FIXED_STDOUT");
    eprintln!("FIXED_STDERR");
    let mode = env::var("OPTIC317_OWNER_MODE").unwrap();
    if mode == "descendant-pipes" {
        let mut command = std::process::Command::new(env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "process::authority_native_owner_tests::native_children::synthetic_descendant",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env("OPTIC317_OWNER_ROOT", &root)
            .env("TEMP", &root)
            .env("TMP", &root)
            .current_dir(&root)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        for key in ["SystemRoot", "WINDIR"] {
            if let Some(v) = env::var_os(key) {
                command.env(key, v);
            }
        }
        let retained_by_job = command.spawn().unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while !root.join("descendant").exists() {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(2));
        }
        // Deliberate Windows root-exits/descendant-retains-pipes fixture.
        // The Job-owning test host, itself in the outer Job, owns termination
        // and empty-tree/EOF proof. This local Child handle is not that owner.
        drop(retained_by_job);
        return;
    }
    if mode == "output-limit" {
        io::stdout().write_all(&vec![b'x'; 100_000]).unwrap();
    }
    thread::sleep(Duration::from_secs(30));
}

#[test]
#[ignore = "fixed managed descendant retaining selected stdout/stderr"]
fn synthetic_descendant() {
    let root = std::path::PathBuf::from(env::var_os("OPTIC317_OWNER_ROOT").unwrap());
    println!("FIXED_DESCENDANT_STDOUT");
    eprintln!("FIXED_DESCENDANT_STDERR");
    fs::write(root.join("descendant"), b"fixed descendant").unwrap();
    thread::sleep(Duration::from_secs(30));
}
