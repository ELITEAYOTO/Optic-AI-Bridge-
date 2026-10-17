//! Actual stdio bootstrap proves permission reductions reach the running core.
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientConfig, Implementation},
};
use serde_json::{Value, json};
use std::{fs, io, path::Path, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt;

struct ProbeClient;
impl ClientHandler for ProbeClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            ClientCapabilities::builder().build(),
            Implementation::new("optic-permission-proof", "1.0.0"),
        )
    }
}

async fn exercise(config: &Path, root: &Path, reduced: bool) {
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_optic-bridge"))
        .args(["--config".as_ref(), config.as_os_str()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap().take(256 * 1024);
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let peer = ProbeClient
            .serve((stdout, stdin))
            .await
            .map_err(io::Error::other)?;
        let request = |name: &str, arguments: Value| -> CallToolRequestParams {
            serde_json::from_value(json!({"name":name,"arguments":arguments})).unwrap()
        };
        let info = peer
            .call_tool(request("session_info", json!({})))
            .await
            .map_err(io::Error::other)?;
        let info = serde_json::to_value(info).map_err(io::Error::other)?;
        let info = &info["structuredContent"];
        let expected = if reduced {
            if cfg!(windows) {
                json!(["directory_create"])
            } else {
                json!([])
            }
        } else {
            json!(["file_read", "file_search"])
        };
        if info["capabilities"] != expected || info["policy_epoch"] != 7 {
            return Err(io::Error::other(format!(
                "unexpected live authority: {info}"
            )));
        }
        let tools = peer.list_tools(None).await.map_err(io::Error::other)?;
        for denied in ["fs_write", "fs_apply_patch", "fs_delete"] {
            if tools.tools.iter().any(|tool| tool.name == denied) {
                return Err(io::Error::other("disabled mutation advertised"));
            }
        }
        for (name, args) in [
            ("fs_read", json!({"path":"sentinel.txt"})),
            ("fs_list", json!({})),
        ] {
            let answer = peer.call_tool(request(name, args)).await;
            if reduced {
                match answer {
                    Err(rmcp::service::ServiceError::McpError(error))
                        if error.message == "optic.policy_denied" =>
                    {
                        ()
                    }
                    other => {
                        return Err(io::Error::other(format!(
                            "read denial was not enforced: {other:?}"
                        )));
                    }
                }
            } else {
                let answer = answer.map_err(io::Error::other)?;
                if answer.is_error == Some(true) {
                    return Err(io::Error::other("legacy read denied"));
                }
            }
        }
        if reduced && cfg!(windows) {
            let made = peer
                .call_tool(request(
                    "fs_create_directory",
                    json!({"path":"modules","expected":"absent"}),
                ))
                .await
                .map_err(io::Error::other)?;
            if made.is_error == Some(true) || !root.join("modules").is_dir() {
                return Err(io::Error::other("directory-only scope did not execute"));
            }
        }
        peer.call_tool(request("session_cancel", json!({})))
            .await
            .map_err(io::Error::other)?;
        let _ = peer.cancel().await;
        Ok::<_, io::Error>(())
    })
    .await;
    let ended = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
    if !matches!(&ended, Ok(Ok(_))) {
        child.kill().await.unwrap();
    }
    assert!(
        matches!(&ended, Ok(Ok(status)) if status.success()),
        "owned stdio core did not exit after client shutdown"
    );
    result
        .expect("MCP deadline")
        .expect("actual permission proof");
}

#[tokio::test]
async fn operator_permissions_reduce_actual_stdio_authority_and_preserve_legacy_reads() {
    let base = std::env::temp_dir().join(format!(
        "optic-permission-proof-{}",
        optic_bridge_core::ActionId::generate().unwrap().to_token()
    ));
    let root = base.join("project");
    let state = base.join("state");
    fs::create_dir(&base).unwrap();
    fs::create_dir(&root).unwrap();
    fs::create_dir(&state).unwrap();
    fs::write(root.join("sentinel.txt"), b"fixed public fixture").unwrap();
    let config = base.join("operator.json");
    let policy = json!({"version":1,"policy_epoch":7,"workspace":root,
        "permissions":{"files":{"create_directories":cfg!(windows)}},
        "mutation":{"write_scopes":["all"],"delete_scopes":["all"],"directory_scopes":["all"],"state_dir":state}});
    fs::write(&config, policy.to_string()).unwrap();
    exercise(&config, &root, true).await;
    fs::write(
        &config,
        json!({"version":1,"policy_epoch":7,"workspace":root}).to_string(),
    )
    .unwrap();
    exercise(&config, &root, false).await;
    let canonical = fs::canonicalize(&base).unwrap();
    assert_eq!(
        canonical.parent().unwrap(),
        fs::canonicalize(std::env::temp_dir()).unwrap()
    );
    assert!(
        canonical
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("optic-permission-proof-")
    );
    fs::remove_dir_all(canonical).unwrap();
}
