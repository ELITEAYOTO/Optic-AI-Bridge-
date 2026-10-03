use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    mem::size_of,
    os::windows::{
        fs::OpenOptionsExt,
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};

use thiserror::Error;
use windows::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_NAME_NORMALIZED,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileAttributeTagInfo, FileIdInfo,
        GetFileInformationByHandleEx, GetFinalPathNameByHandleW, VOLUME_NAME_DOS,
    },
};

const INITIAL_FINAL_PATH_CHARS: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowsFileIdentity {
    pub volume_serial: u64,
    pub file_id: [u8; 16],
}

#[derive(Debug)]
pub struct HandleValidatedFile {
    root: ValidatedHandle,
    target: ValidatedHandle,
}

impl HandleValidatedFile {
    pub fn open_under_root(
        workspace_root: impl AsRef<Path>,
        target: impl AsRef<Path>,
    ) -> Result<Self, WindowsMutationHandleError> {
        let root = ValidatedHandle::open(workspace_root.as_ref(), ExpectedKind::Directory)?;
        let target = ValidatedHandle::open(target.as_ref(), ExpectedKind::File)?;
        ensure_under_root(&root.final_path, &target.final_path, false)?;
        Ok(Self { root, target })
    }

    #[must_use]
    pub fn final_path(&self) -> &Path {
        &self.target.final_path
    }

    #[must_use]
    pub const fn identity(&self) -> WindowsFileIdentity {
        self.target.identity
    }

    #[must_use]
    pub const fn root_identity(&self) -> WindowsFileIdentity {
        self.root.identity
    }

    pub fn try_clone_file(&self) -> Result<File, WindowsMutationHandleError> {
        self.target.file.try_clone().map_err(Into::into)
    }
}

#[derive(Debug)]
pub struct HandleValidatedDirectory {
    root: ValidatedHandle,
    target: ValidatedHandle,
}

impl HandleValidatedDirectory {
    pub fn open_under_root(
        workspace_root: impl AsRef<Path>,
        target: impl AsRef<Path>,
    ) -> Result<Self, WindowsMutationHandleError> {
        let root = ValidatedHandle::open(workspace_root.as_ref(), ExpectedKind::Directory)?;
        let target = ValidatedHandle::open(target.as_ref(), ExpectedKind::Directory)?;
        ensure_under_root(&root.final_path, &target.final_path, true)?;
        Ok(Self { root, target })
    }

    #[must_use]
    pub fn final_path(&self) -> &Path {
        &self.target.final_path
    }

    #[must_use]
    pub const fn identity(&self) -> WindowsFileIdentity {
        self.target.identity
    }

    #[must_use]
    pub const fn root_identity(&self) -> WindowsFileIdentity {
        self.root.identity
    }
}

#[derive(Debug, Error)]
pub enum WindowsMutationHandleError {
    #[error("Windows filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("mutation target is a reparse point and is denied")]
    ReparsePointDenied,
    #[error("mutation target is not a regular file")]
    NotFile,
    #[error("mutation target is not a directory")]
    NotDirectory,
    #[error("resolved mutation target escapes the validated workspace root")]
    OutsideWorkspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpectedKind {
    File,
    Directory,
}

#[derive(Debug)]
struct ValidatedHandle {
    file: File,
    final_path: PathBuf,
    identity: WindowsFileIdentity,
}

impl ValidatedHandle {
    fn open(path: &Path, expected: ExpectedKind) -> Result<Self, WindowsMutationHandleError> {
        let flags = FILE_FLAG_OPEN_REPARSE_POINT.0
            | matches!(expected, ExpectedKind::Directory)
                .then_some(FILE_FLAG_BACKUP_SEMANTICS.0)
                .unwrap_or_default();
        let share = FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0;
        let file = OpenOptions::new()
            .read(true)
            .share_mode(share)
            .custom_flags(flags)
            .open(path)?;

        let attributes = query_attributes(&file)?;
        if attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(WindowsMutationHandleError::ReparsePointDenied);
        }
        let is_directory = attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
        match (expected, is_directory) {
            (ExpectedKind::File, true) => return Err(WindowsMutationHandleError::NotFile),
            (ExpectedKind::Directory, false) => {
                return Err(WindowsMutationHandleError::NotDirectory);
            }
            _ => {}
        }

        let final_path = query_final_path(&file)?;
        let identity = query_identity(&file)?;
        Ok(Self {
            file,
            final_path,
            identity,
        })
    }
}

fn ensure_under_root(
    root: &Path,
    target: &Path,
    allow_root_itself: bool,
) -> Result<(), WindowsMutationHandleError> {
    let inside = target.starts_with(root) && (allow_root_itself || target != root);
    if inside {
        Ok(())
    } else {
        Err(WindowsMutationHandleError::OutsideWorkspace)
    }
}

