# ADR-0013 — One locally selected session per remote alpha connection

Status: ACCEPTED PRODUCT SCOPE / IMPLEMENTATION GATED
Date: 2026-10-13

## Retained alpha restart amendment — 2026-10-15

Expiration revokes authorization, not the generated project. An explicit local
operator ResumeRun name can reopen a retained alpha directly under package output
with fresh bounded authorization (default50minutes, maximum50). No MCP tool can
select a workspace or renew permissions. Node sources and mutation journal remain;
jobs and authentication state are new. Windows byte-range locking excludes two
runner processes for the same run, releasing on crash without stale lock removal.
Reparse run/project/src/journal objects are refused. Invalidate the prior viewer
snapshot while editing and regenerate after confirmed stop; close the prior viewer
before reopening. This is local generated-run selection, not private project
enrollment, per-chat routing, or automatic session renewal.

## Local stop availability amendment — 2026-10-15

The existing16-command console budget applies to diagnostic/control actions,
not to the ability to stop. After exhaustion, further ordinary commands are
refused but `stop` stays available. No new epochs or mutable control histories
are created beyond that budget. EOF or malformed/oversized console input closes
the service through the same stop path; losing the operator input cannot leave
an apparently controllable alpha running until its timer. A closed event loop
is already stopped and must not cause a background console traceback. Proof
output retains its fixed queue/record limits. This is a shutdown correction,
not automatic authorization renewal or the planned persistent connection mode.

## Core operator supervision foundation — 2026-10-15

An explicit local CLI `--supervise-session` enables a trusted core timer while
its stdio service is alive. It renews only the same active session and existing
leases through SessionLifecycleManager; no MCP renewal/provisioning tool is
added. Each lease window remains bounded by the configured renewal horizon,
policy epoch, capability set and scopes. Default launches still expire once.
The initial window must be5..1800seconds; ordinary launches retain their
existing5..3000second option. The timer samples monotonic time inside its worker,
never replays missed ticks and permanently disables renewal on any failed
renewal. Expired or revoked sessions cannot be recreated. Long OS/process stalls
may therefore require an explicit local restart; automatic resurrection is not
part of availability.

Transport closure ends the supervisor and explicitly revokes the session,
including any already-started renewal critical section. Stop/cancel closes
admission before cleanup; renewal cannot undo that decision. This supervisor is
the running trusted Rust service itself, not model activity or an HTTP heartbeat.
It does not certify the responsiveness of a separate parent UI. Parent death
must close its owned stdio pipes; native runner/parent-death integration remains
a separate acceptance gate. The HTTP alpha is not yet wired to this foundation:
its50-minute timer,512-request ceiling and task-history budgets remain until the
supervised adapter and resource-retirement gates are implemented and tested.
No production credentials or private project enrollment are introduced.

## Context

The user explicitly accepts a single-session alpha while the FranceStudent
administrator's answers about outgoing conversation identity and in-flight MCP
cancellation remain unavailable. The public SDK describes account/chat APIs,
not the backend MCP dispatcher. No trustworthy per-conversation binding has
been established. Waiting for it need not block a truthfully scoped alpha.

## Decision

The first remote alpha targets one locally selected Optic application session
and project per configured connection. All host conversations using that
connection share its session and locally granted permissions. A transport
reconnect resolves the existing application-owned binding; it does not create
a session, renew a deadline or expand permissions.

Local enrollment selects the project and provisions authority. Authentication
identifies the configured connection only. A bearer, tunnel URL, host account
or caller-supplied chat id cannot select a different project or mint a grant.
No caller-controlled workspace/session selector is added to tool arguments.

User-facing permissions must say “this Optic connection/session”, with an
explicit shared-conversation notice. “Only this chat” is prohibited without a
later proven one-to-one binding (ADR-0011). Switching projects requires local
replacement and revocation of the old binding, never implicit reuse of rights.
Concurrent calls may still occur inside this one session; single-session scope
does not remove admission serialization, resource bounds or cancellation checks.

Credential rotation/revocation, session expiry and local stop remain fail-closed.
The binding's owner/epoch must be revalidated per request before dispatch and
before sensitive effects. No HTTP connection or provider stream lifetime is a
substitute for those checks. Host cancel propagation is unproved; the local
stop/revoke path remains required.

## Delivery gates

1. Reuse the existing real Rust, generated-workspace, read-only MCP link proof
   (#178) after review; compose bounded authentication and the fixed binding.
   Keep the exact allowlist and generated files. This is integration evidence.
2. Close the real-project read/security issues before accepting user projects.
   Single-session scope does not close #176 or #180–183, credential storage,
   local enrollment/approval or production remote-transport gates.
3. Add mutation, Git and process tools only through their existing typed policy,
   lease, approval, isolation and resource gates, with separate validation.
4. Defer per-conversation remote session routing to later V1 work, contingent on
   evidence or an explicit application-owned session-reference design.

Existing core multi-session isolation and its tests remain intact. No runtime
change, production release, blanket authority or automatic merge is authorized
by this decision alone. It supersedes the requirement to prove native host
conversation correlation before beginning the single-session alpha, not the
security rules in ADR-0006/0009/0011/0012.

## Acceptance evidence required

The integration must demonstrate fixed project/session identity across fresh
transports, denial after local revoke/expiry, old credential denial after
rotation, no caller-selected rebinding, bounded concurrent pressure and absence
of credential/session values from diagnostics. A shared connection must never
be advertised as isolating individual host chats.
