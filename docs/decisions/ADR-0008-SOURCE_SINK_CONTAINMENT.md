# ADR-0008: Source/Sink Containment

Status: Accepted direction

Prompt injection cannot be made impossible by parsing or filtering repository text. Treat external/repository/process content as untrusted sources and constrain sensitive sinks deterministically.

V1 sensitive sinks include network egress, credentials/secrets, writes outside granted roots, security-policy changes, privilege changes and irreversible destructive operations.

This is defense in depth: model robustness is useful but never the authorization boundary.
