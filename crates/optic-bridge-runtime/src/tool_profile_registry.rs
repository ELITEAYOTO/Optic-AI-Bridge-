use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

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
        self.register_all([profile])
    }

    /// Atomically register a controlled startup batch. Validation happens against
    /// both the existing registry and every incoming name before any profile is
    /// published, so a late duplicate/capacity failure cannot leave partial authority.
    pub fn register_all<I>(&self, profiles: I) -> Result<(), ToolProfileRegistryError>
    where
        I: IntoIterator<Item = ToolProfile>,
    {
        let incoming = profiles.into_iter().collect::<Vec<_>>();
        let mut state = self
            .profiles
            .lock()
            .map_err(|_| ToolProfileRegistryError::StateUnavailable)?;
        let mut incoming_names = BTreeSet::new();
        for profile in &incoming {
            if state.contains_key(profile.name()) || !incoming_names.insert(profile.name().clone())
            {
                return Err(ToolProfileRegistryError::AlreadyRegistered);
            }
        }
        if state.len().saturating_add(incoming.len()) > self.max_profiles {
            return Err(ToolProfileRegistryError::CapacityExceeded);
        }
        for profile in incoming {
            state.insert(profile.name().clone(), profile);
        }
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

    /// Resolve authority from the normalized invocation itself. The caller does
    /// not nominate a profile name. Zero matches and ambiguous matches both fail
    /// closed, so overlapping application profiles cannot silently select one.
    pub fn resolve_unique(
        &self,
        invocation: &ToolInvocation,
    ) -> Result<ToolProfile, ToolProfileRegistryError> {
        let state = self
            .profiles
            .lock()
            .map_err(|_| ToolProfileRegistryError::StateUnavailable)?;
        let mut matches = state
            .values()
            .filter(|profile| profile.matches_invocation(invocation));
        let first = matches
            .next()
            .cloned()
            .ok_or(ToolProfileRegistryError::NoMatchingProfile)?;
        if matches.next().is_some() {
            return Err(ToolProfileRegistryError::AmbiguousProfile);
        }
        Ok(first)
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
    #[error("no application-owned tool profile matches the normalized invocation")]
    NoMatchingProfile,
    #[error("multiple application-owned tool profiles match the same normalized invocation")]
    AmbiguousProfile,
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

    #[test]
    fn batch_registration_is_atomic_on_duplicate_and_capacity_failure() {
        let duplicate = ToolProfileRegistry::new();
        assert_eq!(
            duplicate
                .register_all([profile("same", "a"), profile("same", "b")])
                .expect_err("duplicate batch must fail"),
            ToolProfileRegistryError::AlreadyRegistered
        );
        assert_eq!(
            duplicate
                .get(&ToolProfileName::parse("same").expect("name"))
                .expect_err("failed batch must publish nothing"),
            ToolProfileRegistryError::UnknownProfile
        );

        let limits = HardLimits {
            max_tool_profiles: 1,
            ..HardLimits::default()
        };
        let capacity = ToolProfileRegistry::from_hard_limits(limits).expect("registry");
        assert_eq!(
            capacity
                .register_all([profile("one", "a"), profile("two", "b")])
                .expect_err("oversized batch must fail atomically"),
            ToolProfileRegistryError::CapacityExceeded
        );
        for name in ["one", "two"] {
            assert_eq!(
                capacity
                    .get(&ToolProfileName::parse(name).expect("name"))
                    .expect_err("capacity failure must publish nothing"),
                ToolProfileRegistryError::UnknownProfile
            );
        }
    }

    #[test]
    fn unique_resolution_uses_invocation_not_caller_selected_profile_name() {
        let registry = ToolProfileRegistry::new();
        let safe = profile("safe", "--check");
        let invocation = invocation(&safe);
        registry.register(safe).expect("register safe profile");
        registry
            .register(profile("other", "--version"))
            .expect("register other profile");
        let resolved = registry.resolve_unique(&invocation).expect("unique match");
        assert_eq!(resolved.name().as_str(), "safe");

        let mut drifted = invocation;
        drifted.args.push("unexpected".to_owned());
        assert_eq!(
            registry
                .resolve_unique(&drifted)
                .expect_err("unmatched invocation must fail"),
            ToolProfileRegistryError::NoMatchingProfile
        );
    }

    #[test]
    fn overlapping_profiles_fail_closed_as_ambiguous() {
        let registry = ToolProfileRegistry::new();
        let first = profile("first", "--version");
        let invocation = invocation(&first);
        registry
            .register_all([first, profile("second", "--version")])
            .expect("overlap may be provisioned but never auto-selected");
        assert_eq!(
            registry
                .resolve_unique(&invocation)
                .expect_err("ambiguous authority must fail closed"),
            ToolProfileRegistryError::AmbiguousProfile
        );
    }
}
