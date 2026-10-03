# Configuration Specification

## Design

Configuration is layered: secure compiled ceilings → machine config → project grant → session lease. Lower layers cannot override hard security invariants.

Phase 1 introduces executable compiled ceilings in `HardLimits`. User/machine configuration may request smaller values later, but it must never widen these ceilings at runtime.

## Compiled Phase 1 ceilings

Current defaults:

- request body: 1 MiB;
- response body: 256 KiB;
- request lifetime: 30 s;
- concurrent requests: 16;
- active output RAM: 16 MiB;
- single `fs_read`: 256 KiB;
- `fs_list` page: 256 entries;
- deterministic directory scan ceiling: 4096 entries;
- process timeout: 1 h;
- process output: 16 MiB;
- process memory: 8 GiB;
- process count: 32.

Zero is rejected for every hard safety limit; it never means unlimited.

## Example categories

```toml
[transport]
mode = "stdio"
request_default_bytes = 65536
request_hard_max_bytes = 1048576
response_hard_max_bytes = 262144
request_timeout_sec = 30
max_concurrent_requests = 16

[session]
max_sessions = 4
idle_ttl_sec = 1800

[filesystem]
read_default_bytes = 65536
read_hard_max_bytes = 262144
list_page_entries = 128
list_page_hard_max_entries = 256
directory_scan_hard_max_entries = 4096

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
```

The TOML surface above remains a specification until the config loader exists; compiled `HardLimits` are authoritative today.

## Rules

Paths are not silently widened. Zero does not mean unlimited unless the field explicitly documents it and security review accepts it. Secrets/tunnel credentials do not belong in normal committed config. SDK/library defaults never override Optic-owned hard ceilings.
