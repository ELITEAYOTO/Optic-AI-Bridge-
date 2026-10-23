//! Finite fixed schedules. Every native host is externally Job-owned first.
use super::*;

fn close(
    direct: bool,
    registry: &SessionRegistry,
    lifecycle: &SessionLifecycleManager,
    session: &SessionHandle,
) -> (bool, Option<usize>) {
    if direct {
        (registry.revoke(session).unwrap(), None)
    } else {
        let report = lifecycle.revoke(session).unwrap();
        (report.session_changed, Some(report.cancellation_requests))
    }
}
fn wait_closed(registry: &SessionRegistry, handle: &SessionHandle, clock: &dyn Clock) {
    wait(|| {
        matches!(
            registry.get_active(handle, clock.now()),
            Err(SessionRegistryError::Revoked)
        )
    });
}
fn provision(
    lifecycle: &SessionLifecycleManager,
    clock: &dyn Clock,
    name: &str,
) -> optic_bridge_core::SessionGrant {
    lifecycle
        .provision(
            SessionGrantSpec {
                principal: PrincipalId::new(name).unwrap(),
                project: ProjectId::new(name).unwrap(),
                capabilities: BTreeSet::from([Capability::GitRead]),
                expires_at: MonotonicTime::from_millis(10_000),
                policy_epoch: 1,
            },
            clock.now(),
        )
        .unwrap()
}
fn independent_native(
    path: &Path,
    registry: &SessionRegistry,
    manager: &ProcessManager,
    session: &SessionHandle,
    clock: &dyn Clock,
) {
    let dir = path.join("independent-native");
    fs::create_dir(&dir).unwrap();
    let (mut b, client) = Root::new(registry, manager, session, clock);
    b.prepare(|| (), true).unwrap();
    b.commit().unwrap();
    b.start(&dir, "auto-stop", || ()).unwrap();
    wait(|| dir.join("executed").exists());
    drop(client); // control: stop while B authority remains OPEN
    wait(|| b.observe().unwrap());
    assert!(registry.get_active(session, clock.now()).is_ok());
    assert_eq!(manager.active_session_job_count(session).unwrap(), 0);
}
pub(super) fn finish(
    mut root: Root<'_>,
    lifecycle: &SessionLifecycleManager,
    path: &Path,
    mode: &str,
    legacy_rescue: bool,
    extra: serde_json::Value,
) -> serde_json::Value {
    assert!(root.observe().unwrap());
    let k = root.slot.as_ref().unwrap();
    assert!(k.terminal && k.admission.is_none() && k.ticket.is_none());
    let associated = k.native.association_observed;
    let result = root.manager.result(&root.session, &k.id).unwrap();
    let out = k.record.output.lock().unwrap();
    let bytes = out.stdout.len() + out.stderr.len();
    assert!(out.stdout_closed && out.stderr_closed && bytes <= native::OUTPUT);
    drop(out);
    assert_eq!(
        root.manager
            .active_session_job_count(&root.session)
            .unwrap(),
        0
    );
    assert!(lifecycle.seal_quiescent(&root.session).is_ok());
    let error = root.last_error.or_else(|| {
        let code = k.watch.first_error.load(Ordering::Acquire);
        (code != 0).then_some(code as i32)
    });
    let mut proof = serde_json::json!({"mode":mode,"production_wired":false,"real_registry":true,"real_admission":true,
        "completion_kind":"native-or-no-spawn-proof","real_win32":true,"native_created":associated,"atomic_job_association":associated,"native_create_attempted":k.native.creation_attempted(),
        "native_terminal_and_both_eof":true,"stop_worker_joined":true,"active_ownership_released_after_proof":true,
        "legacy_manual_cancel_rescue":legacy_rescue,"status":format!("{:?}",result.status),"combined_output_bytes":bytes,
        "native_error_code":error,"observation_errors":root.errors.load(Ordering::Acquire),"stop_errors":k.watch.errors.load(Ordering::Acquire)});
    if let Some(fields) = extra.as_object() {
        for (k, v) in fields {
            proof[k] = v.clone();
        }
    }
    if matches!(
        mode,
        "close-after-registration"
            | "late-register-after-scan"
            | "close-before-resume"
            | "before-commit-error"
            | "uncommitted-start"
    ) {
        assert!(!path.join("executed").exists());
    }
    drop(root);
    proof
}

