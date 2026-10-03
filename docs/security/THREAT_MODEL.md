# Threat Model

## Assets

Source code, Git history, credentials/secrets, user files outside granted projects, local machine integrity, network identity, build artifacts, session isolation and availability.

## Adversaries/failures

- prompt injection or mistaken AI action;
- malicious repository content/instructions;
- path traversal/reparse-point/symlink tricks;
- TOCTOU file replacement;
- malicious or compromised build dependency/process;
- one session attempting to access another;
- runaway output/process trees;
- stale writes and concurrent edits;
- bridge crash/restart;
- dependency/supply-chain compromise.

## Trust boundaries

ChatGPT/model output is untrusted input for authorization purposes. Repository contents are also untrusted. MCP schema validity is necessary but does not make an action safe.

## Defense in depth

typed schemas → canonical target resolution → session/capability validation → deterministic policy → transactional preconditions → OS process containment → bounded resources → audit/recovery.

## Explicit corrections to research notes

- Job Objects are not a full sandbox.
- JSON Schema does not stop prompt injection.
- Rust prevents many memory-safety bugs but does not guarantee absence of leaks/resource retention.
- A second AI reviewer must not be the security authority.
- RAM targets are hypotheses until measured.
