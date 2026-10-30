# ADR-0039 — Original host-stop provenance

Status: accepted for this bounded cancellation binding; native launch remains disabled.
Date: 2026-10-30.

## Context

The inert native Git status candidate retains original session/TaskLease/Job
stop flags and an immutable service-clock deadline. It lacks an original host
source. Repeated snapshots or an arbitrary replacement flag cannot establish
host provenance. A finished watchdog can fail, and even successful Job termination
does not prohibit later creation (#439–#442).

## Decision

Trusted process startup owns one noncloneable HostStopSource. Its only mutation
publishes irreversible stop; Drop also publishes. Cloneable HostStopWatch values
are read-only and compare the exact retained Arc identity, never a supplied ID.
No MCP argument can supply or reset the source. False means no observed stop,
not authorization to create/resume/publish or release resources.

The app's actual service/OS-shutdown selection publishes stop inside either
selected future before it returns and select cancels its peer, including source
errors and normal/error service completion. The process owner remains in scope
through original session drainage and IO joins. Process abort cannot execute Rust
Drop and is not covered by this source.

Service-clock native preparation requires a live trusted host watch, retains its
original identity and checks it again after preparation. Subsequent original
snapshots/arm reject missing or foreign hosts; original stop remains visible
after a new source is constructed. Only this constructor establishes provenance:
raw logical-time preparation stays inert and cannot be retroactively armed.
The independent native watchdog receives the retained original host signal as
its fourth existing bounded stop source, after session, lease and Job. No lock,
IO, deadline extension or caller-provided flag is introduced there.

## Consequences and limits

Real native owner tests remain no-child; the app hosts no native Git launcher.
This binds cancellation evidence and publishes actual host closure; it is not
an atomic issuer shared with registry mutators/policy/Win32 create/resume. A stop
can race final snapshots, worker startup or future syscalls. Original custody
must remain owned until actual terminal evidence; no successful outcome is
inferred from a stop or a completed worker. Uncertain-stop recovery, in-flight
native ownership, descendant EOF, installed-platform acceptance and earlier
startup failures remain separate. Windows V1 stays NO-GO.
