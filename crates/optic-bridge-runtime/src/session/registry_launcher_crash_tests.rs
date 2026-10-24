//! Fixed cross-PROCESS crash schedules, only under pre-armed outer supervision.
use super::{grant, reserve};
use crate::{ProcessStatus, SessionRegistry};
use optic_bridge_core::MonotonicTime;
use optic_bridge_windows::{ResearchCrashLauncher, ResearchKernelJob, ResearchSuspendedChild};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};

fn wait(mut predicate: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(4);
    while !predicate() {
        assert!(Instant::now() < until, "fixed crash fixture deadline");
        thread::sleep(Duration::from_millis(2));
    }
}
fn write(root: &Path, name: &str, value: serde_json::Value) {
    let tmp = root.join(format!("{name}.tmp"));
    fs::write(&tmp, value.to_string()).unwrap();
    fs::rename(tmp, root.join(name)).unwrap();
}
fn mode(root: &Path) -> String {
    let s = fs::read_to_string(root.join("mode")).unwrap();
    assert!(matches!(
        s.as_str(),
        "before-create"
            | "after-create"
            | "offered"
            | "job-only"
            | "accepted-before-ack"
            | "preowned-before-create"
            | "preowned-after-create"
            | "preowned-offer-failed"
            | "preowned-create-failed"
    ));
    s
}

#[test]
#[ignore = "fixed launcher; only the independently Job-owned crash host may invoke"]
fn launcher_host() {
    let root = PathBuf::from(env::var_os("OPTIC317_TRANSFER_ROOT").unwrap());
    wait(|| root.join("launch-gate").exists());
    let mode = mode(&root);
    if mode == "preowned-offer-failed" {
        assert!(
            !root.join("preowned-job").exists(),
            "rejected DuplicateHandle must not issue a mailbox offer"
        );
        write(
            &root,
            "launcher-phase",
            serde_json::json!({"created":false,"preowned":false,"launcher_terminate_denied":false,"native_duplication_failed":true}),
        );
        thread::sleep(Duration::from_secs(12));
        panic!("fixed launcher survived independent invalid-offer boundary");
    }
    if mode.starts_with("preowned-") {
        // This is a DUPLICATE of the service's original Job, already created
        // and limited before the launcher began any native child creation.
        wait(|| root.join("preowned-job").exists());
        let bytes = fs::read(root.join("preowned-job")).unwrap();
        assert!(bytes.len() < 128);
        let offered: u64 = serde_json::from_slice(&bytes).unwrap();
        let job = ResearchKernelJob::adopt_preowned_job_offer(offered).unwrap();
        assert_eq!(job.active().unwrap(), 0);
        // Negative proof before any child exists: the delegated handle can
        // query its Job, but cannot terminate it on the guardian's behalf.
        job.prove_terminate_denied().unwrap();
        if mode == "preowned-create-failed" {
            ResearchSuspendedChild::prove_missing_cwd_creation_rejected(&job, &root)
                .expect("fixed libtest CreateProcessW rejects nonexistent cwd");
            assert_eq!(job.active().unwrap(), 0);
            write(
                &root,
                "launcher-phase",
                serde_json::json!({"created":false,"native_active":0,"preowned":true,"launcher_terminate_denied":true,"native_duplication_failed":false,"native_creation_failed":true}),
            );
            thread::sleep(Duration::from_secs(12));
            panic!("fixed native-create-failure launcher was not terminated externally");
        }
        if matches!(
            mode.as_str(),
            "preowned-before-create" | "preowned-offer-failed" | "preowned-create-failed"
        ) {
            write(
                &root,
                "launcher-phase",
                serde_json::json!({"created":false,"native_active":0,"preowned":true,"launcher_terminate_denied":true,"native_duplication_failed":false}),
            );
            thread::sleep(Duration::from_secs(12));
            panic!("fixed delegated launcher was not terminated externally");
        }
        let child = ResearchSuspendedChild::create(&job, &root).unwrap();
        assert_eq!(job.active().unwrap(), 1);
        write(
            &root,
            "launcher-phase",
            serde_json::json!({"created":true,"native_active":1,"preowned":true,"launcher_terminate_denied":true,"native_duplication_failed":false}),
        );
        assert!(!root.join("ack").exists());
        // The service retains its OWN kernel Job handle even when this
        // separate launcher dies without returning any child handle/ACK.
        drop(child);
        thread::sleep(Duration::from_secs(12));
        panic!("fixed delegated launcher survived independent crash boundary");
    }
    let job = ResearchKernelJob::prepare().unwrap();
    if mode == "before-create" {
        write(
            &root,
            "launcher-phase",
            serde_json::json!({"created":false,"native_active":job.active().unwrap()}),
        );
        thread::sleep(Duration::from_secs(12));
        panic!("fixed launcher was not terminated externally");
    }
    let child = ResearchSuspendedChild::create(&job, &root).unwrap();
    assert_eq!(job.active().unwrap(), 1);
    if mode != "after-create" {
        write(
            &root,
            "offer",
            serde_json::json!(job.transfer_offer(&child)),
        );
    }
    write(
        &root,
        "launcher-phase",
        serde_json::json!({"created":true,"native_active":1}),
    );
    // No ack is ever issued in this experiment. Originals stay live until the
    // exact launcher process is killed; a send/offer alone is NOT acceptance.
    assert!(!root.join("ack").exists());
    thread::sleep(Duration::from_secs(12));
    panic!("fixed launcher survived the independent crash boundary");
}

