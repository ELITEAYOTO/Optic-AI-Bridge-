use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

#[cfg(windows)]
const NULL_CONFIG_PATH: &str = "NUL";
#[cfg(not(windows))]
const NULL_CONFIG_PATH: &str = "/dev/null";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegisteredWorktree {
    pub(crate) path: PathBuf,
    pub(crate) locked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorktreeListError {
    Malformed,
    EntryLimitExceeded,
}

pub(crate) fn parse_worktree_list(
    bytes: &[u8],
    entry_limit: u32,
) -> Result<Vec<RegisteredWorktree>, WorktreeListError> {
    if bytes.is_empty() || !bytes.ends_with(b"\0\0") {
        return Err(WorktreeListError::Malformed);
    }

    let mut records = Vec::new();
    let mut fields: Vec<&[u8]> = Vec::new();
    for field in bytes.split(|byte| *byte == 0) {
        if field.is_empty() {
            if fields.is_empty() {
                continue;
            }
            if records.len() >= usize::try_from(entry_limit).unwrap_or(usize::MAX) {
                return Err(WorktreeListError::EntryLimitExceeded);
            }
            records.push(parse_worktree_record(&fields)?);
            fields.clear();
        } else {
            fields.push(field);
        }
    }
    if !fields.is_empty() || records.is_empty() {
        return Err(WorktreeListError::Malformed);
    }
    Ok(records)
}

fn parse_worktree_record(fields: &[&[u8]]) -> Result<RegisteredWorktree, WorktreeListError> {
    let first = fields.first().ok_or(WorktreeListError::Malformed)?;
    let raw_path = first
        .strip_prefix(b"worktree ")
        .ok_or(WorktreeListError::Malformed)?;
    if raw_path.is_empty()
        || fields[1..]
            .iter()
            .any(|field| field.starts_with(b"worktree "))
    {
        return Err(WorktreeListError::Malformed);
    }
    let path = std::str::from_utf8(raw_path).map_err(|_| WorktreeListError::Malformed)?;
    let locked = fields[1..]
        .iter()
        .any(|field| *field == b"locked" || field.starts_with(b"locked "));
    Ok(RegisteredWorktree {
        path: PathBuf::from(path),
        locked,
    })
}

#[cfg(windows)]
fn normalized_windows_path(path: &Path) -> String {
    let mut value = path.as_os_str().to_string_lossy().replace('/', "\\");
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        value = format!(r"\\{rest}");
    } else if let Some(rest) = value.strip_prefix(r"\\?\") {
        value = rest.to_owned();
    }
    while value.len() > 3 && value.ends_with('\\') {
        value.pop();
    }
    value.to_lowercase()
}

#[cfg(windows)]
pub(crate) fn paths_lexically_equal(left: &Path, right: &Path) -> bool {
    normalized_windows_path(left) == normalized_windows_path(right)
}

#[cfg(not(windows))]
pub(crate) fn paths_lexically_equal(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(windows)]
pub(crate) fn path_is_lexically_within(root: &Path, candidate: &Path) -> bool {
    let root = normalized_windows_path(root);
    let candidate = normalized_windows_path(candidate);
    candidate == root
        || candidate
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

#[cfg(not(windows))]
pub(crate) fn path_is_lexically_within(root: &Path, candidate: &Path) -> bool {
    candidate.starts_with(root)
}

pub(crate) fn git_path_arg(path: &Path) -> OsString {
    #[cfg(windows)]
    {
        let value = path.as_os_str().to_string_lossy();
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            return OsString::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = value.strip_prefix(r"\\?\") {
            return OsString::from(rest);
        }
    }
    path.as_os_str().to_os_string()
}

pub(crate) fn git_mutation_base_args(disabled_hooks_root: &Path) -> Vec<OsString> {
    vec![
        OsString::from("--no-pager"),
        OsString::from("--literal-pathspecs"),
        OsString::from("-c"),
        OsString::from("core.fsmonitor=false"),
        OsString::from("-c"),
        OsString::from("core.untrackedCache=false"),
        OsString::from("-c"),
        OsString::from(format!(
            "core.hooksPath={}",
            git_path_arg(disabled_hooks_root).to_string_lossy()
        )),
        OsString::from("-c"),
        OsString::from("commit.gpgSign=false"),
    ]
}

pub(crate) fn git_mutation_environment() -> BTreeMap<OsString, OsString> {
    [
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_PAGER", "cat"),
        ("PAGER", "cat"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", NULL_CONFIG_PATH),
        ("GIT_NO_REPLACE_OBJECTS", "1"),
        ("GIT_ATTR_NOSYSTEM", "1"),
        ("LC_ALL", "C"),
    ]
    .into_iter()
    .map(|(key, value)| (OsString::from(key), OsString::from(value)))
    .collect()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn porcelain_z_parser_is_bounded_and_detects_lock() {
        let bytes = b"worktree /tmp/a\0HEAD 0123\0detached\0locked\0\0";
        let parsed = parse_worktree_list(bytes, 1).expect("valid record");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path, PathBuf::from("/tmp/a"));
        assert!(parsed[0].locked);
        assert_eq!(
            parse_worktree_list(bytes, 0).expect_err("zero bound must reject entry"),
            WorktreeListError::EntryLimitExceeded
        );
    }

    #[test]
    fn malformed_porcelain_z_is_rejected() {
        assert_eq!(
            parse_worktree_list(b"worktree /tmp/a\0HEAD 0123\0", 4)
                .expect_err("missing record terminator must fail"),
            WorktreeListError::Malformed
        );
    }

    #[test]
    fn lexical_containment_is_component_bounded() {
        #[cfg(windows)]
        {
            assert!(path_is_lexically_within(
                Path::new(r"C:\\Optic\\Sessions"),
                Path::new(r"c:\\optic\\sessions\\abc")
            ));
            assert!(!path_is_lexically_within(
                Path::new(r"C:\\Optic\\Sessions"),
                Path::new(r"C:\\Optic\\Sessions-escape\\abc")
            ));
        }
        #[cfg(not(windows))]
        {
            assert!(path_is_lexically_within(
                Path::new("/tmp/optic/sessions"),
                Path::new("/tmp/optic/sessions/abc")
            ));
            assert!(!path_is_lexically_within(
                Path::new("/tmp/optic/sessions"),
                Path::new("/tmp/optic/sessions-escape/abc")
            ));
        }
    }

    #[test]
    fn git_mutation_profile_disables_hooks_and_prompts() {
        let args = git_mutation_base_args(Path::new("/tmp/hooks-disabled"));
        assert!(
            args.iter()
                .any(|arg| arg.to_string_lossy().starts_with("core.hooksPath="))
        );
        let env = git_mutation_environment();
        assert_eq!(
            env.get(OsStr::new("GIT_TERMINAL_PROMPT")),
            Some(&OsString::from("0"))
        );
        assert!(!env.contains_key(OsStr::new("PATH")));
    }
}
