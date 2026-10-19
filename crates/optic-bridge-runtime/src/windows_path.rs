use std::path::{Path, PathBuf};

fn component_key(component: &std::ffi::OsStr) -> Option<String> {
    // Distinct native UTF-16 names must not collapse to the same replacement
    // character. WorkspacePath is UTF-8; unrepresentable native paths refuse.
    component.to_str().map(str::to_lowercase)
}

pub(crate) fn windows_relative_path(root: &Path, candidate: &Path) -> Option<PathBuf> {
    let mut candidate_components = candidate.components();
    for root_component in root.components() {
        let candidate_component = candidate_components.next()?;
        if component_key(root_component.as_os_str())?
            != component_key(candidate_component.as_os_str())?
        {
            return None;
        }
    }

    let mut relative = PathBuf::new();
    for component in candidate_components {
        component.as_os_str().to_str()?;
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

    #[test]
    fn distinct_non_unicode_roots_never_compare_through_replacement_characters() {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};

        let native_root = |surrogate: u16| {
            let mut name: Vec<u16> = r"\\?\C:\workspace-".encode_utf16().collect();
            name.push(surrogate);
            PathBuf::from(OsString::from_wide(&name))
        };
        let root = native_root(0xd800);
        let other = native_root(0xd801);
        assert_ne!(root, other);
        assert_eq!(root.to_string_lossy(), other.to_string_lossy());
        assert!(windows_relative_path(&root, &other.join("public.txt")).is_none());
        assert!(windows_relative_path(&root, &root.join("public.txt")).is_none());
    }

    #[test]
    fn relative_path_refuses_non_unicode_suffix_but_preserves_valid_unicode() {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};

        let root = Path::new(r"\\?\C:\Workspace");
        let invalid = root.join(OsString::from_wide(&[0xd800]));
        assert!(windows_relative_path(root, &invalid).is_none());
        assert_eq!(
            windows_relative_path(Path::new(r"\\?\C:\Dév"), Path::new(r"\\?\c:\dév\Réel.txt")),
            Some(PathBuf::from("Réel.txt"))
        );
    }
}
