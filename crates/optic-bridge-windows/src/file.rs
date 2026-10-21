use std::{
    ffi::{OsString, c_void},
    fs::{File, OpenOptions},
    io,
    mem::size_of,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};

use thiserror::Error;
use windows::{
    Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            DELETE, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_DISPOSITION_INFO,
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
            FILE_ID_INFO, FILE_SHARE_READ, FILE_STANDARD_INFO, FileAttributeTagInfo,
            FileDispositionInfo, FileIdInfo, FileStandardInfo, GetFileInformationByHandleEx,
            GetFinalPathNameByHandleW, MOVE_FILE_FLAGS, MoveFileExW, REPLACE_FILE_FLAGS,
            ReplaceFileW, SetFileInformationByHandle, VOLUME_NAME_DOS,
        },
    },
    core::{Error as WindowsError, PCWSTR},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowsFileIdentity {
    pub volume_serial_number: u64,
    pub file_id: [u8; 16],
}

#[derive(Debug)]
pub struct OpenedWindowsFile {
    file: File,
    identity: WindowsFileIdentity,
    final_path: PathBuf,
}

/// Host-owned state only: exclusive read/write handle, no final reparse target,
/// no truncation. Caller pins its parent and compares the returned final path.
pub fn open_exclusive_state_file(
    path: &Path,
    create_new: bool,
) -> Result<OpenedWindowsFile, WindowsFileError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(create_new)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    finish_exclusive_file(file)
}

/// Exclusive existing regular file, with read access only. Never creates or
/// truncates. The caller must pin/authorize its parent and check final identity.
/// This primitive grants no workspace, process or artifact-publication authority.
pub fn open_exclusive_read_file(path: &Path) -> Result<OpenedWindowsFile, WindowsFileError> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    finish_exclusive_file(file)
}

fn finish_exclusive_file(file: File) -> Result<OpenedWindowsFile, WindowsFileError> {
    reject_reparse(&file)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("state target must be a file").into());
    }
    let mut standard = FILE_STANDARD_INFO::default();
    // SAFETY: exclusive live file and an exact native writable output buffer.
    unsafe {
        GetFileInformationByHandleEx(
            file_handle(&file),
            FileStandardInfo,
            (&mut standard as *mut FILE_STANDARD_INFO).cast(),
            size_of::<FILE_STANDARD_INFO>() as u32,
        )?;
    }
    if standard.NumberOfLinks != 1 {
        return Err(io::Error::other("state file must not have hard-link aliases").into());
    }
    let identity = query_identity(&file)?;
    let final_path = query_final_path(&file)?;
    Ok(OpenedWindowsFile {
        file,
        identity,
        final_path,
    })
}

impl OpenedWindowsFile {
    #[must_use]
    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    #[must_use]
    pub const fn identity(&self) -> WindowsFileIdentity {
        self.identity
    }

    /// Number of names sharing the opened file's native identity. Read from
    /// this pinned handle, never from a second pathname lookup.
    pub fn has_multiple_links(&self) -> Result<bool, WindowsFileError> {
        let mut standard = FILE_STANDARD_INFO::default();
        // SAFETY: the live handle and the exact native buffer are valid.
        unsafe {
            GetFileInformationByHandleEx(
                file_handle(&self.file),
                FileStandardInfo,
                (&mut standard as *mut FILE_STANDARD_INFO).cast(),
                size_of::<FILE_STANDARD_INFO>() as u32,
            )?;
        }
        Ok(standard.NumberOfLinks != 1)
    }

    #[must_use]
    pub fn final_path(&self) -> &Path {
        &self.final_path
    }
}

#[derive(Debug)]
pub struct OpenedWindowsDeleteFile {
    file: File,
    identity: WindowsFileIdentity,
    final_path: PathBuf,
}

impl OpenedWindowsDeleteFile {
    #[must_use]
    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    #[must_use]
    pub const fn identity(&self) -> WindowsFileIdentity {
        self.identity
    }

    #[must_use]
    pub fn final_path(&self) -> &Path {
        &self.final_path
    }

