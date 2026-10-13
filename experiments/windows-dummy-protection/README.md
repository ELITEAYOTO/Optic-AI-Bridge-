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

Twelve tests declared: on Windows11 execute/1 non-Windows branch skips; on Linux8
execute/4 native cases explicitly skip. Windows cases include actual silent
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
