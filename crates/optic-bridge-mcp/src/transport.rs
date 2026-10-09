use std::sync::Arc;

use futures::StreamExt;
use rmcp::{
    RoleServer,
    service::{RxJsonRpcMessage, TxJsonRpcMessage},
    transport::Transport,
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};
use tokio_util::codec::{FramedRead, LinesCodec};

pub struct BoundedJsonLineTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    read: FramedRead<R, LinesCodec>,
    write: Arc<Mutex<Option<W>>>,
    max_response_bytes: usize,
    receive_failed: bool,
}

impl<R, W> BoundedJsonLineTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub fn new(
        read: R,
        write: W,
        max_request_bytes: usize,
        max_response_bytes: usize,
    ) -> Result<Self, BoundedTransportError> {
        if max_request_bytes == 0 || max_response_bytes == 0 {
            return Err(BoundedTransportError::InvalidLimits);
        }

        Ok(Self {
            read: FramedRead::new(read, LinesCodec::new_with_max_length(max_request_bytes)),
            write: Arc::new(Mutex::new(Some(write))),
            max_response_bytes,
            receive_failed: false,
        })
    }

    #[must_use]
    pub const fn receive_failed(&self) -> bool {
        self.receive_failed
    }
}

impl<R, W> Transport<RoleServer> for BoundedJsonLineTransport<R, W>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    type Error = BoundedTransportError;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let write = Arc::clone(&self.write);
        let max_response_bytes = self.max_response_bytes;
        async move {
            let encoded = serialize_json_line_bounded(&item, max_response_bytes)?;
            let mut guard = write.lock().await;
            let writer = guard.as_mut().ok_or(BoundedTransportError::NotConnected)?;
            writer
                .write_all(&encoded)
                .await
                .map_err(BoundedTransportError::Io)?;
            writer.flush().await.map_err(BoundedTransportError::Io)
        }
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
        if self.receive_failed {
            return None;
        }

        match self.read.next().await {
            Some(Ok(line)) => match decode_strict_jsonrpc_line(&line) {
                Ok(message) => Some(message),
                Err(()) => {
                    self.receive_failed = true;
                    None
                }
            },
            Some(Err(_)) => {
                self.receive_failed = true;
                None
            }
            None => None,
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        let mut guard = self.write.lock().await;
        if let Some(mut writer) = guard.take() {
            writer.shutdown().await.map_err(BoundedTransportError::Io)?;
        }
        Ok(())
    }
}

/// Decode one already-bounded JSON line only after Optic has rejected JSON-RPC
/// shapes whose fields are mutually exclusive by specification.
///
/// RMCP 3.4.0 uses an untagged enum for JsonRpcMessage. Without this preflight,
/// a response containing both `result` and `error` can deserialize as a success
/// while silently ignoring the error field. Optic therefore validates field
/// presence before handing the value to RMCP. Unknown extension fields remain
/// allowed and all variant-specific validation stays owned by RMCP.
fn decode_strict_jsonrpc_line(line: &str) -> Result<RxJsonRpcMessage<RoleServer>, ()> {
    let value: Value = serde_json::from_str(line).map_err(|_| ())?;
    validate_jsonrpc_shape(&value)?;
    serde_json::from_value(value).map_err(|_| ())
}

fn validate_jsonrpc_shape(value: &Value) -> Result<(), ()> {
    let object = value.as_object().ok_or(())?;
    let has_method = object.contains_key("method");
    let has_result = object.contains_key("result");
    let has_error = object.contains_key("error");

    if has_result && has_error {
        return Err(());
    }
    if has_method && (has_result || has_error) {
        return Err(());
    }
    if !has_method && !has_result && !has_error {
        return Err(());
    }
    Ok(())
}

struct BoundedJsonBuffer {
    bytes: Vec<u8>,
    max_bytes: usize,
    overflowed: bool,
}

impl BoundedJsonBuffer {
    fn new(max_bytes: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(max_bytes.min(8 * 1024)),
            max_bytes,
            overflowed: false,
        }
    }
}

impl std::io::Write for BoundedJsonBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let remaining = self.max_bytes.saturating_sub(self.bytes.len());
        if buf.len() > remaining {
            self.overflowed = true;
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "bounded JSON response buffer exhausted",
            ));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn serialize_json_line_bounded<T: Serialize>(
    value: &T,
    max_bytes: usize,
) -> Result<Vec<u8>, BoundedTransportError> {
    let payload_limit = max_bytes
        .checked_sub(1)
        .ok_or(BoundedTransportError::ResponseTooLarge)?;
    let mut writer = BoundedJsonBuffer::new(payload_limit);
    let result = serde_json::to_writer(&mut writer, value);
    if writer.overflowed {
        return Err(BoundedTransportError::ResponseTooLarge);
    }
    result.map_err(BoundedTransportError::Serialization)?;
    writer.bytes.push(b'\n');
    Ok(writer.bytes)
}