#[test]
#[ignore = "requires verified external Job before gate; fixed native children only"]
fn service_host() {
    let root = PathBuf::from(env::var_os("OPTIC317_TRANSFER_ROOT").unwrap());
    wait(|| root.join("outer-gate").exists());
    let mode = mode(&root);
    let registry = Arc::new(SessionRegistry::new());
    let a = grant();
    let b = grant();
    registry.register(a.clone()).unwrap();
    registry.register(b.clone()).unwrap();
    let now = MonotonicTime::from_millis(1);
    let owned = registry.begin_owned_admission(&a.handle, now).unwrap();
    let (reserved_root, manager, reservation, action) = reserve(&a.handle);
    let id = reservation.id();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (uncertain_tx, uncertain_rx) = mpsc::sync_channel(1);
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    thread::scope(|scope| {
        let path = &root;
        let scenario = &mode;
        let guardian=scope.spawn(move || {
            // The genuine counted admission and registered reservation are
            // owned HERE, never kept alive separately by the test controller.
            let mut reservation=reservation;
            let receipt=owned.research_commit_registered_owner(&reservation,now).unwrap();drop(receipt);
            let stop=owned.stop_watch();reservation.mark_native_created();
            let preowned = scenario.starts_with("preowned-");
            // The service is responsible for this Job *before* the separate
            // creator can receive its duplicated handle or create a child.
            let mut job = if preowned {
                Some(ResearchKernelJob::prepare().unwrap())
            } else {
                None
            };
            let launcher=ResearchCrashLauncher::create(path).unwrap();
            if let Some(native) = &job {
                if scenario == "preowned-offer-failed" {
                    launcher
                        .prove_invalid_job_destination_rejected(native)
                        .expect("real Win32 invalid destination must reject duplication");
                    assert_eq!(native.active().unwrap(), 0);
                    assert!(
                        !path.join("preowned-job").exists(),
                        "failed delegation must not publish a remote handle"
                    );
                } else {
                    let offer = launcher.offer_preowned_job(native).unwrap();
                    write(path, "preowned-job", serde_json::json!(offer));
                }
            }
            fs::write(path.join("launch-gate"),b"real-owner-prearmed").unwrap();
            wait(||path.join("launcher-phase").exists());
            if preowned {
                let phase: serde_json::Value = serde_json::from_slice(
                    &fs::read(path.join("launcher-phase")).unwrap()
                ).unwrap();
                assert_eq!(
                    phase["launcher_terminate_denied"],
                    scenario != "preowned-offer-failed"
                );
                assert_eq!(
                    phase["native_duplication_failed"],
                    scenario == "preowned-offer-failed"
                );
                assert_eq!(
                    phase["native_creation_failed"].as_bool().unwrap_or(false),
                    scenario == "preowned-create-failed"
                );
            }
            let mut child=None;
            if matches!(scenario.as_str(),"job-only"|"accepted-before-ack") {
                let bytes=fs::read(path.join("offer")).unwrap();assert!(bytes.len()<128);
                let values:[u64;2]=serde_json::from_slice(&bytes).unwrap();
                let accepted_job=launcher.accept_job(values[0]).unwrap();assert_eq!(accepted_job.active().unwrap(),1);
                if scenario=="accepted-before-ack" {child=Some(launcher.accept_child(values[1],&accepted_job).unwrap());}
                job=Some(accepted_job);
            }
            assert!(!path.join("ack").exists());
            write(path,"crash-ready",serde_json::json!({"expected_outer":if matches!(scenario.as_str(), "before-create" | "preowned-before-create" | "preowned-offer-failed" | "preowned-create-failed" | "preowned-create-failed") {2}else{3},"job_duplicate":!preowned && job.is_some(),"job_preowned":preowned,"launcher_terminate_denied":preowned && scenario != "preowned-offer-failed","native_duplication_failed":scenario == "preowned-offer-failed","native_creation_failed":scenario == "preowned-create-failed","child_duplicate":child.is_some(),"ack_sent":false}));
            wait(||path.join("crash-confirmed").exists());
            launcher.terminate_and_wait().unwrap();
            write(path,"source-exit",serde_json::json!({"exact_owned_process_exit":true}));
            wait(||path.join("survivor-confirmed").exists());
            if preowned {
                assert_eq!(
                    job.as_ref().unwrap().active().unwrap(),
                    u32::from(scenario == "preowned-after-create"),
                    "service retains the real Job after separate launcher death"
                );
            }
            // FIRST RED: old successful-handoff shape unwinds/loses the guardian
            // on missing native receipt. Its prearmed quota remains uncertain,
            // but it has discarded the true admission and responsible worker.
            if job.is_none() && env::var_os("OPTIC317_RED").is_some() { return; }
            ready_tx.send(()).unwrap();
            wait(||stop.is_stop_required_at(now));
            if let Some(native)=&job {
                assert!(native.observe_active_with_injected_failure(true).is_err());
                uncertain_tx.send(()).unwrap();
                wait(||path.join("reconcile").exists());
                if !matches!(scenario.as_str(), "preowned-before-create" | "preowned-create-failed") {
                    native.terminate().unwrap();
                }
                if let Some(child)=&child {child.await_exit().unwrap();}
                native.await_empty().unwrap();assert_eq!(native.active().unwrap(),0);
                reservation.finish(
                    if matches!(scenario.as_str(), "preowned-before-create" | "preowned-offer-failed" | "preowned-create-failed") {
                        ProcessStatus::Failed
                    } else {
                        ProcessStatus::Stopped
                    },
                    None,
                );
                drop(reservation);drop(child);drop(job);drop(owned);
                done_tx.send(()).unwrap();
            } else {
                uncertain_tx.send(()).unwrap();
                // Identified host-owned keeper stays alive. Only the independent
                // supervisor ends this synthetic service host; no detach/timeout
                // publication or pretend Job-empty receipt is permitted.
                loop {thread::sleep(Duration::from_millis(10));}
            }
        });
        assert!(
            ready_rx.recv_timeout(Duration::from_secs(4)).is_ok(),
            "guardian vanished before retaining uncertain owner after launcher loss"
        );
        assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
        assert_eq!(
            manager.result(&a.handle, &id).unwrap().status,
            ProcessStatus::Running
        );
        assert!(
            registry
                .research_request_close_without_drain(&a.handle)
                .unwrap()
        );
        let (drained_tx, drained_rx) = mpsc::sync_channel(1);
        let reg = &registry;
        let session = &a.handle;
        let closer = scope.spawn(move || {
            drained_tx.send(reg.revoke(session)).unwrap();
        });
        uncertain_rx.recv_timeout(Duration::from_secs(4)).unwrap();
        assert!(!guardian.is_finished());
        assert!(matches!(
            drained_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 1);
        assert_eq!(
            manager.result(&a.handle, &id).unwrap().status,
            ProcessStatus::Running
        );
        assert_eq!(
            manager
                .find_recorded_private_action(&a.handle, &action)
                .unwrap(),
            Some(id.clone())
        );
        assert!(registry.remove_quiescent_inactive(&a.handle, now).is_err());
        assert!(registry.begin_owned_admission(&b.handle, now).is_ok());
        let pending = matches!(mode.as_str(), "before-create" | "after-create" | "offered");
        let base = serde_json::json!({"mode":mode,"production_wired":false,"launcher_exit_proven":true,"ack_sent":false,
            "owner_live_before_reconciliation":true,"original_admission_retained":true,"quota_retained_before_proof":1,
            "status_before_proof":"Running","revoke_pending_before_proof":true,"other_session_usable":true,
            "native_job_receipt":!pending,"job_preowned":mode.starts_with("preowned-"),"launcher_terminate_denied":mode.starts_with("preowned-") && mode != "preowned-offer-failed","native_duplication_failed":mode=="preowned-offer-failed","native_creation_failed":mode=="preowned-create-failed","root_handle_receipt":mode=="accepted-before-ack"});
        if pending {
            let mut proof = base;
            proof["outcome"] = serde_json::json!("PendingNoNativeReceipt");
            proof["termination_proven_by_guardian"] = serde_json::json!(false);
            proof["terminal_published"] = serde_json::json!(false);
            proof["quota_released"] = serde_json::json!(false);
            write(&root, "pending-ready", proof);
            thread::sleep(Duration::from_secs(12));
            panic!("external recovery did not end pending fixed host");
        }
        fs::write(root.join("reconcile"), b"real-native-proof-required").unwrap();
        done_rx.recv_timeout(Duration::from_secs(4)).unwrap();
        guardian.join().unwrap();
        assert!(
            !drained_rx
                .recv_timeout(Duration::from_secs(4))
                .unwrap()
                .unwrap()
        );
        closer.join().unwrap();
        assert_eq!(manager.active_session_job_count(&a.handle).unwrap(), 0);
        assert_eq!(
            manager.result(&a.handle, &id).unwrap().status,
            if matches!(
                mode.as_str(),
                "preowned-before-create" | "preowned-offer-failed" | "preowned-create-failed"
            ) {
                ProcessStatus::Failed
            } else {
                ProcessStatus::Stopped
            }
        );
        let mut proof = base;
        proof["outcome"] = serde_json::json!("GuardianJobEmptyProven");
        proof["terminal_status"] = serde_json::json!(if matches!(
            mode.as_str(),
            "preowned-before-create" | "preowned-offer-failed" | "preowned-create-failed"
        ) {
            "Failed"
        } else {
            "Stopped"
        });
        proof["termination_proven_by_guardian"] = serde_json::json!(true);
        proof["terminal_published"] = serde_json::json!(true);
        proof["quota_released"] = serde_json::json!(true);
        proof["guardian_joined"] = serde_json::json!(true);
        write(&root, "complete-ready", proof);
    });
    drop(manager);
    // This directory was newly created by this fixture only; no old data touched.
    fs::remove_dir_all(reserved_root).unwrap();
    thread::sleep(Duration::from_secs(12));
    panic!("outer supervisor did not finish exact synthetic service host");
}