    pub fn delete(self) -> Result<(), WindowsFileError> {
        let Self { file, .. } = self;
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        unsafe {
            SetFileInformationByHandle(
                file_handle(&file),
                FileDispositionInfo,
                (&raw const disposition).cast::<c_void>(),
                u32::try_from(size_of::<FILE_DISPOSITION_INFO>())
                    .expect("FILE_DISPOSITION_INFO size must fit u32"),
            )?;
        }
        // FILE_DISPOSITION_INFO marks the opened file for deletion when the
        // handle closes. Close it before returning so callers can verify the
        // namespace state immediately after this primitive succeeds.
        drop(file);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsPathIdentity {
    pub identity: WindowsFileIdentity,
    pub final_path: PathBuf,
}

pub fn open_file_no_reparse(path: &Path) -> Result<OpenedWindowsFile, WindowsFileError> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0);
    let file = options.open(path)?;
    reject_reparse(&file)?;
    let identity = query_identity(&file)?;
    let final_path = query_final_path(&file)?;
    Ok(OpenedWindowsFile {
        file,
        identity,
        final_path,
    })
}

pub fn open_file_for_delete_no_reparse(
    path: &Path,
) -> Result<OpenedWindowsDeleteFile, WindowsFileError> {
    let mut options = OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ.0 | DELETE.0)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0);
    let file = options.open(path)?;
    reject_reparse(&file)?;
    let identity = query_identity(&file)?;
    let final_path = query_final_path(&file)?;
    Ok(OpenedWindowsDeleteFile {
        file,
        identity,
        final_path,
    })
}

/// Host-owned deletion of a validated state record. The same handle supports
/// bounded read/verification and deletion; no other opener may replace or
/// modify it between the exact-version precondition and delete. Hard links,
/// reparse points and non-regular files fail closed.
pub fn open_exclusive_file_for_delete_no_reparse(
    path: &Path,
) -> Result<OpenedWindowsDeleteFile, WindowsFileError> {
    let file = OpenOptions::new()
        .access_mode(FILE_GENERIC_READ.0 | DELETE.0)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    let checked = finish_exclusive_file(file)?;
    Ok(OpenedWindowsDeleteFile {
        file: checked.file,
        identity: checked.identity,
        final_path: checked.final_path,
    })
}

pub fn inspect_directory_no_reparse(path: &Path) -> Result<WindowsPathIdentity, WindowsFileError> {
    let mut options = OpenOptions::new();
    options
        .access_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0 | FILE_FLAG_BACKUP_SEMANTICS.0);
    let file = options.open(path)?;
    reject_reparse(&file)?;
    Ok(WindowsPathIdentity {
        identity: query_identity(&file)?,
        final_path: query_final_path(&file)?,
    })
}

/// Publish a prepared same-directory journal without replacing a destination
/// that may have appeared since its last absence check. No copy fallback:
/// publication must remain one namespace operation or fail closed.
/// This does not pin the parent directory or guarantee crash-time directory fsync.
pub fn move_file_no_replace(from: &Path, to: &Path) -> Result<(), WindowsFileError> {
    let from = wide_null(from);
    let to = wide_null(to);
    // SAFETY: both Windows path buffers are terminated and live for the call;
    // zero flags intentionally omit MOVEFILE_REPLACE_EXISTING/COPY_ALLOWED.
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVE_FILE_FLAGS(0),
        )?
    };
    Ok(())
}

pub fn replace_file_atomically(
    replaced_path: &Path,
    replacement_path: &Path,
) -> Result<(), WindowsFileError> {
    let replaced = wide_null(replaced_path);
    let replacement = wide_null(replacement_path);

    unsafe {
        ReplaceFileW(
            PCWSTR(replaced.as_ptr()),
            PCWSTR(replacement.as_ptr()),
            PCWSTR::null(),
            REPLACE_FILE_FLAGS(0),
            None,
            None,
        )?;
    }
    Ok(())
}

