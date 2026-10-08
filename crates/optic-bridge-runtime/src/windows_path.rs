use std::path::Path;

pub(crate) fn windows_path_is_within(root: &Path, candidate: &Path) -> bool {
    fn key(path: &Path) -> String {
        path.as_os_str()
            .to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    }

    let root = key(root);
    let candidate = key(candidate);
    if candidate == root {
        return true;
    }
    candidate
        .strip_prefix(&root)
        .is_some_and(|suffix| suffix.starts_with('\\'))
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
}
