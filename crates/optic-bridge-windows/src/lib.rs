#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(windows), forbid(unsafe_code))]

//! Narrow Windows-only platform primitives.
//!
//! This crate is the only project layer allowed to call the Win32 APIs needed
//! for Job Object containment and handle-first filesystem mutation checks.
//! Core, policy, MCP and the cross-platform runtime remain `unsafe`-free.

#[cfg(windows)]
mod file;
#[cfg(windows)]
mod job;

#[cfg(windows)]
pub use file::{
    OpenedWindowsFile, WindowsFileError, WindowsFileIdentity, WindowsPathIdentity,
    inspect_directory_no_reparse, open_file_no_reparse, replace_file_atomically,
};
#[cfg(windows)]
pub use job::LimitedJobObject;
