use std::collections::BTreeSet;

use optic_bridge_core::{
    Capability, HardLimits, IdError, LeaseScope, LimitError, MonotonicTime, ReadContentScope,
    ReadSensitiveScope, ResourceBudget, SearchMetadataScope, SessionHandle, TaskLease, TaskLeaseId,
    WorkloadClass,
};
use thiserror::Error;

use crate::{TaskLeaseRegistry, TaskLeaseRegistryError};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadAuthoritySpec {
    pub search_metadata: BTreeSet<SearchMetadataScope>,
    pub read_content: BTreeSet<ReadContentScope>,
    pub read_sensitive: BTreeSet<ReadSensitiveScope>,
}

impl ReadAuthoritySpec {
    #[must_use]
    pub fn workspace_all_non_sensitive() -> Self {
        Self {
            search_metadata: BTreeSet::from([SearchMetadataScope::all()]),
            read_content: BTreeSet::from([ReadContentScope::all()]),
            read_sensitive: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadAuthoritySet {
    search_lease: Option<TaskLeaseId>,
    read_lease: Option<TaskLeaseId>,
}

impl ReadAuthoritySet {
    pub fn provision(
        registry: &TaskLeaseRegistry,
        session: &SessionHandle,
        spec: &ReadAuthoritySpec,
        resource_ceiling: ResourceBudget,
        expires_at: MonotonicTime,
        policy_epoch: u64,
    ) -> Result<Self, ReadAuthorityError> {
        resource_ceiling.validate_nonzero()?;

        let search_scopes = spec
            .search_metadata
            .iter()
            .cloned()
            .map(LeaseScope::SearchMetadata)
            .collect::<BTreeSet<_>>();
        let search_lease = provision_capability(
            registry,
            session,
            Capability::FileSearch,
            search_scopes,
            resource_ceiling,
            expires_at,
            policy_epoch,
        )?;

        let mut read_scopes = spec
            .read_content
            .iter()
            .cloned()
            .map(LeaseScope::ReadContent)
            .collect::<BTreeSet<_>>();
        read_scopes.extend(
            spec.read_sensitive
                .iter()
                .cloned()
                .map(LeaseScope::ReadSensitive),
        );
        let read_lease = match provision_capability(
            registry,
            session,
            Capability::FileRead,
            read_scopes,
            resource_ceiling,
            expires_at,
            policy_epoch,
        ) {
            Ok(value) => value,
            Err(error) => {
                if let Some(id) = &search_lease {
                    registry.revoke(id)?;
                    registry.remove_revoked(id)?;
                }
                return Err(error);
            }
        };

        Ok(Self {
            search_lease,
            read_lease,
        })
    }

    #[must_use]
    pub fn lease_for(&self, capability: Capability) -> Option<&TaskLeaseId> {
        match capability {
            Capability::FileSearch => self.search_lease.as_ref(),
            Capability::FileRead => self.read_lease.as_ref(),
            _ => None,
        }
    }

    #[must_use]
    pub const fn has_search(&self) -> bool {
        self.search_lease.is_some()
    }

    #[must_use]
    pub const fn has_read(&self) -> bool {
        self.read_lease.is_some()
    }
}

#[must_use]
pub const fn read_authority_resource_budget(limits: HardLimits) -> ResourceBudget {
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
    scopes: BTreeSet<LeaseScope>,
    resource_ceiling: ResourceBudget,
    expires_at: MonotonicTime,
    policy_epoch: u64,
) -> Result<Option<TaskLeaseId>, ReadAuthorityError> {
    if scopes.is_empty() {
        return Ok(None);
    }

    let id = TaskLeaseId::generate()?;
    registry.register(TaskLease {
        id: id.clone(),
        session: session.clone(),
        capabilities: BTreeSet::from([capability]),
        scopes,
        resource_ceiling,
        workload_class: WorkloadClass::Standard,
        expires_at,
        policy_epoch,
    })?;
    Ok(Some(id))
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ReadAuthorityError {
    #[error("read/search authority resource ceiling is invalid: {0}")]
    InvalidResourceCeiling(#[from] LimitError),
    #[error("operating-system entropy source is unavailable")]
    Entropy(#[from] IdError),
    #[error("task lease registry operation failed: {0}")]
    Registry(#[from] TaskLeaseRegistryError),
}

#[cfg(test)]
mod tests {
    use optic_bridge_core::{WorkspaceAuthorityScope, WorkspacePath};

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
    fn empty_spec_mints_no_read_or_search_authority() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session");
        let authority = ReadAuthoritySet::provision(
            &registry,
            &session,
            &ReadAuthoritySpec::default(),
            budget(),
            MonotonicTime::from_millis(100),
            1,
        )
        .expect("empty authority");
        assert!(!authority.has_search());
        assert!(!authority.has_read());
    }

    #[test]
    fn workspace_all_non_sensitive_mints_distinct_exact_capabilities() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session");
        let spec = ReadAuthoritySpec::workspace_all_non_sensitive();
        let authority = ReadAuthoritySet::provision(
            &registry,
            &session,
            &spec,
            budget(),
            MonotonicTime::from_millis(100),
            7,
        )
        .expect("authority");

        let search = registry
            .get_active(
                authority
                    .lease_for(Capability::FileSearch)
                    .expect("search lease"),
                &session,
                MonotonicTime::from_millis(1),
            )
            .expect("active search lease");
        assert_eq!(
            search.capabilities,
            BTreeSet::from([Capability::FileSearch])
        );
        assert_eq!(search.scopes.len(), 1);
        assert!(search.covers_search_metadata(None));
        assert!(!search.covers_read_content(&WorkspacePath::parse("file.txt").expect("path")));

        let read = registry
            .get_active(
                authority
                    .lease_for(Capability::FileRead)
                    .expect("read lease"),
                &session,
                MonotonicTime::from_millis(1),
            )
            .expect("active read lease");
        assert_eq!(read.capabilities, BTreeSet::from([Capability::FileRead]));
        assert!(read.covers_read_content(&WorkspacePath::parse("file.txt").expect("path")));
        assert!(!read.covers_read_sensitive(&WorkspacePath::parse("file.txt").expect("path")));
        assert_ne!(search.id, read.id);
    }

    #[test]
    fn sensitive_authority_is_explicit_and_does_not_replace_content_authority() {
        let registry = TaskLeaseRegistry::new();
        let session = SessionHandle::generate().expect("session");
        let spec = ReadAuthoritySpec {
            search_metadata: BTreeSet::new(),
            read_content: BTreeSet::new(),
            read_sensitive: BTreeSet::from([ReadSensitiveScope::prefix(
                WorkspacePath::parse("private").expect("path"),
            )]),
        };
        let authority = ReadAuthoritySet::provision(
            &registry,
            &session,
            &spec,
            budget(),
            MonotonicTime::from_millis(100),
            1,
        )
        .expect("authority");
        let read = registry
            .get_active(
                authority
                    .lease_for(Capability::FileRead)
                    .expect("read lease"),
                &session,
                MonotonicTime::from_millis(1),
            )
            .expect("read lease");
        let path = WorkspacePath::parse("private/token.txt").expect("path");
        assert!(read.covers_read_sensitive(&path));
        assert!(!read.covers_read_content(&path));
    }

    #[test]
    fn partial_provision_failure_reclaims_search_lease_capacity() {
        let limits = HardLimits {
            max_task_leases: 1,
            max_task_leases_per_session: 1,
            ..HardLimits::default()
        };
        let registry = TaskLeaseRegistry::from_hard_limits(limits).expect("registry");
        let session = SessionHandle::generate().expect("session");
        assert!(
            ReadAuthoritySet::provision(
                &registry,
                &session,
                &ReadAuthoritySpec::workspace_all_non_sensitive(),
                budget(),
                MonotonicTime::from_millis(100),
                1,
            )
            .is_err()
        );

        let search_only = ReadAuthoritySpec {
            search_metadata: BTreeSet::from([SearchMetadataScope::all()]),
            ..ReadAuthoritySpec::default()
        };
        ReadAuthoritySet::provision(
            &registry,
            &session,
            &search_only,
            budget(),
            MonotonicTime::from_millis(100),
            1,
        )
        .expect("failed batch must release capacity");
    }

    #[test]
    fn canonical_budget_tracks_hard_limits() {
        let limits = HardLimits::default();
        let budget = read_authority_resource_budget(limits);
        assert_eq!(budget.timeout_ms, limits.max_request_duration_ms);
        assert_eq!(budget.output_bytes, limits.max_response_bytes);
        assert_eq!(budget.memory_bytes, limits.max_active_output_ram_bytes);
        assert_eq!(budget.process_count, 1);
    }

    #[test]
    fn scope_shape_remains_workspace_local() {
        let spec = ReadAuthoritySpec::workspace_all_non_sensitive();
        assert_eq!(
            spec.search_metadata
                .iter()
                .next()
                .expect("scope")
                .workspace_scope(),
            &WorkspaceAuthorityScope::All
        );
    }
}
