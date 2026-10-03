#![forbid(unsafe_code)]

//! MCP adapter for Optic AI Bridge.
//!
//! Protocol handling remains outside the core/runtime authorization boundary.
//! Public MCP inputs are normalized and authorized before runtime services are
//! invoked, and stdio JSON-RPC framing is bounded independently of RMCP defaults.

mod process_tools;
mod server;
mod transport;

pub use process_tools::{
    ProcessJobRequest, ProcessReadRequest, ProcessReadResponse, ProcessResultResponse,
    ProcessStartRequest, ProcessStartResponse, ProcessStopResponse, ProcessStreamRequest,
    SessionCancelResponse,
};
pub use server::{
    FsListEntry, FsListRequest, FsListResponse, FsReadRequest, FsReadResponse, ReadonlyMcpServer,
    ServerBuildError, SessionInfoResponse,
};
pub use transport::{BoundedJsonLineTransport, BoundedTransportError};
