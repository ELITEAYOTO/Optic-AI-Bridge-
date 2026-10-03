# ADR-0002: MCP Is an Adapter

Status: Accepted

Core filesystem/Git/process/security logic must not depend on a concrete MCP transport. MCP maps external contracts to internal typed services.

This keeps the project testable and resilient to protocol/connectivity changes.
