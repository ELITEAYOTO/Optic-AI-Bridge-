use std::{collections::BTreeMap, sync::Mutex};

use optic_bridge_core::{HardLimits, ToolInvocation, ToolProfile, ToolProfileName};
use thiserror::Error;

/// Application-owned immutable ToolProfile store.
///
/// Profiles may be registered during controlled application setup, but an
/// existing name can never be replaced or widened in place. Public transports
/// do not receive a mutation API for this registry.
pub struct ToolProfileRegistry {
    profiles: Mutex<BTreeMap<ToolProfileName, ToolProfile>>,
    max_profiles: usize,
}

impl ToolProfileRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::from_hard_limits(HardLimits::default())
            .expect("default hard limits must produce a valid ToolProfile registry")
    }

    pub fn from_hard_limits(limits: HardLimits) -> Result<Self, ToolProfileRegistryError> {
        let limits = limits
            .validate_nonzero()
            .map_err(|_| ToolProfileRegistryError::InvalidLimits)?;
        let max_profiles = usize::try_from(limits.max_tool_profiles)
            .map_err(|_| ToolProfileRegistryError::InvalidLimits)?;
        Ok(Self {
            profiles: Mutex::new(BTreeMap::new()),
            max_profiles,
        })
    }

    pub fn register(&self, profile: ToolProfile) -> Result<(), ToolProfileRegistryError> {
        let mut profiles = self
            .profiles
            .lock()
            .map_err(|_| ToolProfileRegistryError::StateUnavailable)?;
        if profiles.contains_key(profile.name()) {
            return Err(ToolProfileRegistryError::AlreadyRegistered);
        }
        if profiles.len() >= self.max_profiles {
            return Err(ToolProfileRegistryError::CapacityExceeded);
        }
        profiles.insert(profile.name().clone(), profile);
        Ok(())
    }

    pub fn get(&self, name: &ToolProfileName) -> Result<ToolProfile, ToolProfileRegistryError> {
        self.profiles
            .lock()
            .map_err(|_| ToolProfileRegistryError::StateUnavailable)?
            .get(name)
            .cloned()
            .ok_or(ToolProfileRegistryError::UnknownProfile)
    }

    pub fn require_match(
        &self,
        name: &ToolProfileName,
        invocation: &ToolInvocation,
    ) -> Result<ToolProfile, ToolProfileRegistryError> {
        let profile = self.get(name)?;
        if !profile.matches_invocation(invocation) {
            return Err(ToolProfileRegistryError::InvocationMismatch);
        }
        Ok(profile)
    }
}

impl Default for ToolProfileRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ToolProfileRegistryError {
    #[error("tool profile registry limits are invalid")]
    InvalidLimits,
    #[error("tool profile registry state is unavailable")]
    StateUnavailable,
    #[error("tool profile name is already registered and cannot be replaced")]
    AlreadyRegistered,
    #[error("tool profile registry capacity is exhausted")]
    CapacityExceeded,
    #[error("tool profile is unknown")]
    UnknownProfile,
    #[error("process invocation does not exactly match the selected tool profile")]
    InvocationMismatch,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use optic_bridge_core::{
        NetworkAccess, ProcessExecutionClass, ResourceBudget, ToolApprovalRequirement,
        ToolProfileSpec, WorkloadClass, WorkspacePath,
    };

    use super::*;

    fn profile(name: &str, arg: &str) -> ToolProfile {
        ToolProfile::from_spec(ToolProfileSpec {
            name: ToolProfileName::parse(name).expect("profile name"),
            executable: "C:/tool.exe".to_owned(),
            class: ProcessExecutionClass::FixedTool,
            workload_class: WorkloadClass::Standard,
            exact_args: vec![arg.to_owned()],
            cwd: Some(WorkspacePath::parse("scratch").expect("cwd")),
            workspace_read_files: BTreeSet::new(),
            env_allowlist: BTreeSet::new(),
            network: NetworkAccess::Denied,
            resource_ceiling: ResourceBudget {
                timeout_ms: 1_000,
                output_bytes: 1_024,
                memory_bytes: 64 * 1024 * 1024,
                process_count: 1,
            },
            approval: ToolApprovalRequirement::HumanRequired,
        })
        .expect("profile")
    }

    fn invocation(profile: &ToolProfile) -> ToolInvocation {
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
            resources: spec.resource_ceiling,
        }
    }

    #[test]
    fn registered_profile_is_immutable_by_name() {
        let registry = ToolProfileRegistry::new();
        registry.register(profile("fmt", "--check")).expect("first");
        assert_eq!(
            registry
                .register(profile("fmt", "--write"))
                .expect_err("same stable name cannot be widened"),
            ToolProfileRegistryError::AlreadyRegistered
        );
        let loaded = registry
            .get(&ToolProfileName::parse("fmt").expect("name"))
            .expect("profile remains registered");
        assert_eq!(loaded.spec().exact_args, vec!["--check"]);
    }

    #[test]
    fn registry_is_hard_bounded() {
        let limits = HardLimits {
            max_tool_profiles: 1,
            ..HardLimits::default()
        };
        let registry = ToolProfileRegistry::from_hard_limits(limits).expect("registry");
        registry.register(profile("one", "a")).expect("first");
        assert_eq!(
            registry
                .register(profile("two", "b"))
                .expect_err("second must exceed capacity"),
            ToolProfileRegistryError::CapacityExceeded
        );
    }

    #[test]
    fn require_match_rejects_any_invocation_drift() {
        let registry = ToolProfileRegistry::new();
        let profile = profile("node-version", "--version");
        let name = profile.name().clone();
        let exact = invocation(&profile);
        registry.register(profile).expect("register");
        registry.require_match(&name, &exact).expect("exact match");

        let mut drifted = exact;
        drifted.args.push("unexpected.js".to_owned());
        assert_eq!(
            registry
                .require_match(&name, &drifted)
                .expect_err("argument drift must fail"),
            ToolProfileRegistryError::InvocationMismatch
        );
    }
}
