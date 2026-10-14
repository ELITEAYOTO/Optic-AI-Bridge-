# H-02 initial public-marker publication candidate

Date: 2026-10-14. Disposable Windows x64 experiment only, not registry state.

`publish_fresh_public_fixture` requires a live prepared public dummy parent and
creates the fixed private child in the same operation. Existing children are
never adopted, repaired or reset, including a child whose state was deleted.
The child root and writer marker are still created by path: this is not hostile
ancestor exclusion or a production enrollment authority.

The fixed scratch uses explicit root-relative NtCreateFile. A separate internal
candidate requests DELETE only on that scratch handle; the earlier candidate's
default rights remain unchanged. After one bounded public write and flush, it
seeks and verifies exact bytes on the same handle. NtSetInformationFile uses
FileRenameInformation10, the held root, the fixed `state.protected` leaf and zero
replace/flags. There is no absolute-path fallback or overwrite-on-collision retry.
After rename it checks final handle names, file kind/link count/size and exact
bytes again. Success is returned only after scratch and writer contexts close.

The initial Win32 SetFileInformationByHandle experiment returned error87 for the
relative packet on this host. The final implementation selects NtSetInformationFile
explicitly; it does not try multiple APIs at runtime to evade failure.

## Evidence

Nine regressions cover unopened parent refusal, real fresh publication and no
reset of an existing child with missing state, preservation of existing destination
file/directory/hardlink, one-attempt native failure, repeated publish/write refusal,
ambiguous native completion, post-rename readback failure, close uncertainty and
incorrect public bytes of the expected size refused before rename.
Last local Windows suite: 88 cases, 85 passed, 3 deliberate platform skips.
Completion, readback and close failures are injected; successful rename and
destination collisions are actual native operations. CI evidence is recorded
separately when available.

An error after rename can leave the destination present with valid marker bytes.
It does not mean rollback. The candidate returns no success, never retries and
never deletes or auto-promotes scratch on restart. It is synchronous and adds no
caller-cancellation or I/O deadline claim. OS flush plus orderly reopen is not
power-loss durability. Same-user hostile writer exclusion, general ACL policy,
protected registry composition, state replacement/CAS and rollback resistance
remain separate gates. The public marker must never decode as auth metadata.

Primary references:
- [NtSetInformationFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntsetinformationfile), including the user-mode Nt naming note.
- [FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information), relative root, access and collision semantics.
- [SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle), the separately tested Win32 API.

These references guide the selected experiment. Local/native CI results do not
establish every filesystem or OS version's conformance.
