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
    "Additional authority for paths classified sensitive by the deterministic workspace classifier. Ordinary content authority is still required."
);

/// Conservative name-based sensitivity floor for canonical workspace paths.
/// This is not a content-secret detector: an ordinary-looking file can still
/// contain secrets. Real-project remote exposure remains blocked until all
/// read surfaces and content policy are validated.
#[must_use]
pub fn requires_sensitive_authority(path: &WorkspacePath) -> bool {
    path.as_str().split('/').any(|segment| {
        let name = segment.to_ascii_lowercase();
        matches!(
            name.as_str(),
            ".envrc"
                | ".ssh"
                | ".aws"
                | ".azure"
                | ".gnupg"
                | ".kube"
                | ".docker"
                | ".git"
                | ".npmrc"
                | ".pypirc"
                | ".netrc"
                | ".git-credentials"
                | "credentials"
                | "id_rsa"
                | "id_dsa"
                | "id_ecdsa"
                | "id_ed25519"
        ) || name.starts_with(".envrc.")
            || name == ".env"
            || name.starts_with(".env.")
            // Explicit API-key artifact names are sensitive across filesystem
            // reads, Git status and Git diffs. Do not deny ordinary source or
            // documentation names such as api_key_parser.rs or api-key-guide.md.
            || ["api-key", "api_key", "apikey", "api.key"]
                .iter()
                .any(|stem| name == *stem || name.strip_prefix(stem).is_some_and(|rest| rest.starts_with('.')))
            || name.contains("secret")
            || name.contains("credential")
            || name == "token"
            || name.starts_with("token.")
            || name.starts_with("token_")
            || name.starts_with("token-")
            || name.contains("_token")
            || name.contains("-token")
            || [
                ".pem",
                ".key",
                ".p12",
                ".pfx",
                ".kdbx",
                ".jks",
                ".keystore",
                ".tfstate",
                ".tfvars",
            ]
            .iter()
            .any(|suffix| name.ends_with(suffix))
    })
}

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
    fn sensitive_path_classifier_checks_all_segments_and_ascii_case() {
        for candidate in [
            ".env",
            ".ENV.production",
            ".envrc",
            "nested/.ENVRC.private",
            "config/.aws/credentials",
            "nested/private.KEY",
            "keys/id_ed25519",
            "src/SeCrEtS.json",
            "terraform/state.tfstate",
            "auth/api_Token.txt",
            ".git/config",
            "nested/.npmrc",
        ] {
            assert!(
                requires_sensitive_authority(&path(candidate)),
                "sensitive path not recognized: {candidate}"
            );
        }
        for candidate in [
            "src/lib.rs",
            ".gitignore",
            ".gitattributes",
            "README.md",
            "src/environment.rs",
            "docs/security.md",
            "tokenizer.rs",
        ] {
            assert!(
                !requires_sensitive_authority(&path(candidate)),
                "ordinary path unexpectedly classified: {candidate}"
            );
        }
    }

    #[test]
    fn api_key_file_names_require_sensitive_authority_but_docs_stay_ordinary() {
        for name in [
            "api-key.json",
            "API_KEY.yml",
            "apikey.txt",
            "api.key",
            "nested/apiKey.json",
            "config/API.KEY.yaml",
        ] {
            assert!(
                requires_sensitive_authority(&path(name)),
                "API key file could bypass sensitive-read authority: {name}"
            );
        }
        for name in [
            "docs/api-key-guide.md",
            "src/api_key_parser.rs",
            "docs/apikey-usage.md",
            "README.md",
        ] {
            assert!(
                !requires_sensitive_authority(&path(name)),
                "ordinary documentation or code was overclassified: {name}"
            );
        }
    }

    #[test]
    fn all_scope_still_means_workspace_all_not_machine_all() {
        let scope = SearchMetadataScope::all();
        assert!(scope.covers_search_root(None));
        assert!(scope.covers_path(&path("nested/file.txt")));
    }
}
