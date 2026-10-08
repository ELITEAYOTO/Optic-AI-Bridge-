# Phase 3D adversarial multi-session closure

Status: validation gate only; no new public session surface.

This gate composes already-merged Phase 3A/3B guarantees in one adversarial runtime scenario rather than introducing a new authority primitive.

The scenario provisions two application-owned sessions, A and B, with bounded global/per-session capacities and proves that:

- A cannot resolve B's task lease (`WrongSession`);
- A cannot read or stop B's opaque JobId (`UnknownJob`), and B cannot read A's JobId;
- A hitting its per-session active-process ceiling does not prevent B from remaining active;
- the bounded session registry refuses a third session while A and B occupy capacity;
- revoking A revokes only A's lease and requests termination only for A's owned job;
- B's session, lease and running job survive A's revoke;
- quiescent reap removes only A's process record, task lease and session;
- after A is physically reaped, a new session C can reuse the reclaimed session/lease/process capacity while B remains active;
- B remains observable to B after C is admitted.

The test uses only existing public runtime/lifecycle APIs. It adds no MCP session-minting or renewal route and makes no claim that public multi-session orchestration is ready by itself.
