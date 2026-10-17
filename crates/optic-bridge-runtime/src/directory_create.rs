//! One absent directory per action; no recursive creation or implicit success.
use std::path::Path;

use optic_bridge_core::{HardLimits, WorkspacePath};
use thiserror::Error;

use crate::{BoundedFileSystem, FileSystemError};

#[cfg(windows)]
use optic_bridge_windows::{
    PinnedDirectoryError, WindowsFileError, WindowsFileIdentity, inspect_directory_no_reparse,
    pin_directory_chain,
};

pub(crate) struct DirectoryCreationService {
    #[cfg_attr(not(windows), allow(dead_code))]
    filesystem: BoundedFileSystem,
    #[cfg(windows)]
    root_identity: WindowsFileIdentity,
}

impl DirectoryCreationService {
    pub(crate) fn new(
        root: impl AsRef<Path>,
        limits: HardLimits,
    ) -> Result<Self, DirectoryCreateError> {
        let filesystem = BoundedFileSystem::from_hard_limits(root, limits)?;
        #[cfg(windows)]
        let root_identity = inspect_directory_no_reparse(filesystem.root())?.identity;
        Ok(Self {
            filesystem,
            #[cfg(windows)]
            root_identity,
        })
    }

    pub(crate) fn create(&self, path: &WorkspacePath) -> Result<(), DirectoryCreateError> {
        if path.as_str().split('/').count() > 32 {
            return Err(DirectoryCreateError::DepthExceeded);
        }
        #[cfg(windows)]
        {
            let _root = pin_directory_chain(self.filesystem.root(), None)?;
            if inspect_directory_no_reparse(self.filesystem.root())?.identity != self.root_identity
            {
                return Err(DirectoryCreateError::RootChanged);
            }
            let relative = Path::new(path.as_str());
            let parent = relative
                .parent()
                .filter(|path| !path.as_os_str().is_empty());
            // Each ancestor rejects reparse points and denies delete-sharing.
            // Keep every handle live through the single atomic create call.
            let pinned = pin_directory_chain(self.filesystem.root(), parent)?;
            let leaf = relative
                .file_name()
                .ok_or(DirectoryCreateError::InvalidTarget)?;
            std::fs::create_dir(pinned.final_path().join(leaf))?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(DirectoryCreateError::UnsupportedPlatform)
        }
    }
}

#[derive(Debug, Error)]
pub enum DirectoryCreateError {
    #[error("directory creation path exceeds32 components")]
    DepthExceeded,
    #[error("directory creation requires one workspace-relative leaf")]
    InvalidTarget,
    #[error("directory creation is supported only on the validated Windows runtime")]
    UnsupportedPlatform,
    #[error("the selected workspace root has changed identity")]
    RootChanged,
    #[error("directory workspace validation failed: {0}")]
    FileSystem(#[from] FileSystemError),
    #[cfg(windows)]
    #[error("directory ancestor could not be pinned: {0}")]
    Pinned(#[from] PinnedDirectoryError),
    #[cfg(windows)]
    #[error("directory root inspection failed: {0}")]
    Windows(#[from] WindowsFileError),
    #[error("directory creation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use optic_bridge_core::ActionId;
    use std::{env, fs};

    #[test]
    fn absent_leaf_only_and_root_identity_are_enforced() {
        let root = env::temp_dir().join(format!(
            "optic-directory-{}",
            ActionId::generate().expect("entropy").to_token()
        ));
        fs::create_dir(&root).expect("root");
        let service = DirectoryCreationService::new(&root, HardLimits::default()).expect("service");
        let path = WorkspacePath::parse("src").expect("path");
        service.create(&path).expect("create absent directory");
        assert!(
            matches!(service.create(&path), Err(DirectoryCreateError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists)
        );
        fs::write(root.join("file"), b"unchanged").expect("file");
        assert!(
            service
                .create(&WorkspacePath::parse("file").expect("path"))
                .is_err()
        );
        assert_eq!(fs::read(root.join("file")).expect("read"), b"unchanged");
        assert!(
            service
                .create(&WorkspacePath::parse("missing/child").expect("path"))
                .is_err()
        );
        assert!(!root.join("missing").exists());
        let deep =
            WorkspacePath::parse(&std::iter::repeat_n("x", 33).collect::<Vec<_>>().join("/"))
                .expect("path");
        assert!(matches!(
            service.create(&deep),
            Err(DirectoryCreateError::DepthExceeded)
        ));
        let moved = root.with_extension("moved");
        fs::rename(&root, &moved).expect("move selected root");
        fs::create_dir(&root).expect("replacement root");
        assert!(matches!(
            service.create(&WorkspacePath::parse("new").expect("path")),
            Err(DirectoryCreateError::RootChanged)
        ));
        assert!(!root.join("new").exists());
        fs::remove_dir_all(&root).expect("cleanup replacement");
        fs::remove_dir_all(&moved).expect("cleanup original");
    }

    #[test]
    fn reparse_parent_cannot_create_outside_selected_root() {
        let base = env::temp_dir().join(format!(
            "optic-directory-link-{}",
            ActionId::generate().expect("entropy").to_token()
        ));
        let root = base.join("root");
        let outside = base.join("outside");
        fs::create_dir_all(&root).expect("root");
        fs::create_dir(&outside).expect("outside");
        // Native CI must actually exercise the reparse boundary, not silently skip.
        std::os::windows::fs::symlink_dir(&outside, root.join("linked"))
            .expect("native symlink fixture");
        let service = DirectoryCreationService::new(&root, HardLimits::default()).expect("service");
        assert!(
            service
                .create(&WorkspacePath::parse("linked/escape").expect("path"))
                .is_err()
        );
        assert!(!outside.join("escape").exists());
        fs::remove_dir(root.join("linked")).expect("unlink fixture");
        fs::remove_dir_all(base).expect("cleanup");
    }
}
