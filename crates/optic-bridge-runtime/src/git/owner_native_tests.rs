//! Requires independent outer Job/accounting supervisor. No production wiring.
use super::*;
use optic_bridge_core::{Capability, PrincipalId, ProjectId, SessionGrant};
use optic_bridge_windows::{LimitedJobObject, PreparedReadCommand};
use std::{
    collections::BTreeSet,
    env, fs,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "independent supervisor marker missing"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "requires independent Job accounting while observer holds payload"]
fn prepared_owner_stops_while_native_observer_is_blocked() {
    let gate =
        std::path::PathBuf::from(env::var_os("OPTIC317_REGISTRY_GATE").expect("supervisor gate"));
    wait_for(&gate);
    let root = gate.parent().unwrap();
    let started = root.join("started");
    let sessions = SessionRegistry::new();
    let grant = SessionGrant {
        handle: SessionHandle::generate().unwrap(),
        principal: PrincipalId::new("native-prepared").unwrap(),
        project: ProjectId::new("synthetic").unwrap(),
        capabilities: BTreeSet::from([Capability::FileRead]),
        expires_at: MonotonicTime::from_millis(10000),
        policy_epoch: 1,
    };
    sessions.register(grant.clone()).unwrap();
    thread::scope(|scope| {
        let admission = sessions
            .begin_admission(&grant.handle, MonotonicTime::from_millis(1))
            .unwrap();
        let budget = OwnerBudget {
            memory: 512 * 1024 * 1024,
            output: 16384,
            cpu: 25,
        };
        let owners = Arc::new(
            GitOwnerRegistry::new(OwnerCeiling {
                count: 1,
                memory: budget.memory,
                output: budget.output,
                cpu: budget.cpu,
            })
            .unwrap(),
        );
        let domain = OwnerDomain::Session(grant.handle.clone());
        let id = owners
            .reserve_session(&sessions, &admission, MonotonicTime::from_millis(1), budget)
            .unwrap();
        let stop = owners.stop_signal(id, &domain).unwrap();
        let (revoked, _) = sessions
            .watch_active(&grant.handle, MonotonicTime::from_millis(1))
            .unwrap();
        // Real Job, CPU/memory/process cap, pipes and watchdog BEFORE creation.
        let mut native = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(1, budget.memory, budget.cpu).unwrap(),
                budget.output as usize,
                8000,
                vec![stop, Arc::clone(&revoked)],
            )
            .unwrap(),
        );
        owners.adopt(id, &domain, &mut native).unwrap();
        assert!(native.is_none());
        let mut command = Command::new(env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "git_owner_registry::tests::native_registry_fixture_child",
                "--ignored",
                "--nocapture",
            ])
            .env_clear()
            .env("OPTIC317_REGISTRY_CHILD", "1")
            .env("OPTIC317_REGISTRY_STARTED", &started);
        if let Some(value) = env::var_os("SystemRoot") {
            command.env("SystemRoot", value);
        }
        // Starting does not release capacity, even if it returns an error.
        owners
            .with_owner(id, &domain, |native| native.start(&mut command))
            .unwrap()
            .unwrap();
        wait_for(&started);
        fs::write(root.join("ready"), b"native root executed").unwrap();
        wait_for(&root.join("outer-ready"));
        let observing = Arc::clone(&owners);
        let observed_domain = domain.clone();
        let observer = scope.spawn(move || {
            observing
                .with_owner(id, &observed_domain, |native| {
                    fs::write(root.join("observer-held"), b"payload lock held")?;
                    // The independent Python supervisor must confirm that only
                    // this host process remains in the outer Job before release.
                    // No fake native-termination bool or early lock release.
                    wait_for(&root.join("outer-confirmed"));
                    let deadline = Instant::now() + Duration::from_secs(2);
                    while !native.poll_terminal()? {
                        assert!(Instant::now() < deadline, "native tree/EOF proof absent");
                        thread::sleep(Duration::from_millis(5));
                    }
                    let outcome = native.take_outcome()?;
                    assert!(outcome.status.is_some());
                    assert!(
                        outcome.cancelled,
                        "real revoke must not look like successful completion"
                    );
                    assert!(
                        !outcome.deadline_expired,
                        "8s deadline must not explain early stop"
                    );
                    Ok::<(), std::io::Error>(())
                })
                .unwrap()
                .unwrap();
        });
        wait_for(&root.join("observer-held"));
        let revoker = scope.spawn(|| {
            sessions.revoke(&grant.handle).unwrap();
        });
        let until = Instant::now() + Duration::from_secs(2);
        while !revoked.load(Ordering::Acquire) {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(5));
        }
        fs::write(root.join("revoked"), b"real pre-drain stop flag").unwrap();
        assert_eq!(owners.outstanding(&grant.handle).unwrap(), 1);
        assert!(matches!(
            owners.with_owner(id, &domain, |_| Ok::<_, ()>(())),
            Err(OwnerError::ObservationBusy)
        ));
        observer.join().unwrap();
        assert_eq!(owners.outstanding(&grant.handle).unwrap(), 1);
        assert!(
            !revoker.is_finished(),
            "real admission must still prevent drain"
        );
        assert!(
            owners
                .reap_if_proven(id, &domain, |native| native.poll_terminal())
                .unwrap()
                .unwrap()
        );
        assert_eq!(owners.outstanding(&grant.handle).unwrap(), 0);
        drop(admission);
        revoker.join().unwrap();
        println!(
            "OPTIC317_REGISTRY_NATIVE pre_spawn_native_owner=true real_revoke=true stop_while_payload_locked=true tree_empty=true pipes_eof=true capacity_released_after_proof=true"
        );
    });
}

