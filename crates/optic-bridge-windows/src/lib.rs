#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(windows), forbid(unsafe_code))]

//! Narrow Windows-only containment primitives.
//!
//! This crate is the only Phase 1 layer allowed to call Win32 Job Object APIs.
//! Core, policy, MCP and the cross-platform runtime remain `unsafe`-free.

#[cfg(windows)]
mod job;

#[cfg(windows)]
pub use job::LimitedJobObject;
