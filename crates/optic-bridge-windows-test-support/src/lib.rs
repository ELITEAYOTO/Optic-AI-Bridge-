//! Research-only FFI fixtures, linked as a Windows DEV-dependency by runtime tests.
//! Not shipped by optic-bridge-app; no arbitrary executable or permission API.
//! Every effect requires the externally gated synthetic test host protocol.
#![cfg(windows)]
mod native;
pub use native::{Native, StopWatch};
pub const MEMORY: u64 = 256 * 1024 * 1024;
pub const OUTPUT: usize = 16 * 1024;
pub const CPU: u32 = 25;
