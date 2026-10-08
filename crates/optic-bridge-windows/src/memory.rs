use std::{
    io::{Error, ErrorKind, Result},
    mem::size_of,
};

use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

/// Point-in-time physical-memory observation from the Windows host.
///
/// This is telemetry only. It does not reserve memory, modify process admission,
/// or imply control over memory consumed by unrelated host processes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostMemorySnapshot {
    pub total_physical_bytes: u64,
    pub available_physical_bytes: u64,
}

/// Reads the current physical-memory totals reported by Windows.
///
/// The returned snapshot is validated so later policy layers do not need to
/// handle impossible zero-total or available-greater-than-total observations.
pub fn query_host_memory_snapshot() -> Result<HostMemorySnapshot> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };

    // SAFETY: `status` is a live writable `MEMORYSTATUSEX` whose `dwLength`
    // field is initialized to the exact structure size required by Win32.
    unsafe { GlobalMemoryStatusEx(&mut status) }.map_err(Error::other)?;

    if status.ullTotalPhys == 0 || status.ullAvailPhys > status.ullTotalPhys {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Windows reported an inconsistent physical-memory snapshot",
        ));
    }

    Ok(HostMemorySnapshot {
        total_physical_bytes: status.ullTotalPhys,
        available_physical_bytes: status.ullAvailPhys,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_memory_snapshot_is_physically_consistent() {
        let snapshot = query_host_memory_snapshot().expect("query host memory");
        assert!(snapshot.total_physical_bytes > 0);
        assert!(snapshot.available_physical_bytes <= snapshot.total_physical_bytes);
    }
}
