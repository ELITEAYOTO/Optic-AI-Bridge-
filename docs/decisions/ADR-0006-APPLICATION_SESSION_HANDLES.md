# ADR-0006: Application-owned Session Handles

Status: Accepted direction

MCP 2026-07-28 removes protocol-level sessions. Optic AI Bridge therefore owns its developer-session model explicitly.

A server-minted opaque SessionHandle identifies application state but is never sufficient authorization by itself. Each call binds it to authenticated caller/project grant, expiry and policy epoch.

This also prevents architecture coupling to legacy `Mcp-Session-Id` semantics.