fn handle(file: &File) -> HANDLE {
    HANDLE(file.as_raw_handle())
}

fn query_attributes(file: &File) -> Result<FILE_ATTRIBUTE_TAG_INFO, WindowsMutationHandleError> {
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: `file` owns a live Windows file handle. The information class matches
    // the concrete output structure and the buffer remains valid for the call.
    unsafe {
        GetFileInformationByHandleEx(
            handle(file),
            FileAttributeTagInfo,
            (&mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO).cast::<c_void>(),
            size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    }
    .map_err(std::io::Error::other)?;
    Ok(attributes)
}

fn query_identity(file: &File) -> Result<WindowsFileIdentity, WindowsMutationHandleError> {
    let mut info = FILE_ID_INFO::default();
    // SAFETY: `file` owns a live Windows file handle. The information class matches
    // `FILE_ID_INFO` and the output buffer remains valid for the synchronous call.
    unsafe {
        GetFileInformationByHandleEx(
            handle(file),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast::<c_void>(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .map_err(std::io::Error::other)?;
    Ok(WindowsFileIdentity {
        volume_serial: info.VolumeSerialNumber,
        file_id: info.FileId.Identifier,
    })
}

fn query_final_path(file: &File) -> Result<PathBuf, WindowsMutationHandleError> {
    let mut buffer = vec![0_u16; INITIAL_FINAL_PATH_CHARS];
    let flags = FILE_NAME_NORMALIZED | VOLUME_NAME_DOS;

    loop {
        // SAFETY: `file` owns a live handle and `buffer` is a valid mutable UTF-16
        // output slice for the entire call.
        let length = unsafe { GetFinalPathNameByHandleW(handle(file), &mut buffer, flags) };
        if length == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let length = usize::try_from(length).map_err(|_| {
            std::io::Error::other("final Windows path length does not fit usize")
        })?;
        if length < buffer.len() {
            use std::os::windows::ffi::OsStringExt;
            let path = std::ffi::OsString::from_wide(&buffer[..length]);
            return Ok(PathBuf::from(path));
        }
        buffer.resize(length.saturating_add(1), 0);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Read,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("optic-windows-{label}-{token}"));
        fs::create_dir_all(&path).expect("create temp directory");
        path
    }

    fn create_junction(link: &Path, target: &Path) {
        let command = format!(
            "mklink /J \"{}\" \"{}\"",
            link.display(),
            target.display()
        );
        let status = Command::new("cmd")
            .args(["/C", &command])
            .status()
            .expect("invoke mklink");
        assert!(status.success(), "mklink /J failed: {status}");
    }

    #[test]
    fn stable_identity_and_readable_handle_for_same_file() {
        let root = temp_dir("identity");
        let target = root.join("target.txt");
        fs::write(&target, b"alpha").expect("write fixture");

        let first = HandleValidatedFile::open_under_root(&root, &target).expect("first open");
        let second = HandleValidatedFile::open_under_root(&root, &target).expect("second open");
        assert_eq!(first.identity(), second.identity());
        assert_eq!(first.final_path(), second.final_path());

        let mut file = first.try_clone_file().expect("clone validated file");
        let mut content = String::new();
        file.read_to_string(&mut content).expect("read validated file");
        assert_eq!(content, "alpha");

        drop(first);
        drop(second);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn directory_validation_accepts_workspace_root_and_child() {
        let root = temp_dir("directory");
        let child = root.join("child");
        fs::create_dir(&child).expect("create child");

        let root_handle =
            HandleValidatedDirectory::open_under_root(&root, &root).expect("root directory");
        let child_handle =
            HandleValidatedDirectory::open_under_root(&root, &child).expect("child directory");
        assert_ne!(root_handle.identity(), child_handle.identity());

        drop(root_handle);
        drop(child_handle);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn leaf_junction_is_denied_as_reparse_point() {
        let root = temp_dir("leaf-junction");
        let real = root.join("real");
        let alias = root.join("alias");
        fs::create_dir(&real).expect("create real directory");
        create_junction(&alias, &real);

        assert!(matches!(
            HandleValidatedDirectory::open_under_root(&root, &alias),
            Err(WindowsMutationHandleError::ReparsePointDenied)
        ));

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn intermediate_junction_escape_is_denied_by_final_handle_path() {
        let root = temp_dir("junction-root");
        let outside = temp_dir("junction-outside");
        let escape = root.join("escape");
        let secret = outside.join("secret.txt");
        fs::write(&secret, b"outside").expect("write outside fixture");
        create_junction(&escape, &outside);

        assert!(matches!(
            HandleValidatedFile::open_under_root(&root, escape.join("secret.txt")),
            Err(WindowsMutationHandleError::OutsideWorkspace)
        ));

        fs::remove_dir_all(root).expect("remove root fixture");
        fs::remove_dir_all(outside).expect("remove outside fixture");
    }
}
