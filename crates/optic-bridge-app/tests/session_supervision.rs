//! Actual stdio ownership/expiry contract; no host adapter or Python runtime.
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientConfig, Implementation},
};
use serde_json::{Value, json};
use std::{fs, io, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt;

struct ProbeClient;
impl ClientHandler for ProbeClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            ClientCapabilities::builder().build(),
            Implementation::new("optic-supervision-proof", "1.0.0"),
        )
    }
}
fn request(name: &str, arguments: Value) -> CallToolRequestParams {
    serde_json::from_value(json!({"name":name,"arguments":arguments})).unwrap()
}

async fn exercise(supervised: bool, revoke: bool, cross_expiry: bool) {
    let root = std::env::temp_dir().join(format!(
        "optic-supervision-proof-{}",
        optic_bridge_core::ActionId::generate().unwrap().to_token()
    ));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("public.txt"), b"PUBLIC_SUPERVISION_FIXTURE\n").unwrap();
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_optic-bridge"));
    command.arg(&root).args(["--session-lifetime-seconds", "5"]);
    if supervised {
        command.arg("--supervise-session");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap().take(256 * 1024);
    let result = tokio::time::timeout(Duration::from_secs(25), async {
        let peer = ProbeClient
            .serve((stdout, stdin))
            .await
            .map_err(io::Error::other)?;
        let surface = peer.list_tools(None).await.map_err(io::Error::other)?;
        if surface
            .tools
            .iter()
            .any(|tool| matches!(tool.name.as_ref(), "session_renew" | "session_supervise"))
        {
            return Err(io::Error::other("remote renewal tool advertised"));
        }
        let before = peer
            .call_tool(request("session_info", json!({})))
            .await
            .map_err(io::Error::other)?;
        let before = serde_json::to_value(before).map_err(io::Error::other)?;
        let mut before = before["structuredContent"].clone();
        if before["capabilities"] != json!(["file_read", "file_search"]) {
            return Err(io::Error::other("unexpected initial authority"));
        }
        // No traffic/heartbeat renews these leases. Cross the initial window
        // for expiry/renewal tests; the active-EOF case stops after a timer tick.
        tokio::time::sleep(Duration::from_millis(if cross_expiry {
            6500
        } else {
            2100
        }))
        .await;
        if supervised {
            let after = peer
                .call_tool(request("session_info", json!({})))
                .await
                .map_err(io::Error::other)?;
            let after = serde_json::to_value(after).map_err(io::Error::other)?;
            let mut after = after["structuredContent"].clone();
            let old_expiry = before["expires_at_monotonic_ms"]
                .as_u64()
                .ok_or_else(|| io::Error::other("missing finite initial expiry"))?;
            let new_expiry = after["expires_at_monotonic_ms"]
                .as_u64()
                .ok_or_else(|| io::Error::other("missing finite renewed expiry"))?;
            if new_expiry <= old_expiry {
                return Err(io::Error::other("local timer did not renew"));
            }
            before
                .as_object_mut()
                .unwrap()
                .remove("expires_at_monotonic_ms");
            after
                .as_object_mut()
                .unwrap()
                .remove("expires_at_monotonic_ms");
            if before != after {
                return Err(io::Error::other("renewal changed existing authority"));
            }
            let read = peer
                .call_tool(request("fs_read", json!({"path":"public.txt"})))
                .await
                .map_err(io::Error::other)?;
            if read.is_error == Some(true) {
                return Err(io::Error::other("renewed existing read denied"));
            }
            if revoke {
                peer.call_tool(request("session_cancel", json!({})))
                    .await
                    .map_err(io::Error::other)?;
                tokio::time::sleep(Duration::from_millis(2100)).await;
            }
        }
        if !supervised || revoke {
            for (name, args) in [
                ("session_info", json!({})),
                ("fs_read", json!({"path":"public.txt"})),
            ] {
                match peer.call_tool(request(name, args)).await {
                    Err(rmcp::service::ServiceError::McpError(error))
                        if error.message == "optic.session_inactive" => {}
                    other => {
                        return Err(io::Error::other(format!(
                            "inactive authority accepted: {other:?}"
                        )));
                    }
                }
            }
        }
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
        "owned core did not exit cleanly after transport EOF"
    );
    result
        .expect("bounded supervision proof")
        .expect("actual stdio authority proof");
    let canonical = fs::canonicalize(&root).unwrap();
    assert_eq!(
        canonical.parent().unwrap(),
        fs::canonicalize(std::env::temp_dir()).unwrap()
    );
    assert!(
        canonical
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("optic-supervision-proof-")
    );
    fs::remove_dir_all(canonical).unwrap();
}

#[tokio::test]
async fn timed_mode_remains_finite_without_local_supervision() {
    exercise(false, false, true).await;
}
#[tokio::test]
async fn local_supervision_renews_same_scopes_and_revoke_stays_terminal() {
    exercise(true, true, true).await;
}
#[tokio::test]
async fn transport_eof_stops_an_active_local_supervisor_cleanly() {
    exercise(true, false, false).await;
}
