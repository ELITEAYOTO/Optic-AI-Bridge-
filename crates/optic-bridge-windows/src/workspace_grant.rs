use std::{
    ffi::c_void,
    fmt::Write as _,
    fs::{File, OpenOptions},
    io::{Error, Result},
    marker::PhantomData,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
    ptr,
};

use windows::{
    Win32::{
        Foundation::{HANDLE, HLOCAL, LocalFree},
        Security::{
            ACL,
            Authorization::{
                BuildTrusteeWithSidW, EXPLICIT_ACCESS_W, GRANT_ACCESS, GetSecurityInfo,
                REVOKE_ACCESS, SE_FILE_OBJECT, SetEntriesInAclW, SetSecurityInfo, TRUSTEE_W,
            },
            DACL_SECURITY_INFORMATION, DeriveCapabilitySidsFromName, PSECURITY_DESCRIPTOR, PSID,
            SID_AND_ATTRIBUTES,
        },
        Storage::FileSystem::{
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_GENERIC_READ, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
            FileAttributeTagInfo, GetFileInformationByHandleEx,
        },
        System::SystemServices::SE_GROUP_ENABLED,
    },
    core::HSTRING,
};

const WRITE_DAC_ACCESS: u32 = 0x0004_0000;

/// Ephemeral AppContainer capability used only for exact-file read grants.
///
/// The capability name is generated from cryptographic randomness and is never
/// persisted. A stale ACE left by an abrupt crash therefore names a capability
/// that later Optic launches do not reissue.
#[derive(Debug)]
pub struct AppContainerReadCapability {
    sid: PSID,
}

impl AppContainerReadCapability {
    pub fn create_ephemeral() -> Result<Self> {
        let mut random = [0u8; 16];
        getrandom::fill(&mut random)
            .map_err(|_| Error::other("Windows workspace capability entropy unavailable"))?;
        let mut name = String::from("Optic.Workspace.Read.");
        for byte in random {
            write!(&mut name, "{byte:02x}")
                .map_err(|_| Error::other("failed to format workspace capability name"))?;
        }
        Self::derive(&name)
    }

    fn derive(name: &str) -> Result<Self> {
        let name = HSTRING::from(name);
        let mut group_sids = ptr::null_mut::<PSID>();
        let mut group_count = 0u32;
        let mut capability_sids = ptr::null_mut::<PSID>();
        let mut capability_count = 0u32;

        // SAFETY: output pointers reference writable storage and the HSTRING
        // remains live for the synchronous call. Windows owns all returned
        // LocalAlloc buffers until this function wraps/frees them.
        unsafe {
            DeriveCapabilitySidsFromName(
                &name,
                &mut group_sids,
                &mut group_count,
                &mut capability_sids,
                &mut capability_count,
            )
        }
        .map_err(Error::other)?;

        let _groups = LocalSidArray::new(group_sids, group_count);
        let capabilities = LocalSidArray::new(capability_sids, capability_count);
        let sid = capabilities.into_single()?;
        Ok(Self { sid })
    }

    pub(crate) const fn sid_and_attributes(&self) -> SID_AND_ATTRIBUTES {
        SID_AND_ATTRIBUTES {
            Sid: self.sid,
            Attributes: SE_GROUP_ENABLED as u32,
        }
    }

    /// Grant this ephemeral capability read-only access to one exact file.
    ///
    /// The target is opened handle-first with reparse points denied. The ACE is
    /// non-inheritable and is revoked on explicit `revoke` or best-effort Drop.
    pub fn grant_file_read<'a>(&'a self, path: &Path) -> Result<AppContainerReadFileGrant<'a>> {
        AppContainerReadFileGrant::new(path, self)
    }
}

impl Drop for AppContainerReadCapability {
    fn drop(&mut self) {
        free_local_sid(self.sid);
    }
}

/// Handle-bound read-only grant for one exact file object.
#[derive(Debug)]
pub struct AppContainerReadFileGrant<'a> {
    file: File,
    sid: PSID,
    active: bool,
    _capability: PhantomData<&'a AppContainerReadCapability>,
}

