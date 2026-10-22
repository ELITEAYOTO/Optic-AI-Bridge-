# #185 — Owned process snapshots after a natural post-resume failure

Research only. Baseline production main 3fe3de0; no runtime or sandbox change.
The Windows gate for PR327 (CI39845778580 attempt1, source ef3208d) failed
naturally at10s. Its first diagnostic reached target_resumed14ms then remained
silent until10270ms. Later diagnostics passed at2427ms and415ms. All outcomes
remain evidence; the required gate stays red and PR327 remains unmerged.

A separately pushed diagnostic branch adds an opt-in test ownership gate before
listener/target/deadline setup. An independent outer Job owns the test before
that gate opens. At100ms/1s/3s/8s after opening, Python snapshots at most eight
processes enumerated from that exact Job. Each opened handle is revalidated
with IsProcessInJob before reading counters, preventing PID reuse from granting
observation authority. No arbitrary PID termination or unrelated enumeration.

Only fixed roles, cumulative CPU, I/O counters, handles, working set and page
fault counts are serialized. No PID, image path, command line, user data, token
or stack is recorded. Query failures are explicit. GetProcessIoCounters does
not explain all memory-mapped image/page-fault I/O; no cache-causality inference
is authorized. Snapshots are observations, not resource reservations.

The actual native network test, its10s deadline, startup script, AppContainer,
network assertions and resource budgets are unchanged. At most three new-process
trials on a fresh hosted worker; stop at first failed test. No warmup Node launch
or identity hash before observations; do not call these controlled cold-cache
experiments. The pre-existing failed gate is never replaced by this workflow.
A failed natural probe keeps this diagnostic workflow red too.

Local validation before hosted execution: formatting, Python syntax, read-only
Win32 counter pilot on an explicitly Job-owned synthetic sleeping Python child,
and pinned Rust native test compilation. The pilot contained no AppContainer,
network or private data. Actual failure-phase samples require the hosted run.

Hypotheses distinguished: process absent/exited versus resumed/alive; cumulative
CPU advancing versus low CPU; reported I/O/working set changes. These metrics
cannot establish a wait chain, loader/AV cause, JS execution or network denial.
Further tracing would need its own narrow experiment and native proof. Neither
#185 nor #319 is resolved by passing samples or by the #317 native owner work.
