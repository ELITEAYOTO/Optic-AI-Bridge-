# ADR-0014 — Source ancestor metadata for isolated process reads

Status: IMPLEMENTED / NATIVE VALIDATED AT4b5ca2e
Date:2026-10-14

## Context

The Node alpha proved that a process can read exactly granted source bytes but
ordinary require('./math.js') cannot resolve the module. Node first queries the
source directory. Exact-file data permissions alone do not support that lookup.
Granting directory read/listing or ambient filesystem rights would be excessive.

## Decision

An operator-authorized process source-file read grant includes non-inheritable
FILE_READ_ATTRIBUTES, FILE_TRAVERSE and SYNCHRONIZE on ancestors inside the selected workspace,
including its root. It does not include FILE_LIST_DIRECTORY, file data, writes,
creation, delete or ancestors above the workspace. No model-controlled selector
or new permission-granting tool is introduced. Rust ProcessRun/lease ownership,
class/approval/isolation/resource checks remain unchanged.

The launcher derives at most64 unique metadata directories from at most32
already authorized file reads. Fresh AppContainer SID only. Directory targets
and workspace roots are opened without following final reparse points, checked
using handle-derived component paths, and held without delete sharing. Grants
are revoked by exact handle on explicit revoke or best-effort Drop. Crash-left
ACEs cannot be reused by a later randomly named profile. This is not an ACL
anti-rollback or same-user adversary defense claim.

Node profiles may retain local symlink-resolution flags to avoid realpath
metadata probes above the workspace. These flags do not grant filesystem rights.
Whole directory listing/read/write grants, dependency trees, network and build
output write remain separate gates.

Native attribute-only CreateFile opens were denied without SYNCHRONIZE; this
right permits completion synchronization on the directory handle, not listing
or data access. Older Node22/libuv also uses a synchronous attribute handle;
the newer CI runtime's fast stat path masked the missing right. This correction
must pass native attribute/revoke/deny tests and the Node22.15.1 runner proof.
See [Microsoft synchronous file open contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-zwcreatefile)
and [libuv attribute stat implementation](https://github.com/libuv/libuv/blob/v1.49.2/src/win/fs.c).

## Acceptance

Native tests must prove directory metadata access, listing/read/create denial,
outside-workspace/reparse rejection, root rename denial while held, explicit
revocation and actual relative Node module resolution with sibling denial.
No production readiness or successful new native test is claimed before CI.

Acceptance passed in CI779 and real-link31. The matching artifact also passed
locally with Node22.15.1, including relative import, failing assertion, exact
source correction and successful repeat test. This is a generated-project
check/test milestone, not general filesystem writes, npm or server readiness.
