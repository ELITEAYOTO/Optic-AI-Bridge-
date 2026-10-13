import asyncio
import json
import logging
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / 'credential-lifecycle'))
sys.path.insert(0, str(ROOT.parent / 'session-resolver'))
sys.path.insert(0, str(ROOT.parent / 'francestudent-probe'))
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
import uvicorn
from lifecycle_core import Registry
from lifecycle_server import Recheck
from proof_channel import ProofChannel
from probe_core import observe_headers
from server import Guard, request_proof, result, MAX_BODY, MAX_HEADERS, MAX_LIFETIME
from group_core import Groups


def make_app(state, groups, emit, allow=False):
    mcp = MCPServer('Optic-H01-Grouped-Parallel', instructions=(
        'Synthetic grouped concurrency only, authority none. Create group once, '
        'pass its application_handle unchanged to TWO parallel_wait calls concurrently '
        'if the host permits it, then parallel_report. Do not invent overlap results.'))
    def owner():
        proof = request_proof.get()
        return proof['owner'] if proof else None
    @mcp.tool()
    def parallel_create_group() -> dict:
        """Create an expiring synthetic concurrency group, no session/project authority."""
        return result('parallel_create_group', groups.create(owner()))
    @mcp.tool()
    async def parallel_wait(application_handle: str) -> dict:
        """Harmless fixed3s async wait. Two same-group pending calls record real overlap."""
        proof = request_proof.get()
        payload = await groups.wait(owner(), application_handle, proof['sequence'] if proof else None)
        return result('parallel_wait', payload)
    @mcp.tool()
    def parallel_report(application_handle: str) -> dict:
        """Return bounded same-group intervals; completed_pair_overlap is measured, not assumed."""
        return result('parallel_report', groups.report(owner(), application_handle))
    def observe(headers):
        _, safe = observe_headers(headers)
        values = [v for k, v in headers if k.lower() == b'authorization']
        identity = state.authenticate(values[0]) if len(values) == 1 else None
        safe['matches_dummy'] = identity is not None
        return identity, safe
    security = TransportSecuritySettings(enable_dns_rebinding_protection=False) if allow else TransportSecuritySettings(
        allowed_hosts=['127.0.0.1:*', 'localhost:*'], allowed_origins=['http://127.0.0.1:*', 'http://localhost:*'])
    upstream = mcp.streamable_http_app(stateless_http=True, json_response=True,
        max_request_body_size=MAX_BODY, transport_security=security)
    return Guard(Recheck(upstream, state), emit=emit, header_observer=observe,
        zero_arg_tools=('parallel_create_group',), handle_tool='parallel_wait',
        extra_handle_tools=('parallel_report',), timed_diagnostics=True)


async def run():
    channel = ProofChannel()
    state = Registry()
    groups = Groups(state, emit=lambda event: channel.emit('[GROUP] ' + json.dumps(event, sort_keys=True)))
    port = int(os.environ.get('OPTIC_GROUPED_PORT', '8007'))
    allow = os.environ.get('OPTIC_GROUPED_TUNNEL', '0') == '1'
    print(f'Authority NONE | h01-grouped-v1 | http://127.0.0.1:{port}/mcp | lifetime25min', flush=True)
    print('Fixed delay3s. TTL120/groups16/records8/pending4 per group, pending8 globally. No project or controls.', flush=True)
    service = uvicorn.Server(uvicorn.Config(make_app(state, groups, channel.emit, allow),
        host='127.0.0.1', port=port, access_log=False, log_config=None, limit_concurrency=32,
        timeout_keep_alive=5, h11_max_incomplete_event_size=MAX_HEADERS, timeout_graceful_shutdown=5))
    async def stop():
        await asyncio.sleep(MAX_LIFETIME)
        service.should_exit = True
    timer = asyncio.create_task(stop())
    try:
        await service.serve()
    finally:
        timer.cancel()
        channel.close()


if __name__ == '__main__':
    logging.disable(logging.CRITICAL)
    try:
        asyncio.run(run())
    except KeyboardInterrupt:
        pass