pub(crate) fn reject_reparse(file: &File) -> Result<(), WindowsFileError> {
    let handle = file_handle(file);
    let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast::<c_void>(),
            u32::try_from(size_of::<FILE_ATTRIBUTE_TAG_INFO>())
                .expect("FILE_ATTRIBUTE_TAG_INFO size must fit u32"),
        )?;
    }
    if info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(WindowsFileError::ReparsePointDenied);
    }
    Ok(())
}

fn query_identity(file: &File) -> Result<WindowsFileIdentity, WindowsFileError> {
    let handle = file_handle(file);
    let mut info = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast::<c_void>(),
            u32::try_from(size_of::<FILE_ID_INFO>()).expect("FILE_ID_INFO size must fit u32"),
        )?;
    }
    Ok(WindowsFileIdentity {
        volume_serial_number: info.VolumeSerialNumber,
        file_id: info.FileId.Identifier,
    })
}

pub(crate) fn query_final_path(file: &File) -> Result<PathBuf, WindowsFileError> {
    let handle = file_handle(file);
    let mut buffer = vec![0_u16; 512];

    loop {
        let length = unsafe { GetFinalPathNameByHandleW(handle, &mut buffer, VOLUME_NAME_DOS) };
        if length == 0 {
            return Err(WindowsFileError::Windows(WindowsError::from_thread()));
        }
        let length = usize::try_from(length).map_err(|_| WindowsFileError::PathTooLong)?;
        if length < buffer.len() {
            buffer.truncate(length);
            return Ok(PathBuf::from(OsString::from_wide(&buffer)));
        }
        let required = length.checked_add(1).ok_or(WindowsFileError::PathTooLong)?;
        buffer.resize(required, 0);
    }
}

fn file_handle(file: &File) -> HANDLE {
    HANDLE(file.as_raw_handle())
}

