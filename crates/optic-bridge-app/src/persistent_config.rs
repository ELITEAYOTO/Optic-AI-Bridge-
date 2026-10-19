use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde::Deserialize;

const MAX_CONFIG_FILE_BYTES: u64 = 256 * 1024;
const CONFIG_VERSION_V1: u32 = 1;

#[derive(Debug)]
pub struct LoadedPersistentConfig {
    pub policy_epoch: u64,
    pub args: Vec<OsString>,
}

pub fn load_exclusive_config_args(
    args: &[OsString],
) -> Result<Option<LoadedPersistentConfig>, io::Error> {
    let path = match args {
        [flag, path] if flag == "--config" => Some(PathBuf::from(path)),
        [flag] => flag
            .to_str()
            .and_then(|value| value.strip_prefix("--config="))
            .map(PathBuf::from),
        _ if args.iter().any(|arg| is_config_arg(arg)) => {
            return Err(config_error("--config must be the only startup option"));
        }
        _ => None,
    };
    path.map(|path| load_config_file(&path)).transpose()
}

fn is_config_arg(arg: &OsStr) -> bool {
    arg == "--config"
        || arg
            .to_str()
            .is_some_and(|value| value.starts_with("--config="))
}

fn load_config_file(path: &Path) -> Result<LoadedPersistentConfig, io::Error> {
    if !path.is_absolute() {
        return Err(config_error(
            "--config must reference an absolute JSON file path",
        ));
    }
    let (file, observed_source) = open_operator_file(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(config_error("--config must reference a regular file"));
    }
    if metadata.len() > MAX_CONFIG_FILE_BYTES {
        return Err(config_error(
            "Optic config exceeds the hard file-size limit",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_CONFIG_FILE_BYTES {
        return Err(config_error(
            "Optic config exceeds the hard file-size limit",
        ));
    }
    let loaded = parse_config_bytes(&bytes)?;
    let workspace = loaded
        .args
        .last()
        .ok_or_else(|| config_error("missing workspace"))?;
    let workspace = std::fs::canonicalize(Path::new(workspace))?;
    require_external_operator_file(&workspace, &observed_source)?;
    for pair in loaded.args.windows(2) {
        if pair[0] == "--tool-profile-file" {
            let (_file, observed) = open_operator_file(Path::new(&pair[1]))?;
            require_external_operator_file(&workspace, &observed)?;
        }
    }
    Ok(loaded)
}

fn open_operator_file(path: &Path) -> Result<(File, PathBuf), io::Error> {
    if !path.is_absolute() {
        return Err(config_error("operator policy files require absolute paths"));
    }
    #[cfg(windows)]
    {
        let mut opened =
            optic_bridge_windows::open_file_no_reparse(path).map_err(io::Error::other)?;
        let observed = opened.final_path().to_owned();
        Ok((opened.file_mut().try_clone()?, observed))
    }
    #[cfg(not(windows))]
    {
        Ok((File::open(path)?, std::fs::canonicalize(path)?))
    }
}

fn require_external_operator_file(workspace: &Path, source: &Path) -> Result<(), io::Error> {
    #[cfg(windows)]
    let within = {
        if workspace.to_str().is_none() || source.to_str().is_none() {
            return Err(config_error(
                "operator policy boundaries require losslessly representable UTF-8 paths",
            ));
        }
        let mut source = source.components();
        workspace.components().all(|component| {
            source.next().is_some_and(|candidate| {
                component.as_os_str().to_str().map(str::to_lowercase)
                    == candidate.as_os_str().to_str().map(str::to_lowercase)
            })
        })
    };
    #[cfg(not(windows))]
    let within = source.starts_with(workspace);
    if within {
        return Err(config_error(
            "operator config and tool profiles must stay outside the writable workspace",
        ));
    }
    Ok(())
}

fn parse_config_bytes(bytes: &[u8]) -> Result<LoadedPersistentConfig, io::Error> {
    // A generic Value silently collapses duplicate keys before strict typed
    // validation. Authority-bearing configuration must reject that ambiguity.
    let config: OpticConfigV1 = serde_json::from_slice(bytes).map_err(json_error)?;
    config.into_loaded()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpticConfigV1 {
    version: u32,
    policy_epoch: u64,
    workspace: String,
    #[serde(default)]
    executables: Vec<ExecutableConfig>,
    #[serde(default)]
    process_read_grants: Vec<ProcessReadGrantConfig>,
    #[serde(default)]
    isolated_node_executables: Vec<String>,
    tool_profile_file: Option<String>,
    #[serde(default)]
    environment_grants: Vec<EnvironmentGrantConfig>,
    mutation: Option<MutationConfig>,
    git: Option<GitConfig>,
    permissions: Option<Permissions>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Permissions {
    files: FilePermissions,
    processes: ProcessPermissions,
    git: GitPermissions,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FilePermissions {
    read: bool,
    search: bool,
    write: bool,
    delete: bool,
    create_directories: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ProcessPermissions {
    run: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct GitPermissions {
    read: bool,
    merge: bool,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ExecutableClass {
    FixedTool,
    Interpreter,
    RepositoryCode,
}

impl ExecutableClass {
    const fn as_cli(self) -> &'static str {
        match self {
            Self::FixedTool => "fixed-tool",
            Self::Interpreter => "interpreter",
            Self::RepositoryCode => "repository-code",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutableConfig {
    class: ExecutableClass,
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessReadGrantConfig {
    executable: String,
    path: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum EnvironmentClass {
    Benign,
    Sensitive,
    Forbidden,
}

impl EnvironmentClass {
    const fn as_cli(self) -> &'static str {
        match self {
            Self::Benign => "benign",
            Self::Sensitive => "sensitive",
            Self::Forbidden => "forbidden",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvironmentGrantConfig {
    class: EnvironmentClass,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationConfig {
    #[serde(default)]
    write_scopes: Vec<String>,
    #[serde(default)]
    delete_scopes: Vec<String>,
    #[serde(default)]
    directory_scopes: Vec<String>,
    state_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GitConfig {
    executable: Option<String>,
    integration: Option<GitIntegrationConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GitIntegrationConfig {
    executable: String,
    root: String,
    target_ref: String,
}

impl OpticConfigV1 {
    fn into_loaded(self) -> Result<LoadedPersistentConfig, io::Error> {
        if self.version != CONFIG_VERSION_V1 {
            return Err(config_error("unsupported Optic config version"));
        }
        if self.policy_epoch == 0 {
            return Err(config_error("policy_epoch must be greater than zero"));
        }
        if self.workspace.is_empty() {
            return Err(config_error("workspace must not be empty"));
        }

        // Compatibility: no permissions section preserves explicit legacy
        // authority. When present, every omitted boolean defaults to denied.
        // A true flag only enables supplied scopes/profiles; it mints no scope.
        let read = self.permissions.as_ref().is_none_or(|p| p.files.read);
        let search = self.permissions.as_ref().is_none_or(|p| p.files.search);
        let run = self.permissions.as_ref().is_none_or(|p| p.processes.run);
        let write = self.permissions.as_ref().is_none_or(|p| p.files.write);
        let delete = self.permissions.as_ref().is_none_or(|p| p.files.delete);
        let directories = self
            .permissions
            .as_ref()
            .is_none_or(|p| p.files.create_directories);
        let git_read = self.permissions.as_ref().is_none_or(|p| p.git.read);
        let merge = self.permissions.as_ref().is_none_or(|p| p.git.merge);
        let mut args = Vec::new();
        if !read {
            args.push("--deny-file-read".into());
        }
        if !search {
            args.push("--deny-file-search".into());
        }
        for executable in self.executables {
            if !run {
                continue;
            }
            push_pair(
                &mut args,
                "--allow-executable",
                format!("{}:{}", executable.class.as_cli(), executable.path),
            );
        }
        for grant in self.process_read_grants {
            if !run || !read {
                continue;
            }
            args.push("--allow-process-read-file".into());
            args.push(grant.executable.into());
            args.push(grant.path.into());
        }
        for executable in self.isolated_node_executables {
            if !run {
                continue;
            }
            push_pair(&mut args, "--allow-isolated-node", executable);
        }
        if let Some(path) = self.tool_profile_file
            && run
        {
            push_pair(&mut args, "--tool-profile-file", path);
        }
        for grant in self.environment_grants {
            if !run {
                continue;
            }
            push_pair(
                &mut args,
                "--env-grant",
                format!("{}:{}", grant.class.as_cli(), grant.name),
            );
        }
        if let Some(mutation) = self.mutation {
            for scope in mutation.write_scopes {
                if !write {
                    continue;
                }
                push_pair(&mut args, "--allow-write-scope", scope);
            }
            for scope in mutation.delete_scopes {
                if !delete {
                    continue;
                }
                push_pair(&mut args, "--allow-delete-scope", scope);
            }
            for scope in mutation.directory_scopes {
                if !directories {
                    continue;
                }
                push_pair(&mut args, "--allow-directory-create-scope", scope);
            }
            if let Some(state_dir) = mutation.state_dir {
                push_pair(&mut args, "--mutation-state-dir", state_dir);
            }
        }
        if let Some(git) = self.git {
            if let Some(executable) = git.executable
                && git_read
            {
                push_pair(&mut args, "--git-executable", executable);
            }
            if let Some(integration) = git.integration {
                if merge {
                    args.push("--allow-git-integrate".into());
                }
                push_pair(
                    &mut args,
                    "--git-integration-executable",
                    integration.executable,
                );
                push_pair(&mut args, "--git-integration-root", integration.root);
                push_pair(&mut args, "--git-integration-ref", integration.target_ref);
            }
        }
        args.push(self.workspace.into());
        Ok(LoadedPersistentConfig {
            policy_epoch: self.policy_epoch,
            args,
        })
    }
}

fn push_pair(args: &mut Vec<OsString>, flag: &str, value: String) {
    args.push(flag.into());
    args.push(value.into());
}

fn config_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn json_error(error: serde_json::Error) -> io::Error {
    config_error(format!("invalid Optic config JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_authority_fields_are_rejected_before_any_key_can_be_collapsed() {
        for value in [
            r#"{"version":1,"version":1,"policy_epoch":1,"workspace":"x"}"#,
            r#"{"version":1,"policy_epoch":1,"policy_epoch":2,"workspace":"x"}"#,
            r#"{"version":1,"policy_epoch":1,"workspace":"one","workspace":"two"}"#,
            r#"{"version":1,"policy_epoch":1,"workspace":"x","mutation":null,"mutation":{}}"#,
            r#"{"version":1,"policy_epoch":1,"workspace":"x","mutation":{"write_scopes":["prefix:src"],"write_scopes":["all"]}}"#,
            r#"{"version":1,"policy_epoch":1,"workspace":"x","executables":[{"class":"fixed-tool","class":"repository-code","path":"C:\\tool.exe"}]}"#,
        ] {
            let error = parse(value).unwrap_err();
            assert!(error.to_string().contains("duplicate field"), "{error}");
        }
    }

    fn parse(input: &str) -> Result<LoadedPersistentConfig, io::Error> {
        parse_config_bytes(input.as_bytes())
    }

    #[test]
    fn v1_config_compiles_to_existing_cli_authority_surface() {
        let loaded = parse(
            r#"{
                "version": 1,
                "policy_epoch": 7,
                "workspace": "C:\\work",
                "executables": [{"class":"interpreter","path":"C:\\node.exe"}],
                "process_read_grants": [{"executable":"C:\\node.exe","path":"input.txt"}],
                "isolated_node_executables": ["C:\\node.exe"],
                "environment_grants": [{"class":"sensitive","name":"API_KEY"}],
                "mutation": {"write_scopes":["prefix:src"],"delete_scopes":[],"state_dir":"C:\\state"},
                "git": {"executable":"C:\\git.exe","integration":null}
            }"#,
        )
        .expect("valid v1 config");
        assert_eq!(loaded.policy_epoch, 7);
        assert!(loaded.args.iter().any(|arg| arg == "--allow-executable"));
        assert!(loaded.args.iter().any(|arg| arg == "--env-grant"));
        assert_eq!(loaded.args.last(), Some(&OsString::from(r"C:\work")));
    }

    #[test]
    fn unknown_version_unknown_field_and_zero_epoch_fail_closed() {
        assert!(parse(r#"{"version":2,"policy_epoch":1,"workspace":"x"}"#).is_err());
        assert!(parse(r#"{"version":1,"policy_epoch":1,"workspace":"x","extra":true}"#).is_err());
        assert!(parse(r#"{"version":1,"policy_epoch":0,"workspace":"x"}"#).is_err());
    }

    #[test]
    fn config_mode_is_exclusive() {
        let config = OsString::from("--config=C:\\optic.json");
        assert!(load_exclusive_config_args(&[config, OsString::from("workspace")]).is_err());
    }

    #[test]
    fn directory_configuration_does_not_imply_file_write_or_delete() {
        let loaded = parse(r#"{"version":1,"policy_epoch":1,"workspace":"C:\\work","mutation":{"directory_scopes":["prefix:src"],"state_dir":"C:\\state"}}"#).expect("directory-only config");
        assert!(
            loaded
                .args
                .iter()
                .any(|arg| arg == "--allow-directory-create-scope")
        );
        assert!(
            !loaded
                .args
                .iter()
                .any(|arg| arg == "--allow-write-scope" || arg == "--allow-delete-scope")
        );
        assert!(parse(r#"{"version":1,"policy_epoch":1,"workspace":"x","mutation":{"directory_scopes":true}}"#).is_err());
    }

    #[test]
    fn permission_switches_default_to_denied_and_never_mint_missing_scope() {
        let loaded = parse(r#"{"version":1,"policy_epoch":1,"workspace":"x","permissions":{}}"#)
            .expect("deny preset");
        assert!(loaded.args.contains(&"--deny-file-read".into()));
        assert!(loaded.args.contains(&"--deny-file-search".into()));
        let loaded = parse(r#"{"version":1,"policy_epoch":1,"workspace":"x","permissions":{"files":{"read":true,"create_directories":true}}}"#).expect("no implicit scope");
        assert!(
            !loaded
                .args
                .contains(&"--allow-directory-create-scope".into())
        );
        let loaded = parse(r#"{"version":1,"policy_epoch":1,"workspace":"x",
          "permissions":{"files":{"create_directories":true},"processes":{"run":false}},
          "executables":[{"class":"interpreter","path":"C:\\node.exe"}],
          "isolated_node_executables":["C:\\node.exe"],"tool_profile_file":"C:\\profiles.json",
          "process_read_grants":[{"executable":"C:\\node.exe","path":"a.js"}],
          "mutation":{"write_scopes":["all"],"delete_scopes":["all"],"directory_scopes":["prefix:src"],"state_dir":"C:\\state"}
        }"#).expect("directory preset");
        for forbidden in [
            "--allow-write-scope",
            "--allow-delete-scope",
            "--allow-executable",
            "--allow-isolated-node",
            "--tool-profile-file",
            "--allow-process-read-file",
        ] {
            assert!(
                !loaded.args.contains(&forbidden.into()),
                "unexpected authority: {forbidden}"
            );
        }
        assert!(
            loaded
                .args
                .contains(&"--allow-directory-create-scope".into())
        );
    }

    #[test]
    fn malformed_duplicate_or_unknown_permission_fields_fail_closed() {
        for input in [
            r#"{"version":1,"policy_epoch":1,"policy_epoch":2,"workspace":"x"}"#,
            r#"{"version":1,"policy_epoch":1,"workspace":"x","permissions":{"files":{"write":true,"write":false}}}"#,
            r#"{"version":1,"policy_epoch":1,"workspace":"x","permissions":{"files":{"write":"false"}}}"#,
            r#"{"version":1,"policy_epoch":1,"workspace":"x","permissions":{"git":{"push":true}}}"#,
            r#"{"version":1,"policy_epoch":1,"workspace":"x","permissions":{"full_access":true}}"#,
        ] {
            assert!(parse(input).is_err(), "invalid policy accepted");
        }
    }

    #[test]
    fn writable_project_cannot_supply_its_operator_policy_or_tool_profiles() {
        let base = std::env::temp_dir().join(format!(
            "optic-config-boundary-{}",
            optic_bridge_core::ActionId::generate()
                .expect("entropy")
                .to_token()
        ));
        let workspace = base.join("workspace");
        std::fs::create_dir_all(&workspace).expect("workspace");
        let policy = serde_json::json!({"version":1,"policy_epoch":1,"workspace":workspace});
        let outside = base.join("config.json");
        let inside = workspace.join("config.json");
        std::fs::write(&outside, policy.to_string()).expect("outside policy");
        std::fs::write(&inside, policy.to_string()).expect("inside policy");
        assert!(load_config_file(&outside).is_ok());
        assert!(load_config_file(&inside).is_err());
        let mut policy = policy;
        policy["tool_profile_file"] = serde_json::json!(inside);
        std::fs::write(&outside, policy.to_string()).expect("profile policy");
        assert!(load_config_file(&outside).is_err());
        std::fs::remove_dir_all(base).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn operator_policy_boundary_refuses_unrepresentable_native_paths() {
        use std::os::windows::ffi::OsStringExt;

        let mut units: Vec<u16> = r"\\?\C:\workspace-".encode_utf16().collect();
        units.push(0xd800);
        let root = std::path::PathBuf::from(OsString::from_wide(&units));
        let valid = Path::new(r"\\?\C:\operator\config.json");
        assert!(require_external_operator_file(&root, valid).is_err());
        assert!(require_external_operator_file(valid, &root).is_err());
        assert!(require_external_operator_file(&root, &root.join("config.json")).is_err());
    }
}
