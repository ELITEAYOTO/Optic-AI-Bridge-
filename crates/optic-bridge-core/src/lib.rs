#![forbid(unsafe_code)]

//! Security-oriented domain primitives for Optic AI Bridge.
//!
//! This crate deliberately contains no MCP transport, filesystem I/O, process
//! spawning, Git execution, or Windows API calls. It defines the values that
//! those outer layers must satisfy before an effect can be authorized.

mod action;
mod ids;
mod limits;
mod path;
mod session;
mod version;

pub use action::{
    ActionEnvelope, ActionKind, Capability, NetworkAccess, Reversibility, Target,
};
pub use ids::{ActionId, IdError, SessionHandle, TaskLeaseId, TokenParseError};
pub use limits::{HardLimits, LimitError, ResourceBudget};
pub use path::{WorkspacePath, WorkspacePathError};
pub use session::{PrincipalId, ProjectId, SessionGrant, TaskLease};
pub use version::ContentVersion;