impl<'a> AppContainerReadFileGrant<'a> {
    fn new(path: &Path, capability: &'a AppContainerReadCapability) -> Result<Self> {
        let mut options = OpenOptions::new();
        options
            .access_mode(FILE_GENERIC_READ.0 | WRITE_DAC_ACCESS)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0);
        let file = options.open(path)?;
        reject_reparse(&file)?;
        modify_file_dacl(&file, capability.sid, true)?;
        Ok(Self {
            file,
            sid: capability.sid,
            active: true,
            _capability: PhantomData,
        })
    }

    /// Revoke the exact capability ACE. Consuming self prevents accidental reuse.
    pub fn revoke(mut self) -> Result<()> {
        self.revoke_inner()
    }

    fn revoke_inner(&mut self) -> Result<()> {
        if self.active {
            modify_file_dacl(&self.file, self.sid, false)?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for AppContainerReadFileGrant<'_> {
    fn drop(&mut self) {
        let _ = self.revoke_inner();
    }
}

fn modify_file_dacl(file: &File, sid: PSID, grant: bool) -> Result<()> {
    let handle = HANDLE(file.as_raw_handle());
    let mut old_acl = ptr::null_mut::<ACL>();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: handle is a live exact-file handle opened with READ_CONTROL via
    // FILE_GENERIC_READ. Returned DACL aliases the returned LocalAlloc descriptor.
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut old_acl),
            None,
            Some(&mut descriptor),
        )
    };
    win32_status(status.0, "GetSecurityInfo")?;
    let _descriptor = LocalAllocation(descriptor.0);
    if old_acl.is_null() {
        return Err(Error::other(
            "workspace grant refuses a null or absent DACL",
        ));
    }

    let mut trustee = TRUSTEE_W::default();
    // SAFETY: SID is owned by the live capability and remains valid throughout.
    unsafe { BuildTrusteeWithSidW(&mut trustee, Some(sid)) };
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: if grant { FILE_GENERIC_READ.0 } else { 0 },
        grfAccessMode: if grant { GRANT_ACCESS } else { REVOKE_ACCESS },
        grfInheritance: Default::default(),
        Trustee: trustee,
    };

    let mut new_acl = ptr::null_mut::<ACL>();
    // SAFETY: old ACL remains live through `_descriptor`; entry and output
    // storage are valid for the synchronous merge call.
    let status = unsafe {
        SetEntriesInAclW(
            Some(std::slice::from_ref(&entry)),
            (!old_acl.is_null()).then_some(old_acl.cast_const()),
            &mut new_acl,
        )
    };
    win32_status(status.0, "SetEntriesInAclW")?;
    let _new_acl = LocalAllocation(new_acl.cast::<c_void>());

    // SAFETY: the exact file handle is live and was opened with WRITE_DAC;
    // `new_acl` stays allocated through the synchronous update.
    let status = unsafe {
        SetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(new_acl.cast_const()),
            None,
        )
    };
    win32_status(status.0, "SetSecurityInfo")
}

fn reject_reparse(file: &File) -> Result<()> {
    let handle = HANDLE(file.as_raw_handle());
    let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: handle is live and output storage has the exact Windows type/size.
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast::<c_void>(),
            u32::try_from(std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>())
                .expect("FILE_ATTRIBUTE_TAG_INFO size must fit u32"),
        )
    }
    .map_err(Error::other)?;
    if info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(Error::other(
            "workspace grant target cannot be a reparse point",
        ));
    }
    Ok(())
}

fn win32_status(code: u32, operation: &str) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(Error::other(format!(
            "{operation} failed with Win32 error {code}"
        )))
    }
}

#[derive(Debug)]
struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: pointer came from a Win32 LocalAlloc-returning API and is freed once.
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0)));
            }
        }
    }
}

#[derive(Debug)]
struct LocalSidArray {
    ptr: *mut PSID,
    len: u32,
}

impl LocalSidArray {
    const fn new(ptr: *mut PSID, len: u32) -> Self {
        Self { ptr, len }
    }

    fn into_single(mut self) -> Result<PSID> {
        if self.len != 1 || self.ptr.is_null() {
            return Err(Error::other(format!(
                "expected one derived capability SID, received {}",
                self.len
            )));
        }
        // SAFETY: DeriveCapabilitySidsFromName returned an array with one element.
        let sid = unsafe { *self.ptr };
        // Free only the outer pointer array; ownership of the single SID moves out.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.ptr.cast::<c_void>())));
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
        Ok(sid)
    }
}

impl Drop for LocalSidArray {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        for index in 0..self.len as usize {
            // SAFETY: pointer/count pair was returned by DeriveCapabilitySidsFromName.
            let sid = unsafe { *self.ptr.add(index) };
            free_local_sid(sid);
        }
        // SAFETY: outer pointer array was allocated by the same Win32 API.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.ptr.cast::<c_void>())));
        }
    }
}

