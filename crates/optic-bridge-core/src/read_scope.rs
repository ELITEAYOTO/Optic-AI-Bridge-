use crate::WorkspacePath;

/// Workspace-local boundary shared by the typed read/search authority scopes.
///
/// `All` means the whole configured Optic workspace, never the host machine.
/// `Prefix` means one project-relative subtree inside that workspace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkspaceAuthorityScope {
    All,
    Prefix(WorkspacePath),
}

impl WorkspaceAuthorityScope {
    #[must_use]
    pub fn covers_path(&self, path: &WorkspacePath) -> bool {
        match self {
            Self::All => true,
            Self::Prefix(prefix) => path.is_within(prefix),
        }
    }

    #[must_use]
    pub fn covers_search_root(&self, root: Option<&WorkspacePath>) -> bool {
        match (self, root) {
            (Self::All, _) => true,
            (Self::Prefix(_), None) => false,
            (Self::Prefix(prefix), Some(root)) => root.is_within(prefix),
        }
    }
}

macro_rules! typed_workspace_scope {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(WorkspaceAuthorityScope);

        impl $name {
            #[must_use]
            pub const fn all() -> Self {
                Self(WorkspaceAuthorityScope::All)
            }

            #[must_use]
            pub fn prefix(path: WorkspacePath) -> Self {
                Self(WorkspaceAuthorityScope::Prefix(path))
            }

            #[must_use]
            pub fn covers_path(&self, path: &WorkspacePath) -> bool {
                self.0.covers_path(path)
            }

            #[must_use]
            pub fn covers_search_root(&self, root: Option<&WorkspacePath>) -> bool {
                self.0.covers_search_root(root)
            }

            #[must_use]
            pub const fn workspace_scope(&self) -> &WorkspaceAuthorityScope {
                &self.0
            }
        }
    };
}

typed_workspace_scope!(
    SearchMetadataScope,
    "Authority to enumerate/search metadata inside a bounded workspace scope. It does not grant file-content reads."
);
typed_workspace_scope!(
    ReadContentScope,
    "Authority to read ordinary file content inside a bounded workspace scope. It does not imply sensitive-content authority."
);
typed_workspace_scope!(
    ReadSensitiveScope,
    "Additional authority for content classified as sensitive by a future explicit classifier. This type does not classify paths by itself."
);

#[cfg(test)]
mod tests {
    use super::*;

    fn path(value: &str) -> WorkspacePath {
        WorkspacePath::parse(value).expect("workspace path")
    }

    #[test]
    fn prefix_scope_covers_only_its_subtree() {
        let scope = WorkspaceAuthorityScope::Prefix(path("src"));
        assert!(scope.covers_path(&path("src/lib.rs")));
        assert!(!scope.covers_path(&path("docs/readme.md")));
        assert!(scope.covers_search_root(Some(&path("src/bin"))));
        assert!(!scope.covers_search_root(None));
    }

    #[test]
    fn typed_scopes_keep_search_content_and_sensitive_authority_distinct() {
        let prefix = path("src");
        let search = SearchMetadataScope::prefix(prefix.clone());
        let content = ReadContentScope::prefix(prefix.clone());
        let sensitive = ReadSensitiveScope::prefix(prefix);

        assert_eq!(search.workspace_scope(), content.workspace_scope());
        assert_eq!(content.workspace_scope(), sensitive.workspace_scope());
        assert!(search.covers_search_root(Some(&path("src"))));
        assert!(content.covers_path(&path("src/lib.rs")));
        assert!(sensitive.covers_path(&path("src/secrets.txt")));
    }

    #[test]
    fn all_scope_still_means_workspace_all_not_machine_all() {
        let scope = SearchMetadataScope::all();
        assert!(scope.covers_search_root(None));
        assert!(scope.covers_path(&path("nested/file.txt")));
    }
}
