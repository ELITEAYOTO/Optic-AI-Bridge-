use std::{
    collections::BTreeSet,
    time::SystemTime,
};

use crate::{Capability, ResourceBudget, SessionHandle, TaskLeaseId};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrincipalId(String);

impl PrincipalId {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (!value.is_empty() && value.len() <= 256).then_some(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectId(String);

impl ProjectId {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (!value.is_empty() && value.len() <= 256).then_some(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct SessionGrant {
    pub handle: SessionHandle,
    pub principal: PrincipalId,
    pub project: ProjectId,
    pub capabilities: BTreeSet<Capability>,
    pub expires_at: SystemTime,
    pub policy_epoch: u64,
}

impl SessionGrant {
    #[must_use]
    pub fn is_expired_at(&self, now: SystemTime) -> bool {
        now >= self.expires_at
    }

    #[must_use]
    pub fn allows(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}

#[derive(Clone, Debug)]
pub struct TaskLease {
    pub id: TaskLeaseId,
    pub session: SessionHandle,
    pub capabilities: BTreeSet<Capability>,
    pub resource_ceiling: ResourceBudget,
    pub expires_at: SystemTime,
    pub policy_epoch: u64,
}

impl TaskLease {
    #[must_use]
    pub fn is_expired_at(&self, now: SystemTime) -> bool {
        now >= self.expires_at
    }

    #[must_use]
    pub fn allows(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}
