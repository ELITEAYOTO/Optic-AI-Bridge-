# H-02 Windows CurrentUser protection — PUBLIC dummy fixture only

Standalone local API experiment, stacked on #195 docs de3ea457. No production
Rust/storage integration, server, MCP tools, real credentials, project/process/
Git/mutation/session_cancel authority, CLI file/decrypt selector or enrollment.
Only one fixed public fixture can be protected: schema plus SHA256 digests of
published dummy A/B values. These unsalted public hashes are NOT a production
credential verifier. No random real connection credential is minted or used.

Native CryptProtectData/CryptUnprotectData through Python ctypes, fixed System32
libraries. Flag UI_FORBIDDEN1, NULL prompt/description/reserved parameters; no
LOCAL_MACHINE flag. Additional entropy is a fixed PUBLIC purpose label, not a
secret key or authority. Native output buffer zeroed/freed via LocalFree; Python
immutable byte copies are not claimed to be reliably zeroized. No fallback to
plaintext on unsupported platform/API failure. No change to Windows security
policy or inspection/export of existing application credentials/key stores.
DPAPI uses Windows-managed CurrentUser profile key infrastructure through its
normal APIs; this experiment does not implement or export those keys.

check_public_fixture accepts only byte blobs with fixed fixture magic and <=16KiB,
uses same purpose entropy, decrypts locally, then constant-time compares the WHOLE
plaintext to the exact known fixture. It returns only a fixed match bit/label and
authoritynone, never decrypted bytes. Unknown/corrupted/unexpected payloads fail
with fixed error codes, no native exception strings or error-dependent reset.
Application fixture comparison is separate from DPAPI integrity checking. This
is not a general secure state parser: production schema/epoch/owner/tamper policy
still needs design. No arbitrary user data/file is an accepted test input.

Fifty-four tests declared: native Windows52 execute/2 non-Windows branches
skip when symlink creation is available; otherwise one additional native case
explicitly skips. On Linux24 execute/30 native cases skip. Windows cases include actual silent
roundtrip, temporary protected-file roundtrip, corrupted ciphertext rejection,
randomized protection of same public fixture, and fixture produced by a fresh
child process verifying in parent under same CurrentUser context. Other tests
prove input bounds before native call, unexpected plaintext not returned/echoed,
public-only fixture and fixed flag. Child writes only its disposable test fixture,
has10s timeout, outputs fixed PASS/FAIL; no shell or arbitrary PID control.
No actual other-user/other-machine denial test was run; do NOT report such a proof.

Four portable fault-injection cases additionally exercise the native wrapper:
an allocated output is zeroed/freed even when the API reports failure; oversized
output is freed without copying or attempting an out-of-bounds zero; LocalFree
failure prevents a success result; loader failure returns a fixed code without
fallback or exception-message echo. These are controlled fake API results, not
claims that Windows naturally produced these faults. Native roundtrips remain
separate real Windows tests.

Microsoft's documented default user scope has exceptions/recovery context; it
is not protection from same-user malware/admin, rollback, replay of valid prior
state, account password-reset/recovery loss, hardware/power-cut durability or
transport theft. Whole-fixture matching here does not solve production epoch
rollback. Secret/verifier generation, local UI copy/show semantics, filesystem
ownership/ACL, schema/atomic persistence/antirollback/recovery contracts, key
rotation and real effect boundaries remain H-02/H-03 gates. No issue closed.

