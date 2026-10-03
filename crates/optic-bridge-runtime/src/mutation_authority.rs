use std::collections::BTreeSet;

use optic_bridge_core::{
    Capability, HardLimits, IdError, LeaseScope, LimitError, MonotonicTime, ResourceBudget,
    SessionHandle, TaskLease, TaskLeaseId,
};
use thiserror::Error;

use crate::{TaskLeaseRegistry, TaskLeaseRegistryError};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MutationAuthoritySpec {
    pub write_scopes: BTreeSet<LeaseScope>,
    pub delete_scopes: BTreeSet<LeaseScope>,
}

impl MutationAuthoritySpec {
    pub fn validate(&self) -> Result<(), MutationAuthorityError> {
        validate_workspace_scopes(&self.write_scopes)?;
        validate_workspace_scopes(&self.delete_scopes)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MutationAuthoritySet {
    write_lease: Option<TaskLeaseId>,
    delete_lease: Option<TaskLeaseId>,
}

impl MutationAuthoritySet {
    pub fn provision(
        registry: &TaskLeaseRegistry,
        session: &SessionHandle,
        spec: &MutationAuthoritySpec,
        resource_ceiling: ResourceBudget,
        expires_at: MonotonicTime,
        policy_epoch: u64,
    ) -> Result<Self, MutationAuthorityError> {
        spec.validate()?;
        resource_ceiling.validate_nonzero()?;

        let write_lease = provision_capability(
            registry,
            session,
            Capability::FileWrite,
            &spec.write_scopes,
            resource_ceiling,
            expires_at,
            policy_epoch,
        )?;

        let delete_lease = match provision_capability(
            registry,
            session,
            Capability::FileDelete,
            &spec.delete_scopes,
            resource_ceiling,
            expires_at,
            policy_epoch,
        ) {
            Ok(value) => value,
            Err(error) => {
                if let Some(id) = &write_lease {
                    let _ = registry.revoke(id);
                }
                return Err(error);
            }
        };

        Ok(Self {
            write_lease,
            delete_lease,
        })
    }

    #[must_use]
    pub fn lease_for(&self, capability: Capability) -> Option<&TaskLeaseId> {
        match capability {
            Capability::FileWrite => self.write_lease.as_ref(),
            Capability::FileDelete => self.delete_lease.as_ref(),
            _ => None,
        }
    }

    #[must_use]
    pub const fn has_write(&self) -> bool {
        self.write_lease.is_some()
    }

    #[must_use]
    pub const fn has_delete(&self) -> bool {
        self.delete_lease.is_some()
    }
}

#[must_use]
pub const fn mutation_resource_budget(limits: HardLimits) -> ResourceBudget {
    ResourceBudget {
        timeout_ms: limits.max_request_duration_ms,
        output_bytes: limits.max_response_bytes,
        memory_bytes: limits.max_active_output_ram_bytes,
        process_count: 1,
    }
}

fn provision_capability(
    registry: &TaskLeaseRegistry,
    session: &SessionHandle,
    capability: Capability,
    scopes: &BTreeSet<LeaseScope>,
    resource_ceiling: ResourceBudget,
    expires_at: MonotonicTime,
    policy_epoch: u64,
) -> Result<Option<TaskLeaseId>, MutationAuthorityError> {
    if scopes.is_empty() {
        return Ok(None);
    }

    let id = TaskLeaseId::generate()?;
    registry.register(TaskLease {
        id: id.clone(),
        session: session.clone(),
        capabilities: BTreeSet::from([capability]),
        scopes: scopes.clone(),
        resource_ceiling,
        expires_at,
        policy_epoch,
    })?;
    Ok(Some(id))
}

fn validate_workspace_scopes(scopes: &BTreeSet<LeaseScope>) -> Result<(), MutationAuthorityError> {
    for scope in scopes {
        if !matches!(
            scope,
            LeaseScope::WorkspaceAll | LeaseScope::WorkspacePrefix(_)
        ) {
            return Err(MutationAuthorityError::NonWorkspaceScope);
        }
    }
    if scopes.contains(&LeaseScope::WorkspaceAll) && scopes.len() != 1 {
        return Err(MutationAuthorityError::RedundantWorkspaceAll);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum MutationAuthorityError {
    #[error("mutation authority scopes must be workspace scopes")]
    NonWorkspaceScope,
    #[error("WorkspaceAll cannot be combined with narrower workspace prefixes")]
    RedundantWorkspaceAll,
    #[error("mutation authority resource ceiling is invalid: {0}")]
    InvalidResourceCeiling(#[from] LimitError),
    #[error("operating-system entropy source is unavailable")]
    Entropy(#[from] IdError),
    #[error("task lease registry operation failed: {0}")]
    Registry(#[from] TaskLeaseRegistryError),
}

#[cfg(test)]
mod tests {
    use optic_bridge_core::{MonotonicTime, WorkspacePath};

    use super::*;

    fn budget() -> ResourceBudget {
        ResourceBudget {
            timeout_ms: 30_000,
            output_bytes: 256 * 1024,
            memory_bytes: 16 * 1024 * 1024,
            process_count: 1,
        }
    }

    fn prefix(value: &str) -> LeaseScope {
        LeaseScope::WorkspacePrefix(WorkspacePath::parse(value).expect("safe prefix"))
    }

    #[test]
    fn no_configuration_mints_no_mutation_authority() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session entropy");
        let authority = MutationAuthoritySet::provision(
            &registry,
            &session,
            &MutationAuthoritySpec::default(),
            budget(),
            MonotonicTime::from_millis(100),
            7,
        )
        .expect("empty authority");

        assert!(!authority.has_write());
        assert!(!authority.has_delete());
    }

    #[test]
    fn write_and_delete_are_minted_as_distinct_exact_capabilities() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session entropy");
        let spec = MutationAuthoritySpec {
            write_scopes: BTreeSet::from([prefix("src")]),
            delete_scopes: BTreeSet::from([prefix("generated")]),
        };
        let expires_at = MonotonicTime::from_millis(100);
        let authority =
            MutationAuthoritySet::provision(&registry, &session, &spec, budget(), expires_at, 7)
                .expect("authority");

        let write_id = authority
            .lease_for(Capability::FileWrite)
            .expect("write authority");
        let write = registry
            .get_active(write_id, &session, MonotonicTime::from_millis(1))
            .expect("write lease");
        assert_eq!(write.capabilities, BTreeSet::from([Capability::FileWrite]));
        assert_eq!(write.scopes, spec.write_scopes);
        assert_eq!(write.policy_epoch, 7);

        let delete_id = authority
            .lease_for(Capability::FileDelete)
            .expect("delete authority");
        let delete = registry
            .get_active(delete_id, &session, MonotonicTime::from_millis(1))
            .expect("delete lease");
        assert_eq!(
            delete.capabilities,
            BTreeSet::from([Capability::FileDelete])
        );
        assert_eq!(delete.scopes, spec.delete_scopes);
        assert_ne!(write.id, delete.id);
    }

    #[test]
    fn non_workspace_and_redundantly_broad_scopes_fail_closed() {
        let non_workspace = MutationAuthoritySpec {
            write_scopes: BTreeSet::from([LeaseScope::Repository]),
            delete_scopes: BTreeSet::new(),
        };
        assert_eq!(
            non_workspace
                .validate()
                .expect_err("must reject repository scope"),
            MutationAuthorityError::NonWorkspaceScope
        );

        let redundant = MutationAuthoritySpec {
            write_scopes: BTreeSet::from([LeaseScope::WorkspaceAll, prefix("src")]),
            delete_scopes: BTreeSet::new(),
        };
        assert_eq!(
            redundant
                .validate()
                .expect_err("must reject redundant broad scope"),
            MutationAuthorityError::RedundantWorkspaceAll
        );
    }

    #[test]
    fn provisioned_authority_obeys_registry_revocation_and_expiry() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session entropy");
        let spec = MutationAuthoritySpec {
            write_scopes: BTreeSet::from([LeaseScope::WorkspaceAll]),
            delete_scopes: BTreeSet::new(),
        };
        let authority = MutationAuthoritySet::provision(
            &registry,
            &session,
            &spec,
            budget(),
            MonotonicTime::from_millis(10),
            3,
        )
        .expect("authority");
        let id = authority
            .lease_for(Capability::FileWrite)
            .expect("write authority");

        assert_eq!(
            registry
                .get_active(id, &session, MonotonicTime::from_millis(10))
                .expect_err("expired lease must fail"),
            TaskLeaseRegistryError::Expired
        );
        assert!(registry.revoke(id).expect("revoke"));
        assert_eq!(
            registry
                .get_active(id, &session, MonotonicTime::from_millis(1))
                .expect_err("revoked lease must fail"),
            TaskLeaseRegistryError::Revoked
        );
    }

    #[test]
    fn canonical_mutation_budget_tracks_hard_limits() {
        let limits = HardLimits::default();
        let budget = mutation_resource_budget(limits);
        assert_eq!(budget.timeout_ms, limits.max_request_duration_ms);
        assert_eq!(budget.output_bytes, limits.max_response_bytes);
        assert_eq!(budget.memory_bytes, limits.max_active_output_ram_bytes);
        assert_eq!(budget.process_count, 1);
    }
}
