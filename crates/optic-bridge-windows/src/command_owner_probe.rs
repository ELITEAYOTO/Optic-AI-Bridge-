//! Experimental native slice only. Not wired to HardenedCommandRunner.
use super::{OwnedJob, resume_process_threads};
use crate::CapturedOutputPipe;
use std::{
    io::{self, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::HANDLE,
    System::{JobObjects::AssignProcessToJobObject, Threading::CREATE_SUSPENDED},
};

const CAP: usize = 64 * 1024;
const POLL: Duration = Duration::from_millis(5);

struct ProbeOwner {
    child: Child,
    job: OwnedJob,
    out: CapturedOutputPipe,
    err: CapturedOutputPipe,
    assigned: bool,
    root_exit: Option<ExitStatus>,
    terminal_proven: bool,
}
impl ProbeOwner {
    // A failed native creation cannot leave an untracked child. After spawn,
    // assignment/resume are separate operations on a retained owner.
    fn create(root: &Path, mode: &str) -> io::Result<Self> {
        let job = OwnedJob::create(4, 512 * 1024 * 1024, 25)?;
        let out = CapturedOutputPipe::new()?;
        let err = CapturedOutputPipe::new()?;
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args([
                "--exact",
                "job::command_owner_probe::fixture",
                "--nocapture",
            ])
            .env_clear()
            .env("OPTIC317_ROOT", root)
            .env("OPTIC317_MODE", mode)
            .stdin(if mode == "stdin_block" {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::from(out.writer_handle()?.try_clone_to_owned()?))
            .stderr(Stdio::from(err.writer_handle()?.try_clone_to_owned()?))
            .creation_flags(CREATE_SUSPENDED.0);
        if let Some(value) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", value);
        }
        let child = command.spawn()?;
        let mut owner = Self {
            child,
            job,
            out,
            err,
            assigned: false,
            root_exit: None,
            terminal_proven: false,
        };
        owner.out.close_parent_writer();
        owner.err.close_parent_writer();
        Ok(owner)
    }
    fn assign(&mut self) -> io::Result<()> {
        // SAFETY: exact live child and Job handles are owned by self. The root
        // was created suspended and cannot execute fixture code before resume.
        unsafe { AssignProcessToJobObject(self.job.raw(), HANDLE(self.child.as_raw_handle())) }
            .map_err(io::Error::other)?;
        self.assigned = true;
        Ok(())
    }
    fn resume(&self) -> io::Result<()> {
        if !self.assigned {
            return Err(io::Error::other("unassigned root cannot resume"));
        }
        resume_process_threads(HANDLE(self.child.as_raw_handle()))
    }
    fn stop(&mut self) {
        let _ = self.job.terminate(1);
        // Covers an unassigned, still-suspended partial creation too.
        let _ = self.child.kill();
    }
    fn confirm(&mut self, inject_missing_proof: bool) -> io::Result<bool> {
        if self.root_exit.is_none() {
            self.root_exit = self.child.try_wait()?;
        }
        if inject_missing_proof {
            return Ok(false);
        }
        self.terminal_proven = self.root_exit.is_some() && self.job.active_processes()? == 0;
        Ok(self.terminal_proven)
    }
    fn stop_and_confirm(&mut self) -> io::Result<bool> {
        self.stop();
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until {
            if self.confirm(false)? {
                return Ok(true);
            }
            thread::sleep(POLL);
        }
        Ok(false)
    }
}
impl Drop for ProbeOwner {
    fn drop(&mut self) {
        // Backstop only, never a completion proof. Outer independently owned
        // fixture Job also recovers the tree if the test process panics/dies.
        if !self.terminal_proven {
            self.stop();
        }
    }
}

struct Observation {
    elapsed_ms: u128,
    reason: &'static str,
    captured: usize,
    output: [Vec<u8>; 2],
    exit: Option<i32>,
}
fn collect(owner: &mut ProbeOwner, deadline: Duration, cancel: bool) -> io::Result<Observation> {
    let started = Instant::now();
    let mut captured = 0_usize;
    let mut output = [Vec::new(), Vec::new()];
    let mut eof = [false; 2];
    let mut reason = "exit";
    let mut stopping = false;
    let mut cleanup_until = None;
    loop {
        if !stopping && cancel {
            reason = "cancel";
            stopping = true;
        }
        if !stopping && started.elapsed() >= deadline {
            reason = "deadline";
            stopping = true;
        }
        for (index, pipe) in [&mut owner.out, &mut owner.err].into_iter().enumerate() {
            if eof[index] {
                continue;
            }
            let chunk = pipe.try_read(4096)?;
            eof[index] = chunk.eof;
            let remaining = CAP.saturating_sub(captured);
            if chunk.bytes.len() > remaining && !stopping {
                reason = "output";
                stopping = true;
            }
            let accepted = chunk.bytes.len().min(remaining);
            output[index].extend_from_slice(&chunk.bytes[..accepted]);
            captured += accepted;
        }
        if owner.root_exit.is_none() {
            owner.root_exit = owner.child.try_wait()?;
        }
        // End of the root must not leave descendants holding the output pipes.
        if owner.root_exit.is_some() {
            stopping = true;
        }
        if stopping && cleanup_until.is_none() {
            owner.stop();
            cleanup_until = Some(Instant::now() + Duration::from_secs(2));
        }
        if stopping && owner.confirm(false)? && eof.into_iter().all(|value| value) {
            return Ok(Observation {
                elapsed_ms: started.elapsed().as_millis(),
                reason,
                captured,
                output,
                exit: owner.root_exit.and_then(|value| value.code()),
            });
        }
        if cleanup_until.is_some_and(|until| Instant::now() >= until) {
            // Ownership is still in caller's owner, not hidden in an error or
            // dropped worker. The matrix explicitly recovers before proceeding.
            return Err(io::Error::other("retained owner lacks terminal/EOF proof"));
        }
        thread::sleep(POLL);
    }
}

fn marker(root: &Path, name: &str) {
    std::fs::write(root.join(name), b"synthetic").unwrap();
}
fn wait_marker(root: &Path, name: &str) {
    let until = Instant::now() + Duration::from_secs(3);
    while !root.join(name).exists() {
        assert!(Instant::now() < until, "fixture marker deadline");
        thread::sleep(POLL);
    }
}

#[test]
fn fixture() {
    let Some(root) = std::env::var_os("OPTIC317_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let mode = std::env::var("OPTIC317_MODE").unwrap();
    marker(&root, "started");
    match mode.as_str() {
        "normal" => {
            println!("synthetic-out");
            eprintln!("synthetic-err");
        }
        "nonzero" => std::process::exit(7),
        "descendant" => {
            let mut child = Command::new(std::env::current_exe().unwrap());
            child
                .args([
                    "--exact",
                    "job::command_owner_probe::fixture",
                    "--nocapture",
                ])
                .env("OPTIC317_MODE", "holder")
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let _retained_by_job = child.spawn().unwrap();
            wait_marker(&root, "holder");
            std::process::exit(0);
        }
        "holder" => {
            marker(&root, "holder");
            println!("holder-out");
            eprintln!("holder-err");
            thread::sleep(Duration::from_secs(8));
        }
        "idle" | "stdin_block" => thread::sleep(Duration::from_secs(8)),
        "output" => {
            let data = [b'x'; 8192];
            for _ in 0..32 {
                std::io::stdout().write_all(&data).unwrap();
            }
            thread::sleep(Duration::from_secs(8));
        }
        _ => panic!("unknown fixture mode"),
    }
}

#[test]
fn native_windows_owner_matrix() {
    let Some(gate) = std::env::var_os("OPTIC317_OUTER_GATE") else {
        return;
    };
    let gate = PathBuf::from(gate);
    let until = Instant::now() + Duration::from_secs(5);
    while !gate.exists() {
        assert!(Instant::now() < until, "independent ownership gate missing");
        thread::sleep(POLL);
    }
    let base = gate.parent().unwrap();
    let mut cases = 0;
    for (mode, expected, cancel) in [
        ("normal", "exit", false),
        ("nonzero", "exit", false),
        ("descendant", "exit", false),
        ("idle", "deadline", false),
        ("idle", "cancel", true),
        ("output", "output", false),
    ] {
        let root = base.join(format!("case-{cases}"));
        std::fs::create_dir(&root).unwrap();
        let mut owner = ProbeOwner::create(&root, mode).unwrap();
        assert!(!root.join("started").exists(), "suspended root ran");
        assert!(owner.resume().is_err(), "unassigned resume allowed");
        owner.assign().unwrap();
        assert_eq!(owner.job.active_processes().unwrap(), 1);
        assert!(!root.join("started").exists(), "assignment resumed child");
        owner.resume().unwrap();
        // Give non-idle fixtures time to enter before starting the short test
        // deadline. This is startup readiness, not a production timeout change.
        if !cancel {
            wait_marker(&root, "started");
            if mode == "descendant" {
                wait_marker(&root, "holder");
            }
        }
        let result = collect(&mut owner, Duration::from_millis(200), cancel);
        if result.is_err() {
            assert!(owner.stop_and_confirm().unwrap());
        }
        let result = result.unwrap();
        assert_eq!(result.reason, expected);
        assert!(owner.terminal_proven);
        assert!(
            result.elapsed_ms < 2500,
            "bounded native collection exceeded"
        );
        assert!(result.captured <= CAP);
        if mode == "normal" {
            assert_eq!(result.exit, Some(0));
            assert!(result.output[0].windows(13).any(|w| w == b"synthetic-out"));
            assert!(result.output[1].windows(13).any(|w| w == b"synthetic-err"));
        }
        if mode == "nonzero" {
            assert_eq!(result.exit, Some(7));
        }
        println!(
            "OPTIC317_CASE {{\"case\":\"{mode}-{expected}\",\"elapsed_ms\":{},\"captured\":{},\"tree_empty\":true,\"pipes_eof\":true}}",
            result.elapsed_ms, result.captured
        );
        cases += 1;
    }
    // A partially created root is retained before assignment, never resumed,
    // and can be recovered without relying on job membership.
    let root = base.join("partial");
    std::fs::create_dir(&root).unwrap();
    let mut partial = ProbeOwner::create(&root, "idle").unwrap();
    assert!(!partial.assigned);
    assert!(partial.resume().is_err());
    assert!(partial.stop_and_confirm().unwrap());
    assert!(!root.join("started").exists());
    cases += 1;
    println!(
        "OPTIC317_CASE {{\"case\":\"partial-before-assignment\",\"tree_empty\":true,\"never_resumed\":true}}"
    );

    // A child that never reads stdin must not retain a blocked writer after
    // owned tree termination. This is a test-only interruption proof, not a
    // general guarantee of writer cancellation when cleanup is uncertain.
    let root = base.join("stdin-block");
    std::fs::create_dir(&root).unwrap();
    let mut blocked = ProbeOwner::create(&root, "stdin_block").unwrap();
    blocked.assign().unwrap();
    blocked.resume().unwrap();
    wait_marker(&root, "started");
    let mut stdin = blocked.child.stdin.take().expect("piped child stdin");
    let (writer_tx, writer_rx) = std::sync::mpsc::channel();
    let writer = thread::spawn(move || {
        let result = stdin.write_all(&vec![b'i'; 2 * 1024 * 1024]);
        writer_tx.send(result.is_err()).unwrap();
    });
    assert!(
        matches!(
            writer_rx.recv_timeout(Duration::from_millis(100)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ),
        "writer did not encounter backpressure"
    );
    assert!(blocked.stop_and_confirm().unwrap());
    assert!(
        writer_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("blocked stdin writer did not unblock"),
        "unread input completed successfully"
    );
    writer.join().unwrap();
    let observation = collect(&mut blocked, Duration::from_millis(200), false).unwrap();
    assert_eq!(observation.reason, "exit");
    assert!(blocked.terminal_proven);
    assert!(observation.elapsed_ms < 2000);
    cases += 1;
    println!(
        "OPTIC317_CASE {{\"case\":\"stdin-backpressure-stop\",\"tree_empty\":true,\"pipes_eof\":true,\"writer_unblocked\":true,\"writer_joined\":true}}"
    );

    // Missing observation is injected, not an actual OS termination failure.
    // Keep the SAME native owner/handles until a later real confirmation.
    let root = base.join("uncertain");
    std::fs::create_dir(&root).unwrap();
    let mut retained = ProbeOwner::create(&root, "idle").unwrap();
    retained.assign().unwrap();
    retained.resume().unwrap();
    wait_marker(&root, "started");
    retained.stop();
    assert!(!retained.confirm(true).unwrap());
    assert!(!retained.terminal_proven);
    assert!(retained.stop_and_confirm().unwrap());
    cases += 1;
    println!(
        "OPTIC317_CASE {{\"case\":\"retained-missing-proof-retry\",\"tree_empty\":true,\"injected\":true}}"
    );
    println!(
        "OPTIC317_NATIVE_COMPLETE {{\"cases\":{cases},\"production_runner_wired\":false,\"stdin_writer_proven\":false}}"
    );
}
