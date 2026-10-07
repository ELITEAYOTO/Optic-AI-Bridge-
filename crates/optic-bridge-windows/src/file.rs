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
            FILE_ID_INFO, FILE_SHARE_READ, FileAttributeTagInfo, FileDispositionInfo, FileIdInfo,
            GetFileInformationByHandleEx, GetFinalPathNameByHandleW, REPLACE_FILE_FLAGS,
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

impl OpenedWindowsFile {
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

fn reject_reparse(file: &File) -> Result<(), WindowsFileError> {
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
