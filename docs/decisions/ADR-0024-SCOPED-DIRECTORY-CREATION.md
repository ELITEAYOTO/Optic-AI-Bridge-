# ADR0024 — Separately scoped absent directory creation

Status: IMPLEMENTED CANDIDATE / NATIVE VALIDATION REQUIRED
Date: 2026-10-17

Promote the existing generated-alpha directory primitive independently of the
experimental adapter. A project needs directories without receiving delete,
overwrite or shell authority. DirectoryCreate is a distinct typed effect and
capability with its own session-owned, epoch-bound task lease and workspace
scopes. FileWrite does not imply it. Local --allow-directory-create-scope accepts
all or prefix:<workspace-path>; config mutation.directory_scopes is optional and
defaults to no authority. The existing outside-workspace mutation state setup is
required. No model tool edits this configuration or renews authority.

MCP advertises fs_create_directory only when locally provisioned. Its strict
request is {path, expected:"absent"}; unknown fields and missing absence intent
are rejected. One action creates one leaf under an existing parent. Existing
file/directory, missing parent and reparse ancestors are errors, never implicit
success or recursive creation. The existing path/request/execution bounds apply;
depth is at most32 components. Paths stay normalized workspace-relative.

Windows captures root identity at service construction, pins the root and every
ancestor without delete sharing, rejects reparse points, then calls a single
absent-directory create while retaining session admission. Other platforms refuse
the operation until a native boundary is proven. Public visibility is not a
substitute for policy: every call checks live capability, task/epoch and scope.
Transport and blocking-I/O permits remain held through actual synchronous work.

Directory creation is irreversible in the effect model. It is not part of the
file transaction journal and does not claim durable exactly-once replay or
rollback. Lost replies require inspection of the parent before another decision;
no automatic retry or recursive cleanup. Concurrent local actors can change the
result after creation. It does not grant build outputs, package installation,
network access, process execution or an existing private-project credential.

Validation: native absent/existing/file/missing-parent/depth/root-identity/reparse
cases, distinct scoped and revoked authority, config isolation and strict public
schemas. Exact Windows/Linux CI and verified native reproduction precede merge.
