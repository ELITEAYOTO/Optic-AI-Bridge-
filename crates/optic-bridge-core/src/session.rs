use std::collections::BTreeSet;

use crate::{Capability, ResourceBudget, SessionHandle, TaskLeaseId, WorkspacePath};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MonotonicTime(u64);

impl MonotonicTime {
    #[must_use]
    pub const fn from_millis(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn saturating_add_millis(self, value: u64) -> Self {
        Self(self.0.saturating_add(value))
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProcessExecutionClass {
    FixedTool,
    Interpreter,
    RepositoryCode,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeaseScope {
    WorkspaceAll,
    WorkspacePrefix(WorkspacePath),
    Repository,
    ProcessExecutable {
        executable: String,
        class: ProcessExecutionClass,
    },
    /// Server/application-owned marker that one exact classified executable is
    /// eligible for a later strong-isolation policy gate. This marker grants no
    /// process authority by itself and is intentionally distinct from class.
    ProcessIsolationEligible {
        executable: String,
        class: ProcessExecutionClass,
    },
    NetworkAny,
    NetworkEndpoint(String),
}

#[derive(Clone, Debug)]
pub struct SessionGrant {
    pub handle: SessionHandle,
    pub principal: PrincipalId,
    pub project: ProjectId,
    pub capabilities: BTreeSet<Capability>,
    pub expires_at: MonotonicTime,
    pub policy_epoch: u64,
}

impl SessionGrant {
    #[must_use]
    pub const fn is_expired_at(&self, now: MonotonicTime) -> bool {
        now.0 >= self.expires_at.0
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
    pub scopes: BTreeSet<LeaseScope>,
    pub resource_ceiling: ResourceBudget,
    pub expires_at: MonotonicTime,
    pub policy_epoch: u64,
}

impl TaskLease {
    #[must_use]
    pub const fn is_expired_at(&self, now: MonotonicTime) -> bool {
        now.0 >= self.expires_at.0
    }

    #[must_use]
    pub fn allows(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    #[must_use]
    pub fn has_scope(&self, scope: &LeaseScope) -> bool {
        self.scopes.contains(scope)
    }

    #[must_use]
    pub fn process_execution_class(&self, executable: &str) -> Option<ProcessExecutionClass> {
        self.scopes.iter().find_map(|scope| match scope {
            LeaseScope::ProcessExecutable {
                executable: scoped,
                class,
            } if scoped == executable => Some(*class),
            _ => None,
        })
    }

    #[must_use]
    pub fn process_isolation_eligible(
        &self,
        executable: &str,
        class: ProcessExecutionClass,
    ) -> bool {
        self.scopes.iter().any(|scope| {
            matches!(
                scope,
                LeaseScope::ProcessIsolationEligible {
                    executable: scoped,
                    class: scoped_class,
                } if scoped == executable && *scoped_class == class
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonic_deadlines_compare_without_wall_clock_semantics() {
        let issued = MonotonicTime::from_millis(1_000);
        let expires = issued.saturating_add_millis(500);
        assert!(MonotonicTime::from_millis(1_499) < expires);
        assert!(MonotonicTime::from_millis(1_500) >= expires);
    }

    #[test]
    fn process_isolation_eligibility_is_exact_to_executable_and_class() {
        let session = SessionHandle::generate().expect("session");
        let lease = TaskLease {
            id: TaskLeaseId::generate().expect("lease"),
            session,
            capabilities: BTreeSet::from([Capability::ProcessRun]),
            scopes: BTreeSet::from([
                LeaseScope::ProcessExecutable {
                    executable: "C:\\Tools\\node.exe".to_owned(),
                    class: ProcessExecutionClass::Interpreter,
                },
                LeaseScope::ProcessIsolationEligible {
                    executable: "C:\\Tools\\node.exe".to_owned(),
                    class: ProcessExecutionClass::Interpreter,
                },
            ]),
            resource_ceiling: ResourceBudget {
                timeout_ms: 1_000,
                output_bytes: 1_024,
                memory_bytes: 1_024,
                process_count: 1,
            },
            expires_at: MonotonicTime::from_millis(10_000),
            policy_epoch: 1,
        };

        assert!(
            lease.process_isolation_eligible(
                "C:\\Tools\\node.exe",
                ProcessExecutionClass::Interpreter
            )
        );
        assert!(!lease.process_isolation_eligible(
            "C:\\Tools\\python.exe",
            ProcessExecutionClass::Interpreter
        ));
        assert!(!lease.process_isolation_eligible(
            "C:\\Tools\\node.exe",
            ProcessExecutionClass::RepositoryCode
        ));
    }
}
