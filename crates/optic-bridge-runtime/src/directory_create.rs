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
        // Windows pins/rejects reparse ancestors below; an exact .git segment
        // is reserved even when requested through a broad workspace lease.
        if crate::mutation::is_git_control_metadata_path(path) {
            return Err(DirectoryCreateError::ReservedGitMetadata);
        }
        if path.as_str().split('/').count() > 32 {
            return Err(DirectoryCreateError::DepthExceeded);
        }
        validate_names(path)?;
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
            if let Some(parent) = parent {
                let observed = crate::windows_path::windows_relative_path(
                    self.filesystem.root(),
                    pinned.final_path(),
                )
                .ok_or(DirectoryCreateError::AncestorAlias)?;
                let observed = WorkspacePath::from_native_relative(&observed)
                    .map_err(|_| DirectoryCreateError::AncestorAlias)?;
                let expected = WorkspacePath::from_native_relative(parent)
                    .map_err(|_| DirectoryCreateError::AncestorAlias)?;
                if !observed.as_str().eq_ignore_ascii_case(expected.as_str()) {
                    return Err(DirectoryCreateError::AncestorAlias);
                }
            }
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

fn validate_names(path: &WorkspacePath) -> Result<(), DirectoryCreateError> {
    for segment in path.as_str().split('/') {
        let stem = segment
            .split('.')
            .next()
            .unwrap_or_default()
            .trim_end_matches(' ')
            .to_ascii_lowercase();
        let device = matches!(
            stem.as_str(),
            "con" | "prn" | "aux" | "nul" | "conin$" | "conout$"
        ) || ["com", "lpt"].into_iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
        if device
            || segment.ends_with(['.', ' '])
            || segment
                .chars()
                .any(|c| c.is_control() || "<>\"|?*".contains(c))
        {
            return Err(DirectoryCreateError::InvalidName);
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum DirectoryCreateError {
    #[error("generic directory creation cannot change Git control metadata")]
    ReservedGitMetadata,
    #[error(
        "directory names must not use Windows device, control or ambiguous trailing characters"
    )]
    InvalidName,
    #[error("directory ancestor resolves to another native name")]
    AncestorAlias,
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
    fn directory_names_refuse_devices_and_aliases_but_allow_unicode_projects() {
        for name in [
            "src.",
            "src ",
            "NUL",
            "CON.txt",
            "con .txt",
            "COM1",
            "lpt9.log",
            "COM¹",
            "CONIN$",
            "a/child?",
            "a\u{0001}",
            "a/child*",
        ] {
            assert!(
                matches!(
                    validate_names(&WorkspacePath::parse(name).unwrap()),
                    Err(DirectoryCreateError::InvalidName)
                ),
                "{name:?}"
            );
        }
        for name in ["projet français/assets", "日本語", "COM10", "my folder"] {
            validate_names(&WorkspacePath::parse(name).unwrap()).unwrap();
        }
    }

    #[test]
    fn generic_directory_creation_rejects_git_control_names() {
        let root = env::temp_dir().join(format!(
            "optic-directory-git-boundary-{}",
            ActionId::generate().expect("entropy").to_token()
        ));
        fs::create_dir(&root).expect("root");
        fs::create_dir(root.join("nested")).expect("nested");
        let service = DirectoryCreationService::new(&root, HardLimits::default()).expect("service");
        for name in [".git", ".GiT", "nested/.git", "nested/.GiT"] {
            assert!(matches!(
                service.create(&WorkspacePath::parse(name).expect("path")),
                Err(DirectoryCreateError::ReservedGitMetadata)
            ));
            assert!(!root.join(name).exists(), "{name} must not be created");
        }
        service
            .create(&WorkspacePath::parse("ordinary-project").expect("normal path"))
            .expect("normal directory creation remains available");
        assert!(root.join("ordinary-project").is_dir());
        fs::remove_dir_all(root).expect("cleanup");
    }

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
