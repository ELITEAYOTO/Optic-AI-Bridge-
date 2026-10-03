# Observability and Benchmarks

Observability is bounded and local-first. V1 does not require a Prometheus server.

## Measure separately
Bridge private/RSS; child process tree memory/CPU; spool disk; active/completed sessions/jobs; queue sizes; output produced/retained/sent; policy decisions; cleanup/recovery failures.

## Benchmark suite
Idle 10 min; 1000 file reads; 500 Git cycles; 100 x 1 MiB output; 20 x 100 MiB output; 100 completed jobs; repeated two-session lifecycle; same-repo two-worktree workload; 1h/4h mixed soak; forced crash/restart; timeout/cancel descendant cleanup.

## Targets
Initial idle bridge target below roughly 50–75 MiB is a TARGET, not a promise. The stronger invariant is that memory returns to a stable band after TTL/cleanup and scales only with explicit bounded budgets.

Do not use “<5 GB of buffers” as a lightweight target.
