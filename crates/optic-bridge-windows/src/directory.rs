use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::OpenOptionsExt,
    path::{Component, Path, PathBuf},
};

use thiserror::Error;
use windows::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use crate::file::{WindowsFileError, query_final_path, reject_reparse};

#[derive(Debug)]
pub struct PinnedDirectoryChain {
    _handles: Vec<File>,
    final_path: PathBuf,
}

impl PinnedDirectoryChain {
    #[must_use]
    pub fn final_path(&self) -> &Path {
        &self.final_path
    }
}

pub fn pin_directory_chain(
    root: &Path,
    relative: Option<&Path>,
) -> Result<PinnedDirectoryChain, PinnedDirectoryError> {
    if !root.is_absolute() {
        return Err(PinnedDirectoryError::RootMustBeAbsolute);
    }

    let mut handles = Vec::new();
    let mut current = root.to_path_buf();
    let root_handle = open_pinned_directory(&current)?;
    let mut final_path = query_final_path(&root_handle)?;
    handles.push(root_handle);

    if let Some(relative) = relative {
        if relative.is_absolute() {
            return Err(PinnedDirectoryError::InvalidRelativePath);
        }
        for component in relative.components() {
            let Component::Normal(segment) = component else {
                return Err(PinnedDirectoryError::InvalidRelativePath);
            };
            current.push(segment);
            let handle = open_pinned_directory(&current)?;
            final_path = query_final_path(&handle)?;
            handles.push(handle);
        }
    }

    Ok(PinnedDirectoryChain {
        _handles: handles,
        final_path,
    })
}

fn open_pinned_directory(path: &Path) -> Result<File, PinnedDirectoryError> {
    let mut options = OpenOptions::new();
    options
        .access_mode(0)
        .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0 | FILE_FLAG_BACKUP_SEMANTICS.0);
    let file = options.open(path)?;
    reject_reparse(&file)?;
    if !file.metadata()?.is_dir() {
        return Err(PinnedDirectoryError::NotDirectory);
    }
    Ok(file)
}

#[derive(Debug, Error)]
pub enum PinnedDirectoryError {
    #[error("pinned directory root must be absolute")]
    RootMustBeAbsolute,
    #[error("pinned directory relative path must contain normal components only")]
    InvalidRelativePath,
    #[error("pinned cwd component is not a directory")]
    NotDirectory,
    #[error("Windows directory pin operation failed: {0}")]
    Windows(#[from] WindowsFileError),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    fn workspace(label: &str) -> PathBuf {
        let root = env::temp_dir().join(format!(
            "optic-pinned-directory-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("workspace");
        root
    }

    #[test]
    fn pinned_chain_blocks_directory_rename_until_drop() {
        let root = workspace("rename");
        let cwd = root.join("nested").join("cwd");
        fs::create_dir_all(&cwd).expect("cwd");
        let pin = pin_directory_chain(&root, Some(Path::new("nested/cwd"))).expect("pin cwd");
        assert!(pin.final_path().ends_with(Path::new("nested").join("cwd")));

        let renamed = root.join("nested-renamed");
        assert!(
            fs::rename(root.join("nested"), &renamed).is_err(),
            "ancestor rename must fail while a no-delete-share chain handle is live"
        );

        drop(pin);
        fs::rename(root.join("nested"), &renamed).expect("rename after pin drop");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn parent_components_are_rejected() {
        let root = workspace("parent");
        assert!(matches!(
            pin_directory_chain(&root, Some(Path::new("../escape"))),
            Err(PinnedDirectoryError::InvalidRelativePath)
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
