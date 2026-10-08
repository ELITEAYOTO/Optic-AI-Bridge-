use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use optic_bridge_core::{
    ActionEnvelope, ActionId, Capability, ContentVersion, Effect, ExpectedState, WorkspacePath,
};
use optic_bridge_runtime::{
    AtomicMutationError, AuthorizedFileMutationError, AuthorizedFileMutationService, BytePatch,
    CommitVerification, DeleteVerification, FileSystemError, JournaledDeleteCommit,
    JournaledMutationCommit, JournaledMutationError, TransactionalFileError,
    mutation_resource_budget,
};
use rmcp::{ErrorData, Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::server::ReadonlyMcpServer;

#[tool_router(router = mutation_write_tool_router, vis = "pub")]
impl ReadonlyMcpServer {
    #[tool(
        name = "fs_write",
        description = "Create or replace one project-relative file using base64 content and an explicit absent-or-exact-content precondition. Mutation authority is operator-owned and cannot be supplied by the caller."
    )]
    pub async fn fs_write(
        &self,
        params: Parameters<FsWriteRequest>,
    ) -> Result<Json<FsMutationResponse>, ErrorData> {
        let path = parse_workspace_path(&params.0.path)?;
        let expected = params.0.expected.into_expected_state()?;
        let content =
            decode_bounded_base64(&params.0.content_base64, self.limits.max_fs_mutation_bytes)?;
        let effect = Effect::FileWrite {
            path: path.clone(),
            expected,
        };
        let envelope = self.mutation_envelope(Capability::FileWrite, effect)?;
        let result = self
            .run_mutation(move |service| service.write(&envelope, &content))
            .await?;
        let response = write_response(result)?;
        self.ensure_structured_payload_fits(&response)?;
        Ok(Json(response))
    }

    #[tool(
        name = "fs_apply_patch",
        description = "Apply one deterministic byte-range patch to a project-relative file. The caller must supply the exact base content version; inserted bytes are base64."
    )]
    pub async fn fs_apply_patch(
        &self,
        params: Parameters<FsApplyPatchRequest>,
    ) -> Result<Json<FsMutationResponse>, ErrorData> {
        let path = parse_workspace_path(&params.0.path)?;
        let expected = parse_content_version(&params.0.expected_version)?;
        let insert =
            decode_bounded_base64(&params.0.insert_base64, self.limits.max_fs_mutation_bytes)?;
        let patch = BytePatch {
            offset: params.0.offset,
            remove_bytes: params.0.remove_bytes,
            insert,
        };
        let effect = Effect::FileWrite {
            path: path.clone(),
            expected: ExpectedState::Content(expected),
        };
        let envelope = self.mutation_envelope(Capability::FileWrite, effect)?;
        let result = self
            .run_mutation(move |service| service.apply_patch(&envelope, &patch))
            .await?;
        let response = write_response(result)?;
        self.ensure_structured_payload_fits(&response)?;
        Ok(Json(response))
    }
}

#[tool_router(router = mutation_delete_tool_router, vis = "pub")]
impl ReadonlyMcpServer {
    #[tool(
        name = "fs_delete",
        description = "Delete one project-relative file only when its exact content version still matches. Mutation authority is operator-owned and cannot be supplied by the caller."
    )]
    pub async fn fs_delete(
        &self,
        params: Parameters<FsDeleteRequest>,
    ) -> Result<Json<FsDeleteResponse>, ErrorData> {
        let path = parse_workspace_path(&params.0.path)?;
        let expected = parse_content_version(&params.0.expected_version)?;
        let effect = Effect::FileDelete {
            path: path.clone(),
            expected,
        };
        let envelope = self.mutation_envelope(Capability::FileDelete, effect)?;
        let result = self
            .run_mutation(move |service| service.delete(&envelope))
            .await?;
        let response = delete_response(result)?;
        self.ensure_structured_payload_fits(&response)?;
        Ok(Json(response))
    }
}

