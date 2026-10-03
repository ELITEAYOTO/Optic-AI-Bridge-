# ADR-0004: Bounded Output Everywhere

Status: Accepted

All process output, MCP responses, queues, completed-job retention, logs and spool storage have explicit bounds, TTLs or quotas.

Large output is paged/spooled. Resource exhaustion fails predictably rather than retaining unlimited data.
