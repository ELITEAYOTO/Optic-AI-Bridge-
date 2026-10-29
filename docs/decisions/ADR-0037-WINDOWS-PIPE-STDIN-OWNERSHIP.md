# ADR-0037: Owned Windows pipe stdin cancellation and join

Status: targeted Windows transport contract in #427; exact integration and
acceptance receipts belong to #427/#323. Windows V1 remains **NO-GO**.

## Context

#426 physically observed host signals reach original session cleanup while
Tokio's blocking stdin reader retained the process until its parent closed input.
Changing runtime shutdown policy, abandoning a reader or expiring counted
admissions would not establish safe input ownership.

## Decision

The Windows binary claims stdin once per process, before provisioning session
authority. It duplicates the original handle without inheritance, requires pipe
type and moves that owned handle into a dedicated 256KiB-stack reader thread.
The supported contract is a standard synchronous MCP pipe. Console/disk input is
rejected before any session grant; no fallback to the old blocking-pool reader.
This is an explicit compatibility restriction, not a new permission or launcher.

An AsyncRead adapter receives raw bytes through a single-slot channel. Each frame
is at most 8KiB; producer pending/read frame, queued frame and consumer chunk
bound payload storage to 24KiB, plus fixed channel/thread/standard-library overhead.
There is no unbounded spool, parser, dispatch or authority inside the reader.
Original bounded JSON framing and request admission checks remain unchanged.

Only the owner can set its private stop flag. It targets CancelSynchronousIo at
the exact retained JoinHandle's dedicated thread, never a pooled thread or an ID
lookup. A release/acquire stop check prevents subsequent reads. Repeated cancel
attempts until completion cover the check/enter-ReadFile race, including transient
ERROR_NOT_FOUND. Queue backpressure also observes stop, rather than blocking on
an uncancellable send.

A cancel request is not completion: [Microsoft's API contract](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelsynchronousio)
does not wait for the IO to finish. The File and read buffer remain worker-owned;
the owner retains its handle until actual join, including thread destructors.
Other IO/cancellation errors remain errors, reported after join. If cancellation
cannot complete, ownership is retained without a fake timeout or detached thread.
See also [cancellation considerations](https://learn.microsoft.com/en-us/windows/win32/fileio/canceling-pending-i-o-operations).

Main retains that owner across the entire service and original lifecycle cleanup.
Only after original session/TaskLease stop and admission drainage return does
it request input stop and await cancel/join on a blocking cleanup task. Successful
join emits the existing opt-in diagnostic `stdin-reader-joined` phase, then the
original service result is returned. If lifecycle drainage stalls, input ownership
is retained. Startup error unwinding uses the same guard's cancel/join Drop path;
it does not emit a successful session-cleanup receipt. Explicit join errors take
precedence over an otherwise returned service error.

This guard never drops an unjoined JoinHandle; [Rust documents that ordinary
JoinHandle drop detaches the thread](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html).
Its Drop path may block under uncertain cancellation rather than release an
unproved live worker. Abrupt process abort/kill remains outside this guarantee.

## Verification and limits

Test-first bb0efacb67b98fe4e8e530b6554e7b1503bbf758, CI #40869915928, physically
received Ctrl+C and completed original cleanup, then failed the unchanged 5s
process-exit gate while parent stdin remained open. This is behavioral RED.

The Windows physical gate now requires normal root exit while input is still
open and ordered cleanup/join markers. Native tests separately cover raw bytes
and EOF, early/idle cancel with a live writer, backpressure Drop and non-pipe
rejection. Exact-head regular Windows/Ubuntu and three applicable native
workflows, followed by independent main CI/JSON, remain mandatory.
The regular Rust test step has an external 10-minute CI budget for retained-owner
failures; this changes no product lifetime, drain or 5s physical acceptance gate.

Unix keeps its original Tokio stdin and separate retention characterization.
This is not a universal process-exit bound: failed/stalled IO drivers, stdout,
legacy process/pipe joins, held admissions, startup interruption and uncertain
native resources remain separate. No TerminateThread, process::exit, runtime
shutdown timeout, drain deadline reset, TaskLease/quota release, native GitRead
activation, global host/policy launch fence or release is introduced. Hosted
Windows is not Windows10/11 installation certification.
