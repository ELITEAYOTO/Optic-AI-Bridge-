# Configuration Specification

## Design

Configuration is layered: secure compiled ceilings → machine config → project grant → session lease. Lower layers cannot override hard security invariants.

## Example categories

[transport]
mode = "stdio"

[session]
max_sessions = 4
idle_ttl_sec = 1800

[output]
response_default_bytes = 65536
response_hard_max_bytes = 262144
global_ram_budget_bytes = 16777216
spool_quota_bytes = 536870912
spool_ttl_sec = 3600

[process]
max_concurrent_per_session = 4
default_timeout_sec = 600
hard_timeout_sec = 3600

## Rules

Paths are not silently widened. Zero does not mean unlimited unless the field explicitly documents it and security review accepts it. Secrets/tunnel credentials do not belong in normal committed config.
