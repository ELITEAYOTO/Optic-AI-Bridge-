from __future__ import annotations

import asyncio
import os
import time
from typing import Any

from mcp.server import MCPServer
from mcp.server.mcpserver import Context
from mcp.server.transport_security import TransportSecuritySettings

from probe_core import (
    CorrelationStore,
    PROBE_VERSION,
    build_observation,
    sanitize_meta,
    validate_run_id,
)

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
    headers = ctx.headers
    transport_session_id = headers.get("mcp-session-id") if headers else None

    observation = build_observation(
        request_id=ctx.request_id,
        session_id=transport_session_id,
        headers=headers,
        meta=meta,
    )
    observation["protocol_version"] = ctx.protocol_version

    capabilities = ctx.client_capabilities
    if capabilities is None:
        capability_map = None
    elif hasattr(capabilities, "model_dump"):
        capability_map = capabilities.model_dump(mode="json", by_alias=True, exclude_none=True)
    else:
        capability_map = dict(capabilities)
    observation["client_capabilities"] = sanitize_meta(capability_map)
    return observation


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
async def probe_parallel(ctx: Context, delay_ms: int = 250) -> dict[str, Any]:
    """Sleep briefly so two harmless calls can characterize host concurrency."""
    delay_ms = max(0, min(int(delay_ms), 1500))
    start = time.monotonic_ns()
    await asyncio.sleep(delay_ms / 1000.0)
    end = time.monotonic_ns()
    return {
        "probe_version": PROBE_VERSION,
        "delay_ms": delay_ms,
        "elapsed_ms": round((end - start) / 1_000_000, 3),
        "observation": _observation(ctx),
    }


if __name__ == "__main__":
    if ALLOW_RANDOM_TUNNEL_HOST:
        # H-01 only: a Cloudflare Quick Tunnel uses a random public hostname that cannot
        # be predeclared. This experiment has zero project/runtime authority. Production
        # transport must never inherit this relaxation; H-02 owns authenticated remote
        # exposure and a provider-aware host policy.
        security = TransportSecuritySettings(enable_dns_rebinding_protection=False)
        security_mode = "random tunnel host allowed (H-01 only)"
    else:
        security = TransportSecuritySettings(
            enable_dns_rebinding_protection=True,
            allowed_hosts=["127.0.0.1:*", "localhost:*", "[::1]:*"],
            allowed_origins=[
                "http://127.0.0.1:*",
                "http://localhost:*",
                "http://[::1]:*",
            ],
        )
        security_mode = "DNS-rebinding protection enabled; loopback hosts only"

    print("=== Optic H-01 Host Probe ===")
    print(f"Local endpoint : http://{HOST}:{PORT}{PATH}")
    print("Transport      : MCP Streamable HTTP")
    print("Mode           : stateless + JSON responses")
    print("Authority      : NONE (no files, Git, processes, Optic runtime, or mutation)")
    print(f"Transport guard: {security_mode}")
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
