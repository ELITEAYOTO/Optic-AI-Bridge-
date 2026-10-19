# Electron startup: matched symbol and owned result observation

Date: 2026-10-18. Research evidence; general GUI admission remains blocked.

The locally installed Electron44.7.2 executable has SHA256
1f41127064012c5e74422b2e0dfc35ff0b0fe8d7908b97d01503becd1b7fa9e1.
Its PE debug directory declares electron.exe.pdb, GUID
2957A62F-FD9C-300B-4C4C-44205044422E, age1. Electron's
[official symbol-service documentation](https://www.electronjs.org/docs/latest/development/debugging-with-symbol-server)
identifies the service used to look up that exact key. No runtime installation,
ACL, global debugger/symbol setting or user project was changed.

HTTP206 range reads checked the matching MODULE identifier and located:

- FUNC RVA0x3325720, size0x82:
  sandbox::policy::Sandbox::Initialize(sandbox::mojom::Sandbox, SandboxInterfaceInfo*).
- RVA0x332579e belongs to its line71 record, file411636.
- FILE411636 identifies src/sandbox/policy/sandbox.cc in the release build tree.

The address had already been observed in the owned-child debugger. Symbol
lookup read2,101,248bytes; exact FILE lookup655,360bytes. The complete344,422,319-
byte symbol file and4,111,712,256-byte PDB were not downloaded. Each partial read
required exact206/Content-Range/byte count and retained a SHA256. Lookup required
an actual covering FUNC, not a nearest-symbol guess. These are local retained
research outputs, not a model-facing symbol downloader or a trust/cache API.
The [Breakpad format](https://chromium.googlesource.com/breakpad/breakpad/+/HEAD/docs/symbol_files.md)
defines the address, line and file records used here.

At the same RVA, bounded executable disassembly shows test eax,eax then jne to
int3, with no intervening EAX assignment. The version-matched
[sandbox.cc line71](https://chromium.googlesource.com/chromium/src/+/refs/tags/152.0.7977.149/sandbox/policy/sandbox.cc#71)
checks the result of CreateAlternateDesktop(kAlternateWinstation).

The test-only debugger now samples the low32bits of EAX only at noninitial
breakpoints on the initial thread of its freshly created owned child. It uses
the existing CREATE_PROCESS_DEBUG_EVENT thread handle; no PID/thread lookup,
context write, injected code or new process-control/MCP surface. The x64 context
buffer is explicitly16-byte aligned and unit-tested. The Windows Kernel feature
is enabled only as a platform-crate dev dependency. Existing event/log/deadline
limits and exception handling remain unchanged. This debugger changes startup;
normal execution and explicit outer-only comparison remain separate evidence.

The final aligned local diagnostic observes:
exception0x80000003, RVA0x332579e, primary_thread_match=true, EAX12 at both first
and second chance. The actual compatibility test remains failed, exit0x80000003.
Pinned Chromium152.0.7977.149
[sandbox_types.h](https://chromium.googlesource.com/chromium/src/+/refs/tags/152.0.7977.149/sandbox/win/src/sandbox_types.h)
defines12 as SBOX_ERROR_CANNOT_CREATE_WINSTATION. In the matching
[window.cc](https://chromium.googlesource.com/chromium/src/+/refs/tags/152.0.7977.149/sandbox/win/src/window.cc#18),
that result follows failure to obtain a station from CreateWindowStationW,
including its narrower-access retry when the first call reports access denied.
The observation therefore localizes the Chromium failure to alternate-window-
station creation. It does not measure the failing Win32 call/GetLastError value,
establish access denied, desktop-heap exhaustion or antivirus as the root cause,
or explain FS inference cutoffs. The next useful probe must distinguish those
native results rather than disabling Chromium's inner sandbox.

Local ordinary Windows tests:29passed,5explicit compatibility probes ignored;
all-target/workspace Clippy passed. Exact final-head CI and diagnostic evidence
on disposable Windows remain required. No normal Electron success, sandbox
fallback, dependency restore or production GUI authority is introduced.


## Disposable Windows comparison at28d0ecf

Private Application Staging39157390440/job117763318431 reproduces the same normal
exit0x80000003 and exact breakpoint RVA0x332579e. Its owned diagnostic records
EAX11 at both first and second chance, not the local12. Pinned enum11 is
SBOX_ERROR_CANNOT_CREATE_DESKTOP. Thus the shared failed check is established,
but window-station creation is not the sole universal failing step: the CI host
reaches a desktop-creation failure instead. Exact failing Win32 call and native
error values still need separate measurement in each environment. Do not label
this as universal access denied or presume that an ACL/desktop switch repairs it.
The explicit outer-only comparison passes in that same job; the normal gate
remains red and no fallback is enabled. This comparison is evidence from the
code head28d0ecf; later documentation commits do not imply new native execution.


## Separate native station API observation (2026-10-19)

A new ignored test runs a fixed helper in its own ephemeral AppContainer and
kill-on-close job, with the existing bounded output and ten-second deadline.
It reads only the current station DACL into an aligned fixed64KiB buffer, then
attempts the two Chromium access masks with that copied self-relative descriptor.
It records each Windows HRESULT in the calling child, not the debugger thread.
This is a separate API experiment: Chromium reconstructs an absolute descriptor;
this helper does not reproduce Electron's full token/startup/security state.

The local native run observed successful current-station/DACL queries, followed
by HRESULT0x80070005 (Win32 access denied) for both GENERIC_READ|WINSTA_CREATEDESKTOP
and WINSTA_READATTRIBUTES|WINSTA_CREATEDESKTOP. The child exited0 and the job was
drained. A passing observation test only means its evidence was collected; the
normal Electron compatibility gate remains failed. These results support an
access-denial hypothesis locally but do not directly capture Electron's native
GetLastError, explain CI's desktop-stage failure, or establish a safe repair.

No current-station ACL is changed. No default descriptor, desktop creation,
SetProcessWindowStation, inherited station handle or production GUI authority is
introduced. Any returned station reference is closed immediately; the borrowed
current-station handle is never closed. Windows generates the requested name;
existing-object references, if returned, are not treated as newly owned objects.
The feature is a Windows test-only dependency. The dedicated Windows workflow
collects this observation before its unchanged Electron compatibility gate.

API contract: [CreateWindowStationW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowstationw).


### CI comparison and descriptor-format control

At d8ea38b, Windows CI39193400918/job117870855666 successfully created and closed
station references for both access masks, while the unchanged Electron test
still failed0x80000003 and the owned debugger observed result11. The outer-only
comparison passed. Local station access denial is therefore not portable to CI.

A subsequent probe revision compares the self-relative DACL descriptor against
its native MakeAbsoluteSD conversion, retaining all component storage through
the calls. Memory is bounded to the64KiB query buffer, four64KiB conversion
buffers and one fixed SECURITY_DESCRIPTOR; there is no dynamic sizing retry.
The local run again observed HRESULT0x80070005 for all four format/access pairs.
This rules out descriptor representation as the sole cause in this helper on
this PC, but still does not capture Electron's own native error or identify a
safe permission repair. The parent now requires all four observations, so a
query/conversion failure cannot silently count as complete comparison evidence.
No new desktop, process-station switch or default ACL is introduced.

Conversion contract: [MakeAbsoluteSD](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-makeabsolutesd).


### Narrow CI result11 before assuming CreateDesktop failed

Pinned alternate_desktop.cc also returns result11 when the created station has
no readable/nonempty object name, before CreateAltDesktop is called; window.cc
can return11 if SetProcessWindowStation fails. Thus result11 alone identifies
a broader desktop-initialization path, not the precise failed Windows API.

The helper now queries UOI_NAME on each successfully returned station reference
into a fixed2048-byte UTF-16 buffer, recording only error/required length and
whether the name is nonempty. No raw name is logged and the buffer is never
used to launch another process or obtain authority. This readonly check does
not create a desktop or switch a process station. Exact CI measurement of this
new observation remains required; the local station-creation denial means this
success-only branch is not reached on this PC.


The name observation also compares its fixed-buffer bytes with a readonly query
of the borrowed current station. The optional equality is diagnostic only: no
name is logged, used for authorization or passed to a launch/switch API. This
helps distinguish an existing-object reference from an assumed fresh station;
equality/inequality is not itself a proof of how Electron obtained its handle.
Both name buffers are fixed2048bytes. A failed current-name query yields unknown.


### Owned-helper station-switch measurement

At1a38fb9 CI39194121929/job117873063789, all four station/access cases create and
close successfully; their queried names are nonempty (40bytes) and differ from
the queried current station bytes. Electron still stops with result11. This
helper evidence does not prove Electron has identical handles or security state.

The next revision measures SetProcessWindowStation in the disposable fixed
helper only, then restores the borrowed original before closing each returned
reference. It never switches the shared core process or another process, creates
no desktop/window, runs no user/repository code and grants no GUI authority. A
restore failure logs its native error and exits the owned helper126 without
trying to close its current station; Windows releases its references on exit and
the parent requires an empty owned job. Original source/DACL data remains read
only. Local station creation denial prevents reaching this new branch; disposable
Windows CI remains required for its result. This experiment supersedes the
no-switch description of earlier probe revisions, not production policy.


### Descriptor-copy own-package control

At89a8f93 CI39194385861/job117873864600, all four station/name queries and each
helper station-switch/restore succeeded. Normal Electron still reported11. This
narrows the helper's remaining desktop path but is not a trace of Electron's
actual API failures; application state differs.

A third descriptor variant merges one noninherited GENERIC_READ|WINSTA_CREATEDESKTOP
allow entry for the fixed helper's own queried AppContainer SID into a NEW ACL
copy. Original station/descriptors are never installed or changed. The token is
query-only and uniquely closed; SID data stays in a fixed aligned8KiB buffer
through the native ACL copy and is never logged. The returned ACL allocation is
native-size-bounded and freed after calls. No world/package-class grant, runtime
filesystem grant, network authority or model-selected SID is accepted. Original
format controls run before this copy variant; the parent requires six observations.

The local exact owned run still records0x80070005 for all six descriptor/access
pairs. Adding an allow entry to the requested new object's descriptor alone
therefore did not repair station creation in this helper on this PC. This is not
a proposal to edit WinSta0's ACL or disable Chromium's sandbox. Disposable Windows
results for the new copy control remain separate evidence to collect.
