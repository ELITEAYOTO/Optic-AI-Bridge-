# H-02 fixed public work under local ownership

Date: 2026-10-14. Public fixture only, no executable-effect authorization.

The previous coordinator returned snapshots without protecting later work. This
extension adds one fixed, harmless in-memory SHA-256 computation of the compiled
public operation marker. No callback, path, executable, command, credential or
caller-selected payload is accepted. No production ActionEnvelope/Effect changes.

The same immediate-refusal lock spans confirmation observation, the complete
calculation, final observation and result construction. Only confirmed state
can enter. There are at most32 attempts per coordinator; the counter is spent
before observation/work and never refunded. Exhaustion refuses further work
without erasing valid confirmation. Results carry authority=none/fixture_only=true.

Any observation/work interruption or failure permanently blocks the coordinator.
Local close uses the same lock: while work is running it refuses busy and does
not acknowledge completion. Once admitted, close blocks future work. Close only
changes local bookkeeping; it does not revoke a credential or cancel a process.
There is no queue, background task, retained result collection or automatic retry.

Six new regressions cover unconfirmed work refusal, the exact32-attempt budget,
held worker ownership and busy close/read/second-work/publication refusal, changed
state before work, external mutation during work and interruption cleanup. A
running owned test worker cannot be cancelled by Future.cancel(); this is not
proof of async MCP/provider cancellation propagation. Tests use bounded events
and owned future waits; the implementation has no general OS I/O deadline.

Local native Windows suite:143 cases,140 passed,3 deliberate platform skips.

## Explicit negative boundary

The external mutation test performs the harmless calculation hook and changes
the public file before final observation. The result is refused, but the test
records that computation already occurred. A post-check therefore cannot roll
back an external effect or enforce authorization against a writer outside the
same object's lock. This remains a one-object public-work proof, not a process
lock, filesystem snapshot, action lease, durable commit or production solution.

Actual effects still require typed policy/lease ownership through their true
execution boundary, with revocation, cancellation and cleanup semantics. Native
root trust, protected metadata, recovery, antirollback and finite generations
remain separate gates. Existing production Store and replacement-gap evidence
are unchanged. No new remote endpoint or user-project access is introduced.
