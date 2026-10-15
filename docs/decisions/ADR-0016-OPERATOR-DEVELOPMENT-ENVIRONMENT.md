# ADR0016 — Operator-selected development environment

Status: ACCEPTED DIRECTION / INCREMENTAL IMPLEMENTATION
Date: 2026-10-16

## Product scope

The next user acceptance target is a genuine Electron application, then a Rust
project, rather than another fixed three-source Node fixture. Installed toolchains
remain external and optional. The operator selects a dedicated project and grants
filesystem, dependencies, execution, network and Git permissions locally. Existing
private repositories, broader roots and multiple projects need separate admission
gates; current public dummy alpha authentication is not a private-project credential.

The existing versioned local JSON configuration is the authority foundation.
Readable boolean presets must compile to explicit capabilities, workspace scopes,
executable/profile identities, network destinations and finite budgets. No model
tool writes policy or self-approves expansion. Full autonomy removes repeated
approval prompts within the operator's scope, not effect ownership, stale-write
checks, cancellation or accounting. A future visual application edits the same
local model; neither YAML nor a mandatory Electron dependency is needed in core.

Live permission changes must fence new admission, advance policy epoch, revoke
affected leases/work and reconcile cleanup before any replacement authorization
is published. File edits are not yet hot reload. Invalid/partially written policy
must not retain silently broader grants or automatically reinstate revoked work.

## First implementation: typed directory creation

DirectoryCreate is distinct from FileWrite, with its own capability and workspace
lease. Local --allow-directory-create-scope and optional mutation.directory_scopes
in JSON mint that authority. Omission grants nothing. MCP exposes
fs_create_directory only with this authority; expected=absent is mandatory.

One action creates one absent leaf under an existing parent. No recursive mkdir,
overwrite, implicit success for an existing directory, delete, rename or shell
effect is hidden inside it. Depth is bounded to32 components and request/path/
execution limits remain enforced. Windows pins every ancestor without delete
sharing, refuses reparse points and verifies the root's captured native identity
before creation. Authorization admission stays held through the operation.
Other platforms fail closed until the corresponding native boundary is proven.

The effect is irreversible in the policy model: it does not promise file-journal
rollback, durable exactly-once or continued absence/presence after other local
actors run. An interrupted caller must inspect the parent; no automatic retry or
recursive cleanup is permitted. Existing file mutation recovery is unchanged.

Native tests must cover existing file/directory conflicts, missing parents,
prefix boundaries, absent grant/precondition, revoked authority, reparse parents,
root replacement and creating a versioned file inside the new directory.

## Remaining delivery gates

1. Map these primitives into a generated full-project HTTP tool surface with
   bounded directory/source/disk inventory and larger independently reviewed files.
2. Extend contained process writes/modules/children and package-cache ownership;
   prove dependency download and install scripts with explicit egress. An npm
   name allowlist alone does not contain arbitrary lifecycle/build scripts.
3. Build/test an AI-generated Electron project. GUI launch is explicit operator
   authority; the fixed existing viewer does not prove arbitrary main/preload code.
4. Persist bounded project handoff and sanitized owned-task/crash evidence,
   readable as untrusted context. Optic cannot access hidden FS model memory or
   diagnose FS backend crashes without FS-provided evidence.
5. Repeat with cargo check/test/build and the actual installed Rust/native linker.

No new release may advertise all these gates as completed by a directory tool.
