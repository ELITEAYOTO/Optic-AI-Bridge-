//! Attest the actual retained handle's synchronous mode before std File IO.
use std::{
    io,
    os::windows::io::{AsHandle, AsRawHandle},
};
use windows::{
    Wdk::Storage::FileSystem::{
        FILE_MODE_INFORMATION, FILE_SYNCHRONOUS_IO_ALERT, FILE_SYNCHRONOUS_IO_NONALERT,
        FileModeInformation, NtQueryInformationFile,
    },
    Win32::{Foundation::HANDLE, System::IO::IO_STATUS_BLOCK},
};

pub(crate) fn require_synchronous(handle: &impl AsHandle) -> io::Result<()> {
    let mut mode = FILE_MODE_INFORMATION::default();
    let mut status_block = IO_STATUS_BLOCK::default();
    // SAFETY: FileModeInformation is a synchronous metadata query of this live
    // owned handle. Both initialized, correctly sized result buffers remain
    // exclusively borrowed through the call. No ReadFile/WriteFile is submitted.
    let status = unsafe {
        NtQueryInformationFile(
            HANDLE(handle.as_handle().as_raw_handle()),
            &mut status_block,
            std::ptr::from_mut(&mut mode).cast(),
            std::mem::size_of::<FILE_MODE_INFORMATION>() as u32,
            FileModeInformation,
        )
    };
    if status.0 != 0 {
        return Err(io::Error::other(format!(
            "cannot establish stdio handle mode: {status:?}"
        )));
    }
    if mode.Mode & (FILE_SYNCHRONOUS_IO_ALERT.0 | FILE_SYNCHRONOUS_IO_NONALERT.0) == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "owned stdio requires a synchronous handle; overlapped IO is unsupported",
        ));
    }
    Ok(())
}
