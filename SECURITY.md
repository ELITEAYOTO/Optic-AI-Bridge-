# Security Policy

Optic AI Bridge is currently **pre-alpha**. There is no production-supported release yet.

## Security objective

The project does not claim to be “100% secure”. The objective is stronger engineering: define security invariants that are deterministic, executable in tests, fail closed, and remain true even when the AI is mistaken or manipulated.

Core rule:

> The AI decides what it wants to attempt. The bridge decides what is authorized. OS isolation limits impact.

## Trust model

Untrusted for authorization:
- model/tool-call arguments;
- repository files and documentation;
- compiler/test/process output;
- network/external content;
- client-supplied handles and metadata.

Trusted only after local verification:
- canonicalized targets;
- server-minted session/task handles;
- local policy configuration;
- policy decisions;
- cryptographic hashes/preconditions;
- OS-enforced identity/isolation state.

## Non-negotiable controls

Deny by default; least privilege; no AI self-approval; no arbitrary PID control; no privilege elevation; no silent cross-session access; no unbounded queues/logs/output; no stale overwrite; no implicit network permission; no security-policy mutation through normal MCP tools.

See [Security Invariants](docs/security/SECURITY_INVARIANTS.md) and [Threat Model](docs/security/THREAT_MODEL.md).

## Vulnerability reporting

Do not publish exploit details in a normal issue. Prefer GitHub private vulnerability reporting / Security Advisories when enabled for this repository. If a private channel is unavailable, open only a minimal issue asking the maintainer for a private contact path and omit exploit details.

## Release security

A future public binary must pass the security gate, dependency/advisory checks, Windows-native isolation tests and release provenance requirements before being called stable.
