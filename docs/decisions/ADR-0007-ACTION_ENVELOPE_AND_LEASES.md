# ADR-0007: Action Envelope and Capability Leases

Status: Accepted direction for prototype

All executable effects pass through one typed internal ActionEnvelope. Session/task capability leases define narrow, expiring ceilings for targets, action families, resources and optional network access.

The AI may request actions/leases but cannot mint, extend or approve them.

Reason: models are probabilistic and can retry, hallucinate or follow malicious repository instructions. Deterministic authorization must operate on normalized effects rather than natural-language intent.
