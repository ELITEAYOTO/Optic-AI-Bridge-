# Process Runtime

## Fundamental API

Processes are launched as structured executable + args[] + cwd + controlled environment. Arbitrary shell strings are not the fundamental primitive.

process_start returns a JobId immediately. Output is consumed with cursors through process_read. process_stop accepts only a JobId owned by the caller session.

## Ownership

Every job belongs to exactly one SessionId. No V1 tool can terminate an arbitrary PID.

## Windows lifecycle

Reuse/adapt the proven OpticCode pattern:
1. create Job Object;
2. configure kill-on-close and approved limits;
3. spawn root process;
4. assign it immediately;
5. drain stdout/stderr asynchronously;
6. on timeout/cancel/bridge shutdown terminate owned tree;
7. close handles deterministically;
8. finalize journal/result.

Job Objects provide lifecycle/resource containment; they are not a complete security sandbox.

## Limits

Timeouts, process count and output budgets have hard safety ceilings. Configuration may lower or choose within safe ranges but must not turn critical bounds into infinity accidentally.

## Cancellation

Cancellation propagates session → task/job → process tree. Cancellation and timeout paths receive the same orphan-process tests as normal completion.