fn free_local_sid(sid: PSID) {
    if !sid.0.is_null() {
        // SAFETY: derived capability SIDs are LocalAlloc buffers owned by this module.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(sid.0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs::{self, OpenOptions},
        os::windows::io::AsHandle,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use crate::{
        AppContainerProfile, AppContainerStdio, spawn_appcontainer_suspended_with_read_capability,
    };

    use super::*;

    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    const WAIT_MS: u32 = 10_000;

    fn unique_profile_name() -> String {
        format!(
            "Optic.Grant.Test.{}.{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn unique_temp_file() -> PathBuf {
        std::env::temp_dir().join(format!(
            "optic-appcontainer-read-grant-{}-{}.txt",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn system32_executable(name: &str) -> PathBuf {
        PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"))
            .join("System32")
            .join(name)
    }

    fn run_isolated(
        profile: &AppContainerProfile,
        capability: &AppContainerReadCapability,
        executable: &Path,
        args: Vec<OsString>,
    ) -> u32 {
        let cwd = executable.parent().expect("System32 parent");
        let stdin = OpenOptions::new()
            .read(true)
            .open("NUL")
            .expect("NUL stdin");
        let stdout = OpenOptions::new()
            .write(true)
            .open("NUL")
            .expect("NUL stdout");
        let stderr = OpenOptions::new()
            .write(true)
            .open("NUL")
            .expect("NUL stderr");
        let mut child = spawn_appcontainer_suspended_with_read_capability(
            profile,
            capability,
            executable,
            &args,
            cwd,
            AppContainerStdio {
                stdin: stdin.as_handle(),
                stdout: stdout.as_handle(),
                stderr: stderr.as_handle(),
            },
        )
        .expect("spawn capability AppContainer");
        assert!(child.is_appcontainer().expect("query AppContainer token"));
        child.resume().expect("resume AppContainer");
        child.wait_exit(WAIT_MS).expect("wait AppContainer")
    }

    fn read_exit(
        profile: &AppContainerProfile,
        capability: &AppContainerReadCapability,
        file: &Path,
    ) -> u32 {
        let findstr = system32_executable("findstr.exe");
        run_isolated(
            profile,
            capability,
            &findstr,
            vec![
                OsString::from("/c:optic read grant sentinel"),
                file.as_os_str().to_os_string(),
            ],
        )
    }

    #[test]
    fn ephemeral_read_grant_is_exact_read_only_and_revocable() {
        let sentinel = unique_temp_file();
        fs::write(&sentinel, b"optic read grant sentinel\n").expect("write sentinel");
        let _cleanup = Cleanup(sentinel.clone());
        let ungranted = unique_temp_file();
        fs::write(&ungranted, b"optic read grant sentinel\n").expect("write ungranted sentinel");
        let _ungranted_cleanup = Cleanup(ungranted.clone());

        let capability = AppContainerReadCapability::create_ephemeral().expect("derive capability");
        let profile =
            AppContainerProfile::create_with_read_capability(&unique_profile_name(), &capability)
                .expect("create profile with read capability");

        assert_ne!(
            read_exit(&profile, &capability, &sentinel),
            0,
            "capability without an ACL grant must not read the file"
        );

        let grant = capability
            .grant_file_read(&sentinel)
            .expect("grant exact-file read");
        assert_eq!(
            read_exit(&profile, &capability, &sentinel),
            0,
            "read capability plus exact ACL grant must read the file"
        );
        assert_ne!(
            read_exit(&profile, &capability, &ungranted),
            0,
            "the capability must not read a second file without its own ACL grant"
        );

        let cmd = system32_executable("cmd.exe");
        let append_command = format!("echo forbidden-write>>\"{}\"", sentinel.to_string_lossy());
        assert_ne!(
            run_isolated(
                &profile,
                &capability,
                &cmd,
                vec![
                    OsString::from("/d"),
                    OsString::from("/s"),
                    OsString::from("/c"),
                    OsString::from(append_command),
                ],
            ),
            0,
            "read-only capability must not permit file mutation"
        );
        assert_eq!(
            fs::read_to_string(&sentinel).expect("read sentinel after write attempt"),
            "optic read grant sentinel\n"
        );

        grant.revoke().expect("revoke exact-file grant");
        assert_ne!(
            read_exit(&profile, &capability, &sentinel),
            0,
            "the same capability must lose access after its ACE is revoked"
        );

        {
            let _drop_revoke = capability
                .grant_file_read(&sentinel)
                .expect("grant exact-file read for Drop cleanup");
            assert_eq!(
                read_exit(&profile, &capability, &sentinel),
                0,
                "Drop-cleanup grant must be usable while its guard is live"
            );
        }
        assert_ne!(
            read_exit(&profile, &capability, &sentinel),
            0,
            "dropping the grant guard must revoke the capability ACE"
        );
    }

    struct Cleanup(PathBuf);

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
}
