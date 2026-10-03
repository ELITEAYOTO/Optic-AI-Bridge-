# Dependencies and OpticCode Reuse

## Minimal expected stack

Rust stable, Tokio, Serde, maintained Rust MCP SDK, windows-sys/windows, tracing, TOML/config support, focused error crate if useful.

Dependencies are selected by necessity, maintenance/security posture and measurable cost—not popularity alone.

## OpticCode candidates

Strong reuse candidates:
- process request/result/status concepts;
- Windows Job Object runner behavior;
- deterministic policy model and approvals;
- path confinement patterns;
- transaction manifest/rollback/recovery;
- Git state primitives.

Do not make Optic AI Bridge depend on OpticCode Java/RAG/tree-sitter/editor-specific layers.

## Extraction rule

Prefer small generic shared crates only when ownership/API boundaries are stable. If extraction would destabilize OpticCode, port/refactor the minimal proven logic first and record provenance/license.

OpticCode is MIT in the inspected repository; preserve attribution/license obligations as applicable.
