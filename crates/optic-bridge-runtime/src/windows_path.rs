use std::path::{Path, PathBuf};

fn component_key(component: &std::ffi::OsStr) -> String {
    component.to_string_lossy().to_lowercase()
}

pub(crate) fn windows_relative_path(root: &Path, candidate: &Path) -> Option<PathBuf> {
    let mut candidate_components = candidate.components();
    for root_component in root.components() {
        let candidate_component = candidate_components.next()?;
        if component_key(root_component.as_os_str())
            != component_key(candidate_component.as_os_str())
        {
            return None;
        }
    }

    let mut relative = PathBuf::new();
    for component in candidate_components {
        relative.push(component.as_os_str());
    }
    Some(relative)
}

pub(crate) fn windows_path_is_within(root: &Path, candidate: &Path) -> bool {
    windows_relative_path(root, candidate).is_some()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn containment_is_component_bounded_and_case_insensitive() {
        assert!(windows_path_is_within(
            Path::new(r"\\?\C:\Workspace"),
            Path::new(r"\\?\c:\workspace\src\lib.rs")
        ));
        assert!(!windows_path_is_within(
            Path::new(r"\\?\C:\Workspace"),
            Path::new(r"\\?\C:\Workspace-escape\secret.txt")
        ));
    }

    #[test]
    fn relative_path_preserves_observed_component_spelling() {
        assert_eq!(
            windows_relative_path(
                Path::new(r"\\?\C:\Workspace"),
                Path::new(r"\\?\c:\workspace\ActualCase\File.txt")
            ),
            Some(PathBuf::from(r"ActualCase\File.txt"))
        );
        assert_eq!(
            windows_relative_path(
                Path::new(r"\\?\C:\Workspace"),
                Path::new(r"\\?\C:\Workspace")
            ),
            Some(PathBuf::new())
        );
    }
}