#[test]
#[ignore = "requires independent Windows Job supervisor and synthetic Python fixture"]
fn prepared_owner_native_matrix() {
    let gate =
        std::path::PathBuf::from(env::var_os("OPTIC317_REGISTRY_GATE").expect("supervisor gate"));
    wait_for(&gate);
    let python = env::var_os("OPTIC317_PYTHON").expect("synthetic Python executable");
    let fixture = env::var_os("OPTIC317_FIXTURE").expect("synthetic fixture");
    let budget = OwnerBudget {
        memory: 512 * 1024 * 1024,
        output: 16384,
        cpu: 25,
    };
    let owners = GitOwnerRegistry::new(OwnerCeiling {
        count: 1,
        memory: budget.memory,
        output: budget.output,
        cpu: budget.cpu,
    })
    .unwrap();
    let domain = OwnerDomain::HostStartup;
    for mode in [
        "normal",
        "nonzero",
        "output",
        "both-output",
        "stdout",
        "stderr",
        "timeout",
        "null-stdin",
        "missing-exe",
        "pre-stop",
    ] {
        let id = owners.reserve_host_startup(budget).unwrap();
        let stop = owners.stop_signal(id, &domain).unwrap();
        if mode == "pre-stop" {
            stop.store(true, Ordering::Release);
        }
        let timeout = if mode == "timeout" { 200 } else { 1000 };
        let mut native = Some(
            PreparedReadCommand::prepare(
                LimitedJobObject::new(4, budget.memory, budget.cpu).unwrap(),
                budget.output as usize,
                timeout,
                vec![stop],
            )
            .unwrap(),
        );
        owners.adopt(id, &domain, &mut native).unwrap();
        let mut command = if mode == "missing-exe" {
            Command::new(gate.with_extension("nonexistent.exe"))
        } else {
            Command::new(&python)
        };
        command.arg(&fixture).arg(mode).env_clear();
        if let Some(value) = env::var_os("SystemRoot") {
            command.env("SystemRoot", value);
        }
        let start = Instant::now();
        let result = owners
            .with_owner(id, &domain, |owner| owner.start(&mut command))
            .unwrap();
        assert_eq!(
            result.is_err(),
            matches!(mode, "missing-exe" | "pre-stop"),
            "{mode}"
        );
        // A start error itself must not evict or release its real owner.
        let second = owners.reserve_host_startup(budget);
        assert!(matches!(second, Err(OwnerError::Capacity)));
        let outcome = owners
            .with_owner(id, &domain, |owner| {
                assert!(owner.start(&mut command).is_err(), "start must be one-shot");
                let until = Instant::now() + Duration::from_secs(2);
                while !owner.poll_terminal()? {
                    assert!(Instant::now() < until, "{mode}: cleanup unconfirmed");
                    thread::sleep(Duration::from_millis(5));
                }
                // Terminal proof is latched: retrieving the result after its
                // deadline must not relabel an already completed command.
                if mode == "normal" {
                    thread::sleep(Duration::from_millis(1100));
                }
                let outcome = owner.take_outcome()?;
                assert!(owner.take_outcome().is_err(), "outcome is one-shot");
                Ok::<_, std::io::Error>(outcome)
            })
            .unwrap()
            .unwrap();
        assert!(outcome.stdout.len() + outcome.stderr.len() <= budget.output as usize);
        assert_eq!(
            outcome.output_limit_exceeded,
            matches!(mode, "output" | "both-output")
        );
        assert_eq!(outcome.deadline_expired, mode == "timeout", "{mode}");
        assert_eq!(outcome.cancelled, mode == "pre-stop", "{mode}");
        assert_eq!(
            outcome.is_success(),
            matches!(mode, "normal" | "stdout" | "stderr" | "null-stdin"),
            "{mode}"
        );
        match mode {
            "missing-exe" | "pre-stop" => assert!(outcome.status.is_none()),
            "nonzero" => assert_eq!(outcome.status.unwrap().code(), Some(7)),
            "output" | "both-output" | "timeout" => {}
            _ => assert!(outcome.status.unwrap().success(), "{mode}"),
        }
        assert!(
            owners
                .reap_if_proven(id, &domain, |owner| owner.poll_terminal())
                .unwrap()
                .unwrap()
        );
        println!(
            "OPTIC317_PREPARED_CASE {}",
            serde_json::json!({"case":mode,"elapsed_ms":start.elapsed().as_millis(),"tree_empty_and_pipes_eof":true,"output_bytes":outcome.stdout.len()+outcome.stderr.len(),"deadline_expired":outcome.deadline_expired,"output_exceeded":outcome.output_limit_exceeded})
        );
    }
    println!("OPTIC317_REGISTRY_NATIVE prepared_native_cases=10 all_slots_reclaimed=true");
}
