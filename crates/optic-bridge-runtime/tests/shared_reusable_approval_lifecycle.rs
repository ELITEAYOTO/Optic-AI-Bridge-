use std::{env, fs, sync::Arc};

use optic_bridge_core::{HardLimits, SessionHandle};
use optic_bridge_runtime::{
    ApprovalBroker, ProcessManager, ReusableApprovalBroker, SessionLifecycleManager,
    SessionRegistry, TaskLeaseRegistry,
};

fn workspace() -> std::path::PathBuf {
    let token = SessionHandle::generate().expect("test entropy").to_token();
    let root = env::temp_dir().join(format!("optic-shared-reusable-lifecycle-{token}"));
    fs::create_dir_all(&root).expect("create workspace");
    root
}

#[test]
fn lifecycle_single_broker_constructor_reuses_associated_reusable_broker() {
    let root = workspace();
    let limits = HardLimits::default();
    let sessions = Arc::new(SessionRegistry::from_hard_limits(limits).expect("sessions"));
    let leases = Arc::new(TaskLeaseRegistry::from_hard_limits(limits).expect("leases"));
    let processes = Arc::new(ProcessManager::new(&root, limits, Vec::new()).expect("processes"));
    let reusable = Arc::new(ReusableApprovalBroker::new());
    let approvals = Arc::new(
        ApprovalBroker::from_hard_limits_with_reusable(limits, Arc::clone(&reusable))
            .expect("approvals"),
    );

    let lifecycle = SessionLifecycleManager::new_with_approval_broker(
        sessions,
        leases,
        processes,
        Arc::clone(&approvals),
    );
    let service = lifecycle.reusable_approval_service();

    assert!(Arc::ptr_eq(&reusable, service.broker()));
    assert!(Arc::ptr_eq(
        &approvals.reusable_approvals(),
        service.broker()
    ));

    fs::remove_dir_all(root).expect("remove workspace");
}