This experiment does not replace #189's dummy registry or expose it remotely.
No FranceStudent test is needed for the isolated native API behavior.
Windows helper (reuse the isolated existing Python environment):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\windows-dummy-protection\scripts\Test-WindowsDummyProtection.ps1
```

No Start/Initialize/tunnel steps, no persistent fixture deliverables. Actual
DPAPI blobs generated in tests are temporary and are not committed/exported.
Keep Draft, do not merge. H-01 positive host overlap is still unmeasured and
#178 is separate; verified sequential host sample is in grouped-parallel ledger.

## Disposable protected registry composition

`protected_registry.py` combines the existing public-dummy schema/lifecycle and
cooperating-writer lock with native protection. Only schema1/A/B metadata is
accepted; no real verifier/token enrollment, listener or CLI file selector.
Tests use owned TemporaryDirectory fixtures; no application credential store.
Files are fixed `state.protected` and `state.protected.pending`, capped16KiB;
decoded schema capped4096bytes. Pending bytes are never promoted on reopen.
The inherited rotation/revocation path acknowledges only after replacement;
failure blocks the live registry. Corrupt/missing state never resets implicitly.
Eight native integration cases prove reopen lifecycle, replacement failure,
protection failure, corruption refusal, cooperating writer exclusion/missing
state and explicit old-valid-ciphertext replay. Two fixed subprocess cases
additionally prove acknowledged rotation/revocation survive `os._exit(77)`
without registry close, and an abrupt exit immediately before replace never
promotes pending bytes. The OS releases the writer lock on process exit; the
parent reopens and verifies committed state. Children run fixed test source
against owned temporary fixtures, timeout10s, no shell or PID selector, fixed
acknowledgment text. This is process-exit evidence, not hardware/power-loss
durability proof. The Windows protected payload contains dummy metadata, not
production verifier material. The native helper accepts only two fixed internal
purposes: `fixture` and `registry`, each with its own PUBLIC entropy label.
Unknown purposes fail before native library loading. Tests protect the same
public payload under each purpose and prove wrong-purpose native unprotect
fails; relabelling a fixture outer prefix cannot make it a valid registry.
Labels are not secret keys or additional authority. Earlier dummy registry
blobs used the fixture label and are intentionally not accepted by this new
registry purpose. No automatic fallback, migration or reset is performed;
these tests use new disposable directories only, never operator application state.

Old valid state replay **is accepted** in the characterization test: protection
does not prove freshness. No ACL/reparse race/hostile local writer containment,
antirollback or recovery guarantee is added. The inherited lstat/path checks
and cooperating lock do not substitute for a native handle-based file policy.

## Decoded state and live drift

The decoded schema additionally rejects active version0 at epoch>1, which no
allowed dummy lifecycle command can produce. Portable tests cover duplicate
keys, unknown/extra schema fields, malformed/bounded input, typed values and
valid lifecycle states. Decryption success alone is never schema validity.

The store retains its first decoded snapshot. Before a subsequent commit, a
different valid on-disk state causes denial rather than overwriting it with a
candidate based on stale in-memory metadata. Failure blocks the live registry.
Successful commits advance the snapshot. This comparison occurs before the
write; it is NOT an atomic compare-and-replace primitive, external live revoke
notification, hostile-writer containment or antirollback across restart. The
existing valid-old-state replay characterization still succeeds on reopening.
Native cases exercise valid changed state preservation, protected unreachable
schema refusal and oversized ciphertext denial before the native API.

## Native prepared read of the protected fixture

`PreparedFixtureRead` opens only the fixed `state.protected` child of an
absolute local-drive directory chosen by the LOCAL test harness. No remote
file/directory selector, production grant, write primitive or ACL mutation.
Tests create only owned temporary fixtures; the helper itself is not an
authorization policy and must never be exposed as an arbitrary file tool.

Fixed System32 kernel32, OPEN_EXISTING and non-inheritable handles; root/file
reparse attributes are rejected, file must be disk/non-directory/single-link,
canonical handle paths must match the exact root+fixed filename (no prefix
authorization). Root path is reobserved after file open. Path buffer4096,
ciphertext16KiB; size/read mismatch fails closed. Root handle omits delete
sharing; file handle permits read sharing only. File bytes are read from the
same observed handle, held across native decryption/schema validation.
Close runs on normal/exceptional exit, with fixed failure code. Synchronous
local Windows filesystem calls are not advertised as preemptible deadlines.

Native tests prove read correctness, competing write/file rename/root rename
denial while handles are held, release on exceptions, hardlink refusal,
file/root symlink refusal where the runner permits creation, fresh-child write
denial until the parent releases its pin, empty/oversized/
directory target and missing-file denial, and pin retention during decryption.
Failure to create test symlinks is an explicit skip, never positive reparse proof.
Read is effectively one-shot due to the file position; use a new context to read
again. No seek/reuse/reset behavior is promised.

Two portable fake-API cases check that a CloseHandle failure still attempts
both cleanups and denies success, and an invalid/oversized ReadFile count is
rejected before bytes are copied. These simulated failures are separate from
native measurements. A failed native close is reported, not a claim that the
OS released that failed handle. Uncertain cleanup stays failed on later close
calls; no silent later success or blind close retry. The context cannot reopen.

This closes an observation/read gap in the disposable registry, NOT the entire
filesystem boundary: ancestor replacement races, hostile local writers, ACLs,
writer-lock file identity, path-based pending creation/replace, antirollback and
power loss remain unproved. Protected writes still use the earlier path-based
experiment. No production integration is enabled.

Native read API references reviewed2026-10-13:

* [Microsoft CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew):
  sharing, OPEN_EXISTING, reparse-point and directory handle flags.
* [Microsoft GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew):
  normalized handle path and bounded buffer result.
* [Microsoft GetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle):
  handle-observed attributes, file size and link count.

## Fixed public pending-file creation candidate — 2026-10-14

## Explicit root-relative pending candidate — 2026-10-14

`RootRelativeFixturePending` explicitly selects NtCreateFile with the held root
directory handle and one fixed Unicode leaf. It does not call absolute
CreateFileW for scratch creation and never falls back to the path-based candidate.
Windows x64 ABI sizes, two-byte wide chars, fixed System32 ntdll, synchronous
non-directory/open-reparse options, FILE_CREATE, zero sharing and non-inheritable
object attributes are fixed. No DELETE/WRITE_DAC/WRITE_OWNER authority is requested.
Success requires exact NT status0, completion status0 and FILE_CREATED2 with a
valid owned handle before inherited empty-size/identity/public-write checks.

Seven new relative cases cover inactive pin before native load, actual relative creation
with absolute-create disabled, existing target preservation/no fallback,
mutation of the stored path string while creation still uses the held root,
loader/platform failure preserving absence, and simulated ambiguous completion/owned
handle cleanup. A further native child abrupt-exit case verifies kernel release
of its existing writer pin without Python cleanup. Full local Windows78 cases:
75 pass/3 platform skips. Simulated
completion checks are distinct from actual native creation evidence.

This observes interoperability with the existing Win32-opened directory handle
on the tested Windows x64 system. It is not universal filesystem/driver/platform
conformance. Inherited path rechecks may still refuse root-path drift; no hostile
ancestor race, ACL/enrollment, final replacement, Store integration, state
promotion, rollback or power-loss durability guarantee is claimed. No real
payload, credential or remote tool accepts this primitive.

Primary reference: [NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile)
documents RootDirectory/ObjectName relative naming, FILE_CREATE, completion
information and SYNCHRONIZE for synchronous nonalert I/O.

## Fixed public pending-file creation candidate (path-based proof)

`PreparedFixturePending` accepts only a held `PreparedFixtureLock`. It creates
one fixed `state.protected.pending` with native CREATE_NEW and zero sharing;
existing files/directories/hardlinks/symlinks are refused without repair or
removal. Same-handle kind/link/empty-size/final-path observations precede a
single synchronous WriteFile of a fixed public marker. Exact byte count,
FlushFileBuffers, observed final size and final path are required. No arbitrary
payload/leaf/path selector, state promotion, replace or delete operation exists.

Eight new cases cover unopened/closed pin denial, invalid/partial write count,
failed flush, sticky close failure, same-handle native write/share refusal,
re-entry/repeat-write refusal, existing file/directory/link preservation and
native symlink refusal when available. Local Windows70 cases:67 pass/3 platform
skips. Failed creation/write leaves inert scratch bytes; no old scratch is
promoted or implicitly cleaned up. This is not a persistence acknowledgement.

Creation still uses a path derived from the observed root, not a root-relative
native create. Root/path rechecks do not close an ancestor replacement race and
cannot undo an empty scratch creation if a containing path was changed. This
partial fixture proof is NOT general authorized filesystem containment. ACL,
ancestor stability, final replacement, state freshness and power-loss durability
remain separate gates. Synchronous native I/O has no claimed latency bound.
ProtectedStore remains unchanged and does not use this candidate.

Primary API references: [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew),
[WriteFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile),
[FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers).

## Exclusive fixed writer-lock pin candidate (existing proof)

`PreparedFixtureLock` opens only an existing `writer.lock` in a disposable local
fixture. The fixed public marker must be exactly one byte `0`. Native
CreateFileW uses read-only access, zero sharing, OPEN_EXISTING and reparse-point
observation. The root stays pinned against rename; same-handle kind/link/size
and final-path checks precede bounded marker read. No lock create, truncate,
repair, removal, state mutation or credential handling is added.

Eight new cases cover real same-process and fresh-process refusal, release,
write/delete/file/root rename denial, missing/bad marker/hardlink preservation,
context-error cleanup, sticky close failure, file/root symlink refusal when
native creation is supported and re-entry preserving an active pin. Existing
read/protection tests remain in the same suite. Local native Windows62 cases:
59 passed,3 deliberate unsupported-platform skips; symlink support is reported
separately if absent on another runner.

This is an isolated candidate, not wired into Store/ProtectedStore. It excludes
cooperating users of that fixed lock path while held; it does not prevent an
actor ignoring the lock from writing state directly. Full ancestor/ACL policy,
lock initialization, pending identity, final replacement/atomicity and rollback
remain open. Refactoring the native API loader leaves prepared state-file read
semantics unchanged. No production session or remote endpoint uses this pin.

Primary API contract: [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
documents zero-sharing refusal, OPEN_EXISTING and handle-lifetime sharing modes.

## Read-only native owner/DACL observations (existing proof)

`observe_fixture_security` accepts an already opened local `PreparedFixtureRead`,
queries its root/file descriptors by handle and the current PROCESS TokenUser
(TOKEN_QUERY only). It does not query impersonation identity, inspect another
process/account or change any privileges/ACL/security policy. No arbitrary
path/SID input, lookup of account names, SACL access or remote endpoint.

Output contains only authoritynone/observation-only, owner-match bits, DACL
presence/null bits, ACE count and limited ACE category counters. No username,
SID string/raw descriptor/token buffer/credential is returned or logged.
Counts distinguish simple ALLOW write-related mask bits for process user,
LocalSystem, BuiltinAdministrators or other; DENY/inherit-only/unknown-type
counts are separate. Unknown/conditional/object ACE types remain unparsed.
These categories are observation, not trust approval or effective access.
ALLOW counters include inherit-only entries and do NOT account for deny order,
group membership, conditions, owner rights, inheritance evaluation or security
descriptor changes after the snapshot. DACL existence/owner match/zero other
counter is NEVER interpreted as a safe storage verdict.

Caller token buffer4096, known SID buffers68, descriptor decode ceiling64KiB,
ACE ceiling128; native returned pointers/spans/alignment are checked before
SID/ACE interpretation. Native allocated descriptor is freed via LocalFree and
query token closed on normal/error exit; failures return fixed codes, no native
error strings. This bounds decoding, not every internal OS allocation or I/O.

Seven cases cover ACL/token-input bounds, pointer bounds before dereference,
write-mask bits, unknown ACE handling, and actual read-only owner/DACL query
on pinned temporary files. A local diagnostic sample observed matching root/
file owner, non-null present DACL and9 ACEs each. This is one machine snapshot,
not a default policy. No ACL creation/repair or admission integration yet.

Primary references reviewed2026-10-13:

* [GetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo):
  handle-based owner/DACL query, READ_CONTROL, LocalFree and concurrent-change caveat.
* [GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation):
  token query rights and caller-buffer sizing.
* [GetAclInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getaclinformation)
  and [GetAce](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getace):
  bounded counts and returned ACE pointers.
* [CreateWellKnownSid](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-createwellknownsid)
  and [ACCESS_ALLOWED_ACE](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-access_allowed_ace):
  fixed identity categories and ACE layout; no effective-access claim.

## Creation-time explicit ACL candidate for a new dummy directory

`create_private_fixture` accepts only an opened local dummy parent context and
creates one fixed `private-dummy-fixture` child. A caller-owned512byte ACL and
absolute descriptor are initialized IN MEMORY, then supplied in non-inheritable
SecurityAttributes to native CreateDirectoryW. Three fixed ALLOW/OI/CI/full-file
trustees: process TokenUser, LocalSystem and BuiltinAdministrators. Owner is the
process user for the new directory; DACL is protected from parent inheritance.
Ordinary child-file creation inherits the DACL, not this explicit owner: the
creating token supplies its default owner, which differed on the Windows CI
runner. Child owner match remains an observation, never an admission guarantee.
No arbitrary SID/mask/
SDDL input or caller-chosen filename. No existing-object SetSecurityInfo/repair,
account changes, privilege enabling, production storage or real credential.

This is a candidate fixture policy, explicitly NOT protection from System,
administrators or same-user code. Existing file/directory child causes failure
without changing contents/security. Creation failure does not retry with default
permissions. Parent handle final path is reobserved; an uncertain outcome returns
no success and is never followed by blind recursive deletion/adoption/reset.
Native query token closes on normal/error exit; descriptors/SIDs/ACL are bounded
caller-owned buffers, no plaintext SID conversion/logging.

Five cases cover closed-parent refusal before loading, actual creation-time
protected three-entry root DACL and child inheritance, existing directory/file
preservation, and protected dummy registry rotation/revocation/reopen inside the
created fixture. Tests touch only owned temporary roots. New root and file
observations have no other-category simple write ACEs in these runs. This is
structural evidence, not a cross-account effective-access test.

Parent delete-child rights, ancestor replacement, system/admin intervention,
lock-file identity, final write/replace atomicity and rollback remain separate.
Parent permissions may permit denial-of-service/renaming while no handles pin
the child. No persistent path/enrollment integration or availability guarantee.
The protected registry's normal initialize path is unchanged: composition tests
explicitly write initial PUBLIC dummy state after their own fresh creation;
an existing operator directory is never adopted implicitly.

Primary references reviewed2026-10-13:

* [CreateDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectoryw):
  creation-time descriptor, default inheritance and existing-target refusal.
* [InitializeSecurityDescriptor](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-initializesecuritydescriptor)
  and [SetSecurityDescriptorDacl](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-setsecuritydescriptordacl):
  in-memory descriptor construction, not permission repair of existing objects.

Primary references reviewed2026-10-13:

* [Microsoft CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata):
  default user scope, machine-scope flag, noninteractive flag, LocalFree ownership.
* [Microsoft CryptUnprotectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata):
  entropy matching, native output freeing, integrity behavior and application
  integrity checks. No reliance on a specific native tamper error number.
* [Microsoft example/scope limits](https://learn.microsoft.com/en-us/windows/win32/SecCrypto/example-c-program-using-cryptprotectdata):
  account/machine/recovery caveats; these are documentation, not measured cases.
* [Microsoft LoadLibraryEx](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw)
  and [LocalFree](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-localfree):
  System32-only loader flag and native buffer release/failure contract.
# Private-root composition checkpoint — 2026-10-14

One additional native regression composes the creation-time private dummy root,
existing fixed writer pin and root-relative pending write. Read-only descriptor
queries on the live handles verify the root owner/protected DACL and three
parsed lock/scratch entries with no other write-allow category. The test borrows
the scratch handle for observation and restores lock ownership in `finally`.
It publishes no committed state and does not test effective access by another
principal. Current local Windows suite: 79 cases / 76 passed / 3 platform skips.
# Initial public-marker publication checkpoint — 2026-10-14

The explicit fresh-only wrapper creates a private child in the same operation,
writes fixed public scratch bytes, then selects root-relative NtSetInformationFile
without replace/fallback. Same-handle readback and context closure precede success.
Existing child with missing state is never reset. An error after rename can leave
state present without ACK; no retry, rollback, power-loss or production claim.
Local Windows: 88 cases / 85 passed / 3 platform skips. See
`docs/security/H02_INITIAL_PUBLICATION_FIXTURE.md` for evidence and limitations.
