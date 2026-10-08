use optic_bridge_mcp::{
    ExpectedStateRequest, FsApplyPatchRequest, FsDeleteRequest, FsListRequest, FsReadRequest,
    FsWriteRequest, GitDiffRequest, GitLogCursorRequest, GitLogRequest, ProcessJobRequest,
    ProcessReadRequest, ProcessStartRequest,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

fn rejects_unknown_field<T: DeserializeOwned>(value: Value) {
    assert!(
        serde_json::from_value::<T>(value).is_err(),
        "public MCP request schema accepted an unknown field: {}",
        std::any::type_name::<T>()
    );
}

#[test]
fn public_mcp_request_objects_reject_unknown_fields() {
    rejects_unknown_field::<FsReadRequest>(json!({
        "path": "input.txt",
        "unexpected": true
    }));
    rejects_unknown_field::<FsListRequest>(json!({
        "path": null,
        "unexpected": true
    }));
    rejects_unknown_field::<ProcessStartRequest>(json!({
        "executable": "C:/tool.exe",
        "unexpected": true
    }));
    rejects_unknown_field::<ProcessReadRequest>(json!({
        "job_id": "job",
        "stream": "stdout",
        "cursor": 0,
        "max_bytes": 1,
        "unexpected": true
    }));
    rejects_unknown_field::<ProcessJobRequest>(json!({
        "job_id": "job",
        "unexpected": true
    }));
    rejects_unknown_field::<FsWriteRequest>(json!({
        "path": "output.txt",
        "content_base64": "",
        "expected": { "kind": "absent" },
        "unexpected": true
    }));
    rejects_unknown_field::<ExpectedStateRequest>(json!({
        "kind": "absent",
        "unexpected": true
    }));
    rejects_unknown_field::<FsApplyPatchRequest>(json!({
        "path": "output.txt",
        "expected_version": "00",
        "offset": 0,
        "remove_bytes": 0,
        "insert_base64": "",
        "unexpected": true
    }));
    rejects_unknown_field::<FsDeleteRequest>(json!({
        "path": "output.txt",
        "expected_version": "00",
        "unexpected": true
    }));
    rejects_unknown_field::<GitDiffRequest>(json!({
        "path": null,
        "staged": false,
        "max_bytes": 1,
        "unexpected": true
    }));
    rejects_unknown_field::<GitLogRequest>(json!({
        "cursor": null,
        "limit": 1,
        "unexpected": true
    }));
    rejects_unknown_field::<GitLogRequest>(json!({
        "cursor": {
            "head": "0123456789012345678901234567890123456789",
            "offset": 0,
            "unexpected": true
        },
        "limit": 1
    }));
    rejects_unknown_field::<GitLogCursorRequest>(json!({
        "head": "0123456789012345678901234567890123456789",
        "offset": 0,
        "unexpected": true
    }));
}
