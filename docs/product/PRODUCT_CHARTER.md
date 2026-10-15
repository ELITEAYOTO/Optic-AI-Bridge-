# Product Charter

Status: DECIDED

## Mission

Build a lightweight Windows-first Rust bridge that gives an MCP-capable AI controlled access to local developer workflows while keeping authorization and containment outside the model.

## Core principle

**The AI decides. The bridge executes. Policy authorizes. Isolation contains.**

The AI may decide which files, searches, Git information or processes it needs. It cannot grant itself permissions, widen its workspace, disable safety limits, approve critical operations, or mutate security policy.

## Primary use cases

- inspect/search/read a codebase;
- apply controlled code changes;
- inspect Git state and diffs;
- build and test projects;
- supervise long-running local processes;
- run two or more isolated AI sessions;
- split work on one repository without uncontrolled concurrent writes.

## V1 non-goals

No embedded/local LLM, Electron UI, mandatory Node/Python/Docker runtime, office/PDF automation, generic remote desktop, browser automation, arbitrary PID control, plugin marketplace, database server, or project-owned cloud relay.

Cross-platform support is intentionally secondary until the Windows security/runtime model is proven.

## Product invariants

Security-critical defaults cannot silently become unlimited through configuration. All long-lived collections have a bound or TTL. Every spawned process has an owner session. Every write is scoped and conflict-aware. Transport is replaceable without changing execution/security logic.


## Explicit operator supervision

A local operator may opt into a trusted supervisor which renews the same active
session using finite monotonic lease windows. This is availability until local
stop, not unlimited capabilities: each window remains bounded, revocation and
policy epochs stay effective, and an expired grant cannot be resurrected. The
model and ordinary MCP tools cannot enable renewal or widen its authority.
Transport shutdown must revoke the supervised session and its owned work. This
core foundation alone does not deliver a persistent remote alpha connection;
its transport rate limits and retained-resource lifecycle require separate gates.
