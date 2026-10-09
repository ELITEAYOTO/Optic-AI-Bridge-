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
    let file = File::open(path)?;
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
    parse_config_bytes(&bytes)
}

fn parse_config_bytes(bytes: &[u8]) -> Result<LoadedPersistentConfig, io::Error> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(json_error)?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| config_error("Optic config requires integer version"))?;
    if version != u64::from(CONFIG_VERSION_V1) {
        return Err(config_error("unsupported Optic config version"));
    }
    let config: OpticConfigV1 = serde_json::from_value(value).map_err(json_error)?;
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

        let mut args = Vec::new();
        for executable in self.executables {
            push_pair(
                &mut args,
                "--allow-executable",
                format!("{}:{}", executable.class.as_cli(), executable.path),
            );
        }
        for grant in self.process_read_grants {
            args.push("--allow-process-read-file".into());
            args.push(grant.executable.into());
            args.push(grant.path.into());
        }
        for executable in self.isolated_node_executables {
            push_pair(&mut args, "--allow-isolated-node", executable);
        }
        if let Some(path) = self.tool_profile_file {
            push_pair(&mut args, "--tool-profile-file", path);
        }
        for grant in self.environment_grants {
            push_pair(
                &mut args,
                "--env-grant",
                format!("{}:{}", grant.class.as_cli(), grant.name),
            );
        }
        if let Some(mutation) = self.mutation {
            for scope in mutation.write_scopes {
                push_pair(&mut args, "--allow-write-scope", scope);
            }
            for scope in mutation.delete_scopes {
                push_pair(&mut args, "--allow-delete-scope", scope);
            }
            if let Some(state_dir) = mutation.state_dir {
                push_pair(&mut args, "--mutation-state-dir", state_dir);
            }
        }
        if let Some(git) = self.git {
            if let Some(executable) = git.executable {
                push_pair(&mut args, "--git-executable", executable);
            }
            if let Some(integration) = git.integration {
                args.push("--allow-git-integrate".into());
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
}
