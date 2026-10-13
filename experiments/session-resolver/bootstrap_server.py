"""Fixed dummy-only three-tool carrier. Local lifecycle, no project authority."""
import asyncio
import json
import logging
import os
from pathlib import Path
import sys
import threading

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / 'credential-lifecycle'))
sys.path.insert(0, str(ROOT.parent / 'francestudent-probe'))
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
import uvicorn
from lifecycle_core import Registry
from lifecycle_server import Recheck
from probe_core import observe_headers
from server import Guard, request_proof, result, MAX_BODY, MAX_HEADERS, MAX_LIFETIME
from bootstrap_core import BootstrapResolver


def make_app(state, resolver, allow=False):
    mcp = MCPServer('Optic-Synthetic-Session-Bootstrap', instructions=(
        'Authority none. session_bootstrap returns a ticket as application_handle. '
        'Pass it unchanged to session_open; repeated open with SAME ticket returns '
        'same session reference. Pass that session reference to session_probe repeatedly. '
        'Do not confuse ticket and session reference. No workspace, effects or controls.'))
    def owner():
        proof = request_proof.get()
        return proof['owner'] if proof else None
    @mcp.tool()
    def session_bootstrap() -> dict:
        """Mint an expiring dummy bootstrap ticket; no session or effect authority."""
        return result('session_bootstrap', resolver.bootstrap(owner(), 'francestudent'))
    @mcp.tool()
    def session_open(application_handle: str) -> dict:
        """Pass bootstrap ticket unchanged. Retry SAME ticket for same synthetic session."""
        return result('session_open', resolver.open(owner(), 'francestudent', application_handle))
    @mcp.tool()
    def session_probe(application_handle: str) -> dict:
        """Pass SESSION reference from session_open; returns stable diagnostic id only."""
        return result('session_probe', resolver.probe(owner(), 'francestudent', application_handle))
    def observe(headers):
        _, safe = observe_headers(headers)
        values = [v for k, v in headers if k.lower() == b'authorization']
        identity = state.authenticate(values[0]) if len(values) == 1 else None
        safe['matches_dummy'] = identity is not None
        return identity, safe
    security = TransportSecuritySettings(enable_dns_rebinding_protection=False) if allow else TransportSecuritySettings(
        allowed_hosts=['127.0.0.1:*', 'localhost:*'],
        allowed_origins=['http://127.0.0.1:*', 'http://localhost:*'])
    upstream = mcp.streamable_http_app(stateless_http=True, json_response=True,
        max_request_body_size=MAX_BODY, transport_security=security)
    return Guard(Recheck(upstream, state), header_observer=observe,
        zero_arg_tools=('session_bootstrap',), handle_tool='session_open',
        extra_handle_tools=('session_probe',))


def console(state, resolver):
    commands = {'rotate-a': ('rotate', 'A'), 'revoke-a': ('revoke', 'A'),
                'rotate-b': ('rotate', 'B'), 'revoke-b': ('revoke', 'B'),
                'close-a': ('close', 'A'), 'close-b': ('close', 'B')}
    for _ in range(16):
        line = sys.stdin.readline(65)
        if not line or len(line) > 64 or not line.endswith('\n'):
            return
        command = commands.get(line.strip())
        if command is None:
            print('[LOCAL] command_refused', flush=True)
            continue
        operation, owner = command
        payload = resolver.close_owner_local(owner) if operation == 'close' else state.control(operation, owner)
        print('[LOCAL] ' + json.dumps(payload, sort_keys=True), flush=True)


async def run():
    state = Registry()
    resolver = BootstrapResolver(state)
    port = int(os.environ.get('OPTIC_SESSION_PROBE_PORT', '8006'))
    allow = os.environ.get('OPTIC_SESSION_PROBE_TUNNEL', '0') == '1'
    print(f'Authority NONE | synthetic-session-bootstrap-v1 | http://127.0.0.1:{port}/mcp | lifetime25min', flush=True)
    print('Ticket/session TTL120; live32/tickets128. Local close-a/b, rotate-a/b, revoke-a/b. No persistence.', flush=True)
    threading.Thread(target=console, args=(state, resolver), daemon=True).start()
    service = uvicorn.Server(uvicorn.Config(make_app(state, resolver, allow), host='127.0.0.1', port=port,
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


if __name__ == '__main__':
    logging.disable(logging.CRITICAL)
    try:
        asyncio.run(run())
    except KeyboardInterrupt:
        pass
