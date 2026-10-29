# ADR-0038 — owned Windows stdout and confirmed writer join

Status: implementation candidate in #429; exact-head and independent-main
validation required. Windows V1 remains NO-GO.

## Problem and supported handles

#428 physically observed a root retained by unread stdout after original cleanup
and the #427 input-reader join. Tokio stdout uses blocking-pool IO whose future
can disappear while the synchronous write still retains process lifetime.

Windows main now claims stdout once before issuing session authority, alongside
the original stdin owner. It duplicates the process's actual standard handle;
no MCP argument selects a handle, destination or cancellation thread. Standard
synchronous pipes and redirected disk files use the same owned implementation.
Disk output preserves the existing physical supervisor's file capture. Console,
NUL and other character output are rejected before grants; stdin remains pipe
only. Unix keeps its existing Tokio path. Overlapped/unusual driver handles are
not certified by a FILE_TYPE query or these ordinary synchronous-handle tests.

## Buffering and ownership

`OwnedStdoutWriter` is not cloneable. It copies at most 8 KiB into one bounded
channel slot, with only one outstanding completion. Buffered `poll_write` success
means bytes were accepted, not delivered. `poll_flush` waits for the synchronous
write result; the next frame is not accepted before that result. At most two
8 KiB allocations overlap at a completion boundary. Existing MCP frame limits
still apply. Bytes are neither decoded nor converted and order is preserved.

One dedicated thread owns the duplicated handle and copied bytes through every
write, error and cancellation. A manual partial-write loop rechecks the stop
latch before each write, including Interrupted retries. No caller buffer or
pooled thread is retained or cancelled. IO errors reach both the async completion
and the explicit owner join. A cancelled/abandoned frame is not a successful
flush or a guarantee that the peer consumed an entire JSON response.

## Stop and completion

Main keeps both guards through original session/lease stop and counted admission
drainage. It then attempts both joins, even if input cleanup failed. A fixed
stderr `stdout-writer-joined` marker is emitted only after a successful actual
join. Timing remains opt-in and never enters MCP stdout.

Owner stop latches irreversibly, rejects later writes and repeatedly calls
[CancelSynchronousIo](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelsynchronousio)
on its uniquely retained dedicated thread handle until that thread finishes,
then joins it. A cancellation request does not establish completion;
ERROR_NOT_FOUND can precede a write entering the kernel. Retry closes that race.
Unknown cancellation errors are retained and reported after join, not used to
detach ownership. Drop follows the same stop/join path. No authority mutex is
held while calling Win32, sleeping or joining.

## Evidence and limits

Native tests use actual anonymous pipes for raw bytes across frames, flush and
EOF, idle/early stop, explicit cancel and Drop with an unread reader kept open,
and broken-reader error propagation. Disk redirection and character-handle
rejection have separate tests. Offline parser tests prevent partial stderr
records from falsely satisfying a physical gate or crashing its observer.

Schema 4 keeps the existing external five-second exit bound. The stalled-output
case must observe original cleanup, input join, output join and exit 1 while
parent stdin and the unread stdout reader stay open. Queued bytes remain; closing
or draining stdout is not a passing intervention. Fixture failure kills remain
failure recovery only. Normal Windows/headless and Unix cases remain required.

This does not bound active admission drainage, stderr, unknown/stalled drivers,
abrupt owner loss or other legacy IO. The fixture uses metadata, not an admitted
file/process/Git worker. It provides neither a native HostStartup/policy launch
fence nor Git/root/Job/descendant/physical stream acceptance, Windows 10/11 device
certification, or permission for a release. No deadline, quota, containment or
authority relaxation is introduced.
