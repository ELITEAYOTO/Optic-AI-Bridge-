# #317 — Windows 10 confirmation comparison, 24 October 2026

Research only. Base main cf5a6c3f5aff7d9846106effafd52027126e3ba3.
No production behavior change, no V1 GO. The operator explicitly authorized this
bounded physical-PC campaign; repository permanent GitHub-only guidance is not
changed. Optic Desktop and REL were never accessed or modified.

## Question and method

Independent main CI40059745546 returned TerminationUncertain rather than TimedOut
in timeout_terminates_owned_process. Its retained log does not identify whether
child.wait returned an error or the existing two-second confirmation elapsed.
Port ONLY PR340's cfg(all(test,windows)) phase diagnostics from4f1b5a2a. No
production timeout, assertion, kill, wait, output limit or status mapping changes.

A pre-armed, independent outer Job owns a gated Python worker before any fixture
Rust process starts. The worker executes exactly three existing timeout tests in
sequence, then timeout, explicit-stop and output-overflow concurrently (three
fixed hosts maximum). Each test uses its existing fixed current-libtest child.
No PID lookup, arbitrary kill, private repository or network test is involved.
The outer Job must be observed empty BEFORE its emergency termination/close.
The existing doctor_pipe_supervisor Job primitive is reused, not reimplemented.

First physical run: Windows10 Famille x64, build19045, Rust1.99.0. All six targeted
assertions passed. Timeout confirmation receipts were positive, kill_request Ok,
confirmation0/1/0/1ms. Total individual timeout test-host wall times were
515/188/187/219ms (startup/harness included, not solely native confirmation).
Explicit stop156ms, output overflow109ms. The first run source is labeled
cf5a6c3+340-diagnostics-worktree; it is not passed off as a committed-SHA CI.
Strict workspace Clippy and Rustfmt passed. Exact committed-source local/hosted
results are separate evidence, retained in the delivery report.

## Interpretation

The intermittent main red was NOT reproduced. Six selected tests under one
moderate schedule cannot rule out scheduler delays, observation errors or kernel
progress issues. Do not retry until green, increase confirmation budget, or map
uncertainty to TimedOut. Current diagnostics distinguish wait error from elapsed
budget but cannot tell which phase of LimitedJobChild::wait was pending on expiry
(root wait vs Job accounting). This remains the next diagnostic boundary if a
natural failure appears; no cause is claimed here.

PR339 already proves reader joins outlive HardenedCommandRunner's root-only
polling deadline; PR341 independently proves stdin writer joins can do the same.
No identical repeat was needed. These use a DIFFERENT runner path from the
ProcessManager timeout test, so the timing defects are not a demonstrated cause
of this TerminationUncertain. Both threads' joins remain unbounded in production.
BrokenPipe is returned after the other joins; it does not cancel a stalled reader.

## Architecture / next safe slice

One long-lived owner must retain the native Job, root identity, all I/O tasks,
counted session admission and quota until positive native proof. At creation-time
Job membership and an independently observable stop must precede execution.
Readers AND stdin writer need cancellable, bounded I/O with acknowledged task
completion; output truncation or an abort is not clean EOF. Closing authority,
requesting stop and proving quiescence are separate receipts. A keeper with
uncertain native proof remains counted and identified; no Drop/detach/timeout
shortcut can manufacture a safe shutdown. Generation-bound ownership is required.

Current ProcessManager monitor retains TerminationUncertain in accounting, but
a future production guardian must also demonstrably retain native supervision,
pins and recoverability across its host lifecycle. SessionLifecycleManager still
calls synchronous registry revoke/drain before cancel_session: transferring a
long-lived admission without independent stop can therefore deadlock.

No read_service module exists on this main. Earlier service/guardian research is
unmerged and must not be transplanted. PR327 remains separate. Next production
work needs a reviewed authority/owner contract and inverted stdout/stderr/stdin
regressions under a native supervisor. These measurements alone justify no
functional production correction. #317/#323 remain open, Windows V1 NO-GO.
