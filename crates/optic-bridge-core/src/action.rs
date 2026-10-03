use crate::{
    ActionId, ContentVersion, ResourceBudget, SessionHandle, TaskLeaseId, WorkspacePath,
};

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
pub enum ActionKind {
    FileRead,
    FileSearch,
    FileWrite,
    FileDelete,
    GitRead,
    GitIntegrate,
    ProcessRun,
    NetworkAccess,
    PolicyChange,
    PrivilegeElevation,
}

impl ActionKind {
    #[must_use]
    pub const fn required_capability(self) -> Option<Capability> {
        match self {
            Self::FileRead => Some(Capability::FileRead),
            Self::FileSearch => Some(Capability::FileSearch),
            Self::FileWrite => Some(Capability::FileWrite),
            Self::FileDelete => Some(Capability::FileDelete),
            Self::GitRead => Some(Capability::GitRead),
            Self::GitIntegrate => Some(Capability::GitIntegrate),
            Self::ProcessRun => Some(Capability::ProcessRun),
            Self::NetworkAccess => Some(Capability::NetworkAccess),
            Self::PolicyChange | Self::PrivilegeElevation => None,
        }
    }

    #[must_use]
    pub const fn requires_task_lease(self) -> bool {
        matches!(
            self,
            Self::FileWrite
                | Self::FileDelete
                | Self::GitIntegrate
                | Self::ProcessRun
                | Self::NetworkAccess
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    WorkspacePath(WorkspacePath),
    GitRepository,
    ProcessExecutable(String),
    NetworkEndpoint(String),
    SecurityPolicy,
    PrivilegeBoundary,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEnvelope {
    pub action_id: ActionId,
    pub session: SessionHandle,
    pub task_lease: Option<TaskLeaseId>,
    pub kind: ActionKind,
    pub target: Target,
    pub expected_content: Option<ContentVersion>,
    pub resources: ResourceBudget,
    pub network: NetworkAccess,
    pub reversibility: Reversibility,
    pub policy_epoch: u64,
}

impl ActionEnvelope {
    #[must_use]
    pub fn required_capability(&self) -> Option<Capability> {
        self.kind.required_capability()
    }
}
