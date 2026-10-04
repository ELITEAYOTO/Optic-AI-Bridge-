use std::collections::BTreeSet;

use optic_bridge_core::{
    Capability, HardLimits, IdError, LeaseScope, LimitError, MonotonicTime, ResourceBudget,
    SessionHandle, TaskLease, TaskLeaseId,
};
use thiserror::Error;

use crate::{TaskLeaseRegistry, TaskLeaseRegistryError};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GitIntegrationAuthoritySpec {
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitIntegrationAuthoritySet {
    integrate_lease: Option<TaskLeaseId>,
}

impl GitIntegrationAuthoritySet {
    pub fn provision(
        registry: &TaskLeaseRegistry,
        session: &SessionHandle,
        spec: GitIntegrationAuthoritySpec,
        resource_ceiling: ResourceBudget,
        expires_at: MonotonicTime,
        policy_epoch: u64,
    ) -> Result<Self, GitIntegrationAuthorityError> {
        resource_ceiling.validate_nonzero()?;
        if !spec.enabled {
            return Ok(Self::default());
        }

        let id = TaskLeaseId::generate()?;
        registry.register(TaskLease {
            id: id.clone(),
            session: session.clone(),
            capabilities: BTreeSet::from([Capability::GitIntegrate]),
            scopes: BTreeSet::from([LeaseScope::Repository]),
            resource_ceiling,
            expires_at,
            policy_epoch,
        })?;

        Ok(Self {
            integrate_lease: Some(id),
        })
    }

    #[must_use]
    pub fn lease(&self) -> Option<&TaskLeaseId> {
        self.integrate_lease.as_ref()
    }

    #[must_use]
    pub const fn has_integrate(&self) -> bool {
        self.integrate_lease.is_some()
    }
}

#[must_use]
pub const fn git_integration_resource_budget(limits: HardLimits) -> ResourceBudget {
    ResourceBudget {
        timeout_ms: limits.max_request_duration_ms,
        output_bytes: limits.max_response_bytes,
        memory_bytes: limits.max_active_output_ram_bytes,
        process_count: 1,
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GitIntegrationAuthorityError {
    #[error("Git integration authority resource ceiling is invalid: {0}")]
    InvalidResourceCeiling(#[from] LimitError),
    #[error("operating-system entropy source is unavailable")]
    Entropy(#[from] IdError),
    #[error("task lease registry operation failed: {0}")]
    Registry(#[from] TaskLeaseRegistryError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> ResourceBudget {
        ResourceBudget {
            timeout_ms: 30_000,
            output_bytes: 256 * 1024,
            memory_bytes: 16 * 1024 * 1024,
            process_count: 1,
        }
    }

    #[test]
    fn disabled_configuration_mints_no_authority() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session entropy");
        let authority = GitIntegrationAuthoritySet::provision(
            &registry,
            &session,
            GitIntegrationAuthoritySpec::default(),
            budget(),
            MonotonicTime::from_millis(100),
            7,
        )
        .expect("disabled authority");

        assert!(!authority.has_integrate());
        assert!(authority.lease().is_none());
    }

    #[test]
    fn enabled_authority_is_exact_repository_scoped_git_integrate() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session entropy");
        let authority = GitIntegrationAuthoritySet::provision(
            &registry,
            &session,
            GitIntegrationAuthoritySpec { enabled: true },
            budget(),
            MonotonicTime::from_millis(100),
            9,
        )
        .expect("authority");
        let id = authority.lease().expect("integration lease");
        let lease = registry
            .get_active(id, &session, MonotonicTime::from_millis(1))
            .expect("active lease");

        assert_eq!(
            lease.capabilities,
            BTreeSet::from([Capability::GitIntegrate])
        );
        assert_eq!(lease.scopes, BTreeSet::from([LeaseScope::Repository]));
        assert_eq!(lease.policy_epoch, 9);
        assert_eq!(lease.resource_ceiling, budget());
    }

    #[test]
    fn provisioned_authority_obeys_registry_revocation_and_expiry() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session entropy");
        let authority = GitIntegrationAuthoritySet::provision(
            &registry,
            &session,
            GitIntegrationAuthoritySpec { enabled: true },
            budget(),
            MonotonicTime::from_millis(10),
            3,
        )
        .expect("authority");
        let id = authority.lease().expect("integration lease");

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
    fn canonical_budget_tracks_hard_limits() {
        let limits = HardLimits::default();
        let budget = git_integration_resource_budget(limits);
        assert_eq!(budget.timeout_ms, limits.max_request_duration_ms);
        assert_eq!(budget.output_bytes, limits.max_response_bytes);
        assert_eq!(budget.memory_bytes, limits.max_active_output_ram_bytes);
        assert_eq!(budget.process_count, 1);
    }
}
