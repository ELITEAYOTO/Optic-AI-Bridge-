# H-02 process-restart persistence — PUBLIC DUMMY fixture only

Owners #172/#173; stacked on #188 consolidation head
`c9f00d9deaa28056107cb05c2d9e0d1524710978`. Initial main check remains
`156653a406f01383e2fc14cada513963453b04f3`. No Rust changes or remote project
authority. Keep Draft. Full prior findings/admin comparison:
`../../docs/research/FRANCESTUDENT_FINDINGS_AND_ADMIN_COMPARISON_2026-10-12.md`.

## Scope and contract

Earlier dummy registries reset on process restart, restoring initial dummy auth.
This separate fixture tests that an ACKNOWLEDGED local rotation/revocation remains
effective after process death/restart. Exactly one remote tool:
`persistent_credential_probe`, zero-authority diagnostic principal/epoch only.
No workspace/files/process/Git/mutation/session_cancel/session or remote control
tool. Infrastructure writes only fixed local fixture metadata under operator
control; no remote call can write files or choose a fixture path.

The local Python CLI fixture-dir option is for operator/test infrastructure only;
the Windows helpers use their fixed `.fixture-state` sibling directory. Explicit
Initialize creates ONLY a brand-new directory. Existing directory cannot be
reinitialized, even if state.json is absent. Start NEVER initializes/resets on
missing/corrupt state; it fails closed. No automatic secret recovery/reset feature.
Git/archive ignores the fixture. Published dummy values remain in code but are
NEVER stored as credentials in the fixture or logged/returned by the server.

Exact bounded JSON schema: version1 plus fixed ownersA/B; each stores independent
16-hex diagnostic principal, epoch1..65535, dummy-version0/1 and active bool.
No raw secrets, handles, arbitrary metadata/history or paths; max4096 input bytes.
Unknown/duplicate fields, malformed/oversized data, duplicate principals, invalid
types/ranges and inconsistent initial epoch fail closed. Symlink/reparse/nonregular
fixture/state paths are rejected by observation. This is not race-proof hostile
filesystem containment or authenticated/rollback-resistant state storage.

An OS advisory/exclusive cooperating-writer lock (Windows msvcrt, Unix flock)
is held while server loads/uses the fixture; second writer fails. OS process exit
releases lock, so crash does not require deleting a stale-lock marker. Rotate/
revoke runs under Registry lock: validate current committed file, write one fixed
pending scratch file, flush/fsync it, replace committed state, then publish memory
and ACK persisted=true. Unix also fsyncs directory. Pending scratch is NEVER auth
input or promoted on startup. A subsequent local commit may remove the fixed
ordinary bounded stale scratch file under writer lock. No unbounded temp files.

Missing/corrupt current state during local commit is not overwritten: no ACK,
this live registry blocks all owners. Failed/uncertain commit returns only a fixed
code and blocks this process, never exception text. A failed operation has NO
promise of persistence; after operator restart the valid disk state may represent
old or new commit depending on failure point. Successful persisted=true is the
boundary tested here. No human/machine power-cut or storage-hardware durability
claim, especially not Windows power-loss directory durability.

Same two dummy owners, one rotation per owner across restart, per-request strict
Bearer + secure headers, no positive auth cache, pre-SDK stale-epoch recheck.
Local stdin controls only, inherited16-line budget/read65-char ceiling. Principal
is preserved, epoch advances, old dummy stays denied. Revoke persists; independent
B remains active. Registry loading is not SessionResolver or project authority.
Inherited ingress/concurrency/time bounds remain; banner persistent-dummy-auth-v1,
defaultport8004, loopback host policy unless explicit synthetic tunnel switch.

## Automated proof and limits

11 new deterministic/native tests: acknowledged rotate/revoke survives reopen,
one writer, missing state no auto-init, corrupt/duplicate/oversized/schema/type/
range failures, no raw credentials in persisted file, failed write blocks without
echo/ACK, pending not promoted, native symlink denial where platform permits,
actual child kill BEFORE replace retains old committed state and releases lock,
deleted/corrupt live state cannot be overwritten by commit. Native symlink test
may skip when Windows operator lacks permission; do not claim race containment.

Actual loopback HTTP/MCP smoke kills the server abruptly AFTER rotate ACK, restarts
same fixture with new server instance, proves old credential denied/new accepted,
same principal/epoch2; then revoke ACK, kill/restart, proves both A versions denied
and B epoch1 accepted. Exact one-tool discovery and no credential raw bytes in
logs/errors/state. TemporaryDirectory contains all disposable smoke state.
No actual FranceStudent restart proof yet. Prior host tests cover different
finite carrier/handle/lifecycle behaviors; their markers are not reused here.
All24 inherited tests and3 inherited protocol smokes remain separate regressions.

Production still needs generated high-entropy connection credentials, reviewed
verifier/DPAPI/integrity/rollback/recovery contract, safe local UI ownership,
standards OAuth/compatibility decisions, precise rate-limit and admitted-effect
semantics and transactional SessionResolver authority. Public dummy auth is NOT
production authentication. #172/#173 and real-project gates remain open.

## Windows — output review after each step

Extract source into a fresh separate folder. Python3.11+, no Rust. Do not copy old
fixture state, reset files to bypass denial, or expose any real Optic binary.

1. Setup (no state created):
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\persistent-auth\scripts\Setup-PersistentAuthProbe.ps1
   ```
   Expected PERSISTENT AUTH SETUP PASS. Send output and wait.
2. Local tests (their temporary fixtures are removed; manual state untouched):
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\persistent-auth\scripts\Test-PersistentAuthProbe.ps1
   ```
   Expected11 tests + actual restart MCP smoke, PERSISTENT AUTH TEST PASS.
3. Only after review, explicit one-time manual fixture initialization:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\persistent-auth\scripts\Initialize-PersistentAuthFixture.ps1
   ```
   Expected FIXTURE INITIALIZED. Existing directory refusal is intentional: do
   not delete/reset it. Send output and wait.
4. Start existing fixture:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\persistent-auth\scripts\Start-PersistentAuthProbe.ps1 -AllowRandomTunnelHost
   ```
   Expected Authority NONE/persistent-dummy-auth-v1/8004. Send output and wait.
5. Guided new tunnel at port8004, HTTPS URL + /mcp, original A full-prefix dummy
   and same secure dummy JSON headers. Review exact one-tool discovery and real
   baseline principal/epoch before local rotation.
6. Operator-local rotate-a ACK must include persisted=true; stop ONLY server,
   keep tunnel (or start a new guided tunnel), restart from SAME folder/fixture,
   never Initialize again. Check old denied then A-v2 accepted with same
   principal/epoch2. Each control/restart/host call receives separate output review.
7. Operator-local revoke-a ACK persisted=true; stop/restart SAME fixture; A-v2
   remains denied. B can be tested independently. No state resets or real tokens.
8. Ctrl+C server/tunnel, disable connector. Preserve fixture and safe evidence for
   reproducibility; do not upload raw captures/credentials. State contains dummy
   metadata only, but it is not a production configuration or authority grant.

No draft merge or gate closure is authorized by this proof.