#[derive(Debug, Error)]
pub enum BoundedTransportError {
    #[error("MCP transport byte limits must be non-zero")]
    InvalidLimits,
    #[error("MCP response exceeds the Optic hard response limit")]
    ResponseTooLarge,
    #[error("MCP transport is not connected")]
    NotConnected,
    #[error("failed to serialize MCP response: {0}")]
    Serialization(serde_json::Error),
    #[error("MCP stdio I/O failed: {0}")]
    Io(std::io::Error),
}

#[cfg(test)]
mod tests {
    use rmcp::transport::Transport;
    use tokio::io::{AsyncWriteExt, duplex, split};

    use super::*;

    #[test]
    fn response_serializer_counts_json_line_delimiter() {
        let value = serde_json::json!({"x":"1234"});
        let exact = serde_json::to_vec(&value).expect("serialize fixture").len() + 1;
        assert!(serialize_json_line_bounded(&value, exact).is_ok());
        assert!(matches!(
            serialize_json_line_bounded(&value, exact - 1),
            Err(BoundedTransportError::ResponseTooLarge)
        ));
    }

    #[test]
    fn response_serializer_stops_allocating_at_payload_limit() {
        let value = serde_json::json!({"x": "x".repeat(4096)});
        assert!(matches!(
            serialize_json_line_bounded(&value, 64),
            Err(BoundedTransportError::ResponseTooLarge)
        ));

        let mut writer = BoundedJsonBuffer::new(32);
        let result = serde_json::to_writer(&mut writer, &value);
        assert!(result.is_err());
        assert!(writer.overflowed);
        assert!(writer.bytes.len() <= 32);
    }

    #[test]
    fn strict_jsonrpc_shape_rejects_conflicting_result_error_or_method_fields() {
        for value in [
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {},
                "error": {"code": -32603, "message": "injected"}
            }),
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "ping",
                "result": {}
            }),
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "error": {"code": -32603, "message": "injected"}
            }),
        ] {
            assert!(validate_jsonrpc_shape(&value).is_err());
        }
    }

    #[test]
    fn strict_jsonrpc_shape_preserves_clean_shapes_and_extension_fields() {
        for value in [
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"ping","x-extra":true}),
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized","x-extra":true}),
            serde_json::json!({"jsonrpc":"2.0","id":1,"result":{},"x-extra":true}),
            serde_json::json!({
                "jsonrpc":"2.0",
                "id":1,
                "error":{"code":-32603,"message":"boom"},
                "x-extra":true
            }),
        ] {
            validate_jsonrpc_shape(&value).expect("clean JSON-RPC shape");
        }
    }

    #[tokio::test]
    async fn conflicting_jsonrpc_response_closes_receive_side_before_rmcp_deserialization() {
        let (mut peer, transport_side) = duplex(1024);
        let (read, write) = split(transport_side);
        let mut transport =
            BoundedJsonLineTransport::new(read, write, 512, 512).expect("valid transport");

        peer.write_all(
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{},\"error\":{\"code\":-32603,\"message\":\"injected\"}}\n",
        )
        .await
        .expect("write fixture");

        assert!(transport.receive().await.is_none());
        assert!(transport.receive_failed());
    }

    #[tokio::test]
    async fn clean_request_with_extension_field_reaches_rmcp_decoder() {
        let (mut peer, transport_side) = duplex(1024);
        let (read, write) = split(transport_side);
        let mut transport =
            BoundedJsonLineTransport::new(read, write, 512, 512).expect("valid transport");

        peer.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\",\"x-extra\":true}\n")
            .await
            .expect("write fixture");

        assert!(transport.receive().await.is_some());
        assert!(!transport.receive_failed());
    }

    #[tokio::test]
    async fn oversized_request_line_closes_receive_side() {
        let (mut peer, transport_side) = duplex(256);
        let (read, write) = split(transport_side);
        let mut transport =
            BoundedJsonLineTransport::new(read, write, 16, 128).expect("valid transport");

        peer.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n")
            .await
            .expect("write fixture");

        assert!(transport.receive().await.is_none());
        assert!(transport.receive_failed());
    }
}
