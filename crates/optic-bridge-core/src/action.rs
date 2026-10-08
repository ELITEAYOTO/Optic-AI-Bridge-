use crate::{
    ActionId, ContentVersion, ProcessExecutionClass, ResourceBudget, SessionHandle, TaskLeaseId,
    ToolApprovalRequirement, ToolProfileName, WorkspacePath,
};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    FileRead,
    FileSearch,
    FileWrite,
    FileDelete,
    GitRead,
    GitIntegrate,
    ProcessRun,
    NetworkAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkAccess {
    Denied,
    Allowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reversibility {
    ReadOnly,
    Transactional,
    Irreversible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedState {
    Absent,
    Content(ContentVersion),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GitObjectId(String);

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GitObjectIdError {
    #[error("Git object id must be a 40- or 64-character hexadecimal value")]
    Invalid,
}

impl GitObjectId {
    pub fn parse(value: impl Into<String>) -> Result<Self, GitObjectIdError> {
        let value = value.into();
        if matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(value.to_ascii_lowercase()))
        } else {
            Err(GitObjectIdError::Invalid)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    FileRead {
        path: WorkspacePath,
    },
    FileSearch {
        root: Option<WorkspacePath>,
    },
    FileWrite {
        path: WorkspacePath,
        expected: ExpectedState,
    },
    FileDelete {
        path: WorkspacePath,
        expected: ContentVersion,
    },
    GitRead,
    GitIntegrationObserve,
    GitIntegrate {
        source_head: GitObjectId,
        expected_target_head: GitObjectId,
    },
    ProcessRun {
        executable: String,
        class: ProcessExecutionClass,
        network: NetworkAccess,
    },
    ProfiledProcessRun {
        profile: ToolProfileName,
        executable: String,
        class: ProcessExecutionClass,
        network: NetworkAccess,
        approval: ToolApprovalRequirement,
    },
    NetworkAccess {
        endpoint: String,
    },
    PolicyChange,
    PrivilegeElevation,
}

impl Effect {
    #[must_use]
    pub const fn required_capability(&self) -> Option<Capability> {
        match self {
            Self::FileRead { .. } => Some(Capability::FileRead),
            Self::FileSearch { .. } => Some(Capability::FileSearch),
            Self::FileWrite { .. } => Some(Capability::FileWrite),
            Self::FileDelete { .. } => Some(Capability::FileDelete),
            Self::GitRead => Some(Capability::GitRead),
            Self::GitIntegrationObserve | Self::GitIntegrate { .. } => {
                Some(Capability::GitIntegrate)
            }
            Self::ProcessRun { .. } | Self::ProfiledProcessRun { .. } => {
                Some(Capability::ProcessRun)
            }
            Self::NetworkAccess { .. } => Some(Capability::NetworkAccess),
            Self::PolicyChange | Self::PrivilegeElevation => None,
        }
    }

    #[must_use]
    pub const fn requires_task_lease(&self) -> bool {
        matches!(
            self,
            Self::FileWrite { .. }
                | Self::FileDelete { .. }
                | Self::GitIntegrationObserve
                | Self::GitIntegrate { .. }
                | Self::ProcessRun { .. }
                | Self::ProfiledProcessRun { .. }
                | Self::NetworkAccess { .. }
        )
    }

    #[must_use]
    pub const fn requires_network_capability(&self) -> bool {
        matches!(
            self,
            Self::ProcessRun {
                network: NetworkAccess::Allowed,
                ..
            } | Self::ProfiledProcessRun {
                network: NetworkAccess::Allowed,
                ..
            } | Self::NetworkAccess { .. }
        )
    }

    #[must_use]
    pub const fn reversibility(&self) -> Reversibility {
        match self {
            Self::FileRead { .. }
            | Self::FileSearch { .. }
            | Self::GitRead
            | Self::GitIntegrationObserve => Reversibility::ReadOnly,
            Self::FileWrite { .. } | Self::FileDelete { .. } | Self::GitIntegrate { .. } => {
                Reversibility::Transactional
            }
            Self::ProcessRun { .. }
            | Self::ProfiledProcessRun { .. }
            | Self::NetworkAccess { .. }
            | Self::PolicyChange
            | Self::PrivilegeElevation => Reversibility::Irreversible,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEnvelope {
    pub action_id: ActionId,
    pub session: SessionHandle,
    pub task_lease: Option<TaskLeaseId>,
    pub effect: Effect,
    pub resources: ResourceBudget,
    pub policy_epoch: u64,
}

impl ActionEnvelope {
    #[must_use]
    pub const fn required_capability(&self) -> Option<Capability> {
        self.effect.required_capability()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_object_ids_are_normalized_and_validated() {
        let sha1 = "ABCDEF0123456789ABCDEF0123456789ABCDEF01";
        let parsed = GitObjectId::parse(sha1).expect("valid SHA-1 object id");
        assert_eq!(parsed.as_str(), sha1.to_ascii_lowercase());
        assert!(GitObjectId::parse("main").is_err());
    }

    #[test]
    fn git_integration_observation_is_read_only_but_uses_integration_authority() {
        let effect = Effect::GitIntegrationObserve;
        assert_eq!(effect.required_capability(), Some(Capability::GitIntegrate));
        assert!(effect.requires_task_lease());
        assert_eq!(effect.reversibility(), Reversibility::ReadOnly);
    }

    #[test]
    fn profiled_process_keeps_process_and_network_capabilities_explicit() {
        let profile = ToolProfileName::parse("node-version").expect("profile");
        let denied = Effect::ProfiledProcessRun {
            profile: profile.clone(),
            executable: "node".to_owned(),
            class: ProcessExecutionClass::Interpreter,
            network: NetworkAccess::Denied,
            approval: ToolApprovalRequirement::HumanRequired,
        };
        assert_eq!(denied.required_capability(), Some(Capability::ProcessRun));
        assert!(denied.requires_task_lease());
        assert!(!denied.requires_network_capability());
        assert_eq!(denied.reversibility(), Reversibility::Irreversible);

        let allowed = Effect::ProfiledProcessRun {
            profile,
            executable: "node".to_owned(),
            class: ProcessExecutionClass::Interpreter,
            network: NetworkAccess::Allowed,
            approval: ToolApprovalRequirement::HumanRequired,
        };
        assert!(allowed.requires_network_capability());
    }

    #[test]
    fn process_network_is_a_second_explicit_capability() {
        let effect = Effect::ProcessRun {
            executable: "cargo".to_owned(),
            class: ProcessExecutionClass::RepositoryCode,
            network: NetworkAccess::Allowed,
        };
        assert_eq!(effect.required_capability(), Some(Capability::ProcessRun));
        assert!(effect.requires_network_capability());
    }
}
