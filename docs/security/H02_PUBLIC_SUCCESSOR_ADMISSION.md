# H-02 one-process public successor admission

Date: 2026-10-14. Disposable public fixture only; no auth commit selected.

An exact readable pair is insufficient to resume work after an uncertain writer
result. This coordinator makes a narrower, explicit local contract: only this
object's successfully completed publication and subsequent exact observation
can become `confirmed`. Confirmation is volatile bookkeeping, not a durable ACK,
credential state, lease or permission to execute later effects.

## Local transition and admission

The coordinator starts `empty` only after observing an empty valid fixture.
Reserved, validated, malformed or missing existing state starts blocked. A new
coordinator never adopts a previous object's pair, even after known success.

One nonblocking lock spans the entire synchronous publisher and final observation.
Concurrent readers and writers receive a fixed busy error immediately: no queue,
waiting caller or second publication is created. Once publication begins, state
is pessimistically blocked. Only a matching successful publisher result and
observer result change it to confirmed. Every exception or interruption during
publication leaves it blocked, with no retry, repair, deletion or reset.

Invalid arguments fail before publisher I/O and preserve empty admission. A
second publication after confirmation is refused without erasing confirmation.
Each read re-observes the expected empty or exact confirmed pair while holding
the same lock; observation failure/change blocks it permanently. Returned values
are detached snapshots, always fixture_only=true and authority=none.

The candidate explicitly uses the portable closed-stage publisher. It does not
silently switch to native publication. Its file/path trust limitations remain.

## Evidence and limits

Seven added regressions cover detached confirmation, concurrent busy refusal,
running worker cancellation refusal, failure after actual valid publication,
restart refusal for validated/reserved state, invalid input without I/O, sticky
refusal after changed bytes and interruption before any record. The concurrency
test holds an owned worker with bounded test events/future waits; cancellation
cannot release a running worker's publication ownership. This is a synchronous
thread observation, not an async MCP/provider cancellation proof.

Local Windows suite: 137 cases, 134 passed, 3 deliberate platform skips.
Native regressions remain included; the new coordinator itself is portable.

This is one object's coordination, not cross-process serialization. Independent
objects/processes or hostile same-user programs remain outside its lock. Reads
are multi-file path observations, not an atomic filesystem snapshot. OS I/O may
block; no general deadline is claimed. A returned snapshot protects no future
action. Losing the caller reply after local confirmation does not roll back the
files or establish exactly-once effects. Restart is conservatively blocked;
recovery, protected metadata, antirollback, generations and durable/power-loss
commit remain separate gates. Existing production Store and its characterized
replacement gap are unchanged.
