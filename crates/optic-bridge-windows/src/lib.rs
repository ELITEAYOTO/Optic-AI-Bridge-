#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(windows), forbid(unsafe_code))]

//! Narrow Windows-only platform primitives.
//!
//! This crate is the only project layer allowed to call the Win32 APIs needed
//! for Job Object containment, AppContainer isolation primitives and handle-first
//! filesystem mutation checks. Core, policy, MCP and the cross-platform runtime
//! remain `unsafe`-free.

#[cfg(windows)]
mod appcontainer;
#[cfg(all(windows, test))]
mod debug_probe;
#[cfg(windows)]
mod directory;
#[cfg(windows)]
mod executable;
#[cfg(windows)]
mod file;
#[cfg(windows)]
mod job;
#[cfg(windows)]
mod memory;
#[cfg(windows)]
mod output_pipe;
#[cfg(all(windows, test))]
mod rust_compile_probe;
#[cfg(windows)]
mod workspace_grant;

#[cfg(windows)]
pub use appcontainer::{
    AppContainerProfile, AppContainerSecurityCapabilities, AppContainerStdio,
    EphemeralAppContainerPlan, SuspendedAppContainerProcess, spawn_appcontainer_suspended,
    spawn_appcontainer_with_environment_suspended,
};
#[cfg(windows)]
pub use directory::{PinnedDirectoryChain, PinnedDirectoryError, pin_directory_chain};
#[cfg(windows)]
pub use executable::{PinnedExecutableFile, open_pinned_executable};
#[cfg(windows)]
pub use file::{
    OpenedWindowsDeleteFile, OpenedWindowsFile, WindowsFileError, WindowsFileIdentity,
    WindowsPathIdentity, inspect_directory_no_reparse, open_exclusive_state_file,
    open_file_for_delete_no_reparse, open_file_no_reparse, replace_file_atomically,
};
#[cfg(windows)]
pub use job::{ContainedAppContainerProcess, LimitedJobObject};
#[cfg(windows)]
pub use memory::{HostMemorySnapshot, query_host_memory_snapshot};
#[cfg(windows)]
pub use output_pipe::{CapturedOutputChunk, CapturedOutputPipe};
#[cfg(windows)]
pub use workspace_grant::{AppContainerDirectoryMetadataGrant, AppContainerReadFileGrant};