fn wide_null(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[derive(Debug, Error)]
pub enum WindowsFileError {
    #[error("Windows filesystem operation failed: {0}")]
    Windows(#[from] WindowsError),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] io::Error),
    #[error("reparse-point mutation target denied")]
    ReparsePointDenied,
    #[error("resolved Windows path is too long to represent safely")]
    PathTooLong,
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    fn workspace(label: &str) -> PathBuf {
        let root = env::temp_dir().join(format!(
            "optic-bridge-windows-file-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture");
        root
    }

    #[test]
    fn exclusive_state_file_never_truncates_and_rejects_aliases() {
        use std::io::{Read, Write};
        let root = workspace("state-lock");
        let path = root.join("state");
        let mut first = open_exclusive_state_file(&path, true).unwrap();
        first.file_mut().write_all(b"persisted").unwrap();
        first.file_mut().sync_all().unwrap();
        assert!(open_exclusive_state_file(&path, false).is_err());
        drop(first);
        let mut reopened = open_exclusive_state_file(&path, false).unwrap();
        let mut bytes = Vec::new();
        reopened.file_mut().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"persisted");
        drop(reopened);
        fs::hard_link(&path, root.join("alias")).unwrap();
        assert!(open_exclusive_state_file(&path, false).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exclusive_read_file_needs_no_write_right_and_never_creates() {
        use std::io::{Read, Write};
        let root = workspace("exclusive-read");
        let path = root.join("artifact.bin");
        let absent = root.join("missing.bin");
        assert!(open_exclusive_read_file(&absent).is_err());
        assert!(!absent.exists());
        assert!(open_exclusive_read_file(&root).is_err());
        fs::write(&path, b"fixed output").unwrap();
        let observed = open_file_no_reparse(&path).unwrap();
        let identity = observed.identity();
        let final_path = observed.final_path().to_owned();
        drop(observed);
        let original_permissions = fs::metadata(&path).unwrap().permissions();
        let mut permissions = original_permissions.clone();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions.clone()).unwrap();
        // Existing state read/write opening is intentionally stronger.
        assert!(open_exclusive_state_file(&path, false).is_err());
        let mut file = open_exclusive_read_file(&path).unwrap();
        assert_eq!(file.identity(), identity);
        assert_eq!(file.final_path(), final_path);
        let mut bytes = Vec::new();
        file.file_mut().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"fixed output");
        assert!(file.file_mut().write_all(b"corrupt").is_err());
        assert!(open_exclusive_read_file(&path).is_err());
        assert!(fs::rename(&path, root.join("moved.bin")).is_err());
        drop(file);

        fs::set_permissions(&path, original_permissions).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"fixed output");
        let writer = OpenOptions::new().write(true).open(&path).unwrap();
        assert!(open_exclusive_read_file(&path).is_err());
        drop(writer);
        fs::hard_link(&path, root.join("alias.bin")).unwrap();
        assert!(open_exclusive_read_file(&path).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn file_identity_is_stable_for_the_same_open_target() {
        let root = workspace("identity");
        let path = root.join("target.txt");
        fs::write(&path, b"alpha").expect("write fixture");

        let first = open_file_no_reparse(&path).expect("first open");
        let second = open_file_no_reparse(&path).expect("second open");
        assert_eq!(first.identity(), second.identity());
        assert_eq!(first.final_path(), second.final_path());

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn delete_handle_removes_the_exact_opened_target_on_close() {
        let root = workspace("delete");
        let path = root.join("target.txt");
        fs::write(&path, b"alpha").expect("write fixture");

        let opened = open_file_no_reparse(&path).expect("read open");
        let expected_identity = opened.identity();
        drop(opened);

        let delete = open_file_for_delete_no_reparse(&path).expect("delete open");
        assert_eq!(delete.identity(), expected_identity);
        delete.delete().expect("delete by handle");
        assert!(!path.exists());

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn exclusive_delete_handle_checks_links_and_keeps_target_stable() {
        use std::io::Read;
        let root = workspace("exclusive-retire");
        let path = root.join("closed.tasklog");
        fs::write(&path, b"closed evidence").unwrap();
        let mut opened = open_exclusive_file_for_delete_no_reparse(&path).unwrap();
        let mut observed = Vec::new();
        opened.file_mut().read_to_end(&mut observed).unwrap();
        assert_eq!(observed, b"closed evidence");
        assert!(OpenOptions::new().write(true).open(&path).is_err());
        assert!(fs::rename(&path, root.join("replacement.tasklog")).is_err());
        opened.delete().unwrap();
        assert!(!path.exists());

        fs::write(&path, b"linked evidence").unwrap();
        let alias = root.join("alias.tasklog");
        fs::hard_link(&path, &alias).unwrap();
        assert!(open_exclusive_file_for_delete_no_reparse(&path).is_err());
        assert_eq!(fs::read(&alias).unwrap(), b"linked evidence");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_file_no_replace_preserves_existing_evidence_and_source() {
        let root = workspace("no-clobber-publish");
        let from = root.join("prepared.tmp");
        let to = root.join("stable.journal");
        fs::write(&from, b"prepared candidate").expect("prepared bytes");
        fs::write(&to, b"existing durable evidence").expect("existing bytes");

        assert!(move_file_no_replace(&from, &to).is_err());
        assert_eq!(
            fs::read(&from).expect("candidate retained"),
            b"prepared candidate"
        );
        assert_eq!(
            fs::read(&to).expect("existing evidence"),
            b"existing durable evidence"
        );

        fs::remove_file(&to).expect("remove test destination");
        move_file_no_replace(&from, &to).expect("publish absent destination");
        assert!(!from.exists());
        assert_eq!(
            fs::read(&to).expect("published bytes"),
            b"prepared candidate"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn replace_file_replaces_contents_in_one_windows_primitive() {
        let root = workspace("replace");
        let target = root.join("target.txt");
        let replacement = root.join("replacement.tmp");
        fs::write(&target, b"old").expect("write target");
        fs::write(&replacement, b"new").expect("write replacement");

        replace_file_atomically(&target, &replacement).expect("replace file");
        assert_eq!(fs::read(&target).expect("read target"), b"new");
        assert!(!replacement.exists());

        fs::remove_dir_all(root).expect("remove fixture");
    }
}
