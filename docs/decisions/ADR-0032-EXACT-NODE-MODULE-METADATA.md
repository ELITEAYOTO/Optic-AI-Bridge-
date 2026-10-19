# ADR0032 — Metadata for exact isolated Node module reads

Status: NATIVE VALIDATED CANDIDATE / EXACT CI REQUIRED
Date: 2026-10-19

Node module resolution needs attributes and traversal of the directories leading
to individually authorized source files. Promote only the alpha's directory
metadata primitive and launcher integration, independently of its experimental
Electron debugger, private application probe and HTTP adapter.

The trusted launcher derives the directory set from its existing exact read-file
list (at most32 files), never from a separate caller-selected recursive grant.
At most64 directories including the selected workspace root receive non-inherited
FILE_READ_ATTRIBUTES, FILE_TRAVERSE and SYNCHRONIZE rights for the fresh profile's
SID. No FILE_LIST_DIRECTORY, content read, creation or write rights are added.
No ancestor above that workspace receives an entry. An empty read list grants no
directory metadata. Existing policy, process ownership and launcher protocol stay
unchanged; this native primitive is not an MCP tool or an authorization source.

Each grant pins the root, every intermediate directory and the final object
without delete sharing. The primitive rejects parent components, paths outside
the root, more than64 intermediate components and reparse points, including an
intermediate alias whose target is still inside the workspace. Canonical final
paths must match the pinned chain. Explicit revoke removes the profile SID entry;
Drop remains best-effort. Named/reused profiles cannot obtain external ACL grants.
As with existing exact-file grants, the fresh profile and its owned process must
remain alive through native draining; removing an ACL is not a claim to invalidate
handles that an already-running child opened earlier.

Native tests must prove nested real Node require succeeds for two exact files,
while sibling content, directory listing and file creation remain denied. Separate
native probes test metadata-only access, explicit and Drop revocation, root and
intermediate rename pinning, path/alias refusal and named-profile refusal. Node
remains an optional selected runtime, required only by its Windows CI gate. No
runtime installation ACL is changed and no interpreter becomes required by core.

This is source-read compatibility, not writable build admission. Cargo/npm scripts,
dependency restore, private output collection, hard runtime storage bounds and GUI
execution remain separate gates. No Electron sandbox fallback is promoted.

Local validation: both metadata boundary probes, all seven launcher integration
tests, mandatory installed-Node proof, the full Windows workspace suite and
all-target/all-feature Clippy passed. One exploratory suite run incorrectly set
CI-only hosted-toolchain expectations locally and rejected the newly installed
Python shim layout; rerunning with ordinary local expectations passed. No test
assertion or production gate was relaxed to hide that configuration mismatch.

Initial CI is not a passed merge gate: run39284540811 failed Git fixture
initialization at an already-existing hook while native compile/pipes gates
passed. The preexisting test helper used clock-only names and create_dir_all.
A test-only PID/sequence plus create-new-root correction now refuses shared
roots;16 forced equal-clock parallel allocations and all six merge tests pass.
No production Git operation or assertion was weakened; updated exact CI required.
