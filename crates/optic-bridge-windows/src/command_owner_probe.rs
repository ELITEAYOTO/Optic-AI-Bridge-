//! Experimental native slice only. Not wired to HardenedCommandRunner.
use super::{OwnedJob, resume_process_threads};
use crate::CapturedOutputPipe;
use std::{
    collections::BTreeMap,
    io::{self, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Arc, Barrier, Mutex},
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
        self.assign_native(HANDLE(self.child.as_raw_handle()))
    }
    fn assign_native(&mut self, process: HANDLE) -> io::Result<()> {
        // SAFETY: the Job handle is owned here. Windows validates the opaque
        // process handle; the test deliberately passes a null handle and
        // requires native rejection before any root thread may resume.
        unsafe { AssignProcessToJobObject(self.job.raw(), process) }.map_err(io::Error::other)?;
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

// A research-only, capacity-one stand-in for the eventual service owner.
// It is deliberately not exported, not an authority or a production registry.
struct RetainedCapacityFixture {
    next_ticket: u64,
    reserved: Option<u64>,
    owner: Option<ProbeOwner>,
}

impl RetainedCapacityFixture {
    fn new() -> Self {
        Self {
            next_ticket: 0,
            reserved: None,
            owner: None,
        }
    }

    // Reserve before native creation, even while no child/job exists yet.
    fn reserve(&mut self) -> Option<u64> {
        if self.reserved.is_some() {
            return None;
        }
        self.next_ticket += 1;
        self.reserved = Some(self.next_ticket);
        self.reserved
    }

    fn attach(&mut self, ticket: u64, owner: ProbeOwner) {
        assert_eq!(self.reserved, Some(ticket));
        assert!(self.owner.is_none());
        self.owner = Some(owner);
    }

    fn stop(&mut self, ticket: u64) {
        assert_eq!(self.reserved, Some(ticket));
        self.owner.as_mut().expect("attached owner").stop();
    }

    // A missing native observation, even after a stop, must retain the owner,
    // capacity and pipes. Also require both EOF observations before release.
    fn try_reap(&mut self, ticket: u64, missing_proof: bool) -> io::Result<bool> {
        assert_eq!(self.reserved, Some(ticket));
        let owner = self.owner.as_mut().expect("attached owner");
        if !owner.confirm(missing_proof)? {
            return Ok(false);
        }
        let mut both_eof = true;
        for pipe in [&mut owner.out, &mut owner.err] {
            let chunk = pipe.try_read(4096)?;
            both_eof &= chunk.eof;
        }
        if !both_eof {
            return Ok(false);
        }
        self.owner.take();
        self.reserved = None;
        Ok(true)
    }
}

// Research-only two-slot registry. Operation identity and owner domain are
// distinct; this does not authenticate a caller or provision a live session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureAuthority {
    Session,
    HostStartup,
}

enum ConcurrentSlot {
    Reserved(FixtureAuthority),
    Running(FixtureAuthority, ProbeOwner),
}

struct ConcurrentOwnerFixture {
    slots: BTreeMap<u64, ConcurrentSlot>,
    next_ticket: u64,
}

