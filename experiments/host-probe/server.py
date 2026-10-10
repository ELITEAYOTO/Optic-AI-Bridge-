from __future__ import annotations

import asyncio
import os
import time
from typing import Any

from mcp.server import MCPServer
from mcp.server.mcpserver import Context
from mcp.server.transport_security import TransportSecuritySettings

from probe_core import CorrelationStore, PROBE_VERSION, build_observation, validate_run_id

HOST = os.environ.get("OPTIC_HOST_PROBE_HOST", "127.0.0.1")
PORT = int(os.environ.get("OPTIC_HOST_PROBE_PORT", "8000"))
PATH = os.environ.get("OPTIC_HOST_PROBE_PATH", "/mcp")
ALLOW_RANDOM_TUNNEL_HOST = os.environ.get("OPTIC_HOST_PROBE_ALLOW_RANDOM_TUNNEL_HOST", "0") == "1"

store = CorrelationStore()
mcp = MCPServer(
    "Optic-H01-Host-Probe",
    instructions=(
        "Harmless Optic H-01 compatibility probe. It cannot read files, use Git, "
        "start processes, access Optic runtime state, or modify the machine."
    ),
)


def _observation(ctx: Context) -> dict[str, Any]:
    request_context = ctx.request_context
    meta = request_context.meta if request_context is not None else None
    return build_observation(
        request_id=ctx.request_id,
        session_id=ctx.session_id,
        headers=ctx.headers,
        meta=meta,
    )


@mcp.tool()
def probe_ping(ctx: Context) -> dict[str, Any]:
    """Return a harmless liveness result plus protocol context fingerprints."""
    return {"ok": True, "probe_version": PROBE_VERSION, "observation": _observation(ctx)}


@mcp.tool()
def probe_host_context(ctx: Context) -> dict[str, Any]:
    """Report redacted request metadata for host/session compatibility research."""
    return _observation(ctx)


@mcp.tool()
def probe_session_correlation(run_id: str, ctx: Context) -> dict[str, Any]:
    """Compare this call with the first call for a caller-chosen test run id."""
    validate_run_id(run_id)
    observation = _observation(ctx)
    result = store.record(run_id, observation)
    return {
        "probe_version": PROBE_VERSION,
        "run_id": run_id,
        "call_index": result.call_index,
        "same_transport_session_as_first": result.same_transport_session_as_first,
        "matching_header_fingerprints": result.matching_header_fingerprints,
        "matching_meta_fingerprints": result.matching_meta_fingerprints,
        "observation": observation,
    }


@mcp.tool()
def probe_reset(run_id: str) -> dict[str, Any]:
    """Forget correlation state for one caller-chosen test run id."""
    return {"run_id": validate_run_id(run_id), "removed": store.reset(run_id)}


@mcp.tool()
async def probe_parallel(delay_ms: int = 250, ctx: Context | None = None) -> dict[str, Any]:
    """Sleep briefly so two harmless calls can characterize host concurrency."""
    delay_ms = max(0, min(int(delay_ms), 1500))
    start = time.monotonic_ns()
    await asyncio.sleep(delay_ms / 1000.0)
    end = time.monotonic_ns()
    observation = _observation(ctx) if ctx is not None else None
    return {
        "probe_version": PROBE_VERSION,
        "delay_ms": delay_ms,
        "elapsed_ms": round((end - start) / 1_000_000, 3),
        "observation": observation,
    }


if __name__ == "__main__":
    security = TransportSecuritySettings(
        # Only enable this for a temporary random-host tunnel such as trycloudflare.com.
        # The probe carries no project/runtime authority, but keeping the opt-in explicit
        # prevents this experimental setting from silently becoming a production default.
        enable_dns_rebinding_protection=not ALLOW_RANDOM_TUNNEL_HOST
    )

    print("=== Optic H-01 Host Probe ===")
    print(f"Local endpoint : http://{HOST}:{PORT}{PATH}")
    print("Transport      : MCP Streamable HTTP")
    print("Mode           : stateless + JSON responses")
    print("Authority      : NONE (no files, Git, processes, Optic runtime, or mutation)")
    print(f"Probe version  : {PROBE_VERSION}")
    print()

    mcp.run(
        transport="streamable-http",
        host=HOST,
        port=PORT,
        streamable_http_path=PATH,
        stateless_http=True,
        json_response=True,
        transport_security=security,
    )
