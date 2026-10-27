use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;

/// Read handle that deliberately denies concurrent write/delete sharing.
///
/// Keeping this value alive prevents a path-authorized executable from being
/// rewritten, deleted, or renamed between identity verification and process
/// creation. Read sharing stays enabled so normal readers remain compatible.
#[derive(Debug)]
pub struct PinnedExecutableFile {
    file: File,
    final_path: PathBuf,
}

impl PinnedExecutableFile {
    /// Actual kernel-observed final target of this pinned executable handle.
    /// This is path custody only; full lease hash and launch identity must
    /// still be validated against the registered TaskLease before creation.
    #[must_use]
    pub fn final_path(&self) -> &Path {
        &self.final_path
    }

    #[must_use]
    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }
}

pub fn open_pinned_executable(path: &Path) -> io::Result<PinnedExecutableFile> {
    let mut options = OpenOptions::new();
    options.read(true).share_mode(FILE_SHARE_READ.0);
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("pinned executable must be a regular file"));
    }
    let final_path = crate::file::query_final_path(&file).map_err(io::Error::other)?;
    Ok(PinnedExecutableFile { file, final_path })
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    #[test]
    fn pin_denies_write_until_handle_is_dropped() {
        let root = env::temp_dir().join(format!(
            "optic-bridge-executable-pin-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture root");
        let path = root.join("tool.exe");
        fs::write(&path, b"pinned-tool").expect("write fixture");

        let pin = open_pinned_executable(&path).expect("pin executable");
        assert_eq!(pin.final_path(), fs::canonicalize(&path).unwrap());
        assert!(
            OpenOptions::new().write(true).open(&path).is_err(),
            "write-capable open must be denied while executable identity is pinned"
        );

        drop(pin);
        OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("write-capable open after pin drop");
        fs::remove_dir_all(root).expect("remove fixture root");
    }
}
