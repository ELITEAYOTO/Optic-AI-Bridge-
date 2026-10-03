use std::sync::Arc;

use futures::StreamExt;
use rmcp::{
    RoleServer,
    service::{RxJsonRpcMessage, TxJsonRpcMessage},
    transport::{Transport, async_rw::JsonRpcMessageCodec},
};
use serde::Serialize;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};
use tokio_util::codec::FramedRead;

pub struct BoundedJsonLineTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    read: FramedRead<R, JsonRpcMessageCodec<RxJsonRpcMessage<RoleServer>>>,
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
            read: FramedRead::new(
                read,
                JsonRpcMessageCodec::<RxJsonRpcMessage<RoleServer>>::new_with_max_length(
                    max_request_bytes,
                ),
            ),
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
            let writer = guard
                .as_mut()
                .ok_or(BoundedTransportError::NotConnected)?;
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
            Some(Ok(message)) => Some(message),
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
            writer
                .shutdown()
                .await
                .map_err(BoundedTransportError::Io)?;
        }
        Ok(())
    }
}

fn serialize_json_line_bounded<T: Serialize>(
    value: &T,
    max_bytes: usize,
) -> Result<Vec<u8>, BoundedTransportError> {
    let mut bytes = serde_json::to_vec(value).map_err(BoundedTransportError::Serialization)?;
    let encoded_len = bytes
        .len()
        .checked_add(1)
        .ok_or(BoundedTransportError::ResponseTooLarge)?;
    if encoded_len > max_bytes {
        return Err(BoundedTransportError::ResponseTooLarge);
    }
    bytes.push(b'\n');
    Ok(bytes)
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
