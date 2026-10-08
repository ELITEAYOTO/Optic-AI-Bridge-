#![forbid(unsafe_code)]

//! Security-oriented domain primitives for Optic AI Bridge.
//!
//! This crate deliberately contains no MCP transport, filesystem I/O, process
//! spawning, Git execution, or Windows API calls. It defines the values that
//! those outer layers must satisfy before an effect can be authorized.

mod action;
mod approval;
mod ids;
mod limits;
mod path;
mod read_scope;
mod session;
mod tool_profile;
mod version;

pub use action::{
    ActionEnvelope, Capability, Effect, ExpectedState, GitObjectId, GitObjectIdError,
    NetworkAccess, Reversibility,
};
pub use approval::ApprovalGrant;
pub use ids::{ActionId, ApprovalId, IdError, JobId, SessionHandle, TaskLeaseId, TokenParseError};
pub use limits::{HardLimits, LimitError, ResourceBudget};
pub use path::{WorkspacePath, WorkspacePathError};
pub use read_scope::{
    ReadContentScope, ReadSensitiveScope, SearchMetadataScope, WorkspaceAuthorityScope,
};
pub use session::{
    LeaseScope, MonotonicTime, PrincipalId, ProcessExecutionClass, ProjectId, SessionGrant,
    TaskLease, WorkloadClass,
};
pub use tool_profile::{
    ToolApprovalRequirement, ToolInvocation, ToolProfile, ToolProfileError, ToolProfileName,
    ToolProfileSpec,
};
pub use version::{ContentVersion, ContentVersionParseError, ContentVersionReadError};
