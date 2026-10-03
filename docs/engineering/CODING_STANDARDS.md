# Coding Standards

- Safe Rust by default. Any unsafe block requires local justification, narrow scope and tests.
- No panics on untrusted MCP/repository/process data paths.
- Typed domain objects over stringly-typed security decisions.
- Explicit ownership and cancellation.
- Bounded channels/queues unless a documented proof shows otherwise.
- No global mutable current workspace/session.
- Structured executable + args, never command concatenation.
- Errors preserve machine-readable category.
- Security-sensitive normalization is centralized, not reimplemented per tool.
- New dependencies require justification.
- New feature that changes a security invariant requires ADR + tests.
