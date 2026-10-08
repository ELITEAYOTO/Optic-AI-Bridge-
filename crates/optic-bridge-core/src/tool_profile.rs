use std::collections::BTreeSet;

use thiserror::Error;

use crate::{
    LimitError, NetworkAccess, ProcessExecutionClass, ResourceBudget, WorkloadClass, WorkspacePath,
};

const MAX_PROFILE_NAME_BYTES: usize = 128;
const MAX_PROFILE_ARGS: usize = 128;
const MAX_PROFILE_READ_FILES: usize = 32;
const MAX_PROFILE_ENV_VARS: usize = 64;
const MAX_PROFILE_SHAPE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolProfileName(String);

impl ToolProfileName {
    pub fn parse(value: impl Into<String>) -> Result<Self, ToolProfileError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_PROFILE_NAME_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(ToolProfileError::InvalidName);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolApprovalRequirement {
    NotRequired,
    HumanRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolProfileSpec {
    pub name: ToolProfileName,
    pub executable: String,
    pub class: ProcessExecutionClass,
    pub workload_class: WorkloadClass,
    pub exact_args: Vec<String>,
    pub cwd: Option<WorkspacePath>,
    pub workspace_read_files: BTreeSet<WorkspacePath>,
    pub env_allowlist: BTreeSet<String>,
    pub network: NetworkAccess,
    pub resource_ceiling: ResourceBudget,
    pub approval: ToolApprovalRequirement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolProfile {
    spec: ToolProfileSpec,
}

/// Fully normalized process invocation compared against one immutable profile.
/// Outer adapters must canonicalize executable identity and normalize environment
/// names before constructing this value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolInvocation {
    pub executable: String,
    pub class: ProcessExecutionClass,
    pub workload_class: WorkloadClass,
    pub args: Vec<String>,
    pub cwd: Option<WorkspacePath>,
    pub workspace_read_files: BTreeSet<WorkspacePath>,
    pub env_allowlist: BTreeSet<String>,
    pub network: NetworkAccess,
    pub resources: ResourceBudget,
}

impl ToolProfile {
    pub fn from_spec(spec: ToolProfileSpec) -> Result<Self, ToolProfileError> {
        validate_spec(&spec)?;
        Ok(Self { spec })
    }

    #[must_use]
    pub fn name(&self) -> &ToolProfileName {
        &self.spec.name
    }

    #[must_use]
    pub const fn approval_requirement(&self) -> ToolApprovalRequirement {
        self.spec.approval
    }

    #[must_use]
    pub fn matches_invocation(&self, invocation: &ToolInvocation) -> bool {
        self.spec.executable == invocation.executable
            && self.spec.class == invocation.class
            && self.spec.workload_class == invocation.workload_class
            && self.spec.exact_args == invocation.args
            && self.spec.cwd == invocation.cwd
            && self.spec.workspace_read_files == invocation.workspace_read_files
            && self.spec.env_allowlist == invocation.env_allowlist
            && self.spec.network == invocation.network
            && invocation.resources.fits_within(self.spec.resource_ceiling)
    }

    #[must_use]
    pub fn spec(&self) -> &ToolProfileSpec {
        &self.spec
    }
}

fn validate_spec(spec: &ToolProfileSpec) -> Result<(), ToolProfileError> {
    if spec.executable.is_empty() {
        return Err(ToolProfileError::EmptyExecutable);
    }
    if spec.exact_args.len() > MAX_PROFILE_ARGS {
        return Err(ToolProfileError::TooManyArguments);
    }
    if spec.workspace_read_files.len() > MAX_PROFILE_READ_FILES {
        return Err(ToolProfileError::TooManyWorkspaceReadFiles);
    }
    if spec.env_allowlist.len() > MAX_PROFILE_ENV_VARS {
        return Err(ToolProfileError::TooManyEnvironmentVariables);
    }
    if spec.env_allowlist.iter().any(|name| name.is_empty()) {
        return Err(ToolProfileError::InvalidEnvironmentName);
    }
    spec.resource_ceiling.validate_nonzero()?;

    let mut bytes = spec
        .name
        .as_str()
        .len()
        .saturating_add(spec.executable.len());
    for arg in &spec.exact_args {
        bytes = bytes.saturating_add(arg.len());
    }
    if let Some(cwd) = &spec.cwd {
        bytes = bytes.saturating_add(cwd.as_str().len());
    }
    for path in &spec.workspace_read_files {
        bytes = bytes.saturating_add(path.as_str().len());
    }
    for name in &spec.env_allowlist {
        bytes = bytes.saturating_add(name.len());
    }
    if bytes > MAX_PROFILE_SHAPE_BYTES {
        return Err(ToolProfileError::ShapeTooLarge);
    }
    Ok(())
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ToolProfileError {
    #[error("tool profile name must be 1..=128 ASCII alphanumeric, '.', '_' or '-' bytes")]
    InvalidName,
    #[error("tool profile executable must not be empty")]
    EmptyExecutable,
    #[error("tool profile contains too many exact arguments")]
    TooManyArguments,
    #[error("tool profile contains too many workspace read files")]
    TooManyWorkspaceReadFiles,
    #[error("tool profile contains too many environment variables")]
    TooManyEnvironmentVariables,
    #[error("tool profile environment variable names must not be empty")]
    InvalidEnvironmentName,
    #[error("tool profile serialized shape exceeds its hard in-memory ceiling")]
    ShapeTooLarge,
    #[error(transparent)]
    InvalidResourceBudget(#[from] LimitError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(timeout_ms: u64) -> ResourceBudget {
        ResourceBudget {
            timeout_ms,
            output_bytes: 1024,
            memory_bytes: 64 * 1024 * 1024,
            process_count: 1,
        }
    }

    fn profile() -> ToolProfile {
        ToolProfile::from_spec(ToolProfileSpec {
            name: ToolProfileName::parse("node-version").expect("profile name"),
            executable: "C:/Program Files/nodejs/node.exe".to_owned(),
            class: ProcessExecutionClass::Interpreter,
            workload_class: WorkloadClass::Heavy,
            exact_args: vec!["--version".to_owned()],
            cwd: Some(WorkspacePath::parse("scratch").expect("cwd")),
            workspace_read_files: BTreeSet::from([
                WorkspacePath::parse("package.json").expect("read file")
            ]),
            env_allowlist: BTreeSet::from(["PATH".to_owned()]),
            network: NetworkAccess::Denied,
            resource_ceiling: budget(5_000),
            approval: ToolApprovalRequirement::HumanRequired,
        })
        .expect("valid profile")
    }

    fn invocation() -> ToolInvocation {
        let profile = profile();
        let spec = profile.spec();
        ToolInvocation {
            executable: spec.executable.clone(),
            class: spec.class,
            workload_class: spec.workload_class,
            args: spec.exact_args.clone(),
            cwd: spec.cwd.clone(),
            workspace_read_files: spec.workspace_read_files.clone(),
            env_allowlist: spec.env_allowlist.clone(),
            network: spec.network,
            resources: budget(4_000),
        }
    }

    #[test]
    fn profile_matches_only_the_complete_normalized_invocation() {
        let profile = profile();
        let exact = invocation();
        assert!(profile.matches_invocation(&exact));

        let mut changed = exact.clone();
        changed.args.push("script.js".to_owned());
        assert!(!profile.matches_invocation(&changed));

        let mut changed = exact.clone();
        changed.cwd = None;
        assert!(!profile.matches_invocation(&changed));

        let mut changed = exact.clone();
        changed.workspace_read_files.clear();
        assert!(!profile.matches_invocation(&changed));

        let mut changed = exact.clone();
        changed.env_allowlist.clear();
        assert!(!profile.matches_invocation(&changed));

        let mut changed = exact.clone();
        changed.network = NetworkAccess::Allowed;
        assert!(!profile.matches_invocation(&changed));
    }

    #[test]
    fn resource_budget_may_narrow_but_never_exceed_profile_ceiling() {
        let profile = profile();
        let mut invocation = invocation();
        invocation.resources.timeout_ms = 5_000;
        assert!(profile.matches_invocation(&invocation));
        invocation.resources.timeout_ms = 5_001;
        assert!(!profile.matches_invocation(&invocation));
    }

    #[test]
    fn profile_name_is_stable_and_strictly_validated() {
        assert_eq!(
            ToolProfileName::parse("node.version_1")
                .expect("valid name")
                .as_str(),
            "node.version_1"
        );
        for invalid in ["", "has space", "slash/name", "é"] {
            assert_eq!(
                ToolProfileName::parse(invalid).expect_err("invalid profile name"),
                ToolProfileError::InvalidName
            );
        }
    }

    #[test]
    fn profile_shape_and_zero_budget_are_bounded() {
        let mut spec = profile().spec().clone();
        spec.exact_args = (0..=MAX_PROFILE_ARGS).map(|_| "x".to_owned()).collect();
        assert_eq!(
            ToolProfile::from_spec(spec).expect_err("too many args"),
            ToolProfileError::TooManyArguments
        );

        let mut spec = profile().spec().clone();
        spec.resource_ceiling.timeout_ms = 0;
        assert_eq!(
            ToolProfile::from_spec(spec).expect_err("zero budget"),
            ToolProfileError::InvalidResourceBudget(LimitError::ZeroIsNotUnlimited)
        );

        let mut spec = profile().spec().clone();
        spec.exact_args = vec!["x".repeat(MAX_PROFILE_SHAPE_BYTES)];
        assert_eq!(
            ToolProfile::from_spec(spec).expect_err("oversized shape"),
            ToolProfileError::ShapeTooLarge
        );
    }
}