impl ConcurrentOwnerFixture {
    fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            next_ticket: 0,
        }
    }

    fn reserve(&mut self, authority: FixtureAuthority) -> Option<u64> {
        if self.slots.len() >= 2 {
            return None;
        }
        self.next_ticket += 1;
        self.slots
            .insert(self.next_ticket, ConcurrentSlot::Reserved(authority));
        Some(self.next_ticket)
    }

    fn attach(&mut self, ticket: u64, authority: FixtureAuthority, owner: ProbeOwner) {
        let slot = self.slots.get_mut(&ticket).expect("reserved ticket");
        assert!(matches!(slot, ConcurrentSlot::Reserved(a) if *a == authority));
        *slot = ConcurrentSlot::Running(authority, owner);
    }

    fn stop(&mut self, authority: FixtureAuthority, ticket: u64) -> bool {
        match self.slots.get_mut(&ticket) {
            Some(ConcurrentSlot::Running(a, owner)) if *a == authority => {
                owner.stop();
                true
            }
            _ => false,
        }
    }

    fn try_reap(
        &mut self,
        authority: FixtureAuthority,
        ticket: u64,
        inject_missing_proof: bool,
    ) -> io::Result<bool> {
        let Some(ConcurrentSlot::Running(a, owner)) = self.slots.get_mut(&ticket) else {
            return Ok(false);
        };
        if *a != authority || !owner.confirm(inject_missing_proof)? {
            return Ok(false);
        }
        let mut eof = true;
        for pipe in [&mut owner.out, &mut owner.err] {
            eof &= pipe.try_read(4096)?.eof;
        }
        if !eof {
            return Ok(false);
        }
        self.slots.remove(&ticket);
        Ok(true)
    }

    fn cancel_unspawned(&mut self, authority: FixtureAuthority, ticket: u64) -> bool {
        if !matches!(
            self.slots.get(&ticket),
            Some(ConcurrentSlot::Reserved(a)) if *a == authority
        ) {
            return false;
        }
        self.slots.remove(&ticket);
        true
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

    // Demonstrate a real Win32 assignment error while a suspended root is
    // owned but not yet in the Job. The invalid native handle is intentional.
    // This does not simulate a failure after a successful real assignment.
    let root = base.join("assign-invalid-handle");
    std::fs::create_dir(&root).unwrap();
    let mut failed = ProbeOwner::create(&root, "idle").unwrap();
    assert!(
        failed.assign_native(HANDLE::default()).is_err(),
        "native assignment unexpectedly accepted invalid process handle"
    );
    assert!(!failed.assigned);
    assert_eq!(failed.job.active_processes().unwrap(), 0);
    assert!(failed.resume().is_err());
    assert!(!root.join("started").exists());
    assert!(failed.stop_and_confirm().unwrap());
    assert!(!root.join("started").exists());
    assert!(failed.out.try_read(4096).unwrap().eof);
    assert!(failed.err.try_read(4096).unwrap().eof);
    cases += 1;
    println!(
        "OPTIC317_CASE {{\"case\":\"native-assignment-error\",\"tree_empty\":true,\"never_resumed\":true,\"pipes_eof\":true,\"native_error\":true}}"
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

    // Capacity must be reserved *before* spawn and survive loss of a caller
    // reference. An independent thread requests stop without acquiring a
    // second operation slot; missing proof retains the same native owner.
    let root = base.join("retained-capacity");
    std::fs::create_dir(&root).unwrap();
    let registry = Arc::new(Mutex::new(RetainedCapacityFixture::new()));
    let ticket = registry.lock().unwrap().reserve().expect("one free slot");
    assert!(registry.lock().unwrap().reserve().is_none());
    let mut owner = ProbeOwner::create(&root, "idle").unwrap();
    owner.assign().unwrap();
    owner.resume().unwrap();
    wait_marker(&root, "started");
    registry.lock().unwrap().attach(ticket, owner);
    let caller_reference = Arc::clone(&registry);
    drop(caller_reference);
    assert_eq!(Arc::strong_count(&registry), 1);
    assert!(!registry.lock().unwrap().try_reap(ticket, true).unwrap());
    assert!(registry.lock().unwrap().reserve().is_none());

    let stop_registry = Arc::clone(&registry);
    thread::spawn(move || stop_registry.lock().unwrap().stop(ticket))
        .join()
        .unwrap();
    assert!(!registry.lock().unwrap().try_reap(ticket, true).unwrap());
    assert!(registry.lock().unwrap().reserve().is_none());

    let reap_until = Instant::now() + Duration::from_secs(2);
    loop {
        if registry.lock().unwrap().try_reap(ticket, false).unwrap() {
            break;
        }
        assert!(
            Instant::now() < reap_until,
            "retained native owner never reaped"
        );
        thread::sleep(POLL);
    }
    let next_ticket = registry.lock().unwrap().reserve().expect("slot reclaimed");
    assert_ne!(ticket, next_ticket, "operation identity reused");
    assert!(registry.lock().unwrap().owner.is_none());
    cases += 1;
    println!(
        "OPTIC317_CASE {{\"case\":\"retained-capacity-until-proof\",\"pre_spawn_reservation\":true,\"caller_reference_dropped\":true,\"stop_out_of_band_thread\":true,\"uncertain_capacity_retained\":true,\"tree_empty\":true,\"pipes_eof\":true,\"capacity_reclaimed_after_proof\":true}}"
    );

    // Concurrent reservation by two owner domains: all capacity is occupied
    // before spawn and a third request fails. A stop from a separate thread
    // bypasses the saturated operation capacity, but a mismatched domain is
    // denied. Incomplete proof never releases the affected ticket.
    let registry = Arc::new(Mutex::new(ConcurrentOwnerFixture::new()));
    let barrier = Arc::new(Barrier::new(3));
    let session_registry = Arc::clone(&registry);
    let session_barrier = Arc::clone(&barrier);
    let session_ticket = thread::spawn(move || {
        session_barrier.wait();
        session_registry
            .lock()
            .unwrap()
            .reserve(FixtureAuthority::Session)
            .expect("session ticket")
    });
    let startup_registry = Arc::clone(&registry);
    let startup_barrier = Arc::clone(&barrier);
    let startup_ticket = thread::spawn(move || {
        startup_barrier.wait();
        startup_registry
            .lock()
            .unwrap()
            .reserve(FixtureAuthority::HostStartup)
            .expect("startup ticket")
    });
    barrier.wait();
    let session_ticket = session_ticket.join().unwrap();
    let startup_ticket = startup_ticket.join().unwrap();
    assert_ne!(session_ticket, startup_ticket);
    assert_eq!(registry.lock().unwrap().slots.len(), 2);
    assert!(
        registry
            .lock()
            .unwrap()
            .reserve(FixtureAuthority::Session)
            .is_none()
    );
    for (ticket, authority, name) in [
        (session_ticket, FixtureAuthority::Session, "concurrent-session"),
        (
            startup_ticket,
            FixtureAuthority::HostStartup,
            "concurrent-startup",
        ),
    ] {
        let root = base.join(name);
        std::fs::create_dir(&root).unwrap();
        let mut owner = ProbeOwner::create(&root, "idle").unwrap();
        owner.assign().unwrap();
        owner.resume().unwrap();
        wait_marker(&root, "started");
        registry.lock().unwrap().attach(ticket, authority, owner);
    }
    assert!(
        !registry
            .lock()
            .unwrap()
            .stop(FixtureAuthority::Session, startup_ticket),
        "session must not stop HostStartup"
    );
    assert!(
        !registry
            .lock()
            .unwrap()
            .stop(FixtureAuthority::HostStartup, session_ticket),
        "HostStartup must not stop Session"
    );
    let stop_registry = Arc::clone(&registry);
    assert!(
        thread::spawn(move || stop_registry
            .lock()
            .unwrap()
            .stop(FixtureAuthority::Session, session_ticket))
        .join()
        .unwrap()
    );
    assert!(
        !registry
            .lock()
            .unwrap()
            .try_reap(FixtureAuthority::Session, session_ticket, true)
            .unwrap()
    );
    assert!(
        registry
            .lock()
            .unwrap()
            .reserve(FixtureAuthority::Session)
            .is_none(),
        "uncertain session must hold capacity even if other owner is healthy"
    );
    assert!(
        !registry
            .lock()
            .unwrap()
            .try_reap(FixtureAuthority::Session, startup_ticket, false)
            .unwrap(),
        "session must not reap HostStartup"
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if registry
            .lock()
            .unwrap()
            .try_reap(FixtureAuthority::Session, session_ticket, false)
            .unwrap()
        {
            break;
        }
        assert!(Instant::now() < deadline, "session owner proof deadline");
        thread::sleep(POLL);
    }
    assert_eq!(registry.lock().unwrap().slots.len(), 1);
    let next_ticket = registry
        .lock()
        .unwrap()
        .reserve(FixtureAuthority::Session)
        .expect("capacity available only after native proof");
    assert_ne!(session_ticket, next_ticket);
    assert!(
        !registry
            .lock()
            .unwrap()
            .cancel_unspawned(FixtureAuthority::HostStartup, next_ticket)
    );
    assert!(
        registry
            .lock()
            .unwrap()
            .cancel_unspawned(FixtureAuthority::Session, next_ticket)
    );
    assert!(
        registry
            .lock()
            .unwrap()
            .stop(FixtureAuthority::HostStartup, startup_ticket)
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if registry
            .lock()
            .unwrap()
            .try_reap(FixtureAuthority::HostStartup, startup_ticket, false)
            .unwrap()
        {
            break;
        }
        assert!(Instant::now() < deadline, "startup owner proof deadline");
        thread::sleep(POLL);
    }
    assert!(registry.lock().unwrap().slots.is_empty());
    cases += 1;
    println!(
        "OPTIC317_CASE {{\"case\":\"concurrent-owner-domains\",\"parallel_reservations\":true,\"capacity_bound\":true,\"distinct_owner_domains\":true,\"wrong_owner_stop_denied\":true,\"wrong_owner_reap_denied\":true,\"out_of_band_stop\":true,\"uncertain_capacity_retained\":true,\"native_eof_gated_reap\":true,\"capacity_reused_after_proof\":true,\"all_slots_reclaimed\":true}}"
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
