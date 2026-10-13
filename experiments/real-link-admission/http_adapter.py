"""Fixed generated-fixture HTTP composition; caller cannot select a workspace."""
import asyncio
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'francestudent-probe'))
from server import Guard, request_proof, result, MAX_BODY
from probe_core import observe_headers
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
from forward_core import ForwardError


def make_app(gate, state, emit=print, allow_random_tunnel=False):
    mcp = MCPServer('Optic-Fixed-Read-Link', instructions=(
        'Generated test files only. All chats using this connection share one '
        'locally selected Optic session. No caller project or session selection.'))

    async def call(name):
        proof = request_proof.get()
        identity = proof['owner'] if proof else None
        try:
            upstream = await gate.call(identity, name)
            return result(name, {'ok': True, 'fixture_only': True, 'upstream': upstream})
        except ForwardError as error:
            return result(name, {'ok': False, 'code': str(error)})

    @mcp.tool()
    async def optic_session_info() -> dict:
        """Observe the fixed real Optic fixture session; supplies no authority."""
        return await call('optic_session_info')

    @mcp.tool()
    async def optic_fixture_list() -> dict:
        """List at most four entries in the generated public fixture directory."""
        return await call('optic_fixture_list')

    @mcp.tool()
    async def optic_fixture_read() -> dict:
        """Read at most1024bytes from the fixed generated public fixture file."""
        return await call('optic_fixture_read')

    def observe(headers):
        _, safe = observe_headers(headers)
        values = [value for name, value in headers if name.lower() == b'authorization']
        identity = state.authenticate(values[0]) if len(values) == 1 else None
        accepted = gate._current(identity)
        safe['matches_dummy'] = accepted
        return identity if accepted else None, safe

    class Recheck:
        def __init__(self, app):
            self.app = app

        async def __call__(self, scope, receive, send):
            if scope['type'] == 'http':
                proof = request_proof.get()
                if not proof or not gate._current(proof['owner']):
                    return await Guard.reject(send, 401)
            return await self.app(scope, receive, send)

    sdk = mcp.streamable_http_app(stateless_http=True, json_response=True,
        max_request_body_size=MAX_BODY,
        transport_security=TransportSecuritySettings(enable_dns_rebinding_protection=False)
            if allow_random_tunnel else TransportSecuritySettings(
            allowed_hosts=['127.0.0.1:*', 'localhost:*'],
            allowed_origins=['http://127.0.0.1:*', 'http://localhost:*']))
    return Guard(Recheck(sdk), header_observer=observe, emit=emit,
        zero_arg_tools=('optic_session_info', 'optic_fixture_list', 'optic_fixture_read'),
        handle_tool=None, timed_diagnostics=True)


async def local_stop(gate, upstream):
    """Local driver only; never an HTTP/MCP control tool or remote capability."""
    if not await gate.stop():
        return {'ok': False, 'code': 'link.drain_uncertain'}
    try:
        async with asyncio.timeout(5):
            response = await upstream.call_tool('session_cancel', {})
        if response.is_error:
            return {'ok': False, 'code': 'link.session_revoke_failed'}
    except Exception:
        return {'ok': False, 'code': 'link.session_revoke_failed'}
    return {'ok': True, 'drained': True, 'session_revoked': True}
