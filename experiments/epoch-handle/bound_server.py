import asyncio
import logging
import os
from pathlib import Path
import sys
import threading

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / "credential-lifecycle"))
sys.path.insert(0, str(ROOT.parent / "francestudent-probe"))
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
import uvicorn
from lifecycle_core import Registry
from lifecycle_server import Recheck, local_commands
from probe_core import observe_headers
from server import Guard, request_proof, result, MAX_BODY, MAX_HEADERS, MAX_LIFETIME
from bound_core import EpochHandles

registry = Registry()
handles = EpochHandles(registry)
mcp = MCPServer("Optic-Dummy-Epoch-Handle", instructions=(
    "Zero-authority opaque handoff bound to the authenticated dummy principal and epoch. "
    "Call bound_create_handle then bound_consume_handle with the exact application_handle. "
    "Local rotate/revoke invalidate old bindings. No files, processes, Git, project or control tools."))


def observation(headers):
    _, safe = observe_headers(headers)
    values = [v for k, v in headers if k.lower() == b"authorization"]
    identity = registry.authenticate(values[0]) if len(values) == 1 else None
    safe["matches_dummy"] = identity is not None
    return identity, safe


def identity():
    proof = request_proof.get()
    return proof["owner"] if proof else None


@mcp.tool()
def bound_create_handle() -> dict:
    """Mint a zero-authority single-use handle for the current principal+epoch."""
    return result("bound_create_handle", handles.create(identity()))


@mcp.tool()
def bound_consume_handle(application_handle: str) -> dict:
    """Pass the exact minted handle. Other owner/epoch/expiry/replay fails closed."""
    return result("bound_consume_handle", handles.consume(identity(), application_handle))


def app(allow=False):
    security = TransportSecuritySettings(enable_dns_rebinding_protection=False) if allow else TransportSecuritySettings(
        allowed_hosts=["127.0.0.1:*", "localhost:*"],
        allowed_origins=["http://127.0.0.1:*", "http://localhost:*"])
    upstream = mcp.streamable_http_app(stateless_http=True, json_response=True,
        max_request_body_size=MAX_BODY, transport_security=security)
    return Guard(Recheck(upstream, registry), header_observer=observation,
        zero_arg_tools=("bound_create_handle",), handle_tool="bound_consume_handle")


async def main():
    logging.disable(logging.CRITICAL)
    port = int(os.environ.get("OPTIC_EPOCH_HANDLE_PORT", "8003"))
    allow = os.environ.get("OPTIC_EPOCH_HANDLE_TUNNEL", "0") == "1"
    print(f"Authority NONE | dummy-epoch-handle-v1 | http://127.0.0.1:{port}/mcp | lifetime 25 min", flush=True)
    print("Local only: rotate-a/revoke-a/rotate-b/revoke-b. Handles TTL120/single use, capacity32.", flush=True)
    threading.Thread(target=local_commands, args=(registry,), daemon=True).start()
    service = uvicorn.Server(uvicorn.Config(app(allow), host="127.0.0.1", port=port,
        access_log=False, log_config=None, limit_concurrency=32, timeout_keep_alive=5,
        h11_max_incomplete_event_size=MAX_HEADERS, timeout_graceful_shutdown=5))
    async def stop():
        await asyncio.sleep(MAX_LIFETIME)
        service.should_exit = True
    timer = asyncio.create_task(stop())
    try:
        await service.serve()
    finally:
        timer.cancel()


if __name__ == "__main__":
    asyncio.run(main())
