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

Thirty-nine tests declared: native Windows37 execute/2 non-Windows branches
skip when symlink creation is available; otherwise one additional native case
explicitly skips. On Linux15 execute/24 native cases skip. Windows cases include actual silent
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
file/root symlink refusal where the runner permits creation, empty/oversized/
directory target and missing-file denial, and pin retention during decryption.
Failure to create test symlinks is an explicit skip, never positive reparse proof.
Read is effectively one-shot due to the file position; use a new context to read
again. No seek/reuse/reset behavior is promised.

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
