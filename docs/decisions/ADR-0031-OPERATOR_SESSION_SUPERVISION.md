# ADR-0031: explicit local supervision of finite session windows

Status: ACCEPTED DESIGN / MAIN INTEGRATION CANDIDATE
Date: 2026-10-19

## Context

The operator wants a connection to remain available until local stop, without
model traffic extending permissions or requiring periodic project recreation.
The experimental alpha already exercises the existing lifecycle renewal API.
Promote its CLI foundation separately from HTTP credentials, adapter budgets,
project admission, build tools and GUI compatibility.

## Decision

An explicit local --supervise-session flag runs a trusted timer while the owned
stdio service is alive. --session-lifetime-seconds selects a finite window:
5..3000seconds for ordinary timed launches; supervised windows also must fit the
existing hard renewal horizon (currently1800seconds). Default launch remains
finite with its existing initial TTL. The model has no renewal/provisioning tool.

Every tick uses SessionLifecycleManager.renew on the same active session and
existing leases; no capability, owner, epoch, scope or resource limit is widened.
Monotonic time is sampled inside the blocking worker, missed ticks are skipped,
and the timer permanently stops renewing after any rejected renewal. Expired or
revoked authority is never recreated. Stalls beyond the window require an
explicit local restart, not automatic resurrection.

Transport completion drops the timer future and explicitly revokes the session
through the lifecycle manager, including an already-started renewal critical
section. Closing admission and draining existing effects stays authoritative;
cancellation cannot undo revocation. Per-effect/job/output/state bounds remain.
A trusted local parent must close its owned pipes on death; this CLI option alone
does not certify a separate UI or HTTP adapter's supervision contract.

## Validation and limits

Rust stdio integration exercises default expiry, idle local renewal with unchanged
visible scopes/capabilities, continued reads, terminal session_cancel across a
further tick, and clean EOF while a supervisor is active. There are no Python,
Node or GUI dependencies in this integration test or core feature. Existing
lifecycle expiry/revocation/race tests remain required. Exact Windows/Linux CI
must pass before merge. The source was isolated from the validated alpha CLI;
no experimental HTTP server, generic build or Electron probe is promoted here.