pub(super) fn exercise(path: &Path, mode: &str) -> serde_json::Value {
    let registry = Arc::new(SessionRegistry::new());
    let manager = Arc::new(ProcessManager::new(path, HardLimits::default(), Vec::new()).unwrap());
    let lifecycle = SessionLifecycleManager::new(
        Arc::clone(&registry),
        Arc::new(TaskLeaseRegistry::new()),
        Arc::clone(&manager),
    );
    let clock = StdClock::new();
    let grant = provision(&lifecycle, &clock, "native-research");
    let independent = provision(&lifecycle, &clock, "independent");
    let (mut root, client) = Root::new(&registry, &manager, &grant.handle, &clock);
    let mut client = Some(client);
    if mode == "closed-before-admission" {
        assert!(lifecycle.revoke(&grant.handle).unwrap().session_changed);
        assert!(
            root.prepare(|| panic!("closed admission cannot reach preparation"), true)
                .is_err()
        );
        assert!(root.slot.is_none() && root.request_shutdown());
        assert_eq!(manager.active_session_job_count(&grant.handle).unwrap(), 0);
        return serde_json::json!({"mode":mode,"completion_kind":"no-admission-no-native","production_wired":false,
            "real_registry":true,"real_admission":true,"admission_created":false,"real_win32":false,"native_created":false,"native_create_attempted":false,
            "atomic_job_association":false,"native_terminal_and_both_eof":false,"stop_worker_joined":false,
            "active_ownership_released_after_proof":true,"legacy_manual_cancel_rescue":false,"status":"Denied","combined_output_bytes":0});
    }
    if mode == "late-register-after-scan" {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (tx, rx) = mpsc::sync_channel(1);
        thread::scope(|scope| {
            let registry_ref = &registry;
            let lifecycle_ref = &lifecycle;
            let session = &grant.handle;
            let closer = scope.spawn(move || {
                ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                tx.send(close(false, registry_ref, lifecycle_ref, session))
                    .unwrap();
            });
            root.prepare(
                || {
                    ready_tx.send(()).unwrap();
                    wait_closed(&registry, &grant.handle, &clock);
                    assert_eq!(manager.cancel_session(&grant.handle).unwrap(), 0);
                    independent_native(path, &registry, &manager, &independent.handle, &clock);
                },
                true,
            )
            .unwrap();
            let k = root.slot.as_ref().unwrap();
            assert!(
                k.registered
                    && k.record.stop_requested.load(Ordering::Acquire)
                    && k.authority.load(Ordering::Acquire)
            );
            assert!(matches!(root.commit(), Err(SessionRegistryError::Revoked)));
            root.request_shutdown();
            wait(|| root.observe().unwrap());
            let (changed, count) = rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(changed);
            assert_eq!(count, Some(0));
            closer.join().unwrap();
        });
        return finish(
            root,
            &lifecycle,
            path,
            mode,
            false,
            serde_json::json!({"early_scan_count":0,"late_registration_inherited_stop":true,"independent_native_completed":true}),
        );
    }
    root.prepare(|| (), mode != "legacy-no-latch").unwrap();
    if mode == "uncommitted-start" {
        assert!(root.start(path, mode, || ()).is_err());
        assert!(root.slot.as_ref().unwrap().ticket.is_none());
        assert_eq!(root.commit().unwrap_err(), SessionRegistryError::Revoked);
        root.request_shutdown();
        wait(|| root.observe().unwrap());
        return finish(
            root,
            &lifecycle,
            path,
            mode,
            false,
            serde_json::json!({"uncommitted_start_denied":true}),
        );
    }
    if mode == "close-after-registration" {
        let (tx, rx) = mpsc::sync_channel(1);
        thread::scope(|scope| {
            let registry_ref = &registry;
            let lifecycle_ref = &lifecycle;
            let session = &grant.handle;
            let closer = scope.spawn(move || {
                tx.send(close(true, registry_ref, lifecycle_ref, session))
                    .unwrap()
            });
            wait_closed(&registry, &grant.handle, &clock);
            assert_eq!(root.commit().unwrap_err(), SessionRegistryError::Revoked);
            assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
            root.request_shutdown();
            wait(|| root.observe().unwrap());
            assert!(rx.recv_timeout(Duration::from_secs(2)).unwrap().0);
            closer.join().unwrap();
        });
        return finish(
            root,
            &lifecycle,
            path,
            mode,
            false,
            serde_json::json!({"close_before_engagement":true}),
        );
    }
    if mode == "before-commit-error" {
        let error = native::Native::rejected_exit_code_probe().unwrap_err();
        assert_eq!(error.raw_os_error(), Some(6));
        root.last_error = error.raw_os_error();
        root.slot.as_mut().unwrap().failed = true;
        root.request_shutdown();
        assert!(root.commit().is_err());
        wait(|| root.observe().unwrap());
        return finish(
            root,
            &lifecycle,
            path,
            mode,
            false,
            serde_json::json!({"native_error_before_engagement":true}),
        );
    }
    root.commit().unwrap();
    if mode == "close-before-resume" {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (tx, rx) = mpsc::sync_channel(1);
        thread::scope(|scope| {
            let registry_ref = &registry;
            let lifecycle_ref = &lifecycle;
            let session = &grant.handle;
            let closer = scope.spawn(move || {
                ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                tx.send(close(true, registry_ref, lifecycle_ref, session))
                    .unwrap();
            });
            let result = root.start(path, mode, || {
                ready_tx.send(()).unwrap();
                wait_closed(&registry, &grant.handle, &clock);
                independent_native(path, &registry, &manager, &independent.handle, &clock);
                fs::write(
                    path.join("suspended-ready"),
                    b"closed with native phase paused",
                )
                .unwrap();
                wait(|| path.join("suspended-confirmed").exists());
                assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
            });
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
            wait(|| root.observe().unwrap());
            assert!(rx.recv_timeout(Duration::from_secs(2)).unwrap().0);
            closer.join().unwrap();
        });
        return finish(
            root,
            &lifecycle,
            path,
            mode,
            false,
            serde_json::json!({"close_published_during_native_pause":true,"resume_denied":true,"independent_native_completed":true}),
        );
    }
    let result = root.start(path, mode, || ());
    if matches!(mode, "create-error" | "resume-error") {
        assert!(result.is_err());
        root.request_shutdown();
    } else {
        result.unwrap();
        wait(|| path.join("executed").exists());
    }
    if mode == "client-drop" {
        drop(client.take());
        assert!(registry.get_active(&grant.handle, clock.now()).is_ok());
        wait(|| root.observe().unwrap());
        assert!(registry.get_active(&grant.handle, clock.now()).is_ok());
        return finish(
            root,
            &lifecycle,
            path,
            mode,
            false,
            serde_json::json!({"client_drop_stopped_while_authority_open":true}),
        );
    }
    let (tx, rx) = mpsc::sync_channel(1);
    let mut legacy_rescue = false;
    if mode == "stop-error" {
        root.fail_stop.store(true, Ordering::Release);
    }
    thread::scope(|scope| {
        let registry_ref = &registry;
        let lifecycle_ref = &lifecycle;
        let session = &grant.handle;
        let closer = scope.spawn(move || {
            tx.send(close(
                mode == "direct-revoke",
                registry_ref,
                lifecycle_ref,
                session,
            ))
            .unwrap()
        });
        wait_closed(&registry, &grant.handle, &clock);
        if mode == "legacy-no-latch" {
            assert!(
                !root
                    .slot
                    .as_ref()
                    .unwrap()
                    .record
                    .stop_requested
                    .load(Ordering::Acquire)
            );
            fs::write(path.join("live-ready"), b"no authority stop binding").unwrap();
            wait(|| path.join("live-confirmed").exists());
            assert_eq!(manager.cancel_session(&grant.handle).unwrap(), 1);
            legacy_rescue = true;
        }
        if mode == "observation-uncertain" {
            root.fail_query.store(true, Ordering::Release);
            assert_eq!(root.observe().unwrap_err().raw_os_error(), Some(6));
            assert!(!root.closing && !root.stop.load(Ordering::Acquire));
            assert!(
                root.prepare(|| panic!("full root must not prepare"), true)
                    .is_err()
            );
            let k = root.slot.as_ref().unwrap();
            assert!(k.admission.is_some() && k.ticket.is_some());
            assert!(!k.record.stop_requested.load(Ordering::Acquire));
            assert_eq!(manager.active_session_job_count(&grant.handle).unwrap(), 1);
            fs::write(
                path.join("uncertain-ready"),
                b"authority stopped child; proof withheld",
            )
            .unwrap();
            wait(|| path.join("uncertain-confirmed").exists());
            for _ in 0..3 {
                assert!(!root.request_shutdown());
                assert!(root.observe().is_err());
            }
            assert_eq!(root.errors.load(Ordering::Acquire), 4);
            assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
            root.fail_query.store(false, Ordering::Release);
        }
        if mode == "stop-error" {
            wait(|| {
                root.slot
                    .as_ref()
                    .unwrap()
                    .watch
                    .errors
                    .load(Ordering::Acquire)
                    >= 4
            });
            assert!(!root.observe().unwrap());
            assert_eq!(
                root.slot
                    .as_ref()
                    .unwrap()
                    .record
                    .state
                    .lock()
                    .unwrap()
                    .status,
                ProcessStatus::TerminationUncertain
            );
            assert!(!root.request_shutdown());
            fs::write(
                path.join("live-ready"),
                b"termination rejection; real handles retained",
            )
            .unwrap();
            wait(|| path.join("live-confirmed").exists());
            assert_eq!(manager.active_session_job_count(&grant.handle).unwrap(), 1);
            root.fail_stop.store(false, Ordering::Release);
        }
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        wait(|| root.observe().unwrap());
        let (changed, count) = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(changed);
        if mode != "direct-revoke" {
            assert_eq!(count, Some(0));
        }
        closer.join().unwrap();
    });
    drop(client.take());
    finish(
        root,
        &lifecycle,
        path,
        mode,
        legacy_rescue,
        serde_json::json!({}),
    )
}
