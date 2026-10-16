# ADR0021 — Private state preparation primitives

Status: IMPLEMENTED / NO PUBLIC TASK JOURNAL
Date: 2026-10-16

ProjectInputManifest has a deterministic domain-separated fingerprint of its
sorted paths, sizes and content versions. No source bytes or authorization
handles are included. Identical input expectations may hash equally under
different sessions; live owner/lease checks remain mandatory and independent.

An EphemeralAppContainerPlan generates its fresh128-bit name in memory before
native profile creation. The host may persist the intention before consuming it.
There is no factory that recreates plans from journal/model-supplied names.
Existing create_ephemeral delegates to that plan and remains fresh/no-capability.
Explicit close reports native profile cleanup; Drop is best effort and never
claims confirmation. The caller first drains its owned tree/releases storage pins.
Private preparation now acknowledges close before releasing its stage slot, and
releases its own preparation pins before attempting error cleanup.

open_exclusive_state_file is a host-only native file primitive, not an arbitrary
model write tool. It refuses final reparse points, nonfiles and hard-link aliases,
opens exclusively without truncation, and reports native identity/final path.
The caller must pin the selected parent and compare the final path before writes.
No directory selection, storage authorization, journal format, replay, dependency,
process or GUI authority is added by this increment. Core remains runtime-free.

Native tests cover nontruncating reopen, exclusive ownership, hard-link rejection,
fresh profile planning/cleanup and the existing private preparation lifecycle.
The fingerprint test checks order independence, content differences and continued
owner-binding refusal. Exact Windows/Linux/dependency checks precede integration.