impl ReadonlyMcpServer {
    fn mutation_envelope(
        &self,
        capability: Capability,
        effect: Effect,
    ) -> Result<ActionEnvelope, ErrorData> {
        let now = self.clock.now();
        let grant = self.active_grant(now)?;
        let lease_id = self
            .mutation_authorities
            .lease_for(capability)
            .cloned()
            .ok_or_else(|| {
                ErrorData::invalid_request("optic.mutation_authority_unavailable", None)
            })?;
        let action_id = ActionId::generate()
            .map_err(|_| ErrorData::internal_error("optic.action_id_unavailable", None))?;
        Ok(ActionEnvelope {
            action_id,
            session: self.session.clone(),
            task_lease: Some(lease_id),
            effect,
            resources: mutation_resource_budget(self.limits),
            policy_epoch: grant.policy_epoch,
        })
    }

    async fn run_mutation<T, F>(&self, operation: F) -> Result<T, ErrorData>
    where
        T: Send + 'static,
        F: FnOnce(Arc<AuthorizedFileMutationService>) -> Result<T, AuthorizedFileMutationError>
            + Send
            + 'static,
    {
        let permit = self
            .transport_guard
            .begin_execution(self.clock.now())
            .map_err(super::server::map_transport_error)?;
        let service = self.mutation_service.as_ref().cloned().ok_or_else(|| {
            ErrorData::invalid_request("optic.mutation_runtime_unavailable", None)
        })?;

        // A started spawn_blocking task cannot be cancelled safely. Once a durable
        // mutation is admitted, keep the transport permit and follow the operation
        // to a known transactional/recovery outcome instead of returning a timeout
        // while the filesystem effect may still commit in the background.
        let result = tokio::task::spawn_blocking(move || operation(service))
            .await
            .map_err(|_| ErrorData::internal_error("optic.runtime_join_failed", None))?
            .map_err(map_authorized_mutation_error)?;
        drop(permit);
        Ok(result)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FsWriteRequest {
    pub path: String,
    pub content_base64: String,
    pub expected: ExpectedStateRequest,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExpectedStateRequest {
    Absent {},
    Content { version_hex: String },
}

impl ExpectedStateRequest {
    fn into_expected_state(self) -> Result<ExpectedState, ErrorData> {
        match self {
            Self::Absent {} => Ok(ExpectedState::Absent),
            Self::Content { version_hex } => {
                parse_content_version(&version_hex).map(ExpectedState::Content)
            }
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FsApplyPatchRequest {
    pub path: String,
    pub expected_version: String,
    pub offset: u64,
    pub remove_bytes: u64,
    pub insert_base64: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FsDeleteRequest {
    pub path: String,
    pub expected_version: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FsMutationResponse {
    pub action_id: String,
    pub path: String,
    pub content_version: String,
    pub journal_retired: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FsDeleteResponse {
    pub action_id: String,
    pub path: String,
    pub absent: bool,
    pub journal_retired: bool,
}

fn parse_workspace_path(value: &str) -> Result<WorkspacePath, ErrorData> {
    WorkspacePath::parse(value)
        .map_err(|_| ErrorData::invalid_params("optic.invalid_workspace_path", None))
}

fn parse_content_version(value: &str) -> Result<ContentVersion, ErrorData> {
    ContentVersion::from_hex(value)
        .map_err(|_| ErrorData::invalid_params("optic.invalid_content_version", None))
}

fn decode_bounded_base64(value: &str, limit: u64) -> Result<Vec<u8>, ErrorData> {
    let decoded = STANDARD
        .decode(value)
        .map_err(|_| ErrorData::invalid_params("optic.invalid_base64", None))?;
    let len = u64::try_from(decoded.len())
        .map_err(|_| ErrorData::invalid_params("optic.fs_mutation_limit_exceeded", None))?;
    if len > limit {
        return Err(ErrorData::invalid_params(
            "optic.fs_mutation_limit_exceeded",
            None,
        ));
    }
    Ok(decoded)
}

fn write_response(result: JournaledMutationCommit) -> Result<FsMutationResponse, ErrorData> {
    let version = match result.commit.verification {
        CommitVerification::Verified(version) => version,
        CommitVerification::CommittedButUnverified => {
            return Err(ErrorData::internal_error(
                "optic.mutation_recovery_required",
                None,
            ));
        }
    };
    Ok(FsMutationResponse {
        action_id: result.action_id.to_token(),
        path: result.commit.canonical_path.as_str().to_owned(),
        content_version: version.to_hex(),
        journal_retired: result.journal_retired,
    })
}

fn delete_response(result: JournaledDeleteCommit) -> Result<FsDeleteResponse, ErrorData> {
    if result.commit.verification != DeleteVerification::VerifiedAbsent {
        return Err(ErrorData::internal_error(
            "optic.mutation_recovery_required",
            None,
        ));
    }
    Ok(FsDeleteResponse {
        action_id: result.action_id.to_token(),
        path: result.commit.canonical_path.as_str().to_owned(),
        absent: true,
        journal_retired: result.journal_retired,
    })
}

fn map_authorized_mutation_error(error: AuthorizedFileMutationError) -> ErrorData {
    match error {
        AuthorizedFileMutationError::Session(_) => {
            ErrorData::invalid_request("optic.session_inactive", None)
        }
        AuthorizedFileMutationError::TaskLease(_)
        | AuthorizedFileMutationError::MissingTaskLease => {
            ErrorData::invalid_request("optic.mutation_authority_inactive", None)
        }
        AuthorizedFileMutationError::PolicyDenied(_) => {
            ErrorData::invalid_request("optic.policy_denied", None)
        }
        AuthorizedFileMutationError::EffectMismatch => {
            ErrorData::internal_error("optic.mutation_effect_mismatch", None)
        }
        AuthorizedFileMutationError::PatchRequiresContentState => {
            ErrorData::invalid_params("optic.patch_requires_content_state", None)
        }
        AuthorizedFileMutationError::Transaction(error) => map_transactional_error(error),
    }
}

fn map_transactional_error(error: TransactionalFileError) -> ErrorData {
    match error {
        TransactionalFileError::NewContentTooLarge { .. }
        | TransactionalFileError::SnapshotTooLarge { .. } => {
            ErrorData::invalid_params("optic.fs_mutation_limit_exceeded", None)
        }
        TransactionalFileError::PatchOutOfRange => {
            ErrorData::invalid_params("optic.patch_out_of_range", None)
        }
        TransactionalFileError::PatchBaseChanged => {
            ErrorData::invalid_request("optic.precondition_failed", None)
        }
        TransactionalFileError::SnapshotDidNotProgress => {
            ErrorData::internal_error("optic.filesystem_error", None)
        }
        TransactionalFileError::FileSystem(error) => map_filesystem_error(error),
        TransactionalFileError::Mutation(error) => map_journaled_mutation_error(error),
    }
}

fn map_journaled_mutation_error(error: JournaledMutationError) -> ErrorData {
    match error {
        JournaledMutationError::Atomic(error) => map_atomic_mutation_error(error),
        JournaledMutationError::UnsupportedPlatform => {
            ErrorData::invalid_request("optic.mutation_platform_unsupported", None)
        }
        JournaledMutationError::RecoveryRequiredAfterAtomicError { .. }
        | JournaledMutationError::CommittedButNeedsRecovery { .. }
        | JournaledMutationError::RecoveryRequiredAfterJournalError { .. } => {
            ErrorData::internal_error("optic.mutation_recovery_required", None)
        }
        JournaledMutationError::Journal(_)
        | JournaledMutationError::UnsafeStagingArtifact { .. }
        | JournaledMutationError::UnexpectedPreparedStagingArtifact { .. }
        | JournaledMutationError::UnexpectedAbsentIntentStagingArtifact { .. }
        | JournaledMutationError::StagingArtifactTooLarge { .. }
        | JournaledMutationError::StagingObservationFailed { .. }
        | JournaledMutationError::StagingContentMismatch { .. }
        | JournaledMutationError::StagingCleanupFailed { .. } => {
            ErrorData::internal_error("optic.mutation_recovery_required", None)
        }
    }
}

fn map_atomic_mutation_error(error: AtomicMutationError) -> ErrorData {
    match error {
        AtomicMutationError::Mutation(error) => match error {
            optic_bridge_runtime::MutationError::PreconditionFailed { .. } => {
                ErrorData::invalid_request("optic.precondition_failed", None)
            }
            optic_bridge_runtime::MutationError::SymlinkDenied => {
                ErrorData::invalid_params("optic.symlink_mutation_denied", None)
            }
            optic_bridge_runtime::MutationError::TargetTooLarge { .. } => {
                ErrorData::invalid_params("optic.fs_mutation_limit_exceeded", None)
            }
            optic_bridge_runtime::MutationError::InvalidTarget
            | optic_bridge_runtime::MutationError::NotFile
            | optic_bridge_runtime::MutationError::NotDirectory
            | optic_bridge_runtime::MutationError::NonUtf8Name
            | optic_bridge_runtime::MutationError::Path(_) => {
                ErrorData::invalid_params("optic.invalid_mutation_target", None)
            }
            optic_bridge_runtime::MutationError::FileSystem(error) => map_filesystem_error(error),
            optic_bridge_runtime::MutationError::Io(_) => {
                ErrorData::internal_error("optic.filesystem_error", None)
            }
        },
        AtomicMutationError::NewContentTooLarge { .. } => {
            ErrorData::invalid_params("optic.fs_mutation_limit_exceeded", None)
        }
        AtomicMutationError::CanonicalTargetChanged
        | AtomicMutationError::TargetIdentityChanged
        | AtomicMutationError::ParentIdentityChanged
        | AtomicMutationError::StrongPreconditionMismatch
        | AtomicMutationError::TargetAppearedDuringCommit => {
            ErrorData::invalid_request("optic.precondition_failed", None)
        }
        AtomicMutationError::OutsideWorkspace => {
            ErrorData::invalid_params("optic.path_outside_workspace", None)
        }
        AtomicMutationError::UnsupportedPlatform => {
            ErrorData::invalid_request("optic.mutation_platform_unsupported", None)
        }
        AtomicMutationError::StagingArtifactAlreadyExists => {
            ErrorData::internal_error("optic.mutation_action_active", None)
        }
        AtomicMutationError::FileSystem(error) => map_filesystem_error(error),
        AtomicMutationError::InvalidTarget
        | AtomicMutationError::InvalidPreparedMutation
        | AtomicMutationError::TempNameEntropy
        | AtomicMutationError::Io(_) => ErrorData::internal_error("optic.mutation_failed", None),
        #[cfg(windows)]
        AtomicMutationError::Windows(_) => ErrorData::internal_error("optic.mutation_failed", None),
    }
}

fn map_filesystem_error(error: FileSystemError) -> ErrorData {
    match error {
        FileSystemError::OutsideWorkspace => {
            ErrorData::invalid_params("optic.path_outside_workspace", None)
        }
        FileSystemError::NotFile => ErrorData::invalid_params("optic.not_file", None),
        FileSystemError::NotDirectory => ErrorData::invalid_params("optic.not_directory", None),
        FileSystemError::ReadLimitExceeded | FileSystemError::OffsetOutOfRange => {
            ErrorData::invalid_params("optic.fs_read_limit_exceeded", None)
        }
        FileSystemError::ListLimitExceeded | FileSystemError::DirectoryScanLimitExceeded => {
            ErrorData::invalid_params("optic.fs_list_limit_exceeded", None)
        }
        FileSystemError::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ErrorData::invalid_params("optic.path_not_found", None)
        }
        FileSystemError::InvalidLimits
        | FileSystemError::RootNotDirectory
        | FileSystemError::Path(_)
        | FileSystemError::NonUtf8Name
        | FileSystemError::Io(_) => ErrorData::internal_error("optic.filesystem_error", None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_state_request_is_never_blind() {
        assert!(matches!(
            ExpectedStateRequest::Absent {}
                .into_expected_state()
                .expect("absent"),
            ExpectedState::Absent
        ));
        let version = ContentVersion::from_bytes(b"alpha");
        assert_eq!(
            ExpectedStateRequest::Content {
                version_hex: version.to_hex(),
            }
            .into_expected_state()
            .expect("content"),
            ExpectedState::Content(version)
        );
    }

    #[test]
    fn invalid_base64_and_content_versions_fail_as_public_input_errors() {
        assert!(decode_bounded_base64("%%%", 1024).is_err());
        assert!(parse_content_version("not-a-version").is_err());
    }
}
