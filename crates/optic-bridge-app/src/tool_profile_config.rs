use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    io::Read,
    path::Path,
};

use optic_bridge_core::{
    HardLimits, NetworkAccess, ProcessExecutionClass, ResourceBudget, ToolApprovalRequirement,
    ToolProfile, ToolProfileName, ToolProfileSpec, WorkloadClass, WorkspacePath,
};
use optic_bridge_runtime::{ProcessManager, ToolProfileRegistry};
use serde::Deserialize;

const TOOL_PROFILE_FILE_VERSION: u32 = 1;
const MAX_TOOL_PROFILE_FILE_BYTES: u64 = 256 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolProfileDocument {
    version: u32,
    profiles: Vec<ToolProfileConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolProfileConfig {
    name: String,
    executable: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<String>,
    #[serde(default)]
    env_allowlist: Vec<String>,
    resources: ToolProfileResourceConfig,
    #[serde(default = "default_require_approval")]
    require_approval: bool,
}

fn default_require_approval() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolProfileResourceConfig {
    timeout_ms: u64,
    output_bytes: u64,
    memory_bytes: u64,
    process_count: u32,
}

pub(crate) struct LoadedToolProfiles {
    pub registry: ToolProfileRegistry,
    pub profiles: Vec<ToolProfile>,
}

pub(crate) fn load_tool_profiles(
    path: Option<&Path>,
    processes: &ProcessManager,
    executables: &BTreeMap<String, ProcessExecutionClass>,
    read_grants: &BTreeMap<String, BTreeSet<WorkspacePath>>,
    isolation_eligible: &BTreeSet<String>,
    limits: HardLimits,
) -> Result<Option<LoadedToolProfiles>, Box<dyn Error + Send + Sync>> {
    let Some(path) = path else {
        return Ok(None);
    };
    if !path.is_absolute() {
        return Err(config_error("--tool-profile-file must be an absolute path"));
    }

    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(config_error(
            "--tool-profile-file must reference a regular file",
        ));
    }
    if metadata.len() > MAX_TOOL_PROFILE_FILE_BYTES {
        return Err(config_error(
            "tool profile file exceeds the 256 KiB hard limit",
        ));
    }
    let mut bytes = Vec::with_capacity(
        usize::try_from(metadata.len().min(MAX_TOOL_PROFILE_FILE_BYTES)).unwrap_or(0),
    );
    file.by_ref()
        .take(MAX_TOOL_PROFILE_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_TOOL_PROFILE_FILE_BYTES {
        return Err(config_error(
            "tool profile file exceeds the 256 KiB hard limit",
        ));
    }

    let document: ToolProfileDocument = serde_json::from_slice(&bytes)
        .map_err(|error| config_error(format!("invalid tool profile JSON: {error}")))?;
    if document.version != TOOL_PROFILE_FILE_VERSION {
        return Err(config_error("unsupported tool profile file version"));
    }
    if document.profiles.is_empty() {
        return Err(config_error(
            "tool profile file must contain at least one profile",
        ));
    }
    if document.profiles.len() > usize::try_from(limits.max_tool_profiles).unwrap_or(usize::MAX) {
        return Err(config_error("tool profile count exceeds the hard limit"));
    }

    let mut profiles = Vec::with_capacity(document.profiles.len());
    for config in document.profiles {
        let canonical = processes.canonicalize_executable(&config.executable)?;
        let class = *executables.get(&canonical).ok_or_else(|| {
            config_error("tool profile executable must also be authorized by --allow-executable")
        })?;
        let resource_ceiling = ResourceBudget {
            timeout_ms: config.resources.timeout_ms,
            output_bytes: config.resources.output_bytes,
            memory_bytes: config.resources.memory_bytes,
            process_count: config.resources.process_count,
        }
        .validate_nonzero()
        .map_err(|_| config_error("tool profile resources must all be nonzero"))?;
        let authority_ceiling = if isolation_eligible.contains(&canonical) {
            ResourceBudget {
                process_count: 1,
                ..limits.max_process_budget
            }
        } else {
            limits.max_process_budget
        };
        if !resource_ceiling.fits_within(authority_ceiling) {
            return Err(config_error(
                "tool profile resource ceiling exceeds its process authority ceiling",
            ));
        }
        let cwd = config
            .cwd
            .as_deref()
            .map(WorkspacePath::parse)
            .transpose()
            .map_err(|_| config_error("tool profile cwd must be a safe workspace-relative path"))?;
        let env_allowlist = processes.normalize_environment_allowlist(&config.env_allowlist)?;
        let workspace_read_files = read_grants.get(&canonical).cloned().unwrap_or_default();
        let approval = if config.require_approval {
            ToolApprovalRequirement::HumanRequired
        } else {
            ToolApprovalRequirement::NotRequired
        };
        profiles.push(ToolProfile::from_spec(ToolProfileSpec {
            name: ToolProfileName::parse(config.name)
                .map_err(|_| config_error("invalid tool profile name"))?,
            executable: canonical,
            class,
            workload_class: WorkloadClass::Heavy,
            exact_args: config.args,
            cwd,
            workspace_read_files,
            env_allowlist,
            network: NetworkAccess::Denied,
            resource_ceiling,
            approval,
        })?);
    }

    validate_nonoverlapping_profiles(&profiles)?;
    let registry = ToolProfileRegistry::from_hard_limits(limits)?;
    registry.register_all(profiles.clone())?;
    Ok(Some(LoadedToolProfiles { registry, profiles }))
}

fn validate_nonoverlapping_profiles(
    profiles: &[ToolProfile],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for (index, left) in profiles.iter().enumerate() {
        for right in &profiles[index + 1..] {
            let left = left.spec();
            let right = right.spec();
            if left.executable == right.executable
                && left.class == right.class
                && left.workload_class == right.workload_class
                && left.exact_args == right.exact_args
                && left.cwd == right.cwd
                && left.workspace_read_files == right.workspace_read_files
                && left.env_allowlist == right.env_allowlist
                && left.network == right.network
            {
                return Err(config_error(format!(
                    "tool profiles '{}' and '{}' overlap the same normalized invocation",
                    left.name.as_str(),
                    right.name.as_str()
                )));
            }
        }
    }
    Ok(())
}

fn config_error(message: impl Into<String>) -> Box<dyn Error + Send + Sync> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message.into(),
    ))
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use optic_bridge_core::ActionId;
    use optic_bridge_runtime::{EnvironmentGrant, EnvironmentVariableClass};

    use super::*;

    fn workspace(label: &str) -> PathBuf {
        let token = ActionId::generate().expect("entropy").to_token();
        let root = env::temp_dir().join(format!("optic-tool-profile-config-{label}-{token}"));
        fs::create_dir_all(&root).expect("workspace");
        root
    }

    fn write_config(root: &Path, executable: &Path, profiles: &str) -> PathBuf {
        let path = root.join("profiles.json");
        let executable = executable.to_string_lossy().replace('\\', "\\\\");
        let body =
            format!(r#"{{"version":1,"profiles":{profiles}}}"#).replace("__EXE__", &executable);
        fs::write(&path, body).expect("write config");
        path
    }

    fn authority(
        root: &Path,
    ) -> (
        ProcessManager,
        String,
        BTreeMap<String, ProcessExecutionClass>,
    ) {
        let manager = ProcessManager::new(root, HardLimits::default(), ["PATH".to_owned()])
            .expect("process manager");
        let executable = env::current_exe()
            .expect("current exe")
            .canonicalize()
            .expect("canonical exe")
            .to_string_lossy()
            .into_owned();
        let canonical = manager
            .canonicalize_executable(&executable)
            .expect("canonical executable");
        let executables = BTreeMap::from([(canonical.clone(), ProcessExecutionClass::FixedTool)]);
        (manager, canonical, executables)
    }

    #[test]
    fn explicit_profile_file_is_bounded_and_derives_existing_authority() {
        let root = workspace("load");
        let (manager, executable, executables) = authority(&root);
        let path = write_config(
            &root,
            Path::new(&executable),
            r#"[{"name":"safe-check","executable":"__EXE__","args":["--check"],"cwd":"scratch","env_allowlist":["PATH"],"resources":{"timeout_ms":1000,"output_bytes":1024,"memory_bytes":67108864,"process_count":1}}]"#,
        );
        fs::create_dir_all(root.join("scratch")).expect("scratch");
        let loaded = load_tool_profiles(
            Some(&path),
            &manager,
            &executables,
            &BTreeMap::new(),
            &BTreeSet::new(),
            HardLimits::default(),
        )
        .expect("load")
        .expect("configured");
        assert_eq!(loaded.profiles.len(), 1);
        let spec = loaded.profiles[0].spec();
        assert_eq!(spec.executable, executable);
        assert_eq!(spec.class, ProcessExecutionClass::FixedTool);
        assert_eq!(spec.workload_class, WorkloadClass::Heavy);
        assert_eq!(spec.network, NetworkAccess::Denied);
        assert_eq!(spec.approval, ToolApprovalRequirement::HumanRequired);
        assert_eq!(spec.env_allowlist, BTreeSet::from(["PATH".to_owned()]));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn sensitive_environment_grant_cannot_be_exported_by_tool_profile() {
        let root = workspace("sensitive-env");
        let manager = ProcessManager::new_with_environment_grants(
            &root,
            HardLimits::default(),
            [EnvironmentGrant {
                name: "SECRET".to_owned(),
                class: EnvironmentVariableClass::Sensitive,
            }],
        )
        .expect("classified manager");
        let executable = env::current_exe()
            .expect("current exe")
            .canonicalize()
            .expect("canonical exe")
            .to_string_lossy()
            .into_owned();
        let canonical = manager
            .canonicalize_executable(&executable)
            .expect("canonical executable");
        let executables = BTreeMap::from([(canonical.clone(), ProcessExecutionClass::FixedTool)]);
        let path = write_config(
            &root,
            Path::new(&canonical),
            r#"[{"name":"secret-profile","executable":"__EXE__","args":[],"env_allowlist":["SECRET"],"resources":{"timeout_ms":1000,"output_bytes":1024,"memory_bytes":67108864,"process_count":1}}]"#,
        );
        assert!(
            load_tool_profiles(
                Some(&path),
                &manager,
                &executables,
                &BTreeMap::new(),
                &BTreeSet::new(),
                HardLimits::default(),
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn profile_cannot_mint_executable_or_environment_authority() {
        let root = workspace("authority");
        let (manager, executable, _) = authority(&root);
        let path = write_config(
            &root,
            Path::new(&executable),
            r#"[{"name":"unsafe","executable":"__EXE__","args":[],"env_allowlist":["SECRET"],"resources":{"timeout_ms":1000,"output_bytes":1024,"memory_bytes":67108864,"process_count":1}}]"#,
        );
        assert!(
            load_tool_profiles(
                Some(&path),
                &manager,
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeSet::new(),
                HardLimits::default(),
            )
            .is_err()
        );

        let (_, _, executables) = authority(&root);
        assert!(
            load_tool_profiles(
                Some(&path),
                &manager,
                &executables,
                &BTreeMap::new(),
                &BTreeSet::new(),
                HardLimits::default(),
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn overlapping_profiles_fail_at_startup() {
        let root = workspace("overlap");
        let (manager, executable, executables) = authority(&root);
        let path = write_config(
            &root,
            Path::new(&executable),
            r#"[{"name":"one","executable":"__EXE__","args":["--version"],"resources":{"timeout_ms":1000,"output_bytes":1024,"memory_bytes":67108864,"process_count":1}},{"name":"two","executable":"__EXE__","args":["--version"],"resources":{"timeout_ms":2000,"output_bytes":2048,"memory_bytes":134217728,"process_count":1},"require_approval":true}]"#,
        );
        let error = load_tool_profiles(
            Some(&path),
            &manager,
            &executables,
            &BTreeMap::new(),
            &BTreeSet::new(),
            HardLimits::default(),
        )
        .err()
        .expect("overlap must fail");
        assert!(error.to_string().contains("overlap"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn absent_profile_file_keeps_profile_mode_disabled() {
        let root = workspace("disabled");
        let (manager, _, executables) = authority(&root);
        assert!(
            load_tool_profiles(
                None,
                &manager,
                &executables,
                &BTreeMap::new(),
                &BTreeSet::new(),
                HardLimits::default(),
            )
            .expect("disabled")
            .is_none()
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
