import argparse
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
from lifecycle_server import Recheck, local_commands
from probe_core import observe_headers
from server import Guard, request_proof, result, MAX_BODY, MAX_HEADERS, MAX_LIFETIME
from persistent_core import Store, PersistentRegistry, FixtureError


def make_app(state, allow=False):
    mcp = MCPServer("Optic-Persistent-Dummy-Auth", instructions=(
        "Only zero-authority diagnostic metadata. Local operator controls, no session or project tools."))
    @mcp.tool()
    def persistent_credential_probe() -> dict:
        """Return diagnostic principal and epoch, never authority or credentials."""
        proof = request_proof.get()
        identity = proof["owner"] if proof else None
        if not state.current(identity):
            return result("persistent_credential_probe", {"ok": False, "code": "probe.authentication_inactive"})
        return result("persistent_credential_probe", {"ok": True, "authority": "none",
            "principal_id": identity.principal, "auth_epoch": identity.epoch})
    def observation(headers):
        _, safe = observe_headers(headers)
        values = [v for k, v in headers if k.lower() == b"authorization"]
        identity = state.authenticate(values[0]) if len(values) == 1 else None
        safe["matches_dummy"] = identity is not None
        return identity, safe
    security = TransportSecuritySettings(enable_dns_rebinding_protection=False) if allow else TransportSecuritySettings(
        allowed_hosts=["127.0.0.1:*", "localhost:*"], allowed_origins=["http://127.0.0.1:*", "http://localhost:*"])
    upstream = mcp.streamable_http_app(stateless_http=True, json_response=True,
        max_request_body_size=MAX_BODY, transport_security=security)
    return Guard(Recheck(upstream, state), header_observer=observation,
        zero_arg_tools=("persistent_credential_probe",), handle_tool=None)


async def run(directory):
    state = PersistentRegistry(directory)
    try:
        port = int(os.environ.get("OPTIC_PERSISTENT_AUTH_PORT", "8004"))
        allow = os.environ.get("OPTIC_PERSISTENT_AUTH_TUNNEL", "0") == "1"
        print(f"Authority NONE | persistent-dummy-auth-v1 | http://127.0.0.1:{port}/mcp | lifetime 25 min", flush=True)
        print("Local only rotate-a/revoke-a/rotate-b/revoke-b. Existing fixture loaded; no implicit reset.", flush=True)
        threading.Thread(target=local_commands, args=(state,), daemon=True).start()
        service = uvicorn.Server(uvicorn.Config(make_app(state, allow), host="127.0.0.1", port=port,
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
    finally:
        state.close()


def main():
    logging.disable(logging.CRITICAL)
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture-dir", type=Path, default=ROOT / ".fixture-state")
    parser.add_argument("--initialize", action="store_true")
    args = parser.parse_args()
    try:
        if args.initialize:
            Store.initialize(args.fixture_dir)
            print("FIXTURE INITIALIZED - public dummy metadata only", flush=True)
        else:
            asyncio.run(run(args.fixture_dir))
    except FixtureError as error:
        print("[LOCAL] START BLOCKED: " + error.code, flush=True)
        return 1
    except OSError:
        print("[LOCAL] START BLOCKED: fixture.io_error", flush=True)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
