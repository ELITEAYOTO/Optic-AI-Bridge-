# H-02 explicit native public seal publication

Date:2026-10-14. Windows x64 disposable fixture, not credential storage.

The optional native variant creates one fixed staged seal relative to a held
root using NtCreateFile FILE_CREATE/share0. It refuses existing stages and has
no path-based or unsupported-platform fallback. Root/file metadata and final
names are observed; this does not authorize arbitrary roots or exclude hostile
ancestors during the initial root selection. Baseline/record election/reader
still use the earlier path-based public fixture.

## Ownership and publication

The owned NT source handle remains open throughout publication. One non-inherited
DuplicateHandle copy is transferred to an explicitly owned CRT descriptor;
FileIO is a borrowed closefd=false view. Exact public write/flush/readback uses
that view. The view and CRT descriptor close before publication, but the NT
source reference stays live. A failed view construction closes the owned
descriptor exactly once; uncertainty blocks publication and never retries.
Re-entry is rejected before cleanup can release live ownership.

NtSetInformationFile FileLinkInformation11 uses the held root, fixed final leaf
and replace=false. After observing two links, FileDispositionInformation13 marks
the owned source for removal at close. No further source-handle operation occurs
after successful disposition, except close. Root and source handles are then
closed with existing sticky uncertainty handling. No Path.unlink, absolute link,
replacement or copy fallback is used by this variant.

The original share0 source reference prevents ordinary reader opens while live,
including after the final link appears. Actual close/disposition removes the
source name, leaving the final one-link seal readable. This is the observed
Windows fixture behavior, not all-volume/OS conformance or durable auth commit.

## Evidence

Ten native regressions cover actual link/disposition without path unlink, live
sharing denial, failed disposition, existing stage preservation, failed FileIO
view construction, CRT close uncertainty, actual destination collision, ambiguous
NT completion, re-entry ownership and actual process exits after link/disposition.
Local full Windows suite:129 cases,126 passed,3 deliberate platform skips.

Actual exit after link (before disposition) leaves two links and reader refusal.
Exit after disposition closes the original kernel handle and leaves a validated
one-link pair without any ACK. The child harness has ten-second waits and bounded
64/4096-byte outputs; only owned children are terminated on failure.

Injected close/completion failures demonstrate fixed refusal/no retry, not an
actual OS close failure. Native integration operations and abrupt exits are real.
All reader results remain fixture_only=true / authority=none. Public digest is
not authentic and no committed credential state is selected.

## Remaining gates

No production runtime/Store changes, user-project enrollment or auth journal.
Logical commit/admission serialization, protected schema, finite generations,
recovery/rollback, root trust and power-loss durability remain unproved. Source
handle-relative removal avoids the portable path unlink step but does not close
the entire path-based fixture's containment or same-user threat model.

Primary references:
- [FILE_LINK_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_link_information): relative root/name and no-replace semantics.
- [FILE_DISPOSITION_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/ns-ntddk-_file_disposition_information): DELETE authority and close-after-disposition constraint.
- [NtSetInformationFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntsetinformationfile): information classes and user-mode naming.

These references guide the explicit candidate; actual tests remain its evidence.
