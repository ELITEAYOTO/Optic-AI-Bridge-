#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(windows), forbid(unsafe_code))]

//! Narrow Windows-only containment primitives.
//!
//! This crate is the only layer allowed to call the Win32 APIs used for
//! process containment and mutation-time filesystem handle validation.
//! Core, policy, MCP and the cross-platform runtime remain `unsafe`-free.

#[cfg(windows)]
mod job;
#[cfg(windows)]
mod mutation;

#[cfg(windows)]
pub use job::LimitedJobObject;
#[cfg(windows)]
pub use mutation::{
    HandleValidatedDirectory, HandleValidatedFile, WindowsFileIdentity, WindowsMutationHandleError,
};
