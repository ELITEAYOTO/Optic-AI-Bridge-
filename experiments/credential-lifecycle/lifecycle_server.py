import asyncio
import logging
import os
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "francestudent-probe"))
import json
import threading
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
import uvicorn
from probe_core import observe_headers
from server import Guard, request_proof, result, MAX_BODY, MAX_HEADERS, MAX_LIFETIME
from lifecycle_core import Registry

registry = Registry()
mcp = MCPServer("Optic-Dummy-Credential-Lifecycle", instructions=(
    "Only a zero-authority credential_probe. No files, processes, Git, sessions or control tools."
    " Revoke and rotate are local console operations, never model permissions."))


def observation(headers):
    _, safe = observe_headers(headers)
    values = [v for k, v in headers if k.lower() == b"authorization"]
    identity = registry.authenticate(values[0]) if len(values) == 1 else None
    safe["matches_dummy"] = identity is not None
    return identity, safe


@mcp.tool()
def credential_probe() -> dict:
    """Return independent diagnostic principal/epoch only. Confers zero authority."""
    proof = request_proof.get()
    identity = proof["owner"] if proof else None
    if not registry.current(identity):
        return result("credential_probe", {"ok": False, "code": "probe.authentication_inactive"})
    return result("credential_probe", {"ok": True, "authority": "none",
        "principal_id": identity.principal, "auth_epoch": identity.epoch})


class Recheck:
    """Revalidate epoch after bounded body ingestion, before SDK dispatch."""
    def __init__(self, app, state):
        self.app, self.state = app, state

    async def __call__(self, scope, receive, send):
        if scope["type"] == "http":
            proof = request_proof.get()
            if proof is None or not self.state.current(proof["owner"]):
                return await Guard.reject(send, 401)
        return await self.app(scope, receive, send)


def app(allow=False):
    security = TransportSecuritySettings(enable_dns_rebinding_protection=False) if allow else TransportSecuritySettings(
        allowed_hosts=["127.0.0.1:*", "localhost:*"],
        allowed_origins=["http://127.0.0.1:*", "http://localhost:*"])
    upstream = mcp.streamable_http_app(stateless_http=True, json_response=True,
        max_request_body_size=MAX_BODY, transport_security=security)
    return Guard(Recheck(upstream, registry), header_observer=observation,
        zero_arg_tools=("credential_probe",), handle_tool=None)


def local_commands():
    # Fixed command grammar, bounded read. EOF leaves the registry unchanged.
    # No local HTTP control endpoint and no remote MCP control tool.
    for _ in range(16):
        line = sys.stdin.readline(65)
        if not line:
            return
        if not line.endswith("\n"):
            print('[LOCAL] {"ok":false,"code":"probe.control_input_closed"}', flush=True)
            return
        command = line.strip()
        commands = {"rotate-a": ("rotate", "A"), "revoke-a": ("revoke", "A"),
                    "rotate-b": ("rotate", "B"), "revoke-b": ("revoke", "B")}
        outcome = registry.control(*commands[command]) if command in commands else {
            "ok": False, "code": "probe.invalid_local_command"}
        print("[LOCAL] " + json.dumps(outcome, sort_keys=True), flush=True)
    print('[LOCAL] {"ok":false,"code":"probe.local_command_budget"}', flush=True)


async def main():
    logging.disable(logging.CRITICAL)
    port = int(os.environ.get("OPTIC_LIFECYCLE_PORT", "8002"))
    allow = os.environ.get("OPTIC_LIFECYCLE_TUNNEL", "0") == "1"
    print(f"Authority NONE | dummy-lifecycle-v1 | http://127.0.0.1:{port}/mcp | lifetime 25 min", flush=True)
    print("LOCAL commands: rotate-a, revoke-a, rotate-b, revoke-b. One rotation per owner. Ctrl+C stops.", flush=True)
    threading.Thread(target=local_commands, daemon=True).start()
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
